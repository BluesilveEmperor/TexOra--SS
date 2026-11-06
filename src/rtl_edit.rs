//! The notes editor used when the notes contain Arabic, Hebrew or another right-to-left script.
//!
//! The built-in text box places every line from left to right in the order it was typed, so a
//! sentence such as «ذهبت الى السوق» showed its words backwards. This editor lays each line out with
//! the Unicode bidirectional algorithm (see `bidi`): a paragraph that starts in a right-to-left script
//! runs from the right edge, its words in reading order, while English words and numbers inside it
//! still read left to right. It keeps the text in typing order, so files and copying stay correct.
//!
//! It supports typing, Enter, Backspace and Delete, the arrow keys (which move the way the line
//! reads), Home and End, clicking and dragging to select, Shift to extend a selection, select all,
//! copy, cut, paste and undo.
use crate::bidi;
use egui::*;
use std::ops::Range;

/// One line on screen. `start..end` are character positions in the whole text (typing order).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub start: usize,
    pub end: usize,
    pub rtl: bool,
    pub y: f32,
    pub pieces: Vec<Piece>,
}

/// A piece of a line drawn as one: its characters `start..end`, where it sits and how wide it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub start: usize,
    pub end: usize,
    pub rtl: bool,
    pub x: f32,
    pub w: f32,
    pub text: String,
}

/// Where the cursor is, and the other end of the selection.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub cursor: usize,
    pub anchor: usize,
    undo: Vec<(String, usize)>,
    redo: Vec<(String, usize)>,
    /// What the last change was, so typing a word is undone in one step.
    last: Option<Change>,
    dragging: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Change {
    Typing,
    Deleting,
}

impl State {
    pub fn at(i: usize) -> Self {
        State { cursor: i, anchor: i, ..Default::default() }
    }

    pub fn selection(&self) -> Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }
}

/// A paragraph with no letters at all (empty, or only numbers) follows the direction of the notes.
fn paragraph_rtl(para: &str, default: bool) -> bool {
    if para.chars().any(|c| matches!(unicode_bidi::bidi_class(c), unicode_bidi::BidiClass::L | unicode_bidi::BidiClass::R | unicode_bidi::BidiClass::AL)) {
        bidi::starts_rtl(para)
    } else {
        default
    }
}

/// Lays out the text in lines no wider than `width`, `row` apart. `measure` gives a text's width.
pub fn layout(text: &str, width: f32, row: f32, measure: &mut dyn FnMut(&str) -> f32) -> Vec<Line> {
    let chars: Vec<char> = text.chars().collect();
    let default_rtl = bidi::starts_rtl(text);
    let mut lines = vec![];
    let mut start = 0;
    loop {
        let end = chars[start..].iter().position(|c| *c == '\n').map(|p| start + p).unwrap_or(chars.len());
        let para: String = chars[start..end].iter().collect();
        let rtl = paragraph_rtl(&para, default_rtl);
        let styled: Vec<(char, usize)> = chars[start..end].iter().map(|c| (*c, 0)).collect();
        let wrapped = bidi::wrap(&styled, width, |s, _| measure(s));
        if wrapped.is_empty() {
            lines.push(Line { start, end: start, rtl, y: 0.0, pieces: vec![] });
        }
        let mut at = start;
        for w in wrapped {
            let mut pieces = vec![];
            for r in bidi::visual_runs(&w, rtl) {
                let text = bidi::run_text(w[r.range.clone()].iter().map(|x| x.0), r.rtl);
                let wd = measure(&text);
                pieces.push(Piece { start: at + r.range.start, end: at + r.range.end, rtl: r.rtl, x: 0.0, w: wd, text });
            }
            let total: f32 = pieces.iter().map(|p| p.w).sum();
            let mut x = if rtl { (width - total).max(0.0) } else { 0.0 };
            for p in &mut pieces {
                p.x = x;
                x += p.w;
            }
            lines.push(Line { start: at, end: at + w.len(), rtl, y: 0.0, pieces });
            at += w.len();
        }
        if end >= chars.len() {
            break;
        }
        start = end + 1;
    }
    for (n, l) in lines.iter_mut().enumerate() {
        l.y = n as f32 * row;
    }
    lines
}

/// The line the cursor at `i` is drawn on. Where a long paragraph wraps, the cursor between two lines
/// goes to the start of the next one.
pub fn line_of(lines: &[Line], i: usize) -> usize {
    for (n, l) in lines.iter().enumerate() {
        if i >= l.start && i <= l.end {
            let wraps_on = lines.get(n + 1).map(|next| next.start == i && i == l.end).unwrap_or(false);
            if !wraps_on {
                return n;
            }
        }
    }
    lines.len().saturating_sub(1)
}

/// Where the cursor at `i` is drawn, across the line, with `width` the box's width.
pub fn caret_x(text: &[char], line: &Line, i: usize, width: f32, measure: &mut dyn FnMut(&str) -> f32) -> f32 {
    if line.pieces.is_empty() {
        return if line.rtl { width } else { 0.0 };
    }
    let piece = line.pieces.iter().find(|p| p.start <= i && i < p.end).or_else(|| line.pieces.iter().find(|p| p.start < i && i <= p.end));
    let Some(p) = piece else {
        return if line.rtl { line.pieces.first().map(|p| p.x).unwrap_or(0.0) } else { line.pieces.last().map(|p| p.x + p.w).unwrap_or(0.0) };
    };
    let before: String = text[p.start..i].iter().collect();
    let w = measure(&before).min(p.w);
    if p.rtl {
        p.x + p.w - w
    } else {
        p.x + w
    }
}

/// The cursor position nearest to a point in the box.
pub fn index_at(text: &[char], lines: &[Line], pos: Vec2, row: f32, width: f32, measure: &mut dyn FnMut(&str) -> f32) -> usize {
    let n = ((pos.y / row).floor().max(0.0) as usize).min(lines.len().saturating_sub(1));
    let Some(l) = lines.get(n) else { return 0 };
    (l.start..=l.end)
        .map(|i| (i, (caret_x(text, l, i, width, measure) - pos.x).abs()))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|x| x.0)
        .unwrap_or(l.start)
}

fn byte(text: &str, i: usize) -> usize {
    text.char_indices().nth(i).map(|x| x.0).unwrap_or(text.len())
}

fn remember(text: &str, st: &mut State, change: Change) {
    if st.last != Some(change) {
        st.undo.push((text.to_string(), st.cursor));
        if st.undo.len() > 200 {
            st.undo.remove(0);
        }
        st.redo.clear();
    }
    st.last = Some(change);
}

fn delete_selection(text: &mut String, st: &mut State) -> bool {
    let r = st.selection();
    if r.is_empty() {
        return false;
    }
    text.replace_range(byte(text, r.start)..byte(text, r.end), "");
    st.cursor = r.start;
    st.anchor = r.start;
    true
}

pub fn insert(text: &mut String, st: &mut State, s: &str) {
    let s = s.replace("\r\n", "\n").replace('\r', "\n");
    if s.is_empty() {
        return;
    }
    remember(text, st, Change::Typing);
    if s.contains(char::is_whitespace) {
        // A new undo step starts after each word.
        st.last = None;
    }
    delete_selection(text, st);
    let at = byte(text, st.cursor);
    text.insert_str(at, &s);
    st.cursor += s.chars().count();
    st.anchor = st.cursor;
}

/// Backspace (`forward` false) or Delete (`forward` true).
pub fn delete(text: &mut String, st: &mut State, forward: bool) {
    remember(text, st, Change::Deleting);
    if delete_selection(text, st) {
        return;
    }
    let n = text.chars().count();
    let r = if forward {
        if st.cursor >= n {
            return;
        }
        st.cursor..st.cursor + 1
    } else {
        if st.cursor == 0 {
            return;
        }
        st.cursor - 1..st.cursor
    };
    text.replace_range(byte(text, r.start)..byte(text, r.end), "");
    st.cursor = r.start;
    st.anchor = r.start;
}

fn undo(text: &mut String, st: &mut State, back: bool) {
    let (from, to) = if back { (&mut st.undo, &mut st.redo) } else { (&mut st.redo, &mut st.undo) };
    if let Some((t, c)) = from.pop() {
        to.push((text.clone(), st.cursor));
        *text = t;
        st.cursor = c.min(text.chars().count());
        st.anchor = st.cursor;
    }
    st.last = None;
}

/// Moves the cursor for an arrow key, Home or End. Left and Right follow the way the line reads.
pub fn step(text: &[char], lines: &[Line], st: &mut State, key: Key, shift: bool, width: f32, row: f32, measure: &mut dyn FnMut(&str) -> f32) {
    let n = text.len();
    let sel = st.selection();
    let li = line_of(lines, st.cursor);
    let rtl = lines.get(li).map(|l| l.rtl).unwrap_or(false);
    let forward = match key {
        Key::ArrowRight => Some(!rtl),
        Key::ArrowLeft => Some(rtl),
        _ => None,
    };
    let to = match (key, forward) {
        (_, Some(f)) if !shift && !sel.is_empty() => {
            if f {
                sel.end
            } else {
                sel.start
            }
        }
        (_, Some(true)) => (st.cursor + 1).min(n),
        (_, Some(false)) => st.cursor.saturating_sub(1),
        (Key::Home, _) => lines.get(li).map(|l| l.start).unwrap_or(0),
        (Key::End, _) => lines.get(li).map(|l| l.end).unwrap_or(n),
        (Key::ArrowUp | Key::ArrowDown, _) => {
            let target = if key == Key::ArrowUp { li.checked_sub(1) } else { (li + 1 < lines.len()).then_some(li + 1) };
            match target {
                Some(t) => {
                    let x = caret_x(text, &lines[li], st.cursor, width, measure);
                    index_at(text, lines, vec2(x, lines[t].y + row * 0.5), row, width, measure)
                }
                None if key == Key::ArrowUp => 0,
                None => n,
            }
        }
        _ => return,
    };
    st.cursor = to;
    if !shift {
        st.anchor = to;
    }
    st.last = None;
}

/// The editor. Draws the text in `ui`'s available width and handles keys, mouse and clipboard while
/// it has keyboard focus. Returns true when the text changed.
pub fn show(ui: &mut Ui, id: Id, text: &mut String, font: FontId, color: Color32, min_height: f32) -> bool {
    let width = ui.available_width().max(40.0);
    let row = ui.fonts_mut(|f| f.row_height(&font)) * 1.15;
    let fonts_ctx = ui.ctx().clone();
    let mut measure = |s: &str| -> f32 {
        if s.is_empty() {
            return 0.0;
        }
        fonts_ctx.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font.clone(), color)).size().x
    };
    let mut st: State = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| State::at(text.chars().count()));
    let mut lines = layout(text, width, row, &mut measure);
    let height = (lines.len() as f32 * row).max(min_height);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let focused = ui.memory(|m| m.has_focus(id));
    if resp.clicked() || resp.drag_started() {
        ui.memory_mut(|m| m.request_focus(id));
    }
    ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { tab: false, horizontal_arrows: true, vertical_arrows: true, escape: false }));
    let mut changed = false;

    // The mouse: press to place the cursor, drag or Shift-click to select.
    if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
        let chars: Vec<char> = text.chars().collect();
        let local = p - rect.min;
        if resp.is_pointer_button_down_on() {
            let i = index_at(&chars, &lines, local, row, width, &mut measure);
            if !st.dragging {
                st.dragging = true;
                if !ui.input(|i| i.modifiers.shift) {
                    st.anchor = i;
                }
            }
            st.cursor = i;
            st.last = None;
        } else {
            st.dragging = false;
        }
    }
    if resp.double_clicked() {
        let chars: Vec<char> = text.chars().collect();
        let mut a = st.cursor.min(chars.len());
        let mut b = a;
        while a > 0 && !chars[a - 1].is_whitespace() {
            a -= 1;
        }
        while b < chars.len() && !chars[b].is_whitespace() {
            b += 1;
        }
        st.anchor = a;
        st.cursor = b;
    }

    // The keyboard and clipboard.
    if focused {
        let events = ui.input(|i| i.events.clone());
        for e in events {
            let chars: Vec<char> = text.chars().collect();
            match e {
                Event::Text(t) if !t.chars().any(char::is_control) => {
                    insert(text, &mut st, &t);
                    changed = true;
                }
                Event::Ime(ImeEvent::Commit(t)) => {
                    insert(text, &mut st, &t);
                    changed = true;
                }
                Event::Paste(t) => {
                    st.last = None;
                    insert(text, &mut st, &t);
                    st.last = None;
                    changed = true;
                }
                Event::Copy | Event::Cut => {
                    let r = st.selection();
                    if !r.is_empty() {
                        ui.ctx().copy_text(chars[r].iter().collect());
                        if matches!(e, Event::Cut) {
                            st.last = None;
                            delete(text, &mut st, false);
                            changed = true;
                        }
                    }
                }
                Event::Key { key, pressed: true, modifiers, .. } => match key {
                    Key::Enter if !modifiers.command => {
                        insert(text, &mut st, "\n");
                        st.last = None;
                        changed = true;
                    }
                    Key::Backspace | Key::Delete => {
                        delete(text, &mut st, key == Key::Delete);
                        changed = true;
                    }
                    Key::A if modifiers.command => {
                        st.anchor = 0;
                        st.cursor = chars.len();
                    }
                    Key::Z if modifiers.command => {
                        undo(text, &mut st, !modifiers.shift);
                        changed = true;
                    }
                    Key::Y if modifiers.command => {
                        undo(text, &mut st, false);
                        changed = true;
                    }
                    Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown | Key::Home | Key::End => {
                        step(&chars, &lines, &mut st, key, modifiers.shift, width, row, &mut measure);
                    }
                    _ => {}
                },
                _ => {}
            }
            if changed {
                lines = layout(text, width, row, &mut measure);
            }
        }
    }
    let n = text.chars().count();
    st.cursor = st.cursor.min(n);
    st.anchor = st.anchor.min(n);

    // Drawing: the selection, the text, then the cursor.
    let chars: Vec<char> = text.chars().collect();
    let painter = ui.painter_at(rect.expand(2.0));
    let sel = st.selection();
    if !sel.is_empty() {
        let fill = ui.visuals().selection.bg_fill;
        for l in &lines {
            for p in &l.pieces {
                let (a, b) = (sel.start.max(p.start), sel.end.min(p.end));
                if a < b {
                    let xa = caret_x(&chars, l, a, width, &mut measure);
                    let xb = if b == p.end {
                        if p.rtl {
                            p.x
                        } else {
                            p.x + p.w
                        }
                    } else {
                        caret_x(&chars, l, b, width, &mut measure)
                    };
                    let r = Rect::from_x_y_ranges(xa.min(xb)..=xa.max(xb), l.y..=l.y + row).translate(rect.min.to_vec2());
                    painter.rect_filled(r, 0.0, fill);
                }
            }
        }
    }
    for l in &lines {
        for p in &l.pieces {
            let g = ui.fonts_mut(|f| f.layout_no_wrap(p.text.clone(), font.clone(), color));
            let top = rect.min + vec2(p.x, l.y + (row - g.size().y) * 0.5);
            painter.galley(top, g, color);
        }
    }
    if focused || ui.memory(|m| m.has_focus(id)) {
        let li = line_of(&lines, st.cursor);
        if let Some(l) = lines.get(li) {
            let x = caret_x(&chars, l, st.cursor, width, &mut measure).clamp(1.0, width - 1.0);
            let caret = Rect::from_x_y_ranges(x - 1.0..=x + 1.0, l.y + row * 0.1..=l.y + row * 0.9).translate(rect.min.to_vec2());
            painter.rect_filled(caret, 0.0, ui.visuals().text_cursor.stroke.color);
            if changed || resp.is_pointer_button_down_on() || ui.input(|i| i.events.iter().any(|e| matches!(e, Event::Key { pressed: true, .. }))) {
                ui.scroll_to_rect(caret.expand(row), None);
            }
        }
    }
    ui.data_mut(|d| d.insert_temp(id, st));
    resp.on_hover_cursor(CursorIcon::Text);
    changed
}

/// Puts the cursor (and the other end of a selection) of the editor `id`, for example when the plain
/// text box hands over as the first Arabic or Hebrew letter is typed.
pub fn set_cursor(ctx: &Context, id: Id, cursor: usize, anchor: usize) {
    ctx.data_mut(|d| {
        let st = d.get_temp_mut_or_default::<State>(id);
        st.cursor = cursor;
        st.anchor = anchor;
    });
}

/// The cursor and the other end of the selection of the editor `id`, if it has been used.
pub fn cursor(ctx: &Context, id: Id) -> Option<(usize, usize)> {
    ctx.data(|d| d.get_temp::<State>(id)).map(|s| (s.cursor, s.anchor))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character is 10 wide, so positions are easy to check.
    fn m(s: &str) -> f32 {
        s.chars().count() as f32 * 10.0
    }

    fn lay(t: &str, width: f32) -> Vec<Line> {
        layout(t, width, 20.0, &mut m)
    }

    #[test]
    fn an_arabic_sentence_reads_from_the_right_in_order() {
        // «I went to the market»: the first word typed is drawn at the right edge.
        let t = "ذهبت الى السوق";
        let l = &lay(t, 300.0)[0];
        assert!(l.rtl);
        let order: Vec<&str> = l.pieces.iter().map(|p| p.text.as_str()).filter(|s| s.trim() != "").collect();
        assert_eq!(order, ["السوق", "الى", "ذهبت"]);
        // Right-aligned: the last piece ends at the box's right edge.
        let last = l.pieces.last().unwrap();
        assert_eq!(last.x + last.w, 300.0);
    }

    #[test]
    fn the_cursor_moves_leftwards_while_typing_arabic() {
        let t: Vec<char> = "ذهبت الى".chars().collect();
        let lines = lay(&t.iter().collect::<String>(), 300.0);
        let start = caret_x(&t, &lines[0], 0, 300.0, &mut m);
        let after_word = caret_x(&t, &lines[0], 4, 300.0, &mut m);
        let end = caret_x(&t, &lines[0], t.len(), 300.0, &mut m);
        assert_eq!(start, 300.0);
        assert!(after_word < start && end < after_word, "{start} {after_word} {end}");
    }

    #[test]
    fn english_inside_arabic_still_reads_left_to_right() {
        let t = "افتح ملف PDF الآن";
        let l = &lay(t, 400.0)[0];
        let order: Vec<&str> = l.pieces.iter().map(|p| p.text.as_str()).filter(|s| s.trim() != "").collect();
        assert_eq!(order, ["الآن", "PDF", "ملف", "افتح"]);
    }

    #[test]
    fn english_paragraphs_stay_left_aligned() {
        let lines = lay("Hello\nمرحبا\n", 300.0);
        assert_eq!(lines.len(), 3);
        assert!(!lines[0].rtl && lines[0].pieces[0].x == 0.0);
        assert!(lines[1].rtl);
        // The empty last line follows the notes' first letter: English here.
        assert!(!lines[2].rtl);
    }

    #[test]
    fn long_arabic_paragraphs_wrap_from_the_top_right() {
        let t = "واحد اثنان ثلاثة اربعة";
        let lines = lay(t, 120.0);
        assert!(lines.len() >= 2);
        // The first words typed are on the first line.
        let first: String = t.chars().skip(lines[0].start).take(lines[0].end - lines[0].start).collect();
        assert!(first.starts_with("واحد"), "{first}");
        assert_eq!(line_of(&lines, lines[1].start), 1);
    }

    #[test]
    fn typing_deleting_and_undo() {
        let mut t = String::new();
        let mut st = State::default();
        for w in ["ذهبت", " ", "الى", " ", "السوق"] {
            insert(&mut t, &mut st, w);
        }
        assert_eq!(t, "ذهبت الى السوق");
        delete(&mut t, &mut st, false);
        assert_eq!(t, "ذهبت الى السو");
        undo(&mut t, &mut st, true);
        assert_eq!(t, "ذهبت الى السوق");
        st.anchor = 0;
        st.cursor = 5;
        insert(&mut t, &mut st, "ذهبنا ");
        assert_eq!(t, "ذهبنا الى السوق");
    }

    #[test]
    fn arrow_keys_follow_the_reading_direction() {
        let t: Vec<char> = "ذهبت الى".chars().collect();
        let lines = lay(&t.iter().collect::<String>(), 300.0);
        let mut st = State::at(0);
        // In Arabic, Left moves on through the text.
        step(&t, &lines, &mut st, Key::ArrowLeft, false, 300.0, 20.0, &mut m);
        assert_eq!(st.cursor, 1);
        step(&t, &lines, &mut st, Key::ArrowRight, false, 300.0, 20.0, &mut m);
        assert_eq!(st.cursor, 0);
        step(&t, &lines, &mut st, Key::End, true, 300.0, 20.0, &mut m);
        assert_eq!(st.selection(), 0..t.len());
    }

    #[test]
    fn clicking_finds_the_nearest_letter() {
        let t: Vec<char> = "ذهبت الى السوق".chars().collect();
        let lines = lay(&t.iter().collect::<String>(), 300.0);
        // The right edge is the start of the text, the left end of the line its end.
        assert_eq!(index_at(&t, &lines, vec2(299.0, 5.0), 20.0, 300.0, &mut m), 0);
        assert_eq!(index_at(&t, &lines, vec2(300.0 - 140.0, 5.0), 20.0, 300.0, &mut m), t.len());
    }
}
