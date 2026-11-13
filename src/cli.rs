//! What ? > Command-line tool shows: the commands that run this very copy of TeXora, written for the
//! terminal of the system it runs on, wherever it was installed.

use crate::i18n::Lang;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Os {
    Mac,
    Linux,
    Windows,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Mac
        } else {
            Os::Linux
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CliHelp {
    /// The program's own path, as found at run time.
    pub path: String,
    /// What to type to run it: the path, quoted for this system's terminal.
    pub run: String,
    /// Which terminal these commands are written for.
    pub terminal: &'static str,
    /// Ready-to-run examples and what each does.
    pub examples: Vec<(String, &'static str)>,
    /// How to make a short name work, and the line that does it.
    pub setup_text: String,
    pub setup_line: String,
    /// A note for other terminals on this system, if any.
    pub note: Option<String>,
    /// A warning when this copy runs from a place that will not last.
    pub warning: Option<String>,
}

/// The program that is running now, with links resolved, wherever it was installed.
pub fn this_program() -> String {
    std::env::current_exe().ok().map(|p| p.canonicalize().unwrap_or(p)).map(|p| p.display().to_string()).unwrap_or_default()
}

/// Quotes a path for zsh and bash only when it needs it.
pub fn sh_quote(s: &str) -> String {
    let safe = !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "/._-+:@%,=".contains(c));
    if safe {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// A line for ~/.zshrc or ~/.bashrc that makes `texora` run the program at `path`. The alias's value
/// must carry its own quotes, or a folder name with a space splits the command in two.
pub fn shell_alias(path: &str) -> String {
    let quoted = sh_quote(path);
    if quoted == path {
        format!("alias texora={path}")
    } else if !path.chars().any(|c| matches!(c, '"' | '$' | '`' | '\\' | '!' | '\'')) {
        format!("alias texora=\"'{path}'\"")
    } else {
        // Rare characters: quote twice over with single quotes, so the shell reads the path back exactly.
        format!("alias texora={}", sh_quote(&quoted))
    }
}

/// The text of a string in the language asked for. English is the source language, so it is the
/// key itself; the Chinese text comes from the table of the interface, which is written in the
/// language the interface is in. So `lang` is the language of the interface.
fn text(lang: Lang, key: &'static str) -> &'static str {
    crate::i18n::t_in(lang, key)
}

/// A text with its {0}, {1}… placeholders filled in, in the order they are given.
fn fill(s: &str, args: &[&str]) -> String {
    let mut out = s.to_string();
    for (i, arg) in args.iter().enumerate() {
        let key = format!("{{{i}}}");
        out = out.replace(key.as_str(), *arg);
    }
    out
}

/// The help of the command-line tool, in the language of the interface. The commands themselves
/// stay as they are: they are what the reader types. Only the words that explain them change.
pub fn cli_help_in(os: Os, path: &str, lang: Lang) -> CliHelp {
    const EXAMPLES: [(&str, &str); 4] = [
        ("--help", "every option"),
        ("--version", "which version this is"),
        ("--self-test", "check this copy works"),
        ("talk.pdf", "open a presentation"),
    ];
    match os {
        Os::Windows => {
            let run = format!("& \"{path}\"");
            let dir = path.rsplit_once('\\').map(|(d, _)| d).unwrap_or(path);
            CliHelp {
                path: path.to_string(),
                terminal: "PowerShell",
                examples: EXAMPLES.iter().map(|(a, w)| (format!("{run} {a}"), text(lang, *w))).collect(),
                setup_text: text(lang, "To type just TeXora from any folder, run this once in PowerShell, then open a new terminal:").to_string(),
                setup_line: format!(
                    "$p = [Environment]::GetEnvironmentVariable('Path', 'User'); [Environment]::SetEnvironmentVariable('Path', \"$p;{}\", 'User')",
                    dir.replace('\'', "''")
                ),
                note: Some(fill(text(lang, "In Command Prompt, leave out the & at the start: \"{0}\" --help"), &[path])),
                warning: temporary_place(os, path, lang),
                run,
            }
        }
        Os::Mac | Os::Linux => {
            let run = sh_quote(path);
            let (terminal, file) = if os == Os::Mac { ("Terminal (zsh)", "~/.zshrc") } else { ("a terminal (bash)", "~/.bashrc") };
            let app = path.find(".app/Contents/MacOS/").map(|i| path[..i + 4].to_string());
            CliHelp {
                path: path.to_string(),
                terminal,
                examples: EXAMPLES.iter().map(|(a, w)| (format!("{run} {a}"), text(lang, *w))).collect(),
                setup_text: fill(text(lang, "To type just texora instead, add this line to {0}, then open a new terminal:"), &[file]),
                setup_line: shell_alias(path),
                note: app.map(|a| fill(text(lang, "This is the program inside {0}. If you move the app, open this box again for the new path."), &[a.as_str()])),
                warning: temporary_place(os, path, lang),
                run,
            }
        }
    }
}

/// Copies run from a place that disappears give paths that stop working.
fn temporary_place(os: Os, path: &str, lang: Lang) -> Option<String> {
    let lower = path.to_lowercase();
    if os == Os::Mac && path.contains("/AppTranslocation/") {
        Some(text(lang, "macOS is running this copy from a temporary place, because it was opened straight from where it was downloaded. Move TeXora.app into Applications, open it from there, then open this box again for a path that lasts.").to_string())
    } else if lower.contains("\\appdata\\local\\temp\\") || lower.contains("/tmp/") || lower.starts_with("/private/var/folders/") {
        Some(text(lang, "This copy is running from a temporary folder. Move it somewhere permanent, open it from there, then open this box again.").to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_paths_with_spaces_are_quoted_everywhere() {
        let h = cli_help_in(Os::Mac, "/Users/a/My Apps/TeXora.app/Contents/MacOS/TeXora", Lang::English);
        assert_eq!(h.run, "'/Users/a/My Apps/TeXora.app/Contents/MacOS/TeXora'");
        assert_eq!(h.examples[0].0, "'/Users/a/My Apps/TeXora.app/Contents/MacOS/TeXora' --help");
        assert_eq!(h.setup_line, "alias texora=\"'/Users/a/My Apps/TeXora.app/Contents/MacOS/TeXora'\"");
        assert!(h.note.unwrap().contains("/Users/a/My Apps/TeXora.app."));
        assert!(h.warning.is_none());
    }

    #[test]
    fn simple_paths_stay_plain() {
        let h = cli_help_in(Os::Mac, "/Applications/TeXora.app/Contents/MacOS/TeXora", Lang::English);
        assert_eq!(h.run, "/Applications/TeXora.app/Contents/MacOS/TeXora");
        assert_eq!(h.setup_line, "alias texora=/Applications/TeXora.app/Contents/MacOS/TeXora");
        let h = cli_help_in(Os::Linux, "/home/sam/apps/TeXora-0.0.16-linux-x64/TeXora", Lang::English);
        assert_eq!(h.examples[1].0, "/home/sam/apps/TeXora-0.0.16-linux-x64/TeXora --version");
        assert!(h.setup_text.contains("~/.bashrc"));
        assert!(h.note.is_none());
    }

    #[test]
    fn windows_uses_powershell_and_adds_the_folder_to_path() {
        let h = cli_help_in(Os::Windows, r"C:\Users\Sam O'Neil\TeXora\TeXora.exe", Lang::English);
        assert_eq!(h.run, r#"& "C:\Users\Sam O'Neil\TeXora\TeXora.exe""#);
        assert_eq!(h.examples[0].0, r#"& "C:\Users\Sam O'Neil\TeXora\TeXora.exe" --help"#);
        assert!(h.setup_line.contains(r"$p;C:\Users\Sam O''Neil\TeXora"));
        assert!(h.note.unwrap().contains("leave out the &"));
    }

    #[test]
    fn temporary_places_are_flagged() {
        let h = cli_help_in(Os::Mac, "/private/var/folders/x/AppTranslocation/ABC/d/TeXora.app/Contents/MacOS/TeXora", Lang::English);
        assert!(h.warning.unwrap().contains("Applications"));
        assert!(cli_help_in(Os::Windows, r"C:\Users\s\AppData\Local\Temp\x\TeXora.exe", Lang::English).warning.is_some());
    }

    #[test]
    fn odd_characters_survive_quoting() {
        assert_eq!(sh_quote("/a/it's/TeXora"), r"'/a/it'\''s/TeXora'");
        assert!(shell_alias("/a/it's here/TeXora").starts_with("alias texora='"));
    }
}
