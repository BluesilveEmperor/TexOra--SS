//! The Chinese text for the English strings in the program.
//!
//! Every file here holds one part of the table, so that the work of translating can be split up
//! and so that a string is written down once. A key that appears twice takes the last text; the
//! test at the end of this file makes sure that never happens by accident.

use std::collections::HashMap;
use std::sync::OnceLock;

pub mod app;
pub mod chrome;
pub mod cli;
pub mod latex;
pub mod notes;
pub mod panels;
pub mod present;
pub mod settings;
pub mod start;
pub mod studio;
pub mod tools;
pub mod video;

/// Every part of the table.
pub const TABLES: &[&[(&str, &str)]] = &[
    app::TABLE,
    chrome::TABLE,
    cli::TABLE,
    latex::TABLE,
    notes::TABLE,
    panels::TABLE,
    present::TABLE,
    settings::TABLE,
    start::TABLE,
    studio::TABLE,
    tools::TABLE,
    video::TABLE,
];

/// All the translations, by English text.
fn map() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::with_capacity(1024);
        for table in TABLES {
            for (en, zh) in table.iter() {
                m.insert(*en, *zh);
            }
        }
        m
    })
}

/// The Chinese text for an English string, if there is one.
pub fn lookup(key: &str) -> Option<&'static str> {
    map().get(key).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_english_string_is_translated_twice() {
        let mut seen = std::collections::HashSet::new();
        for table in TABLES {
            for (en, zh) in table.iter() {
                assert!(seen.insert(*en), "translated twice: {en:?}");
                assert!(!zh.trim().is_empty(), "empty translation: {en:?}");
                assert_ne!(*zh, *en, "left in English: {en:?}");
            }
        }
    }

    #[test]
    fn the_table_is_not_empty() {
        assert!(TABLES.iter().all(|t| !t.is_empty()));
    }
}
