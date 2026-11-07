//! Video and other media in a PDF, as made by LaTeX: Beamer's `\movie` (multimedia package), which
//! points at a file beside the PDF, and media9's `\includemedia`, which packs the file into the PDF.
//! TeXora cannot play video inside a slide, so, like Acrobat's "external player" setting, it finds the
//! clip (unpacking a packed one to a temporary folder) and opens it in the computer's video player.
//!
//! PDFium does not expose these annotations' files, so this reads the PDF's objects itself: the plain
//! ones (`12 0 obj ... endobj`) and those packed into compressed object streams.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A clip on a page. `rect` is in PDF points (x0, y0, x1, y1, from the bottom left), as in the file.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaClip {
    pub rect: [f32; 4],
    /// The file's name as the PDF gives it, for showing.
    pub name: String,
    /// Where the file is on this computer: beside the PDF, or unpacked into a temporary folder.
    pub file: Option<PathBuf>,
    /// Plays when its page is shown (`autostart` for `\movie`, `activate=pageopen` for media9).
    pub autoplay: bool,
    /// Plays again and again (`loop`, or the movie's `/Mode /Repeat`).
    pub repeat: bool,
}

/// Every object of a PDF, by number: the dictionary text, and the raw stream bytes where it has one.
pub struct Objects {
    pub text: HashMap<u32, String>,
    stream: HashMap<u32, (String, Vec<u8>)>,
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= hay.len() {
        return None;
    }
    hay[from..].windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

fn inflate(dict: &str, raw: &[u8]) -> Vec<u8> {
    if dict.contains("FlateDecode") {
        miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(raw, 600_000_000).unwrap_or_default()
    } else {
        raw.to_vec()
    }
}

/// The value after `/Key` in a dictionary's text, up to the next key or the end.
fn value<'a>(dict: &'a str, key: &str) -> Option<&'a str> {
    let mut from = 0;
    while let Some(p) = dict[from..].find(key) {
        let at = from + p + key.len();
        // "/F" must not match "/Filter" or "/First".
        if dict[at..].chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) {
            from = at;
            continue;
        }
        return Some(dict[at..].trim_start());
    }
    None
}

/// The first `N 0 R` at the start of `s`.
fn reference(s: &str) -> Option<u32> {
    let mut it = s.split_whitespace();
    let n = it.next()?.trim_end_matches(|c: char| !c.is_ascii_digit()).parse().ok()?;
    let g = it.next()?;
    let r = it.next()?;
    (g.chars().all(|c| c.is_ascii_digit()) && r.starts_with('R')).then_some(n)
}

/// A literal string `(...)` at the start of `s`, with its escapes undone.
fn literal(s: &str) -> Option<String> {
    let s = s.strip_prefix('(')?;
    let (mut out, mut depth, mut chars) = (String::new(), 0, s.chars());
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '(' => {
                depth += 1;
                out.push(c);
            }
            ')' if depth == 0 => return Some(out),
            ')' => {
                depth -= 1;
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    None
}

impl Objects {
    pub fn read(bytes: &[u8]) -> Objects {
        let mut text = HashMap::new();
        let mut stream = HashMap::new();
        let mut at = 0;
        while let Some(p) = find(bytes, b" 0 obj", at) {
            at = p + 6;
            // The object number just before " 0 obj".
            let start = bytes[..p].iter().rposition(|b| !b.is_ascii_digit()).map(|i| i + 1).unwrap_or(0);
            let Ok(num) = std::str::from_utf8(&bytes[start..p]).unwrap_or("").parse::<u32>() else { continue };
            let Some(end) = find(bytes, b"endobj", at) else { break };
            let body = &bytes[at..end];
            match find(body, b"stream", 0) {
                Some(s) => {
                    let dict = String::from_utf8_lossy(&body[..s]).into_owned();
                    let mut from = s + 6;
                    if body.get(from) == Some(&b'\r') {
                        from += 1;
                    }
                    if body.get(from) == Some(&b'\n') {
                        from += 1;
                    }
                    let mut to = find(body, b"endstream", from).unwrap_or(body.len());
                    // The data is /Length bytes long where that is a plain number; otherwise the line
                    // break before "endstream" is not part of it.
                    let length = value(&dict, "/Length").filter(|v| reference(v).is_none()).and_then(|v| v.split(|c: char| !c.is_ascii_digit()).next()?.parse::<usize>().ok());
                    match length {
                        Some(n) if from + n <= to => to = from + n,
                        _ => {
                            if body[..to].ends_with(b"\r\n") {
                                to -= 2;
                            } else if body[..to].ends_with(b"\n") || body[..to].ends_with(b"\r") {
                                to -= 1;
                            }
                        }
                    }
                    let raw = body[from..to].to_vec();
                    text.insert(num, dict.clone());
                    stream.insert(num, (dict, raw));
                }
                None => {
                    text.insert(num, String::from_utf8_lossy(body).into_owned());
                }
            }
            at = end + 6;
        }
        // Objects packed into object streams: "num offset num offset ..." then the objects.
        let packed: Vec<(String, Vec<u8>)> = stream.values().filter(|(d, _)| d.contains("/ObjStm")).cloned().collect();
        for (dict, raw) in packed {
            let data = inflate(&dict, &raw);
            let n: usize = value(&dict, "/N").and_then(|v| v.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()).unwrap_or(0);
            let first: usize = value(&dict, "/First").and_then(|v| v.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()).unwrap_or(0);
            let head = String::from_utf8_lossy(&data[..first.min(data.len())]).into_owned();
            let nums: Vec<usize> = head.split_whitespace().filter_map(|t| t.parse().ok()).collect();
            for i in 0..n.min(nums.len() / 2) {
                let (num, off) = (nums[2 * i], nums[2 * i + 1]);
                let s = (first + off).min(data.len());
                let e = if 2 * i + 3 < nums.len() { (first + nums[2 * i + 3]).min(data.len()) } else { data.len() };
                text.entry(num as u32).or_insert_with(|| String::from_utf8_lossy(&data[s..e.max(s)]).into_owned());
            }
        }
        Objects { text, stream }
    }

    /// The decoded bytes of stream object `num`.
    pub fn stream_bytes(&self, num: u32) -> Option<Vec<u8>> {
        self.stream.get(&num).map(|(d, raw)| inflate(d, raw))
    }

    /// The text of a value that may be written in place or as a reference to another object.
    fn resolve<'a>(&'a self, v: &'a str) -> &'a str {
        match reference(v) {
            Some(n) => self.text.get(&n).map(String::as_str).unwrap_or(v),
            None => v,
        }
    }

    fn rect(dict: &str) -> Option<[f32; 4]> {
        let v = value(dict, "/Rect")?.strip_prefix('[')?;
        let nums: Vec<f32> = v.split(']').next()?.split_whitespace().filter_map(|t| t.parse().ok()).collect();
        (nums.len() == 4).then(|| [nums[0].min(nums[2]), nums[1].min(nums[3]), nums[0].max(nums[2]), nums[1].max(nums[3])])
    }

    /// Every clip in the PDF at `pdf`, with its file found or unpacked into `unpack_to`.
    pub fn clips(&self, pdf: &Path, unpack_to: &Path) -> Vec<MediaClip> {
        let mut out = vec![];
        let page_open_plays = |label: &str| self.text.values().any(|t| t.contains("/Operation") && t.contains("/Play") && t.contains(&format!("({label})")));
        for t in self.text.values() {
            let compact: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
            let subtype = compact.replace("/Subtype /", "/Subtype/");
            if subtype.contains("/Subtype/Movie") {
                let Some(rect) = Self::rect(t) else { continue };
                let movie = value(t, "/Movie").map(|v| self.resolve(v)).unwrap_or("");
                let Some(name) = value(movie, "/F").map(|v| self.resolve(v)).and_then(|v| literal(v).or_else(|| value(v, "/F").and_then(literal))) else {
                    continue;
                };
                let file = pdf.parent().map(|d| d.join(&name)).filter(|p| p.exists());
                let label = value(t, "/T").and_then(literal).unwrap_or_default();
                out.push(MediaClip { rect, autoplay: !label.is_empty() && page_open_plays(&label), repeat: compact.contains("/Repeat"), file, name });
            } else if subtype.contains("/Subtype/RichMedia") && !subtype.contains("/Subtype/RichMediaC") {
                let Some(rect) = Self::rect(t) else { continue };
                let content = value(t, "/RichMediaContent").map(|v| self.resolve(v)).unwrap_or("");
                let assets = value(content, "/Assets").map(|v| self.resolve(v)).unwrap_or("");
                let names = value(assets, "/Names").unwrap_or("");
                // (name) N 0 R pairs; the clip is the first file that is not the Flash player.
                let mut best: Option<(String, u32)> = None;
                let mut rest = names;
                while let Some(p) = rest.find('(') {
                    let Some(name) = literal(&rest[p..]) else { break };
                    let after = &rest[p + name.len() + 2..];
                    if let Some(n) = reference(after.trim_start()) {
                        let lower = name.to_lowercase();
                        let media = [".mp4", ".m4v", ".mov", ".webm", ".mkv", ".avi", ".gif", ".mp3", ".wav", ".m4a", ".flv"].iter().any(|e| lower.ends_with(e));
                        if media && best.is_none() {
                            best = Some((name.clone(), n));
                        }
                    }
                    rest = after;
                }
                let Some((name, spec)) = best else { continue };
                let spec_text = self.text.get(&spec).map(String::as_str).unwrap_or("");
                let ef = value(spec_text, "/EF").map(|v| self.resolve(v)).unwrap_or("");
                let file = value(ef, "/F").and_then(reference).and_then(|n| self.stream_bytes(n)).and_then(|bytes| {
                    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || ".-_".contains(c) { c } else { '_' }).collect();
                    let path = unpack_to.join(safe);
                    std::fs::create_dir_all(unpack_to).ok()?;
                    std::fs::write(&path, bytes).ok()?;
                    Some(path)
                });
                let settings = value(t, "/RichMediaSettings").map(|v| self.resolve(v)).unwrap_or("");
                let activation = value(settings, "/Activation").map(|v| self.resolve(v)).unwrap_or("");
                let autoplay = activation.contains("/PO") || activation.contains("/PV") || activation.contains("/PageOpen") || activation.contains("/PageVisible");
                out.push(MediaClip { rect, name, file, autoplay, repeat: false });
            }
        }
        out.sort_by(|a, b| a.rect[1].total_cmp(&b.rect[1]).reverse().then(a.rect[0].total_cmp(&b.rect[0])));
        out
    }
}

/// Opens a file with the computer's own program for it (for a video, its video player).
pub fn open_with_system(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("cmd").args(["/C", "start", ""]).arg(path).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(path).spawn();
    r.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_references_and_strings_are_read() {
        assert_eq!(value("<</Filter/FlateDecode/F (a.mp4)>>", "/F"), Some("(a.mp4)>>"));
        assert_eq!(reference("12 0 R/Next"), Some(12));
        assert_eq!(reference("(x)"), None);
        assert_eq!(literal("(my \\(clip\\).mp4) 3 0 R").as_deref(), Some("my (clip).mp4"));
    }

    #[test]
    fn a_beamer_movie_points_at_its_file() {
        let d = std::env::temp_dir().join(format!("txp-media-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("clip.mp4"), b"x").unwrap();
        let pdf = b"%PDF-1.5\n7 0 obj\n<< /Type /Annot /Subtype /Movie /T (mmlabel) /Movie << /F (clip.mp4) >> /A << /Mode /Repeat >> /Rect [28 98 141 185] >>\nendobj\n9 0 obj\n<< /S /Movie /T (mmlabel) /Operation /Play >>\nendobj\n";
        let clips = Objects::read(pdf).clips(&d.join("talk.pdf"), &d.join("unpacked"));
        assert_eq!(clips.len(), 1);
        let c = &clips[0];
        assert_eq!((c.name.as_str(), c.autoplay, c.repeat, c.rect), ("clip.mp4", true, true, [28.0, 98.0, 141.0, 185.0]));
        assert_eq!(c.file.as_deref(), Some(d.join("clip.mp4").as_path()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_media9_clip_is_unpacked_from_the_pdf() {
        let d = std::env::temp_dir().join(format!("txp-media9-{}", std::process::id()));
        let mut pdf = b"%PDF-1.5\n5 0 obj\n<</Type/Annot/Subtype/RichMedia/RichMediaSettings 6 0 R/RichMediaContent 8 0 R /Rect [10 20 110 95]>>\nendobj\n6 0 obj\n<</Type/RichMediaSettings/Activation <</Condition/PO>>>>\nendobj\n8 0 obj\n<</Type/RichMediaContent/Assets <</Names [(VPlayer.swf) 9 0 R (clip.mp4) 10 0 R]>>>>\nendobj\n10 0 obj\n<</Type/Filespec/F (clip.mp4)/EF <</F 11 0 R>>>>\nendobj\n11 0 obj\n<</Type/EmbeddedFile/Length 10>>\nstream\n".to_vec();
        pdf.extend_from_slice(b"VIDEOBYTES\nendstream\nendobj\n");
        let clips = Objects::read(&pdf).clips(&d.join("talk.pdf"), &d.join("unpacked"));
        assert_eq!(clips.len(), 1, "{clips:?}");
        assert_eq!((clips[0].name.as_str(), clips[0].autoplay), ("clip.mp4", true));
        let f = clips[0].file.clone().expect("unpacked");
        assert_eq!(std::fs::read(&f).unwrap(), b"VIDEOBYTES");
        let _ = std::fs::remove_dir_all(&d);
    }
}
