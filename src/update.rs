//! Looks for a newer TeXora among the releases published on GitHub.
use serde_json::Value;
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The GitHub project TeXora is published in, as "owner/name". Until it has a published release,
/// GitHub answers "not found" and TeXora counts as up to date.
const GITHUB_REPO: &str = "khalidt/TeXora";

pub const WEBSITE: &str = "https://github.com/khalidt/TeXora";

pub const DEVELOPER: &str = "Khalid Alkhaldi";

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    UpToDate,
    Available { version: String, url: String, notes: String },
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
        return Outcome::Failed("The update information could not be read.".to_string());
    };
    let Some(tag) = v["tag_name"].as_str() else {
        return Outcome::Failed("The update information has no version number.".to_string());
    };
    if is_newer(tag, current) {
        Outcome::Available {
            version: tag.trim_start_matches(['v', 'V']).to_string(),
            url: v["html_url"].as_str().unwrap_or("").to_string(),
            notes: v["body"].as_str().unwrap_or("").trim().to_string(),
        }
    } else {
        Outcome::UpToDate
    }
}

fn from_status(status: u16, body: impl FnOnce() -> Option<String>) -> Outcome {
    match status {
        200 => match body() {
            Some(t) => interpret(&t, VERSION),
            None => Outcome::Failed("The update information could not be read.".to_string()),
        },
        // GitHub says this when the project has no published release: nothing newer exists.
        404 => Outcome::UpToDate,
        403 | 429 => Outcome::Failed("GitHub is busy right now. Try again later.".to_string()),
        s => Outcome::Failed(format!("GitHub answered with an error ({s}).")),
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
        Err(_) => Outcome::Failed("Could not reach GitHub. Check your internet connection.".to_string()),
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
        assert_eq!(o, Outcome::Available { version: "9.0.0".into(), url: "https://github.com/a/b/releases/tag/v9.0.0".into(), notes: "Better".into() });
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
