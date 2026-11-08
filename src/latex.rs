//! Compiles LaTeX in the background and talks to SyncTeX for source-to-slide jumps.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Default)]
pub enum Engine {
    #[default]
    Auto,
    PdfLatex,
    XeLatex,
    LuaLatex,
}

impl Engine {
    pub const ALL: [Engine; 4] = [Engine::Auto, Engine::PdfLatex, Engine::XeLatex, Engine::LuaLatex];
    pub fn name(self) -> &'static str {
        match self {
            Engine::Auto => "Auto",
            Engine::PdfLatex => "pdfLaTeX",
            Engine::XeLatex => "XeLaTeX",
            Engine::LuaLatex => "LuaLaTeX",
        }
    }
    fn program(self) -> &'static str {
        match self {
            Engine::Auto | Engine::PdfLatex => "pdflatex",
            Engine::XeLatex => "xelatex",
            Engine::LuaLatex => "lualatex",
        }
    }
}

/// `% !TEX program = xelatex` wins; otherwise packages that need a Unicode engine decide.
pub fn detect_engine(src: &str) -> Engine {
    for l in src.lines().take(12) {
        let l = l.to_lowercase();
        if l.contains("!tex") && l.contains("program") {
            if l.contains("xelatex") {
                return Engine::XeLatex;
            }
            if l.contains("lualatex") {
                return Engine::LuaLatex;
            }
            if l.contains("pdflatex") {
                return Engine::PdfLatex;
            }
        }
    }
    if src.contains("{luacode}") || src.contains("{luatextra}") {
        Engine::LuaLatex
    } else if src.contains("{fontspec}") || src.contains("{polyglossia}") || src.contains("{unicode-math}") {
        Engine::XeLatex
    } else {
        Engine::PdfLatex
    }
}

/// `% !TEX root = main.tex` lets a chapter file build the whole talk.
pub fn root_of(tex: &Path, src: &str) -> PathBuf {
    for l in src.lines().take(12) {
        if l.to_lowercase().contains("!tex root") {
            if let Some((_, r)) = l.split_once('=') {
                if let Some(d) = tex.parent() {
                    let p = d.join(r.trim());
                    if p.exists() {
                        return p;
                    }
                }
            }
        }
    }
    tex.to_path_buf()
}

/// Finds a TeX tool even when the app is launched without a shell PATH (as on macOS).
pub fn find_tool(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    for d in ["/Library/TeX/texbin", "/usr/texbin", "/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin", "/usr/bin"] {
        dirs.push(PathBuf::from(d));
    }
    if let Some(h) = dirs::home_dir() {
        for d in [".local/bin", ".npm-global/bin", ".bun/bin", ".volta/bin", ".cargo/bin"] {
            dirs.push(h.join(d));
        }
        dirs.push(h.join("bin"));
        dirs.push(h.join("AppData/Local/Programs/MiKTeX/miktex/bin/x64"));
    }
    dirs.push(PathBuf::from("C:/Program Files/MiKTeX/miktex/bin/x64"));
    dirs.into_iter().map(|d| d.join(&exe)).find(|p| p.is_file())
}

#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub file: String,
    pub line: Option<usize>,
    pub message: String,
    pub is_error: bool,
}

pub struct CompileResult {
    /// The tab that asked for this build.
    pub tab: u64,
    pub ok: bool,
    pub pdf: PathBuf,
    pub problems: Vec<Problem>,
    pub log: String,
    pub seconds: f32,
    pub engine: Engine,
    pub note: String,
}

/// Picks out errors and the warnings that matter from a TeX log.
pub fn parse_log(log: &str) -> Vec<Problem> {
    let mut out: Vec<Problem> = vec![];
    let lines: Vec<&str> = log.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        // -file-line-error form: ./talk.tex:12: Undefined control sequence.
        if !l.starts_with(' ') {
            let mut parts = l.splitn(3, ':');
            if let (Some(f), Some(n), Some(m)) = (parts.next(), parts.next(), parts.next()) {
                if f.ends_with(".tex") || f.ends_with(".sty") || f.ends_with(".cls") {
                    if let Ok(line) = n.parse::<usize>() {
                        let mut message = m.trim().to_string();
                        if let Some(next) = lines.get(i + 1) {
                            if next.starts_with("l.") {
                                message = format!("{message}  ({})", next.trim());
                            }
                        }
                        out.push(Problem { file: f.trim_start_matches("./").to_string(), line: Some(line), message, is_error: true });
                        continue;
                    }
                }
            }
        }
        if let Some(m) = l.strip_prefix("! ") {
            out.push(Problem { file: String::new(), line: None, message: m.to_string(), is_error: true });
        } else if l.contains("LaTeX Warning:") && (l.contains("undefined") || l.contains("multiply")) {
            let line = l.rsplit("input line ").next().and_then(|t| t.trim_end_matches('.').trim().parse().ok());
            let message = l.split("LaTeX Warning:").nth(1).unwrap_or(l).trim().to_string();
            out.push(Problem { file: String::new(), line, message, is_error: false });
        }
    }
    out.dedup();
    // This line only repeats that the errors above stopped the build.
    out.retain(|p| !p.message.contains("Fatal error occurred"));
    out
}

fn run_with_timeout(mut cmd: Command, limit: Duration) -> Result<bool, String> {
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    let start = Instant::now();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(s) => return Ok(s.success()),
            None if start.elapsed() > limit => {
                let _ = child.kill();
                return Err(format!("stopped after {} seconds", limit.as_secs()));
            }
            None => std::thread::sleep(Duration::from_millis(40)),
        }
    }
}

pub fn compile(tex: &Path, engine: Engine) -> CompileResult {
    let start = Instant::now();
    let src = std::fs::read_to_string(tex).unwrap_or_default();
    let root = root_of(tex, &src);
    let root_src = if root == tex { src } else { std::fs::read_to_string(&root).unwrap_or_default() };
    let engine = if engine == Engine::Auto { detect_engine(&root_src) } else { engine };
    let dir = root.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let pdf = root.with_extension("pdf");
    let logf = root.with_extension("log");
    let Some(prog) = find_tool(engine.program()) else {
        return CompileResult {
            tab: 0,
            ok: false,
            pdf,
            problems: vec![],
            log: String::new(),
            seconds: 0.0,
            engine,
            note: format!("{} is not installed, so .tex files cannot be compiled. PDFs still work. {}", engine.name(), crate::syscheck::tex_install_hint(crate::cli::Os::current())),
        };
    };
    let file = root.file_name().unwrap().to_owned();
    let stem = root.file_stem().unwrap().to_owned();
    // Videos first: their posters must be there when LaTeX looks for them. TeXora's own packages
    // (texoravideo) are found through TEXINPUTS; nothing is copied into the presentation's folder.
    let videos = crate::video_prep::prepare(&root);
    let texinputs = crate::video_prep::tex_dir().ok().map(|d| crate::video_prep::texinputs(&d));
    let mut pass = 0;
    let mut note = String::new();
    let (ok, log) = loop {
        pass += 1;
        let mut c = Command::new(&prog);
        c.current_dir(&dir).args(["-interaction=nonstopmode", "-file-line-error", "-synctex=1"]).arg(&file);
        if let Some(t) = &texinputs {
            c.env("TEXINPUTS", t);
        }
        let run = run_with_timeout(c, Duration::from_secs(180));
        let log = String::from_utf8_lossy(&std::fs::read(&logf).unwrap_or_default()).to_string();
        let ran = match run {
            Ok(b) => b,
            Err(e) => {
                note = e;
                false
            }
        };
        if !ran {
            break (false, log);
        }
        if pass == 1 {
            // Bibliography step, only when the document asks for one.
            let bcf = root.with_extension("bcf");
            let aux = std::fs::read_to_string(root.with_extension("aux")).unwrap_or_default();
            let tool = if bcf.exists() {
                "biber"
            } else if aux.contains("\\bibdata") {
                "bibtex"
            } else {
                ""
            };
            if !tool.is_empty() {
                if let Some(t) = find_tool(tool) {
                    let mut c = Command::new(t);
                    c.current_dir(&dir).arg(&stem);
                    let _ = run_with_timeout(c, Duration::from_secs(60));
                }
                continue;
            }
        }
        let wants_rerun = log.contains("Rerun to get") || log.contains("may have changed. Rerun");
        if pass < 4 && wants_rerun {
            continue;
        }
        break (true, log);
    };
    let mut problems = parse_log(&log);
    let has_error = problems.iter().any(|p| p.is_error);
    // A clip that cannot be played does not stop the slides; it is listed, at its \includevideo.
    for (f, line, message) in videos.problems {
        let file = f.strip_prefix(&dir).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| f.to_string_lossy().to_string());
        problems.push(Problem { file, line: Some(line), message, is_error: false });
    }
    CompileResult { tab: 0, ok: ok && !has_error && pdf.exists(), pdf, problems, log, seconds: start.elapsed().as_secs_f32(), engine, note }
}

pub fn compile_async(tab: u64, tex: PathBuf, engine: Engine, tx: Sender<CompileResult>, wake: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let _ = tx.send(CompileResult { tab, ..compile(&tex, engine) });
        wake();
    });
}

#[derive(Clone, Debug, PartialEq)]
pub struct SyncTarget {
    pub page: usize,
    pub x: f32,
    pub y: f32,
}

fn field<'a>(out: &'a str, key: &str) -> Option<&'a str> {
    out.lines().find_map(|l| l.strip_prefix(key))
}

/// Source line → every place on every page that SyncTeX links to it. A line usually has many: one per
/// box it made, on each overlay step, and often a few on the page before (where TeX was when the
/// frame started). Choosing among them is up to the caller; see `app::pick_sync`.
pub fn forward_sync(tex: &Path, line: usize, pdf: &Path) -> Vec<SyncTarget> {
    let Some(tool) = find_tool("synctex") else { return vec![] };
    let arg = format!("{}:1:{}", line, tex.display());
    match Command::new(tool).arg("view").arg("-i").arg(arg).arg("-o").arg(pdf).output() {
        Ok(out) => parse_sync_view(&String::from_utf8_lossy(&out.stdout)),
        Err(_) => vec![],
    }
}

/// Reads `synctex view` output: each record starts with `Page:` and has its own `x:` and `y:`.
pub fn parse_sync_view(s: &str) -> Vec<SyncTarget> {
    let mut out: Vec<SyncTarget> = vec![];
    let mut have_x = false;
    for l in s.lines() {
        if let Some(p) = l.strip_prefix("Page:") {
            if let Some(page) = p.trim().parse::<usize>().ok().and_then(|p| p.checked_sub(1)) {
                out.push(SyncTarget { page, x: 0.0, y: 0.0 });
                have_x = false;
            }
        } else if let (Some(v), Some(last)) = (l.strip_prefix("x:"), out.last_mut()) {
            last.x = v.trim().parse().unwrap_or(0.0);
            have_x = true;
        } else if let (Some(v), Some(last)) = (l.strip_prefix("y:"), out.last_mut()) {
            if have_x {
                last.y = v.trim().parse().unwrap_or(0.0);
            }
        }
    }
    out
}

/// Place on a page → source file and line.
pub fn inverse_sync(page: usize, x: f32, y: f32, pdf: &Path) -> Option<(PathBuf, usize)> {
    let tool = find_tool("synctex")?;
    let arg = format!("{}:{}:{}:{}", page + 1, x, y, pdf.display());
    let out = Command::new(tool).arg("edit").arg("-o").arg(arg).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let line: usize = field(&s, "Line:")?.trim().parse().ok()?;
    let input = field(&s, "Input:")?.trim().to_string();
    let p = PathBuf::from(&input);
    let p = if p.is_absolute() { p } else { pdf.parent()?.join(p) };
    Some((p, line.max(1)))
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    #[test]
    fn every_synctex_record_is_read() {
        let out = "SyncTeX result begin\nOutput:a.pdf\nPage:2\nx:10.5\ny:20.0\nh:1\nv:2\nW:3\nH:4\nbefore:\noffset:0\nmiddle:\nafter:\nOutput:a.pdf\nPage:4\nx:30\ny:40\nSyncTeX result end\n";
        let r = parse_sync_view(out);
        assert_eq!(r, vec![SyncTarget { page: 1, x: 10.5, y: 20.0 }, SyncTarget { page: 3, x: 30.0, y: 40.0 }]);
        assert!(parse_sync_view("").is_empty());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReqKind {
    Class,
    Package,
    Theme,
    TikzLibrary,
    File,
    Tool,
}

impl ReqKind {
    pub fn label(self) -> &'static str {
        match self {
            ReqKind::Class => "Document class",
            ReqKind::Package => "Package",
            ReqKind::Theme => "Beamer theme",
            ReqKind::TikzLibrary => "TikZ library",
            ReqKind::File => "File",
            ReqKind::Tool => "Program",
        }
    }
}

/// One thing the document needs in order to compile.
#[derive(Clone, Debug, PartialEq)]
pub struct Requirement {
    pub kind: ReqKind,
    pub name: String,
    /// Names to look for, any of which satisfies this requirement.
    pub candidates: Vec<String>,
    pub found: bool,
    pub hint: String,
}

fn commands<'a>(src: &'a str, name: &str) -> Vec<(&'a str, usize)> {
    let mut out = vec![];
    let mut pos = 0;
    while let Some(p) = src[pos..].find(name) {
        let at = pos + p + name.len();
        pos = at;
        // `\\use` must not match `\\usepackagefoo`, so the next character has to end the name.
        if src[at..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            continue;
        }
        let mut i = at;
        let b = src.as_bytes();
        while i < b.len() {
            match b[i] {
                b' ' | b'\n' | b'\t' => i += 1,
                b'[' => match src[i..].find(']') {
                    Some(e) => i += e + 1,
                    None => break,
                },
                b'{' => {
                    if let Some(e) = src[i..].find('}') {
                        out.push((&src[i + 1..i + e], i));
                    }
                    break;
                }
                _ => break,
            }
        }
    }
    out
}

fn uncomment(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for l in src.lines() {
        let b = l.as_bytes();
        let end = (0..b.len()).find(|&i| b[i] == b'%' && (i == 0 || b[i - 1] != b'\\')).unwrap_or(b.len());
        out.push_str(&l[..end]);
        out.push('\n');
    }
    out
}

/// Everything the source asks for. Nothing is looked up yet.
pub fn requirements(src: &str, engine: Engine) -> Vec<Requirement> {
    let clean = uncomment(src);
    let mut out: Vec<Requirement> = vec![];
    let mut add = |kind, name: &str, candidates: Vec<String>, hint: String| {
        let name = name.trim();
        if !name.is_empty() && !name.contains('#') && !out.iter().any(|r: &Requirement| r.kind == kind && r.name == name) {
            out.push(Requirement { kind, name: name.to_string(), candidates, found: false, hint });
        }
    };
    for (c, _) in commands(&clean, "\\documentclass") {
        add(ReqKind::Class, c, vec![format!("{c}.cls")], format!("tlmgr install {c}"));
    }
    for cmd in ["\\usepackage", "\\RequirePackage"] {
        for (list, _) in commands(&clean, cmd) {
            for p in list.split(',').map(str::trim) {
                add(ReqKind::Package, p, vec![format!("{p}.sty")], format!("tlmgr install {p}"));
            }
        }
    }
    for (cmd, prefix) in [
        ("\\usetheme", "beamertheme"),
        ("\\usecolortheme", "beamercolortheme"),
        ("\\usefonttheme", "beamerfonttheme"),
        ("\\useinnertheme", "beamerinnertheme"),
        ("\\useoutertheme", "beameroutertheme"),
    ] {
        for (list, _) in commands(&clean, cmd) {
            for t in list.split(',').map(str::trim) {
                add(ReqKind::Theme, t, vec![format!("{prefix}{t}.sty")], "Check the theme name, or install the package that provides it.".to_string());
            }
        }
    }
    for cmd in ["\\usetikzlibrary", "\\usepgflibrary", "\\usepgfplotslibrary"] {
        for (list, _) in commands(&clean, cmd) {
            for l in list.split(',').map(str::trim) {
                let c = vec![
                    format!("tikzlibrary{l}.code.tex"),
                    format!("pgflibrary{l}.code.tex"),
                    format!("pgfplotslibrary{l}.code.tex"),
                ];
                add(ReqKind::TikzLibrary, l, c, "Update pgf, or check the library name.".to_string());
            }
        }
    }
    let exts = |base: &str, list: &[&str]| -> Vec<String> {
        let mut v = vec![base.to_string()];
        v.extend(list.iter().map(|e| format!("{base}.{e}")));
        v
    };
    for (f, _) in commands(&clean, "\\includegraphics") {
        add(ReqKind::File, f, exts(f, &["pdf", "png", "jpg", "jpeg", "eps"]), "Check the file name and folder.".to_string());
    }
    for u in crate::video_prep::uses_in(&clean, std::path::Path::new("")) {
        add(ReqKind::File, &u.path, vec![u.path.clone()], "Put the video in the .tex file's folder, or in a folder inside it written in the name ({videos/clip.mp4}).".to_string());
    }
    for cmd in ["\\input", "\\include"] {
        for (f, _) in commands(&clean, cmd) {
            add(ReqKind::File, f, exts(f, &["tex"]), "Check the file name and folder.".to_string());
        }
    }
    for (list, _) in commands(&clean, "\\bibliography") {
        for f in list.split(',').map(str::trim) {
            add(ReqKind::File, f, exts(f, &["bib"]), "Check the bibliography file name.".to_string());
        }
    }
    for (f, _) in commands(&clean, "\\addbibresource") {
        add(ReqKind::File, f, vec![f.to_string()], "Check the bibliography file name.".to_string());
    }
    let engine = if engine == Engine::Auto { detect_engine(src) } else { engine };
    add(ReqKind::Tool, engine.program(), vec![engine.program().to_string()], crate::syscheck::tex_install_hint(crate::cli::Os::current()).to_string());
    if clean.contains("\\addbibresource") {
        add(ReqKind::Tool, "biber", vec!["biber".to_string()], "tlmgr install biber".to_string());
    } else if clean.contains("\\bibliography{") {
        add(ReqKind::Tool, "bibtex", vec!["bibtex".to_string()], crate::syscheck::tex_install_hint(crate::cli::Os::current()).to_string());
    }
    add(ReqKind::Tool, "synctex", vec!["synctex".to_string()], "Needed to jump between source and slides.".to_string());
    out
}

/// Looks each requirement up the way LaTeX does when TeXora compiles: the presentation's own folder
/// first (a `.sty`, `.cls` or theme file beside the `.tex`), then TeXora's packages, then the TeX
/// installation (kpsewhich, run from the presentation's folder with the same TEXINPUTS as a compile).
pub fn check_requirements(src: &str, engine: Engine, dir: &Path) -> (Vec<Requirement>, Option<String>) {
    let mut reqs = requirements(src, engine);
    let kpse = find_tool("kpsewhich");
    let texinputs = crate::video_prep::tex_dir().ok().map(|d| crate::video_prep::texinputs(&d));
    let in_folder = |r: &Requirement| r.candidates.iter().any(|c| dir.join(c).is_file());
    for r in reqs.iter_mut() {
        r.found = match r.kind {
            ReqKind::Tool => find_tool(&r.candidates[0]).is_some(),
            ReqKind::File => in_folder(r),
            // Built into TeXora: found through TEXINPUTS when TeXora compiles.
            ReqKind::Package if r.name == "texoravideo" => true,
            // A package, class, theme or TikZ library of your own, beside the presentation.
            _ if in_folder(r) => true,
            _ => match &kpse {
                Some(k) => {
                    let mut c = Command::new(k);
                    c.args(&r.candidates).current_dir(dir);
                    if let Some(t) = &texinputs {
                        c.env("TEXINPUTS", t);
                    }
                    c.output().map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty()).unwrap_or(false)
                }
                None => false,
            },
        };
        // Found means no install hint; one of your own files never needs tlmgr.
        if r.found && !matches!(r.kind, ReqKind::Tool) {
            r.hint.clear();
        }
    }
    // A Unicode font package cannot work under pdfLaTeX, whatever is installed.
    let chosen = if engine == Engine::Auto { detect_engine(src) } else { engine };
    let clean = uncomment(src);
    let warn = (chosen == Engine::PdfLatex && (clean.contains("{fontspec}") || clean.contains("{polyglossia}")))
        .then(|| "fontspec and polyglossia need XeLaTeX or LuaLaTeX. Change the engine to compile.".to_string())
        .or_else(|| kpse.is_none().then(|| "TeX is not installed, so packages could not be checked.".to_string()));
    (reqs, warn)
}

pub fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_from_magic_comment_and_packages() {
        assert_eq!(detect_engine("% !TEX program = xelatex\n\\documentclass{beamer}"), Engine::XeLatex);
        assert_eq!(detect_engine("% !TeX TS-program = lualatex\n"), Engine::LuaLatex);
        assert_eq!(detect_engine("\\usepackage{fontspec}"), Engine::XeLatex);
        assert_eq!(detect_engine("\\usepackage{amsmath}"), Engine::PdfLatex);
    }

    #[test]
    fn log_errors_keep_file_and_line() {
        let log = "./talk.tex:12: Undefined control sequence.\nl.12 \\foo\n\nLaTeX Warning: Citation `x' on page 1 undefined on input line 30.\n! Emergency stop.\n";
        let p = parse_log(log);
        assert_eq!(p[0].file, "talk.tex");
        assert_eq!(p[0].line, Some(12));
        assert!(p[0].message.contains("l.12"));
        assert!(!p[1].is_error);
        assert_eq!(p[1].line, Some(30));
        assert!(p[2].is_error);
    }

    fn sample_in(dir: &str) -> PathBuf {
        let d = std::env::temp_dir().join(dir);
        std::fs::create_dir_all(&d).unwrap();
        let t = d.join("t.tex");
        std::fs::write(&t, "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}{One}\nHello\n\\end{frame}\n\\begin{frame}{Two}\nWorld\n\\end{frame}\n\\end{document}\n").unwrap();
        t
    }

    /// Real compile with each engine; skipped quietly on machines without TeX.
    #[test]
    fn all_three_engines_build_a_beamer_file() {
        for (e, dir) in [(Engine::PdfLatex, "txp-pdf"), (Engine::XeLatex, "txp-xe"), (Engine::LuaLatex, "txp-lua")] {
            if find_tool(e.program()).is_none() {
                continue;
            }
            let t = sample_in(dir);
            let r = compile(&t, e);
            assert!(r.ok, "{:?}: {:?} {}", e, r.problems, r.note);
            assert!(r.pdf.exists());
        }
    }

    #[test]
    fn source_and_slide_sync_both_ways() {
        if find_tool("pdflatex").is_none() || find_tool("synctex").is_none() {
            return;
        }
        let t = sample_in("txp-sync");
        let r = compile(&t, Engine::PdfLatex);
        assert!(r.ok);
        let all = forward_sync(&t, 7, &r.pdf);
        let hit = all.iter().find(|h| h.page == 1).cloned().expect("forward");
        let (file, line) = inverse_sync(hit.page, hit.x, hit.y, &r.pdf).expect("inverse");
        assert!(file.ends_with("t.tex"));
        assert!((6..=8).contains(&line), "line {line}");
    }

    #[test]
    fn requirements_read_from_source() {
        let src = "% \\usepackage{ignored}\n\\documentclass[11pt]{beamer}\n\\usetheme{Madrid}\n\\usepackage[utf8]{inputenc}\n\\usepackage{amsmath, tikz}\n\\usetikzlibrary{arrows.meta,positioning}\n\\includegraphics[width=2cm]{figs/a}\n\\bibliography{refs}\n";
        let r = requirements(src, Engine::PdfLatex);
        let names: Vec<&str> = r.iter().map(|x| x.name.as_str()).collect();
        for n in ["beamer", "Madrid", "inputenc", "amsmath", "tikz", "arrows.meta", "positioning", "figs/a", "refs", "pdflatex", "bibtex"] {
            assert!(names.contains(&n), "{n} in {names:?}");
        }
        assert!(!names.contains(&"ignored"));
    }

    #[test]
    fn check_finds_installed_and_missing_things() {
        if find_tool("kpsewhich").is_none() {
            return;
        }
        let src = "\\documentclass{beamer}\n\\usetheme{Madrid}\n\\usepackage{amsmath}\n\\usepackage{nosuchpackagehere}\n\\usetheme{NoSuchTheme}\n\\usetikzlibrary{arrows.meta}\n\\usetikzlibrary{nosuchlib}\n\\includegraphics{nofile}\n";
        let (r, warn) = check_requirements(src, Engine::PdfLatex, &std::env::temp_dir());
        assert!(warn.is_none());
        let found = |n: &str| r.iter().find(|x| x.name == n).unwrap().found;
        assert!(found("beamer") && found("Madrid") && found("amsmath") && found("arrows.meta"));
        assert!(!found("nosuchpackagehere") && !found("NoSuchTheme") && !found("nosuchlib") && !found("nofile"));
    }

    /// Issue #3: a package, class or theme file beside the presentation is found, with no tlmgr hint.
    #[test]
    fn files_beside_the_presentation_count_as_found() {
        let dir = std::env::temp_dir().join(format!("texora-local-sty-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("styles")).unwrap();
        for f in ["mytesting.sty", "myslides.cls", "beamerthemeCampus.sty", "styles/house.sty", "tikzlibrarymyshapes.code.tex"] {
            std::fs::write(dir.join(f), "% local\n").unwrap();
        }
        let src = "\\documentclass{myslides}\n\\usepackage{mytesting}\n\\usepackage{styles/house}\n\\usetheme{Campus}\n\\usetikzlibrary{myshapes}\n\\usepackage{texoravideo}\n\\usepackage{nosuchpackagehere}\n";
        let (r, _) = check_requirements(src, Engine::PdfLatex, &dir);
        let get = |n: &str| r.iter().find(|x| x.name == n).unwrap().clone();
        for n in ["myslides", "mytesting", "styles/house", "Campus", "myshapes", "texoravideo"] {
            let x = get(n);
            assert!(x.found, "{n} is beside the presentation (or built in)");
            assert!(!x.hint.contains("tlmgr"), "no install hint for {n}: {}", x.hint);
        }
        let missing = get("nosuchpackagehere");
        assert!(!missing.found && missing.hint.contains("tlmgr"), "a package found nowhere is still missing");
    }

    #[test]
    fn fontspec_under_pdflatex_is_flagged() {
        let (_, warn) = check_requirements("\\usepackage{fontspec}", Engine::PdfLatex, &std::env::temp_dir());
        assert!(warn.unwrap().contains("XeLaTeX"));
    }

    #[test]
    fn sample_lecture_compiles() {
        if find_tool("pdflatex").is_none() {
            return;
        }
        let d = std::env::temp_dir().join("txp-sample-compile");
        std::fs::create_dir_all(&d).unwrap();
        let t = d.join("new.tex");
        std::fs::write(&t, include_str!("../assets/sample/texora-sample.tex")).unwrap();
        let r = compile(&t, Engine::Auto);
        assert!(r.ok, "{:?} {}", r.problems, r.note);
    }

    #[test]
    fn broken_source_reports_the_line() {
        if find_tool("pdflatex").is_none() {
            return;
        }
        let d = std::env::temp_dir().join("txp-bad");
        std::fs::create_dir_all(&d).unwrap();
        let t = d.join("bad.tex");
        std::fs::write(&t, "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}\n\\nosuchcommand\n\\end{frame}\n\\end{document}\n").unwrap();
        let r = compile(&t, Engine::PdfLatex);
        assert!(!r.ok);
        assert!(r.problems.iter().any(|p| p.is_error && p.line.is_some()), "{:?}", r.problems);
    }
}
