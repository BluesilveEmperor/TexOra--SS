//! The presenter screen, the audience screen and everything that draws on a slide.
use crate::analytics::{fmt_time, InkKind, Stroke as InkStroke};
use crate::app::*;
use crate::icons::{self, ribbon, sized_button, Icon};
use crate::latex::Engine;
use crate::pdf::{LinkTarget, Region};
use egui::epaint::Shape;
use egui::*;

/// Share of the width for the main panel so a slide of this shape fills the height.
pub fn auto_col(width: f32, height: f32, aspect: f32, gap: f32) -> f32 {
    ((height * aspect + gap) / width.max(1.0)).clamp(0.45, 0.85)
}

/// Share of the presenter's width for the Notes column when no divider has been moved. A PDF notes
/// page (`notes_aspect`) gets just the width its full height needs, so its card has no empty strip,
/// as long as the slide column keeps `slides_w`; whatever is left goes to the slides, which grow into
/// it. Written notes keep what is left once the slide column fits.
pub fn presenter_auto_col(inner_w: f32, notes_h: f32, notes_aspect: Option<f32>, slides_w: f32, gap: f32) -> f32 {
    let inner_w = inner_w.max(1.0);
    let notes_w = match notes_aspect {
        Some(a) => (notes_h * a + 8.0).min(inner_w - slides_w - gap),
        None => inner_w - slides_w - gap,
    };
    ((notes_w + gap / 2.0) / inner_w).clamp(0.4, 0.8)
}

/// Share of the slide column's height for the current slide when no divider has been moved: at least
/// half, and more when the column is wide enough for the current slide to fill its width, so widening
/// the column makes the current slide bigger. The next slide keeps at least 30%.
pub fn current_row(col_w: f32, col_h: f32, aspect: f32, chrome: f32, gap: f32) -> f32 {
    let fit_h = (col_w - 28.0).max(1.0) / aspect.max(0.1) + chrome;
    ((fit_h + gap / 2.0) / col_h.max(1.0)).clamp(0.5, 0.7)
}

/// What the pointer did to a divider this frame.
pub struct Handle {
    pub delta: Vec2,
    double: bool,
    done: bool,
}

impl Handle {
    /// Moves a stored divider. A double-click hands it back to automatic fit.
    pub fn apply(&self, slot: &mut Option<f32>, now: f32, change: f32) -> bool {
        if self.double {
            *slot = None;
        } else if change != 0.0 {
            *slot = Some((now + change).clamp(0.15, 0.9));
        }
        self.done || self.double
    }
}

/// A divider to drag, with a small grip so it can be found.
fn drag_handle(ui: &Ui, id: &str, r: Rect, vertical: bool, line: bool) -> Handle {
    let cursor = if vertical { CursorIcon::ResizeHorizontal } else { CursorIcon::ResizeVertical };
    let resp = ui.interact(r, sid(id), Sense::click_and_drag()).on_hover_cursor(cursor);
    let hot = resp.hovered() || resp.dragged();
    let color = if hot { LEAF } else { Color32::from_gray(165) };
    if line {
        let c = if hot { LEAF } else { LINE };
        ui.painter().vline(r.center().x, r.y_range(), Stroke::new(if hot { 2.0 } else { 1.0 }, c));
    }
    let size = if vertical { vec2(4.0, 34.0) } else { vec2(34.0, 4.0) };
    ui.painter().rect_filled(Rect::from_center_size(r.center(), size), 2.0, color);
    let resp = resp.on_hover_text("Drag to resize. Double-click to fit automatically.");
    Handle { delta: resp.drag_delta(), double: resp.double_clicked(), done: resp.drag_stopped() }
}

/// The wheel or a pinch zooms around the pointer, and dragging moves an enlarged view.
fn pan_zoom(ui: &Ui, resp: &Response, r: Rect, zoom: &mut Option<[f32; 4]>, changed: &mut std::time::Instant) {
    let uv = uv_rect(*zoom);
    if let Some(pos) = resp.hover_pos() {
        let (wheel, pinch) = ui.input(|i| (i.smooth_scroll_delta().y, i.zoom_delta()));
        let factor = (wheel * 0.004).exp() * pinch;
        if (factor - 1.0).abs() > 0.0005 {
            *zoom = zoomed(*zoom, factor, to_slide(r, uv, pos));
            *changed = std::time::Instant::now();
        }
    }
    if let (true, Some(z)) = (resp.dragged(), zoom.as_mut()) {
        let size = z[2] - z[0];
        let d = resp.drag_delta();
        z[0] = (z[0] - d.x / r.width() * size).clamp(0.0, 1.0 - size);
        z[1] = (z[1] - d.y / r.height() * size).clamp(0.0, 1.0 - size);
        z[2] = z[0] + size;
        z[3] = z[1] + size;
    }
    if resp.double_clicked() {
        *zoom = None;
    }
}

/// A stable widget id from a name.
pub fn sid(name: &str) -> Id {
    Id::NULL.with(name)
}

/// Largest rectangle of this shape inside `r`; at the top, or centred.
pub fn fit(r: Rect, aspect: f32, top: bool) -> Rect {
    let w = r.width().min(r.height() * aspect);
    let h = w / aspect;
    let x = r.center().x - w / 2.0;
    let y = if top { r.top() } else { r.center().y - h / 2.0 };
    Rect::from_min_size(pos2(x, y), vec2(w, h))
}

fn uv_rect(z: Option<[f32; 4]>) -> Rect {
    match z {
        Some(u) => Rect::from_min_max(pos2(u[0], u[1]), pos2(u[2], u[3])),
        None => Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
    }
}

pub fn to_screen(r: Rect, uv: Rect, p: [f32; 2]) -> Pos2 {
    pos2(r.left() + (p[0] - uv.left()) / uv.width() * r.width(), r.top() + (p[1] - uv.top()) / uv.height() * r.height())
}

pub fn to_slide(r: Rect, uv: Rect, p: Pos2) -> [f32; 2] {
    [uv.left() + (p.x - r.left()) / r.width() * uv.width(), uv.top() + (p.y - r.top()) / r.height() * uv.height()]
}

fn paint_stroke(p: &Painter, r: Rect, uv: Rect, s: &InkStroke) {
    let pts: Vec<Pos2> = s.points.iter().map(|q| to_screen(r, uv, *q)).collect();
    let w = (s.width * r.width() / uv.width()).max(1.0);
    let c = match s.kind {
        InkKind::Pen => Color32::from_rgb(s.color[0], s.color[1], s.color[2]),
        InkKind::Highlighter => Color32::from_rgba_unmultiplied(s.color[0], s.color[1], s.color[2], 90),
    };
    if pts.iter().all(|q| *q == pts[0]) {
        p.circle_filled(pts[0], w / 2.0, c);
    } else {
        p.add(Shape::line(pts, Stroke::new(w, c)));
    }
}

/// Border of the presenter's cards: a little softer than the lines elsewhere.
pub const CARD_LINE: Color32 = Color32::from_rgb(220, 219, 216);

/// A rounded card with its label inside, at the top left. Returns the height the label takes.
pub fn card(p: &Painter, r: Rect, label: &str, size: f32) -> f32 {
    p.rect_filled(r, 10.0, SURFACE);
    p.rect_stroke(r, 10.0, Stroke::new(1.0, CARD_LINE), StrokeKind::Inside);
    if label.is_empty() {
        return 8.0;
    }
    p.text(r.min + vec2(14.0, 10.0), Align2::LEFT_TOP, label, FontId::new(size, FontFamily::Name("medium".into())), MUTED);
    size + 20.0
}

/// A colour mixed toward white by `k` (0 keeps it, 1 is white).
fn soften(c: [u8; 3], k: f32) -> Color32 {
    let m = |v: u8| (v as f32 + (255.0 - v as f32) * k) as u8;
    Color32::from_rgb(m(c[0]), m(c[1]), m(c[2]))
}

/// A darker shade of a colour, for text on its softened version.
fn deepen(c: [u8; 3]) -> Color32 {
    let m = |v: u8| (v as f32 * 0.55) as u8;
    Color32::from_rgb(m(c[0]), m(c[1]), m(c[2]))
}

/// A rounded label. `at` is its left edge, or its right edge when `right` is set; returns where it went.
fn pill(p: &Painter, at: Pos2, right: bool, text: &str, size: f32, fill: Color32, ink: Color32, icon: Option<Icon>) -> Rect {
    let g = p.layout_no_wrap(text.to_string(), FontId::new(size, FontFamily::Name("medium".into())), ink);
    let lead = if icon.is_some() { size + 6.0 } else { 0.0 };
    let size_r = g.size() + vec2(size * 1.5 + lead, size * 0.5);
    let left = if right { at.x - size_r.x } else { at.x };
    let r = Rect::from_min_size(pos2(left, at.y - size_r.y / 2.0), size_r);
    p.rect_filled(r, r.height() / 2.0, fill);
    if let Some(i) = icon {
        icons::paint(p, Rect::from_center_size(pos2(r.left() + size * 0.75 + size / 2.0, r.center().y), Vec2::splat(size)), i, ink);
    }
    p.galley(pos2(r.left() + size * 0.75 + lead, r.center().y - g.size().y / 2.0), g, ink);
    r
}

/// A white button with a soft border, for text and arrows; it can be greyed out.
fn soft_button(ui: &Ui, r: Rect, id: &str, tip: &str, enabled: bool, active: bool) -> Response {
    let resp = ui.interact(r, sid(id), if enabled { Sense::click() } else { Sense::hover() });
    let resp = if enabled { resp.on_hover_cursor(CursorIcon::PointingHand) } else { resp };
    let resp = if tip.is_empty() { resp } else { resp.on_hover_text(tip) };
    let fill = if active {
        LEAF_SOFT
    } else if resp.hovered() && enabled {
        Color32::from_rgb(240, 239, 236)
    } else {
        Color32::WHITE
    };
    ui.painter().rect_filled(r, 9.0, fill);
    ui.painter().rect_stroke(r, 9.0, Stroke::new(1.0, if active { Color32::from_rgb(183, 211, 169) } else { CARD_LINE }), StrokeKind::Inside);
    resp
}

impl App {
    /// Draws one PDF page region. Returns false while no picture is available yet.
    pub fn paint_page(&mut self, ui: &Ui, rect: Rect, page: usize, region: Region, uv: Rect) -> bool {
        let ppp = ui.ctx().pixels_per_point();
        let want = (rect.width() * ppp / uv.width()) as u32;
        match self.texture(page, region, want) {
            Some(t) => {
                ui.painter().with_clip_rect(rect).image(t.id(), rect, uv, Color32::WHITE);
                true
            }
            None => {
                ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
                false
            }
        }
    }

    fn paint_overlays(&self, ui: &Ui, rect: Rect, uv: Rect, idx: usize) {
        let p = ui.painter().with_clip_rect(rect);
        let key = self.ink_key();
        if idx == self.live() {
            if let Some(list) = self.side.ink.get(&key) {
                for s in list {
                    paint_stroke(&p, rect, uv, s);
                }
            }
            if let Some(s) = &self.stroke {
                paint_stroke(&p, rect, uv, s);
            }
            let t = ui.input(|i| i.time);
            let c = self.st.laser_color;
            let unit = rect.width() / 900.0;
            for (q, born) in &self.laser {
                let age = ((t - born) / 0.45).clamp(0.0, 1.0) as f32;
                let at = to_screen(rect, uv, *q);
                let a = ((1.0 - age) * 230.0) as u8;
                p.circle_filled(at, (9.0 - 5.0 * age) * unit, Color32::from_rgba_unmultiplied(c[0], c[1], c[2], a / 3));
                p.circle_filled(at, (4.5 - 2.5 * age) * unit, Color32::from_rgba_unmultiplied(c[0], c[1], c[2], a));
            }
        }
    }

    /// A slide with its ink, laser and zoom, shown the same way on both screens.
    fn paint_slide(&mut self, ui: &Ui, rect: Rect, idx: usize) {
        let Some(s) = self.deck.slides.get(idx).cloned() else {
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(235, 235, 233));
            return;
        };
        let uv = if idx == self.live() { uv_rect(self.zoom) } else { uv_rect(None) };
        self.paint_page(ui, rect, s.page, s.region, uv);
        self.paint_animations(ui, rect, s.page, s.region, uv);
        self.paint_videos(ui, rect, idx, uv);
        if self.left == Some(LeftTab::Search) {
            // Search matches on this page. Page positions are of the whole page, so a slide
            // that fills only half of a double-width page needs converting.
            let p = ui.painter().with_clip_rect(rect);
            for hit in self.hits.iter().filter(|h| h.page == s.page) {
                for q in &hit.rects {
                    let (x0, x1) = match s.region {
                        Region::Full => (q[0], q[2]),
                        Region::Left => (q[0] * 2.0, q[2] * 2.0),
                        Region::Right => (q[0] * 2.0 - 1.0, q[2] * 2.0 - 1.0),
                    };
                    if x1 <= 1.0 && x0 >= 0.0 {
                        let r = Rect::from_two_pos(to_screen(rect, uv, [x0, q[1]]), to_screen(rect, uv, [x1, q[3]]));
                        p.rect_filled(r, 1.0, Color32::from_rgba_unmultiplied(250, 210, 40, 110));
                    }
                }
            }
        }
        self.paint_overlays(ui, rect, uv, idx);
    }

    /// Where each animation of a page sits on screen, for a slide drawn in `rect` (only those that show).
    pub fn animation_rects(&self, rect: Rect, page: usize, region: Region, uv: Rect) -> Vec<(usize, Rect)> {
        let Some(pg) = self.doc.as_ref().and_then(|d| d.pages.get(page)) else { return vec![] };
        pg.animations
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                let (x0, x1) = match region {
                    Region::Full => (a.rect[0], a.rect[2]),
                    Region::Left => (a.rect[0] * 2.0, a.rect[2] * 2.0),
                    Region::Right => (a.rect[0] * 2.0 - 1.0, a.rect[2] * 2.0 - 1.0),
                };
                (x1 > 0.0 && x0 < 1.0).then(|| (i, Rect::from_two_pos(to_screen(rect, uv, [x0, a.rect[1]]), to_screen(rect, uv, [x1, a.rect[3]]))))
            })
            .collect()
    }

    /// Where each clip of a page sits on screen, for a slide drawn in `rect`.
    pub fn media_rects(&self, rect: Rect, page: usize, region: Region, uv: Rect) -> Vec<(crate::media::MediaClip, Rect)> {
        let Some(pg) = self.doc.as_ref().and_then(|d| d.pages.get(page)) else { return vec![] };
        pg.media
            .iter()
            .filter_map(|m| {
                let (x0, x1) = match region {
                    Region::Full => (m.rect[0], m.rect[2]),
                    Region::Left => (m.rect[0] * 2.0, m.rect[2] * 2.0),
                    Region::Right => (m.rect[0] * 2.0 - 1.0, m.rect[2] * 2.0 - 1.0),
                };
                (x1 > 0.0 && x0 < 1.0).then(|| (m.clip.clone(), Rect::from_two_pos(to_screen(rect, uv, [x0, m.rect[1]]), to_screen(rect, uv, [x1, m.rect[3]]))))
            })
            .collect()
    }

    /// A play badge on each clip of the live slide, on the speaker's screen only.
    fn media_badges(&mut self, ui: &mut Ui, rect: Rect, idx: usize) {
        let Some(s) = self.deck.slides.get(idx).cloned() else { return };
        for (clip, r) in self.media_rects(rect, s.page, s.region, uv_rect(self.zoom)) {
            let p = ui.painter().with_clip_rect(rect);
            p.rect_stroke(r, 4.0, Stroke::new(1.5, LEAF), StrokeKind::Outside);
            let d = (r.height().min(r.width()) * 0.35).clamp(18.0, 44.0);
            let c = r.center();
            p.circle_filled(c, d / 2.0, Color32::from_rgba_unmultiplied(20, 40, 25, 200));
            icons::paint(&p, Rect::from_center_size(c + vec2(d * 0.05, 0.0), vec2(d * 0.5, d * 0.5)), Icon::Play, Color32::WHITE);
            let note = match (&clip.file, clip.autoplay) {
                (None, _) => format!("{}: not found beside the presentation", clip.name),
                (Some(_), true) => format!("{}: plays when shown, or click", clip.name),
                (Some(_), false) => format!("{}: click to play", clip.name),
            };
            let g = ui.fonts_mut(|f| f.layout_no_wrap(note, FontId::proportional(11.5), Color32::WHITE));
            let label = Rect::from_center_size(pos2(c.x, (r.bottom() - 10.0).max(c.y + d / 2.0 + 10.0)), g.size() + vec2(10.0, 4.0));
            // Inside the slide, even for a clip near its edge.
            let dx = (rect.left() + 3.0 - label.left()).max(0.0) - (label.right() - rect.right() + 3.0).max(0.0);
            let label = label.translate(vec2(dx, 0.0));
            p.rect_filled(label, 4.0, Color32::from_rgba_unmultiplied(20, 40, 25, 200));
            p.galley(label.min + vec2(5.0, 2.0), g, Color32::WHITE);
        }
    }

    /// The frame on show of each animation on the page, over the page picture. A slide that is not
    /// playing shows its animations' first frame, as the PDF itself does.
    fn paint_animations(&mut self, ui: &Ui, rect: Rect, page: usize, region: Region, uv: Rect) {
        let ppp = ui.ctx().pixels_per_point();
        let page_width = (rect.width() * ppp / uv.width() * if region == Region::Full { 1.0 } else { 2.0 }) as u32;
        for (i, r) in self.animation_rects(rect, page, region, uv) {
            let Some(frames) = self.doc.as_ref().and_then(|d| d.pages.get(page)).and_then(|p| p.animations.get(i)).map(|a| a.frames) else { continue };
            let frame = self.anim_play.get(&(page, i)).map(|s| s.frame).unwrap_or(0);
            if let Some(t) = self.anim_frame(page, i, frame, frames, page_width) {
                ui.painter().with_clip_rect(rect).image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
        }
    }

    /// Play, pause and step buttons under each animation of the live slide, on the speaker's screen.
    fn animation_controls(&mut self, ui: &mut Ui, rect: Rect, idx: usize) {
        let Some(s) = self.deck.slides.get(idx).cloned() else { return };
        let uv = uv_rect(self.zoom);
        for (i, r) in self.animation_rects(rect, s.page, s.region, uv) {
            let Some(a) = self.doc.as_ref().and_then(|d| d.pages.get(s.page)).and_then(|p| p.animations.get(i)).cloned() else { continue };
            let frames = a.frames;
            let st = self.anim_play.get(&(s.page, i)).cloned();
            let (frame, playing) = st.map(|x| (x.frame, x.playing)).unwrap_or((0, false));
            let size = 26.0;
            let bar_w = size * 5.0 + 48.0;
            // Over the PDF's own buttons where it has them, so there is one set of controls;
            // otherwise just under the animation.
            let own = a.controls.map(|c| {
                let (x0, x1) = match s.region {
                    Region::Full => (c[0], c[2]),
                    Region::Left => (c[0] * 2.0, c[2] * 2.0),
                    Region::Right => (c[0] * 2.0 - 1.0, c[2] * 2.0 - 1.0),
                };
                Rect::from_two_pos(to_screen(rect, uv, [x0, c[1]]), to_screen(rect, uv, [x1, c[3]]))
            });
            let bar = match own {
                Some(c) => Rect::from_min_size(pos2(c.left(), (c.center().y - size / 2.0).max(r.bottom() + 1.0)), vec2(bar_w.max(c.width()), size)),
                None => {
                    let below = r.bottom() + size + 6.0 <= rect.bottom();
                    let top = if below { r.bottom() + 3.0 } else { r.bottom() - size - 3.0 };
                    Rect::from_min_size(pos2(r.left().max(rect.left() + 2.0), top), vec2(bar_w, size))
                }
            };
            // Always inside the slide, even where the PDF's own buttons reach past its edge.
            let x = bar.left().clamp(rect.left() + 3.0, (rect.right() - bar.width() - 3.0).max(rect.left() + 3.0));
            let bar = bar.translate(vec2(x - bar.left(), 0.0));
            let cover = own.map_or(bar, |c| bar.union(c)).expand(2.0);
            ui.painter().with_clip_rect(rect).rect_filled(cover, 6.0, Color32::from_rgb(250, 250, 248));
            ui.painter().with_clip_rect(rect).rect_stroke(cover, 6.0, Stroke::new(1.0, LINE), StrokeKind::Outside);
            let mut child = ui.new_child(UiBuilder::new().max_rect(bar).id_salt(("anim-controls", s.page, i)).layout(Layout::left_to_right(Align::Center)));
            child.spacing_mut().item_spacing.x = 0.0;
            if sized_button(&mut child, Icon::First, false, "First frame", size, 13.0).clicked() {
                self.anim_seek(s.page, i, 0);
            }
            if sized_button(&mut child, Icon::Prev, false, "Previous frame", size, 13.0).clicked() {
                self.anim_seek(s.page, i, frame.saturating_sub(1));
            }
            let (icon, tip) = if playing { (Icon::Pause, "Pause the animation") } else { (Icon::Play, "Play the animation") };
            if sized_button(&mut child, icon, playing, tip, size, 13.0).clicked() {
                self.anim_toggle(s.page, i);
            }
            if sized_button(&mut child, Icon::Next, false, "Next frame", size, 13.0).clicked() {
                self.anim_seek(s.page, i, (frame + 1).min(frames.saturating_sub(1)));
            }
            if sized_button(&mut child, Icon::Last, false, "Last frame", size, 13.0).clicked() {
                self.anim_seek(s.page, i, frames.saturating_sub(1));
            }
            child.label(RichText::new(format!(" {}/{}", frame + 1, frames)).size(12.0).color(MUTED));
        }
    }

    /// Pointer behaviour over the live slide: drawing tools, laser, zoom, links and citation peeks.
    pub fn slide_input(&mut self, ui: &mut Ui, rect: Rect, idx: usize, id: &str) {
        let uv = uv_rect(self.zoom);
        let resp = ui.interact(rect, sid(id), Sense::click_and_drag());
        // The animation buttons come after the slide's own area, so they sit on top of it and get
        // their clicks; made first, the slide took every click and the buttons did nothing.
        if !id.starts_with("audience") {
            self.animation_controls(ui, rect, idx);
            self.media_badges(ui, rect, idx);
            self.video_help(ui, rect, idx, uv);
        }
        let pos = resp.hover_pos().or(resp.interact_pointer_pos());
        let Some(pos) = pos else { return };
        let p = to_slide(rect, uv, pos);
        if idx == self.live() {
            self.zoom_input(ui, p);
        }
        // While the audience is frozen on another slide, ink would land there, so tools rest.
        let tool = if idx == self.live() { self.tool } else { Tool::Pointer };
        match tool {
            Tool::Pen | Tool::Highlighter => {
                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                // A drag is only recognised after moving a little, so start from the press itself.
                if resp.drag_started() {
                    let from = ui.input(|i| i.pointer.press_origin()).unwrap_or(pos);
                    self.begin_stroke(to_slide(rect, uv, from));
                }
                if resp.dragged() {
                    if let Some(s) = &mut self.stroke {
                        s.points.push(p);
                    }
                }
                if resp.clicked() {
                    self.begin_stroke(p);
                }
                if resp.drag_stopped() || resp.clicked() {
                    self.end_stroke();
                }
            }
            Tool::Eraser => {
                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                if resp.dragged() || resp.clicked() {
                    self.erase_at(p);
                }
            }
            Tool::Laser => {
                ui.ctx().set_cursor_icon(CursorIcon::None);
                let t = ui.input(|i| i.time);
                self.laser.push((p, t));
                ui.ctx().request_repaint();
            }
            Tool::Zoom => {
                ui.ctx().set_cursor_icon(CursorIcon::ZoomIn);
                if resp.drag_started() {
                    // A drag is only recognised after moving, so start from where the press began.
                    let from = ui.input(|i| i.pointer.press_origin()).unwrap_or(pos);
                    self.zoom_drag = Some(to_slide(rect, uv, from));
                }
                if let Some(a) = self.zoom_drag {
                    let r = Rect::from_two_pos(to_screen(rect, uv, a), pos);
                    ui.painter().rect_stroke(r, 2.0, Stroke::new(2.0, LEAF), StrokeKind::Outside);
                    if resp.drag_stopped() {
                        self.zoom_drag = None;
                        let (x0, x1) = (a[0].min(p[0]).max(0.0), a[0].max(p[0]).min(1.0));
                        let (y0, y1) = (a[1].min(p[1]).max(0.0), a[1].max(p[1]).min(1.0));
                        if x1 - x0 > 0.04 && y1 - y0 > 0.04 {
                            // Keep the slide's shape so nothing looks stretched.
                            let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                            let half = ((x1 - x0).max(y1 - y0)) / 2.0;
                            let (x, y) = (cx.clamp(half, 1.0 - half), cy.clamp(half, 1.0 - half));
                            self.zoom = Some([x - half, y - half, x + half, y + half]);
                        }
                    }
                }
                if resp.double_clicked() {
                    self.zoom = None;
                }
            }
            Tool::Pointer => {
                // With the slide enlarged, dragging moves it around and a double-click fits it again.
                if self.zoom.is_some() && idx == self.live() {
                    if resp.dragged() {
                        let d = resp.drag_delta();
                        self.zoom_pan([-d.x / rect.width() * uv.width(), -d.y / rect.height() * uv.height()]);
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    } else {
                        ui.ctx().set_cursor_icon(CursorIcon::Grab);
                    }
                    if resp.double_clicked() {
                        self.zoom = None;
                    }
                }
                // A click on a texoravideo clip plays or pauses it, inside the slide.
                if let Some(s) = self.deck.slides.get(idx).cloned() {
                    let hit = self.video_rects(rect, s.page, s.region, uv).into_iter().find(|(_, r)| r.contains(pos)).map(|(v, _)| v);
                    if let Some(v) = hit {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        // A press and release in place on the clip is a click even when egui does not
                        // report one, so play and pause never miss.
                        let (down, up, at) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.interact_pos()));
                        if down {
                            self.video_press = at;
                        }
                        let own = up && self.video_press.take().zip(at).is_some_and(|(a, b)| a.distance(b) < 8.0);
                        if resp.clicked() || own {
                            self.video_click(ui.ctx(), idx, &v);
                        }
                        return;
                    }
                }
                // A click on a video clip plays it in the computer's video player.
                if let Some(s) = self.deck.slides.get(idx).cloned() {
                    let clip = self.media_rects(rect, s.page, s.region, uv).into_iter().find(|(_, r)| r.contains(pos)).map(|(c, _)| c);
                    if let Some(c) = clip {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        if resp.clicked() {
                            self.play_media(&c);
                        }
                        return;
                    }
                }
                // A click on an animation plays or pauses it, as in Acrobat.
                if let Some(s) = self.deck.slides.get(idx).cloned() {
                    let hit = self.animation_rects(rect, s.page, s.region, uv).into_iter().find(|(_, r)| r.contains(pos)).map(|(i, _)| i);
                    if let Some(i) = hit {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        if resp.clicked() {
                            self.anim_toggle(s.page, i);
                        }
                        return;
                    }
                }
                self.link_input(ui, &resp, idx, p)
            }
        }
    }

    /// Mouse wheel, and pinch on a trackpad, zoom around the pointer.
    pub fn zoom_input(&mut self, ui: &Ui, at: [f32; 2]) {
        let (wheel, pinch) = ui.input(|i| (i.smooth_scroll_delta().y, i.zoom_delta()));
        let factor = (wheel * 0.004).exp() * pinch;
        if (factor - 1.0).abs() > 0.0005 {
            self.zoom_by(factor, at);
            ui.ctx().request_repaint();
        }
    }

    fn link_input(&mut self, ui: &mut Ui, resp: &Response, idx: usize, p: [f32; 2]) {
        let Some(s) = self.deck.slides.get(idx) else { return };
        let Some(doc) = &self.doc else { return };
        let Some(pg) = doc.pages.get(s.page) else { return };
        let full_x = match s.region {
            Region::Full => p[0],
            Region::Left => p[0] / 2.0,
            Region::Right => 0.5 + p[0] / 2.0,
        };
        let hit = pg.links.iter().find(|l| full_x >= l.rect[0] && full_x <= l.rect[2] && p[1] >= l.rect[1] && p[1] <= l.rect[3]);
        let Some(hit) = hit.map(|h| h.target.clone()) else { return };
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        if resp.clicked() {
            self.follow_link(&hit);
        } else if let LinkTarget::Page(t) = hit {
            self.peek = self.deck.slide_for_page(t);
        } else if let LinkTarget::Uri(u) = hit {
            resp.show_tooltip_text(u);
        }
    }

    /// A small picture of a link's target, so a citation can be read without leaving the slide.
    pub fn peek_popup(&mut self, ctx: &Context) {
        let Some(idx) = self.peek.take() else { return };
        let Some(s) = self.deck.slides.get(idx).cloned() else { return };
        let Some(pos) = ctx.pointer_hover_pos() else { return };
        let w = 340.0;
        let h = w / self.deck.aspect.max(0.1);
        let screen = ctx.content_rect();
        let at = pos2((pos.x + 14.0).min(screen.right() - w - 12.0), (pos.y + 14.0).min(screen.bottom() - h - 36.0));
        Area::new(sid("peek")).order(Order::Tooltip).fixed_pos(at).interactable(false).show(ctx, |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(RichText::new(format!("Slide {}: click to go there", s.frame + 1)).size(11.0).color(MUTED));
                let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                self.paint_page(ui, r, s.page, s.region, uv_rect(None));
            });
        });
    }

    // ---------- presenter ----------

    pub fn presenter_ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let ctx = ui.ctx().clone();
        ui.painter().rect_filled(full, 0.0, BG);
        if self.doc.is_none() {
            self.busy_ui(ui, full);
            return;
        }
        let s = (full.width() / 1388.0).clamp(0.72, 1.7);
        let k = s.min(1.2);
        let pad = 16.0 * k;
        let gap = 14.0 * k;
        let head_h = (54.0 * s).clamp(44.0, 72.0);
        let foot_h = (102.0 * s).clamp(88.0, 140.0);
        let nav_h = (52.0 * s).clamp(42.0, 66.0);
        let aspect = self.deck.aspect.max(0.1);
        let large = self.swap;

        let head = Rect::from_min_max(pos2(full.left() + pad, full.top()), pos2(full.right() - pad, full.top() + head_h));
        self.head_row(ui, head, s);
        let body_top = head.bottom() + 12.0 * k;
        let body_bottom = full.bottom() - foot_h;
        let inner_w = full.width() - 2.0 * pad;
        // Room at the left of the large slide for the floating drawing tools.
        let tools_w = if large { 66.0 * k } else { 0.0 };

        // The main column takes the height of a slide unless a divider has been moved.
        let stored = if large { self.st.layout.large_col } else { self.st.layout.presenter_col };
        let split = stored.unwrap_or_else(|| {
            if large {
                auto_col(inner_w, body_bottom - body_top - nav_h, aspect, gap + tools_w)
            } else {
                // The slide column needs the width of the current and next slide stacked; the notes
                // page gets what its height needs and the slides take the rest.
                let slide_w = ((body_bottom - body_top - 10.0 * k - gap) / 2.0 - 14.0 * k - 34.0) * aspect + 28.0;
                let notes_h = body_bottom - nav_h - body_top - 14.0 * k - 30.0;
                presenter_auto_col(inner_w, notes_h, self.notes_page_aspect(), slide_w, gap)
            }
        });
        let div_x = full.left() + pad + inner_w * split;
        let main = Rect::from_min_max(pos2(full.left() + pad, body_top), pos2(div_x - gap / 2.0, body_bottom - nav_h));
        let col = Rect::from_min_max(pos2(div_x + gap / 2.0, body_top), pos2(full.right() - pad, body_bottom - 10.0 * k));
        let label = 14.0 * k;

        // Right column: notes (large slide) or the current slide on top, the next slide below. The
        // next slide's card fits its picture unless that divider has been moved.
        let next_fit = (col.width() - 28.0) / aspect + label + 34.0;
        // With the large slide, a PDF notes page and the next slide share the column at the same width.
        let auto_row = if large {
            match self.notes_page_aspect() {
                Some(na) => {
                    let notes_fit = (col.width() - 8.0) / na + label + 10.0;
                    (notes_fit / (notes_fit + next_fit)).clamp(0.3, 0.7)
                }
                None => (1.0 - (next_fit + gap) / col.height().max(1.0)).clamp(0.3, 0.7),
            }
        } else { current_row(col.width(), col.height(), aspect, label + 34.0, gap) };
        let row_share = (if large { self.st.layout.large_row } else { self.st.layout.presenter_row }).unwrap_or(auto_row);
        let mid = col.top() + col.height() * row_share;
        let top_box = Rect::from_min_max(col.min, pos2(col.right(), mid - gap / 2.0));
        let next_box = Rect::from_min_max(pos2(col.left(), mid + gap / 2.0), col.max);
        let p = ui.painter().clone();
        let top_label = card(&p, top_box, if large { "Notes" } else { "Current slide" }, label);
        let next_label = card(&p, next_box, "Next slide", label);
        let next_rect = fit(Rect::from_min_max(next_box.min + vec2(14.0, next_label), next_box.max - vec2(14.0, 12.0)), aspect, false);

        let cur = self.cur;
        if large {
            let area = Rect::from_min_max(main.min + vec2(tools_w, 0.0), main.max);
            let r = fit(area, aspect, true);
            self.paint_stage(ui, r);
            p.rect_stroke(r, 4.0, Stroke::new(1.0, CARD_LINE), StrokeKind::Outside);
            self.slide_input(ui, r, cur, "big-slide");
            self.tool_palette(ui, pos2(r.left() - tools_w, r.top() + 18.0 * k), k);
            self.notes_panel(ui, Rect::from_min_max(top_box.min + vec2(4.0, top_label), top_box.max - vec2(4.0, 6.0)), s * 0.85);
            self.nav_row(ui, Rect::from_min_max(pos2(area.left(), main.bottom()), pos2(area.right(), body_bottom)), s);
        } else {
            let notes_label = card(&p, main, "Notes", label);
            self.notes_panel(ui, Rect::from_min_max(main.min + vec2(4.0, notes_label), main.max - vec2(4.0, 6.0)), s);
            let r = fit(Rect::from_min_max(top_box.min + vec2(14.0, top_label), top_box.max - vec2(14.0, 12.0)), aspect, false);
            self.paint_stage(ui, r);
            p.rect_stroke(r, 2.0, Stroke::new(1.0, CARD_LINE), StrokeKind::Outside);
            self.slide_input(ui, r, cur, "small-slide");
            self.nav_row(ui, Rect::from_min_max(pos2(main.left(), main.bottom()), pos2(main.right(), body_bottom)), s);
        }
        self.paint_next(ui, next_rect);
        p.rect_stroke(next_rect, 2.0, Stroke::new(1.0, CARD_LINE), StrokeKind::Outside);

        let across = Rect::from_center_size(pos2(div_x, (body_top + body_bottom) / 2.0), vec2(gap.max(10.0), body_bottom - body_top));
        let a = drag_handle(ui, "main-col", across, true, false);
        let b = drag_handle(ui, "main-row", Rect::from_center_size(pos2(col.center().x, mid), vec2(col.width() * 0.5, gap.max(10.0))), false, false);
        let layout = &mut self.st.layout;
        let (slot_col, slot_row) = if large { (&mut layout.large_col, &mut layout.large_row) } else { (&mut layout.presenter_col, &mut layout.presenter_row) };
        let moved = a.apply(slot_col, split, a.delta.x / inner_w.max(1.0)) | b.apply(slot_row, row_share, b.delta.y / col.height().max(1.0));
        if moved {
            self.save_layout();
        }

        self.bottom_bar(ui, Rect::from_min_max(pos2(full.left() + pad, full.bottom() - foot_h), pos2(full.right() - pad, full.bottom())), s);
        self.peek_popup(&ctx);
    }

    /// The files on show, whether everything is ready (or what the audience sees), and Present.
    fn head_row(&mut self, ui: &mut Ui, r: Rect, s: f32) {
        let k = s.min(1.2);
        let p = ui.painter().clone();
        p.hline(r.x_range(), r.bottom(), Stroke::new(1.0, CARD_LINE));
        let cy = r.center().y;

        // Present opens the audience window, and closes it again, like F5.
        let open = self.audience_open;
        let bh = (40.0 * k).min(r.height() - 8.0);
        let word = if open { "Presenting" } else { "Present" };
        let font = FontId::new(17.0 * k, FontFamily::Name("medium".into()));
        let g = p.layout_no_wrap(word.to_string(), font, INK);
        let br = Rect::from_min_size(pos2(r.right() - g.size().x - 66.0 * k, cy - bh / 2.0), vec2(g.size().x + 66.0 * k, bh));
        let tip = if open { "Close the audience window (F5)" } else { "Open the audience window and start the timer (F5)" };
        if soft_button(ui, br, "present-button", tip, true, open).clicked() {
            let ctx = ui.ctx().clone();
            self.run(crate::settings::Action::Present, &ctx);
        }
        icons::paint(&p, Rect::from_center_size(pos2(br.left() + 26.0 * k, cy), Vec2::splat(20.0 * k)), Icon::Windows, if open { LEAF_DARK } else { INK });
        p.galley(pos2(br.left() + 44.0 * k, cy - g.size().y / 2.0), g, INK);

        // What the audience sees while presenting; before that, whether everything is ready.
        let warn = (Color32::from_rgb(244, 222, 171), Color32::from_rgb(107, 74, 10));
        let calm = (Color32::from_rgb(234, 233, 230), MUTED);
        let good = (LEAF_SOFT, LEAF_DARK);
        let (text, (fill, ink), icon) = if let Some(f) = self.frozen {
            let n = self.deck.slides.get(f).map(|x| x.frame + 1).unwrap_or(0);
            (format!("Audience stays on slide {n}. Press F to release."), warn, Some(Icon::Freeze))
        } else if self.screen == Screen::Black {
            ("Audience sees a black screen".to_string(), warn, Some(Icon::Black))
        } else if self.screen == Screen::White {
            ("Audience sees the whiteboard".to_string(), warn, Some(Icon::White))
        } else if open {
            ("Audience window is live".to_string(), good, Some(Icon::Check))
        } else {
            let n = self.displays.len().max(1);
            // Notes of any kind: PDF notes pages, the .note.md file, or \note{} in the source.
            let notes = self.deck.slides.iter().any(|x| x.notes.is_some()) || (0..self.deck.frames.len()).any(|f| self.note_for(f).is_some());
            let parts = format!(
                "{n} display{}, {}, {}",
                if n == 1 { "" } else { "s" },
                if self.side.target_minutes > 0.0 { "timer set" } else { "no duration set" },
                if notes { "notes found" } else { "no notes" }
            );
            if n >= 2 {
                (format!("Ready: {parts}"), good, Some(Icon::Check))
            } else {
                (parts, calm, None)
            }
        };
        // The time of day, between the ready information and Present.
        let g = p.layout_no_wrap(wall_clock(), FontId::monospace(17.0 * k), MUTED);
        let clock_x = br.left() - 18.0 * k - g.size().x;
        p.galley(pos2(clock_x, cy - g.size().y / 2.0), g, MUTED);
        let pill_r = pill(&p, pos2(clock_x - 18.0 * k, cy), true, &text, 14.5 * k, fill, ink, icon);

        // The source and the PDF, as far as there is room.
        let name = |f: &std::path::Path| f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mut files: Vec<String> = vec![];
        if let Some(src) = &self.src {
            files.push(name(&src.path));
        }
        if let Some(pdf) = &self.pdf_path {
            files.push(name(pdf));
        }
        let clip = p.with_clip_rect(Rect::from_min_max(r.min, pos2(pill_r.left() - 10.0, r.bottom())));
        let mut x = r.left();
        for (i, f) in files.into_iter().enumerate() {
            let (family, color) = if i == 0 { (FontFamily::Name("strong".into()), INK) } else { (FontFamily::Proportional, MUTED) };
            let g = clip.layout_no_wrap(f, FontId::new(16.0 * k, family), color);
            let w = g.size().x;
            clip.galley(pos2(x, cy - g.size().y / 2.0), g, color);
            x += w + 14.0 * k;
        }
    }

    /// The drawing tools beside the large slide. They work like the same tools in the toolbar.
    fn tool_palette(&mut self, ui: &mut Ui, at: Pos2, k: f32) {
        use crate::settings::Action;
        let tools = [
            (Tool::Pointer, Icon::Pointer, Action::Pointer, "Pointer (V)"),
            (Tool::Pen, Icon::Pen, Action::Pen, "Pen (D)"),
            (Tool::Highlighter, Icon::Marker, Action::Highlighter, "Marker (H)"),
            (Tool::Laser, Icon::Laser, Action::Laser, "Laser pointer (L)"),
            (Tool::Eraser, Icon::Eraser, Action::Eraser, "Eraser (X)"),
            (Tool::Zoom, Icon::ZoomBox, Action::ZoomTool, "Zoom into an area (Z)"),
        ];
        let b = 42.0 * k;
        let space = 6.0 * k;
        let inset = 6.0 * k;
        let frame = Rect::from_min_size(at, vec2(b + inset * 2.0, b * 6.0 + space * 5.0 + inset * 2.0));
        let p = ui.painter().clone();
        p.rect_filled(frame.translate(vec2(0.0, 3.0)).expand(2.0), 16.0, Color32::from_black_alpha(10));
        p.rect_filled(frame, 14.0, SURFACE);
        p.rect_stroke(frame, 14.0, Stroke::new(1.0, CARD_LINE), StrokeKind::Inside);
        for (i, (tool, icon, action, tip)) in tools.into_iter().enumerate() {
            let r = Rect::from_min_size(frame.min + vec2(inset, inset + i as f32 * (b + space)), Vec2::splat(b));
            let on = self.tool == tool;
            if soft_button(ui, r, &format!("palette-{i}"), tip, true, on).clicked() {
                let ctx = ui.ctx().clone();
                self.run(action, &ctx);
            }
            icons::paint(&p, Rect::from_center_size(r.center(), Vec2::splat(20.0 * k)), icon, if on { LEAF_DARK } else { INK });
        }
    }

    /// Previous and next, the slide number and the overlay step, under the main column.
    fn nav_row(&mut self, ui: &mut Ui, r: Rect, s: f32) {
        use crate::settings::Action;
        let k = s.min(1.2);
        let p = ui.painter().clone();
        let frames = self.deck.frames.len().max(1);
        let talk = self.deck.talk_frames().max(1);
        let number = format!("{} / {frames}", self.frame() + 1);
        let steps = self.deck.frames.get(self.frame()).map(|f| f.last - f.first + 1).unwrap_or(1);
        let step = self.deck.slides.get(self.cur).map(|x| x.step + 1).unwrap_or(1);
        let mut tag = String::new();
        if self.frame() >= talk {
            tag += "backup slide";
        }
        if steps > 1 {
            tag += &format!("{}step {step} of {steps}", if tag.is_empty() { "" } else { ", " });
        }
        let g1 = p.layout_no_wrap(number, FontId::new(20.0 * k, FontFamily::Name("medium".into())), INK);
        let g2 = p.layout_no_wrap(tag.clone(), FontId::proportional(17.0 * k), if self.frame() >= talk { WARN } else { MUTED });
        let (bw, bh) = (56.0 * k, (40.0 * k).min(r.height() - 8.0));
        let space = 18.0 * k;
        // The timer sits left of the previous button; in a narrow panel the step text gives way first.
        let timer_w = self.timer_width(&p, s);
        let timer_gap = 28.0 * k;
        let (w1, w2) = (g1.size().x, g2.size().x);
        let nav_w = |tag_shown: bool| bw * 2.0 + space * 2.0 + w1 + if tag_shown { 12.0 * k + w2 } else { 0.0 };
        let show_tag = !tag.is_empty() && timer_w + timer_gap + nav_w(true) <= r.width();
        let total = timer_w + timer_gap + nav_w(show_tag);
        let cy = r.center().y;
        let mut x = (r.center().x - total / 2.0).max(r.left());
        self.timer_controls(ui, x, cy, s);
        x += timer_w + timer_gap;
        let last = self.deck.slides.len().saturating_sub(1);
        let prev = Rect::from_min_size(pos2(x, cy - bh / 2.0), vec2(bw, bh));
        let ctx = ui.ctx().clone();
        if soft_button(ui, prev, "nav-prev", "Previous (Left arrow)", self.cur > 0, false).clicked() {
            self.run(Action::Prev, &ctx);
        }
        icons::paint(&p, Rect::from_center_size(prev.center(), Vec2::splat(18.0 * k)), Icon::Prev, if self.cur > 0 { INK } else { Color32::from_gray(190) });
        x += bw + space;
        p.galley(pos2(x, cy - g1.size().y / 2.0), g1, INK);
        x += w1 + 12.0 * k;
        if show_tag {
            p.galley(pos2(x, cy - g2.size().y / 2.0), g2, MUTED);
            x += w2;
        } else {
            x -= 12.0 * k;
        }
        x += space;
        let next = Rect::from_min_size(pos2(x, cy - bh / 2.0), vec2(bw, bh));
        if soft_button(ui, next, "nav-next", "Next (Right arrow)", self.cur < last, false).clicked() {
            self.run(Action::Next, &ctx);
        }
        icons::paint(&p, Rect::from_center_size(next.center(), Vec2::splat(18.0 * k)), Icon::Next, if self.cur < last { INK } else { Color32::from_gray(190) });
    }

    /// Bookmark and question badges. These are for the presenter, so the audience window has none.
    pub fn paint_marks(&self, ui: &Ui, r: Rect, frame: usize) {
        let k = (r.width() / 700.0).clamp(0.5, 1.2);
        let p = ui.painter().with_clip_rect(r);
        let mut x = r.right() - 12.0 * k;
        if self.side.bookmarks.contains(&frame) {
            let [red, green, blue] = self.section_color(frame);
            let area = Rect::from_min_size(pos2(x - 20.0 * k, r.top()), vec2(20.0 * k, 34.0 * k));
            // A white edge keeps it visible on a slide of the same colour.
            ribbon(&p, area.expand2(vec2(1.5, 0.0)).with_max_y(area.bottom() + 2.0), Color32::WHITE);
            ribbon(&p, area, Color32::from_rgb(red, green, blue));
            x -= 28.0 * k;
        }
        if self.side.questions.contains(&frame) {
            let c = pos2(x - 12.0 * k, r.top() + 15.0 * k);
            p.circle_filled(c, 12.0 * k, WARN);
            p.text(c, Align2::CENTER_CENTER, "?", FontId::new(16.0 * k, FontFamily::Name("strong".into())), Color32::WHITE);
        }
    }

    pub fn paint_stage(&mut self, ui: &Ui, r: Rect) {
        match self.screen {
            Screen::Live => self.paint_slide(ui, r, self.cur),
            Screen::Black => {
                ui.painter().rect_filled(r, 0.0, Color32::BLACK);
                self.paint_overlays(ui, r, uv_rect(None), self.live());
            }
            Screen::White => {
                ui.painter().rect_filled(r, 0.0, Color32::WHITE);
                self.paint_overlays(ui, r, uv_rect(None), self.live());
            }
        }
        if self.screen == Screen::Live {
            self.paint_marks(ui, r, self.frame());
        }
    }

    pub fn prefetch(&mut self, idx: usize) {
        for k in [idx + 1, idx.saturating_sub(2)] {
            if let Some(s) = self.deck.slides.get(k).cloned() {
                if !self.cache.contains_key(&(s.page, s.region)) {
                    self.texture(s.page, s.region, 900);
                }
            }
        }
    }

    /// Typing changes only the draft; the tick in the toolbar (or Cmd/Ctrl+Enter) saves it, the cross
    /// or Esc drops it.
    fn note_editor(&mut self, ui: &mut Ui, rect: Rect, s: f32) {
        let size = (self.st.notes_size * s).clamp(14.0, 40.0);
        let shown = self.frame();
        let focus = std::mem::take(&mut self.note_focus);
        // A .tex presentation: a box under the editor chooses where the note is saved.
        let rect = if self.src.is_some() && self.note_draft.is_some() {
            let strip = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 30.0), rect.max);
            ui.painter().rect_filled(strip, 0.0, Color32::from_rgb(244, 243, 240));
            let mut row = ui.new_child(UiBuilder::new().max_rect(strip.shrink2(vec2(10.0, 3.0))).layout(Layout::left_to_right(Align::Center)));
            let tip = "On: the note also goes into the .tex file, as \\note{...}, and the slides rebuild in the background. Off: it is saved in the .note.md file beside the presentation. Either way it shows at once.";
            if row.checkbox(&mut self.st.notes_in_tex, "Also save in the .tex file (\\note)").on_hover_text(tip).changed() {
                self.save_layout();
            }
            Rect::from_min_max(rect.min, pos2(rect.right(), strip.top() - 2.0))
        } else {
            rect
        };
        let Some((slide, draft)) = &mut self.note_draft else { return };
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(250, 250, 248));
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.5, LEAF), StrokeKind::Inside);
        let inner = rect.shrink(10.0);
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
        child.set_clip_rect(inner.intersect(ui.clip_rect()));
        if *slide != shown {
            child.label(RichText::new(format!("Editing the notes for slide {}", *slide + 1)).size(13.0).color(WARN));
        }
        ScrollArea::vertical().id_salt("notes-edit").auto_shrink([false, false]).show(&mut child, |ui| {
            let id = sid("notes-editor");
            let ctx = ui.ctx().clone();
            if crate::bidi::has_rtl(draft) {
                // Arabic, Hebrew and similar scripts: an editor that lays lines out right to left. It
                // takes over from the plain box (and its cursor) as soon as such a letter is typed.
                if focus {
                    let end = draft.chars().count();
                    crate::rtl_edit::set_cursor(&ctx, id, end, end);
                    ui.memory_mut(|m| m.request_focus(id));
                }
                let min = ui.available_height();
                crate::rtl_edit::show(ui, id, draft, FontId::proportional(size), Color32::from_rgb(35, 35, 35), min);
                if let Some((cursor, anchor)) = crate::rtl_edit::cursor(&ctx, id) {
                    let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(anchor), egui::text::CCursor::new(cursor))));
                    state.store(&ctx, id);
                }
            } else {
                let rows = (ui.available_height() / (size * 1.3)) as usize;
                let edit = TextEdit::multiline(draft)
                    .id(id)
                    .hint_text("Type your notes. **bold**, *emphasis*, - lists and # headings work.")
                    .font(FontId::proportional(size))
                    .desired_width(f32::INFINITY)
                    .desired_rows(rows.max(2))
                    .frame(Frame::NONE);
                let r = ui.add(edit);
                if focus {
                    r.request_focus();
                }
                if let Some(range) = egui::text_edit::TextEditState::load(&ctx, id).and_then(|s| s.cursor.char_range()) {
                    crate::rtl_edit::set_cursor(&ctx, id, range.primary.index.0, range.secondary.index.0);
                }
            }
        });
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter)) {
            self.save_note_draft();
        }
    }

    /// What the notes area shows for the current slide: the PDF notes page or written notes.
    fn notes_mode(&self) -> NotesView {
        let frame = self.frame();
        let has_pdf = self.deck.slides.get(self.cur).and_then(|x| x.notes).is_some();
        match self.notes_view {
            NotesView::Auto if self.notes.contains_key(&frame) || self.note_pending.contains_key(&frame) => NotesView::Text,
            NotesView::Auto if has_pdf => NotesView::Pdf,
            NotesView::Auto => NotesView::Text,
            NotesView::Pdf if !has_pdf => NotesView::Text,
            m => m,
        }
    }

    /// Width over height of the PDF notes page the notes area is showing now, or None when it shows
    /// written notes (or the note editor), which can take any shape.
    pub fn notes_page_aspect(&self) -> Option<f32> {
        if self.note_draft.is_some() || self.notes_mode() != NotesView::Pdf {
            return None;
        }
        let (p, reg) = self.deck.slides.get(self.cur)?.notes?;
        let (pw, ph) = self.doc.as_ref().map(|d| (d.pages[p].width, d.pages[p].height)).unwrap_or((4.0, 3.0));
        Some((if reg == Region::Full { pw / ph } else { pw / 2.0 / ph }).max(0.1))
    }

    pub fn notes_panel(&mut self, ui: &mut Ui, rect: Rect, s: f32) {
        let frame = self.frame();
        let sl = self.deck.slides.get(self.cur).cloned();
        let text = self.note_for(frame);
        if self.note_draft.is_some() {
            self.note_editor(ui, rect, s);
            return;
        }
        let mode = self.notes_mode();
        match mode {
            NotesView::Pdf => {
                let (p, reg) = sl.and_then(|x| x.notes).unwrap();
                let a = self.notes_page_aspect().unwrap_or(4.0 / 3.0);
                // Centred across the box, so a box wider than the page has no empty strip on one side.
                let r = fit(rect, a, true);
                // The notes page has its own zoom, for reading it from further away.
                let uv = uv_rect(self.notes_zoom);
                self.paint_page(ui, r, p, reg, uv);
                let resp = ui.interact(r, sid("notes-page"), Sense::click_and_drag());
                pan_zoom(ui, &resp, r, &mut self.notes_zoom, &mut self.zoom_changed);
                // A faint frame shows where the notes page ends, the same as around the slide.
                ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, LINE), StrokeKind::Outside);
            }
            _ => {
                let inner = rect.shrink(14.0 * s.max(0.6));
                let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
                child.set_clip_rect(inner.intersect(ui.clip_rect()));
                let mut write = false;
                let size = (self.st.notes_size * s).max(11.0);
                // The source has \note{} commands, but its frames cannot be matched to the slides.
                let unmatched = !self.source_lines_up() && self.src.as_ref().is_some_and(|x| x.frames.iter().any(|f| !f.note.is_empty()));
                let out = ScrollArea::vertical().id_salt("notes-text").auto_shrink([false, false]).show(&mut child, |ui| match &text {
                    Some(t) => markdown(ui, t, size),
                    None => {
                        ui.label(RichText::new("No notes on this slide yet.").size((20.0 * s).max(11.0)).color(MUTED));
                        if unmatched {
                            let why = "The .tex file has \\note{} commands, but its frames cannot be matched to these slides one for one (some frames are in separate files or made by a macro), so they are not shown here. Notes written here are saved in the .note.md file.";
                            ui.label(RichText::new(why).size((13.0 * s).max(10.5)).color(MUTED));
                        }
                        write = ui.button("Write notes").clicked();
                    }
                });
                if write {
                    self.begin_note_edit();
                }
                // Text resizes instead of zooming: the wheel scrolls long notes, and otherwise
                // sets the size, as pinch and Cmd or Ctrl with the wheel always do.
                if ui.rect_contains_pointer(rect) {
                    let (wheel, pinch) = ui.input(|i| (i.smooth_scroll_delta().y, i.zoom_delta()));
                    let long = out.content_size.y > out.inner_rect.height() + 1.0;
                    let factor = if long { pinch } else { (wheel * 0.002).exp() * pinch };
                    if (factor - 1.0).abs() > 0.0005 {
                        self.st.notes_size = (self.st.notes_size * factor).clamp(14.0, 60.0);
                    }
                }
            }
        }
    }

    /// The talk's sections as a timeline with where you are, then the pace.
    fn bottom_bar(&mut self, ui: &mut Ui, r: Rect, s: f32) {
        let k = s.min(1.2);
        let p = ui.painter().clone();
        p.hline(r.x_range(), r.top(), Stroke::new(1.0, CARD_LINE));
        let line = Rect::from_min_size(pos2(r.left(), r.top() + 12.0 * k), vec2(r.width(), 30.0 * k));
        self.section_timeline(ui, line, k);
        self.timer_row(ui, Rect::from_min_max(pos2(r.left(), line.bottom() + 6.0 * k), pos2(r.right(), r.bottom() - 4.0 * k)), s);

        // Messages sit above the bar for a few seconds, clear of the numbers.
        if self.status_seen.0 != self.status {
            self.status_seen = (self.status.clone(), std::time::Instant::now());
        }
        if !self.status.is_empty() && self.status_seen.1.elapsed().as_secs_f32() > 8.0 && self.doc.is_some() {
            self.status.clear();
        }
        // The last compile's errors stay in view until the file compiles cleanly.
        let compile_problem = (self.src.is_some() && self.compile_ok == Some(false) && !self.compiling).then(|| {
            let first = self.problems.iter().find(|p| p.is_error).map(|p| format!("{}{}", p.line.map(|l| format!("line {l}: ")).unwrap_or_default(), p.message));
            format!("LaTeX: {}  Fix it in your editor and save; the slides rebuild.", first.unwrap_or_else(|| self.compile_note.clone()))
        });
        let msg = self.engine_error.clone().or(if self.status.is_empty() { compile_problem.clone() } else { None }).unwrap_or_else(|| self.status.clone());
        if !msg.is_empty() {
            let color = if self.engine_error.is_some() { BAD } else if compile_problem.as_ref() == Some(&msg) { Color32::from_rgb(255, 196, 120) } else { Color32::WHITE };
            let g = p.layout_no_wrap(msg, FontId::proportional(14.0), color);
            let box_r = Rect::from_min_size(pos2(r.left() + 8.0, r.top() - g.size().y - 20.0), g.size() + vec2(18.0, 10.0));
            p.rect_filled(box_r, 6.0, Color32::from_rgba_unmultiplied(40, 40, 40, 225));
            p.galley(box_r.min + vec2(9.0, 5.0), g, color);
        }
    }

    /// One block per section, as wide as its share of the slides, in the section's colour. A black
    /// line marks the current slide and small ribbons the bookmarks. Click a block to go there.
    fn section_timeline(&mut self, ui: &mut Ui, r: Rect, k: f32) {
        let n = self.deck.frames.len();
        if n == 0 {
            return;
        }
        let talk = self.deck.talk_frames().clamp(1, n);
        let mut starts: Vec<(String, usize)> = self.deck.sections.iter().filter(|x| x.first_frame < talk).map(|x| (x.title.clone(), x.first_frame)).collect();
        if starts.first().is_none_or(|f| f.1 > 0) {
            starts.insert(0, (if starts.is_empty() { "Slides" } else { "Opening" }.to_string(), 0));
        }
        let mut segs: Vec<(String, usize, usize, [u8; 3])> = vec![];
        for (i, (t, a)) in starts.iter().enumerate() {
            let b = starts.get(i + 1).map(|x| x.1).unwrap_or(talk);
            if b > *a {
                segs.push((t.clone(), *a, b, self.section_color(*a)));
            }
        }
        if talk < n {
            segs.push(("Backup".to_string(), talk, n, [140, 110, 70]));
        }
        let space = 4.0 * k;
        let room = r.width() - space * segs.len().saturating_sub(1) as f32;
        let p = ui.painter().clone();
        let mut x = r.left();
        let mut placed: Vec<(Rect, usize, usize)> = vec![];
        let mut go = None;
        for (i, (title, a, b, c)) in segs.iter().enumerate() {
            let seg = Rect::from_min_size(pos2(x, r.top()), vec2(room * (b - a) as f32 / n as f32, r.height()));
            let tip = if b - a == 1 { format!("{title}: slide {}", a + 1) } else { format!("{title}: slides {} to {}", a + 1, b) };
            let resp = ui.interact(seg, sid(&format!("timeline-{i}")), Sense::click()).on_hover_cursor(CursorIcon::PointingHand).on_hover_text(format!("{tip}. Click to go there."));
            p.rect_filled(seg, 6.0 * k, soften(*c, if resp.hovered() { 0.6 } else { 0.78 }));
            // A name too long for its block is shortened with an ellipsis, or left out.
            let font = FontId::proportional(15.0 * k);
            let chars: Vec<char> = title.chars().collect();
            let mut keep = chars.len();
            let shown = loop {
                let t = if keep == chars.len() { title.clone() } else { format!("{}\u{2026}", chars[..keep].iter().collect::<String>().trim_end()) };
                if keep < 3 && keep != chars.len() {
                    break String::new();
                }
                if p.layout_no_wrap(t.clone(), font.clone(), INK).size().x <= seg.width() - 14.0 * k {
                    break t;
                }
                keep -= 1;
            };
            p.text(pos2(seg.left() + 10.0 * k, seg.center().y), Align2::LEFT_CENTER, shown, font, deepen(*c));
            if resp.clicked() {
                go = self.deck.slides.iter().position(|s| s.frame == *a);
            }
            placed.push((seg, *a, *b));
            x = seg.right() + space;
        }
        let at = |f: usize, part: f32| {
            placed.iter().find(|(_, a, b)| f >= *a && f < *b).map(|(seg, a, b)| seg.left() + seg.width() * ((f - a) as f32 + part) / (b - a) as f32)
        };
        for f in self.side.bookmarks.iter().copied().filter(|f| *f < n) {
            if let Some(bx) = at(f, 0.5) {
                let [red, green, blue] = self.section_color(f);
                ribbon(&p, Rect::from_min_size(pos2(bx - 4.0 * k, r.top()), vec2(8.0 * k, 12.0 * k)), Color32::from_rgb(red, green, blue));
            }
        }
        if let Some(hx) = at(self.frame(), 0.5) {
            p.rect_filled(Rect::from_center_size(pos2(hx, r.center().y), vec2(3.0 * k, r.height() + 8.0 * k)), 1.5, INK);
        }
        if let Some(slide) = go {
            self.jump(slide);
        }
    }

    /// How wide `timer_controls` will be, so the row under the main panel can be laid out.
    fn timer_width(&self, p: &Painter, s: f32) -> f32 {
        let k = s.min(1.2);
        let t: String = fmt_time(self.elapsed as f32).chars().map(|c| if c.is_ascii_digit() { '8' } else { c }).collect();
        let time_w = p.layout_no_wrap(t, FontId::new(22.0 * k, FontFamily::Name("strong".into())), INK).size().x;
        let target = self.side.target_minutes;
        let dur = if target > 0.0 { format!("of {}", fmt_time(target * 60.0)) } else { "Set duration".to_string() };
        let dur_w = p.layout_no_wrap(dur, FontId::proportional(18.0 * k), MUTED).size().x;
        let btn = (32.0 * s).clamp(26.0, 40.0);
        30.0 * k + time_w + 12.0 * k + dur_w + 16.0 * k + btn * 3.0 + 6.0
    }

    /// Time elapsed (click to pause), the duration (click to set it), and play, pause and stop,
    /// starting at `x0` on the line `cy`.
    fn timer_controls(&mut self, ui: &mut Ui, x0: f32, cy: f32, s: f32) {
        let k = s.min(1.2);
        let p = ui.painter().clone();
        let target = self.side.target_minutes;
        let mut x = x0;

        // A small clock face.
        let face = pos2(x + 10.0 * k, cy);
        p.circle_stroke(face, 9.0 * k, Stroke::new(1.8 * k, MUTED));
        p.line_segment([face, face + vec2(0.0, -5.0 * k)], Stroke::new(1.6 * k, MUTED));
        p.line_segment([face, face + vec2(3.5 * k, 2.0 * k)], Stroke::new(1.6 * k, MUTED));
        x += 30.0 * k;

        let mut color = INK;
        if target > 0.0 {
            let left = target as f64 * 60.0 - self.elapsed;
            if left < 0.0 {
                color = BAD;
            } else if left < target as f64 * 6.0 {
                color = WARN;
            }
        }
        let t = fmt_time(self.elapsed as f32);
        let font = FontId::new(22.0 * k, FontFamily::Name("strong".into()));
        // Kept as wide as its widest digits, so the rest of the row does not shuffle every second.
        let width = p.layout_no_wrap(t.chars().map(|c| if c.is_ascii_digit() { '8' } else { c }).collect(), font.clone(), color).size().x;
        let g = p.layout_no_wrap(t, font, color);
        let time_r = Rect::from_min_size(pos2(x, cy - g.size().y / 2.0), vec2(width, g.size().y));
        let tip = if self.timer_on { "Click to pause the timer" } else { "Click to start the timer" };
        let hit = ui.interact(time_r.expand(5.0), sid("time-elapsed"), Sense::click()).on_hover_text(tip).on_hover_cursor(CursorIcon::PointingHand);
        if hit.hovered() {
            p.rect_filled(time_r.expand(5.0), 6.0, LEAF_SOFT);
        }
        p.galley(time_r.min, g, color);
        if hit.clicked() {
            let c = ui.ctx().clone();
            self.run(crate::settings::Action::ToggleTimer, &c);
        }
        x = time_r.right() + 12.0 * k;

        let dur = if target > 0.0 { format!("of {}", fmt_time(target * 60.0)) } else { "Set duration".to_string() };
        let g = p.layout_no_wrap(dur, FontId::proportional(18.0 * k), MUTED);
        let dur_r = Rect::from_min_size(pos2(x, cy - g.size().y / 2.0), g.size());
        let dur_hit = ui.interact(dur_r.expand(5.0), sid("duration"), Sense::click()).on_hover_text("Click to set the talk duration").on_hover_cursor(CursorIcon::PointingHand);
        if dur_hit.hovered() {
            p.rect_filled(dur_r.expand(5.0), 6.0, LEAF_SOFT);
        }
        p.galley(dur_r.min, g, MUTED);
        self.duration_rect = dur_r.expand(5.0);
        if dur_hit.clicked() {
            self.duration_open = !self.duration_open;
        }
        x = dur_r.right() + 16.0 * k;

        let btn = (32.0 * s).clamp(26.0, 40.0);
        let buttons = Rect::from_min_size(pos2(x, cy - btn / 2.0), vec2(btn * 3.0 + 6.0, btn));
        let mut row = ui.new_child(UiBuilder::new().max_rect(buttons).layout(Layout::left_to_right(Align::Center)));
        row.spacing_mut().item_spacing.x = 3.0;
        let glyph = btn * 0.55;
        if sized_button(&mut row, Icon::Play, self.timer_on, "Start or resume the timer", btn, glyph).clicked() {
            self.start_timer();
        }
        if sized_button(&mut row, Icon::Pause, !self.timer_on && self.elapsed > 0.0, "Pause the timer", btn, glyph).clicked() {
            self.pause_timer();
        }
        if sized_button(&mut row, Icon::Stop, false, "Stop and reset the timer", btn, glyph).clicked() {
            self.stop_timer();
        }
    }

    /// Under the sections: the pace (time left, ahead, behind or over), whether the timer is paused,
    /// and a way to catch up when behind.
    fn timer_row(&mut self, ui: &mut Ui, r: Rect, s: f32) {
        let k = s.min(1.2);
        let p = ui.painter().clone();
        let cy = r.center().y;
        let target = self.side.target_minutes;
        let talk = self.deck.talk_frames().max(1);
        let mut x = r.left();

        // The pace: over time, behind or ahead of an even pace, or the time left.
        let mut behind = 0.0;
        if target > 0.0 && self.elapsed > 1.0 {
            let left = target * 60.0 - self.elapsed as f32;
            let done = self.frame() as f32 / talk as f32;
            let should = (self.elapsed as f32 / (target * 60.0)).min(1.0);
            let diff = (should - done) * target * 60.0;
            let (msg, fill, ink) = if left < 0.0 {
                (format!("{} over", fmt_time(-left)), Color32::from_rgb(246, 214, 214), BAD)
            } else if diff.abs() > 20.0 {
                if diff > 0.0 {
                    behind = diff;
                    (format!("{} behind", fmt_time(diff)), Color32::from_rgb(244, 222, 171), Color32::from_rgb(107, 74, 10))
                } else {
                    (format!("{} ahead", fmt_time(-diff)), LEAF_SOFT, LEAF_DARK)
                }
            } else {
                (format!("{} left", fmt_time(left)), Color32::from_rgb(234, 233, 230), MUTED)
            };
            x = pill(&p, pos2(x, cy), false, &msg, 16.0 * k, fill, ink, None).right() + 14.0 * k;
        }
        if !self.timer_on && self.elapsed > 0.0 {
            let g = p.layout_no_wrap("paused".to_string(), FontId::proportional(16.0 * k), MUTED);
            let w = g.size().x;
            p.galley(pos2(x, cy - g.size().y / 2.0), g, MUTED);
            x += w + 12.0 * k;
        }

        // Behind an even pace: offer to leave out the next slides that would make up the time.
        let f = self.frame();
        if behind > 0.0 && f + 2 < talk {
            let per = target * 60.0 / talk as f32;
            let skip = ((behind / per.max(1.0)).ceil() as usize).clamp(1, 3).min(talk - f - 2);
            let land = f + 1 + skip;
            let text = if skip == 1 { format!("Skip slide {} to catch up?", f + 2) } else { format!("Skip slides {} to {} to catch up?", f + 2, f + 1 + skip) };
            let g = p.layout_no_wrap(text, FontId::proportional(17.0 * k), INK);
            let bh = (38.0 * k).min(r.height() - 4.0);
            let br = Rect::from_min_size(pos2(r.right() - g.size().x - 32.0 * k, cy - bh / 2.0), vec2(g.size().x + 32.0 * k, bh));
            if br.left() > x {
                let tip = format!("Goes on to slide {}, leaving out the slides before it.", land + 1);
                if soft_button(ui, br, "catch-up", &tip, true, false).clicked() {
                    if let Some(slide) = self.deck.slides.iter().position(|s| s.frame == land) {
                        self.jump(slide);
                    }
                }
                p.galley(pos2(br.left() + 16.0 * k, cy - g.size().y / 2.0), g, INK);
            }
        }
    }

    pub fn duration_popup(&mut self, ctx: &Context) {
        if !self.duration_open {
            return;
        }
        let w = 262.0;
        let at = pos2(self.duration_rect.left().max(8.0), self.duration_rect.top() - 182.0);
        let inner = Area::new(sid("duration-popup")).order(Order::Foreground).fixed_pos(at).show(ctx, |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(w - 20.0);
                ui.label(RichText::new("Talk duration").size(15.0).family(FontFamily::Name("medium".into())).color(INK));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                    for m in [5.0_f32, 10.0, 15.0, 20.0, 30.0, 45.0, 60.0, 90.0] {
                        if ui.add(Button::selectable(self.side.target_minutes == m, format!("{m:.0} min"))).clicked() {
                            self.side.target_minutes = m;
                            self.save_side();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Other");
                    ui.add(DragValue::new(&mut self.side.target_minutes).range(0.0..=600.0).speed(1.0).suffix(" min"));
                    if ui.button("Clear").clicked() {
                        self.side.target_minutes = 0.0;
                    }
                });
            });
        });
        let on_pane = ctx.pointer_hover_pos().map(|p| self.duration_rect.contains(p)).unwrap_or(false);
        if ctx.input(|i| i.pointer.any_click()) && !inner.response.contains_pointer() && !on_pane {
            self.duration_open = false;
            self.save_side();
        }
    }

    /// Slide and notes side by side in one window, with a divider you can drag.
    pub fn split_ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, BG);
        if self.doc.is_none() {
            self.busy_ui(ui, full);
            return;
        }
        let s = (full.width() / 1388.0).clamp(0.72, 1.7);
        let k = s.min(1.2);
        let pad = 16.0 * k;
        let head_h = (54.0 * s).clamp(44.0, 72.0);
        let bottom = (102.0 * s).clamp(88.0, 140.0);
        let head = Rect::from_min_max(pos2(full.left() + pad, full.top()), pos2(full.right() - pad, full.top() + head_h));
        self.head_row(ui, head, s);
        let body = Rect::from_min_max(pos2(full.left() + pad, head.bottom() + 12.0 * k), pos2(full.right() - pad, full.bottom() - bottom - 10.0 * k));
        let aspect = self.deck.aspect.max(0.1);
        let nav_h = (52.0 * s).clamp(42.0, 66.0);
        let col = self.st.layout.split_col.unwrap_or_else(|| auto_col(body.width(), body.height() - 50.0 - nav_h, aspect, 28.0));
        let x = body.left() + body.width() * col;
        let left = Rect::from_min_max(body.min, pos2(x - 7.0, body.bottom() - nav_h));
        let right = Rect::from_min_max(pos2(x + 7.0, body.top()), body.max);
        let label = 14.0 * k;
        let p = ui.painter().clone();
        let top = card(&p, left, "Audience view", label);
        let slide = fit(Rect::from_min_max(left.min + vec2(14.0, top), left.max - vec2(14.0, 12.0)), aspect, false);
        let cur = self.cur;
        self.paint_stage(ui, slide);
        p.rect_stroke(slide, 2.0, Stroke::new(1.0, CARD_LINE), StrokeKind::Outside);
        self.slide_input(ui, slide, cur, "split-slide");
        self.nav_row(ui, Rect::from_min_max(pos2(left.left(), left.bottom()), pos2(left.right(), body.bottom())), s);

        let next_h = (right.height() * 0.32).min((right.width() - 28.0) / aspect) + label + 34.0;
        // A PDF notes page gets the height that lets it fill the column's width, when there is room.
        let auto_row = match self.notes_page_aspect() {
            Some(na) => (((right.width() - 8.0) / na + label + 6.0) / right.height().max(1.0)).min(1.0 - (next_h + 14.0) / right.height().max(1.0)).max(0.4),
            None => 1.0 - (next_h + 14.0) / right.height().max(1.0),
        };
        let row = self.st.layout.split_row.unwrap_or(auto_row.clamp(0.3, 0.8));
        let cut = right.top() + right.height() * row;
        let notes = Rect::from_min_max(right.min, pos2(right.right(), cut));
        let next = Rect::from_min_max(pos2(right.left(), cut + 14.0), right.max);
        let top = card(&p, notes, "Notes", label);
        self.notes_panel(ui, Rect::from_min_max(notes.min + vec2(4.0, top), notes.max - vec2(4.0, 6.0)), (notes.width() / 1000.0).clamp(0.5, 1.1));
        let top = card(&p, next, "Next slide", label);
        let next_rect = fit(Rect::from_min_max(next.min + vec2(14.0, top), next.max - vec2(14.0, 12.0)), aspect, false);
        self.paint_next(ui, next_rect);
        p.rect_stroke(next_rect, 2.0, Stroke::new(1.0, CARD_LINE), StrokeKind::Outside);

        let handle = Rect::from_center_size(pos2(x, body.center().y), vec2(12.0, body.height()));
        let a = drag_handle(ui, "split-col", handle, true, false);
        let b = drag_handle(ui, "split-row", Rect::from_center_size(pos2(right.center().x, cut + 7.0), vec2(right.width() * 0.5, 12.0)), false, false);
        let moved = a.apply(&mut self.st.layout.split_col, col, a.delta.x / body.width()) | b.apply(&mut self.st.layout.split_row, row, b.delta.y / right.height().max(1.0));
        if moved {
            self.save_layout();
        }
        self.bottom_bar(ui, Rect::from_min_max(pos2(full.left() + pad, full.bottom() - bottom), pos2(full.right() - pad, full.bottom())), s);
        let ctx = ui.ctx().clone();
        self.peek_popup(&ctx);
    }

    /// Next-slide preview; clicking it moves on, and the wheel looks closer.
    fn paint_next(&mut self, ui: &mut Ui, r: Rect) {
        let next = self.cur + 1;
        if next < self.deck.slides.len() {
            let sl = self.deck.slides[next].clone();
            self.paint_page(ui, r, sl.page, sl.region, uv_rect(self.next_zoom));
            let resp = ui.interact(r, sid("next-click"), Sense::click_and_drag());
            pan_zoom(ui, &resp, r, &mut self.next_zoom, &mut self.zoom_changed);
            self.paint_marks(ui, r, sl.frame);
            if resp.clicked() {
                self.goto(next);
            }
            self.prefetch(next);
        } else {
            ui.painter().rect_filled(r, 0.0, Color32::from_rgb(235, 235, 233));
            ui.painter().text(r.center(), Align2::CENTER_CENTER, "End of slides", FontId::proportional(15.0), MUTED);
        }
    }

    fn busy_ui(&mut self, ui: &mut Ui, full: Rect) {
        // Without its PDF engine TeXora cannot show slides; say so instead of waiting for ever.
        if let Some(e) = self.engine_error.clone() {
            let os = crate::cli::Os::current();
            let mut child = ui.new_child(UiBuilder::new().max_rect(full.shrink(40.0)));
            child.vertical_centered(|ui| {
                ui.add_space(full.height() * 0.3);
                let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                crate::icons::paint(ui.painter(), r, crate::icons::Icon::Warn, BAD);
                ui.label(RichText::new("TeXora's PDF engine could not start").size(20.0).color(INK));
                ui.label(RichText::new(crate::syscheck::engine_problem(os, &e)).size(15.0).color(MUTED));
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Download TeXora again and keep its files together: {} must stay beside the TeXora program.", crate::syscheck::engine_file(os))).size(14.0).color(MUTED));
                ui.add_space(10.0);
                if ui.button("Open System check").clicked() {
                    self.syscheck_open = true;
                }
            });
            return;
        }
        if !self.compiling && !self.loading {
            // The .tex file did not compile: say why, since there is no editor here to show it in.
            self.view = View::Studio;
            return;
        }
        let mut child = ui.new_child(UiBuilder::new().max_rect(full));
        child.vertical_centered(|ui| {
            ui.add_space(full.height() * 0.4);
            ui.spinner();
            let what = if self.compiling { "Compiling your slides…" } else { "Opening your slides…" };
            ui.label(RichText::new(what).size(17.0).color(MUTED));
        });
    }

    // ---------- audience ----------

    pub fn audience_ui(&mut self, ui: &mut Ui) {
        self.painting_audience = true;
        self.audience_inner(ui);
        self.painting_audience = false;
    }

    fn audience_inner(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, Color32::BLACK);
        let idx = self.live();
        match self.screen {
            Screen::Black => {}
            Screen::White => {
                let r = fit(full, self.deck.aspect.max(0.1), false);
                ui.painter().rect_filled(r, 0.0, Color32::WHITE);
                self.paint_overlays(ui, r, uv_rect(None), idx);
                self.slide_input(ui, r, idx, "audience-board");
            }
            Screen::Live => {
                let r = fit(full, self.deck.aspect.max(0.1), false);
                self.paint_slide(ui, r, idx);
                // On a single screen the fullscreen slide is the only place to point and draw.
                self.slide_input(ui, r, idx, "audience-slide");
                if let Some(s) = self.deck.slides.get(idx + 1).cloned() {
                    let ppp = ui.ctx().pixels_per_point();
                    self.texture(s.page, s.region, (r.width() * ppp) as u32);
                }
            }
        }
    }

    pub fn audience_window(&mut self, ctx: &Context) {
        if !self.audience_open {
            return;
        }
        self.audience_age += 1;
        // The room keeps seeing the presentation even when another tab is in front.
        let front = self.active_id();
        if let Some(i) = self.tab_index(self.audience_tab) {
            self.activate(i);
        }
        let multi = self.displays.len() >= 2;
        let mut vb = ViewportBuilder::default().with_title("TeXora Audience View").with_inner_size([960.0, 720.0]);
        if multi {
            if let Some(d) = self.displays.get(self.display) {
                let scale = if cfg!(target_os = "macos") { 1.0 } else { d.scale_factor.max(1.0) };
                vb = vb.with_position(pos2(d.x as f32 / scale + 20.0, d.y as f32 / scale + 20.0));
            }
        }
        let id = audience_id();
        if multi && self.audience_age == 12 && !self.audience_full {
            self.audience_full = true;
            ctx.send_viewport_cmd_to(id, ViewportCommand::Fullscreen(true));
        }
        ctx.show_viewport_immediate(id, vb, |ui, _class| {
            let c = ui.ctx().clone();
            Frame::NONE.show(ui, |ui| {
                self.keys(&c);
                self.audience_ui(ui);
            });
            if c.input(|i| i.viewport().close_requested()) {
                self.audience_open = false;
                self.audience_full = false;
            }
            let idle = c.input(|i| i.pointer.time_since_last_movement()) > 2.0;
            if self.audience_full && self.tool == Tool::Pointer && idle {
                c.set_cursor_icon(CursorIcon::None);
            }
        });
        if let Some(i) = self.tab_index(front) {
            self.activate(i);
        }
    }

    // ---------- overview ----------

    pub fn overview_ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, BG);
        let n = self.deck.frames.len();
        let cols = ((full.width() / 250.0) as usize).clamp(3, 8);
        let cell_w = (full.width() - 32.0) / cols as f32;
        let aspect = self.deck.aspect.max(0.1);
        let thumb = vec2(cell_w - 14.0, (cell_w - 14.0) / aspect);
        let cur = self.frame();
        let inner = full.shrink(10.0);
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
        child.horizontal(|ui| {
            ui.label(RichText::new("Slide overview").size(17.0).color(INK));
            ui.label(RichText::new("Click a slide to go there. Esc returns.").size(12.0).color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Back to presenter").clicked() {
                    self.view = View::Presenter;
                }
            });
        });
        child.add_space(6.0);
        ScrollArea::vertical().id_salt("overview").show(&mut child, |ui| {
            let mut f = 0;
            while f < n {
                if let Some(sec) = self.deck.sections.iter().find(|x| x.first_frame == f).cloned() {
                    ui.label(RichText::new(sec.title).size(14.0).color(LEAF_DARK));
                }
                if self.deck.backup_start == Some(f) {
                    ui.label(RichText::new("Backup slides").size(14.0).color(WARN));
                }
                let row_end = (f + cols).min(n);
                let stop = (f + 1..row_end)
                    .find(|k| self.deck.sections.iter().any(|x| x.first_frame == *k) || self.deck.backup_start == Some(*k))
                    .unwrap_or(row_end);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for k in f..stop {
                        let (rect, resp) = ui.allocate_exact_size(thumb + vec2(0.0, 18.0), Sense::click());
                        let pic = Rect::from_min_size(rect.min, thumb);
                        if ui.is_rect_visible(rect) {
                            let fr = self.deck.frames[k].clone();
                            let sl = self.deck.slides[fr.last].clone();
                            self.paint_page(ui, pic, sl.page, sl.region, uv_rect(None));
                            self.paint_marks(ui, pic, k);
                            let (col, wid) = if k == cur {
                                (LEAF, 3.0)
                            } else if resp.hovered() {
                                (LEAF, 1.5)
                            } else {
                                (LINE, 1.0)
                            };
                            ui.painter().rect_stroke(pic, 2.0, Stroke::new(wid, col), StrokeKind::Outside);
                            let steps = fr.last - fr.first + 1;
                            let cap = if steps > 1 { format!("{}  {}  ({steps} steps)", k + 1, fr.title) } else { format!("{}  {}", k + 1, fr.title) };
                            let g = ui.painter().layout(cap, FontId::proportional(11.5), MUTED, thumb.x);
                            ui.painter().with_clip_rect(rect).galley(pos2(rect.left(), pic.bottom() + 3.0), g, MUTED);
                        }
                        if resp.clicked() {
                            let t = self.deck.frames[k].first;
                            self.jump(t);
                            self.view = View::Presenter;
                        }
                    }
                });
                ui.add_space(8.0);
                f = stop;
            }
        });
    }
}

pub fn wall_clock() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let local = secs + utc_offset();
    format!("{:02}:{:02}:{:02}", (local / 3600).rem_euclid(24), (local / 60).rem_euclid(60), local.rem_euclid(60))
}

fn utc_offset() -> i64 {
    use std::sync::OnceLock;
    static OFFSET: OnceLock<i64> = OnceLock::new();
    *OFFSET.get_or_init(|| {
        #[cfg(unix)]
        {
            // `date +%z` gives the local offset without pulling in a calendar library.
            if let Ok(o) = std::process::Command::new("date").arg("+%z").output() {
                let t = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if t.len() == 5 {
                    let sign = if t.starts_with('-') { -1 } else { 1 };
                    let h: i64 = t[1..3].parse().unwrap_or(0);
                    let m: i64 = t[3..5].parse().unwrap_or(0);
                    return sign * (h * 3600 + m * 60);
                }
            }
        }
        #[cfg(windows)]
        {
            let script = "[int][TimeZoneInfo]::Local.GetUtcOffset([DateTime]::Now).TotalSeconds";
            if let Ok(o) = std::process::Command::new("powershell").args(["-NoProfile", "-Command", script]).output() {
                if let Ok(v) = String::from_utf8_lossy(&o.stdout).trim().parse::<i64>() {
                    return v;
                }
            }
        }
        0
    })
}

/// Small Markdown renderer for notes. Every line is laid out here at the
/// width available, so text wraps inside its panel and can never push the panel wider.
pub fn markdown(ui: &mut Ui, src: &str, size: f32) {
    use egui::text::{LayoutJob, TextFormat};
    let width = ui.available_width().max(40.0);
    let strong_font = FontId::new(size, FontFamily::Name("strong".into()));
    let mut in_code = false;
    for raw in src.lines() {
        let line = raw.trim_end();
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            let mut job = LayoutJob::default();
            job.wrap.max_width = width;
            let f = TextFormat { font_id: FontId::monospace(size * 0.86), color: Color32::from_rgb(35, 35, 35), ..Default::default() };
            job.append(if line.is_empty() { " " } else { line }, 0.0, f);
            let g = ui.fonts_mut(|f| f.layout_job(job));
            ui.add(Label::new(g).selectable(true));
            continue;
        }
        if line.trim().is_empty() {
            ui.add_space(size * 0.4);
            continue;
        }
        let (body, sz, heading) = if let Some(h) = line.strip_prefix("### ") {
            (h, size * 1.08, true)
        } else if let Some(h) = line.strip_prefix("## ") {
            (h, size * 1.18, true)
        } else if let Some(h) = line.strip_prefix("# ") {
            (h, size * 1.3, true)
        } else {
            (line, size, false)
        };
        let (body, indent) = match body.trim_start().strip_prefix("- ").or(body.trim_start().strip_prefix("* ")) {
            Some(b) => (b, size * 0.9),
            None => (body, 0.0),
        };
        let mut job = LayoutJob::default();
        job.wrap.max_width = (width - indent).max(20.0);
        let base = TextFormat { font_id: FontId::proportional(sz), color: Color32::from_rgb(35, 35, 35), ..Default::default() };
        let strong = TextFormat { color: Color32::BLACK, font_id: FontId::new(sz, strong_font.family.clone()), ..base.clone() };
        let em = TextFormat { italics: true, ..base.clone() };
        let code = TextFormat { font_id: FontId::monospace(sz * 0.9), background: Color32::from_gray(218), ..base.clone() };
        let head = TextFormat { color: LEAF_DARK, font_id: FontId::new(sz, strong_font.family.clone()), ..base.clone() };
        // The line's pieces with their style: 0 plain (or heading), 1 bold, 2 code, 3 emphasis.
        let formats = [if heading { head.clone() } else { base.clone() }, strong.clone(), code.clone(), em.clone()];
        let mut segs: Vec<(&str, usize)> = vec![];
        let mut rest = body;
        while !rest.is_empty() {
            let next = ["**", "`", "*"].iter().filter_map(|m| rest.find(m).map(|p| (p, *m))).min_by_key(|x| (x.0, std::cmp::Reverse(x.1.len())));
            match next {
                Some((p, m)) if rest[p + m.len()..].contains(m) => {
                    segs.push((&rest[..p], 0));
                    let after = &rest[p + m.len()..];
                    let end = after.find(m).unwrap();
                    segs.push((&after[..end], match m {
                        "**" => 1,
                        "`" => 2,
                        _ => 3,
                    }));
                    rest = &after[end + m.len()..];
                }
                _ => {
                    segs.push((rest, 0));
                    rest = "";
                }
            }
        }
        if crate::bidi::has_rtl(body) {
            rtl_paragraph(ui, &segs, &formats, width, indent, sz);
            continue;
        }
        for (t, s) in segs {
            job.append(t, 0.0, formats[s].clone());
        }
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            if indent > 0.0 {
                let (r, _) = ui.allocate_exact_size(vec2(indent, sz * 1.2), Sense::hover());
                ui.painter().circle_filled(pos2(r.left() + indent * 0.45, r.top() + sz * 0.62), (sz * 0.11).max(2.0), LEAF_DARK);
            }
            ui.add(Label::new(galley).selectable(true));
        });
    }
}

/// A paragraph with Arabic, Hebrew or another right-to-left script: wrapped, then each line put in the
/// order it is read, and aligned to the right when the paragraph starts in such a script.
fn rtl_paragraph(ui: &mut Ui, segs: &[(&str, usize)], formats: &[egui::text::TextFormat], width: f32, indent: f32, sz: f32) {
    use egui::text::LayoutJob;
    let plain: String = segs.iter().map(|s| s.0).collect();
    let rtl = crate::bidi::starts_rtl(&plain);
    let chars: Vec<(char, usize)> = segs.iter().flat_map(|(t, s)| t.chars().map(move |c| (c, *s))).collect();
    let max = (width - indent).max(20.0);
    let lines = crate::bidi::wrap(&chars, max, |t, s| ui.fonts_mut(|f| f.layout_no_wrap(t.to_string(), formats[s].font_id.clone(), Color32::BLACK)).size().x);
    for (n, line) in lines.iter().enumerate() {
        let mut job = LayoutJob::default();
        for p in crate::bidi::visual(line, rtl) {
            job.append(&p.text, 0.0, formats[p.style].clone());
        }
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let layout = if rtl { Layout::right_to_left(Align::TOP) } else { Layout::left_to_right(Align::TOP) };
        ui.allocate_ui_with_layout(vec2(width, galley.size().y), layout, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            if indent > 0.0 {
                let (r, _) = ui.allocate_exact_size(vec2(indent, sz * 1.2), Sense::hover());
                if n == 0 {
                    let x = if rtl { r.right() - indent * 0.45 } else { r.left() + indent * 0.45 };
                    ui.painter().circle_filled(pos2(x, r.top() + sz * 0.62), (sz * 0.11).max(2.0), LEAF_DARK);
                }
            }
            ui.add(Label::new(galley).selectable(false));
        });
    }
}

pub fn engine_combo(ui: &mut Ui, e: &mut Engine) {
    ComboBox::from_id_salt("engine").selected_text(e.name()).width(92.0).show_ui(ui, |ui| {
        for x in Engine::ALL {
            ui.selectable_value(e, x, x.name());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The case from the report: a 4:3 notes page in a wide window left an empty strip beside it.
    #[test]
    fn notes_column_fits_its_page_and_the_slides_take_the_rest() {
        let (inner_w, notes_h, slides_w, gap) = (1300.0, 560.0, 330.0, 14.0);
        let share = presenter_auto_col(inner_w, notes_h, Some(4.0 / 3.0), slides_w, gap);
        let notes_w = share * inner_w - gap / 2.0;
        assert!((notes_w - (notes_h * 4.0 / 3.0 + 8.0)).abs() < 1.0, "notes column {notes_w} fits the page");
        assert!(inner_w * (1.0 - share) - gap / 2.0 > slides_w, "slides get the rest");
        // Narrow window: the slide column keeps its width and the notes page shrinks to fit.
        let share = presenter_auto_col(900.0, notes_h, Some(4.0 / 3.0), slides_w, gap);
        assert!(900.0 * (1.0 - share) >= slides_w + gap / 2.0 - 1.0 || share <= 0.4 + 1e-6);
        // Written notes: as before, what is left once the slides fit.
        let share = presenter_auto_col(inner_w, notes_h, None, slides_w, gap);
        assert!((inner_w * (1.0 - share) - gap / 2.0 - slides_w).abs() < 1.0);
    }

    #[test]
    fn a_wider_slide_column_gives_a_bigger_current_slide() {
        let (h, a, chrome, gap) = (700.0, 4.0 / 3.0, 48.0, 14.0);
        let mut last = 0.0;
        for w in [250.0, 350.0, 450.0, 550.0, 650.0, 800.0] {
            let row = current_row(w, h, a, chrome, gap);
            assert!((0.5..=0.7).contains(&row));
            // The current slide's picture: as wide as fits in its share of the height.
            let cur = fit(Rect::from_min_size(Pos2::ZERO, vec2(w - 28.0, h * row - gap / 2.0 - chrome)), a, false);
            assert!(cur.width() + 0.5 >= last, "current slide shrank at column width {w}");
            last = cur.width();
        }
        assert!(current_row(650.0, h, a, chrome, gap) > 0.5, "a wide column gives the current slide more than half");
    }

    /// Long notes wrap at the panel's width and never push it wider.
    #[test]
    fn automatic_fit_gives_a_slide_the_full_height() {
        let share = auto_col(1400.0, 750.0, 4.0 / 3.0, 20.0);
        assert!((share * 1400.0 - 1020.0).abs() < 0.5, "{share}");
    }

    #[test]
    fn automatic_fit_stops_at_the_edges() {
        assert_eq!(auto_col(800.0, 750.0, 16.0 / 9.0, 20.0), 0.85);
        assert_eq!(auto_col(4000.0, 400.0, 4.0 / 3.0, 20.0), 0.45);
    }

    #[test]
    fn dragging_stores_a_divider_and_double_click_returns_to_auto() {
        let mut slot = None;
        let drag = Handle { delta: vec2(10.0, 0.0), double: false, done: true };
        assert!(drag.apply(&mut slot, 0.5, 0.1));
        assert!((slot.unwrap() - 0.6).abs() < 1e-5);
        drag.apply(&mut slot, 0.88, 0.1);
        assert_eq!(slot, Some(0.9));
        let double = Handle { delta: Vec2::ZERO, double: true, done: false };
        assert!(double.apply(&mut slot, 0.5, 0.0));
        assert_eq!(slot, None);
    }

    #[test]
    fn long_text_stays_inside_its_panel() {
        let ctx = Context::default();
        let long = "word ".repeat(200);
        let src = format!("{long}\n- {long}\n# {long}\n**{long}**\n```\n{long}\n```\n{}", "x".repeat(400));
        let mut used = 0.0;
        let mut text_height = 0.0;
        for _ in 0..2 {
            let mut out = ctx.run_ui(RawInput::default(), |ui| {
                ui.scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(Pos2::ZERO, vec2(300.0, 900.0))), |ui| {
                    ui.set_max_width(300.0);
                    let top = ui.next_widget_position().y;
                    markdown(ui, &src, 14.0);
                    used = ui.min_rect().width();
                    text_height = ui.next_widget_position().y - top;
                });
            });
            out.textures_delta.clear();
        }
        assert!(used <= 300.5, "text widened its panel to {used}");
        assert!(text_height > 300.0, "long text should wrap onto many lines, got {text_height}");
    }
}
