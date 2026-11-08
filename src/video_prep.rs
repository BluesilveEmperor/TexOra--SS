//! Getting a presentation's videos ready before LaTeX runs, for `\usepackage{texoravideo}`.
//!
//! TeXora ships the package (`assets/tex/texoravideo.sty`) and puts its folder in TEXINPUTS while it
//! compiles, so there is nothing to install or copy. Before each compile it finds every
//! `\includevideo{...}`, finds the file, and makes the clip's poster: `texora-video/<MD5>.jpg` beside
//! the presentation, where MD5 is that of the clip's path (the package works out the same name).
//!
//! The path is read exactly as written, from the presentation's folder: `{clip.mp4}` is beside the
//! .tex file and `{videos/clip.mp4}` is in its `videos` folder (any folder name). Each video has one
//! poster, however often it is used; if uses ask for different `posterat` values, the
//! first one counts and the others are reported. A poster is made again only when its clip's size or
//! modification time changes; what was found is kept in `texora-video/index.json`, which stays with
//! the PDF so it plays when opened later. Posters no longer used are removed.
//!
//! Safety: paths come from the document, so only files inside the presentation's folder are used
//! (no absolute paths, no `..` out of it, no links leading out, no web addresses), and files are opened
//! directly by the decoder; no command is ever built from them.
use crate::video::{self, Problem};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

pub const STY: &str = include_str!("../assets/tex/texoravideo.sty");
pub const FOLDER: &str = "texora-video";
/// Posters are at most this wide; a bigger one only makes the PDF larger.
const POSTER_MAX_WIDTH: u32 = 1280;

/// One `\includevideo` in the source.
#[derive(Clone, Debug, PartialEq)]
pub struct Use {
    /// The path as written, with runs of spaces made one, as TeX reads it.
    pub path: String,
    /// `posterat`, in percent, when this use gives one.
    pub at: Option<u32>,
    /// Line in the file it is in (from 1).
    pub line: usize,
    pub file: PathBuf,
}

/// What `index.json` keeps about each poster.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub path: String,
    pub at: u32,
    pub size: u64,
    /// Modification time, in seconds since 1970.
    pub modified: u64,
    pub width: u32,
    pub height: u32,
    pub duration: f64,
    /// Why it cannot be played, if so (the words shown over its poster).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    /// Something worth knowing that does not stop it playing (listed under Problems).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Index {
    /// By poster file name.
    pub posters: BTreeMap<String, Entry>,
}

/// What getting ready did, for the compile report.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub made: usize,
    pub reused: usize,
    /// Each clip that cannot be played: the file and line of its `\includevideo`, and the words.
    pub problems: Vec<(PathBuf, usize, String)>,
}

/// MD5 in capitals, as `\pdf@mdfivesum` writes it.
pub fn md5_upper(s: &str) -> String {
    use md5::{Digest, Md5};
    Md5::digest(s.as_bytes()).iter().map(|b| format!("{b:02X}")).collect()
}

/// The poster's file name for a clip's path (as found: `clip.mp4` or `media/clip.mp4`), the same one
/// the package computes. One poster per video.
pub fn poster_name(path: &str) -> String {
    format!("{}.jpg", md5_upper(path))
}

/// Whether a file in `texora-video/` is a poster TeXora made (an MD5 in capitals and `.jpg`).
/// Older versions named a poster taken partway in `<MD5>-<percent>.jpg`; those are TeXora's too.
fn is_poster_file(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".jpg") else { return false };
    let (hash, rest) = stem.split_at(stem.len().min(32));
    let hex = hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b));
    let old_suffix = rest.is_empty() || rest.strip_prefix('-').is_some_and(|p| !p.is_empty() && p.len() <= 2 && p.bytes().all(|b| b.is_ascii_digit()));
    hex && old_suffix
}

/// The folder holding the package; it is made (or refreshed) when needed. Its path never holds a
/// doubled separator, since TeX would read `//` as "search every folder below".
pub fn tex_dir() -> std::io::Result<PathBuf> {
    let dir = crate::settings::data_dir().join("tex");
    std::fs::create_dir_all(&dir)?;
    let dir = dir.canonicalize().unwrap_or(dir);
    write_if_changed(&dir.join("texoravideo.sty"), STY.as_bytes())?;
    write_if_changed(&dir.join("texoravideo-placeholder.png"), &placeholder_png())?;
    Ok(dir)
}

fn write_if_changed(p: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if std::fs::read(p).ok().as_deref() != Some(bytes) {
        std::fs::write(p, bytes)?;
    }
    Ok(())
}

/// TEXINPUTS with TeXora's package folder first and the usual places after it (the empty entry at the
/// end means "and the default search path").
pub fn texinputs(dir: &Path) -> std::ffi::OsString {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut s = std::ffi::OsString::from(dir.as_os_str());
    s.push(sep);
    if let Some(old) = std::env::var_os("TEXINPUTS") {
        s.push(&old);
        if !old.to_string_lossy().ends_with(sep) {
            s.push(sep);
        }
    }
    s
}

/// A plain dark 16:9 picture with a play sign, used when a clip has no poster (missing, refused or
/// unsupported). TeXora writes the reason over it.
pub fn placeholder_png() -> Vec<u8> {
    let (w, h) = (640u32, 360u32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let (cx, cy, r) = (w as f32 / 2.0, h as f32 / 2.0, 54.0f32);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let mut c = [43u8, 47, 51];
            if (fx * fx + fy * fy).sqrt() < r {
                c = [70, 76, 82];
                // A triangle pointing right, inside the circle.
                let (tx, ty) = (fx + 8.0, fy);
                if tx > -18.0 && tx < 26.0 && ty.abs() < (26.0 - tx) * 0.62 {
                    c = [225, 228, 230];
                }
            }
            let o = ((y * w + x) * 4) as usize;
            px[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    crate::pdf::encode_png(w, h, &px).unwrap_or_default()
}

/// Leaves out comments: a `%` not written as `\%` ends the line's text.
fn uncomment(src: &str) -> String {
    src.lines()
        .map(|l| {
            let b = l.as_bytes();
            let end = (0..b.len()).find(|&i| b[i] == b'%' && (i == 0 || b[i - 1] != b'\\')).unwrap_or(b.len());
            &l[..end]
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text inside a balanced pair starting at `open` (`[` or `{`), and where it ends.
fn balanced(s: &str, open: usize) -> Option<(&str, usize)> {
    let (o, c) = match s.as_bytes()[open] {
        b'[' => (b'[', b']'),
        _ => (b'{', b'}'),
    };
    let mut depth = 0i32;
    let mut braces = 0i32;
    for (i, &b) in s.as_bytes().iter().enumerate().skip(open) {
        if o == b'[' {
            // Brackets inside braces do not count: [width={a]b}].
            match b {
                b'{' => braces += 1,
                b'}' => braces -= 1,
                b'[' if braces == 0 => depth += 1,
                b']' if braces == 0 => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((&s[open + 1..i], i + 1));
                    }
                }
                _ => {}
            }
        } else if b == o {
            depth += 1;
        } else if b == c {
            depth -= 1;
            if depth == 0 {
                return Some((&s[open + 1..i], i + 1));
            }
        }
    }
    None
}

/// The `key=value` pairs of a video options list.
pub fn options(list: &str) -> Vec<(String, String)> {
    list.split(',').filter(|p| !p.trim().is_empty()).map(|p| match p.split_once('=') {
        Some((k, v)) => (k.trim().to_string(), v.trim().to_string()),
        None => (p.trim().to_string(), "true".to_string()),
    }).collect()
}

/// Every `\includevideo` in one file's text.
pub fn uses_in(src: &str, file: &Path) -> Vec<Use> {
    let clean = uncomment(src);
    let mut out = vec![];
    let name = "\\includevideo";
    let mut pos = 0;
    while let Some(p) = clean[pos..].find(name) {
        let start = pos + p;
        let mut i = start + name.len();
        pos = i;
        if clean[i..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            continue;
        }
        let mut opts: Vec<&str> = vec![];
        let b = clean.as_bytes();
        loop {
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < b.len() && b[i] == b'[' && opts.len() < 2 {
                let Some((inner, end)) = balanced(&clean, i) else { break };
                opts.push(inner);
                i = end;
                continue;
            }
            break;
        }
        if i >= b.len() || b[i] != b'{' {
            continue;
        }
        let Some((arg, end)) = balanced(&clean, i) else { continue };
        pos = end;
        // TeX reads any run of spaces or line ends as one space.
        let path = arg.split_whitespace().collect::<Vec<_>>().join(" ");
        let path = if arg.starts_with(char::is_whitespace) { format!(" {path}") } else { path };
        let at = opts.get(1).map(|o| options(o)).unwrap_or_default().iter().find(|(k, _)| k == "posterat").and_then(|(_, v)| v.trim_end_matches('%').trim().parse::<f64>().ok()).map(|v| v.clamp(0.0, 99.0) as u32);
        let line = clean[..start].matches('\n').count() + 1;
        out.push(Use { path, at, line, file: file.to_path_buf() });
    }
    out
}

/// Every `\includevideo` of a document: the main file and the files it `\input`s or `\include`s,
/// inside its folder.
pub fn uses(root: &Path) -> Vec<Use> {
    let dir = root.parent().unwrap_or(Path::new("."));
    let mut out = vec![];
    let mut seen: Vec<PathBuf> = vec![];
    let mut todo = vec![root.to_path_buf()];
    while let Some(f) = todo.pop() {
        if seen.contains(&f) || seen.len() > 200 {
            continue;
        }
        seen.push(f.clone());
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        out.extend(uses_in(&src, &f));
        let clean = uncomment(&src);
        for cmd in ["\\input{", "\\include{", "\\subfile{"] {
            for (k, _) in clean.match_indices(cmd) {
                let open = k + cmd.len() - 1;
                if let Some((name, _)) = balanced(&clean, open) {
                    let name = name.trim();
                    if let Ok(p) = resolve(dir, name) {
                        todo.push(p);
                    } else if let Ok(p) = resolve(dir, &format!("{name}.tex")) {
                        todo.push(p);
                    }
                }
            }
        }
    }
    out
}

/// The file a path written in the document means, if it may be used: inside the presentation's
/// folder `dir`, and a file.
pub fn resolve(dir: &Path, written: &str) -> Result<PathBuf, Problem> {
    let w = written.trim();
    if w.is_empty() {
        return Err(Problem::Refused("No file name was given.".into()));
    }
    if w.contains("://") || w.starts_with("www.") {
        return Err(Problem::Refused("Web addresses are not played. Save the video in the presentation's folder.".into()));
    }
    let rel = Path::new(w);
    let drive = w.len() > 1 && w.as_bytes()[1] == b':';
    if rel.is_absolute() || rel.has_root() || drive || w.starts_with('~') {
        return Err(Problem::Refused("Only files inside the presentation's folder are played. Write the path from that folder.".into()));
    }
    if rel.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(Problem::Refused("The path leads out of the presentation's folder (\"..\").".into()));
    }
    let base = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let full = base.join(rel);
    if !full.exists() {
        return Err(Problem::Missing);
    }
    // A link inside the folder that points out of it is refused too.
    let real = full.canonicalize().map_err(|e| Problem::Broken(e.to_string()))?;
    if !real.starts_with(&base) {
        return Err(Problem::Refused("It is a link to a file outside the presentation's folder.".into()));
    }
    if !real.is_file() {
        return Err(Problem::Refused("It is a folder, not a file.".into()));
    }
    Ok(real)
}

/// The clip written as `written`: the path is read exactly as written, from the presentation's folder
/// (`clip.mp4` beside it, `videos/clip.mp4` in a folder inside it). Returns the file and the path,
/// which names its poster.
pub fn locate(dir: &Path, written: &str) -> Result<(PathBuf, String), Problem> {
    resolve(dir, written.trim()).map(|f| (f, written.to_string()))
}

fn stamp(p: &Path) -> (u64, u64) {
    let m = std::fs::metadata(p).ok();
    let size = m.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified = m.and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    (size, modified)
}

/// Whether a clip is the same as when its index entry was written (same size and modification time).
pub fn unchanged(e: &Entry, file: &Path) -> bool {
    let (size, modified) = stamp(file);
    e.size == size && e.modified == modified
}

pub fn read_index(dir: &Path) -> Index {
    std::fs::read(dir.join(FOLDER).join("index.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// A JPEG of an RGBA picture, made smaller if wider than posters need to be.
fn poster_jpeg(w: u32, h: u32, rgba: Vec<u8>) -> Option<Vec<u8>> {
    let img = image::RgbaImage::from_raw(w, h, rgba)?;
    let img = if w > POSTER_MAX_WIDTH {
        image::imageops::resize(&img, POSTER_MAX_WIDTH, (h as u64 * POSTER_MAX_WIDTH as u64 / w as u64).max(1) as u32, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let mut out = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88).encode_image(&rgb).ok()?;
    Some(out)
}

/// Gets every clip of the document `root` ready: checks it and makes its poster unless an up-to-date
/// one is there. Clips are worked on side by side.
pub fn prepare(root: &Path) -> Report {
    let uses = uses(root);
    let mut report = Report::default();
    if uses.is_empty() {
        return report;
    }
    let dir = root.parent().unwrap_or(Path::new(".")).to_path_buf();
    let out_dir = dir.join(FOLDER);
    if std::fs::create_dir_all(&out_dir).is_err() {
        return report;
    }
    let old = read_index(&dir);
    // One job per video, wherever it is used and however it is written: `clip.mp4` and
    // `media/clip.mp4` are the same clip when it is in `media`. The first `posterat` given counts.
    struct Job {
        key: String,
        found: Result<PathBuf, Problem>,
        at: u32,
        uses: Vec<usize>,
    }
    let mut jobs: Vec<Job> = vec![];
    for (i, u) in uses.iter().enumerate() {
        let found = locate(&dir, &u.path);
        let key = found.as_ref().map(|(_, k)| k.clone()).unwrap_or_else(|_| u.path.clone());
        match jobs.iter_mut().find(|j| j.key == key) {
            Some(j) => j.uses.push(i),
            None => jobs.push(Job { key, found: found.map(|(f, _)| f), at: 0, uses: vec![i] }),
        }
    }
    for j in &mut jobs {
        let first = j.uses.iter().find_map(|&i| uses[i].at.map(|a| (i, a)));
        if let Some((fi, at)) = first {
            j.at = at;
            for &i in &j.uses {
                if let Some(other) = uses[i].at.filter(|&a| a != at) {
                    let u = &uses[i];
                    report.problems.push((u.file.clone(), u.line, format!("posterat={other} is not used: {} has one poster, taken at {at}% as asked on line {}.", j.key, uses[fi].line)));
                }
            }
        }
    }
    let results: Vec<(String, Entry, bool)> = std::thread::scope(|s| {
        let handles: Vec<_> = jobs
            .iter()
            .map(|j| {
                let (out_dir, old) = (&out_dir, &old);
                s.spawn(move || {
                    let name = poster_name(&j.key);
                    let mut e = Entry { path: j.key.clone(), at: j.at, ..Default::default() };
                    let file = match &j.found {
                        Ok(f) => f.clone(),
                        Err(p) => {
                            e.problem = Some(p.message(&j.key));
                            return (name, e, false);
                        }
                    };
                    (e.size, e.modified) = stamp(&file);
                    let target = out_dir.join(&name);
                    if let Some(prev) = old.posters.get(&name) {
                        if prev.size == e.size && prev.modified == e.modified && prev.at == e.at && prev.problem.is_none() && target.is_file() {
                            return (name, prev.clone(), true);
                        }
                    }
                    match video::poster(&file, j.at as f64 / 100.0) {
                        Ok((w, h, rgba)) => {
                            (e.width, e.height) = (w, h);
                            let saved = poster_jpeg(w, h, rgba).is_some_and(|jpg| std::fs::write(&target, jpg).is_ok());
                            if !saved {
                                e.problem = Some(Problem::Broken("Its poster could not be saved.".into()).message(&j.key));
                            }
                        }
                        // VLC plays the clip, but only ffmpeg makes posters: a plain box shows instead.
                        Err(Problem::NeedsPlayer) if matches!(video::backend(), video::Backend::Vlc(_)) => {
                            e.note = Some(format!("No poster for {}: posters need ffmpeg, so a plain box shows. The clip plays in VLC when clicked.", j.key));
                        }
                        Err(p) => {
                            e.problem = Some(p.message(&j.key));
                        }
                    }
                    (name, e, false)
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    let mut index = Index::default();
    for (name, e, reused) in results {
        if reused {
            report.reused += 1;
        } else if e.problem.is_none() {
            report.made += 1;
        }
        if let Some(p) = e.problem.as_ref().or(e.note.as_ref()) {
            for &i in &jobs.iter().find(|j| j.key == e.path).map(|j| j.uses.clone()).unwrap_or_default() {
                let u = &uses[i];
                report.problems.push((u.file.clone(), u.line, p.replace('\n', " ")));
            }
        }
        index.posters.insert(name, e);
    }
    report.problems.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    // Only the posters in use stay: one per video, none for a clip that cannot be played.
    if let Ok(list) = std::fs::read_dir(&out_dir) {
        for f in list.flatten() {
            let n = f.file_name().to_string_lossy().to_string();
            let wanted = index.posters.get(&n).is_some_and(|e| e.problem.is_none());
            if is_poster_file(&n) && !wanted {
                let _ = std::fs::remove_file(f.path());
            }
        }
    }
    if let Ok(json) = serde_json::to_vec_pretty(&index) {
        let _ = std::fs::write(out_dir.join("index.json"), json);
    }
    report
}

/// A clip as a link in the PDF says: `texora-video:a<0|1>l<0|1>m<0|1>p<percent>:<path>`.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkClip {
    pub path: String,
    pub autoplay: bool,
    pub repeat: bool,
    pub mute: bool,
    pub at: u32,
}

pub fn parse_link(uri: &str) -> Option<LinkClip> {
    let rest = uri.strip_prefix("texora-video:")?;
    let (flags, path) = rest.split_once(':')?;
    let flag = |c: char| flags.find(c).and_then(|i| flags[i + 1..].chars().next()) == Some('1');
    let at = flags.find('p').map(|i| flags[i + 1..].chars().take_while(|c| c.is_ascii_digit()).collect::<String>()).and_then(|s| s.parse().ok()).unwrap_or(0);
    Some(LinkClip { path: percent_decode(path), autoplay: flag('a'), repeat: flag('l'), mute: flag('m'), at })
}

/// Undoes `%XX` escapes, which some viewers' copies of a link have; other text is left as it is.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 3 <= b.len() {
            if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poster_names_match_the_package() {
        // The same values pdfTeX, XeTeX and LuaTeX give (see the package).
        assert_eq!(poster_name("test.mp4"), "1A156A436A2D37E2343AC6BD9CDA5686.jpg");
        assert_eq!(poster_name("média/Clip été (1) & more_x.mp4"), "730843651C848A99311D2B74132437A1.jpg");
        assert_eq!(poster_name("فيديو/مقطع.mp4"), "9EDE6849DA6808BFD1AD659703C57605.jpg");
        assert!(is_poster_file("1A156A436A2D37E2343AC6BD9CDA5686.jpg") && !is_poster_file("index.json") && !is_poster_file("cover.jpg"));
        assert!(is_poster_file("1A156A436A2D37E2343AC6BD9CDA5686-50.jpg"), "old names are cleaned up too");
        assert!(!is_poster_file("1A156A436A2D37E2343AC6BD9CDA5686-cover.jpg"));
    }

    #[test]
    fn uses_are_read_with_both_optional_arguments() {
        let src = "% \\includevideo{commented.mp4}\n\\begin{frame}\n  \\includevideo[width=0.5\\textwidth]{test.mp4}\n  \\includevideo[height=5cm][loop, posterat=25]{media/clip  one.gif}\n  \\includevideo[width={a]b}][poster=cover.png,autoplay=false]{vidéo é.mp4}\\pause\n  \\includevideos{not-this}\n\\end{frame}";
        let u = uses_in(src, Path::new("x.tex"));
        let got: Vec<(&str, Option<u32>, usize)> = u.iter().map(|u| (u.path.as_str(), u.at, u.line)).collect();
        assert_eq!(got, vec![("test.mp4", None, 3), ("media/clip one.gif", Some(25), 4), ("vidéo é.mp4", None, 5)]);
    }

    #[test]
    fn unsafe_paths_are_refused() {
        let dir = std::env::temp_dir().join("texora-resolve-test");
        std::fs::create_dir_all(dir.join("media")).unwrap();
        std::fs::write(dir.join("media/ok.mp4"), b"x").unwrap();
        assert!(resolve(&dir, "media/ok.mp4").is_ok());
        for bad in ["/etc/passwd", "../outside.mp4", "media/../../x.mp4", "https://example.com/a.mp4", "C:/Windows/x.mp4", "~/x.mp4", "media"] {
            assert!(matches!(resolve(&dir, bad), Err(Problem::Refused(_))), "{bad} should be refused: {:?}", resolve(&dir, bad));
        }
        assert_eq!(resolve(&dir, "nothing.mp4"), Err(Problem::Missing));
        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(dir.join("out.mp4"));
            std::os::unix::fs::symlink("/etc/hosts", dir.join("out.mp4")).unwrap();
            assert!(matches!(resolve(&dir, "out.mp4"), Err(Problem::Refused(_))));
        }
    }

    #[test]
    fn links_are_read() {
        let l = parse_link("texora-video:a1l0m1p25:vidéos/été (1) & more.mp4").unwrap();
        assert_eq!(l, LinkClip { path: "vidéos/été (1) & more.mp4".into(), autoplay: true, repeat: false, mute: true, at: 25 });
        assert_eq!(parse_link("texora-video:a0l1m0p0:my%20clip.mp4").unwrap().path, "my clip.mp4");
        assert!(parse_link("https://example.com").is_none());
    }

    /// Uses GIFs only, which TeXora decodes itself, so it gives the same result on every computer,
    /// with or without ffmpeg or VLC (the test machines on GitHub have neither).
    #[test]
    fn posters_are_made_once_and_problems_reported() {
        let dir = std::env::temp_dir().join(format!("texora-prep-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("media")).unwrap();
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/test/video");
        std::fs::copy(src.join("test.gif"), dir.join("wave.gif")).unwrap();
        std::fs::copy(src.join("test.gif"), dir.join("media/clip é.gif")).unwrap();
        std::fs::copy(src.join("test.txt"), dir.join("test.txt")).unwrap();
        // A poster left from an older compile, which is no longer used.
        std::fs::create_dir_all(dir.join(FOLDER)).unwrap();
        std::fs::write(dir.join(FOLDER).join("0123456789ABCDEF0123456789ABCDEF.jpg"), b"old").unwrap();
        let tex = dir.join("deck.tex");
        std::fs::write(&tex, "\\includevideo[][posterat=30]{wave.gif}\n\\includevideo[][posterat=50]{wave.gif}\n\\includevideo{media/clip é.gif}\n\\includevideo{does-not-exist.mp4}\n\\includevideo{test.txt}\n\\includevideo{../escape.mp4}\n\\includevideo{media/clip é.gif}\n\\includevideo{wave.gif}\n").unwrap();
        let r = prepare(&tex);
        assert_eq!((r.made, r.reused), (2, 0), "one poster per video: {r:?}");
        let lines: Vec<usize> = r.problems.iter().map(|p| p.1).collect();
        assert_eq!(lines, vec![2, 4, 5, 6], "{:?}", r.problems);
        assert!(r.problems[0].2.contains("posterat=50 is not used") && r.problems[0].2.contains("30%"));
        assert_eq!(r.problems[1].2, "Video not found: Place the video in the same folder as your .tex file or in a subfolder. If it's in a subfolder, include the folder path in \\includevideo.");
        assert!(r.problems[2].2.contains("Not a video"), "{}", r.problems[2].2);
        assert!(r.problems[3].2.contains("out of"));
        // Each GIF, used two or three times, has one poster; the old one is gone.
        let posters: Vec<String> = std::fs::read_dir(dir.join(FOLDER)).unwrap().flatten().map(|f| f.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".jpg")).collect();
        assert_eq!(posters.len(), 2, "{posters:?}");
        assert!(dir.join(FOLDER).join(poster_name("wave.gif")).is_file());
        assert!(dir.join(FOLDER).join(poster_name("media/clip é.gif")).is_file());
        let again = prepare(&tex);
        assert_eq!((again.made, again.reused), (0, 2), "unchanged clips are not made again");
        let idx = read_index(&dir);
        assert_eq!((idx.posters[&poster_name("wave.gif")].width, idx.posters[&poster_name("wave.gif")].at), (320, 30));
        assert!(idx.posters[&poster_name("does-not-exist.mp4")].problem.is_some());
    }

    /// An MP4's poster needs ffmpeg, so this runs only where it is installed.
    #[test]
    fn mp4_posters_are_made_with_installed_ffmpeg() {
        if video::find_ffmpeg().is_none() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("texora-prep-mp4-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/test/video/test.mp4"), dir.join("test.mp4")).unwrap();
        let tex = dir.join("deck.tex");
        std::fs::write(&tex, "\\includevideo[][posterat=30]{test.mp4}\n\\includevideo{test.mp4}\n").unwrap();
        let r = prepare(&tex);
        assert_eq!((r.made, r.reused, r.problems.len()), (1, 0, 0), "{r:?}");
        let idx = read_index(&dir);
        assert_eq!((idx.posters[&poster_name("test.mp4")].width, idx.posters[&poster_name("test.mp4")].at), (640, 30));
    }

    /// A presentation can never choose which program runs or add to its arguments: the player and its
    /// path come only from TeXora's own Settings. Here a project tries every place it controls.
    #[test]
    fn a_project_cannot_set_the_player_or_its_arguments() {
        let before = video::tools();
        let dir = std::env::temp_dir().join(format!("texora-hostile-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(FOLDER)).unwrap();
        // A settings file in the project, an index.json with extra fields, and a .tex that names
        // programs and options in every way it can.
        let evil = r#"{"video_player":"vlc","ffmpeg_path":"/bin/sh","vlc_path":"/bin/sh","check_updates":false}"#;
        std::fs::write(dir.join("settings.json"), evil).unwrap();
        std::fs::write(dir.join(FOLDER).join("index.json"), r#"{"posters":{},"ffmpeg_path":"/bin/sh","args":["-f","lavfi"]}"#).unwrap();
        let tex = dir.join("talk.tex");
        std::fs::write(&tex, "% !TEX program = /bin/sh\n% ffmpeg_path = /bin/sh\n\\includevideo[][autoplay,ffmpeg=/bin/sh,args=-f lavfi]{--ffmpeg=/bin/sh}\n\\includevideo{-i http://example.com/x.mp4}\n").unwrap();
        std::fs::write(dir.join("--ffmpeg=%2Fbin%2Fsh"), b"x").unwrap();
        let r = prepare(&tex);
        assert!(r.problems.iter().all(|p| !p.2.is_empty()));
        let after = video::tools();
        assert_eq!((after.choice, after.ffmpeg, after.vlc), (before.choice, before.ffmpeg, before.vlc), "the project changed nothing");
        // The engine named in the .tex is one of three built-in names, never a program path.
        assert_eq!(crate::latex::detect_engine("% !TEX program = /bin/sh\n"), crate::latex::Engine::PdfLatex);
        // A link in a PDF carries flags and a path, nothing else, and the path stays inside the folder.
        let l = parse_link("texora-video:a1l1m1p5 -f lavfi -i /bin/sh:../../bin/sh").unwrap();
        assert_eq!((l.autoplay, l.repeat, l.mute, l.at), (true, true, true, 5));
        assert!(matches!(resolve(&dir, &l.path), Err(Problem::Refused(_))));
        // A file whose name looks like an option is only ever a full path, after `-i` or `--`.
        let odd = resolve(&dir, "--ffmpeg=%2Fbin%2Fsh").unwrap();
        assert!(video::ffmpeg_input_args(&odd, None).unwrap().last().unwrap().starts_with("file:/"));
        assert_eq!(video::vlc_args(&odd, false).unwrap()[1], "--");
    }

    #[test]
    fn the_path_is_read_as_written_in_any_subfolder() {
        let dir = std::env::temp_dir().join(format!("texora-locate-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("my videos/2026")).unwrap();
        std::fs::write(dir.join("beside.mp4"), b"x").unwrap();
        std::fs::write(dir.join("my videos/2026/inside.mp4"), b"x").unwrap();
        assert_eq!(locate(&dir, "beside.mp4").unwrap().1, "beside.mp4");
        assert_eq!(locate(&dir, "my videos/2026/inside.mp4").unwrap().1, "my videos/2026/inside.mp4");
        // A clip in a folder must be written with its folder: TeXora does not search for it.
        assert_eq!(locate(&dir, "inside.mp4"), Err(Problem::Missing));
        assert_eq!(locate(&dir, "my videos/beside.mp4"), Err(Problem::Missing));
        assert!(matches!(locate(&dir, "../beside.mp4"), Err(Problem::Refused(_))));
    }
}
