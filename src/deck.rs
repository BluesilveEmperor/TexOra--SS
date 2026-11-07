//! Turns PDF pages into a presentation: slides, Beamer overlay groups, notes pages, backup slides.
use crate::pdf::{DocInfo, Region};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum NotesMode {
    /// Double-width pages hold slide and notes side by side.
    #[default]
    Auto,
    Right,
    Left,
    /// Beamer "show notes": every second page is a notes page.
    Interleaved,
    None,
}

#[derive(Clone, Debug)]
pub struct Slide {
    pub page: usize,
    pub region: Region,
    pub notes: Option<(usize, Region)>,
    pub label: String,
    pub frame: usize,
    pub step: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub first: usize,
    pub last: usize,
    pub title: String,
    pub words: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Section {
    pub title: String,
    pub first_frame: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Deck {
    pub slides: Vec<Slide>,
    pub frames: Vec<Frame>,
    pub sections: Vec<Section>,
    /// First frame of the appendix (backup slides), if there is one.
    pub backup_start: Option<usize>,
    pub aspect: f32,
}

/// What Beamer's .nav file adds: exact overlay groups and where the appendix starts (1-based pages).
#[derive(Clone, Debug, Default)]
pub struct Nav {
    pub frame_pages: Vec<(usize, usize)>,
    pub appendix_page: Option<usize>,
}

pub fn read_nav(pdf: &Path) -> Nav {
    let mut nav = Nav::default();
    let Ok(src) = std::fs::read_to_string(pdf.with_extension("nav")) else { return nav };
    let args = |line: &str, key: &str| -> Option<Vec<usize>> {
        let at = line.find(key)? + key.len();
        Some(line[at..].split(['{', '}']).filter_map(|p| p.trim().parse().ok()).collect())
    };
    for line in src.lines() {
        if let Some(a) = args(line, "\\beamer@framepages") {
            if a.len() >= 2 {
                nav.frame_pages.push((a[0], a[1]));
            }
        } else if let Some(a) = args(line, "\\beamer@appendixpages") {
            nav.appendix_page = a.first().copied();
        }
    }
    nav
}

pub fn build(doc: &DocInfo, mode: NotesMode, nav: &Nav, titles: &[String]) -> Deck {
    let mut deck = Deck::default();
    let n = doc.pages.len();
    let wide = |i: usize| doc.pages[i].width / doc.pages[i].height.max(1.0) > 2.2;
    let split = match mode {
        NotesMode::Auto => n > 0 && wide(0),
        NotesMode::Right | NotesMode::Left => true,
        _ => false,
    };
    let (slide_region, note_region) = match mode {
        NotesMode::Left => (Region::Right, Region::Left),
        _ if split => (Region::Left, Region::Right),
        _ => (Region::Full, Region::Full),
    };
    let stride = if mode == NotesMode::Interleaved { 2 } else { 1 };
    let mut i = 0;
    while i < n {
        let notes = if split {
            Some((i, note_region))
        } else if mode == NotesMode::Interleaved && i + 1 < n {
            Some((i + 1, Region::Full))
        } else {
            None
        };
        let label = doc.pages[i].label.clone().unwrap_or_else(|| (i + 1).to_string());
        deck.slides.push(Slide { page: i, region: slide_region, notes, label, frame: 0, step: 0 });
        i += stride;
    }
    // Overlay groups: Beamer's own record when we have it, otherwise repeated page labels.
    let from_nav = !nav.frame_pages.is_empty() && stride == 1;
    let mut cur = 0;
    for k in 0..deck.slides.len() {
        let page = deck.slides[k].page;
        let f = if from_nav {
            nav.frame_pages.iter().position(|(a, b)| page + 1 >= *a && page < *b).unwrap_or(cur)
        } else if k == 0 {
            0
        } else if deck.slides[k - 1].label == deck.slides[k].label {
            cur
        } else {
            cur + 1
        };
        cur = f;
        deck.slides[k].frame = f;
        if f >= deck.frames.len() {
            deck.frames.resize(f + 1, Frame::default());
            deck.frames[f] = Frame { first: k, last: k, ..Default::default() };
        } else {
            deck.frames[f].last = k;
            deck.slides[k].step = k - deck.frames[f].first;
        }
    }
    for (f, frame) in deck.frames.iter_mut().enumerate() {
        frame.title = titles.get(f).filter(|t| !t.is_empty()).cloned().unwrap_or_else(|| format!("Slide {}", f + 1));
        let s = &deck.slides[frame.last];
        let p = &doc.pages[s.page];
        let text = match s.region {
            Region::Full => &p.text_full,
            Region::Left => &p.text_left,
            Region::Right => &p.text_right,
        };
        frame.words = text.split_whitespace().count();
    }
    if let Some(ap) = nav.appendix_page {
        deck.backup_start = deck.slides.iter().find(|s| s.page + 1 >= ap).map(|s| s.frame);
    }
    for o in doc.outline.iter().filter(|o| o.depth == 0) {
        if let Some(s) = deck.slides.iter().find(|s| s.page >= o.page) {
            if !deck.sections.iter().any(|x| x.title == o.title) {
                deck.sections.push(Section { title: o.title.clone(), first_frame: s.frame });
            }
        }
    }
    if let Some(p) = doc.pages.first() {
        let w = if slide_region == Region::Full { p.width } else { p.width / 2.0 };
        deck.aspect = w / p.height.max(1.0);
    }
    deck
}

impl Deck {
    pub fn slide_for_page(&self, page: usize) -> Option<usize> {
        self.slides.iter().rposition(|s| s.page <= page)
    }
    /// Frames in the main talk, leaving out backup slides.
    pub fn talk_frames(&self) -> usize {
        self.backup_start.unwrap_or(self.frames.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::PageInfo;

    fn doc(labels: &[&str], w: f32) -> DocInfo {
        DocInfo {
            pages: labels
                .iter()
                .map(|l| PageInfo { width: w, height: 100.0, label: Some(l.to_string()), ..Default::default() })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn repeated_labels_become_overlay_steps() {
        let d = build(&doc(&["1", "2", "2", "2", "3"], 133.0), NotesMode::Auto, &Nav::default(), &[]);
        assert_eq!(d.frames.len(), 3);
        assert_eq!(d.slides[3].frame, 1);
        assert_eq!(d.slides[3].step, 2);
        assert_eq!(d.frames[1].first, 1);
        assert_eq!(d.frames[1].last, 3);
        assert!(d.slides[0].notes.is_none());
    }

    #[test]
    fn double_width_pages_split_into_slide_and_notes() {
        let d = build(&doc(&["1", "2"], 266.0), NotesMode::Auto, &Nav::default(), &[]);
        assert_eq!(d.slides[0].region, Region::Left);
        assert_eq!(d.slides[0].notes, Some((0, Region::Right)));
        assert!((d.aspect - 1.33).abs() < 0.01);
    }

    #[test]
    fn interleaved_notes_use_every_second_page() {
        let d = build(&doc(&["1", "1", "2", "2"], 133.0), NotesMode::Interleaved, &Nav::default(), &[]);
        assert_eq!(d.slides.len(), 2);
        assert_eq!(d.slides[1].page, 2);
        assert_eq!(d.slides[1].notes, Some((3, Region::Full)));
    }

    #[test]
    fn nav_file_gives_groups_and_backup_start() {
        let nav = Nav { frame_pages: vec![(1, 1), (2, 3), (4, 4)], appendix_page: Some(4) };
        let d = build(&doc(&["1", "2", "3", "4"], 133.0), NotesMode::Auto, &nav, &[]);
        assert_eq!(d.frames.len(), 3);
        assert_eq!(d.backup_start, Some(2));
        assert_eq!(d.talk_frames(), 2);
    }
}
