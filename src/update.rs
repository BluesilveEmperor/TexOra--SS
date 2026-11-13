//! Looks for a newer TeXora among the releases published on GitHub.
use serde_json::Value;
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The GitHub project TeXora is published in, as "owner/name". Until it has a published release,
/// GitHub answers "not found" and TeXora counts as up to date.
const GITHUB_REPO: &str = "BluesilveEmperor/TexOra--SS";

pub const WEBSITE: &str = "https://github.com/BluesilveEmperor/TexOra--SS";

pub const DEVELOPER: &str = "Khalid Alkhaldi";

/// A translated text with its {0}, {1}… placeholders filled in, in the order they are given.
///
/// `format!` wants its format string to be a literal, but a translated one is only known when the
/// program runs, so the placeholders are put in by hand: the text first, the values after it.
macro_rules! tfill {
    ($text:expr $(, $arg:expr)* $(,)?) => {{
        let mut text = String::from($text);
        let args: Vec<String> = vec![$(format!("{}", $arg)),*];
        for (i, arg) in args.iter().enumerate() {
            let key = format!("{{{i}}}");
            text = text.replace(key.as_str(), arg);
        }
        text
    }};
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    UpToDate,
    /// `download` is the package for this computer, as (file name, address), when the release has one.
    Available { version: String, url: String, notes: String, download: Option<(String, String)> },
    Failed(String),
}

pub fn update_url() -> String {
    if let Ok(u) = std::env::var("TEXORA_UPDATE_URL") {
        return u;
    }
    if GITHUB_REPO.is_empty() {
        String::new()
    } else {
        format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest")
    }
}

fn parts(v: &str) -> Vec<u64> {
    v.trim().trim_start_matches(['v', 'V']).split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// True when `candidate` is a later version than `current`.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let (a, b) = (parts(candidate), parts(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// Reads GitHub's description of the latest release.
pub fn interpret(body: &str, current: &str) -> Outcome {
    let Ok(v) = serde_json::from_str::<Value>(body) else {
        return Outcome::Failed(crate::t!("The update information could not be read.").to_string());
    };
    let Some(tag) = v["tag_name"].as_str() else {
        return Outcome::Failed(crate::t!("The update information has no version number.").to_string());
    };
    if is_newer(tag, current) {
        let files: Vec<(String, String)> = v["assets"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| Some((x["name"].as_str()?.to_string(), x["browser_download_url"].as_str()?.to_string()))).collect())
            .unwrap_or_default();
        Outcome::Available {
            version: tag.trim_start_matches(['v', 'V']).to_string(),
            url: v["html_url"].as_str().unwrap_or("").to_string(),
            notes: v["body"].as_str().unwrap_or("").trim().to_string(),
            download: pick_asset(&files, std::env::consts::OS, std::env::consts::ARCH, std::path::Path::new("/usr/bin/dpkg").exists()),
        }
    } else {
        Outcome::UpToDate
    }
}

/// How often TeXora looks again while it stays open, so a release made during a long session is noticed.
pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 3600);

/// The release file that suits this computer: the installer first, the portable package otherwise.
/// `os` and `arch` are Rust's names (macos, windows, linux; aarch64, x86_64).
pub fn pick_asset(files: &[(String, String)], os: &str, arch: &str, has_dpkg: bool) -> Option<(String, String)> {
    let a = if matches!(arch, "aarch64" | "arm64") { "arm64" } else { "x64" };
    let wanted: Vec<String> = match os {
        "macos" => vec![format!("-mac-{a}.dmg"), format!("-mac-{a}.zip")],
        "windows" => vec!["-windows-x64-setup.exe".into(), "-windows-x64.zip".into()],
        _ => {
            let mut w = vec![];
            if has_dpkg {
                w.push(if a == "arm64" { "_arm64.deb".to_string() } else { "_amd64.deb".to_string() });
            }
            w.push(format!("-linux-{a}.tar.gz"));
            w
        }
    };
    wanted.iter().find_map(|end| files.iter().find(|(n, _)| n.ends_with(end.as_str())).cloned())
}

fn from_status(status: u16, body: impl FnOnce() -> Option<String>) -> Outcome {
    match status {
        200 => match body() {
            Some(t) => interpret(&t, VERSION),
            None => Outcome::Failed(crate::t!("The update information could not be read.").to_string()),
        },
        // GitHub says this when the project has no published release: nothing newer exists.
        404 => Outcome::UpToDate,
        403 | 429 => Outcome::Failed(crate::t!("GitHub is busy right now. Try again later.").to_string()),
        s => Outcome::Failed(tfill!(crate::t!("GitHub answered with an error ({0})."), s)),
    }
}

pub fn check(url: &str) -> Outcome {
    if url.trim().is_empty() {
        return Outcome::UpToDate;
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).http_status_as_error(false).build().into();
    let sent = agent
        .get(url)
        .header("user-agent", format!("TeXora/{VERSION}"))
        .header("accept", "application/vnd.github+json")
        .call();
    match sent {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let mut body = resp.into_body();
            from_status(status, move || body.read_to_string().ok())
        }
        Err(_) => Outcome::Failed(crate::t!("Could not reach GitHub. Check your internet connection.").to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_numbers() {
        assert!(is_newer("0.0.10", "0.0.9"));
        assert!(is_newer("v0.1.0", "0.0.9"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.0.3", "0.0.3"));
        assert!(!is_newer("v0.0.3", "0.0.3"));
        assert!(!is_newer("0.0.2", "0.0.3"));
    }

    #[test]
    fn newer_release_is_reported_with_its_page() {
        let o = interpret(r#"{"tag_name":"v9.0.0","html_url":"https://github.com/a/b/releases/tag/v9.0.0","body":"Better"}"#, "0.0.3");
        assert_eq!(o, Outcome::Available { version: "9.0.0".into(), url: "https://github.com/a/b/releases/tag/v9.0.0".into(), notes: "Better".into(), download: None });
    }

    fn release_files() -> Vec<(String, String)> {
        [
            "SHA256SUMS.txt",
            "TeXora-1.0.1-linux-arm64.tar.gz",
            "TeXora-1.0.1-linux-x64.tar.gz",
            "TeXora-1.0.1-mac-arm64.dmg",
            "TeXora-1.0.1-mac-arm64.zip",
            "TeXora-1.0.1-mac-x64.dmg",
            "TeXora-1.0.1-mac-x64.zip",
            "TeXora-1.0.1-windows-x64-setup.exe",
            "TeXora-1.0.1-windows-x64.zip",
            "texora_1.0.1_amd64.deb",
            "texora_1.0.1_arm64.deb",
        ]
        .iter()
        .map(|n| (n.to_string(), format!("https://github.com/BluesilveEmperor/TexOra--SS/releases/download/v1.0.1/{n}")))
        .collect()
    }

    #[test]
    fn each_computer_gets_its_own_package() {
        let f = release_files();
        let name = |os, arch, dpkg| pick_asset(&f, os, arch, dpkg).map(|(n, _)| n);
        assert_eq!(name("macos", "aarch64", false).as_deref(), Some("TeXora-1.0.1-mac-arm64.dmg"));
        assert_eq!(name("macos", "x86_64", false).as_deref(), Some("TeXora-1.0.1-mac-x64.dmg"));
        assert_eq!(name("windows", "x86_64", false).as_deref(), Some("TeXora-1.0.1-windows-x64-setup.exe"));
        assert_eq!(name("linux", "x86_64", true).as_deref(), Some("texora_1.0.1_amd64.deb"));
        assert_eq!(name("linux", "aarch64", true).as_deref(), Some("texora_1.0.1_arm64.deb"));
        assert_eq!(name("linux", "x86_64", false).as_deref(), Some("TeXora-1.0.1-linux-x64.tar.gz"));
        // With no installer for this computer, the portable package is offered; with nothing, no button.
        let zips: Vec<_> = f.iter().filter(|(n, _)| !n.ends_with(".dmg")).cloned().collect();
        assert_eq!(pick_asset(&zips, "macos", "aarch64", false).map(|(n, _)| n).as_deref(), Some("TeXora-1.0.1-mac-arm64.zip"));
        assert_eq!(pick_asset(&[], "macos", "aarch64", false), None);
    }

    #[test]
    fn github_release_from_1_0_0_to_1_0_1() {
        // The shape GitHub's "latest release" answer has, as published for BluesilveEmperor/TexOra--SS.
        let assets: Vec<String> = release_files()
            .iter()
            .map(|(n, u)| format!(r#"{{"name":"{n}","browser_download_url":"{u}","size":1}}"#))
            .collect();
        let body = format!(
            r#"{{"tag_name":"v1.0.1","name":"TeXora 1.0.1","draft":false,"prerelease":false,"html_url":"https://github.com/BluesilveEmperor/TexOra--SS/releases/tag/v1.0.1","body":"What is new","assets":[{}]}}"#,
            assets.join(",")
        );
        match interpret(&body, "1.0.0") {
            Outcome::Available { version, url, notes, download } => {
                assert_eq!(version, "1.0.1");
                assert!(url.ends_with("/releases/tag/v1.0.1"));
                assert_eq!(notes, "What is new");
                let (name, link) = download.expect("a package for this computer");
                assert!(name.contains("1.0.1") && link.ends_with(&name));
            }
            o => panic!("{o:?}"),
        }
        // The same release seen from 1.0.1 itself, or from a later version, is not an update.
        assert_eq!(interpret(&body, "1.0.1"), Outcome::UpToDate);
        assert_eq!(interpret(&body, "1.0.2"), Outcome::UpToDate);
    }

    #[test]
    fn same_version_is_up_to_date() {
        assert_eq!(interpret(r#"{"tag_name":"v0.0.3"}"#, "0.0.3"), Outcome::UpToDate);
    }

    #[test]
    fn bad_answers_are_failures_not_updates() {
        assert!(matches!(interpret("<html>", "0.0.3"), Outcome::Failed(_)));
        assert!(matches!(interpret("{}", "0.0.3"), Outcome::Failed(_)));
    }

    #[test]
    fn nothing_published_counts_as_up_to_date() {
        assert_eq!(check("  "), Outcome::UpToDate);
        assert_eq!(from_status(404, || None), Outcome::UpToDate);
    }

    #[test]
    fn busy_github_is_not_mistaken_for_up_to_date() {
        assert!(matches!(from_status(403, || None), Outcome::Failed(_)));
        assert!(matches!(from_status(500, || None), Outcome::Failed(_)));
    }
}
