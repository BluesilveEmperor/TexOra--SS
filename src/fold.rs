//! Code folding for the LaTeX studio: which lines fold together, and how folded lines are hidden.
//!
//! A block that folds is an environment (`\begin{x}` … `\end{x}`), a sectioning command up to the next
//! heading of the same or a higher level, the preamble, or a `{ … }` group spanning several lines (a
//! long `\note{…}`). Folded lines stay in the text, so editing, saving and undo work on all of it;
//! they are only drawn in a tiny, invisible font, and a fold opens again when the cursor enters it.
use egui::text::{LayoutJob, LayoutSection};
use egui::epaint::text::ByteIndex;
use egui::Color32;
use std::collections::BTreeSet;
use std::ops::Range;

/// The font size of hidden text: rows this small take almost no room.
const HIDDEN_SIZE: f32 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Environment,
    Section,
    Preamble,
    Group,
}

/// A block of lines that can fold. Lines count from 1; the start line stays visible when folded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
    /// The environment or command name ("frame", "section"), for the folded line's label.
    pub name: String,
}

/// Environments whose contents are not LaTeX: nothing inside them opens or closes a block.
const VERBATIM: [&str; 6] = ["verbatim", "verbatim*", "lstlisting", "minted", "comment", "Verbatim"];

/// Heading commands and their level; a lower number is a higher level.
const HEADINGS: [(&str, u8); 7] = [("part", 0), ("chapter", 1), ("section", 2), ("subsection", 3), ("subsubsection", 4), ("paragraph", 5), ("subparagraph", 6)];

/// A line without its comment: everything from the first `%` that is not escaped as `\%`.
fn code(line: &str) -> &str {
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'%' => return &line[..i],
            _ => i += 1,
        }
    }
    line
}

/// The name in `{name}` right at the start of `s`.
fn braced(s: &str) -> Option<&str> {
    let s = s.strip_prefix('{')?;
    let end = s.find('}')?;
    Some(s[..end].trim())
}

/// The heading level of a line starting with `\section`, `\subsection*[…]{…}` and so on.
fn heading(code: &str) -> Option<(u8, &'static str)> {
    let t = code.trim_start().strip_prefix('\\')?;
    for (name, level) in HEADINGS {
        if let Some(rest) = t.strip_prefix(name) {
            let rest = rest.strip_prefix('*').unwrap_or(rest).trim_start();
            if rest.starts_with('{') || rest.starts_with('[') {
                return Some((level, name));
            }
        }
    }
    None
}

/// Every block that can fold, sorted by first line. At most one block starts on a line: the largest.
pub fn regions(text: &str) -> Vec<Region> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut found: Vec<Region> = Vec::new();
    let mut envs: Vec<(String, usize)> = Vec::new();
    let mut verbatim: Option<String> = None;
    let mut braces: Vec<usize> = Vec::new();
    let mut heads: Vec<(usize, u8, &'static str)> = Vec::new();
    let (mut class_line, mut doc_begin, mut doc_end, mut appendix) = (None, None, None, None);
    for (i, raw) in lines.iter().enumerate() {
        let n = i + 1;
        // Inside a verbatim environment, only its own \end counts.
        if let Some(v) = &verbatim {
            let close = format!("\\end{{{v}}}");
            if raw.contains(&close) {
                if let Some(pos) = envs.iter().rposition(|(e, _)| e == v) {
                    let (_, start) = envs.remove(pos);
                    envs.truncate(pos);
                    if n > start {
                        found.push(Region { start, end: n, kind: Kind::Environment, name: v.clone() });
                    }
                }
                verbatim = None;
            }
            continue;
        }
        let c = code(raw);
        if class_line.is_none() && c.trim_start().starts_with("\\documentclass") {
            class_line = Some(n);
        }
        if appendix.is_none() && c.trim() == "\\appendix" {
            appendix = Some(n);
        }
        if let Some((level, name)) = heading(c) {
            heads.push((n, level, name));
        }
        let b = c.as_bytes();
        let mut j = 0;
        while j < b.len() {
            match b[j] {
                b'\\' => {
                    let rest = &c[j + 1..];
                    if let Some(name) = rest.strip_prefix("begin").and_then(|r| braced(r.trim_start())) {
                        if name == "document" {
                            doc_begin = Some(n);
                        }
                        envs.push((name.to_string(), n));
                        if VERBATIM.contains(&name) {
                            verbatim = Some(name.to_string());
                            break;
                        }
                    } else if let Some(name) = rest.strip_prefix("end").and_then(|r| braced(r.trim_start())) {
                        if name == "document" {
                            doc_end = Some(n);
                        }
                        // An \end closes the nearest open environment of its name; anything left open
                        // inside it was never closed and is dropped.
                        if let Some(pos) = envs.iter().rposition(|(e, _)| e == name) {
                            let (e, start) = envs.remove(pos);
                            envs.truncate(pos);
                            if n > start {
                                found.push(Region { start, end: n, kind: Kind::Environment, name: e });
                            }
                        }
                    }
                    j += 2;
                }
                b'{' => {
                    braces.push(n);
                    j += 1;
                }
                b'}' => {
                    if let Some(start) = braces.pop() {
                        if n > start {
                            found.push(Region { start, end: n, kind: Kind::Group, name: String::new() });
                        }
                    }
                    j += 1;
                }
                _ => j += 1,
            }
        }
    }
    // A heading runs to the next heading of its level or higher, or to \appendix or \end{document}.
    let last = lines.len();
    let blank = |l: usize| lines.get(l - 1).is_none_or(|s| s.trim().is_empty());
    for (k, &(start, level, name)) in heads.iter().enumerate() {
        let next = heads[k + 1..].iter().find(|h| h.1 <= level).map(|h| h.0);
        let stops = [next, appendix.filter(|&a| a > start), doc_end.filter(|&d| d > start)];
        let mut end = stops.iter().flatten().min().map(|&s| s - 1).unwrap_or(last);
        while end > start && blank(end) {
            end -= 1;
        }
        if end > start {
            found.push(Region { start, end, kind: Kind::Section, name: name.to_string() });
        }
    }
    // The preamble: from \documentclass to the line before \begin{document}.
    if let (Some(start), Some(begin)) = (class_line, doc_begin) {
        let mut end = begin - 1;
        while end > start && blank(end) {
            end -= 1;
        }
        if end > start {
            found.push(Region { start, end, kind: Kind::Preamble, name: "preamble".into() });
        }
    }
    // One block per start line: the largest, and an environment or heading before a plain group.
    found.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)).then((a.kind == Kind::Group).cmp(&(b.kind == Kind::Group))));
    found.dedup_by(|later, first| later.start == first.start);
    found
}

/// The lines hidden by the folded blocks, as merged ranges of first and last line.
pub fn hidden_lines(regions: &[Region], folded: &BTreeSet<usize>) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for r in regions.iter().filter(|r| folded.contains(&r.start)) {
        let (a, b) = (r.start + 1, r.end);
        match out.last_mut() {
            Some(last) if a <= last.1 + 1 => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

pub fn is_hidden(hidden: &[(usize, usize)], line: usize) -> bool {
    hidden.iter().any(|&(a, b)| (a..=b).contains(&line))
}

/// The bytes of `text` on the hidden lines, each line with its newline.
pub fn byte_ranges(text: &str, hidden: &[(usize, usize)]) -> Vec<Range<usize>> {
    if hidden.is_empty() {
        return vec![];
    }
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    let at = |line: usize| starts.get(line - 1).copied().unwrap_or(text.len());
    hidden.iter().map(|&(a, b)| at(a)..at(b + 1)).filter(|r| !r.is_empty()).collect()
}

/// Draws the bytes in `ranges` in a tiny, invisible font, so their rows take almost no room.
pub fn hide(job: &mut LayoutJob, ranges: &[Range<usize>]) {
    if ranges.is_empty() {
        return;
    }
    let mut out: Vec<LayoutSection> = Vec::with_capacity(job.sections.len() + ranges.len() * 2);
    for s in job.sections.drain(..) {
        let mut at = s.byte_range.start.0;
        let end = s.byte_range.end.0;
        let from = at;
        for r in ranges.iter().filter(|r| r.start < end && r.end > from) {
            let (a, b) = (r.start.max(at), r.end.min(end));
            if a > at {
                out.push(LayoutSection { byte_range: ByteIndex(at)..ByteIndex(a), ..s.clone() });
            }
            let mut format = s.format.clone();
            format.font_id.size = HIDDEN_SIZE;
            format.color = Color32::TRANSPARENT;
            out.push(LayoutSection { leading_space: 0.0, byte_range: ByteIndex(a)..ByteIndex(b), format });
            at = b;
        }
        if at < end {
            out.push(LayoutSection { byte_range: ByteIndex(at)..ByteIndex(end), ..s });
        }
    }
    job.sections = out;
}

/// After an edit on line `at` added (`delta` > 0) or removed lines, folds further down move with
/// their text.
pub fn shift(folded: &mut BTreeSet<usize>, at: usize, delta: isize) {
    if delta == 0 {
        return;
    }
    *folded = folded
        .iter()
        .filter_map(|&l| {
            if l <= at {
                Some(l)
            } else {
                let moved = l as isize + delta;
                (moved > at as isize).then_some(moved as usize)
            }
        })
        .collect();
}

/// The innermost block holding `line`, for the block guide beside the editor.
pub fn innermost(regions: &[Region], line: usize) -> Option<&Region> {
    regions.iter().filter(|r| r.kind != Kind::Preamble && (r.start..=r.end).contains(&line)).min_by_key(|r| r.end - r.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECK: &str = "\\documentclass{beamer}\n\
\\usepackage{amsmath}\n\
\\tikzset{a,\n\
  b}\n\
\n\
\\begin{document}\n\
\\section{Intro}\n\
\\begin{frame}{One}\n\
  \\begin{itemize}\n\
    \\item a % \\end{itemize} in a comment does not count\n\
  \\end{itemize}\n\
  \\note{Line one\n\
  line two}\n\
\\end{frame}\n\
\n\
\\section{Two}\n\
\\subsection{Two A}\n\
\\begin{frame}\n\
  \\begin{verbatim}\n\
  \\begin{frame} is only text here {\n\
  \\end{verbatim}\n\
\\end{frame}\n\
\\end{document}\n";

    fn at(r: &[Region], start: usize) -> &Region {
        r.iter().find(|x| x.start == start).unwrap_or_else(|| panic!("no block at line {start}: {r:?}"))
    }

    #[test]
    fn environments_sections_preamble_and_groups_fold() {
        let r = regions(DECK);
        assert_eq!((at(&r, 1).end, at(&r, 1).kind), (4, Kind::Preamble), "preamble up to the last line before \\begin{{document}}");
        assert_eq!((at(&r, 3).end, at(&r, 3).kind), (4, Kind::Group), "a {{…}} over two lines");
        assert_eq!((at(&r, 6).end, at(&r, 6).name.as_str()), (23, "document"));
        assert_eq!((at(&r, 7).end, at(&r, 7).kind), (14, Kind::Section), "a section ends before the next one, without blank lines");
        assert_eq!((at(&r, 8).end, at(&r, 8).name.as_str()), (14, "frame"));
        assert_eq!(at(&r, 9).end, 11, "\\end in a comment is ignored");
        assert_eq!((at(&r, 12).end, at(&r, 12).kind), (13, Kind::Group), "a long \\note folds");
        assert_eq!(at(&r, 16).end, 22, "a section holds its subsections, up to \\end{{document}}");
        assert_eq!(at(&r, 17).end, 22);
        assert_eq!(at(&r, 18).end, 22, "verbatim text does not open a frame or a brace");
        assert_eq!(at(&r, 19).end, 21);
        assert!(!r.iter().any(|x| x.start == 20), "nothing starts inside verbatim");
        let starts: Vec<usize> = r.iter().map(|x| x.start).collect();
        assert!(starts.windows(2).all(|w| w[0] < w[1]), "one block per line, in order: {starts:?}");
    }

    #[test]
    fn folded_lines_are_hidden_and_merge() {
        let r = regions(DECK);
        let folded: BTreeSet<usize> = [8, 9].into();
        let h = hidden_lines(&r, &folded);
        assert_eq!(h, vec![(9, 14)], "the inner fold lies inside the outer one");
        assert!(is_hidden(&h, 14) && !is_hidden(&h, 8) && !is_hidden(&h, 15));
        let b = byte_ranges(DECK, &h);
        let text = &DECK[b[0].clone()];
        assert!(text.starts_with("\\begin{itemize}") && text.ends_with("\\end{frame}\n"), "{text:?}");
    }

    #[test]
    fn hiding_keeps_every_byte_in_order() {
        let mut job = crate::ui_studio::highlight(DECK, 13.0);
        let ranges = byte_ranges(DECK, &[(9, 13)]);
        hide(&mut job, &ranges);
        let mut at = 0;
        for s in &job.sections {
            assert_eq!(s.byte_range.start.0, at, "sections cover the text with no gap");
            at = s.byte_range.end.0;
            let inside = ranges.iter().any(|r| r.contains(&s.byte_range.start.0));
            assert_eq!(s.format.color == Color32::TRANSPARENT, inside);
        }
        assert_eq!(at, DECK.len());
    }

    #[test]
    fn folds_move_with_their_text() {
        let mut f: BTreeSet<usize> = [3, 10, 20].into();
        shift(&mut f, 5, 2);
        assert_eq!(f, [3, 12, 22].into());
        shift(&mut f, 5, -3);
        assert_eq!(f, [3, 9, 19].into());
        shift(&mut f, 8, -2);
        assert_eq!(f, [3, 17].into(), "a fold whose line was deleted goes");
    }

    #[test]
    fn the_guide_marks_the_innermost_block() {
        let r = regions(DECK);
        assert_eq!(innermost(&r, 10).map(|x| x.name.as_str()), Some("itemize"));
        assert_eq!(innermost(&r, 13).map(|x| x.start), Some(12));
        assert_eq!(innermost(&r, 2), None, "the preamble has no guide");
    }
}
