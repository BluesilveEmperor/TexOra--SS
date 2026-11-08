//! Playing texoravideo clips inside the slide.
//!
//! Each clip that plays has its own thread, which reads its frames (`video.rs`: a GIF decoded by TeXora,
//! another video read from the person's own ffmpeg) and keeps time by the clock, handing the newest
//! frame to the screen as an ordinary texture. A click plays or pauses, and showing another slide stops
//! the player and its ffmpeg (overlay steps of the same slide keep it going). With VLC instead of
//! ffmpeg, a click opens the clip in VLC's own window.
use crate::video::{self, Frame};
use egui::{Color32, Context, Rect, TextureHandle};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

enum Cmd {
    Play,
    Pause,
    Stop,
}

/// What the player's thread shares with the screen.
#[derive(Default)]
pub struct Shared {
    pub frame: Option<Frame>,
    /// Goes up with every new frame, so the screen uploads each one once.
    pub seq: u64,
    pub playing: bool,
    pub ended: bool,
    pub error: Option<String>,
    /// Seconds into the clip of the frame on show, and the clip's length.
    pub at: f64,
    pub duration: f64,
}

pub struct Player {
    pub id: u64,
    /// The slide (Beamer frame) it belongs to, and the document's generation.
    pub frame_no: usize,
    pub gen: u64,
    pub path: PathBuf,
    pub shared: Arc<Mutex<Shared>>,
    tx: mpsc::Sender<Cmd>,
    /// The texture the newest frame is in, and which frame that is.
    rgba: Option<(u64, TextureHandle)>,
}

impl Player {
    pub fn start(id: u64, frame_no: usize, gen: u64, path: PathBuf, name: String, repeat: bool, ctx: Context) -> Player {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let (tx, rx) = mpsc::channel();
        let (s, p) = (shared.clone(), path.clone());
        std::thread::Builder::new()
            .name("texora-video".into())
            .spawn(move || run(&p, &name, repeat, &s, &rx, &ctx))
            .ok();
        Player { id, frame_no, gen, path, shared, tx, rgba: None }
    }

    pub fn playing(&self) -> bool {
        self.shared.lock().map(|s| s.playing).unwrap_or(false)
    }

    pub fn play(&self) {
        let _ = self.tx.send(Cmd::Play);
    }

    pub fn toggle(&self) {
        let _ = self.tx.send(if self.playing() { Cmd::Pause } else { Cmd::Play });
    }
}

impl Drop for Player {
    /// The thread ends, and with it its ffmpeg process if the clip had one.
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Stop);
    }
}

/// The player's thread: waits while paused, and while playing shows each frame at its time.
fn run(path: &std::path::Path, name: &str, repeat: bool, shared: &Mutex<Shared>, rx: &mpsc::Receiver<Cmd>, ctx: &Context) {
    let set = |f: &dyn Fn(&mut Shared)| {
        if let Ok(mut s) = shared.lock() {
            f(&mut s);
        }
        ctx.request_repaint();
    };
    let mut stream = match video::open(path) {
        Ok(s) => s,
        Err(p) => {
            let msg = p.message(name);
            set(&|s| s.error = Some(msg.clone()));
            return;
        }
    };
    let duration = stream.info().duration;
    set(&|s| s.duration = duration);
    let (mut playing, mut ended) = (false, false);
    let mut held: Option<Frame> = None;
    let mut waiting: Option<Cmd> = None;
    // The clip time `base` was on show at the instant `origin`.
    let (mut origin, mut base, mut last) = (Instant::now(), 0.0f64, 0.0f64);
    loop {
        let cmd = match waiting.take() {
            Some(c) => Some(c),
            None if playing => rx.try_recv().ok(),
            None => match rx.recv() {
                Ok(c) => Some(c),
                Err(_) => return,
            },
        };
        match cmd {
            Some(Cmd::Stop) => return,
            Some(Cmd::Pause) => {
                playing = false;
                set(&|s| s.playing = false);
                continue;
            }
            Some(Cmd::Play) => {
                if ended {
                    stream.seek(0.0);
                    (held, last, ended) = (None, 0.0, false);
                }
                playing = true;
                (origin, base) = (Instant::now(), last);
                set(&|s| (s.playing, s.ended) = (true, false));
            }
            None => {}
        }
        if !playing {
            continue;
        }
        let Some(f) = held.take().or_else(|| stream.next_frame()) else {
            if repeat && last > 0.0 {
                stream.seek(0.0);
                (origin, base, last) = (Instant::now(), 0.0, 0.0);
                continue;
            }
            (playing, ended) = (false, true);
            set(&|s| (s.playing, s.ended) = (false, true));
            continue;
        };
        let now = Instant::now();
        let due = origin + Duration::from_secs_f64((f.at - base).max(0.0));
        if due > now {
            match rx.recv_timeout(due - now) {
                Ok(c) => {
                    // Answer at once; this frame waits for its turn.
                    held = Some(f);
                    waiting = Some(c);
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        } else if now - due > Duration::from_millis(400) {
            // Far behind (the computer was busy): carry on from here rather than race.
            (origin, base) = (now, f.at);
        }
        last = f.at;
        if let Ok(mut s) = shared.lock() {
            s.at = f.at;
            s.frame = Some(f);
            s.seq += 1;
        }
        ctx.request_repaint();
    }
}

// ---------- drawing ----------

/// Draws the player's newest frame in `rect`. Returns false while there is none yet (the poster in the
/// PDF shows instead).
pub fn paint(painter: &egui::Painter, ctx: &Context, player: &mut Player, rect: Rect) -> bool {
    let fresh = {
        let Ok(s) = player.shared.lock() else { return false };
        let Some(f) = &s.frame else { return false };
        let new = player.rgba.as_ref().is_none_or(|(q, _)| *q != s.seq);
        new.then(|| (s.seq, egui::ColorImage::from_rgba_unmultiplied([f.picture.width as usize, f.picture.height as usize], &f.picture.data)))
    };
    if let Some((seq, img)) = fresh {
        match &mut player.rgba {
            Some((q, t)) => {
                t.set(img, egui::TextureOptions::LINEAR);
                *q = seq;
            }
            None => player.rgba = Some((seq, ctx.load_texture(format!("video-{}", player.id), img, egui::TextureOptions::LINEAR))),
        }
    }
    if let Some((_, t)) = &player.rgba {
        painter.image(t.id(), rect, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
    }
    true
}

// ---------- help to install a video tool ----------

/// Room at the bottom of a clip for the install buttons: two rows of small buttons.
const HELP_H: f32 = 56.0;

pub const VLC_PAGE: &str = "https://www.videolan.org";
pub const FFMPEG_PAGE: &str = "https://ffmpeg.org/download.html";

/// The command that installs ffmpeg on this system, for the person to run themselves. Windows has
/// none: there the official download page is the way.
pub fn install_command() -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        Some("brew install ffmpeg")
    } else if cfg!(target_os = "linux") {
        Some(concat!("sudo", " apt install ffmpeg"))
    } else {
        None
    }
}

/// "Get VLC", "Get ffmpeg" and "Copy install command". TeXora opens the pages and copies the command;
/// it never downloads, installs or runs anything itself. Returns what to say after a copy.
pub fn install_buttons(ui: &mut egui::Ui) -> Option<String> {
    let mut said = None;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.horizontal(|ui| {
            if ui.small_button("Get VLC").on_hover_text(VLC_PAGE).clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(VLC_PAGE));
            }
            if ui.small_button("Get ffmpeg").on_hover_text(FFMPEG_PAGE).clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(FFMPEG_PAGE));
            }
        });
        if let Some(cmd) = install_command() {
            if ui.small_button("Copy install command").on_hover_text(cmd).clicked() {
                ui.ctx().copy_text(cmd.to_string());
                said = Some(format!("Copied \"{cmd}\". Paste it into Terminal and run it yourself; TeXora never runs it."));
            }
        }
    });
    said
}

// ---------- the app's side ----------

use crate::app::App;
use crate::pdf::{PageVideo, Region};
use crate::ui_present::to_screen;

/// Where a clip's rectangle is across the slide shown (one half of a double-width page, or all of it),
/// if it lies inside that slide. A clip only counts when it is wholly inside: Beamer's notes pages
/// carry a small copy of the slide whose links reach past the page's edge, and those are not clips.
fn in_region(r: [f32; 4], region: Region) -> Option<(f32, f32)> {
    let (x0, x1) = match region {
        Region::Full => (r[0], r[2]),
        Region::Left => (r[0] * 2.0, r[2] * 2.0),
        Region::Right => (r[0] * 2.0 - 1.0, r[2] * 2.0 - 1.0),
    };
    let e = 0.01;
    (x0 >= -e && x1 <= 1.0 + e && r[1] >= -e && r[3] <= 1.0 + e && x1 > x0).then_some((x0, x1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_clips_inside_the_shown_slide_count() {
        // The GIF of the Video Guide, and the copy of its link on the notes half, past the page's top.
        let clip = [0.0278, 0.2837, 0.2633, 0.7581];
        let notes_copy = [0.4028, -0.4663, 0.6383, 0.0081];
        assert!(in_region(clip, Region::Left).is_some());
        assert!(in_region(notes_copy, Region::Left).is_none());
        assert!(in_region(notes_copy, Region::Right).is_none());
        assert!(in_region([0.1, 0.2, 0.6, 0.8], Region::Full).is_some());
        // A clip on the right half is not shown on the left.
        assert!(in_region([0.6, 0.2, 0.9, 0.8], Region::Left).is_none());
        assert!(in_region([0.6, 0.2, 0.9, 0.8], Region::Right).is_some());
    }

    #[test]
    fn install_help_only_points_the_way() {
        let cmd = install_command();
        if cfg!(target_os = "macos") {
            assert_eq!(cmd, Some("brew install ffmpeg"));
        }
        if cfg!(windows) {
            assert_eq!(cmd, None, "Windows: the official download page only");
        }
        assert!(VLC_PAGE.starts_with("https://www.videolan.org") && FFMPEG_PAGE.starts_with("https://ffmpeg.org/"));
    }
}

impl App {
    /// Where each texoravideo clip of a page sits on screen, for a slide drawn in `rect`.
    pub fn video_rects(&self, rect: Rect, page: usize, region: Region, uv: Rect) -> Vec<(PageVideo, Rect)> {
        let Some(pg) = self.doc.as_ref().and_then(|d| d.pages.get(page)) else { return vec![] };
        pg.videos
            .iter()
            .filter_map(|v| in_region(v.rect, region).map(|(x0, x1)| (v.clone(), Rect::from_two_pos(to_screen(rect, uv, [x0, v.rect[1]]), to_screen(rect, uv, [x1, v.rect[3]])))))
            .collect()
    }

    fn player_index(&self, frame_no: usize, v: &PageVideo) -> Option<usize> {
        let file = v.file.as_ref()?;
        self.players.iter().position(|p| p.gen == self.gen && p.frame_no == frame_no && &p.path == file)
    }

    fn start_player(&mut self, ctx: &Context, frame_no: usize, v: &PageVideo) -> Option<usize> {
        let file = v.file.clone()?;
        self.player_seq += 1;
        self.players.push(Player::start(self.player_seq, frame_no, self.gen, file, v.link.path.clone(), v.link.repeat, ctx.clone()));
        Some(self.players.len() - 1)
    }

    /// A click on a clip: plays it, or pauses it while it plays. With VLC as the video tool, a clip that
    /// is not a GIF opens in VLC's own window.
    pub fn video_click(&mut self, ctx: &Context, idx: usize, v: &PageVideo) {
        if let Some(p) = &v.problem {
            self.status = p.replace('\n', " ");
            return;
        }
        let Some(frame_no) = self.deck.slides.get(idx).map(|s| s.frame) else { return };
        if let (Some(file), video::Backend::Vlc(vlc)) = (&v.file, video::backend()) {
            if !video::is_gif_file(file) {
                self.status = if self.is_screenshot() {
                    format!("Would play {} in VLC.", v.link.path)
                } else {
                    match video::open_in_vlc(&vlc, file, v.link.repeat) {
                        Ok(()) => format!("{} is playing in VLC.", v.link.path),
                        Err(e) => format!("VLC could not be started: {e}"),
                    }
                };
                return;
            }
        }
        match self.player_index(frame_no, v) {
            Some(i) => self.players[i].toggle(),
            None => {
                if let Some(i) = self.start_player(ctx, frame_no, v) {
                    self.players[i].play();
                }
            }
        }
    }

    /// Showing another slide stops its clips; the new slide's `autoplay` clips start. Overlay steps of
    /// one slide count as the same slide, so a clip plays on through them.
    pub fn video_slide_changed(&mut self, ctx: &Context) {
        let Some(s) = self.deck.slides.get(self.live()).cloned() else {
            self.players.clear();
            self.video_frame = None;
            return;
        };
        let now = Some((self.gen, s.frame));
        if now == self.video_frame {
            return;
        }
        self.video_frame = now;
        let gen = self.gen;
        self.players.retain(|p| p.gen == gen && p.frame_no == s.frame);
        let videos = self.doc.as_ref().and_then(|d| d.pages.get(s.page)).map(|p| p.videos.clone()).unwrap_or_default();
        // Clips start by themselves inside the slide only; VLC opens on a click.
        let vlc = matches!(video::backend(), video::Backend::Vlc(_));
        for v in videos.iter().filter(|v| v.link.autoplay && v.problem.is_none() && in_region(v.rect, s.region).is_some()) {
            if vlc && !v.file.as_ref().is_some_and(|f| video::is_gif_file(f)) {
                continue;
            }
            if self.player_index(s.frame, v).is_none() {
                if let Some(i) = self.start_player(ctx, s.frame, v) {
                    self.players[i].play();
                }
            }
        }
    }

    /// Settings > Video: which installed tool plays videos that are not GIFs, where it is, and a test.
    pub fn video_settings(&mut self, ui: &mut egui::Ui) {
        use crate::app::{LEAF_DARK, MUTED};
        use egui::RichText;
        ui.label(RichText::new("GIFs play by themselves. MP4 and other videos play through VLC or ffmpeg installed on your computer: ffmpeg plays them inside the slide, VLC in its own window. TeXora contains no video decoder and never installs one.").size(12.5));
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Video player");
            let now = video::Choice::from_setting(&self.st.video_player);
            egui::ComboBox::from_id_salt("video-player").selected_text(now.label()).show_ui(ui, |ui| {
                for c in video::Choice::ALL {
                    if ui.selectable_label(now == c, c.label()).clicked() {
                        self.st.video_player = c.setting().to_string();
                        changed = true;
                    }
                }
            });
        });
        let found = video::tools();
        let test_id = crate::ui_present::sid("video-tool-test");
        for (name, vlc) in [("ffmpeg", false), ("VLC", true)] {
            let text = if vlc { &mut self.st.vlc_path } else { &mut self.st.ffmpeg_path };
            let auto = if vlc { &found.vlc } else { &found.ffmpeg };
            let program = if text.trim().is_empty() { auto.clone() } else { Some(PathBuf::from(text.trim())).filter(|p| p.is_file()) };
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).strong());
                let state = match &program {
                    Some(_) if text.trim().is_empty() => "found",
                    Some(_) => "set by you",
                    None => "not found",
                };
                ui.label(RichText::new(state).size(12.0).color(MUTED));
                if ui.small_button("Test").clicked() {
                    let said = match &program {
                        Some(p) => match video::test_tool(p, vlc) {
                            Ok(v) => format!("{name} works: {v}"),
                            Err(e) => format!("{name} did not work: {e}"),
                        },
                        None => format!("{name} was not found on this computer."),
                    };
                    ui.ctx().data_mut(|d| d.insert_temp(test_id, said));
                }
            });
            let hint = match auto {
                Some(p) => p.display().to_string(),
                None => "Path to the program (leave empty to look for it)".to_string(),
            };
            if ui.add(egui::TextEdit::singleline(text).hint_text(hint).desired_width(f32::INFINITY)).lost_focus() {
                changed = true;
            }
        }
        if let Some(said) = ui.ctx().data(|d| d.get_temp::<String>(test_id)) {
            ui.label(RichText::new(said).size(12.5).color(LEAF_DARK));
        }
        let using = match video::backend() {
            video::Backend::Ffmpeg(p) => format!("Videos play inside the slide through ffmpeg ({}).", p.display()),
            video::Backend::Vlc(p) => format!("Videos open in VLC ({}). Install ffmpeg to play them inside the slide and to make their posters.", p.display()),
            video::Backend::Off => "Video playing is off. GIFs still play.".to_string(),
            video::Backend::None => video::NEEDS_PLAYER.to_string(),
        };
        ui.label(RichText::new(using).size(12.5).color(LEAF_DARK));
        if matches!(video::backend(), video::Backend::None) || found.ffmpeg.is_none() {
            if let Some(said) = install_buttons(ui) {
                self.status = said;
            }
        }
        if changed {
            video::configure(&self.st.video_player, &self.st.ffmpeg_path, &self.st.vlc_path);
            self.save_layout();
        }
    }

    /// The install buttons over each clip of the slide that needs a video tool, on the speaker's screen.
    /// They are made after the slide's own click area, so they get the clicks.
    pub fn video_help(&mut self, ui: &mut egui::Ui, rect: Rect, idx: usize, uv: Rect) {
        let Some(s) = self.deck.slides.get(idx).cloned() else { return };
        for (i, (v, r)) in self.video_rects(rect, s.page, s.region, uv).into_iter().enumerate() {
            if v.problem.as_deref() != Some(video::NEEDS_PLAYER) || r.height() <= 60.0 {
                continue;
            }
            // Two rows, so the buttons wrap on a small clip instead of being cut off.
            let area = Rect::from_min_max(egui::pos2(r.left() + 6.0, r.bottom() - HELP_H), egui::pos2(r.right() - 6.0, r.bottom() - 4.0)).intersect(rect);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area).id_salt(("video-help", s.page, i)).layout(egui::Layout::top_down(egui::Align::Min)));
            child.set_clip_rect(area);
            if let Some(said) = install_buttons(&mut child) {
                self.status = said;
            }
        }
    }

    /// The clips of a slide drawn in `rect`: their picture while they play, and for the speaker the
    /// reason one cannot play, a play sign and how far it has got.
    pub fn paint_videos(&mut self, ui: &egui::Ui, rect: Rect, idx: usize, uv: Rect) {
        let Some(s) = self.deck.slides.get(idx).cloned() else { return };
        let list = self.video_rects(rect, s.page, s.region, uv);
        if list.is_empty() {
            return;
        }
        let speaker = !self.painting_audience;
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let ctx = ui.ctx().clone();
        for (v, r) in list {
            let mut state: Option<(bool, f64, f64, Option<String>)> = None;
            if let Some(i) = self.player_index(s.frame, &v) {
                paint(&painter, &ctx, &mut self.players[i], r);
                state = self.players[i].shared.lock().ok().map(|x| (x.playing, x.at, x.duration, x.error.clone()));
            }
            if !speaker {
                continue;
            }
            let problem = v.problem.clone().or_else(|| state.as_ref().and_then(|x| x.3.clone()));
            if let Some(msg) = problem {
                // Room below the words for the install buttons (see `video_help`).
                let room = msg == video::NEEDS_PLAYER && r.height() > 60.0;
                message(&painter, r, &msg, room);
                continue;
            }
            let playing = state.as_ref().is_some_and(|x| x.0);
            let hovered = ui.rect_contains_pointer(r);
            if !playing && (hovered || state.is_some()) {
                let d = (r.height().min(r.width()) * 0.3).clamp(22.0, 64.0);
                painter.circle_filled(r.center(), d / 2.0, Color32::from_rgba_unmultiplied(20, 30, 25, 170));
                crate::icons::paint(&painter, Rect::from_center_size(r.center() + egui::vec2(d * 0.05, 0.0), egui::vec2(d * 0.5, d * 0.5)), crate::icons::Icon::Play, Color32::WHITE);
            }
            if let Some((_, at, duration, _)) = state {
                if duration > 0.0 {
                    let bar = Rect::from_min_max(egui::pos2(r.left(), r.bottom() - 3.0), r.max);
                    painter.rect_filled(bar, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 90));
                    let done = (at / duration).clamp(0.0, 1.0) as f32;
                    painter.rect_filled(Rect::from_min_max(bar.min, egui::pos2(bar.left() + bar.width() * done, bar.bottom())), 0.0, crate::app::LEAF);
                }
            }
        }
    }
}

/// The reason a clip cannot play, written over its poster; `room_below` leaves space for buttons.
fn message(painter: &egui::Painter, r: Rect, text: &str, room_below: bool) {
    painter.rect_filled(r, 2.0, Color32::from_rgba_unmultiplied(25, 25, 28, 215));
    let size = (r.height() / 11.0).clamp(10.0, 17.0);
    let galley = painter.layout(text.to_string(), egui::FontId::proportional(size), Color32::WHITE, (r.width() - 16.0).max(40.0));
    let centre = if room_below { egui::pos2(r.center().x, (r.top() + r.bottom() - HELP_H) / 2.0) } else { r.center() };
    let at = centre - galley.size() / 2.0;
    painter.galley(at, galley, Color32::WHITE);
}
