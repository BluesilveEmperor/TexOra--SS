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
            out.push('\n');
            i += 2;
            continue;
        }
        if rest.starts_with("\\item") {
            out.push_str("\n- ");
            i += 5;
            continue;
        }
        if rest.starts_with("\\begin{") || rest.starts_with("\\end{") {
            if let Some(p) = rest.find('}') {
                i += p + 1;
                continue;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out.lines().map(str::trim).collect::<Vec<_>>().join("\n").trim().to_string()
}

/// One entry per `\begin{frame}` in reading order: where it is, its title and its `\note`s.
pub fn parse_tex_frames(src: &str) -> Vec<TexFrame> {
    let clean: String = src.lines().map(strip_comment).collect::<Vec<_>>().join("\n");
    let line_at = |pos: usize| clean[..pos].matches('\n').count() + 1;
    let mut frames = vec![];
    let mut pos = 0;
    while let Some(p) = clean[pos..].find("\\begin{frame}") {
        let start = pos + p;
        let after = start + "\\begin{frame}".len();
        let end = clean[after..].find("\\end{frame}").map(|e| after + e).unwrap_or(clean.len());
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
        pos = end.max(after);
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(f[0].title, "50\\% done");
    }

    #[test]
    fn hyperlink_keeps_visible_text() {
        assert_eq!(latex_to_md("See \\hyperlink{x}{the backup}."), "See the backup.");
    }
}
