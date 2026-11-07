//! LaTeX studio: source editor, live slide preview, build log, and jumps between source and slide.
use crate::app::*;
use crate::icons::{self, labelled_button, sized_button, Icon};
use crate::settings::Action;
use crate::ui_present::{engine_combo, fit, sid, to_screen, to_slide};
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::*;

const COMMAND: Color32 = Color32::from_rgb(30, 90, 170);
const COMMENT: Color32 = Color32::from_rgb(120, 130, 120);
const MATH: Color32 = Color32::from_rgb(130, 50, 150);
const BRACE: Color32 = Color32::from_rgb(170, 90, 20);
const TEXT: Color32 = Color32::from_rgb(40, 40, 40);

/// Colours LaTeX source by simple rules; it never changes the text.
pub fn highlight(src: &str, size: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font = FontId::monospace(size);
    let fmt = |c: Color32| TextFormat { font_id: font.clone(), color: c, ..Default::default() };
    let mut in_math = false;
    for line in src.split_inclusive('\n') {
        let b = line.as_bytes();
        let mut i = 0;
        let mut start = 0;
        macro_rules! flush {
            ($to:expr) => {
                if $to > start {
                    job.append(&line[start..$to], 0.0, fmt(if in_math { MATH } else { TEXT }));
                }
            };
        }
        while i < b.len() {
            match b[i] {
                b'%' => {
                    flush!(i);
                    job.append(&line[i..], 0.0, fmt(COMMENT));
                    start = line.len();
                    i = line.len();
                }
                b'\\' => {
                    flush!(i);
                    let mut e = i + 1;
                    while e < b.len() && b[e].is_ascii_alphabetic() {
                        e += 1;
                    }
                    if e == i + 1 && e < b.len() {
                        e += 1;
                        while !line.is_char_boundary(e) {
                            e += 1;
                        }
                    }
                    job.append(&line[i..e], 0.0, fmt(COMMAND));
                    start = e;
                    i = e;
                }
                b'{' | b'}' | b'[' | b']' => {
                    flush!(i);
                    job.append(&line[i..i + 1], 0.0, fmt(BRACE));
                    start = i + 1;
                    i += 1;
                }
                b'$' => {
                    flush!(i);
                    job.append("$", 0.0, fmt(MATH));
                    in_math = !in_math;
                    start = i + 1;
                    i += 1;
                }
                _ => i += 1,
            }
        }
        if start < line.len() {
            job.append(&line[start..], 0.0, fmt(if in_math { MATH } else { TEXT }));
        }
    }
    job
}

fn char_of_line(text: &str, line: usize) -> usize {
    let mut chars = 0;
    for (i, l) in text.split_inclusive('\n').enumerate() {
        if i + 1 == line {
            break;
        }
        chars += l.chars().count();
    }
    chars
}

fn line_of_char(text: &str, ch: usize) -> usize {
    text.chars().take(ch).filter(|c| *c == '\n').count() + 1
}

impl App {
    pub fn studio_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let frame = Frame::new().fill(SURFACE).inner_margin(Margin::symmetric(8, 4));
        Panel::top("studio-top").exact_size(44.0).frame(frame).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if labelled_button(ui, Icon::Refresh, "Compile", true).clicked() {
                    self.run(Action::Compile, &ctx);
                }
                if labelled_button(ui, Icon::Save, "Save", false).clicked() {
                    self.run(Action::Save, &ctx);
                }
                engine_combo(ui, &mut self.st.engine);
                if labelled_button(ui, Icon::Check, "Check packages", false).clicked() {
                    self.run(Action::CheckPackages, &ctx);
                }
                ui.checkbox(&mut self.st.live_edit, "Live").on_hover_text("Save and rebuild shortly after you stop typing");
                ui.checkbox(&mut self.st.auto_compile, "Rebuild on save");
                ui.checkbox(&mut self.follow_cursor, "Follow cursor").on_hover_text("Show the slide for the line you are on");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if labelled_button(ui, Icon::Presenter, "Present", false).clicked() {
                        self.view = View::Presenter;
                    }
                });
            });
        });
        // Every compile message lives in this one panel. Drag its top edge to resize it.
        let max = (ui.available_height() - 160.0).max(120.0);
        let log = Panel::bottom("studio-log").resizable(self.log_open).frame(Frame::new().fill(SURFACE).inner_margin(Margin::symmetric(10, 4)));
        let log = if self.log_open { log.default_size(170.0).size_range(90.0..=max) } else { log.exact_size(38.0) };
        log.show(ui, |ui| self.log_panel(ui));
        Panel::right("studio-preview").resizable(true).default_size(ui.available_width() * 0.5).size_range(240.0..=(ui.available_width() - 240.0).max(260.0)).show(ui, |ui| self.preview_panel(ui));
        CentralPanel::default().frame(Frame::new().fill(BG).inner_margin(Margin::symmetric(8, 4))).show(ui, |ui| self.editor_panel(ui));
    }

    fn log_panel(&mut self, ui: &mut Ui) {
        let errs = self.problems.iter().filter(|p| p.is_error).count();
        let warns = self.problems.len() - errs;
        let missing = self.reqs.iter().filter(|r| !r.found).count();
        let (icon, color, text) = if self.compiling {
            (Icon::Refresh, WARN, "Compiling…".to_string())
        } else if self.compile_ok == Some(false) {
            (Icon::Warn, BAD, self.compile_note.clone())
        } else if self.compile_ok == Some(true) {
            (Icon::Check, LEAF, self.compile_note.clone())
        } else {
            (Icon::Code, MUTED, "Ready to compile".to_string())
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
            icons::paint(ui.painter(), r, icon, color);
            ui.add(Label::new(RichText::new(text).color(color).size(13.5)).truncate());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let chevron = if self.log_open { "Hide" } else { "Show" };
                if ui.small_button(chevron).clicked() {
                    self.log_open = !self.log_open;
                }
                if sized_button(ui, Icon::Copy, false, "Copy this text", 28.0, 15.0).clicked() {
                    let t = self.log_text();
                    ui.ctx().copy_text(t);
                }
                if sized_button(ui, Icon::Wrap, self.log_wrap, "Wrap long lines", 28.0, 16.0).clicked() {
                    self.log_wrap = !self.log_wrap;
                }
                ui.add_space(8.0);
                for (tab, label) in [(LogTab::Log, "Full log".to_string()), (LogTab::Packages, format!("Packages ({missing} missing)")), (LogTab::Problems, format!("Problems ({})", errs + warns))] {
                    if ui.add(Button::selectable(self.log_tab == tab, RichText::new(label).size(13.0))).clicked() {
                        self.log_tab = tab;
                        self.log_open = true;
                    }
                }
            });
        });
        if !self.log_open {
            return;
        }
        ui.separator();
        let wrap = self.log_wrap;
        let area = if wrap { ScrollArea::vertical() } else { ScrollArea::both() };
        area.id_salt(("log", self.log_tab == LogTab::Log)).auto_shrink([false, false]).show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(if wrap { TextWrapMode::Wrap } else { TextWrapMode::Extend });
            match self.log_tab {
                LogTab::Problems => self.problems_list(ui),
                LogTab::Packages => self.packages_list(ui),
                LogTab::Log => {
                    let t = if self.full_log.is_empty() { "Compile to see the full log here." } else { self.full_log.as_str() };
                    ui.add(Label::new(RichText::new(t).monospace().size(12.5)).selectable(true));
                }
            }
        });
    }

    fn log_text(&self) -> String {
        match self.log_tab {
            LogTab::Log => self.full_log.clone(),
            LogTab::Problems => self.problems.iter().map(|p| format!("{}{}", p.line.map(|l| format!("line {l}: ")).unwrap_or_default(), p.message)).collect::<Vec<_>>().join("\n"),
            LogTab::Packages => self.reqs.iter().filter(|r| !r.found).map(|r| format!("{} {}: {}", r.kind.label(), r.name, r.hint)).collect::<Vec<_>>().join("\n"),
        }
    }

    fn problems_list(&mut self, ui: &mut Ui) {
        if self.problems.is_empty() {
            ui.label(RichText::new("No problems.").color(MUTED));
        }
        for p in self.problems.clone() {
            let c = if p.is_error { BAD } else { WARN };
            let where_ = p.line.map(|l| format!("line {l}: ")).unwrap_or_default();
            let lbl = Label::new(RichText::new(format!("{where_}{}", p.message)).color(c).size(13.5)).sense(Sense::click());
            let r = ui.add(lbl).on_hover_text("Click to go to this line");
            if r.clicked() {
                if let (Some(l), Some(s)) = (p.line, &mut self.src) {
                    s.goto_line = Some(l);
                }
            }
        }
    }

    fn packages_list(&mut self, ui: &mut Ui) {
        if self.src.is_none() {
            ui.label(RichText::new("Open a .tex file to check what it needs.").color(MUTED));
            return;
        }
        if self.req_rx.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Checking packages…");
            });
        }
        if let Some(w) = &self.req_warning {
            ui.label(RichText::new(w).color(BAD).size(13.5));
        }
        let missing: Vec<_> = self.reqs.iter().filter(|r| !r.found).cloned().collect();
        if missing.is_empty() && self.req_rx.is_none() && !self.reqs.is_empty() {
            ui.label(RichText::new(format!("All {} requirements found.", self.reqs.len())).color(LEAF_DARK).size(13.5));
        }
        for r in missing {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("Missing {}: {}", r.kind.label().to_lowercase(), r.name)).color(BAD).size(13.5));
                ui.label(RichText::new(r.hint).monospace().size(12.5).color(MUTED));
            });
        }
        CollapsingHeader::new(format!("Everything checked ({})", self.reqs.len())).show(ui, |ui| {
            for r in &self.reqs {
                let c = if r.found { MUTED } else { BAD };
                ui.label(RichText::new(format!("{} {}: {}", if r.found { "Found" } else { "Missing" }, r.kind.label().to_lowercase(), r.name)).color(c).size(12.5));
            }
        });
    }

    fn editor_panel(&mut self, ui: &mut Ui) {
        let Some(mut src) = self.src.take() else {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(RichText::new("No LaTeX source is open").size(18.0));
                ui.label("Open a .tex file to edit it here and watch the slides update.");
                if ui.button("Open LaTeX file…").clicked() {
                    let c = ui.ctx().clone();
                    self.run(Action::Open, &c);
                }
            });
            return;
        };
        ui.label(RichText::new(src.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).size(12.0).color(MUTED));
        let size = self.st.editor_size;
        let row_h = ui.fonts_mut(|f| f.row_height(&FontId::monospace(size)));
        let lines = src.text.lines().count().max(1);
        let gutter = size * 3.4;
        let id = sid("tex-editor");
        let goto = src.goto_line.take();
        if let Some(l) = goto {
            let mut st = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
            let at = CCursor::new(char_of_line(&src.text, l));
            st.cursor.set_char_range(Some(CCursorRange::one(at)));
            TextEdit::store_state(ui.ctx(), id, st);
        }
        let errors: Vec<usize> = self.problems.iter().filter(|p| p.is_error).filter_map(|p| p.line).collect();
        ScrollArea::both().id_salt("editor-scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let (g, _) = ui.allocate_exact_size(vec2(gutter, row_h * lines as f32 + 8.0), Sense::hover());
                let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, _w: f32| {
                    let mut job = highlight(buf.as_str(), size);
                    job.wrap.max_width = f32::INFINITY;
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                let out = TextEdit::multiline(&mut src.text)
                    .id(id)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(lines.max(30))
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .layouter(&mut layouter)
                    .show(ui);
                // Numbers follow the rows actually drawn, so they stay level with the text.
                let clip = ui.clip_rect();
                for (n, row) in out.galley.rows.iter().enumerate() {
                    let y = out.galley_pos.y + row.pos.y;
                    if y + row_h < clip.top() || y > clip.bottom() {
                        continue;
                    }
                    let c = if errors.contains(&(n + 1)) {
                        BAD
                    } else if n + 1 == src.cursor_line {
                        INK
                    } else {
                        Color32::from_gray(165)
                    };
                    ui.painter().text(pos2(g.right() - 10.0, y), Align2::RIGHT_TOP, (n + 1).to_string(), FontId::monospace(size - 1.0), c);
                }
                if out.response.changed() {
                    src.dirty = true;
                    src.last_edit = std::time::Instant::now();
                }
                if let Some(r) = out.cursor_range {
                    let line = line_of_char(&src.text, r.primary.index.0);
                    if line != src.cursor_line {
                        src.cursor_line = line;
                        src.line_changed = std::time::Instant::now();
                    }
                }
                if let Some(l) = goto {
                    out.response.request_focus();
                    let y = g.top() + (l.saturating_sub(1)) as f32 * row_h;
                    ui.scroll_to_rect(Rect::from_min_size(pos2(g.left(), y), vec2(10.0, row_h)), Some(Align::Center));
                }
            });
        });
        let wait = src.line_changed.elapsed().as_secs_f32();
        // Not while a rebuild is running: LaTeX is rewriting the map between source and slides then, and
        // a look-up in the half-written map lands on the first page.
        let want_sync = self.follow_cursor && src.cursor_line != src.synced_line && wait > 0.25 && !src.dirty && !self.compiling;
        self.src = Some(src);
        if want_sync {
            self.sync_from_cursor();
        }
    }

    fn preview_panel(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let c = ui.ctx().clone();
            if ui.button("<").clicked() {
                self.run(Action::Prev, &c);
            }
            if ui.button(">").clicked() {
                self.run(Action::Next, &c);
            }
            let n = self.deck.slides.len();
            if n == 0 {
                ui.label(RichText::new("No slides yet. Compile to see them here.").color(MUTED).size(13.0));
            } else {
                ui.label(RichText::new(format!("Slide {} (page {} of {n})", self.frame() + 1, self.cur + 1)).color(MUTED).size(13.0));
                ui.label(RichText::new("Double-click for source. Wheel to zoom.").color(MUTED).size(12.5));
            }
        });
        // The slide on top and its notes below, in two boxes of the same size.
        let area = ui.available_rect_before_wrap().shrink(4.0);
        let caption = 22.0;
        let half = Rect::from_min_max(area.min, pos2(area.right(), area.top() + (area.height() - caption) / 2.0));
        let r = fit(half, self.deck.aspect.max(0.1), true);
        let idx = self.cur;
        let Some(sl) = self.deck.slides.get(idx).cloned() else { return };
        let notes = r.translate(vec2(0.0, r.height() + caption));
        ui.painter().text(pos2(notes.left(), notes.top() - caption / 2.0), Align2::LEFT_CENTER, "Notes", FontId::proportional(13.0), MUTED);
        ui.painter().rect_filled(notes, 0.0, Color32::from_rgb(228, 228, 228));
        ui.painter().rect_stroke(notes, 0.0, Stroke::new(1.0, LINE), StrokeKind::Inside);
        self.notes_panel(ui, notes, (notes.width() / 1000.0).clamp(0.5, 1.1));
        let full = match self.zoom {
            Some(z) => Rect::from_min_max(pos2(z[0], z[1]), pos2(z[2], z[3])),
            None => Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
        };
        self.paint_page(ui, r, sl.page, sl.region, full);
        ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, LINE), StrokeKind::Outside);
        if let Some((i, pos, at)) = self.sync_mark {
            let age = at.elapsed().as_secs_f32();
            if i == idx && age < 1.6 {
                let a = ((1.0 - age / 1.6) * 150.0) as u8;
                let c = to_screen(r, full, pos);
                let band = Rect::from_center_size(pos2(r.center().x, c.y), vec2(r.width(), r.height() * 0.05));
                ui.painter().rect_filled(band, 2.0, Color32::from_rgba_unmultiplied(240, 200, 40, a));
                ui.ctx().request_repaint();
            }
        }
        let resp = ui.interact(r, sid("studio-slide"), Sense::click_and_drag());
        if let Some(p) = resp.hover_pos() {
            self.zoom_input(ui, to_slide(r, full, p));
        }
        if self.zoom.is_some() && resp.dragged() {
            let d = resp.drag_delta();
            self.zoom_pan([-d.x / r.width() * full.width(), -d.y / r.height() * full.height()]);
        }
        if resp.double_clicked() || (resp.clicked() && ui.input(|i| i.modifiers.command)) {
            if let Some(p) = resp.interact_pointer_pos() {
                self.sync_from_slide(idx, to_slide(r, full, p));
            }
        }
    }
}

/// What the comment shortcuts do to the lines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommentMode {
    /// Comment them out, or back in when every one of them already is.
    Toggle,
    Comment,
    Uncomment,
}

/// Comments out (with `% `) or back in the lines that the selection `sel` (character positions, either
/// order) touches; with no selection, the cursor's line. Returns the new text and the new selection,
/// which covers the same lines. A selection ending at the very start of a line leaves that line out, and
/// blank lines are left as they are. Comments go at the lines' shared indentation, so the code keeps
/// its shape.
pub fn comment_lines(text: &str, sel: (usize, usize), mode: CommentMode) -> (String, (usize, usize)) {
    let chars: Vec<char> = text.chars().collect();
    let (a, b) = (sel.0.min(sel.1).min(chars.len()), sel.0.max(sel.1).min(chars.len()));
    let line_start = |i: usize| chars[..i].iter().rposition(|c| *c == '\n').map(|p| p + 1).unwrap_or(0);
    let first = line_start(a);
    // A selection that ends at the start of a line does not take that line.
    let b = if b > a && b > 0 && chars[b - 1] == '\n' { b - 1 } else { b };
    let last_end = chars[b..].iter().position(|c| *c == '\n').map(|p| b + p).unwrap_or(chars.len());
    let block: String = chars[first..last_end].iter().collect();
    let lines: Vec<&str> = block.split('\n').collect();
    let filled: Vec<&&str> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    if filled.is_empty() {
        return (text.to_string(), sel);
    }
    let commented = |l: &str| l.trim_start().starts_with('%');
    let uncomment = match mode {
        CommentMode::Toggle => filled.iter().all(|l| commented(l)),
        CommentMode::Comment => false,
        CommentMode::Uncomment => true,
    };
    let indent = filled.iter().map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').count()).min().unwrap_or(0);
    let new_lines: Vec<String> = lines
        .iter()
        .map(|l| {
            if l.trim().is_empty() {
                return l.to_string();
            }
            if uncomment {
                let lead = l.chars().take_while(|c| *c == ' ' || *c == '\t').count();
                let rest: String = l.chars().skip(lead).collect();
                match rest.strip_prefix('%') {
                    Some(r) => {
                        let r = r.strip_prefix(' ').unwrap_or(r);
                        format!("{}{}", l.chars().take(lead).collect::<String>(), r)
                    }
                    None => l.to_string(),
                }
            } else {
                let (head, tail): (String, String) = (l.chars().take(indent).collect(), l.chars().skip(indent).collect());
                format!("{head}% {tail}")
            }
        })
        .collect();
    let new_block = new_lines.join("\n");
    let mut out: String = chars[..first].iter().collect();
    out.push_str(&new_block);
    out.extend(chars[last_end..].iter());
    let block_len = new_block.chars().count();
    // With a selection, the changed lines stay selected; otherwise the cursor keeps its place.
    let new_sel = if sel.0 != sel.1 {
        (first, first + block_len)
    } else {
        let shift = block_len as isize - (last_end - first) as isize;
        let c = (sel.0 as isize + shift).clamp(first as isize, (first + block_len) as isize) as usize;
        (c, c)
    };
    (out, new_sel)
}

#[cfg(test)]
mod tests {

    #[test]
    fn one_line_comments_and_uncomments() {
        let t = "\\begin{frame}\n  \\item One\n\\end{frame}";
        let cursor = t.find("One").unwrap();
        let (c, s) = comment_lines(t, (cursor, cursor), CommentMode::Toggle);
        assert_eq!(c, "\\begin{frame}\n  % \\item One\n\\end{frame}");
        assert_eq!(s.0, s.1);
        let (u, _) = comment_lines(&c, s, CommentMode::Toggle);
        assert_eq!(u, t);
    }

    #[test]
    fn selected_lines_keep_their_shape() {
        let t = "a\n  b\n    c\nd";
        let (start, end) = (t.find('b').unwrap(), t.find('c').unwrap());
        let (c, s) = comment_lines(t, (start, end), CommentMode::Toggle);
        assert_eq!(c, "a\n  % b\n  %   c\nd");
        assert_eq!(&c.chars().collect::<Vec<_>>()[s.0..s.1].iter().collect::<String>(), "  % b\n  %   c");
        // Pressed again on the same lines: all commented, so they come back.
        assert_eq!(comment_lines(&c, s, CommentMode::Toggle).0, t);
    }

    #[test]
    fn mixed_lines_are_commented_and_blank_lines_left_alone() {
        let t = "% done\n\nnot yet";
        let (c, _) = comment_lines(t, (0, t.chars().count()), CommentMode::Toggle);
        assert_eq!(c, "% % done\n\n% not yet");
        let (u, _) = comment_lines(t, (0, t.chars().count()), CommentMode::Uncomment);
        assert_eq!(u, "done\n\nnot yet");
        // Comment never takes comments away, and uncomment leaves plain lines as they are.
        assert_eq!(comment_lines("x", (0, 0), CommentMode::Comment).0, "% x");
        assert_eq!(comment_lines("x", (0, 0), CommentMode::Uncomment).0, "x");
    }

    #[test]
    fn a_selection_ending_at_a_line_start_leaves_that_line() {
        let t = "one\ntwo\nthree";
        let (c, _) = comment_lines(t, (0, t.find("three").unwrap()), CommentMode::Toggle);
        assert_eq!(c, "% one\n% two\nthree");
    }

    use super::*;

    #[test]
    fn highlighting_keeps_every_character() {
        let src = "% c\n\\begin{frame}{é}\nmath $x^2$ and 100\\% done é\n\\\\ end\n";
        let j = highlight(src, 12.0);
        assert_eq!(j.text, src);
    }

    #[test]
    fn line_and_char_positions_agree() {
        let t = "aé\nbb\ncc";
        assert_eq!(char_of_line(t, 2), 3);
        assert_eq!(line_of_char(t, 3), 2);
        assert_eq!(line_of_char(t, char_of_line(t, 3)), 3);
    }
}
