//! The language of the interface: English and Simplified Chinese.
//!
//! English is the source language. The text in the program is the English text, and the Chinese
//! table in [`zh`] maps each English string to its Chinese one. A string with no entry shows in
//! English, so a missing translation leaves a hole in nothing: the interface is always readable.
//!
//! The language is a global because the interface asks for text from all over the program, far from
//! the place that holds the settings. It is read once per string, so changing it takes effect on
//! the next frame, without a restart.

use std::sync::atomic::{AtomicU8, Ordering};

mod zh;

/// The languages the interface can be in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    English,
    Chinese,
}

impl Lang {
    /// Every language, in the order they are offered.
    pub const ALL: [Lang; 2] = [Lang::English, Lang::Chinese];

    /// The code written to the settings file.
    pub fn code(self) -> &'static str {
        match self {
            Lang::English => "en",
            Lang::Chinese => "zh-CN",
        }
    }

    /// The name of the language in that language, so a reader who does not speak the current one
    /// can still find their own.
    pub fn own_name(self) -> &'static str {
        match self {
            Lang::English => "English",
            Lang::Chinese => "简体中文",
        }
    }

    /// The name of the language in English, for the settings file and for the command line.
    pub fn english_name(self) -> &'static str {
        match self {
            Lang::English => "English",
            Lang::Chinese => "Simplified Chinese",
        }
    }

    /// Reads a code from the settings file. Anything unknown is English, so a file written by a
    /// later version, or by hand, still opens.
    pub fn from_code(code: &str) -> Lang {
        match code.trim().to_ascii_lowercase().as_str() {
            "zh" | "zh-cn" | "zh-hans" | "zh-sg" | "zh-hans-cn" | "chinese" | "中文" => Lang::Chinese,
            _ => Lang::English,
        }
    }

    /// The locale to ask the operating system fonts for. Han characters exist in Japanese and
    /// Chinese fonts alike, and the two draw some of them differently, so the language of the
    /// interface decides which ones are used.
    pub fn font_locale(self) -> Option<&'static str> {
        match self {
            Lang::English => None,
            Lang::Chinese => Some("zh-Hans"),
        }
    }
}

/// Which language the interface is in. 0 is English, 1 is Chinese.
static LANG: AtomicU8 = AtomicU8::new(0);

/// The language the interface is in right now.
pub fn lang() -> Lang {
    match LANG.load(Ordering::Relaxed) {
        1 => Lang::Chinese,
        _ => Lang::English,
    }
}

/// Sets the language of the interface. It takes effect at once: the interface is drawn again every
/// frame, and every string is looked up as it is drawn.
pub fn set_lang(lang: Lang) {
    LANG.store(match lang {
        Lang::English => 0,
        Lang::Chinese => 1,
    }, Ordering::Relaxed);
}

/// The text of an English string in the chosen language. The English string itself is the key, so
/// the program reads as English and needs no table of its own.
pub fn t(key: &'static str) -> &'static str {
    match lang() {
        Lang::English => key,
        Lang::Chinese => zh::lookup(key).unwrap_or(key),
    }
}

/// The text of an English string in a language asked for, whatever the language of the interface
/// is. For the few places that show the text of a language other than the one in use.
pub fn t_in(lang: Lang, key: &'static str) -> &'static str {
    match lang {
        Lang::English => key,
        Lang::Chinese => zh::lookup(key).unwrap_or(key),
    }
}

/// Fills the `{0}`, `{1}`, ... placeholders of a translated text with the values given, in the
/// order they are given.
///
/// `format!` wants its format string to be a literal, but a translated text is only known when the
/// program runs, so the placeholders are put in by hand: the text first, the values after it.
#[macro_export]
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

/// The text of the interface in the chosen language: `t!("Open file")`.
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::t($key)
    };
}

/// The built-in documentation, in the chosen language.
pub fn guide(lang: Lang) -> &'static str {
    match lang {
        Lang::English => include_str!("../assets/guide.md"),
        Lang::Chinese => include_str!("../assets/guide.zh-CN.md"),
    }
}

/// The text of the About window, in the chosen language.
pub fn about(lang: Lang) -> &'static str {
    match lang {
        Lang::English => include_str!("../assets/about.md"),
        Lang::Chinese => include_str!("../assets/about.zh-CN.md"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_names_its_language_and_anything_else_is_english() {
        assert_eq!(Lang::from_code("en"), Lang::English);
        assert_eq!(Lang::from_code("zh-CN"), Lang::Chinese);
        assert_eq!(Lang::from_code("zh"), Lang::Chinese);
        assert_eq!(Lang::from_code("ZH-HANS"), Lang::Chinese);
        assert_eq!(Lang::from_code("  zh_cn "), Lang::English);
        assert_eq!(Lang::from_code("kl"), Lang::English);
        assert_eq!(Lang::from_code(""), Lang::English);
    }

    #[test]
    fn english_is_the_text_itself() {
        // The language is a global, so the tests ask for the language they want instead of
        // changing it: one test setting it would change it for the others too.
        assert_eq!(t_in(Lang::English, "Open file"), "Open file");
    }

    #[test]
    fn a_string_with_no_translation_stays_in_english() {
        assert_eq!(t_in(Lang::Chinese, "a string nobody translated"), "a string nobody translated");
    }

    #[test]
    fn every_language_has_a_code_and_a_name() {
        for l in Lang::ALL {
            assert_eq!(Lang::from_code(l.code()), l);
            assert!(!l.own_name().is_empty());
            assert!(!l.english_name().is_empty());
        }
    }
}
