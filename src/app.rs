//! Application state and everything that is not drawing: opening files, navigation, the timer,
//! background compiles, ink, and the screenshot option.
use crate::analytics::{InkKind, Session, Sidecar, Stroke};
use crate::deck::{self, Deck};
use crate::latex::{self, CompileResult, Problem};
use crate::notes::{self, Notes, TexFrame};
use crate::pdf::{self, Cmd, DocInfo, LinkTarget, PdfEngine, Region, RenderReq, Reply, SearchHit};
use crate::settings::{self, Action, KeyBind, Settings};
use crate::tabs::Tab;
use egui::{Color32, Context, TextureHandle, TextureOptions, ViewportCommand, ViewportId};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

pub const BG: Color32 = Color32::from_rgb(245, 244, 242);
pub const LINE: Color32 = Color32::from_rgb(204, 204, 202);
pub const INK: Color32 = Color32::from_rgb(20, 20, 20);
pub const MUTED: Color32 = Color32::from_rgb(112, 112, 110);
pub const LEAF: Color32 = Color32::from_rgb(74, 132, 58);
pub const LEAF_DARK: Color32 = Color32::from_rgb(33, 65, 29);
pub const LEAF_SOFT: Color32 = Color32::from_rgb(222, 236, 214);
pub const SURFACE: Color32 = Color32::from_rgb(252, 251, 249);
pub const MAX_ZOOM: f32 = 6.0;
pub const WARN: Color32 = Color32::from_rgb(190, 110, 10);
pub const BAD: Color32 = Color32::from_rgb(190, 40, 40);
pub const PALETTE: [[u8; 3]; 6] =
    [[214, 40, 40], [240, 150, 20], [240, 210, 40], [40, 160, 70], [30, 110, 220], [170, 60, 200]];

const MAX_RENDER_WIDTH: u32 = 3000;
pub const SAMPLE_TEX: &str = include_str!("../assets/sample/texora-sample.tex");
pub const SAMPLE_PDF: &[u8] = include_bytes!("../assets/sample/texora-sample.pdf");
const SAMPLE_NAV: &str = include_str!("../assets/sample/texora-sample.nav");
/// Marks the current sample lecture; a copy without it was made by an older version.
const SAMPLE_MARK: &str = "% TeXora sample lecture, edition 2";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Start,
    Presenter,
    /// Slide and notes side by side in one window.
    Split,
    Overview,
    /// The LaTeX studio: source editor, slide preview and compile messages.
    Studio,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LeftTab {
    Bookmarks,
    Search,
    Analytics,
    Settings,
    Docs,
}

/// The tabs of the studio's message panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogTab {
    Problems,
    Packages,
    Log,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Pointer,
    Pen,
    Highlighter,
    Eraser,
    Laser,
    Zoom,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Live,
    Black,
    White,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotesView {
    Auto,
    Pdf,
    Text,
}

pub struct Source {
    pub path: PathBuf,
    pub text: String,
    pub frames: Vec<TexFrame>,
    pub dirty: bool,
    pub last_edit: Instant,
    pub disk_mtime: Option<SystemTime>,
    pub cursor_line: usize,
    pub synced_line: usize,
    pub line_changed: Instant,
    pub goto_line: Option<usize>,
    /// First lines of the folded blocks in the studio's editor.
    pub folded: std::collections::BTreeSet<usize>,
}

/// Where one animation is: the frame on show, whether it plays, which way, and when the next frame is due.
#[derive(Clone, Debug)]
pub struct AnimPlay {
    pub frame: usize,
    pub playing: bool,
    pub forward: bool,
    pub next_at: Instant,
}

impl AnimPlay {
    pub fn start() -> Self {
        AnimPlay { frame: 0, playing: true, forward: true, next_at: Instant::now() }
    }
}

/// One step of an animation in the way it plays, with what happens at either end.
pub fn anim_step(st: &mut AnimPlay, frames: usize, end: pdf::AnimEnd) {
    let last = frames.saturating_sub(1);
    if st.forward {
        if st.frame < last {
            st.frame += 1;
        } else {
            match end {
                pdf::AnimEnd::Loop => st.frame = 0,
                pdf::AnimEnd::Bounce => {
                    st.forward = false;
                    st.frame = last.saturating_sub(1);
                }
                pdf::AnimEnd::Stop => st.playing = false,
            }
        }
    } else if st.frame > 0 {
        st.frame -= 1;
    } else {
        match end {
            pdf::AnimEnd::Loop => st.frame = last,
            pdf::AnimEnd::Bounce => {
                st.forward = true;
                st.frame = 1.min(last);
            }
            pdf::AnimEnd::Stop => st.playing = false,
        }
    }
}

pub struct Args {
    pub file: Option<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub view: Option<String>,
    pub goto: Option<usize>,
    pub demo: bool,
    /// Create a presentation here from a template (`--new`), instead of opening a file.
    pub new_deck: Option<PathBuf>,
    /// The template for `new_deck`, by id.
    pub template: Option<String>,
    /// Window size in points, for checking small screens.
    pub window: Option<[f32; 2]>,
    /// Test input: `w:milliseconds`, `k:Key`, `m:x,y` (move), `d:x,y` (press), `u:x,y` (release), separated by `;`.
    pub script: Vec<String>,
}

pub struct App {
    /// One entry per tab. The active tab's working state lives in this struct instead of its entry.
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub next_tab_id: u64,
    pub next_gen: u64,
    /// The tab shown in the audience window, which need not be the one in front.
    pub audience_tab: u64,
    pub close_request: Option<u64>,
    pub failed_open: Option<(u64, String)>,
    pub st: Settings,
    pub engine: PdfEngine,
    pub gen: u64,
    pub doc: Option<DocInfo>,
    pub deck: Deck,
    pub pdf_path: Option<PathBuf>,
    /// The deck whose ink, timings and notes are loaded. Only changes once a new PDF has really opened.
    pub deck_path: Option<PathBuf>,
    pub pdf_mtime: Option<SystemTime>,
    /// A PDF has been asked for and has not arrived yet.
    pub loading: bool,
    pub cur: usize,
    pub frozen: Option<usize>,
    pub history: Vec<usize>,
    pub view: View,
    pub left: Option<LeftTab>,
    pub focus_jump: bool,
    pub about_open: bool,
    /// The box that shows where this copy's command-line tool is.
    pub cli_open: bool,
    /// The template picker for a new presentation, with the template selected.
    pub template_picker: Option<usize>,
    /// Open Settings at the Required LaTeX packages section once.
    pub show_packages: bool,
    /// ? > System check.
    pub syscheck_open: bool,
    /// The last text copied from that box, for the test report.
    pub cli_copied: Option<String>,
    /// In a test run, what the editor put on the clipboard (kept off the real clipboard).
    pub test_clip: Option<String>,
    pub update_open: bool,
    pub update_rx: Option<Receiver<crate::update::Outcome>>,
    pub update_result: Option<crate::update::Outcome>,
    /// What the quiet check at launch, or a later one, found.
    pub update_notice: Option<crate::update::Outcome>,
    /// When the last quiet check started.
    pub update_checked: Option<Instant>,
    /// The speaker chose "Later" on the update notice: it stays away until TeXora starts again.
    pub update_later: bool,
    /// Whether the update notice was on screen in the last frame, for the test report.
    pub update_toast: bool,
    pub update_bg_rx: Option<Receiver<crate::update::Outcome>>,
    pub focus_search: bool,
    pub duration_open: bool,
    pub duration_rect: egui::Rect,
    /// Notes being edited: the slide they belong to and the text so far, not yet saved.
    pub note_draft: Option<(usize, String)>,
    pub note_focus: bool,
    pub status: String,
    pub status_seen: (String, Instant),
    pub engine_error: Option<String>,

    pub cache: HashMap<(usize, Region), Vec<(u32, TextureHandle, u64)>>,
    pub pending: HashSet<RenderReq>,
    pub tick: u64,

    pub side: Sidecar,
    pub notes: Notes,
    pub notes_dirty: Option<Instant>,
    pub notes_view: NotesView,
    pub swap: bool,

    pub tool: Tool,
    pub screen: Screen,
    pub stroke: Option<Stroke>,
    pub laser: Vec<([f32; 2], f64)>,
    pub zoom: Option<[f32; 4]>,
    pub zoom_drag: Option<[f32; 2]>,
    pub zoom_changed: Instant,
    pub notes_zoom: Option<[f32; 4]>,
    pub next_zoom: Option<[f32; 4]>,
    pub peek: Option<usize>,

    pub timer_on: bool,
    /// Set once the timer has been started or stopped, so the first slide change
    /// only starts it automatically if nobody has touched it yet.
    pub timer_touched: bool,
    pub elapsed: f64,
    pub timer_last: Instant,
    pub session: Session,

    pub audience_open: bool,
    pub audience_full: bool,
    pub audience_age: u32,
    pub displays: Vec<display_info::DisplayInfo>,
    pub display: usize,

    pub src: Option<Source>,
    pub compiling: bool,
    pub compile_again: bool,
    pub compile_tx: Sender<CompileResult>,
    pub compile_rx: Receiver<CompileResult>,
    pub problems: Vec<Problem>,
    pub full_log: String,
    pub reqs: Vec<latex::Requirement>,
    pub req_warning: Option<String>,
    pub req_rx: Option<Receiver<(Vec<latex::Requirement>, Option<String>)>>,
    pub log_tab: LogTab,
    pub log_wrap: bool,
    pub log_open: bool,
    pub compile_ok: Option<bool>,
    pub compile_note: String,
    pub open_after_compile: bool,
    /// Notes just written into the .tex source, by slide. They show at once, while the slides
    /// rebuild, and give way when the source's note for that slide is changed in the studio.
    pub note_pending: crate::notes::Notes,
    pub sync_mark: Option<(usize, [f32; 2], Instant)>,
    /// Animations in the PDF (the animate package): where each one is in its play, by (page, number).
    pub anim_play: HashMap<(usize, usize), AnimPlay>,
    /// Frame pictures, by (page, animation, frame): the page width they were made for, and the picture.
    pub anim_cache: HashMap<(usize, usize, usize), (u32, TextureHandle)>,
    pub anim_pending: HashSet<pdf::FrameReq>,
    /// The page whose animations were last started, so a new slide starts its own.
    pub anim_page: Option<usize>,
    /// texoravideo clips playing or paused on the slide on show; another slide stops them.
    pub players: Vec<crate::ui_video::Player>,
    /// The document generation and slide (Beamer frame) the players belong to.
    pub video_frame: Option<(u64, usize)>,
    pub player_seq: u64,
    /// True while the audience window is being drawn: it shows clips but no messages or buttons.
    pub painting_audience: bool,
    /// Where a press on a clip began, to tell a click on it.
    pub video_press: Option<egui::Pos2>,
    /// In the studio, show the slide for the line the cursor is on.
    pub follow_cursor: bool,
    pub tex_checked: Instant,

    pub search_q: String,
    pub search_sent: String,
    pub hits: Vec<SearchHit>,
    pub jump_text: String,
    pub capture: Option<Action>,

    shot: Option<PathBuf>,
    shot_sent: bool,
    shot_done: Arc<AtomicBool>,
    frames_drawn: u32,
    goto_on_open: Option<usize>,
    demo: bool,
    script: std::collections::VecDeque<String>,
    script_wait: u32,
    script_until: Option<Instant>,
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Unpacks the sample lecture so it behaves like any other file the person could open.
pub fn ensure_sample() -> Option<PathBuf> {
    let dir = settings::data_dir().join("sample");
    std::fs::create_dir_all(&dir).ok()?;
    let pdf = dir.join("texora-sample.pdf");
    let tex = dir.join("texora-sample.tex");
    // A copy made by an older version is refreshed; it is kept beside, in case it was edited.
    let current = std::fs::read_to_string(&tex).map(|t| t.contains(SAMPLE_MARK)).unwrap_or(false);
    if !pdf.exists() || !current {
        if tex.exists() && !current {
            let _ = std::fs::copy(&tex, dir.join("texora-sample.tex.old"));
        }
        std::fs::write(&pdf, SAMPLE_PDF).ok()?;
        std::fs::write(dir.join("texora-sample.nav"), SAMPLE_NAV).ok()?;
        std::fs::write(&tex, SAMPLE_TEX).ok()?;
    }
    // The sample comes with a 20-minute duration and a bookmark on slide 6, as in the guide's
    // pictures. Ink and rehearsals already saved for it stay.
    let side_path = crate::analytics::Sidecar::path_for(&pdf);
    let mut side = crate::analytics::Sidecar::load(&pdf);
    if !side_path.exists() || side.target_minutes == 0.0 {
        if !side_path.exists() {
            side.bookmarks.insert(5);
        }
        if side.target_minutes == 0.0 {
            side.target_minutes = 20.0;
        }
        let _ = side.save(&pdf);
    }
    Some(pdf)
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, args: Args) -> Self {
        let ctx = cc.egui_ctx.clone();
        crate::ui_chrome::apply_theme(&ctx);
        let wake = ctx.clone();
        let engine = pdf::spawn(move || wake.request_repaint());
        let (compile_tx, compile_rx) = channel();
        let displays = display_info::DisplayInfo::all().unwrap_or_default();
        let display = displays.iter().position(|d| !d.is_primary).unwrap_or(0);
        let st = Settings::load();
        ctx.set_zoom_factor(st.ui_scale);
        // Which installed video tool (if any) plays videos that are not GIFs.
        crate::video::configure(&st.video_player, &st.ffmpeg_path, &st.vlc_path, st.video_sound);
        let mut app = App {
            tabs: vec![Tab::blank(1)],
            active: 0,
            next_tab_id: 2,
            next_gen: 0,
            audience_tab: 1,
            close_request: None,
            failed_open: None,
            st,
            engine,
            gen: 0,
            doc: None,
            deck: Deck::default(),
            pdf_path: None,
            deck_path: None,
            pdf_mtime: None,
            loading: false,
            cur: 0,
            frozen: None,
            history: vec![],
            view: View::Start,
            left: None,
            focus_jump: false,
            about_open: false,
            cli_open: false,
            show_packages: false,
            template_picker: None,
            syscheck_open: false,
            cli_copied: None,
            test_clip: None,
            update_open: false,
            update_rx: None,
            update_result: None,
            update_notice: None,
            update_checked: None,
            update_later: false,
            update_toast: false,
            update_bg_rx: None,
            focus_search: false,
            duration_open: false,
            duration_rect: egui::Rect::NOTHING,
            note_draft: None,
            note_focus: false,
            status: String::new(),
            status_seen: (String::new(), Instant::now()),
            engine_error: None,
            cache: HashMap::new(),
            pending: HashSet::new(),
            tick: 0,
            side: Sidecar::default(),
            notes: Notes::new(),
            notes_dirty: None,
            notes_view: NotesView::Auto,
            swap: false,
            tool: Tool::Pointer,
            screen: Screen::Live,
            stroke: None,
            laser: vec![],
            zoom: None,
            zoom_drag: None,
            zoom_changed: Instant::now(),
            notes_zoom: None,
            next_zoom: None,
            peek: None,
            timer_on: false,
            timer_touched: false,
            elapsed: 0.0,
            timer_last: Instant::now(),
            session: Session { started_unix: now_unix(), ..Default::default() },
            audience_open: false,
            audience_full: false,
            audience_age: 0,
            displays,
            display,
            src: None,
            compiling: false,
            compile_again: false,
            compile_tx,
            compile_rx,
            problems: vec![],
            full_log: String::new(),
            reqs: vec![],
            req_warning: None,
            req_rx: None,
            log_tab: LogTab::Problems,
            log_wrap: true,
            log_open: true,
            compile_ok: None,
            compile_note: String::new(),
            open_after_compile: false,
            note_pending: crate::notes::Notes::new(),
            sync_mark: None,
            anim_play: HashMap::new(),
            anim_cache: HashMap::new(),
            anim_pending: HashSet::new(),
            anim_page: None,
            players: vec![],
            video_frame: None,
            player_seq: 0,
            painting_audience: false,
            video_press: None,
            follow_cursor: true,
            tex_checked: Instant::now(),
            search_q: String::new(),
            search_sent: String::new(),
            hits: vec![],
            jump_text: String::new(),
            capture: None,
            shot: args.screenshot.clone(),
            shot_sent: false,
            shot_done: Arc::new(AtomicBool::new(false)),
            frames_drawn: 0,
            goto_on_open: args.goto,
            demo: args.demo,
            script: args.script.iter().cloned().collect(),
            script_wait: 0,
            script_until: None,
        };
        app.check_updates_quietly(&ctx);
        // A file on the command line opens straight away; otherwise the start page greets you.
        let wants_deck = args.file.is_some() || args.view.is_some() || args.goto.is_some();
        match args.view.as_deref() {
            Some("analytics") => app.left = Some(LeftTab::Analytics),
            Some("settings") => app.left = Some(LeftTab::Settings),
            Some("outline") => app.left = Some(LeftTab::Bookmarks),
            Some("search") => {
                app.left = Some(LeftTab::Search);
                app.search_q = "graph".to_string();
            }
            Some("textnotes") => app.notes_view = NotesView::Text,
            _ => {}
        }
        if let Some(p) = &args.new_deck {
            let t = crate::templates::by_id(args.template.as_deref().unwrap_or(""));
            app.create_presentation(p, t);
        } else if wants_deck {
            match args.file.clone().or_else(ensure_sample) {
                Some(p) => app.open_path(&p),
                None => app.status = "Could not open that file.".to_string(),
            }
            let opened = app.view != View::Start;
            match args.view.as_deref().filter(|_| opened) {
                Some("overview") => app.view = View::Overview,
                Some("studio") => app.view = View::Studio,
                Some("split") => app.view = View::Split,
                Some("swap") => app.swap = true,
                _ => {}
            }
        }
        app
    }

    pub fn audience_only_shot(view: &Option<String>) -> bool {
        view.as_deref() == Some("audience")
    }

    // ---------- files ----------

    pub fn open_path(&mut self, path: &Path) {
        // A short path typed in a terminal ("talk.tex") becomes a full one, so LaTeX, the package check
        // and the recent list all find the file's folder.
        let full = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let path = full.as_path();
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        self.st.recent.retain(|p| p != path);
        self.st.recent.insert(0, path.to_path_buf());
        self.st.recent.truncate(8);
        if self.shot.is_none() {
            let _ = self.st.save();
        }
        if !matches!(ext.as_str(), "pdf" | "tex") {
            self.status = format!("TeXora opens .pdf and .tex files, not {}.", path.display());
            return;
        }
        // Each presentation has one tab: a file already open comes to the front.
        if let Some(i) = self.tab_for_path(path) {
            self.activate(i);
            self.close_home_tabs();
            return;
        }
        self.new_tab();
        self.close_home_tabs();
        match ext.as_str() {
            "pdf" => {
                let tex = path.with_extension("tex");
                self.src = if tex.exists() { self.read_source(&tex) } else { None };
                if self.view == View::Start {
                    self.view = View::Presenter;
                }
                self.check_packages();
                self.load_pdf(path);
            }
            "tex" => {
                self.src = self.read_source(path);
                let text = self.src.as_ref().map(|s| s.text.clone()).unwrap_or_default();
                let root = latex::root_of(path, &text);
                let pdf = root.with_extension("pdf");
                let stale = match (latex::mtime(&pdf), latex::mtime(path)) {
                    (Some(p), Some(t)) => p < t,
                    _ => true,
                };
                self.pdf_path = Some(pdf.clone());
                if self.view == View::Start {
                    self.view = View::Presenter;
                }
                self.check_packages();
                if stale {
                    self.open_after_compile = true;
                    self.start_compile();
                    self.status = "Compiling your slides…".to_string();
                } else {
                    self.load_pdf(&pdf);
                }
            }
            _ => {}
        }
    }

    fn read_source(&self, path: &Path) -> Option<Source> {
        let text = notes::read_text(path)?;
        // One untouched copy, byte for byte, made the first time TeXora opens the file.
        let bak = path.with_extension("tex.texora-backup");
        if !bak.exists() {
            let _ = std::fs::copy(path, &bak);
        }
        let frames = notes::parse_tex_frames(&text);
        Some(Source {
            path: path.to_path_buf(),
            text,
            frames,
            dirty: false,
            last_edit: Instant::now(),
            disk_mtime: latex::mtime(path),
            cursor_line: 1,
            synced_line: 0,
            line_changed: Instant::now(),
            goto_line: None,
            folded: Default::default(),
        })
    }

    pub fn load_pdf(&mut self, path: &Path) {
        let old = self.gen;
        self.next_gen += 1;
        self.gen = self.next_gen;
        self.pdf_mtime = latex::mtime(path);
        self.loading = true;
        let _ = self.engine.tx.send(Cmd::Open { path: path.to_path_buf(), gen: self.gen });
        if old > 0 {
            let _ = self.engine.tx.send(Cmd::Close { gen: old });
        }
    }

    fn on_opened(&mut self, doc: DocInfo) {
        self.loading = false;
        // A rebuild may hand back the same file under another spelling of its path (/tmp or
        // /private/tmp, a "./" in it). It is still the same presentation: its slide, notes and timer stay.
        let same = self.deck_path.as_deref().is_some_and(|d| same_file(d, &doc.path)) && self.doc.is_some();
        if !same {
            self.save_note_draft();
            self.finish_session();
            self.save_side();
            if self.notes_dirty.is_some() {
                self.save_notes();
            }
            self.deck_path = Some(doc.path.clone());
            self.side = Sidecar::load(&doc.path);
            self.notes = notes::load_notes(&doc.path);
            self.note_pending.clear();
            for (k, v) in notes::load_pdfpc(&doc.path.with_extension("pdfpc")) {
                self.notes.entry(k).or_insert(v);
            }
            self.cur = 0;
            self.history.clear();
            self.frozen = None;
            self.zoom = None;
            self.hits.clear();
            self.elapsed = 0.0;
            self.timer_on = false;
            self.timer_touched = false;
        }
        self.pdf_path = Some(doc.path.clone());
        if same {
            // Keep showing the old pictures until the new ones arrive, so a rebuild never flashes.
            for list in self.cache.values_mut() {
                list.sort_by_key(|e| std::cmp::Reverse(e.0));
                list.truncate(1);
                if let Some(e) = list.first_mut() {
                    e.0 = 0;
                }
            }
        } else {
            self.cache.clear();
        }
        self.pending.clear();
        // Frames are made again from the new file, and the slide's animations start afresh.
        self.anim_cache.clear();
        self.anim_pending.clear();
        self.anim_play.clear();
        self.anim_page = None;
        self.doc = Some(doc);
        self.rebuild_deck();
        if let Some(g) = self.goto_on_open.take() {
            self.cur = g.min(self.deck.slides.len().saturating_sub(1));
        }
        self.cur = self.cur.min(self.deck.slides.len().saturating_sub(1));
        if self.demo {
            self.fill_demo();
        }
        if self.left == Some(LeftTab::Search) && !self.search_q.is_empty() {
            self.run_search();
        }
        if self.deck.slides.is_empty() {
            self.status = "This PDF has no pages.".to_string();
        } else if same && self.status.is_empty() {
            self.status = "Slides refreshed.".to_string();
        } else if !same && !self.status.starts_with("Compiled") {
            self.status.clear();
        }
    }

    /// Rehearsal numbers for checking the analytics screen; only used with --demo.
    fn fill_demo(&mut self) {
        let n = self.deck.frames.len();
        let mut s = Session { started_unix: now_unix(), ..Default::default() };
        for f in 0..self.deck.talk_frames() {
            s.visit(f);
            s.tick(f, 28.0 + ((f * 37) % 11) as f32 * 9.0 + if f == 5 { 95.0 } else { 0.0 });
        }
        s.questions.push(5);
        let mut prev = s.clone();
        for t in prev.per_frame.iter_mut() {
            *t *= 0.86;
        }
        prev.total_secs *= 0.86;
        self.session = s;
        self.side.sessions = vec![prev];
        self.side.target_minutes = 8.0;
        self.elapsed = self.session.total_secs as f64;
        self.deck.frames.truncate(n);
    }

    pub fn rebuild_deck(&mut self) {
        let Some(doc) = &self.doc else { return };
        let nav = deck::read_nav(&doc.path);
        let titles: Vec<String> = self.src.as_ref().map(|s| s.frames.iter().map(|f| f.title.clone()).collect()).unwrap_or_default();
        self.deck = deck::build(doc, self.side.notes_mode, &nav, &titles);
    }

    pub fn save_side(&mut self) {
        if let Some(p) = &self.deck_path {
            let s = &self.side;
            let used = !s.ink.is_empty() || !s.sessions.is_empty() || s.target_minutes > 0.0 || !s.bookmarks.is_empty() || !s.questions.is_empty() || !s.section_colors.is_empty();
            // An existing file is rewritten too, so removing the last mark sticks.
            if used || Sidecar::path_for(p).exists() {
                if let Err(e) = self.side.save(p) {
                    self.status = format!("Could not save this deck's data: {e}");
                }
            }
        }
    }

    // ---------- editing notes ----------

    /// Opens the notes of the slide on screen for editing, in the notes panel.
    pub fn begin_note_edit(&mut self) {
        if self.doc.is_none() {
            return;
        }
        if !matches!(self.view, View::Presenter | View::Split) {
            self.view = View::Presenter;
        }
        let f = self.frame();
        let text = self.note_for(f).unwrap_or_default();
        self.note_draft = Some((f, text));
        self.note_focus = true;
    }

    /// Saves the draft into `<presentation>.note.md` beside the presentation. Clearing all the text
    /// brings back the notes from the PDF or the LaTeX source.
    pub fn save_note_draft(&mut self) {
        let Some((f, text)) = self.note_draft.take() else { return };
        // A presentation made from LaTeX keeps its notes in the source, as \note{...} in each frame, so
        // the presenter and the LaTeX studio always show the same notes. The source is saved and rebuilt.
        // With "save in the .tex file" off, it goes into the .note.md file instead, as for a PDF.
        let new_src = if self.st.notes_in_tex { self.src.as_ref().and_then(|s| notes::set_frame_note(&s.text, f, &notes::md_to_latex(&text))) } else { None };
        if let (Some(new), Some(s)) = (new_src, self.src.as_mut()) {
            s.text = new;
            s.dirty = true;
            s.last_edit = Instant::now();
            let file = s.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if self.notes.remove(&f).is_some() {
                self.save_notes();
            }
            self.save_source();
            // The note shows at once; LaTeX rebuilds the slides in the background. (Set after saving,
            // since saving checks the source's notes and would take it back.)
            self.note_pending.insert(f, text);
            if !self.status.starts_with("Could not") {
                self.status = format!("Notes for slide {} saved in {file}, in the slide's \\note. The slides rebuild in the background.", f + 1);
                self.start_compile();
            }
            return;
        }
        self.note_pending.remove(&f);
        if text.trim().is_empty() {
            self.notes.remove(&f);
        } else {
            self.notes.insert(f, text);
        }
        self.save_notes();
        if !self.status.starts_with("Could not") {
            let file = self.deck_path.as_ref().and_then(|p| notes::notes_file(p).file_name().map(|n| n.to_string_lossy().to_string())).unwrap_or_default();
            self.status = format!("Notes for slide {} saved in {file}, beside the presentation.", f + 1);
        }
    }

    pub fn save_notes(&mut self) {
        if let Some(p) = &self.deck_path {
            if let Err(e) = notes::save_notes(p, &self.notes) {
                self.status = format!("Could not save notes: {e}");
            }
        }
        self.notes_dirty = None;
    }

    pub fn save_source(&mut self) {
        if let Some(s) = &mut self.src {
            match std::fs::write(&s.path, &s.text) {
                Ok(_) => {
                    s.dirty = false;
                    s.disk_mtime = latex::mtime(&s.path);
                    let before = std::mem::replace(&mut s.frames, notes::parse_tex_frames(&s.text));
                    self.source_notes_changed(&before);
                }
                Err(e) => self.status = format!("Could not save: {e}"),
            }
        }
    }

    /// After the source changed (in the studio or another editor): a slide whose `\note` changed shows
    /// it, even where an older copy of its note sits in the `.note.md` file. That copy goes.
    fn source_notes_changed(&mut self, before: &[notes::TexFrame]) {
        let Some(s) = &self.src else { return };
        // When frames were added or removed, numbers no longer match; nothing is matched up then.
        if s.frames.len() != before.len() {
            return;
        }
        let changed: Vec<usize> = (0..before.len()).filter(|&i| before[i].note != s.frames[i].note).collect();
        let mut gone = false;
        for i in changed {
            gone |= self.notes.remove(&i).is_some();
            self.note_pending.remove(&i);
        }
        if gone {
            self.save_notes();
        }
    }

    pub fn start_compile(&mut self) {
        let Some(path) = self.src.as_ref().map(|s| s.path.clone()) else {
            self.compile_note = "Open a .tex file to compile.".to_string();
            return;
        };
        if self.compiling {
            self.compile_again = true;
            return;
        }
        self.compiling = true;
        self.compile_note = "Compiling…".to_string();
        self.compile_ok = None;
        self.check_packages();
        latex::compile_async(self.active_id(), path, self.st.engine, self.compile_tx.clone(), || {});
    }

    fn on_compiled(&mut self, r: CompileResult) {
        self.compiling = false;
        self.problems = r.problems;
        self.full_log = r.log;
        self.compile_ok = Some(r.ok);
        if !self.problems.is_empty() {
            self.log_tab = LogTab::Problems;
            self.log_open = true;
        }
        if r.ok {
            let warns = self.problems.len();
            self.compile_note = format!(
                "Compiled with {} in {:.1}s{}",
                r.engine.name(),
                r.seconds,
                match warns {
                    0 => String::new(),
                    1 => ", no errors, 1 warning".to_string(),
                    w => format!(", no errors, {w} warnings"),
                }
            );
            self.status = self.compile_note.clone();
            self.open_after_compile = false;
            self.load_pdf(&r.pdf);
        } else {
            let first = self.problems.iter().find(|p| p.is_error).map(|p| p.message.clone());
            self.compile_note = if !r.note.is_empty() { r.note } else { format!("Could not compile: {}", first.unwrap_or_else(|| "see the log".to_string())) };
            self.status = self.compile_note.clone();
            if self.open_after_compile {
                self.open_after_compile = false;
                self.view = View::Studio;
                // An older PDF is still better than nothing to present.
                if r.pdf.exists() {
                    self.load_pdf(&r.pdf);
                }
            }
        }
        if self.compile_again {
            self.compile_again = false;
            self.start_compile();
        }
    }

    // ---------- every frame ----------

    pub fn pump(&mut self, ctx: &Context) {
        self.tick += 1;
        // The audience window is drawn from this one, and macOS stops drawing a hidden window, as
        // it does behind a fullscreen slide on one screen. A talk must not freeze because of that.
        // This request lasts one frame, so it is sent every frame.
        if self.audience_open || self.shot.is_some() {
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::RequestPaintWhileHidden);
        }
        if !self.script.is_empty() {
            ctx.request_repaint();
        }
        if let Some(id) = self.close_request.take() {
            if self.closing_ends_app(id) {
                ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close);
            } else {
                self.close_tab(id, ctx);
            }
        }
        if let Some((id, msg)) = self.failed_open.take() {
            self.close_tab(id, ctx);
            self.go_home();
            self.status = msg;
        }
        while let Ok(r) = self.engine.rx.try_recv() {
            match r {
                Reply::Opened { gen, doc } => {
                    if gen == self.gen {
                        self.on_opened(doc);
                    } else if let Some(id) = self.tab_of_gen(gen) {
                        self.with_tab(id, |a| a.on_opened(doc));
                    }
                }
                Reply::Frame { req, size, rgba } => {
                    self.anim_pending.remove(&req);
                    if req.gen == self.gen {
                        let img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                        let tex = ctx.load_texture(format!("a{}-{}-{}-{}", req.page, req.anim, req.frame, req.width), img, TextureOptions::LINEAR);
                        let key = (req.page, req.anim, req.frame);
                        if self.anim_cache.get(&key).is_none_or(|(w, _)| *w <= req.width) {
                            self.anim_cache.insert(key, (req.width, tex));
                        }
                    }
                }
                Reply::Image { req, size, rgba } => {
                    self.pending.remove(&req);
                    if req.gen == self.gen {
                        let img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                        let tex = ctx.load_texture(format!("p{}-{}", req.page, req.width), img, TextureOptions::LINEAR);
                        let list = self.cache.entry((req.page, req.region)).or_default();
                        list.retain(|e| e.0 != req.width && e.0 != 0);
                        list.push((req.width, tex, self.tick));
                        self.evict();
                    }
                }
                Reply::Search { gen, query, hits } if gen == self.gen && query == self.search_sent => self.hits = hits,
                Reply::Failed { gen, msg } => {
                    let id = if gen == self.gen { Some(self.active_id()) } else { self.tab_of_gen(gen) };
                    let text = friendly_open_error(&msg);
                    if let Some(id) = id {
                        self.with_tab(id, |a| {
                            a.loading = false;
                            if a.doc.is_none() && a.src.is_none() {
                                a.failed_open = Some((id, text));
                            } else {
                                a.status = text;
                            }
                        });
                    }
                }
                Reply::EngineMissing(m) => self.engine_error = Some(m),
                _ => {}
            }
        }
        while let Ok(r) = self.compile_rx.try_recv() {
            self.with_tab(r.tab, |a| a.on_compiled(r));
        }
        self.animate(ctx);
        if let Some(rx) = &self.update_bg_rx {
            if let Ok(o) = rx.try_recv() {
                // A failed later look (no internet for a while) keeps what an earlier one found.
                let keep = matches!(o, crate::update::Outcome::Failed(_)) && self.update_notice.is_some();
                if !keep {
                    self.update_notice = Some(o);
                }
                self.update_bg_rx = None;
            }
        }
        // Look again every few hours while TeXora stays open, and wake up for it even when idle.
        if let Some(t) = self.update_checked {
            let left = crate::update::CHECK_EVERY.saturating_sub(t.elapsed());
            if left.is_zero() && self.update_bg_rx.is_none() {
                self.check_updates_quietly(ctx);
            } else {
                ctx.request_repaint_after(left);
            }
        }
        if let Some(rx) = &self.req_rx {
            if let Ok((reqs, warn)) = rx.try_recv() {
                self.reqs = reqs;
                self.req_warning = warn;
                self.req_rx = None;
                if self.reqs.iter().any(|r| !r.found) || self.req_warning.is_some() {
                    self.log_tab = LogTab::Packages;
                    self.log_open = true;
                }
            }
        }

        // Timer and analytics
        let now = Instant::now();
        let dt = now.duration_since(self.timer_last).as_secs_f64();
        self.timer_last = now;
        self.tick_background_timers(dt);
        if self.timer_on {
            self.elapsed += dt;
            if let Some(s) = self.deck.slides.get(self.live()) {
                self.session.tick(s.frame, dt as f32);
            }
        }
        if let Some(t) = self.notes_dirty {
            if t.elapsed().as_secs_f32() > 1.0 {
                self.save_notes();
            }
        }

        // Files changed by another program (your editor, or a build elsewhere).
        if self.tex_checked.elapsed().as_secs_f32() > 1.0 {
            self.tex_checked = Instant::now();
            self.watch_files();
        }

        // Laser trail fades out by itself.
        let t = ctx.input(|i| i.time);
        self.laser.retain(|(_, born)| t - born < 0.45);

        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            let p = f.path().to_path_buf();
            self.open_path(&p);
        }
        if !self.laser.is_empty() || self.stroke.is_some() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }

    fn watch_files(&mut self) {
        let auto = self.st.auto_compile;
        let mut rebuild = false;
        let mut changed_frames = None;
        if let Some(s) = &mut self.src {
            let m = latex::mtime(&s.path);
            if m.is_some() && m != s.disk_mtime && !s.dirty {
                s.disk_mtime = m;
                if let Some(t) = notes::read_text(&s.path) {
                    changed_frames = Some(std::mem::replace(&mut s.frames, notes::parse_tex_frames(&t)));
                    s.text = t;
                }
                rebuild = auto;
            }
        }
        if let Some(before) = changed_frames {
            self.source_notes_changed(&before);
        }
        if rebuild {
            self.start_compile();
        }
        if !self.compiling {
            if let Some(p) = self.pdf_path.clone() {
                let m = latex::mtime(&p);
                if m.is_some() && m != self.pdf_mtime {
                    self.load_pdf(&p);
                }
            }
        }
    }

    fn evict(&mut self) {
        let total: usize = self.cache.values().map(Vec::len).sum();
        if total <= 90 {
            return;
        }
        let mut all: Vec<u64> = self.cache.values().flatten().map(|e| e.2).collect();
        all.sort_unstable();
        let cut = all[total - 70];
        for v in self.cache.values_mut() {
            v.retain(|e| e.2 >= cut);
        }
    }

    // ---------- animations in the PDF ----------

    /// Runs the animations of the slide on show. As in Acrobat: a slide's animations that play by
    /// themselves start from the first frame when it is shown, and stop when another slide is shown.
    fn animate(&mut self, ctx: &Context) {
        self.video_slide_changed(ctx);
        let Some(doc) = &self.doc else { return };
        let mut to_play: Vec<crate::media::MediaClip> = vec![];
        let page = self.deck.slides.get(self.live()).map(|s| s.page);
        if page != self.anim_page {
            if let Some(old) = self.anim_page {
                for ((p, _), st) in self.anim_play.iter_mut() {
                    if *p == old {
                        st.playing = false;
                    }
                }
            }
            if let Some(p) = page {
                for (i, a) in doc.pages.get(p).map(|pg| pg.animations.clone()).unwrap_or_default().iter().enumerate() {
                    if a.autoplay {
                        self.anim_play.insert((p, i), AnimPlay::start());
                    }
                }
                // Clips that start by themselves do so during a talk, when the audience sees the slide.
                if self.audience_open && self.shot.is_none() {
                    for m in doc.pages.get(p).map(|pg| pg.media.clone()).unwrap_or_default() {
                        if m.clip.autoplay {
                            to_play.push(m.clip);
                        }
                    }
                }
            }
            self.anim_page = page;
        }
        let now = Instant::now();
        let mut soonest: Option<Instant> = None;
        for ((p, i), st) in self.anim_play.iter_mut() {
            if !st.playing {
                continue;
            }
            let Some(a) = doc.pages.get(*p).and_then(|pg| pg.animations.get(*i)) else { continue };
            let step = Duration::from_secs_f32(1.0 / a.fps.max(0.1));
            // After a long stall (a hidden window), carry on from now instead of racing to catch up.
            if now.duration_since(st.next_at) > Duration::from_secs(1) {
                st.next_at = now;
            }
            while st.playing && now >= st.next_at {
                anim_step(st, a.frames, a.end);
                st.next_at += step;
            }
            if st.playing {
                soonest = Some(soonest.map_or(st.next_at, |s| s.min(st.next_at)));
            }
        }
        if let Some(t) = soonest {
            ctx.request_repaint_after(t.saturating_duration_since(now));
        }
        for clip in to_play {
            self.play_media(&clip);
        }
    }

    /// The best picture of one animation frame, asking for a sharper one (and the other frames) in the background.
    pub fn anim_frame(&mut self, page: usize, anim: usize, frame: usize, frames: usize, page_width: u32) -> Option<TextureHandle> {
        let want = page_width.clamp(64, MAX_RENDER_WIDTH).div_ceil(64) * 64;
        for f in std::iter::once(frame).chain(0..frames) {
            let have = self.anim_cache.get(&(page, anim, f)).map(|(w, _)| *w).unwrap_or(0);
            if have < want {
                let req = pdf::FrameReq { gen: self.gen, page, anim, frame: f, width: want };
                if !self.anim_pending.iter().any(|r| r.page == page && r.anim == anim && r.frame == f && r.width >= want) && self.anim_pending.insert(req) {
                    let _ = self.engine.tx.send(Cmd::Frame(req));
                }
            }
        }
        self.anim_cache.get(&(page, anim, frame)).map(|(_, t)| t.clone())
    }

    /// Opens a clip in the computer's video player.
    pub fn play_media(&mut self, clip: &crate::media::MediaClip) {
        match &clip.file {
            Some(f) if self.shot.is_none() => {
                if let Err(e) = crate::media::open_with_system(f) {
                    self.status = format!("Could not open {}: {e}", clip.name);
                } else {
                    self.status = format!("{} is playing in your video player.", clip.name);
                }
            }
            Some(_) => self.status = format!("Would play {}.", clip.name),
            None => self.status = format!("The clip {} was not found. Keep it in the same folder as the presentation.", clip.name),
        }
    }

    /// Play or pause an animation; a finished one starts again.
    pub fn anim_toggle(&mut self, page: usize, anim: usize) {
        let frames = self.doc.as_ref().and_then(|d| d.pages.get(page)).and_then(|p| p.animations.get(anim)).map(|a| a.frames).unwrap_or(0);
        let st = self.anim_play.entry((page, anim)).or_insert(AnimPlay { frame: 0, playing: false, forward: true, next_at: Instant::now() });
        if st.playing {
            st.playing = false;
        } else {
            if st.forward && st.frame + 1 >= frames {
                st.frame = 0;
            }
            st.playing = true;
            st.next_at = Instant::now() + Duration::from_millis(1);
        }
    }

    /// Show frame `frame` of an animation, paused there.
    pub fn anim_seek(&mut self, page: usize, anim: usize, frame: usize) {
        let st = self.anim_play.entry((page, anim)).or_insert(AnimPlay { frame: 0, playing: false, forward: true, next_at: Instant::now() });
        st.frame = frame;
        st.playing = false;
    }

    /// Best picture we have now; asks for the exact size in the background.
    pub fn texture(&mut self, page: usize, region: Region, want: u32) -> Option<TextureHandle> {
        // Sizes come in steps, and a page is never rendered larger than is useful, so zooming
        // reuses pictures instead of queueing a new huge one for every small change.
        let want = match want {
            0..=1600 => want.max(64).div_ceil(32) * 32,
            1601..=2000 => 2000,
            2001..=2500 => 2500,
            _ => MAX_RENDER_WIDTH,
        };
        let settling = self.zoom_changed.elapsed().as_secs_f32() < 0.18;
        let tick = self.tick;
        let list = self.cache.entry((page, region)).or_default();
        if let Some(e) = list.iter_mut().find(|e| e.0 == want) {
            e.2 = tick;
            return Some(e.1.clone());
        }
        let req = RenderReq { gen: self.gen, page, region, width: want };
        // While the wheel is turning, stretch what we have; sharpen once it stops.
        let have_any = !list.is_empty();
        if !(settling && have_any) && self.pending.insert(req) {
            let _ = self.engine.tx.send(Cmd::Render(req));
        }
        list.iter_mut().max_by_key(|e| e.0).map(|e| {
            e.2 = tick;
            e.1.clone()
        })
    }

    // ---------- navigation ----------

    pub fn live(&self) -> usize {
        self.frozen.unwrap_or(self.cur)
    }

    pub fn frame(&self) -> usize {
        self.deck.slides.get(self.cur).map(|s| s.frame).unwrap_or(0)
    }

    pub fn goto(&mut self, slide: usize) {
        let slide = slide.min(self.deck.slides.len().saturating_sub(1));
        if slide == self.cur {
            return;
        }
        let before = self.frame();
        self.cur = slide;
        self.next_zoom = None;
        // Zoom stays while the steps of one slide appear, and ends on a new slide.
        if self.frame() != before {
            self.zoom = None;
            self.notes_zoom = None;
            self.stroke = None;
            let f = self.frame();
            if self.timer_on && self.frozen.is_none() {
                self.session.visit(f);
            }
        }
        let presenting = matches!(self.view, View::Presenter | View::Split);
        if !self.timer_on && !self.timer_touched && self.frozen.is_none() && presenting {
            self.start_timer();
        }
        self.sync_editor_to_slide();
    }

    /// A deliberate jump, remembered so Back can return from it.
    pub fn jump(&mut self, slide: usize) {
        self.history.push(self.cur);
        self.goto(slide);
    }

    pub fn start_timer(&mut self) {
        self.timer_touched = true;
        if !self.timer_on {
            self.timer_on = true;
            self.timer_last = Instant::now();
            if self.session.total_secs == 0.0 {
                self.session.started_unix = now_unix();
                self.session.visit(self.frame());
            }
        }
    }

    pub fn finish_session(&mut self) {
        if self.session.total_secs > 5.0 {
            self.side.sessions.push(std::mem::take(&mut self.session));
            self.side.sessions.reverse();
            self.side.sessions.truncate(30);
            self.side.sessions.reverse();
        }
        self.session = Session { started_unix: now_unix(), ..Default::default() };
    }

    fn sync_editor_to_slide(&mut self) {
        // Keep the source in view while moving through slides in the studio.
        if self.view != View::Studio {
            return;
        }
        let f = self.frame();
        if let Some(s) = &mut self.src {
            if let Some(tf) = s.frames.get(f) {
                let line = tf.line;
                if !(tf.line..=tf.end_line).contains(&s.cursor_line) {
                    s.goto_line = Some(line);
                    s.synced_line = line;
                }
            }
        }
    }

    pub fn follow_link(&mut self, t: &LinkTarget) {
        match t {
            LinkTarget::Page(p) => {
                if let Some(s) = self.deck.slide_for_page(*p) {
                    self.jump(s);
                }
            }
            LinkTarget::Uri(u) => self.open_external(u),
        }
    }

    /// Web links only; anything else in a PDF link is ignored.
    pub fn open_external(&mut self, url: &str) {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            self.status = "That link is not a web address, so it was not opened.".to_string();
            return;
        }
        #[cfg(target_os = "macos")]
        let r = std::process::Command::new("open").arg(url).spawn();
        #[cfg(target_os = "windows")]
        let r = std::process::Command::new("explorer").arg(url).spawn();
        #[cfg(all(unix, not(target_os = "macos")))]
        let r = std::process::Command::new("xdg-open").arg(url).spawn();
        if r.is_err() {
            self.status = format!("Could not open {url}");
        }
    }

    // ---------- source sync ----------

    pub fn sync_from_cursor(&mut self) {
        let (Some(s), Some(pdf)) = (&mut self.src, &self.pdf_path) else { return };
        let line = s.cursor_line;
        s.synced_line = line;
        let path = s.path.clone();
        let frame_at = s.frames.iter().position(|f| (f.line..=f.end_line).contains(&line));
        let frame_end = frame_at.map(|i| s.frames[i].end_line);
        let current_page = self.deck.slides.get(self.cur).map(|x| x.page);
        let deck = &self.deck;
        let frame_of = |p: usize| deck.slides.iter().find(|x| x.page == p).map(|x| x.frame);
        let on_line = latex::forward_sync(&path, line, pdf);
        // Inside a frame, its \end{frame} line says which slide it is: SyncTeX links the frame's
        // opening lines to the slide before. The cursor's own line then picks the overlay step.
        let chosen = match frame_end {
            Some(end) => {
                let whole = latex::forward_sync(&path, end, pdf);
                pick_sync_in_frame(&on_line, &whole, frame_of, current_page).cloned()
            }
            None => pick_sync(&on_line, frame_of, current_page).cloned(),
        };
        if let Some(t) = chosen {
            if let Some(i) = self.deck.slides.iter().position(|x| x.page == t.page) {
                self.cur = i;
                let (w, h) = self.page_points(t.page);
                self.sync_mark = Some((i, [t.x / w, t.y / h], Instant::now()));
                return;
            }
        }
        if let Some(fr) = frame_at.and_then(|f| self.deck.frames.get(f)) {
            self.cur = fr.first;
        }
    }

    /// Slide width and height in PDF points.
    fn page_points(&self, page: usize) -> (f32, f32) {
        let Some(p) = self.doc.as_ref().and_then(|d| d.pages.get(page)) else { return (1.0, 1.0) };
        let wide = self.deck.slides.first().map(|s| s.region != Region::Full).unwrap_or(false);
        (if wide { p.width / 2.0 } else { p.width }, p.height)
    }

    /// `pos` is a position on the slide from 0 to 1.
    pub fn sync_from_slide(&mut self, idx: usize, pos: [f32; 2]) {
        let Some(sl) = self.deck.slides.get(idx) else { return };
        let (page, region) = (sl.page, sl.region);
        let (w, h) = self.page_points(page);
        let x = pos[0] * w + if region == Region::Right { w } else { 0.0 };
        let Some(pdf) = self.pdf_path.clone() else { return };
        match latex::inverse_sync(page, x, pos[1] * h, &pdf) {
            Some((file, line)) => {
                let same = self.src.as_ref().map(|s| s.path == file).unwrap_or(false);
                if !same && file.exists() {
                    self.src = self.read_source(&file);
                }
                self.view = View::Studio;
                if let Some(s) = &mut self.src {
                    s.goto_line = Some(line);
                    s.cursor_line = line;
                    s.synced_line = line;
                }
            }
            None => self.status = "No source position here. Compile from TeXora so it can link slides to source.".to_string(),
        }
    }


    // ---------- ink ----------

    pub fn ink_key(&self) -> String {
        let f = self.deck.slides.get(self.live()).map(|s| s.frame).unwrap_or(0);
        if self.screen == Screen::White {
            format!("board{f}")
        } else {
            f.to_string()
        }
    }

    pub fn begin_stroke(&mut self, p: [f32; 2]) {
        let c = self.st.ink_color;
        self.stroke = Some(match self.tool {
            Tool::Highlighter => Stroke { kind: InkKind::Highlighter, color: c, width: 0.03, points: vec![p] },
            _ => Stroke { kind: InkKind::Pen, color: c, width: 0.004, points: vec![p] },
        });
    }

    pub fn end_stroke(&mut self) {
        if let Some(s) = self.stroke.take() {
            let k = self.ink_key();
            self.side.ink.entry(k).or_default().push(s);
            self.session.ink_strokes += 1;
        }
    }

    pub fn erase_at(&mut self, p: [f32; 2]) {
        let k = self.ink_key();
        if let Some(list) = self.side.ink.get_mut(&k) {
            list.retain(|s| !s.points.iter().any(|q| (q[0] - p[0]).hypot(q[1] - p[1]) < 0.015));
        }
    }

    pub fn undo_ink(&mut self) {
        let k = self.ink_key();
        if let Some(list) = self.side.ink.get_mut(&k) {
            list.pop();
        }
    }

    pub fn clear_ink(&mut self) {
        let k = self.ink_key();
        self.side.ink.remove(&k);
    }

    // ---------- editing notes ----------

    pub fn save_layout(&self) {
        if self.shot.is_none() {
            let _ = self.st.save();
        }
    }

    pub fn go_to_slide(&mut self, text: &str) {
        let total = self.deck.frames.len();
        match text.trim().parse::<usize>() {
            Ok(n) if n >= 1 && n <= total => {
                let t = self.deck.frames[n - 1].first;
                self.jump(t);
            }
            _ => self.status = format!("Enter a slide number from 1 to {total}."),
        }
    }

    pub fn set_tool(&mut self, t: Tool) {
        self.tool = if self.tool == t { Tool::Pointer } else { t };
        // Drawing happens on a large slide, which the split view already has.
        if self.tool != Tool::Pointer && self.view == View::Presenter {
            self.swap = true;
        }
    }

    // ---------- zoom ----------

    pub fn zoom_level(&self) -> f32 {
        self.zoom.map(|z| 1.0 / (z[2] - z[0])).unwrap_or(1.0)
    }

    /// Zooms by `factor`, keeping the slide point under `anchor` where it is.
    pub fn zoom_by(&mut self, factor: f32, anchor: [f32; 2]) {
        self.zoom = zoomed(self.zoom, factor, anchor);
        self.zoom_changed = Instant::now();
    }

    /// Button and keyboard zoom, centred on what is on screen now.
    pub fn zoom_step(&mut self, factor: f32) {
        let z = self.zoom.unwrap_or([0.0, 0.0, 1.0, 1.0]);
        self.zoom_by(factor, [(z[0] + z[2]) / 2.0, (z[1] + z[3]) / 2.0]);
    }

    /// Moves the zoomed view; `by` is in slide units.
    pub fn zoom_pan(&mut self, by: [f32; 2]) {
        if let Some(z) = &mut self.zoom {
            let size = z[2] - z[0];
            z[0] = (z[0] + by[0]).clamp(0.0, 1.0 - size);
            z[1] = (z[1] + by[1]).clamp(0.0, 1.0 - size);
            z[2] = z[0] + size;
            z[3] = z[1] + size;
        }
    }

    // ---------- timer ----------

    pub fn pause_timer(&mut self) {
        self.timer_touched = true;
        self.timer_on = false;
    }

    /// Stops, saves this run, and sets the clock back to zero.
    pub fn stop_timer(&mut self) {
        self.timer_touched = true;
        self.timer_on = false;
        self.elapsed = 0.0;
        self.finish_session();
        self.save_side();
    }

    // ---------- sidebars, files and packages ----------

    fn toggle_left(&mut self, tab: LeftTab) {
        self.left = if self.left == Some(tab) { None } else { Some(tab) };
        if self.left == Some(LeftTab::Search) && !self.search_q.is_empty() {
            self.run_search();
        }
    }

    /// At launch, and every few hours after: looks in the background. If something newer exists, a
    /// notice appears by itself in the corner and a dot marks the ? button; otherwise nothing shows.
    fn check_updates_quietly(&mut self, ctx: &Context) {
        let url = crate::update::update_url();
        // Test pictures never reach the internet unless a test asks for it.
        let test_run = self.shot.is_some() && std::env::var_os("TEXORA_UPDATE_URL").is_none();
        if url.is_empty() || !self.st.check_updates || test_run {
            return;
        }
        let (tx, rx) = channel();
        self.update_bg_rx = Some(rx);
        self.update_checked = Some(Instant::now());
        let wake = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(crate::update::check(&url));
            wake.request_repaint();
        });
    }

    pub fn check_updates(&mut self, ctx: &Context) {
        self.update_open = true;
        self.update_result = None;
        let (tx, rx) = channel();
        self.update_rx = Some(rx);
        let wake = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(crate::update::check(&crate::update::update_url()));
            wake.request_repaint();
        });
    }

    pub fn open_dialog(&mut self, label: &str, exts: &[&str]) {
        if let Some(p) = rfd::FileDialog::new().add_filter(label, exts).pick_file() {
            self.open_path(&p);
        }
    }

    /// Opens the template picker, on the template chosen last time.
    pub fn new_presentation_dialog(&mut self) {
        self.template_picker = Some(crate::templates::by_id(&self.st.last_template));
    }

    /// Asks where to save, then creates the presentation from template `t`.
    pub fn save_new_presentation(&mut self, t: usize) {
        let dialog = rfd::FileDialog::new().add_filter("LaTeX source", &["tex"]).set_file_name("presentation.tex");
        if let Some(p) = dialog.save_file() {
            self.create_presentation(&p, t);
        }
    }

    /// Writes template `t` to `path` and opens it in the studio.
    pub fn create_presentation(&mut self, path: &Path, t: usize) {
        let path = if path.extension().is_some() { path.to_path_buf() } else { path.with_extension("tex") };
        let template = &crate::templates::TEMPLATES[t.min(crate::templates::TEMPLATES.len() - 1)];
        match std::fs::write(&path, template.tex) {
            Ok(_) => {
                // Pictures and clips of an animated template go into a media folder beside it.
                let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
                let failed = crate::templates::write_files(&dir, template);
                self.st.last_template = template.id.to_string();
                let _ = self.st.save();
                self.open_path(&path);
                self.view = View::Studio;
                if !failed.is_empty() {
                    self.status = format!("Could not save {} beside the presentation.", failed.join(", "));
                }
            }
            Err(e) => self.status = format!("Could not create that file: {e}"),
        }
    }

    /// The comment shortcuts, on the line or lines selected in the studio's editor.
    fn comment_selection(&mut self, mode: crate::ui_studio::CommentMode, ctx: &Context) {
        if self.view != View::Studio {
            return;
        }
        let Some(s) = self.src.as_mut() else { return };
        let id = crate::ui_present::sid("tex-editor");
        let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
        let (cursor, anchor) = state.cursor.char_range().map(|r| (r.primary.index.0, r.secondary.index.0)).unwrap_or((0, 0));
        let (text, (from, to)) = crate::ui_studio::comment_lines(&s.text, (anchor, cursor), mode);
        if text == s.text {
            return;
        }
        s.text = text;
        s.dirty = true;
        s.last_edit = Instant::now();
        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(from), egui::text::CCursor::new(to))));
        state.store(ctx, id);
        ctx.memory_mut(|m| m.request_focus(id));
    }

    /// Looks up everything the source needs, in the background.
    pub fn check_packages(&mut self) {
        let Some(s) = &self.src else {
            self.reqs.clear();
            self.req_warning = None;
            return;
        };
        let (text, engine) = (s.text.clone(), self.st.engine);
        let dir = s.path.parent().map(Path::to_path_buf).unwrap_or_default();
        let (tx, rx) = channel();
        self.req_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(latex::check_requirements(&text, engine, &dir));
        });
    }

    // ---------- commands ----------

    pub fn run(&mut self, a: Action, ctx: &Context) {
        use Action::*;
        // Home has no slides, so only the actions that make sense there run.
        if self.view == View::Start && !matches!(a, Open | Settings | StartPage | CloseTab | Cancel) {
            return;
        }
        match a {
            Next => self.goto(self.cur + 1),
            Prev => self.goto(self.cur.saturating_sub(1)),
            NextFrame => {
                let f = self.frame();
                if let Some(fr) = self.deck.frames.get(f + 1) {
                    self.goto(fr.first);
                }
            }
            PrevFrame => {
                let f = self.frame();
                if f > 0 {
                    self.goto(self.deck.frames[f - 1].first);
                }
            }
            First => self.jump(0),
            Last => self.jump(self.deck.slides.len().saturating_sub(1)),
            Back => {
                if let Some(s) = self.history.pop() {
                    self.goto(s);
                }
            }
            Jump => self.focus_jump = true,
            ToggleTimer => {
                if self.timer_on {
                    self.pause_timer();
                } else {
                    self.start_timer();
                }
            }
            ResetTimer => self.stop_timer(),
            Present => {
                if self.audience_open {
                    self.close_audience(ctx);
                } else {
                    self.audience_open = true;
                    self.audience_tab = self.active_id();
                    self.audience_age = 0;
                    self.audience_full = false;
                    self.start_timer();
                    if self.displays.len() < 2 {
                        self.status = "One display found. Press F11 for fullscreen slides.".to_string();
                    }
                }
            }
            Fullscreen => {
                if self.audience_open {
                    self.audience_full = !self.audience_full;
                    ctx.send_viewport_cmd_to(audience_id(), ViewportCommand::Fullscreen(self.audience_full));
                } else {
                    let on = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Fullscreen(!on));
                }
            }
            Blackout => self.screen = if self.screen == Screen::Black { Screen::Live } else { Screen::Black },
            Whiteboard => {
                self.screen = if self.screen == Screen::White { Screen::Live } else { Screen::White };
                if self.screen == Screen::White {
                    self.swap = true;
                }
            }
            Freeze => {
                self.frozen = match self.frozen {
                    Some(_) => None,
                    None => Some(self.cur),
                }
            }
            Overview => self.view = if self.view == View::Overview { View::Presenter } else { View::Overview },
            SplitView => self.view = if self.view == View::Split { View::Presenter } else { View::Split },
            StartPage => self.go_home(),
            CloseTab => {
                self.close_request = Some(self.active_id());
                ctx.request_repaint();
            }
            AddBookmark => self.toggle_bookmark(),
            SetDuration => self.duration_open = !self.duration_open,
            ZoomIn => self.zoom_step(1.25),
            ZoomOut => self.zoom_step(0.8),
            ZoomFit => {
                self.zoom = None;
                self.notes_zoom = None;
            }
            EditNotes => self.begin_note_edit(),
            CheckPackages => {
                // The list opens in the studio's message panel, beside the source.
                self.check_packages();
                self.log_tab = LogTab::Packages;
                self.log_open = true;
                if self.view != View::Studio {
                    self.view = View::Studio;
                    self.sync_editor_to_slide();
                }
            }
            ToggleComment | Comment | Uncomment => {
                let mode = match a {
                    Comment => crate::ui_studio::CommentMode::Comment,
                    Uncomment => crate::ui_studio::CommentMode::Uncomment,
                    _ => crate::ui_studio::CommentMode::Toggle,
                };
                self.comment_selection(mode, ctx);
            }
            Studio => {
                self.view = if self.view == View::Studio { View::Presenter } else { View::Studio };
                self.sync_editor_to_slide();
            }
            SwapNotes => self.swap = !self.swap,
            Outline => self.toggle_left(LeftTab::Bookmarks),
            Search => {
                self.toggle_left(LeftTab::Search);
                self.focus_search = true;
            }
            Analytics => self.toggle_left(LeftTab::Analytics),
            Settings => self.toggle_left(LeftTab::Settings),
            Pointer => self.tool = crate::app::Tool::Pointer,
            Pen => self.set_tool(crate::app::Tool::Pen),
            Highlighter => self.set_tool(crate::app::Tool::Highlighter),
            Eraser => self.set_tool(crate::app::Tool::Eraser),
            Laser => self.set_tool(crate::app::Tool::Laser),
            ZoomTool => self.set_tool(crate::app::Tool::Zoom),
            Undo => self.undo_ink(),
            ClearInk => self.clear_ink(),
            MarkQuestion => self.toggle_question(),
            Open => self.open_dialog("Slides", &["pdf", "tex"]),
            Save => {
                self.save_source();
                if self.st.auto_compile {
                    self.start_compile();
                }
            }
            Compile => {
                self.save_source();
                self.start_compile();
            }
            Cancel => self.cancel(ctx),
        }
    }

    /// Escape steps back one thing at a time, and never ends a talk by itself.
    fn cancel(&mut self, ctx: &Context) {
        if self.capture.take().is_some() {
            return;
        }
        if self.about_open {
            self.about_open = false;
        } else if self.update_open {
            self.update_open = false;
        } else if self.note_draft.is_some() {
            self.note_draft = None;
        } else if self.duration_open {
            self.duration_open = false;
        } else if self.zoom.is_some() {
            self.zoom = None;
        } else if self.tool != Tool::Pointer {
            self.tool = Tool::Pointer;
        } else if self.left.is_some() {
            self.left = None;
        } else if !matches!(self.view, View::Presenter | View::Start) {
            self.view = View::Presenter;
        } else if self.audience_full {
            self.audience_full = false;
            ctx.send_viewport_cmd_to(audience_id(), ViewportCommand::Fullscreen(false));
        }
    }

    pub fn close_audience(&mut self, ctx: &Context) {
        self.audience_open = false;
        self.audience_full = false;
        ctx.send_viewport_cmd_to(audience_id(), ViewportCommand::Close);
    }

    pub fn keys(&mut self, ctx: &Context) {
        if self.capture.is_some() {
            self.capture_key(ctx);
            return;
        }
        let typing = ctx.egui_wants_keyboard_input();
        let mut binds: Vec<(Action, KeyBind)> =
            self.st.keys.iter().flat_map(|(a, ks)| ks.iter().map(move |k| (*a, k.clone()))).collect();
        // Most specific first, so Shift+Right is not taken by plain Right.
        binds.sort_by_key(|(_, k)| std::cmp::Reverse(k.cmd as u8 + k.shift as u8 + k.alt as u8));
        for (a, k) in binds {
            if typing && !a.works_while_typing() {
                continue;
            }
            let Some(key) = egui::Key::from_name(&k.key) else { continue };
            if ctx.input_mut(|i| i.consume_key(k.modifiers(), key)) {
                self.run(a, ctx);
            }
        }
        if self.view == View::Overview && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.view = View::Presenter;
        }
    }

    fn capture_key(&mut self, ctx: &Context) {
        let got = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            })
        });
        if let (Some((key, m)), Some(action)) = (got, self.capture) {
            self.capture = None;
            if key == egui::Key::Escape {
                return;
            }
            let bind = KeyBind { key: key.name().to_string(), cmd: m.command, shift: m.shift, alt: m.alt };
            if let Some(other) = self.st.conflict(&bind, action) {
                self.status = format!("{} was used by \"{}\", which now has no such key.", bind.text(), other.label());
                for (a, ks) in self.st.keys.iter_mut() {
                    if *a == other {
                        ks.retain(|x| *x != bind);
                    }
                }
            }
            if let Some((_, ks)) = self.st.keys.iter_mut().find(|(a, _)| *a == action) {
                *ks = vec![bind];
            }
            let _ = self.st.save();
        }
    }

    // ---------- search ----------

    pub fn run_search(&mut self) {
        self.search_sent = self.search_q.clone();
        let _ = self.engine.tx.send(Cmd::Search { gen: self.gen, query: self.search_q.clone() });
    }

    // ---------- notes ----------

    pub fn note_for(&self, frame: usize) -> Option<String> {
        // A note just written into the source wins, even when it is empty (a note taken away).
        if let Some(t) = self.note_pending.get(&frame) {
            return Some(t.clone()).filter(|t| !t.trim().is_empty());
        }
        self.notes
            .get(&frame)
            .filter(|n| !n.trim().is_empty())
            .cloned()
            .or_else(|| self.src.as_ref().and_then(|s| s.frames.get(frame)).map(|f| f.note.clone()).filter(|n| !n.is_empty()))
    }

    // ---------- screenshot option ----------

    pub fn screenshot_tick(&mut self, ctx: &Context) {
        let Some(path) = self.shot.clone() else { return };
        // A test run leaves the real clipboard alone and notes what would have been copied.
        ctx.output_mut(|o| {
            o.commands.retain(|c| match c {
                egui::OutputCommand::CopyText(t) => {
                    self.test_clip = Some(t.clone());
                    false
                }
                _ => true,
            })
        });
        self.frames_drawn += 1;
        ctx.request_repaint();
        let settled = self.src.as_ref().map(|s| !s.dirty).unwrap_or(true);
        let on_start = self.view == View::Start;
        let ready = (self.doc.is_some() || on_start) && self.pending.is_empty() && !self.compiling && self.script.is_empty() && settled && self.script_until.map(|t| Instant::now() >= t).unwrap_or(true);
        // A long test script runs to its end; TEXORA_SHOT_LIMIT still stops one that hangs.
        let waited = self.script_until.map(|t| Instant::now() >= t).unwrap_or(true);
        let give_up = self.frames_drawn >= 1200 && self.script.is_empty() && waited;
        if !self.shot_sent && ((self.frames_drawn >= 120 && ready) || give_up) {
            self.shot_sent = true;
            let done = self.shot_done.clone();
            let wake = ctx.clone();
            ctx.request_screenshot(move |img| {
                let [w, h] = img.size;
                let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(d) = path.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                match pdf::encode_png(w as u32, h as u32, &bytes) {
                    Some(png) => {
                        if let Err(e) = std::fs::write(&path, png) {
                            eprintln!("could not write screenshot: {e}");
                        }
                    }
                    None => eprintln!("could not encode screenshot"),
                }
                done.store(true, Ordering::SeqCst);
                wake.request_repaint();
            });
        }
        if self.shot_done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close);
        }
    }

    /// Feeds one scripted input step into the app, exactly as if it came from the keyboard or mouse.
    pub fn script_input(&mut self, raw: &mut egui::RawInput) {
        let waiting = self.doc.is_none() && !matches!(self.view, View::Start | View::Studio);
        if waiting || self.script.is_empty() {
            return;
        }
        if self.script_until.map(|t| Instant::now() < t).unwrap_or(false) {
            return;
        }
        self.script_wait += 1;
        if self.script_wait < 5 {
            return;
        }
        self.script_wait = 0;
        let Some(step) = self.script.pop_front() else { return };
        let Some((kind, arg)) = step.split_once(':') else { return };
        if kind == "w" {
            self.script_until = Some(Instant::now() + std::time::Duration::from_millis(arg.parse().unwrap_or(0)));
            return;
        }
        let pos = || {
            let mut it = arg.split(',').filter_map(|v| v.parse::<f32>().ok());
            egui::pos2(it.next().unwrap_or(0.0), it.next().unwrap_or(0.0))
        };
        let button = |pressed| egui::Event::PointerButton {
            pos: pos(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        match kind {
            "k" | "K" => {
                let modifiers = if kind == "K" { egui::Modifiers::COMMAND } else { egui::Modifiers::NONE };
                if let Some(key) = egui::Key::from_name(arg) {
                    for pressed in [true, false] {
                        raw.events.push(egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
                    }
                }
            }
            "t" => raw.events.push(egui::Event::Text(arg.to_string())),
            // Shift with a key (to select), and the clipboard: copy, cut, and paste of the given text.
            "S" => {
                if let Some(key) = egui::Key::from_name(arg) {
                    for pressed in [true, false] {
                        raw.events.push(egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: egui::Modifiers::SHIFT });
                    }
                }
            }
            "c" => raw.events.push(egui::Event::Copy),
            "x" => raw.events.push(egui::Event::Cut),
            "p" => raw.events.push(egui::Event::Paste(arg.replace("\\n", "\n"))),
            "z" => {
                let dy = arg.parse().unwrap_or(0.0);
                raw.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, dy),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            "m" => raw.events.push(egui::Event::PointerMoved(pos())),
            "d" => {
                raw.events.push(egui::Event::PointerMoved(pos()));
                raw.events.push(button(true));
            }
            "u" => {
                raw.events.push(egui::Event::PointerMoved(pos()));
                raw.events.push(button(false));
            }
            _ => {}
        }
    }

    /// What a test script needs to check once it has finished.
    pub fn script_report(&self) -> String {
        let ink: usize = self.side.ink.values().map(Vec::len).sum();
        format!(
            "slide={} frame={} step={} tool={:?} screen={:?} frozen={:?} zoom={} ink={} laser={} timer={} audience={} swap={} history={} view={:?} questions={} slides={} tabs={:?} active={} bookmarks={:?} asked={:?} about={} update={:?} notice={:?} saved={:?} layout={:?} level={:.2} elapsed={:.1} target={} left={:?} missing={} warn={} status={:?} note={:?} cli={} copied={:?} syscheck={} packages={} recent={} toast={} skipped={:?} draft={:?} line={} log_open={} wrap={} follow={} anims={:?} videos={:?} clip={:?} avsync={:?}",
            self.cur + 1,
            self.frame() + 1,
            self.deck.slides.get(self.cur).map(|s| s.step + 1).unwrap_or(0),
            self.tool,
            self.screen,
            self.frozen,
            self.zoom.is_some(),
            ink,
            self.laser.len(),
            self.timer_on,
            self.audience_open,
            self.swap,
            self.history.len(),
            self.view,
            self.session.questions.len(),
            self.deck.slides.len(),
            (0..self.tabs.len()).map(|i| self.tab_title(i)).collect::<Vec<_>>(),
            self.active,
            self.side.bookmarks,
            self.side.questions,
            self.about_open,
            self.update_result,
            self.update_notice,
            self.notes.get(&self.frame()),
            self.st.layout,
            self.zoom_level(),
            self.elapsed,
            self.side.target_minutes,
            self.left,
            self.reqs.iter().filter(|r| !r.found).map(|r| r.name.clone()).collect::<Vec<_>>().join(","),
            self.req_warning.is_some(),
            self.status,
            self.compile_note,
            self.cli_open,
            self.cli_copied,
            self.syscheck_open,
            self.show_packages,
            self.st.recent.len(),
            self.update_toast,
            self.st.skipped_update,
            self.note_draft,
            self.src.as_ref().map(|s| s.cursor_line).unwrap_or(0),
            self.log_open,
            self.log_wrap,
            self.follow_cursor,
            {
                let mut v: Vec<_> = self.anim_play.iter().map(|((p, i), a)| (*p, *i, a.frame, a.playing)).collect();
                v.sort();
                v
            },
            // Each clip's player: slide, file name, playing, seconds in, error.
            self.players
                .iter()
                .map(|p| {
                    let s = p.shared.lock().map(|s| (s.playing, (s.at * 10.0).round() / 10.0, if s.sound { None } else { Some("silent".to_string()) }.or(s.error.clone()))).unwrap_or_default();
                    (p.frame_no, p.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), s.0, s.1, s.2)
                })
                .collect::<Vec<_>>(),
            self.test_clip,
            // Each clip with sound: name, frames measured, mean and worst picture-minus-sound in ms.
            self.players
                .iter()
                .filter_map(|p| {
                    let s = p.shared.lock().ok()?;
                    (s.sync.frames > 0).then(|| (p.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), s.sync.frames, s.sync.mean_ms().round(), s.sync.worst_ms.round()))
                })
                .collect::<Vec<_>>(),
        )
    }

    pub fn is_screenshot(&self) -> bool {
        self.shot.is_some()
    }

    pub fn on_exit(&mut self) {
        if self.is_screenshot() {
            println!("{}", self.script_report());
            return;
        }
        for i in 0..self.tabs.len() {
            self.activate(i);
            self.save_open_work();
        }
        let _ = self.st.save();
    }
}

/// What to tell someone when a PDF will not open.
pub fn friendly_open_error(detail: &str) -> String {
    let d = detail.to_lowercase();
    if d.contains("password") {
        "That PDF is protected by a password, which TeXora cannot open yet.".to_string()
    } else if d.contains("no such file") || d.contains("not found") {
        "That file could not be found. It may have been moved or renamed.".to_string()
    } else {
        "That file could not be opened. It may be damaged, or not really a PDF.".to_string()
    }
}

/// The view after zooming: always square in slide units, and never past the slide's edges.
pub fn zoomed(zoom: Option<[f32; 4]>, factor: f32, anchor: [f32; 2]) -> Option<[f32; 4]> {
    let z = zoom.unwrap_or([0.0, 0.0, 1.0, 1.0]);
    let old = z[2] - z[0];
    let level = (factor / old).clamp(1.0, MAX_ZOOM);
    if level <= 1.001 {
        return None;
    }
    let size = 1.0 / level;
    let fx = ((anchor[0] - z[0]) / old).clamp(0.0, 1.0);
    let fy = ((anchor[1] - z[1]) / old).clamp(0.0, 1.0);
    let x = (anchor[0] - fx * size).clamp(0.0, 1.0 - size);
    let y = (anchor[1] - fy * size).clamp(0.0, 1.0 - size);
    Some([x, y, x + size, y + size])
}

pub fn audience_id() -> ViewportId {
    ViewportId::from_hash_of("texora-audience")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_errors_are_plain_words() {
        assert!(friendly_open_error("PdfiumLibraryInternalError(\n FormatError,\n)").contains("damaged"));
        assert!(friendly_open_error("PasswordError").contains("password"));
        assert!(friendly_open_error("No such file or directory (os error 2)").contains("moved"));
    }

    #[test]
    fn zoom_in_and_back_out_returns_to_fit() {
        let mut z = None;
        for _ in 0..6 {
            z = zoomed(z, 1.25, [0.3, 0.7]);
        }
        assert!(z.is_some());
        for _ in 0..12 {
            z = zoomed(z, 0.8, [0.9, 0.1]);
        }
        assert_eq!(z, None, "zooming out must always reach the full slide");
    }

    #[test]
    fn zoom_keeps_the_point_under_the_pointer() {
        let z = zoomed(None, 2.0, [0.5, 0.5]).unwrap();
        assert!((z[0] - 0.25).abs() < 1e-5 && (z[2] - 0.75).abs() < 1e-5);
        let z = zoomed(Some(z), 2.0, [0.25, 0.25]).unwrap();
        assert!((z[0] - 0.25).abs() < 1e-5, "{z:?}");
    }

    #[test]
    fn zoom_stays_on_the_slide_and_has_a_limit() {
        let mut z = None;
        for _ in 0..40 {
            z = zoomed(z, 1.3, [0.99, 0.01]);
        }
        let z = z.unwrap();
        assert!(z[0] >= 0.0 && z[1] >= 0.0 && z[2] <= 1.0001 && z[3] <= 1.0001, "{z:?}");
        assert!((1.0 / (z[2] - z[0]) - MAX_ZOOM).abs() < 0.01);
    }
}

/// Chooses the place a source line belongs to among everything SyncTeX links it to. SyncTeX lists a
/// few boxes on the page before a frame (where TeX stood when the frame began) ahead of the frame's own,
/// so the first answer is often the previous slide. The slide whose pages hold most of the line's
/// boxes is the right one; within it, the slide on screen stays if it shows the line, and otherwise
/// the first overlay step that does.
pub fn pick_sync(found: &[latex::SyncTarget], frame_of: impl Fn(usize) -> Option<usize>, current_page: Option<usize>) -> Option<&latex::SyncTarget> {
    let mut counts: HashMap<usize, usize> = HashMap::new();
    for t in found {
        if let Some(f) = frame_of(t.page) {
            *counts.entry(f).or_default() += 1;
        }
    }
    let current_frame = current_page.and_then(&frame_of);
    let best = counts.iter().max_by(|a, b| a.1.cmp(b.1).then((Some(*a.0) == current_frame).cmp(&(Some(*b.0) == current_frame))).then(b.0.cmp(a.0)))?.0;
    let mine: Vec<&latex::SyncTarget> = found.iter().filter(|t| frame_of(t.page) == Some(*best)).collect();
    mine.iter().find(|t| Some(t.page) == current_page).or_else(|| mine.iter().min_by_key(|t| t.page)).copied()
}

/// For a line inside a frame: `whole` is what SyncTeX gives for the frame's last line, which holds the
/// frame's own pages; `on_line` is what it gives for the line itself. The slide comes from `whole`; the
/// step is the slide on screen if it shows the line, else the first step that does, else the first step.
pub fn pick_sync_in_frame<'a>(on_line: &'a [latex::SyncTarget], whole: &'a [latex::SyncTarget], frame_of: impl Fn(usize) -> Option<usize> + Copy, current_page: Option<usize>) -> Option<&'a latex::SyncTarget> {
    let target = frame_of(pick_sync(whole, frame_of, current_page)?.page)?;
    let in_target = |t: &&latex::SyncTarget| frame_of(t.page) == Some(target);
    let mine: Vec<&latex::SyncTarget> = on_line.iter().filter(in_target).collect();
    if !mine.is_empty() {
        return mine.iter().find(|t| Some(t.page) == current_page).or_else(|| mine.iter().min_by_key(|t| t.page)).copied();
    }
    let all: Vec<&latex::SyncTarget> = whole.iter().filter(in_target).collect();
    all.iter().find(|t| Some(t.page) == current_page).or_else(|| all.iter().min_by_key(|t| t.page)).copied()
}

#[cfg(test)]
mod sync_pick_tests {
    use super::*;
    use crate::latex::SyncTarget;

    fn at(page: usize) -> SyncTarget {
        SyncTarget { page, x: 1.0, y: 1.0 }
    }

    // Pages 0-1 are slide 0, page 2 is slide 1 (one step), pages 3-5 are slide 2 (three steps).
    fn frame_of(p: usize) -> Option<usize> {
        match p {
            0 | 1 => Some(0),
            2 => Some(1),
            3..=5 => Some(2),
            _ => None,
        }
    }

    #[test]
    fn the_page_before_a_frame_does_not_win() {
        // What SyncTeX says for a line inside slide 2: a few boxes on page 2 first, then many on 3-5.
        let mut found: Vec<SyncTarget> = (0..19).map(|_| at(2)).collect();
        found.extend((0..200).map(|_| at(4)).chain((0..200).map(|_| at(5))).chain((0..200).map(|_| at(3))));
        assert_eq!(pick_sync(&found, frame_of, Some(0)).map(|t| t.page), Some(3), "first step of the right slide");
        assert_eq!(pick_sync(&found, frame_of, Some(5)).map(|t| t.page), Some(5), "stays on the step on screen");
    }

    #[test]
    fn a_line_that_appears_later_goes_to_its_step() {
        let found: Vec<SyncTarget> = vec![at(2), at(2), at(5), at(5), at(5), at(4), at(4), at(4)];
        assert_eq!(pick_sync(&found, frame_of, None).map(|t| t.page), Some(4));
    }

    #[test]
    fn a_frames_opening_line_shows_that_frame_not_the_one_before() {
        // Real SyncTeX answers for the TeXPress sample: \begin{frame}{Why Graph Search…} (line 50)
        // gives only the previous slide's pages; its \end{frame} line gives its own.
        let opening: Vec<SyncTarget> = (0..14).map(|_| at(0)).chain((0..206).map(|_| at(2))).collect();
        let closing: Vec<SyncTarget> = (0..19).map(|_| at(2)).chain((0..246).map(|_| at(3))).chain((0..246).map(|_| at(4))).chain((0..238).map(|_| at(5))).collect();
        let frame_of = |p: usize| match p {
            0 => Some(0),
            2 => Some(1),
            3..=5 => Some(2),
            _ => None,
        };
        assert_eq!(pick_sync_in_frame(&opening, &closing, frame_of, Some(0)).map(|t| t.page), Some(3), "first step of its own slide");
        assert_eq!(pick_sync_in_frame(&opening, &closing, frame_of, Some(4)).map(|t| t.page), Some(4), "keeps the step on screen");
        // A line revealed on the second step goes to that step.
        let second: Vec<SyncTarget> = (0..19).map(|_| at(2)).chain((0..246).map(|_| at(4))).chain((0..238).map(|_| at(5))).collect();
        assert_eq!(pick_sync_in_frame(&second, &closing, frame_of, Some(0)).map(|t| t.page), Some(4));
    }

    #[test]
    fn nothing_found_means_no_choice() {
        assert!(pick_sync(&[], frame_of, Some(0)).is_none());
        assert!(pick_sync(&[at(9)], frame_of, None).is_none());
    }
}

/// Whether two paths name the same file, however they are written.
pub fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

#[cfg(test)]
mod same_file_tests {
    use super::*;

    #[test]
    fn one_file_under_two_spellings_is_one_file() {
        let d = std::env::temp_dir().join(format!("txp-same-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("Lecture.pdf");
        std::fs::write(&f, "x").unwrap();
        assert!(same_file(&f, &d.join(".").join("Lecture.pdf")));
        assert!(same_file(&f, &f.canonicalize().unwrap()));
        assert!(!same_file(&f, &d.join("Other.pdf")));
        let _ = std::fs::remove_dir_all(&d);
    }
}

/// Shows a folder in the system's file manager.
pub fn open_folder(dir: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(dir).spawn();
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("explorer").arg(dir).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(dir).spawn();
    r.map(|_| ())
}
