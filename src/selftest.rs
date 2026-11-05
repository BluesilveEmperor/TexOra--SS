//! `--self-test`: proves a packaged copy really works, with no screen needed, so the same
//! check runs on every system, including build servers.
use crate::pdf::{self, Cmd, LinkTarget, Region, RenderReq, Reply};
use crate::{app, deck, latex, notes, settings, update};
use std::path::PathBuf;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

struct Report {
    lines: Vec<String>,
    failed: usize,
}

impl Report {
    fn check(&mut self, name: &str, result: Result<String, String>) {
        match result {
            Ok(d) => self.lines.push(format!("PASS  {name}: {d}")),
            Err(e) => {
                self.failed += 1;
                self.lines.push(format!("FAIL  {name}: {e}"));
            }
        }
    }
    fn note(&mut self, text: String) {
        self.lines.push(format!("INFO  {text}"));
    }
}

fn wait(rx: &std::sync::mpsc::Receiver<Reply>) -> Result<Reply, String> {
    rx.recv_timeout(Duration::from_secs(60)).map_err(|e| match e {
        RecvTimeoutError::Timeout => "no answer from the PDF engine within a minute".to_string(),
        RecvTimeoutError::Disconnected => "the PDF engine stopped".to_string(),
    })
}

fn engine_checks(r: &mut Report, path: PathBuf) {
    let e = pdf::spawn(|| {});
    let _ = e.tx.send(Cmd::Open { path, gen: 1 });
    let doc = match wait(&e.rx) {
        Ok(Reply::Opened { doc, .. }) => doc,
        Ok(Reply::EngineMissing(m)) => return r.check("PDF engine loads", Err(m)),
        Ok(Reply::Failed { msg, .. }) => return r.check("PDF opens", Err(msg)),
        Ok(_) => return r.check("PDF opens", Err("unexpected answer".to_string())),
        Err(m) => return r.check("PDF opens", Err(m)),
    };
    r.check("PDF engine loads", Ok("found next to the program".to_string()));
    let n = doc.pages.len();
    r.check("PDF opens", if n == 23 { Ok(format!("{n} pages")) } else { Err(format!("expected 23 pages, found {n}")) });

    let d = deck::build(&doc, deck::NotesMode::Auto, &deck::Nav::default(), &[]);
    let split = d.slides.first().map(|s| s.notes.is_some()).unwrap_or(false);
    r.check("Slides and notes separate", if split { Ok(format!("{} slides", d.slides.len())) } else { Err("notes half not found".to_string()) });
    r.check(
        "Text reads",
        if doc.pages[0].text_left.contains("Sample Lecture") && doc.pages[0].text_right.contains("Thanks for being here") {
            Ok("slide and notes text".to_string())
        } else {
            Err("expected words are missing".to_string())
        },
    );
    let links = doc.pages.iter().flat_map(|p| &p.links).filter(|l| matches!(l.target, LinkTarget::Page(_))).count();
    r.check("Links read", if links > 0 { Ok(format!("{links} links")) } else { Err("none found".to_string()) });
    r.check("Bookmarks read", if doc.outline.is_empty() { Err("none found".to_string()) } else { Ok(format!("{} bookmarks", doc.outline.len())) });

    let _ = e.tx.send(Cmd::Render(RenderReq { gen: 1, page: 0, region: Region::Left, width: 800 }));
    let picture = match wait(&e.rx) {
        Ok(Reply::Image { size, rgba, .. }) => {
            let colours: std::collections::HashSet<&[u8]> = rgba.chunks(4).collect();
            let green = rgba.chunks(4).filter(|p| p[1] > p[0].saturating_add(30) && p[1] > p[2].saturating_add(30)).count();
            if size[0] == 800 && colours.len() > 50 && green > 1000 {
                Ok(format!("{}x{}, {} colours, slide green present", size[0], size[1], colours.len()))
            } else {
                Err(format!("{}x{}, {} colours, {green} green pixels: looks wrong", size[0], size[1], colours.len()))
            }
        }
        Ok(_) => Err("unexpected answer".to_string()),
        Err(m) => Err(m),
    };
    r.check("Slide draws correctly", picture);

    let _ = e.tx.send(Cmd::Search { gen: 1, query: "frontier".to_string() });
    let found = match wait(&e.rx) {
        Ok(Reply::Search { hits, .. }) if !hits.is_empty() => Ok(format!("{} matches", hits.len())),
        Ok(_) => Err("no matches".to_string()),
        Err(m) => Err(m),
    };
    r.check("Search works", found);
}

/// Everything except drawing a window. Returns true when all checks pass.
pub fn run(out: Option<PathBuf>) -> bool {
    let mut r = Report { lines: vec![], failed: 0 };
    r.note(format!("TeXora {} on {} {}", update::VERSION, std::env::consts::OS, std::env::consts::ARCH));

    let dir = std::env::temp_dir().join(format!("texora-self-test-{}", std::process::id()));
    let written = std::fs::create_dir_all(&dir).and_then(|_| {
        let p = dir.join("sample.pdf");
        std::fs::write(&p, app::SAMPLE_PDF).map(|_| p)
    });
    match written {
        Ok(p) => engine_checks(&mut r, p),
        Err(e) => r.check("Temporary folder", Err(e.to_string())),
    }

    let icon = crate::icons::window_icon().map(|i| (i.width, i.height));
    r.check("App icon reads", icon.filter(|(w, h)| w == h && *w >= 512).map(|(w, h)| format!("{w}x{h}")).ok_or_else(|| "missing or not square".to_string()));

    let frames = notes::parse_tex_frames(app::SAMPLE_TEX);
    r.check("LaTeX source reads", if frames.len() == 14 { Ok("14 slides".to_string()) } else { Err(format!("found {}", frames.len())) });
    let keys = settings::default_keys().len();
    r.check("Shortcuts load", if keys > 30 { Ok(format!("{keys} actions")) } else { Err(format!("only {keys}")) });
    let reqs = latex::requirements(app::SAMPLE_TEX, latex::Engine::Auto);
    r.check("Package check reads", if reqs.len() > 5 { Ok(format!("{} requirements", reqs.len())) } else { Err("too few".to_string()) });
    r.check("Update check counts versions", if update::is_newer("99.0.0", update::VERSION) { Ok("correct".to_string()) } else { Err("wrong".to_string()) });

    // What this computer offers; useful to know, but not needed for TeXora itself to work.
    let tex = ["pdflatex", "xelatex", "lualatex", "synctex"].iter().filter(|t| latex::find_tool(t).is_some()).count();
    r.note(format!("LaTeX tools found here: {tex} of 4"));

    r.lines.push(if r.failed == 0 { "RESULT PASS".to_string() } else { format!("RESULT FAIL ({} failed)", r.failed) });
    let text = r.lines.join("\n") + "\n";
    print!("{text}");
    if let Some(p) = out {
        let _ = std::fs::write(p, &text);
    }
    r.failed == 0
}
