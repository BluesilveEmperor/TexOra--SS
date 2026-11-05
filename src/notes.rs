//! Text speaker notes: Markdown side files, `\note{}` in LaTeX source, and .pdfpc imports.
use std::collections::BTreeMap;
use std::path::Path;

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

pub fn load_pdfpc(path: &Path) -> Notes {
    let mut out = Notes::new();
    let Ok(src) = std::fs::read_to_string(path) else { return out };
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
