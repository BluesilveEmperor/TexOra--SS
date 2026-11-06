//! The presenter screen, the audience screen and everything that draws on a slide.
use crate::analytics::{fmt_time, InkKind, Stroke as InkStroke};
use crate::app::*;
use crate::icons::{ribbon, sized_button, Icon};
use crate::latex::Engine;
use crate::pdf::{LinkTarget, Region};
use egui::epaint::Shape;
use egui::*;

/// Share of the width for the main panel so a slide of this shape fills the height.
pub fn auto_col(width: f32, height: f32, aspect: f32, gap: f32) -> f32 {
    ((height * aspect + gap) / width.max(1.0)).clamp(0.45, 0.85)
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

pub fn labelled_frame(p: &Painter, r: Rect, label: &str, size: f32, color: Color32) {
    p.rect_stroke(r, 0.0, Stroke::new(1.0, LINE), StrokeKind::Inside);
    let galley = p.layout_no_wrap(label.to_string(), FontId::proportional(size), color);
    let at = pos2(r.left(), r.top() - galley.size().y / 2.0);
    p.rect_filled(Rect::from_min_size(at, galley.size() + vec2(3.0, 0.0)), 0.0, BG);
    p.galley(at, galley, color);
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

    /// Pointer behaviour over the live slide: drawing tools, laser, zoom, links and citation peeks.
    pub fn slide_input(&mut self, ui: &mut Ui, rect: Rect, idx: usize, id: &str) {
        let uv = uv_rect(self.zoom);
        let resp = ui.interact(rect, sid(id), Sense::click_and_drag());
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
        let w = full.width();
        let row = 18.0;
        let bottom = (84.0 * s).max(68.0);
        let aspect = self.deck.aspect.max(0.1);
        let body_bottom = full.bottom() - bottom;
        let large = self.swap;
        // The main panel takes the full height unless a divider has been moved.
        let stored = if large { self.st.layout.large_col } else { self.st.layout.presenter_col };
        let split = stored.unwrap_or_else(|| auto_col(w, body_bottom - full.top() - row, aspect, 10.0 + 0.011 * w));
        let div_x = full.left() + w * split;
        let right_x = div_x + 0.012 * w;
        let main = Rect::from_min_max(pos2(full.left() + 10.0, full.top() + row), pos2(div_x - 0.011 * w, body_bottom));
        let main_frame = Rect::from_min_max(main.min - vec2(0.0, 8.0), main.max);
        labelled_frame(ui.painter(), main_frame, if self.swap { "Slide" } else { "Notes" }, 14.0 * s.min(1.2), INK);

        // Right column: current and next slide, each with a label on its frame line.
        let col = Rect::from_min_max(pos2(right_x, full.top() + 9.0), pos2(full.right() - 8.0, body_bottom));
        let row_share = (if large { self.st.layout.large_row } else { self.st.layout.presenter_row }).unwrap_or(0.5);
        let mid = col.top() + col.height() * row_share;
        let top_box = Rect::from_min_max(col.min, pos2(col.right(), mid - 4.0));
        let next_box = Rect::from_min_max(pos2(col.left(), mid + 4.0), col.max);
        let label_size = 13.0 * s.min(1.2);

        let live_tag = match (self.screen, self.frozen.is_some(), self.audience_open) {
            (Screen::Black, _, _) => "audience: black",
            (Screen::White, _, _) => "audience: whiteboard",
            (_, true, _) => "audience: frozen",
            (_, _, true) => "audience: live",
            _ => "",
        };
        let swapped = self.swap;
        labelled_frame(ui.painter(), top_box, if swapped { "Notes" } else { "Current slide" }, label_size, INK);
        labelled_frame(ui.painter(), next_box, "Next slide", label_size, INK);
        if !live_tag.is_empty() {
            let c = if self.screen == Screen::Live && self.frozen.is_none() { LEAF } else { BAD };
            let g = ui.painter().layout_no_wrap(live_tag.to_string(), FontId::proportional(11.0), c);
            let at = pos2(top_box.right() - g.size().x - 2.0, top_box.top() - g.size().y / 2.0);
            ui.painter().rect_filled(Rect::from_min_size(at - vec2(4.0, 0.0), g.size() + vec2(6.0, 0.0)), 0.0, BG);
            ui.painter().galley(at, g, c);
        }
        let cur_rect = fit(top_box.shrink2(vec2(0.0, 10.0)).translate(vec2(0.0, 0.0)), aspect, true);
        let next_rect = fit(next_box.shrink2(vec2(0.0, 10.0)), aspect, true);

        // Main pane: notes by default, or the large slide when swapped.
        let cur = self.cur;
        if swapped {
            let r = fit(main, aspect, false);
            self.paint_stage(ui, r);
            self.slide_input(ui, r, cur, "big-slide");
            self.notes_panel(ui, top_box.shrink2(vec2(0.0, 10.0)), s * 0.55);
        } else {
            self.notes_panel(ui, main, s);
            self.paint_stage(ui, cur_rect);
            self.slide_input(ui, cur_rect, cur, "small-slide");
        }
        if self.tool == Tool::Pointer && self.frozen.is_some() {
            // While frozen the presenter browses privately; show what the audience still sees.
            let f = self.frozen.unwrap_or(0);
            let tag = Rect::from_min_size(cur_rect.left_bottom() + vec2(0.0, 4.0), vec2(cur_rect.width(), 16.0));
            let lbl = self.deck.slides.get(f).map(|x| x.frame + 1).unwrap_or(0);
            ui.painter().text(tag.left_center(), Align2::LEFT_CENTER, format!("Audience stays on slide {lbl}. Press F to release."), FontId::proportional(11.0), BAD);
        }
        self.paint_next(ui, next_rect);

        let across = Rect::from_center_size(pos2(div_x, (full.top() + body_bottom) / 2.0), vec2(12.0, body_bottom - full.top()));
        let a = drag_handle(ui, "main-col", across, true, true);
        let b = drag_handle(ui, "main-row", Rect::from_center_size(pos2(col.center().x, mid), vec2(col.width(), 10.0)), false, false);
        let layout = &mut self.st.layout;
        let (slot_col, slot_row) = if large { (&mut layout.large_col, &mut layout.large_row) } else { (&mut layout.presenter_col, &mut layout.presenter_row) };
        let moved = a.apply(slot_col, split, a.delta.x / w) | b.apply(slot_row, row_share, b.delta.y / col.height().max(1.0));
        if moved {
            self.save_layout();
        }

        self.bottom_bar(ui, Rect::from_min_max(pos2(full.left(), full.bottom() - bottom), full.max), s);
        self.peek_popup(&ctx);
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

    pub fn notes_panel(&mut self, ui: &mut Ui, rect: Rect, s: f32) {
        let frame = self.frame();
        let sl = self.deck.slides.get(self.cur).cloned();
        let has_pdf = sl.as_ref().and_then(|x| x.notes).is_some();
        let text = self.note_for(frame);
        if self.note_draft.is_some() {
            self.note_editor(ui, rect, s);
            return;
        }
        let mode = match self.notes_view {
            NotesView::Auto if self.notes.contains_key(&frame) => NotesView::Text,
            NotesView::Auto if has_pdf => NotesView::Pdf,
            NotesView::Auto => NotesView::Text,
            NotesView::Pdf if !has_pdf => NotesView::Text,
            m => m,
        };
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, LINE), StrokeKind::Inside);
        match mode {
            NotesView::Pdf => {
                let (p, reg) = sl.and_then(|x| x.notes).unwrap();
                let (pw, ph) = self.doc.as_ref().map(|d| (d.pages[p].width, d.pages[p].height)).unwrap_or((4.0, 3.0));
                let a = if reg == Region::Full { pw / ph } else { pw / 2.0 / ph };
                let r = fit(rect, a, true);
                let r = r.translate(vec2(rect.left() - r.left(), 0.0));
                // The notes page has its own zoom, for reading it from further away.
                let uv = uv_rect(self.notes_zoom);
                self.paint_page(ui, r, p, reg, uv);
                let resp = ui.interact(r, sid("notes-page"), Sense::click_and_drag());
                pan_zoom(ui, &resp, r, &mut self.notes_zoom, &mut self.zoom_changed);
                ui.painter().vline(r.right(), r.y_range(), Stroke::new(1.5, Color32::BLACK));
            }
            _ => {
                ui.painter().rect_filled(rect.shrink(1.0), 0.0, Color32::from_rgb(228, 228, 228));
                let inner = rect.shrink(14.0 * s.max(0.6));
                let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
                child.set_clip_rect(inner.intersect(ui.clip_rect()));
                let mut write = false;
                let size = (self.st.notes_size * s).max(11.0);
                let out = ScrollArea::vertical().id_salt("notes-text").auto_shrink([false, false]).show(&mut child, |ui| match &text {
                    Some(t) => markdown(ui, t, size),
                    None => {
                        ui.label(RichText::new("No notes on this slide yet.").size((20.0 * s).max(11.0)).color(MUTED));
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

    fn bottom_bar(&mut self, ui: &mut Ui, r: Rect, s: f32) {
        // Four boxes with the same margins as the panels above, and room below the last one.
        let gap = 10.0;
        let inner = Rect::from_min_max(pos2(r.left() + 10.0, r.top() + 14.0 * s), pos2(r.right() - 8.0, r.bottom() - 12.0));
        let shares = [0.335, 0.42, 0.115, 0.13];
        let room = inner.width() - gap * 3.0;
        let mut boxes = [Rect::NOTHING; 4];
        let mut x = inner.left();
        for (i, share) in shares.iter().enumerate() {
            let width = room * share / shares.iter().sum::<f32>();
            boxes[i] = Rect::from_min_max(pos2(x, inner.top()), pos2(x + width, inner.bottom()));
            x += width + gap;
        }
        let [slide_r, time_r, dur_r, clock_r] = boxes;
        let p = ui.painter().clone();
        let lab = 17.0 * s.min(1.2);
        labelled_frame(&p, slide_r, "Slide number", lab, INK);
        labelled_frame(&p, time_r, "Time elapsed", lab, INK);
        labelled_frame(&p, dur_r, "Duration", lab * 0.92, MUTED);
        labelled_frame(&p, clock_r, "Clock", lab * 0.92, MUTED);
        // Values sit in the middle of their boxes, a little below the label on the top edge.
        let vy = inner.center().y + 3.0 * s;
        // A hover highlight starts below the box's label, which sits on its top edge.
        let below_label = |b: Rect| Rect::from_min_max(pos2(b.left() + 2.0, b.top() + lab * 0.6), b.max - vec2(2.0, 2.0));

        let total = self.deck.slides.len().max(1);
        let sl = self.deck.slides.get(self.cur);
        let label = sl.map(|x| x.frame + 1).unwrap_or(1);
        p.text(pos2(slide_r.center().x, vy), Align2::CENTER_CENTER, format!("{label} ({}/{total})", self.cur + 1), FontId::proportional(30.0 * s), INK);
        // Section and talk progress: quiet context beside the number.
        if let Some(sec) = self.deck.section_of(label - 1) {
            p.text(pos2(slide_r.left() + 10.0, vy), Align2::LEFT_CENTER, &sec.title, FontId::proportional(13.0 * s), MUTED);
        }
        let talk = self.deck.talk_frames().max(1);
        let steps = self.deck.frames.get(self.frame()).map(|f| f.last - f.first + 1).unwrap_or(1);
        let step = self.deck.slides.get(self.cur).map(|x| x.step + 1).unwrap_or(1);
        let mut tag = String::new();
        if self.frame() >= talk {
            tag += "backup slide";
        }
        if steps > 1 {
            tag += &format!("{}step {step} of {steps}", if tag.is_empty() { "" } else { ", " });
        }
        p.text(pos2(slide_r.right() - 10.0, vy), Align2::RIGHT_CENTER, tag, FontId::proportional(13.0 * s), if self.frame() >= talk { WARN } else { MUTED });
        let frac = ((self.frame() + 1) as f32 / talk as f32).min(1.0);
        // The progress line runs along the inside of the box's bottom edge, clear of the window edge.
        let track = Rect::from_min_size(pos2(slide_r.left() + 1.0, slide_r.bottom() - 4.0), vec2(slide_r.width() - 2.0, 3.0));
        p.rect_filled(track, 0.0, LINE);
        p.rect_filled(Rect::from_min_size(track.min, vec2(track.width() * frac, 3.0)), 0.0, LEAF);
        // Bookmarks along the progress line, so they can be seen coming.
        for f in self.side.bookmarks.iter().copied().filter(|f| *f < talk) {
            let [red, green, blue] = self.section_color(f);
            let x = track.left() + track.width() * (f as f32 + 0.5) / talk as f32;
            p.rect_filled(Rect::from_center_size(pos2(x, track.center().y - 1.0), vec2(4.0, 7.0)), 1.0, Color32::from_rgb(red, green, blue));
        }

        // Time elapsed, with its three controls at the right.
        let mono = |sz: f32| FontId::monospace(sz * s);
        let target = self.side.target_minutes;
        let mut color = INK;
        if target > 0.0 {
            let left = target as f64 * 60.0 - self.elapsed;
            if left < 0.0 {
                color = BAD;
            } else if left < target as f64 * 6.0 {
                color = WARN;
            }
        }
        let btn = (32.0 * s).clamp(26.0, 40.0);
        let buttons = Rect::from_min_size(pos2(time_r.right() - btn * 3.0 - 10.0, vy - btn / 2.0), vec2(btn * 3.0 + 6.0, btn));
        let room = buttons.left() - time_r.left();
        // The time itself is a button: click it to pause, and again to carry on.
        let area = Rect::from_min_max(time_r.min, pos2(buttons.left() - 2.0, time_r.bottom())).shrink(1.0);
        let tip = if self.timer_on { "Click to pause the timer" } else { "Click to start the timer" };
        let time_hit = ui.interact(area, sid("time-elapsed"), Sense::click()).on_hover_text(tip).on_hover_cursor(CursorIcon::PointingHand);
        if time_hit.hovered() {
            p.rect_filled(below_label(area), 5.0, LEAF_SOFT);
        }
        if time_hit.clicked() {
            let c = ui.ctx().clone();
            self.run(crate::settings::Action::ToggleTimer, &c);
        }
        let t = fmt_time(self.elapsed as f32);
        let paused = if !self.timer_on && room > 330.0 * s { " (paused)" } else { "" };
        p.text(pos2(time_r.left() + room / 2.0, vy), Align2::CENTER_CENTER, format!("{t}{paused}"), mono(32.0), color);
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

        // Duration: click to change it. Below it, how much is left and how the pace looks.
        let dur_text = fmt_time(target * 60.0);
        let dur_hit = ui.interact(dur_r.shrink(1.0), sid("duration"), Sense::click()).on_hover_text("Click to set the talk duration").on_hover_cursor(CursorIcon::PointingHand);
        if dur_hit.hovered() {
            p.rect_filled(below_label(dur_r), 5.0, LEAF_SOFT);
        }
        self.duration_rect = dur_r;
        if dur_hit.clicked() {
            self.duration_open = !self.duration_open;
        }
        let has_target = target > 0.0;
        let two_lines = has_target && self.elapsed > 1.0;
        let dur_y = if two_lines { vy - 8.0 * s } else { vy };
        p.text(pos2(dur_r.center().x, dur_y), Align2::CENTER_CENTER, dur_text, mono(if two_lines { 20.0 } else { 25.0 }), if has_target { INK } else { MUTED });
        if has_target && self.elapsed > 1.0 {
            let left = target * 60.0 - self.elapsed as f32;
            let done = self.frame() as f32 / talk as f32;
            let should = (self.elapsed as f32 / (target * 60.0)).min(1.0);
            let diff = (should - done) * target * 60.0;
            let (msg, c) = if left < 0.0 {
                (format!("{} over", fmt_time(-left)), BAD)
            } else if diff.abs() > 20.0 {
                (format!("{} {}", fmt_time(diff.abs()), if diff > 0.0 { "behind" } else { "ahead" }), if diff > 0.0 { WARN } else { LEAF })
            } else {
                (format!("{} left", fmt_time(left)), MUTED)
            };
            p.text(pos2(dur_r.center().x, dur_r.bottom() - 9.0 * s), Align2::CENTER_CENTER, msg, FontId::proportional(11.5), c);
        }
        p.text(pos2(clock_r.center().x, vy), Align2::CENTER_CENTER, wall_clock(), mono(25.0), MUTED);

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
            let box_r = Rect::from_min_size(pos2(r.left() + 18.0, r.top() - g.size().y - 20.0), g.size() + vec2(18.0, 10.0));
            p.rect_filled(box_r, 6.0, Color32::from_rgba_unmultiplied(40, 40, 40, 225));
            p.galley(box_r.min + vec2(9.0, 5.0), g, color);
        }
    }

    pub fn duration_popup(&mut self, ctx: &Context) {
        if !self.duration_open {
            return;
        }
        let w = 262.0;
        let at = pos2((self.duration_rect.right() - w).max(8.0), self.duration_rect.top() - 200.0);
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
        let bottom = (84.0 * s).max(68.0);
        let body = Rect::from_min_max(full.min + vec2(10.0, 12.0), full.max - vec2(8.0, bottom));
        let aspect = self.deck.aspect.max(0.1);
        let col = self.st.layout.split_col.unwrap_or_else(|| auto_col(body.width(), body.height() - 22.0, aspect, 8.0));
        let x = body.left() + body.width() * col;
        let left = Rect::from_min_max(body.min, pos2(x - 7.0, body.bottom()));
        let right = Rect::from_min_max(pos2(x + 7.0, body.top()), body.max);
        let label = 14.0 * s.min(1.2);
        labelled_frame(ui.painter(), left, "Audience view", label, INK);
        let slide = fit(left.shrink2(vec2(1.0, 11.0)), aspect, false);
        let cur = self.cur;
        self.paint_stage(ui, slide);
        self.slide_input(ui, slide, cur, "split-slide");

        let next_h = (right.height() * 0.32).min(right.width() / aspect);
        let row = self.st.layout.split_row.unwrap_or(1.0 - (next_h + 18.0) / right.height().max(1.0));
        let cut = right.top() + right.height() * row;
        let notes = Rect::from_min_max(right.min, pos2(right.right(), cut));
        let next = Rect::from_min_max(pos2(right.left(), cut + 18.0), right.max);
        labelled_frame(ui.painter(), notes, "Notes", label, INK);
        self.notes_panel(ui, notes.shrink2(vec2(0.0, 10.0)), (notes.width() / 1000.0).clamp(0.5, 1.1));
        labelled_frame(ui.painter(), next, "Next slide", label, INK);
        self.paint_next(ui, fit(next.shrink2(vec2(0.0, 10.0)), aspect, false));

        let handle = Rect::from_center_size(pos2(x, body.center().y), vec2(12.0, body.height()));
        let a = drag_handle(ui, "split-col", handle, true, true);
        let b = drag_handle(ui, "split-row", Rect::from_center_size(pos2(right.center().x, cut + 9.0), vec2(right.width(), 12.0)), false, false);
        let moved = a.apply(&mut self.st.layout.split_col, col, a.delta.x / body.width()) | b.apply(&mut self.st.layout.split_row, row, b.delta.y / right.height().max(1.0));
        if moved {
            self.save_layout();
        }
        self.bottom_bar(ui, Rect::from_min_max(pos2(full.left(), full.bottom() - bottom), full.max), s);
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
