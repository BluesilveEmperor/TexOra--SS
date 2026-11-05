//! The start page shown at every launch.
use crate::app::*;
use crate::icons::{self, logo, Icon};
use egui::*;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// "today", "yesterday" or "12 days ago", from a file's last change.
pub fn age_text(then: SystemTime, now: SystemTime) -> String {
    let days = now.duration_since(then).map(|d| d.as_secs() / 86_400).unwrap_or(0);
    match days {
        0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        2..=59 => format!("{days} days ago"),
        _ => format!("{} months ago", days / 30),
    }
}

/// Shortens a folder for display, keeping the part nearest the file.
fn short_dir(p: &Path, max: usize) -> String {
    let mut s = p.parent().map(|d| d.display().to_string()).unwrap_or_default();
    if let Some(h) = dirs::home_dir() {
        let home = h.display().to_string();
        if let Some(rest) = s.strip_prefix(&home) {
            s = format!("~{rest}");
        }
    }
    let n = s.chars().count();
    if n > max {
        let tail: String = s.chars().skip(n - max + 1).collect();
        format!("…{tail}")
    } else {
        s
    }
}

impl App {
    pub fn start_ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, BG);
        home_background(ui, full);
        let width = (full.width() - 64.0).min(1020.0);
        let inner = Rect::from_center_size(full.center(), vec2(width, full.height() - 40.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
        ScrollArea::vertical().id_salt("start").auto_shrink([false, false]).show(&mut child, |ui| {
            ui.add_space(if full.height() > 720.0 { 34.0 } else { 8.0 });
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(58.0, 58.0), Sense::hover());
                logo(ui, r);
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(RichText::new("TeXora").size(32.0).family(FontFamily::Name("strong".into())).color(INK));
                    ui.label(RichText::new("Present PDF and LaTeX Beamer slides with confidence").size(16.0).color(MUTED));
                });
            });
            ui.add_space(24.0);
            if !self.status.is_empty() {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    icons::paint(ui.painter(), r, Icon::Warn, WARN);
                    ui.label(RichText::new(self.status.clone()).color(WARN));
                });
                ui.add_space(8.0);
            }
            let stacked = width < 760.0;
            let actions = if stacked { width } else { 340.0 };
            if stacked {
                self.start_actions(ui, actions);
                ui.add_space(20.0);
                self.start_recent(ui, width);
            } else {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 34.0;
                    ui.vertical(|ui| self.start_actions(ui, actions));
                    ui.vertical(|ui| self.start_recent(ui, width - actions - 34.0));
                });
            }
        });
    }

    fn start_actions(&mut self, ui: &mut Ui, width: f32) {
        ui.label(RichText::new("Get started").size(17.0).family(FontFamily::Name("medium".into())).color(INK));
        ui.add_space(4.0);
        if card(ui, width, Icon::New, "New LaTeX presentation", "Start from a ready Beamer template", true).clicked() {
            self.new_presentation_dialog();
        }
        if card(ui, width, Icon::Pdf, "Open PDF presentation", "Present any PDF, with notes if it has them", false).clicked() {
            self.open_dialog("PDF presentation", &["pdf"]);
        }
        if card(ui, width, Icon::Folder, "Open LaTeX project", "Compile a .tex file and present it", false).clicked() {
            self.open_dialog("LaTeX source", &["tex"]);
        }
        if card(ui, width, Icon::Spark, "Try the sample lecture", "See notes, overlays and citations at work", false).clicked() {
            if let Some(p) = ensure_sample() {
                self.open_path(&p);
            }
        }
    }

    fn start_recent(&mut self, ui: &mut Ui, width: f32) {
        ui.label(RichText::new("Recent files").size(17.0).family(FontFamily::Name("medium".into())).color(INK));
        ui.add_space(4.0);
        let recents = self.st.recent.clone();
        if recents.is_empty() {
            self.welcome(ui, width);
            return;
        }
        let mut open: Option<PathBuf> = None;
        let mut remove: Option<PathBuf> = None;
        let now = SystemTime::now();
        Frame::new().fill(SURFACE).stroke(Stroke::new(1.0, LINE)).corner_radius(10).show(ui, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = 0.0;
            for p in &recents {
                let (rect, resp) = ui.allocate_exact_size(vec2(width, 60.0), Sense::click());
                let exists = p.exists();
                if resp.hovered() {
                    ui.painter().rect_filled(rect.shrink(3.0), 8.0, Color32::from_rgb(241, 240, 236));
                }
                let is_tex = p.extension().map(|e| e.eq_ignore_ascii_case("tex")).unwrap_or(false);
                let chip = Rect::from_min_size(rect.min + vec2(14.0, 13.0), vec2(36.0, 34.0));
                let (bg, fg, tag) = if is_tex { (LEAF_SOFT, LEAF_DARK, "TEX") } else { (Color32::from_rgb(246, 226, 222), BAD, "PDF") };
                ui.painter().rect_filled(chip, 7.0, bg);
                ui.painter().text(chip.center(), Align2::CENTER_CENTER, tag, FontId::new(11.5, FontFamily::Name("strong".into())), fg);
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let main = if exists { INK } else { MUTED };
                ui.painter().text(rect.min + vec2(62.0, 12.0), Align2::LEFT_TOP, name, FontId::new(15.0, FontFamily::Name("medium".into())), main);
                let room = ((width - 200.0) / 7.0).max(16.0) as usize;
                ui.painter().text(rect.min + vec2(62.0, 33.0), Align2::LEFT_TOP, short_dir(p, room), FontId::proportional(12.5), MUTED);
                let when = if exists {
                    std::fs::metadata(p).and_then(|m| m.modified()).map(|t| age_text(t, now)).unwrap_or_default()
                } else {
                    "File not found".to_string()
                };
                let right = if resp.hovered() { 44.0 } else { 14.0 };
                ui.painter().text(pos2(rect.right() - right, rect.center().y), Align2::RIGHT_CENTER, when, FontId::proportional(12.5), if exists { MUTED } else { BAD });
                if resp.hovered() {
                    let b = Rect::from_center_size(pos2(rect.right() - 24.0, rect.center().y), vec2(28.0, 28.0));
                    let mut c = ui.new_child(UiBuilder::new().max_rect(b));
                    if icons::sized_button(&mut c, Icon::Trash, false, "Remove from this list", 28.0, 15.0).clicked() {
                        remove = Some(p.clone());
                    }
                }
                if resp.clicked() && exists {
                    open = Some(p.clone());
                }
            }
        });
        if let Some(p) = remove {
            self.st.recent.retain(|x| *x != p);
            let _ = self.st.save();
        } else if let Some(p) = open {
            self.open_path(&p);
        }
    }

    /// Shown until the first file has been opened.
    fn welcome(&mut self, ui: &mut Ui, width: f32) {
        Frame::new().fill(SURFACE).stroke(Stroke::new(1.0, LINE)).corner_radius(10).inner_margin(20).show(ui, |ui| {
            ui.set_width(width - 42.0);
            ui.label(RichText::new("Welcome to TeXora").size(19.0).family(FontFamily::Name("medium".into())).color(LEAF_DARK));
            ui.add_space(2.0);
            ui.label(RichText::new("Your recent presentations will appear here.").color(MUTED));
            ui.add_space(8.0);
            for (icon, t) in [
                (Icon::Presenter, "See notes, the next slide and your time while the room sees only the slide."),
                (Icon::Code, "Open a .tex file: it compiles, and rebuilds each time you save it."),
                (Icon::Chart, "Rehearse, then see where the time went on each slide."),
            ] {
                ui.horizontal_top(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                    icons::paint(ui.painter(), r, icon, LEAF);
                    ui.add(Label::new(t).wrap());
                });
            }
        });
    }
}

fn card(ui: &mut Ui, width: f32, icon: Icon, title: &str, detail: &str, primary: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 68.0), Sense::click());
    let hover = resp.hovered();
    let (bg, title_c, detail_c, glyph) = if primary {
        (if hover { LEAF_DARK } else { LEAF }, Color32::WHITE, Color32::from_rgb(226, 238, 220), Color32::WHITE)
    } else {
        (if hover { LEAF_SOFT } else { SURFACE }, INK, MUTED, LEAF_DARK)
    };
    ui.painter().rect_filled(rect, 10.0, bg);
    if !primary {
        ui.painter().rect_stroke(rect, 10.0, Stroke::new(1.0, LINE), StrokeKind::Inside);
    }
    icons::paint(ui.painter(), Rect::from_center_size(pos2(rect.left() + 34.0, rect.center().y), vec2(24.0, 24.0)), icon, glyph);
    ui.painter().text(rect.min + vec2(62.0, 14.0), Align2::LEFT_TOP, title, FontId::new(15.5, FontFamily::Name("medium".into())), title_c);
    ui.painter().text(rect.min + vec2(62.0, 37.0), Align2::LEFT_TOP, detail, FontId::proportional(12.5), detail_c);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn ages_read_naturally() {
        let now = SystemTime::now();
        let d = |n: u64| now - Duration::from_secs(n * 86_400);
        assert_eq!(age_text(d(0), now), "Today");
        assert_eq!(age_text(d(1), now), "Yesterday");
        assert_eq!(age_text(d(12), now), "12 days ago");
        assert_eq!(age_text(d(120), now), "4 months ago");
    }

    #[test]
    fn long_folders_keep_their_ending() {
        let s = short_dir(Path::new("/a/very/long/folder/name/that/keeps/going/talk.pdf"), 16);
        assert!(s.starts_with('…') && s.ends_with("keeps/going"), "{s}");
    }

    #[test]
    fn background_fills_without_stretching() {
        // A wider area shows the full width and trims top and bottom equally.
        let uv = cover_uv(vec2(1600.0, 1000.0), vec2(2000.0, 1000.0));
        assert!((uv.width() - 1.0).abs() < 1e-6);
        assert!((uv.height() - 0.8).abs() < 1e-6 && (uv.top() - 0.1).abs() < 1e-6);
        // A taller area shows the full height and trims the sides.
        let uv = cover_uv(vec2(1600.0, 1000.0), vec2(800.0, 1000.0));
        assert!((uv.height() - 1.0).abs() < 1e-6 && (uv.width() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn background_picture_reads() {
        let p = image::load_from_memory_with_format(HOME_BACKGROUND, image::ImageFormat::Png).expect("reads");
        assert!(p.width() >= 1280 && p.height() >= 800);
    }
}

/// The picture behind the Home page. Replace assets/home-background.png to change it.
const HOME_BACKGROUND: &[u8] = include_bytes!("../assets/home-background.png");

/// Fills `rect` with the Home picture, cropping its edges rather than stretching it.
fn home_background(ui: &Ui, rect: Rect) {
    let key = Id::NULL.with("home-background");
    let ctx = ui.ctx().clone();
    let stored: Option<Option<(TextureHandle, Vec2)>> = ctx.data(|d| d.get_temp(key));
    let made = stored.unwrap_or_else(|| {
        // Read once, the first time Home is shown.
        let picture = image::load_from_memory_with_format(HOME_BACKGROUND, image::ImageFormat::Png).ok().map(|p| p.to_rgba8());
        let made = picture.map(|p| {
            let size = [p.width() as usize, p.height() as usize];
            let image = ColorImage::from_rgba_unmultiplied(size, p.as_raw());
            let smooth = TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
            (ctx.load_texture("home-background", image, smooth), vec2(size[0] as f32, size[1] as f32))
        });
        ctx.data_mut(|d| d.insert_temp(key, made.clone()));
        made
    });
    let Some((texture, size)) = made else { return };
    ui.painter().image(texture.id(), rect, cover_uv(size, rect.size()), Color32::WHITE);
}

/// The part of a picture of `picture` size that fills an area of `area` size without stretching.
fn cover_uv(picture: Vec2, area: Vec2) -> Rect {
    let (pa, aa) = (picture.x / picture.y.max(1.0), area.x / area.y.max(1.0));
    if aa > pa {
        let h = pa / aa;
        Rect::from_min_max(pos2(0.0, (1.0 - h) / 2.0), pos2(1.0, (1.0 + h) / 2.0))
    } else {
        let w = aa / pa;
        Rect::from_min_max(pos2((1.0 - w) / 2.0, 0.0), pos2((1.0 + w) / 2.0, 1.0))
    }
}

/// A picture packed into the program, turned into a texture once and kept.
fn packed_picture(ctx: &Context, name: &str, png: &'static [u8]) -> Option<(TextureHandle, Vec2)> {
    let key = Id::NULL.with(("packed-picture", name.to_string()));
    let stored: Option<Option<(TextureHandle, Vec2)>> = ctx.data(|d| d.get_temp(key));
    stored.unwrap_or_else(|| {
        let made = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok().map(|p| p.to_rgba8()).map(|p| {
            let size = [p.width() as usize, p.height() as usize];
            let image = ColorImage::from_rgba_unmultiplied(size, p.as_raw());
            let smooth = TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
            (ctx.load_texture(name, image, smooth), vec2(size[0] as f32, size[1] as f32))
        });
        ctx.data_mut(|d| d.insert_temp(key, made.clone()));
        made
    })
}

fn draw_picture(ui: &mut Ui, ctx: &Context, name: &str, png: &'static [u8], width: f32) -> Response {
    match packed_picture(ctx, name, png) {
        Some((t, size)) => {
            let h = width * size.y / size.x.max(1.0);
            let (r, resp) = ui.allocate_exact_size(vec2(width, h), Sense::click());
            ui.painter().image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, LINE), StrokeKind::Outside);
            resp
        }
        None => ui.allocate_exact_size(vec2(width, width * 0.5625), Sense::hover()).1,
    }
}

impl App {
    /// Choosing a template for a new presentation: the list on the left, a large preview on the right.
    pub fn template_picker(&mut self, ctx: &Context) {
        let Some(mut pick) = self.template_picker else { return };
        let mut create = false;
        let mut cancel = false;
        let resp = Modal::new(Id::NULL.with("template-picker")).show(ctx, |ui| {
            ui.set_width(900.0);
            ui.label(RichText::new("New LaTeX presentation").size(20.0).family(FontFamily::Name("strong".into())).color(INK));
            ui.label(RichText::new("Choose a style. Each template is a complete Beamer file with title, outline, step-by-step points, columns, an equation and speaker notes, ready to edit.").size(13.0).color(MUTED));
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(390.0);
                    for (i, t) in crate::templates::TEMPLATES.iter().enumerate() {
                        let selected = i == pick;
                        let fill = if selected { LEAF_SOFT } else { Color32::from_rgb(247, 246, 243) };
                        let stroke = if selected { Stroke::new(1.5, LEAF) } else { Stroke::new(1.0, LINE) };
                        let row = Frame::new().fill(fill).stroke(stroke).corner_radius(8).inner_margin(Margin::same(8)).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let img = draw_picture(ui, ctx, &format!("tpl-{}-1", t.id), t.title_png, 118.0);
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.label(RichText::new(t.name).size(15.0).family(FontFamily::Name("medium".into())).color(INK));
                                    ui.label(RichText::new(t.tags).size(11.5).color(LEAF_DARK));
                                    ui.add(Label::new(RichText::new(t.about).size(12.0).color(MUTED)).wrap());
                                });
                                img
                            })
                            .inner
                        });
                        let whole = ui.interact(row.response.rect, Id::NULL.with(("template-row", i)), Sense::click());
                        if whole.clicked() || row.inner.clicked() {
                            pick = i;
                        }
                        if whole.double_clicked() || row.inner.double_clicked() {
                            pick = i;
                            create = true;
                        }
                        ui.add_space(5.0);
                    }
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    let t = &crate::templates::TEMPLATES[pick];
                    ui.label(RichText::new(t.name).size(17.0).family(FontFamily::Name("strong".into())).color(INK));
                    ui.add_space(4.0);
                    let w = ui.available_width().min(470.0);
                    let wide = if t.tags.starts_with("4:3") { w * 0.82 } else { w };
                    draw_picture(ui, ctx, &format!("tpl-{}-1", t.id), t.title_png, wide);
                    ui.add_space(6.0);
                    draw_picture(ui, ctx, &format!("tpl-{}-5", t.id), t.content_png, wide);
                });
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("You choose where to save it next. Change its colours later on the \\definecolor lines at the top.").size(12.0).color(MUTED));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = format!("Create with {}…", crate::templates::TEMPLATES[pick].name);
                    if ui.add(Button::new(RichText::new(label).color(Color32::WHITE)).fill(LEAF)).clicked() {
                        create = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        });
        if cancel || resp.should_close() {
            self.template_picker = None;
            return;
        }
        if create {
            self.template_picker = None;
            // Test pictures never open a save dialog; they create the presentation with --new instead.
            if !self.is_screenshot() {
                self.save_new_presentation(pick);
            }
            return;
        }
        self.template_picker = Some(pick);
    }
}
