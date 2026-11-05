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

#[derive(Clone, Debug, Default)]
pub struct PageInfo {
    pub width: f32,
    pub height: f32,
    pub label: Option<String>,
    pub text_left: String,
    pub text_right: String,
    pub text_full: String,
    pub links: Vec<PageLink>,
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

pub enum Cmd {
    Open { path: PathBuf, gen: u64 },
    Close { gen: u64 },
    Render(RenderReq),
    Search { gen: u64, query: String },
}

pub enum Reply {
    Opened { gen: u64, doc: DocInfo },
    Image { req: RenderReq, size: [usize; 2], rgba: Vec<u8> },
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
        info.pages.push(p);
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
