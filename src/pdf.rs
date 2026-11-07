//! PDF access. PDFium lives on one worker thread; the UI sends requests and receives results.
use pdfium_render::prelude::*;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Region {
    Full,
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LinkTarget {
    Page(usize),
    Uri(String),
}

/// A link; `rect` is x0, y0, x1, y1 as fractions of the full page, measured from the top left.
#[derive(Clone, Debug)]
pub struct PageLink {
    pub rect: [f32; 4],
    pub target: LinkTarget,
}

/// What an animation does after its last frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimEnd {
    Stop,
    Loop,
    /// Plays backwards, then forwards again (the `palindrome` option).
    Bounce,
}

/// An animation made with the LaTeX `animate` package. As in Acrobat, its frames are hidden buttons
/// named `<id>.<n>`, stacked on one rectangle, and a script shows them one after another; TeXora
/// reads the frames and the script's settings and plays them itself. `rect` is x0, y0, x1, y1 as
/// fractions of the full page, measured from the top left.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub id: String,
    pub rect: [f32; 4],
    pub frames: usize,
    pub fps: f32,
    pub autoplay: bool,
    pub end: AnimEnd,
    /// Where the PDF's own play and pause buttons are (the `controls` option), if it has them.
    pub controls: Option<[f32; 4]>,
}

#[derive(Clone, Debug, Default)]
pub struct PageInfo {
    pub width: f32,
    pub height: f32,
    pub label: Option<String>,
    pub text_left: String,
    pub text_right: String,
    pub text_full: String,
    pub links: Vec<PageLink>,
    pub animations: Vec<Animation>,
    /// Video and other clips on the page; `rect` is x0, y0, x1, y1 as fractions of the full page,
    /// from the top left, like links.
    pub media: Vec<PageMedia>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PageMedia {
    pub rect: [f32; 4],
    pub clip: crate::media::MediaClip,
}

#[derive(Clone, Debug)]
pub struct OutlineItem {
    pub title: String,
    pub page: usize,
    pub depth: usize,
}

#[derive(Clone, Debug, Default)]
pub struct DocInfo {
    pub path: PathBuf,
    pub title: String,
    pub pages: Vec<PageInfo>,
    pub outline: Vec<OutlineItem>,
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub page: usize,
    pub rects: Vec<[f32; 4]>,
    pub snippet: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderReq {
    pub gen: u64,
    pub page: usize,
    pub region: Region,
    pub width: u32,
}

/// One frame of an animation, at a page width in pixels.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FrameReq {
    pub gen: u64,
    pub page: usize,
    pub anim: usize,
    pub frame: usize,
    pub width: u32,
}

pub enum Cmd {
    Open { path: PathBuf, gen: u64 },
    Close { gen: u64 },
    Render(RenderReq),
    Frame(FrameReq),
    Search { gen: u64, query: String },
}

pub enum Reply {
    Opened { gen: u64, doc: DocInfo },
    Image { req: RenderReq, size: [usize; 2], rgba: Vec<u8> },
    Frame { req: FrameReq, size: [usize; 2], rgba: Vec<u8> },
    Search { gen: u64, query: String, hits: Vec<SearchHit> },
    Failed { gen: u64, msg: String },
    EngineMissing(String),
}

pub struct PdfEngine {
    pub tx: Sender<Cmd>,
    pub rx: Receiver<Reply>,
}

/// Where the PDF engine library might be, most specific first.
fn library_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            dirs.push(d.to_path_buf());
            dirs.push(d.join("../Resources"));
            dirs.push(d.join("../lib"));
        }
    }
    // Development builds may use the copy in the source folder. A release must carry its own,
    // so a package with a missing engine fails its self-test instead of passing by accident.
    if cfg!(debug_assertions) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/pdfium");
        if let Ok(rd) = std::fs::read_dir(&root) {
            for e in rd.flatten() {
                dirs.push(e.path().join("lib"));
                dirs.push(e.path().join("bin"));
            }
        }
    }
    dirs
}

fn bind() -> Result<Pdfium, String> {
    for d in library_dirs() {
        let p = Pdfium::pdfium_platform_library_name_at_path(&d);
        if p.exists() {
            if let Ok(b) = Pdfium::bind_to_library(&p) {
                return Ok(Pdfium::new(b));
            }
        }
    }
    Pdfium::bind_to_system_library().map(Pdfium::new).map_err(|e| format!("PDF engine not found: {e}"))
}

pub fn spawn(wake: impl Fn() + Send + 'static) -> PdfEngine {
    let (tx, cmd_rx) = channel::<Cmd>();
    let (reply_tx, rx) = channel::<Reply>();
    std::thread::spawn(move || {
        let pdfium = match bind() {
            Ok(p) => p,
            Err(e) => {
                let _ = reply_tx.send(Reply::EngineMissing(e));
                wake();
                return;
            }
        };
        // One open document per tab, found by its number.
        let mut docs: std::collections::HashMap<u64, PdfDocument> = std::collections::HashMap::new();
        let mut queue: VecDeque<Cmd> = VecDeque::new();
        loop {
            if queue.is_empty() {
                match cmd_rx.recv() {
                    Ok(c) => queue.push_back(c),
                    Err(_) => return,
                }
            }
            while let Ok(c) = cmd_rx.try_recv() {
                queue.push_back(c);
            }
            // Opening and closing documents come first, in order, so nothing can overtake a close.
            // Then newest render first, so holding the arrow key never leaves the presenter behind.
            let cmd = if let Some(i) = queue.iter().position(|c| matches!(c, Cmd::Open { .. } | Cmd::Close { .. })) {
                queue.remove(i).unwrap()
            } else {
                queue.pop_back().unwrap()
            };
            match cmd {
                Cmd::Open { path, gen } => {
                    let loaded = std::fs::read(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| pdfium.load_pdf_from_byte_vec(bytes, None).map_err(|e| e.to_string()));
                    match loaded {
                        Ok(d) => {
                            let info = inspect(&d, &path);
                            docs.insert(gen, d);
                            let _ = reply_tx.send(Reply::Opened { gen, doc: info });
                        }
                        Err(msg) => {
                            let _ = reply_tx.send(Reply::Failed { gen, msg });
                        }
                    }
                }
                Cmd::Close { gen } => {
                    docs.remove(&gen);
                }
                Cmd::Render(req) => {
                    if let Some(d) = docs.get(&req.gen) {
                        if let Some((size, rgba)) = render(d, req.page, req.region, req.width) {
                            let _ = reply_tx.send(Reply::Image { req, size, rgba });
                        }
                    }
                }
                Cmd::Frame(req) => {
                    if let Some(d) = docs.get(&req.gen) {
                        if let Some((size, rgba)) = render_frame(d, req.page, req.anim, req.frame, req.width) {
                            let _ = reply_tx.send(Reply::Frame { req, size, rgba });
                        }
                    }
                }
                Cmd::Search { gen, query } => {
                    if let Some(d) = docs.get(&gen) {
                        let hits = search(d, &query);
                        let _ = reply_tx.send(Reply::Search { gen, query, hits });
                    }
                }
            }
            wake();
        }
    });
    PdfEngine { tx, rx }
}

pub fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
    let mut out = vec![];
    PngEncoder::new(&mut out).write_image(rgba, w, h, ExtendedColorType::Rgba8).ok()?;
    Some(out)
}

fn inspect(doc: &PdfDocument, path: &Path) -> DocInfo {
    let mut info = DocInfo { path: path.to_path_buf(), ..Default::default() };
    info.title = doc
        .metadata()
        .get(PdfDocumentMetadataTagType::Title)
        .map(|t| t.value().to_string())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
    let mut media_at: Vec<(usize, [f32; 4], [f32; 4])> = vec![];
    for page in doc.pages().iter() {
        let (w, h) = (page.width().value, page.height().value);
        let mut p = PageInfo { width: w, height: h, label: page.label().map(str::to_string), ..Default::default() };
        if let Ok(text) = page.text() {
            p.text_full = text.all();
            if w / h > 2.2 {
                p.text_left = text.inside_rect(PdfRect::new_from_values(0.0, 0.0, h, w / 2.0));
                p.text_right = text.inside_rect(PdfRect::new_from_values(0.0, w / 2.0, h, w));
            }
        }
        for link in page.links().iter() {
            let Ok(r) = link.rect() else { continue };
            let rect = [r.left().value / w, 1.0 - r.top().value / h, r.right().value / w, 1.0 - r.bottom().value / h];
            let target = link
                .action()
                .and_then(|a| a.as_uri_action().and_then(|u| u.uri().ok()))
                .map(LinkTarget::Uri)
                .or_else(|| link.destination().and_then(|d| d.page_index().ok()).map(|i| LinkTarget::Page(i as usize)))
                .or_else(|| {
                    link.action().and_then(|a| {
                        a.as_local_destination_action()
                            .and_then(|l| l.destination().ok())
                            .and_then(|d| d.page_index().ok())
                            .map(|i| LinkTarget::Page(i as usize))
                    })
                });
            if let Some(target) = target {
                p.links.push(PageLink { rect, target });
            }
        }
        p.animations = find_animations(&page, w, h);
        for a in page.annotations().iter() {
            if matches!(a.annotation_type(), PdfPageAnnotationType::Movie | PdfPageAnnotationType::RichMedia) {
                if let Ok(r) = a.bounds() {
                    let pts = [r.left().value, r.bottom().value, r.right().value, r.top().value];
                    media_at.push((info.pages.len(), pts, [pts[0] / w, 1.0 - pts[3] / h, pts[2] / w, 1.0 - pts[1] / h]));
                }
            }
        }
        info.pages.push(p);
    }
    if !media_at.is_empty() {
        // Their files: beside the PDF, or packed inside it and unpacked to a temporary folder.
        let unpack = std::env::temp_dir().join("texora-media").join(path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
        let clips = std::fs::read(path).map(|b| crate::media::Objects::read(&b).clips(path, &unpack)).unwrap_or_default();
        for (page, pts, rect) in media_at {
            let near = |c: &&crate::media::MediaClip| (0..4).all(|k| (c.rect[k] - pts[k]).abs() < 1.5);
            if let Some(clip) = clips.iter().find(near) {
                info.pages[page].media.push(PageMedia { rect, clip: clip.clone() });
            }
        }
    }
    if info.pages.iter().any(|p| !p.animations.is_empty()) {
        // The frame rate and how each animation plays are in its script, often compressed.
        let texts = std::fs::read(path).map(|b| pdf_texts(&b)).unwrap_or_default();
        for p in &mut info.pages {
            for a in &mut p.animations {
                let (fps, autoplay, end) = animate_settings(&texts, &a.id);
                (a.fps, a.autoplay, a.end) = (fps, autoplay, end);
            }
        }
    }
    fn walk(b: PdfBookmark, depth: usize, out: &mut Vec<OutlineItem>) {
        let page = b
            .destination()
            .and_then(|d| d.page_index().ok())
            .or_else(|| {
                b.action().and_then(|a| {
                    a.as_local_destination_action().and_then(|l| l.destination().ok()).and_then(|d| d.page_index().ok())
                })
            })
            .map(|i| i as usize);
        if let (Some(title), Some(page)) = (b.title(), page) {
            out.push(OutlineItem { title, page, depth });
        }
        for c in b.iter_direct_children() {
            walk(c, depth + 1, out);
        }
    }
    // Sibling iterators skip the bookmark they start from, so step through by hand.
    let mut next = doc.bookmarks().root();
    while let Some(b) = next {
        next = b.next_sibling();
        walk(b, 0, &mut info.outline);
    }
    info
}

/// The form field name of a widget annotation (`/T`), where it has one.
fn field_name(a: &PdfPageAnnotation) -> Option<String> {
    if a.annotation_type() != PdfPageAnnotationType::Widget {
        return None;
    }
    a.as_form_field().and_then(|f| f.name())
}

/// `"<id>.<n>"`: frame `n` of animation `id`.
fn frame_of(name: &str) -> Option<(&str, usize)> {
    let (id, n) = name.rsplit_once('.')?;
    if id.is_empty() || n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((id, n.parse().ok()?))
}

fn find_animations(page: &PdfPage, w: f32, h: f32) -> Vec<Animation> {
    let mut found: Vec<Animation> = vec![];
    let mut controls: Vec<(String, [f32; 4])> = vec![];
    for a in page.annotations().iter() {
        let Some(name) = field_name(&a) else { continue };
        let Ok(r) = a.bounds() else { continue };
        let rect = [r.left().value / w, 1.0 - r.top().value / h, r.right().value / w, 1.0 - r.bottom().value / h];
        let Some((id, n)) = frame_of(&name) else {
            // Buttons such as "0.PlayRight" or "0.StepLeft".
            if let Some((id, word)) = name.rsplit_once('.') {
                if !word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic()) {
                    controls.push((id.to_string(), rect));
                }
            }
            continue;
        };
        match found.iter_mut().find(|x| x.id == id) {
            Some(x) => x.frames = x.frames.max(n + 1),
            None => found.push(Animation { id: id.to_string(), rect, frames: n + 1, fps: 10.0, autoplay: false, end: AnimEnd::Stop, controls: None }),
        }
    }
    for a in &mut found {
        for (_, r) in controls.iter().filter(|(id, _)| *id == a.id) {
            a.controls = Some(match a.controls {
                Some(c) => [c[0].min(r[0]), c[1].min(r[1]), c[2].max(r[2]), c[3].max(r[3])],
                None => *r,
            });
        }
    }
    found.retain(|a| a.frames > 1);
    found
}

/// The readable text of a PDF: the file itself and every compressed stream in it, unpacked.
pub fn pdf_texts(bytes: &[u8]) -> Vec<String> {
    let mut out = vec![String::from_utf8_lossy(bytes).into_owned()];
    let find = |hay: &[u8], needle: &[u8], from: usize| hay[from.min(hay.len())..].windows(needle.len()).position(|w| w == needle).map(|p| p + from);
    let mut at = 0;
    while let Some(s) = find(bytes, b"stream", at) {
        at = s + 6;
        // "endstream" contains "stream" too; skip it.
        if s >= 3 && &bytes[s - 3..s] == b"end" {
            continue;
        }
        let mut start = s + 6;
        if bytes.get(start) == Some(&b'\r') {
            start += 1;
        }
        if bytes.get(start) == Some(&b'\n') {
            start += 1;
        }
        let Some(end) = find(bytes, b"endstream", start) else { break };
        let dict_from = s.saturating_sub(600);
        let dict = String::from_utf8_lossy(&bytes[dict_from..s]);
        let dict = dict.rsplit("obj").next().unwrap_or("");
        if dict.contains("FlateDecode") && end - start < 8_000_000 {
            if let Ok(d) = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&bytes[start..end], 32_000_000) {
                out.push(String::from_utf8_lossy(&d).into_owned());
            }
        }
        at = end + 9;
    }
    out
}

/// The frame rate, autoplay and ending of animation `id`, from the script the animate package
/// writes for Acrobat: `a<id>_fps=4;`, a call to play when the page opens, and what showing the
/// frame after the last one does (go to the first: loop; turn round: palindrome).
pub fn animate_settings(texts: &[String], id: &str) -> (f32, bool, AnimEnd) {
    let mut fps = 10.0;
    let mut autoplay = false;
    let mut end = AnimEnd::Stop;
    let v = format!("a{id}_");
    for t in texts {
        if let Some(p) = t.find(&format!("{v}fps=")) {
            let rest = &t[p + v.len() + 4..];
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(f) = num.parse::<f32>() {
                if f > 0.0 {
                    fps = f;
                }
            }
        }
        if t.contains(&format!("}}if({v}playsRight){{{v}playFwd();}}else{{{v}playBwd();}}")) {
            autoplay = true;
        }
        if let Some(p) = t.find(&format!("{v}gotoNext=function(){{")) {
            let body: String = t[p..].chars().take(220).collect();
            if body.contains(&format!("if({v}playing){{{v}seekFrame(0)")) {
                end = AnimEnd::Loop;
            } else if body.contains(&format!("{v}playLeft()")) {
                end = AnimEnd::Bounce;
            }
        }
    }
    (fps, autoplay, end)
}

/// One frame of an animation, as a picture of its rectangle on the page `width` pixels wide: the
/// page drawn with only that frame's button showing.
fn render_frame(doc: &PdfDocument, page: usize, anim: usize, frame: usize, width: u32) -> Option<([usize; 2], Vec<u8>)> {
    let mut p = doc.pages().get(page as _).ok()?;
    let (w, h) = (p.width().value, p.height().value);
    let a = find_animations(&p, w, h).into_iter().nth(anim)?;
    let shown = format!("{}.{frame}", a.id);
    let display = format!("anm{}", a.id);
    let mut changed: Vec<(usize, bool)> = vec![];
    {
        let annots = p.annotations_mut();
        for i in 0..annots.len() {
            let Ok(mut x) = annots.get(i) else { continue };
            let Some(name) = field_name(&x) else { continue };
            let hide = if name == shown {
                false
            } else if name == display || frame_of(&name).is_some_and(|(id, _)| id == a.id) {
                true
            } else {
                continue;
            };
            if x.is_hidden() != hide && x.set_is_hidden(hide).is_ok() {
                changed.push((i as usize, !hide));
            }
        }
    }
    let pic = p
        .render_with_config(&PdfRenderConfig::new().set_target_width(width as i32).render_annotations(true).render_form_data(true))
        .ok()
        .map(|bmp| (bmp.width() as usize, bmp.height() as usize, bmp.as_rgba_bytes()));
    // The document stays as it was, for the next frame and every other picture.
    {
        let annots = p.annotations_mut();
        for (i, was_hidden) in changed {
            if let Ok(mut x) = annots.get(i as _) {
                let _ = x.set_is_hidden(was_hidden);
            }
        }
    }
    let (bw, bh, rgba) = pic?;
    let x0 = ((a.rect[0] * bw as f32).floor() as usize).min(bw);
    let x1 = ((a.rect[2] * bw as f32).ceil() as usize).clamp(x0, bw);
    let y0 = ((a.rect[1] * bh as f32).floor() as usize).min(bh);
    let y1 = ((a.rect[3] * bh as f32).ceil() as usize).clamp(y0, bh);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
    for row in y0..y1 {
        out.extend_from_slice(&rgba[(row * bw + x0) * 4..(row * bw + x1) * 4]);
    }
    Some(([x1 - x0, y1 - y0], out))
}

fn render(doc: &PdfDocument, page: usize, region: Region, width: u32) -> Option<([usize; 2], Vec<u8>)> {
    let p = doc.pages().get(page as _).ok()?;
    let full_w = if region == Region::Full { width } else { width * 2 } as i32;
    let bmp = p.render_with_config(&PdfRenderConfig::new().set_target_width(full_w).render_annotations(false)).ok()?;
    let (w, h) = (bmp.width() as usize, bmp.height() as usize);
    let rgba = bmp.as_rgba_bytes();
    let (x0, x1) = match region {
        Region::Full => (0, w),
        Region::Left => (0, w / 2),
        Region::Right => (w / 2, w),
    };
    if x0 == 0 && x1 == w {
        return Some(([w, h], rgba));
    }
    let mut out = Vec::with_capacity((x1 - x0) * h * 4);
    for row in 0..h {
        out.extend_from_slice(&rgba[(row * w + x0) * 4..(row * w + x1) * 4]);
    }
    Some(([x1 - x0, h], out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn animations_are_found_with_their_settings_and_frames() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/test/animation.pdf");
        let e = engine();
        e.tx.send(Cmd::Open { path, gen: 31 }).unwrap();
        let Reply::Opened { doc, .. } = wait(&e) else { panic!("open") };
        let a = &doc.pages[0].animations;
        assert_eq!(a.len(), 1, "{a:?}");
        assert_eq!((a[0].frames, a[0].fps, a[0].autoplay, a[0].end), (6, 4.0, true, AnimEnd::Loop));
        assert!(a[0].rect[0] < a[0].rect[2] && a[0].rect[1] < a[0].rect[3]);
        // Its own buttons are just under it.
        let c = a[0].controls.expect("controls");
        assert!(c[1] >= a[0].rect[3] - 0.01, "{c:?} {:?}", a[0].rect);
        let b = &doc.pages[1].animations;
        assert_eq!((b[0].frames, b[0].fps, b[0].autoplay, b[0].end), (4, 2.0, false, AnimEnd::Bounce));
        // Each frame is its own picture: the ball moves along.
        let mut pics = vec![];
        for frame in [0, 3, 5] {
            e.tx.send(Cmd::Frame(FrameReq { gen: 31, page: 0, anim: 0, frame, width: 800 })).unwrap();
            let Reply::Frame { req, size, rgba } = wait(&e) else { panic!("frame") };
            assert_eq!(req.frame, frame);
            assert!(size[0] > 50 && size[1] > 10, "{size:?}");
            // Not blank: the frame has dark green in it.
            assert!(rgba.chunks(4).any(|p| p[1] > 60 && p[0] < 60 && p[2] < 60), "frame {frame} is blank");
            pics.push(rgba);
        }
        assert!(pics[0] != pics[1] && pics[1] != pics[2]);
        // The page's own picture is unchanged afterwards (frames stay hidden there).
        e.tx.send(Cmd::Close { gen: 31 }).unwrap();
    }

    #[test]
    fn animate_scripts_are_read() {
        let t = vec!["var a2_fps;a2_fps=12.5;a2_gotoNext=function(){if(a2_seekFrame(a2_idx+1)<0){if(a2_playing){a2_seekFrame(0);a2_setFps(12.5);}else{a2_stopLast();}}};a2_seekFrame(0);}if(a2_playsRight){a2_playFwd();}else{a2_playBwd();}".to_string()];
        assert_eq!(animate_settings(&t, "2"), (12.5, true, AnimEnd::Loop));
        assert_eq!(animate_settings(&t, "7"), (10.0, false, AnimEnd::Stop));
    }

    fn sample() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sample/texora-sample.pdf")
    }

    /// The PDF library allows one engine per program, as in the app, so the tests take turns on it.
    fn engine() -> std::sync::MutexGuard<'static, PdfEngine> {
        static ENGINE: std::sync::OnceLock<std::sync::Mutex<PdfEngine>> = std::sync::OnceLock::new();
        ENGINE.get_or_init(|| std::sync::Mutex::new(spawn(|| {}))).lock().unwrap_or_else(|e| e.into_inner())
    }

    fn wait(e: &PdfEngine) -> Reply {
        e.rx.recv_timeout(Duration::from_secs(20)).expect("worker reply")
    }

    /// The real sample deck goes through the real PDF engine.
    #[test]
    fn sample_deck_opens_renders_and_searches() {
        let e = engine();
        e.tx.send(Cmd::Open { path: sample(), gen: 1 }).unwrap();
        let doc = match wait(&e) {
            Reply::Opened { doc, .. } => doc,
            Reply::Failed { msg, .. } => panic!("{msg}"),
            Reply::EngineMissing(m) => panic!("{m}"),
            _ => panic!("unexpected reply"),
        };
        assert_eq!(doc.pages.len(), 23);
        assert!(doc.pages[0].width / doc.pages[0].height > 2.2, "slide and notes share a page");
        assert!(doc.pages[0].text_left.contains("Sample Lecture"));
        assert!(doc.pages[0].text_right.contains("Thanks for being here"));
        assert!(!doc.pages[0].text_left.contains("Thanks for being here"));

        // Bookmarks, and the citation links on the history slide (frame 5, page 8).
        assert!(doc.outline.iter().any(|o| o.title == "Introduction"), "{:?}", doc.outline);
        let cites: Vec<_> = doc.pages[7].links.iter().filter(|l| l.rect[0] < 0.5 && l.rect[1] > 0.3).collect();
        assert!(cites.iter().any(|l| l.target == LinkTarget::Page(22)), "{:?}", doc.pages[7].links);

        let req = RenderReq { gen: 1, page: 0, region: Region::Left, width: 400 };
        e.tx.send(Cmd::Render(req)).unwrap();
        match wait(&e) {
            Reply::Image { size, rgba, .. } => {
                assert_eq!(size[0], 400);
                assert_eq!(rgba.len(), size[0] * size[1] * 4);
                let distinct: std::collections::HashSet<&[u8]> = rgba.chunks(4).collect();
                assert!(distinct.len() > 20, "picture should not be blank");
            }
            _ => panic!("no image"),
        }

        e.tx.send(Cmd::Search { gen: 1, query: "reproducib".into() }).unwrap();
        match wait(&e) {
            Reply::Search { hits, .. } => {
                assert!(!hits.is_empty());
                assert!(hits.iter().all(|h| !h.rects.is_empty() && h.rects[0][2] <= 1.0));
            }
            _ => panic!("no search result"),
        }
    }

    /// Two tabs at once: each document answers for itself until it is closed.
    #[test]
    fn several_documents_stay_open_side_by_side() {
        let e = engine();
        e.tx.send(Cmd::Open { path: sample(), gen: 11 }).unwrap();
        e.tx.send(Cmd::Open { path: sample(), gen: 12 }).unwrap();
        assert!(matches!(wait(&e), Reply::Opened { gen: 11, .. }));
        assert!(matches!(wait(&e), Reply::Opened { gen: 12, .. }));
        for gen in [11, 12] {
            e.tx.send(Cmd::Render(RenderReq { gen, page: 0, region: Region::Left, width: 200 })).unwrap();
            assert!(matches!(wait(&e), Reply::Image { req, .. } if req.gen == gen));
        }
        e.tx.send(Cmd::Close { gen: 11 }).unwrap();
        e.tx.send(Cmd::Search { gen: 11, query: "frontier".into() }).unwrap();
        e.tx.send(Cmd::Search { gen: 12, query: "frontier".into() }).unwrap();
        assert!(matches!(wait(&e), Reply::Search { gen: 12, .. }));
    }

    #[test]
    fn newest_open_replaces_the_previous_document() {
        let e = engine();
        e.tx.send(Cmd::Open { path: PathBuf::from("/no/such/file.pdf"), gen: 21 }).unwrap();
        match wait(&e) {
            Reply::Failed { gen: 21, .. } => {}
            Reply::EngineMissing(m) => panic!("engine failed to start: {m}"),
            _ => panic!("unexpected reply"),
        }
    }
}

fn search(doc: &PdfDocument, query: &str) -> Vec<SearchHit> {
    let mut hits = vec![];
    if query.trim().is_empty() {
        return hits;
    }
    let opts = PdfSearchOptions::new();
    for (i, page) in doc.pages().iter().enumerate() {
        let (w, h) = (page.width().value, page.height().value);
        let Ok(text) = page.text() else { continue };
        let Ok(found) = text.search(query, &opts) else { continue };
        for segs in found.iter(PdfSearchDirection::SearchForward).take(40) {
            let mut rects = vec![];
            let mut snippet = String::new();
            for s in segs.iter() {
                let r = s.bounds();
                rects.push([r.left().value / w, 1.0 - r.top().value / h, r.right().value / w, 1.0 - r.bottom().value / h]);
                snippet.push_str(&s.text());
            }
            hits.push(SearchHit { page: i, rects, snippet });
        }
    }
    hits
}
