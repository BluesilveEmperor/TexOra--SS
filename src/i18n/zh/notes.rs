//! The Chinese text for the speaker notes (`src/notes.rs`).
//!
//! These are the words of the notes file that TeXora writes beside a presentation: the title at
//! the top of it, and nothing else.
//!
//! The headings under the title (`## Slide 1`) are the format of the file, not words of ours, and
//! stay as they are, so a notes file written under one language is read under another.
//!
//! The notes themselves are the writer's own words, and are never translated.
pub const TABLE: &[(&str, &str)] = &[
    ("Speaker notes (TeXora)", "演讲者备注（TeXora）"),
];
