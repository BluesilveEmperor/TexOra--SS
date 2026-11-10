//! Text speaker notes: Markdown side files, `\note{}` in LaTeX source, and .pdfpc imports.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The notes file of a presentation, in the same folder, named after it: `Lecture.pdf` (or
/// `Lecture.tex`) keeps its notes in `Lecture.note.md`.
pub fn notes_file(deck: &Path) -> PathBuf {
    deck.with_extension("note.md")
}

/// The name used before (`Lecture.notes.md`). It is still read when there is no `.note.md` yet.
pub fn older_notes_file(deck: &Path) -> PathBuf {
    deck.with_extension("notes.md")
}

/// The notes saved for a presentation, from its notes file (or the file under the older name).
pub fn load_notes(deck: &Path) -> Notes {
    let text = read_text(&notes_file(deck)).or_else(|| read_text(&older_notes_file(deck))).unwrap_or_default();
    parse_md(&text)
}

/// Writes the notes into the presentation's notes file. A notes file under the older name becomes the
/// new one first, so a presentation's notes stay in one file and nothing is left behind.
pub fn save_notes(deck: &Path, notes: &Notes) -> std::io::Result<()> {
    let (new, old) = (notes_file(deck), older_notes_file(deck));
    if !new.exists() && old.exists() {
        let _ = std::fs::rename(&old, &new);
    }
    std::fs::write(&new, to_md(notes))
}

/// Notes by frame number (0-based).
pub type Notes = BTreeMap<usize, String>;

/// Notes file format: a heading `## Slide N` starts the notes for slide N.
pub fn parse_md(src: &str) -> Notes {
    let mut out = Notes::new();
    let mut cur: Option<usize> = None;
    let mut buf = String::new();
    for line in src.lines() {
        let head = line
            .trim()
            .strip_prefix("## Slide ")
            .and_then(|r| r.split_whitespace().next())
            .and_then(|n| n.parse::<usize>().ok());
        if let Some(n) = head.filter(|n| *n > 0) {
            if let Some(c) = cur {
                out.insert(c, buf.trim().to_string());
            }
            cur = Some(n - 1);
            buf.clear();
        } else if cur.is_some() {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    if let Some(c) = cur {
        out.insert(c, buf.trim().to_string());
    }
    out.retain(|_, v| !v.is_empty());
    out
}

pub fn to_md(notes: &Notes) -> String {
    let mut s = String::from("# Speaker notes (TeXora)\n\n");
    for (k, v) in notes.iter().filter(|(_, v)| !v.trim().is_empty()) {
        s.push_str(&format!("## Slide {}\n{}\n\n", k + 1, v.trim()));
    }
    s
}

/// Reads a text file in whatever encoding it was saved in: UTF-8 (with or without a byte-order mark),
/// UTF-16 (as Windows Notepad's "Unicode" saves), or an older one-byte Windows encoding. Notes that
/// could not be read would look empty and be written over at the next save, so nothing is refused.
pub fn read_text(path: &Path) -> Option<String> {
    std::fs::read(path).ok().map(|b| decode_text(&b))
}

pub fn decode_text(bytes: &[u8]) -> String {
    if let Some(b) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(b).into_owned();
    }
    let utf16 = |b: &[u8], little: bool| {
        let units: Vec<u16> = b.chunks_exact(2).map(|p| if little { u16::from_le_bytes([p[0], p[1]]) } else { u16::from_be_bytes([p[0], p[1]]) }).collect();
        String::from_utf16_lossy(&units)
    };
    if let Some(b) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(b, true);
    }
    if let Some(b) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return utf16(b, false);
    }
    // UTF-16 without a mark: most characters of ordinary text have a zero byte beside them. Checked
    // before UTF-8, because zero bytes are allowed there too, though no real text has them.
    let zeros = |start: usize| bytes.iter().skip(start).step_by(2).filter(|b| **b == 0).count();
    let half = bytes.len() / 2;
    if bytes.len() >= 4 && zeros(1) * 2 > half {
        return utf16(bytes, true);
    }
    if bytes.len() >= 4 && zeros(0) * 2 > half {
        return utf16(bytes, false);
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    // Windows-1252, which covers Latin-1 too.
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š',
        '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    bytes.iter().map(|&b| if (0x80..0xA0).contains(&b) { HIGH[(b - 0x80) as usize] } else { b as char }).collect()
}

pub fn load_pdfpc(path: &Path) -> Notes {
    let mut out = Notes::new();
    let Some(src) = read_text(path) else { return out };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&src) else { return out };
    for p in v["pages"].as_array().into_iter().flatten() {
        if let (Some(i), Some(n)) = (p["idx"].as_u64(), p["note"].as_str()) {
            out.entry(i as usize).or_insert_with(|| n.trim().to_string());
        }
    }
    out
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TexFrame {
    pub line: usize,
    pub end_line: usize,
    pub title: String,
    pub note: String,
}

fn strip_comment(l: &str) -> &str {
    let b = l.as_bytes();
    for i in 0..b.len() {
        if b[i] == b'%' && (i == 0 || b[i - 1] != b'\\') {
            return &l[..i];
        }
    }
    l
}

/// Reads a balanced `{...}` that starts at byte `at`; returns contents and the end position.
fn braced(s: &str, at: usize) -> Option<(&str, usize)> {
    let b = s.as_bytes();
    if b.get(at) != Some(&b'{') {
        return None;
    }
    let (mut depth, mut i) = (0i32, at);
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[at + 1..i], i + 1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Skips `<...>` and `[...]` groups and spaces, as found after \begin{frame} or \note.
fn skip_opts(s: &str, mut i: usize) -> usize {
    let b = s.as_bytes();
    loop {
        match b.get(i) {
            Some(b' ') | Some(b'\t') | Some(b'\n') => i += 1,
            Some(b'[') | Some(b'<') => {
                let close = if b[i] == b'[' { b']' } else { b'>' };
                match b[i..].iter().position(|c| *c == close) {
                    Some(p) => i += p + 1,
                    None => return i,
                }
            }
            _ => return i,
        }
    }
}

pub fn latex_to_md(s: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    'scan: while i < s.len() {
        let rest = &s[i..];
        for (cmd, mark) in [("\\textbf", "**"), ("\\emph", "*"), ("\\textit", "*"), ("\\texttt", "`")] {
            if rest.starts_with(cmd) {
                if let Some((inner, end)) = braced(s, i + cmd.len()) {
                    out.push_str(mark);
                    out.push_str(&latex_to_md(inner));
                    out.push_str(mark);
                    i = end;
                    continue 'scan;
                }
            }
        }
        if rest.starts_with("\\hyperlink") {
            if let Some((_, e)) = braced(s, i + 10) {
                if let Some((text, end)) = braced(s, e) {
                    out.push_str(&latex_to_md(text));
                    i = end;
                    continue;
                }
            }
        }
        if rest.starts_with("\\\\") {
            // A line break; the source's own newline after it is the same break.
            out.push('\n');
            i = skip_break(s, i + 2);
            continue;
        }
        for (cmd, text) in [("\\textbackslash{}", "\\"), ("\\textasciitilde{}", "~"), ("\\textasciicircum{}", "^")] {
            if rest.starts_with(cmd) {
                out.push_str(text);
                i += cmd.len();
                continue 'scan;
            }
        }
        if let Some(c) = rest.strip_prefix('\\').and_then(|r| r.chars().next()).filter(|c| "&%$#_{}".contains(*c)) {
            out.push(c);
            i += 2;
            continue;
        }
        if rest.starts_with("\\item") {
            trim_end_in_place(&mut out);
            out.push_str("\n- ");
            i = skip_spaces(s, i + 5);
            continue;
        }
        if rest.starts_with("\\begin{") || rest.starts_with("\\end{") {
            if let Some(p) = rest.find('}') {
                trim_end_in_place(&mut out);
                out.push('\n');
                i = skip_spaces(s, i + p + 1);
                continue;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out.lines().map(str::trim).collect::<Vec<_>>().join("\n").trim().to_string()
}

fn trim_end_in_place(s: &mut String) {
    let n = s.trim_end().len();
    s.truncate(n);
}

/// Past spaces, tabs and newlines.
fn skip_spaces(s: &str, mut i: usize) -> usize {
    while s.as_bytes().get(i).is_some_and(|b| b.is_ascii_whitespace()) {
        i += 1;
    }
    i
}

/// Past spaces and tabs, and at most one newline.
fn skip_break(s: &str, mut i: usize) -> usize {
    while matches!(s.as_bytes().get(i), Some(b' ') | Some(b'\t')) {
        i += 1;
    }
    if s.as_bytes().get(i) == Some(&b'\n') {
        i += 1;
    }
    i
}

/// LaTeX's special signs, written so they print as themselves.
fn escape_latex(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '~' => out.push_str("\\textasciitilde{}"),
            '^' => out.push_str("\\textasciicircum{}"),
            '&' | '%' | '$' | '#' | '_' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// One line of Markdown with **bold**, *emphasis* and `code`, as LaTeX.
fn inline_md_to_latex(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while !rest.is_empty() {
        let next = ["**", "`", "*"].iter().filter_map(|m| rest.find(m).map(|p| (p, *m))).min_by_key(|x| (x.0, std::cmp::Reverse(x.1.len())));
        match next {
            Some((p, m)) if rest[p + m.len()..].contains(m) => {
                out.push_str(&escape_latex(&rest[..p]));
                let after = &rest[p + m.len()..];
                let end = after.find(m).unwrap();
                let inner = &after[..end];
                match m {
                    "**" => out.push_str(&format!("\\textbf{{{}}}", inline_md_to_latex(inner))),
                    "`" => out.push_str(&format!("\\texttt{{{}}}", escape_latex(inner))),
                    _ => out.push_str(&format!("\\emph{{{}}}", inline_md_to_latex(inner))),
                }
                rest = &after[end + m.len()..];
            }
            _ => {
                out.push_str(&escape_latex(rest));
                rest = "";
            }
        }
    }
    out
}

/// Notes written in the presenter (Markdown) as the body of a LaTeX `\note{...}`: lines break with
/// `\\`, `- ` lines become a list, and `latex_to_md` reads it back as the same text.
pub fn md_to_latex(md: &str) -> String {
    #[derive(PartialEq)]
    enum K {
        Text,
        Blank,
        Other,
    }
    let mut parts: Vec<(K, String)> = vec![];
    let mut in_list = false;
    for line in md.trim().lines() {
        let t = line.trim();
        let item = t.strip_prefix("- ").or_else(|| t.strip_prefix("* "));
        if let Some(item) = item {
            if !in_list {
                parts.push((K::Other, "\\begin{itemize}".to_string()));
                in_list = true;
            }
            parts.push((K::Other, format!("\\item {}", inline_md_to_latex(item.trim()))));
            continue;
        }
        if in_list {
            parts.push((K::Other, "\\end{itemize}".to_string()));
            in_list = false;
        }
        if t.is_empty() {
            parts.push((K::Blank, String::new()));
        } else if t.starts_with('#') {
            parts.push((K::Text, format!("\\textbf{{{}}}", inline_md_to_latex(t.trim_start_matches('#').trim()))));
        } else {
            parts.push((K::Text, inline_md_to_latex(t)));
        }
    }
    if in_list {
        parts.push((K::Other, "\\end{itemize}".to_string()));
    }
    let mut out = String::new();
    for i in 0..parts.len() {
        out.push_str(&parts[i].1);
        // A line break between two lines of text; lists and blank lines break by themselves.
        if parts[i].0 == K::Text && parts.get(i + 1).is_some_and(|n| n.0 == K::Text) {
            out.push_str("\\\\");
        }
        if i + 1 < parts.len() {
            out.push('\n');
        }
    }
    out
}

/// Environments that make a Beamer frame: `frame`, and any environment defined (in the source, or in
/// a `.sty` or `.cls` file beside it, such as a custom theme or class) around `\begin{frame}`,
/// for example `\newenvironment<>{slide}[1]{\begin{frame}[environment=slide]{#1}}{\end{frame}}`.
pub fn frame_envs(src: &str, dir: Option<&Path>) -> Vec<String> {
    let mut defs = src.to_string();
    if let Some(entries) = dir.and_then(|d| std::fs::read_dir(d).ok()) {
        for e in entries.flatten().take(400) {
            let p = e.path();
            let local = p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sty") || x.eq_ignore_ascii_case("cls"));
            if local && e.metadata().is_ok_and(|m| m.len() < 1_000_000) {
                if let Ok(t) = std::fs::read_to_string(&p) {
                    defs.push('\n');
                    defs.push_str(&t);
                }
            }
        }
    }
    let clean: String = defs.lines().map(strip_comment).collect::<Vec<_>>().join("\n");
    let mut names = vec!["frame".to_string()];
    for kw in ["\\newenvironment", "\\renewenvironment", "\\NewDocumentEnvironment", "\\RenewDocumentEnvironment", "\\DeclareDocumentEnvironment", "\\ProvideDocumentEnvironment"] {
        let mut pos = 0;
        while let Some(p) = clean[pos..].find(kw) {
            let at = pos + p + kw.len();
            pos = at;
            let rest = clean[at..].trim_start_matches(['*', ' ']);
            let rest = rest.strip_prefix("<>").unwrap_or(rest).trim_start();
            let Some((name, _)) = rest.strip_prefix('{').and_then(|r| r.split_once('}')) else { continue };
            let name = name.trim();
            if name.is_empty() || name == "frame" || !name.chars().all(|c| c.is_alphanumeric() || "*-_:".contains(c)) {
                continue;
            }
            // The definition: up to the next definition, and not much further.
            let tail = &clean[at..];
            let mut cut = tail.char_indices().nth(800).map(|(k, _)| k).unwrap_or(tail.len());
            if let Some(next) = tail[1..cut].find("environment") {
                cut = cut.min(next + 1);
            }
            if tail[..cut].contains("\\begin{frame}") && !names.iter().any(|n| n == name) {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// One frame of the source, as found by `frame_spans`.
#[derive(Clone, Debug)]
struct Span {
    /// Where the command starts, where its body starts and where the body ends.
    start: usize,
    body: usize,
    end: usize,
    /// Its `label=` option, which `\againframe` refers to.
    label: Option<String>,
    /// For `\againframe{label}`: the frame it shows again (an index into the list).
    again: Option<usize>,
}

/// The value of `label=` in a frame's options, if it has one.
fn frame_label(opts: &str) -> Option<String> {
    let at = opts.find("label=")? + "label=".len();
    let v: String = opts[at..].chars().take_while(|c| !matches!(c, ',' | ']' | '}')).collect();
    Some(v.trim().to_string()).filter(|v| !v.is_empty())
}

/// Every frame in comment-free source, in reading order: `\begin{…}` … `\end{…}` of a frame
/// environment (`envs`), `\frame{…}`, and `\againframe{label}`, which shows a labelled frame again
/// and so is a slide of its own in the PDF. Only the document body counts, so a definition in the
/// preamble is not taken for a frame.
fn frame_spans(clean: &str, envs: &[String]) -> Vec<Span> {
    let b = clean.as_bytes();
    let mut out: Vec<Span> = vec![];
    let mut i = clean.find("\\begin{document}").map(|p| p + "\\begin{document}".len()).unwrap_or(0);
    while i < b.len() {
        if b[i] != b'\\' {
            i += 1;
            continue;
        }
        let rest = &clean[i..];
        if let Some((name, _)) = rest.strip_prefix("\\begin{").and_then(|r| r.split_once('}')) {
            if envs.iter().any(|e| e == name) {
                let after = i + "\\begin{".len() + name.len() + 1;
                let close = format!("\\end{{{name}}}");
                let end = clean[after..].find(&close).map(|e| after + e).unwrap_or(clean.len());
                let label = frame_label(&clean[after..skip_opts(clean, after).min(end)]);
                out.push(Span { start: i, body: after, end, label, again: None });
                i = end.max(after);
                continue;
            }
        } else if rest.starts_with("\\frame") && !rest[6..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            // The command form: \frame<…>[…]{…}.
            let o = skip_opts(clean, i + 6);
            if let Some((_, e)) = braced(clean, o) {
                let label = frame_label(&clean[i + 6..o]);
                out.push(Span { start: i, body: o + 1, end: e - 1, label, again: None });
                i = e;
                continue;
            }
        } else if rest.starts_with("\\againframe") && !rest[11..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            let o = skip_opts(clean, i + 11);
            if let Some((name, e)) = braced(clean, o) {
                let again = out.iter().position(|f| f.label.as_deref() == Some(name.trim()));
                out.push(Span { start: i, body: e, end: e, label: None, again });
                i = e;
                continue;
            }
        }
        // A backslash and the character after it go together (\\, \%, \{).
        i += if rest.len() > 1 && !rest.as_bytes()[1].is_ascii_alphabetic() { 2 } else { 1 };
    }
    out
}

/// Puts `note` (LaTeX) as the `\note{...}` of frame number `frame` (from 0) in the source, and returns
/// the new source. An existing `\note` is replaced (more than one become one); a frame without one
/// gets it just before its `\end{frame}`; an empty `note` removes the frame's notes. `None` when the
/// frame is not there.
#[cfg(test)]
pub fn set_frame_note(src: &str, frame: usize, note: &str) -> Option<String> {
    set_frame_note_with(src, frame, note, &frame_envs(src, None))
}

/// `set_frame_note`, with the frame environments already known (see `frame_envs`).
pub fn set_frame_note_with(src: &str, frame: usize, note: &str, envs: &[String]) -> Option<String> {
    let lines: Vec<&str> = src.split('\n').collect();
    let clean: String = lines.iter().map(|l| strip_comment(l)).collect::<Vec<_>>().join("\n");
    // Comments only ever end a line, so a position keeps its line and column in the original text.
    let mut starts = vec![0usize];
    for l in &lines {
        starts.push(starts.last().unwrap() + l.len() + 1);
    }
    let clean_starts: Vec<usize> = {
        let mut v = vec![0usize];
        for l in clean.split('\n') {
            v.push(v.last().unwrap() + l.len() + 1);
        }
        v
    };
    let to_orig = |p: usize| {
        let line = clean_starts.partition_point(|s| *s <= p) - 1;
        starts[line] + (p - clean_starts[line])
    };
    let spans = frame_spans(&clean, envs);
    // A note for a slide shown again with \againframe belongs to the frame it repeats.
    let span = spans.get(frame)?;
    let span = match span.again {
        Some(k) => spans.get(k)?,
        None if span.body == span.end => return None,
        None => span,
    };
    let (after, end) = (span.body, span.end);
    if end >= clean.len() {
        return None;
    }
    // Every \note in the frame: (where the command starts, where its braces start, where they end).
    let mut spans = vec![];
    let mut np = after;
    while let Some(q) = clean[np..end].find("\\note") {
        let at = np + q;
        let o = skip_opts(&clean, at + 5);
        match braced(&clean, o) {
            Some((_, e)) if e <= end => {
                spans.push((at, o, e));
                np = e;
            }
            _ => np = at + 5,
        }
    }
    let end_o = to_orig(end);
    let line_start = src[..end_o].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let before_end = &src[line_start..end_o];
    let alone = before_end.trim().is_empty();
    let indent = format!("{}  ", if alone { before_end } else { "" });
    let body = note.lines().collect::<Vec<_>>().join(&format!("\n{indent}  "));
    // Changes from the back, so earlier positions stay right.
    let mut edits: Vec<(usize, usize, String)> = vec![];
    if note.trim().is_empty() {
        for &(at, _, e) in &spans {
            edits.push((to_orig(at), to_orig(e), String::new()));
        }
    } else if let Some(&(_, o, e)) = spans.first() {
        edits.push((to_orig(o), to_orig(e), format!("{{{body}}}")));
        for &(at, _, e) in &spans[1..] {
            edits.push((to_orig(at), to_orig(e), String::new()));
        }
    } else if alone {
        edits.push((line_start, line_start, format!("{indent}\\note{{{body}}}\n")));
    } else {
        edits.push((end_o, end_o, format!("\\note{{{body}}} ")));
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = src.to_string();
    for (a, b, text) in edits {
        out.replace_range(a..b, &text);
        // A line left with nothing on it goes too.
        if text.is_empty() {
            let ls = out[..a].rfind('\n').map(|p| p + 1).unwrap_or(0);
            let le = out[a..].find('\n').map(|p| a + p).unwrap_or(out.len());
            if out[ls..le].trim().is_empty() && le < out.len() {
                out.replace_range(ls..le + 1, "");
            }
        }
    }
    Some(out)
}

/// One entry per `\begin{frame}` in reading order: where it is, its title and its `\note`s.
pub fn parse_tex_frames(src: &str) -> Vec<TexFrame> {
    parse_tex_frames_with(src, &frame_envs(src, None))
}

/// `parse_tex_frames`, with the frame environments already known (see `frame_envs`).
pub fn parse_tex_frames_with(src: &str, envs: &[String]) -> Vec<TexFrame> {
    let clean: String = src.lines().map(strip_comment).collect::<Vec<_>>().join("\n");
    let line_at = |pos: usize| clean[..pos].matches('\n').count() + 1;
    let mut frames = vec![];
    for span in frame_spans(&clean, envs) {
        let (start, after, end) = (span.start, span.body, span.end);
        // \againframe repeats a frame: same title and notes, at the \againframe line.
        if let Some(k) = span.again {
            let mut f: TexFrame = frames.get(k).cloned().unwrap_or_default();
            (f.line, f.end_line) = (line_at(start), line_at(end));
            frames.push(f);
            continue;
        }
        let body = &clean[start..end];
        let mut f = TexFrame { line: line_at(start), end_line: line_at(end), ..Default::default() };
        if let Some((t, _)) = braced(&clean, skip_opts(&clean, after)) {
            f.title = latex_to_md(t).replace(['*', '`'], "");
        }
        if let Some(q) = body.find("\\frametitle") {
            if let Some((t, _)) = braced(body, skip_opts(body, q + 11)) {
                f.title = latex_to_md(t).replace(['*', '`'], "");
            }
        }
        let mut np = 0;
        while let Some(q) = body[np..].find("\\note") {
            let at = np + q + 5;
            if let Some((t, e)) = braced(body, skip_opts(body, at)) {
                if !f.note.is_empty() {
                    f.note.push('\n');
                }
                f.note.push_str(&latex_to_md(t));
                np = e;
            } else {
                np = at;
            }
        }
        frames.push(f);
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECK: &str = "\\begin{document}\n\\begin{frame}{One}\n  Hello % a comment {\n  \\note{Old note}\n\\end{frame}\n\n\\begin{frame}{Two}\n  World\n\\end{frame}\n\\end{document}\n";

    /// A deck with a custom Beamer class, a frame environment of its own (defined in the source), and
    /// the command form \frame{…}.
    const CUSTOM: &str = "\\documentclass{acmedeck}\n\
\\newenvironment<>{acmeslide}[1]{\\begin{frame}[environment=acmeslide]{#1}}{\\end{frame}}\n\
\\begin{document}\n\
\\begin{frame}{One}\n\
  A \\note{Note one}\n\
\\end{frame}\n\
\\frame{\\frametitle{Two} B \\note{Note two}}\n\
\\begin{acmeslide}{Three}\n\
  C \\note{Note three}\n\
\\end{acmeslide}\n\
\\frame<1-2>[label=four]{\n\
  \\frametitle{Four} D\n\
}\n\
\\frametitle{not a frame} \\framezoom<1>{2}(0cm,0cm)(1cm,1cm)\n\
\\end{document}\n";

    #[test]
    fn frames_of_any_beamer_form_are_found() {
        assert_eq!(frame_envs(CUSTOM, None), vec!["frame".to_string(), "acmeslide".to_string()]);
        let f = parse_tex_frames(CUSTOM);
        let got: Vec<(&str, &str, usize)> = f.iter().map(|x| (x.title.as_str(), x.note.as_str(), x.line)).collect();
        assert_eq!(got, vec![("One", "Note one", 4), ("Two", "Note two", 7), ("Three", "Note three", 8), ("Four", "", 11)], "the definition in the preamble is not a frame");
        assert_eq!(f[2].end_line, 10);
        assert_eq!(f[3].end_line, 13);
    }

    #[test]
    fn notes_go_into_the_right_frame_of_any_form() {
        let envs = frame_envs(CUSTOM, None);
        let s = set_frame_note_with(CUSTOM, 1, "New two", &envs).unwrap();
        assert!(s.contains("\\frame{\\frametitle{Two} B \\note{New two}}"), "{s}");
        let s = set_frame_note_with(CUSTOM, 2, "New three", &envs).unwrap();
        assert!(s.contains("C \\note{New three}\n\\end{acmeslide}"), "{s}");
        let s = set_frame_note_with(CUSTOM, 3, "Added", &envs).unwrap();
        assert_eq!(parse_tex_frames_with(&s, &envs)[3].note, "Added", "{s}");
        assert!(set_frame_note_with(CUSTOM, 4, "x", &envs).is_none(), "there is no fifth frame");
    }

    #[test]
    fn a_frame_shown_again_keeps_its_notes() {
        let src = "\\documentclass{beamer}\n\\begin{document}\n\
\\begin{frame}[label=goals]{Goals}\n\\note{Goals note}\n\\end{frame}\n\
\\frame[label=res]{\\frametitle{Results}\\note{Results note}}\n\
\\againframe<2>[t]{goals}\n\
\\begin{frame}{After}\n\\note{After note}\n\\end{frame}\n\
\\againframe{res}\n\
\\end{document}\n";
        let f = parse_tex_frames(src);
        let got: Vec<(&str, &str, usize)> = f.iter().map(|x| (x.title.as_str(), x.note.as_str(), x.line)).collect();
        assert_eq!(got, vec![("Goals", "Goals note", 3), ("Results", "Results note", 6), ("Goals", "Goals note", 7), ("After", "After note", 8), ("Results", "Results note", 11)], "five slides, one per entry, in order");
        // A note written on a repeated slide goes into the frame it repeats.
        let s = set_frame_note(src, 2, "New goals").unwrap();
        assert!(s.contains("[label=goals]{Goals}\n\\note{New goals}"), "{s}");
        assert_eq!(parse_tex_frames(&s)[2].note, "New goals", "and shows on both slides");
        let s = set_frame_note(src, 4, "New results").unwrap();
        assert!(s.contains("\\frametitle{Results}\\note{New results}}"), "{s}");
    }

    #[test]
    fn a_frame_environment_from_a_local_theme_counts() {
        let dir = std::env::temp_dir().join(format!("texora-theme-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("beamerthemeAcme.sty"), "\\mode<presentation>\n\\NewDocumentEnvironment{customframe}{m}{\\begin{frame}[environment=customframe]{#1}}{\\end{frame}}\n\\mode<all>\n").unwrap();
        let src = "\\documentclass{beamer}\n\\usetheme{Acme}\n\\begin{document}\n\\begin{customframe}{Hi}\n\\note{N}\n\\end{customframe}\n\\end{document}\n";
        assert_eq!(parse_tex_frames(src).len(), 0, "without the theme's folder it is unknown");
        let envs = frame_envs(src, Some(&dir));
        assert!(envs.contains(&"customframe".to_string()), "{envs:?}");
        let f = parse_tex_frames_with(src, &envs);
        assert_eq!((f.len(), f[0].title.as_str(), f[0].note.as_str()), (1, "Hi", "N"));
    }

    #[test]
    fn notes_written_in_the_presenter_go_into_the_frame() {
        let md = "First line\nSecond **bold** & 50% with `code`\n- one\n- two\nAfter the list";
        let tex = md_to_latex(md);
        // Replacing a frame's note.
        let s = set_frame_note(DECK, 0, &tex).unwrap();
        assert!(s.contains("% a comment {"), "comments are kept: {s}");
        let f = parse_tex_frames(&s);
        assert_eq!(f[0].note, md);
        assert_eq!(f[1].note, "");
        // Adding one where there was none, just before \end{frame}.
        let s = set_frame_note(&s, 1, &md_to_latex("New")).unwrap();
        assert_eq!(parse_tex_frames(&s)[1].note, "New");
        assert!(s.contains("  World\n  \\note{New}\n\\end{frame}"), "{s}");
        // Clearing it takes the \note away, line and all.
        let s = set_frame_note(&s, 1, "").unwrap();
        assert_eq!(parse_tex_frames(&s)[1].note, "");
        assert!(!s.contains("\\note{New}") && s.contains("  World\n\\end{frame}"), "{s}");
        // A frame that is not there.
        assert!(set_frame_note(DECK, 5, "x").is_none());
    }

    #[test]
    fn every_language_and_sign_survives_the_round_trip() {
        for md in ["ذهبت الى السوق", "100% sure: $5 & #1_a {b} ~ ^ \\", "*emphasis* and **bold**", "Paragraph one\n\nParagraph two"] {
            let s = set_frame_note(DECK, 0, &md_to_latex(md)).unwrap();
            assert_eq!(parse_tex_frames(&s)[0].note, md, "{s}");
        }
    }

    #[test]
    fn latex_notes_read_without_stray_blank_lines() {
        let src = "\\begin{frame}\n\\note{Hi.\\\\\n  Next line.\n  \\begin{itemize}\n    \\item a\n    \\item b\n  \\end{itemize}\n  Done.}\n\\end{frame}";
        assert_eq!(parse_tex_frames(src)[0].note, "Hi.\nNext line.\n- a\n- b\nDone.");
    }

    #[test]
    fn notes_files_are_named_after_the_presentation() {
        assert_eq!(notes_file(Path::new("/talks/Lecture.pdf")), Path::new("/talks/Lecture.note.md"));
        assert_eq!(notes_file(Path::new("/talks/Thesis defense.tex")), Path::new("/talks/Thesis defense.note.md"));
        assert_eq!(notes_file(Path::new("/talks/Week 3.v2.pdf")), Path::new("/talks/Week 3.v2.note.md"));
    }

    #[test]
    fn notes_under_the_older_name_are_kept_and_moved_over() {
        let d = std::env::temp_dir().join(format!("txp-note-name-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let deck = d.join("Lecture.pdf");
        let _ = std::fs::remove_file(notes_file(&deck));
        std::fs::write(older_notes_file(&deck), "## Slide 1\nOld note\n").unwrap();
        let mut n = load_notes(&deck);
        assert_eq!(n.get(&0).map(String::as_str), Some("Old note"));
        n.insert(1, "New note".to_string());
        save_notes(&deck, &n).unwrap();
        assert!(!older_notes_file(&deck).exists(), "the old file became the new one");
        let saved = load_notes(&deck);
        assert_eq!(saved.get(&0).map(String::as_str), Some("Old note"));
        assert_eq!(saved.get(&1).map(String::as_str), Some("New note"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn notes_files_read_in_any_common_encoding() {
        let text = "## Slide 1\nمرحبا — שלום — 你好 — café\n";
        assert_eq!(decode_text(text.as_bytes()), text);
        let mut bom = vec![0xEF, 0xBB, 0xBF];
        bom.extend(text.as_bytes());
        assert_eq!(decode_text(&bom), text);
        let le: Vec<u8> = [0xFF, 0xFE].into_iter().chain(text.encode_utf16().flat_map(|u| u.to_le_bytes())).collect();
        assert_eq!(decode_text(&le), text);
        let be: Vec<u8> = [0xFE, 0xFF].into_iter().chain(text.encode_utf16().flat_map(|u| u.to_be_bytes())).collect();
        assert_eq!(decode_text(&be), text);
        let plain = "## Slide 1\nHello there\n";
        let no_mark: Vec<u8> = plain.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(decode_text(&no_mark), plain);
        // café and “quotes” in Windows-1252.
        assert_eq!(decode_text(&[b'c', b'a', b'f', 0xE9, b' ', 0x93, b'q', 0x94]), "café “q”");
    }

    #[test]
    fn notes_in_any_language_survive_saving_and_loading() {
        let mut n = Notes::new();
        n.insert(0, "العربية: مرحبا بكم".to_string());
        n.insert(1, "עברית, 中文, 日本語, 한국어, हिन्दी, ไทย 🎤".to_string());
        assert_eq!(parse_md(&decode_text(to_md(&n).as_bytes())), n);
    }

    #[test]
    fn markdown_round_trip() {
        let mut n = Notes::new();
        n.insert(0, "Hello **all**".into());
        n.insert(4, "- one\n- two".into());
        assert_eq!(parse_md(&to_md(&n)), n);
    }

    #[test]
    fn frames_titles_and_notes_from_source() {
        let src = "% \\begin{frame} ignored\n\\begin{frame}[fragile]{Intro \\emph{x}}\n\\note{Say \\textbf{hi}\\\\ then go}\n\\end{frame}\n\\begin{frame}<1->\n\\frametitle{Second}\n\\note[item]{a}\n\\note<2>{b}\n\\end{frame}\n";
        let f = parse_tex_frames(src);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].line, 2);
        assert_eq!(f[0].title, "Intro x");
        assert_eq!(f[0].note, "Say **hi**\nthen go");
        assert_eq!(f[1].title, "Second");
        assert_eq!(f[1].note, "a\nb");
    }

    #[test]
    fn escaped_percent_is_not_a_comment() {
        let f = parse_tex_frames("\\begin{frame}{50\\% done}\n\\end{frame}");
        // Not a comment, and shown as the sign itself.
        assert_eq!(f[0].title, "50% done");
    }

    #[test]
    fn hyperlink_keeps_visible_text() {
        assert_eq!(latex_to_md("See \\hyperlink{x}{the backup}."), "See the backup.");
    }
}
