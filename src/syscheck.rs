//! ? > System check: what this computer offers TeXora, what each part is for, and how to get what is
//! missing. Nothing here is needed to start TeXora; packages carry the program, its PDF engine, fonts,
//! icons and short guide.
use crate::cli::Os;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    /// Present and working.
    Ready,
    /// Missing, but only one feature needs it.
    Optional,
    /// Missing and TeXora cannot work properly without it.
    Problem,
    /// Facts, not something to check.
    Info,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub name: &'static str,
    pub status: Status,
    /// What was found, or what is missing.
    pub found: String,
    /// What it is used for.
    pub used_for: &'static str,
    /// How to get it, when it is missing.
    pub how: Option<String>,
}

/// What the app knows about this computer, gathered by the caller.
pub struct Facts {
    pub os: Os,
    pub pdf_engine_error: Option<String>,
    /// LaTeX programs that were found, by name.
    pub tex_tools: Vec<(&'static str, bool)>,
}

/// The PDF engine's file name on each system.
pub fn engine_file(os: Os) -> &'static str {
    match os {
        Os::Mac => "libpdfium.dylib",
        Os::Linux => "libpdfium.so",
        Os::Windows => "pdfium.dll",
    }
}

/// A plain sentence for a PDF engine that did not start, instead of the system's raw error.
pub fn engine_problem(os: Os, raw: &str) -> String {
    let r = raw.to_lowercase();
    if r.contains("no such file") || r.contains("not found") || r.contains("loadlibrary") || r.contains("cannot open") {
        format!("Its file, {}, is missing from beside the TeXora program", engine_file(os))
    } else {
        let first = raw.lines().next().unwrap_or("").trim();
        format!("It could not start: {}", first.chars().take(120).collect::<String>())
    }
}

/// How to install LaTeX on this kind of system.
pub fn tex_install_hint(os: Os) -> &'static str {
    match os {
        Os::Mac => "Install MacTeX from https://tug.org/mactex (or BasicTeX for a smaller download).",
        Os::Windows => "Install MiKTeX from https://miktex.org, or TeX Live from https://tug.org/texlive.",
        Os::Linux => "Install TeX Live from your package manager, for example: sudo apt install texlive-latex-extra (Ubuntu, Debian) or sudo dnf install texlive-scheme-medium (Fedora).",
    }
}

pub fn checks(f: &Facts) -> Vec<Check> {
    let mut out = vec![];
    out.push(Check {
        name: "TeXora and its PDF engine",
        status: if f.pdf_engine_error.is_some() { Status::Problem } else { Status::Ready },
        found: match &f.pdf_engine_error {
            None => "Included in TeXora and working".to_string(),
            Some(e) => engine_problem(f.os, e),
        },
        used_for: "Showing every presentation",
        how: f.pdf_engine_error.as_ref().map(|_| {
            format!("Download TeXora again and keep its files together: {} must stay beside the TeXora program.", engine_file(f.os))
        }),
    });
    let engines: Vec<&str> = f.tex_tools.iter().filter(|(n, ok)| *ok && *n != "synctex").map(|(n, _)| *n).collect();
    out.push(Check {
        name: "LaTeX",
        status: if engines.is_empty() { Status::Optional } else { Status::Ready },
        found: if engines.is_empty() { "Not found".to_string() } else { format!("Found: {}", engines.join(", ")) },
        used_for: "Presenting .tex files: TeXora compiles them. PDF presentations do not need it.",
        how: engines.is_empty().then(|| tex_install_hint(f.os).to_string()),
    });
    out.push(Check {
        name: "Web browser",
        status: Status::Info,
        found: "Your default browser".to_string(),
        used_for: "The website and update downloads",
        how: None,
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(os: Os) -> Facts {
        Facts { os, pdf_engine_error: None, tex_tools: vec![("pdflatex", false), ("xelatex", false), ("lualatex", false), ("synctex", false)] }
    }

    #[test]
    fn a_bare_computer_still_runs_texora() {
        let c = checks(&facts(Os::Windows));
        assert!(c.iter().all(|c| c.status != Status::Problem), "nothing missing stops TeXora itself");
        let tex = c.iter().find(|c| c.name == "LaTeX").unwrap();
        assert_eq!(tex.status, Status::Optional);
        assert!(tex.how.as_ref().unwrap().contains("miktex.org"));
        assert!(tex.used_for.contains("PDF presentations do not need it"));
    }

    #[test]
    fn hints_fit_each_system() {
        assert!(tex_install_hint(Os::Mac).contains("MacTeX"));
        assert!(tex_install_hint(Os::Linux).contains("apt install"));
        assert!(tex_install_hint(Os::Windows).contains("MiKTeX"));
    }

    #[test]
    fn found_tools_are_named_and_engine_trouble_is_a_problem() {
        let mut f = facts(Os::Mac);
        f.tex_tools = vec![("pdflatex", true), ("xelatex", true), ("lualatex", false), ("synctex", true)];
        f.pdf_engine_error = Some("missing".to_string());
        let c = checks(&f);
        assert_eq!(c[0].status, Status::Problem);
        let raw = "PDF engine not found: LoadLibraryError(\n DlOpen { source: \"dlopen(libpdfium.dylib): (no such file)\" })";
        assert_eq!(engine_problem(Os::Mac, raw), "Its file, libpdfium.dylib, is missing from beside the TeXora program");
        let tex = c.iter().find(|c| c.name == "LaTeX").unwrap();
        assert_eq!(tex.found, "Found: pdflatex, xelatex");
        assert!(tex.how.is_none());
    }
}
