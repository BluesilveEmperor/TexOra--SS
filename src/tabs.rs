//! Browser-style tabs, and the bookmarks and question marks kept for each deck.
//!
//! The App always holds the working state of the active tab. Other tabs keep theirs in a `Tab`, and
//! switching exchanges the two, so nothing is lost and the rest of the app need not know about tabs.
use crate::analytics::{Session, Sidecar, Stroke};
use crate::app::*;
use crate::deck::Deck;
use crate::latex::{self, Problem};
use crate::notes::Notes;
use crate::pdf::{Cmd, DocInfo, Region, SearchHit};
use egui::{Context, TextureHandle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Instant, SystemTime};

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

macro_rules! tab_state {
    ($($field:ident : $kind:ty = $start:expr),* $(,)?) => {
        pub struct Tab {
            pub id: u64,
            $(pub $field: $kind,)*
        }

        impl Tab {
            pub fn blank(id: u64) -> Self {
                Tab { id, $($field: $start,)* }
            }
        }

        impl App {
            fn swap_state(&mut self, tab: &mut Tab) {
                $(std::mem::swap(&mut self.$field, &mut tab.$field);)*
            }
        }
    };
}

tab_state! {
    gen: u64 = 0,
    doc: Option<DocInfo> = None,
    deck: Deck = Deck::default(),
    pdf_path: Option<PathBuf> = None,
    deck_path: Option<PathBuf> = None,
    pdf_mtime: Option<SystemTime> = None,
    loading: bool = false,
    cur: usize = 0,
    frozen: Option<usize> = None,
    history: Vec<usize> = vec![],
    view: View = View::Start,
    status: String = String::new(),
    cache: HashMap<(usize, Region), Vec<(u32, TextureHandle, u64)>> = HashMap::new(),
    side: Sidecar = Sidecar::default(),
    notes: Notes = Notes::new(),
    notes_dirty: Option<Instant> = None,
    notes_view: NotesView = NotesView::Auto,
    note_draft: Option<(usize, String)> = None,
    note_focus: bool = false,
    swap: bool = false,
    tool: Tool = Tool::Pointer,
    screen: Screen = Screen::Live,
    stroke: Option<Stroke> = None,
    laser: Vec<([f32; 2], f64)> = vec![],
    zoom: Option<[f32; 4]> = None,
    zoom_drag: Option<[f32; 2]> = None,
    notes_zoom: Option<[f32; 4]> = None,
    next_zoom: Option<[f32; 4]> = None,
    timer_on: bool = false,
    timer_touched: bool = false,
    elapsed: f64 = 0.0,
    session: Session = Session::default(),
    src: Option<Source> = None,
    compiling: bool = false,
    compile_again: bool = false,
    problems: Vec<Problem> = vec![],
    full_log: String = String::new(),
    reqs: Vec<latex::Requirement> = vec![],
    req_warning: Option<String> = None,
    req_rx: Option<Receiver<(Vec<latex::Requirement>, Option<String>)>> = None,
    log_tab: LogTab = LogTab::Problems,
    log_open: bool = true,
    compile_ok: Option<bool> = None,
    compile_note: String = String::new(),
    open_after_compile: bool = false,
    note_pending: Notes = Notes::new(),
    sync_mark: Option<(usize, [f32; 2], Instant)> = None,
    anim_play: HashMap<(usize, usize), crate::app::AnimPlay> = HashMap::new(),
    anim_cache: HashMap<(usize, usize, usize), (u32, TextureHandle)> = HashMap::new(),
    anim_pending: std::collections::HashSet<crate::pdf::FrameReq> = std::collections::HashSet::new(),
    anim_page: Option<usize> = None,
    search_q: String = String::new(),
    search_sent: String = String::new(),
    hits: Vec<SearchHit> = vec![],
}

/// A different starting colour for each section; the Contents sidebar can change them.
pub const SECTION_COLORS: [[u8; 3]; 8] =
    [[37, 99, 200], [226, 120, 20], [46, 140, 70], [130, 70, 190], [205, 50, 60], [20, 145, 150], [215, 80, 150], [140, 100, 50]];
const NO_SECTION: [u8; 3] = [120, 120, 118];

/// The colour of a section by its place in the deck, or grey before the first section.
pub fn section_slot_color(index: Option<usize>) -> [u8; 3] {
    index.map(|i| SECTION_COLORS[i % SECTION_COLORS.len()]).unwrap_or(NO_SECTION)
}

impl App {
    // ---------- tabs ----------

    pub fn active_id(&self) -> u64 {
        self.tabs[self.active].id
    }

    pub fn tab_index(&self, id: u64) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// Exchanges the working state with the active tab's storage slot.
    fn stow(&mut self) {
        let a = self.active;
        let mut slot = std::mem::replace(&mut self.tabs[a], Tab::blank(0));
        self.swap_state(&mut slot);
        self.tabs[a] = slot;
    }

    pub fn activate(&mut self, i: usize) {
        if i != self.active && i < self.tabs.len() {
            self.stow();
            self.active = i;
            self.stow();
        }
    }

    /// Runs `f` as if tab `id` were active, then comes back.
    pub fn with_tab(&mut self, id: u64, f: impl FnOnce(&mut App)) {
        let here = self.active_id();
        let Some(i) = self.tab_index(id) else { return };
        self.activate(i);
        f(self);
        if let Some(back) = self.tab_index(here) {
            self.activate(back);
        }
    }

    pub fn tab_of_gen(&self, gen: u64) -> Option<u64> {
        self.tabs.iter().enumerate().find(|(i, t)| *i != self.active && t.gen == gen).map(|(_, t)| t.id)
    }

    /// Adds an empty tab at the end and goes to it.
    pub fn new_tab(&mut self) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(Tab::blank(id));
        self.activate(self.tabs.len() - 1);
    }

    pub fn tab_is_home(&self, i: usize) -> bool {
        if i == self.active {
            self.view == View::Start
        } else {
            self.tabs[i].view == View::Start
        }
    }

    pub fn go_home(&mut self) {
        match (0..self.tabs.len()).find(|i| self.tab_is_home(*i)) {
            Some(i) => self.activate(i),
            None => self.new_tab(),
        }
    }

    /// A presentation takes Home's place.
    pub fn close_home_tabs(&mut self) {
        let here = self.active_id();
        self.tabs.retain(|t| t.id == here || t.view != View::Start);
        self.active = self.tab_index(here).unwrap_or(0);
    }

    /// Closing Home when nothing else is open ends the app. Closing the last presentation does not:
    /// that goes back to Home.
    pub fn closing_ends_app(&self, id: u64) -> bool {
        self.tabs.len() == 1 && self.tab_index(id).is_some_and(|i| self.tab_is_home(i))
    }

    pub fn tab_title(&self, i: usize) -> String {
        let (pdf, src, compiling) = if i == self.active {
            (&self.pdf_path, &self.src, self.compiling)
        } else {
            (&self.tabs[i].pdf_path, &self.tabs[i].src, self.tabs[i].compiling)
        };
        let path = pdf.as_ref().or(src.as_ref().map(|s| &s.path));
        let Some(path) = path else { return crate::t!("Home").to_string() };
        let name = path.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let dirty = src.as_ref().map(|s| s.dirty).unwrap_or(false);
        format!("{}{name}{}", if dirty { "• " } else { "" }, if compiling { " …" } else { "" })
    }

    /// The tab already showing this file, if any.
    pub fn tab_for_path(&self, path: &Path) -> Option<usize> {
        let pdf = path.with_extension("pdf");
        (0..self.tabs.len()).find(|i| {
            let (p, s) = if *i == self.active { (&self.pdf_path, &self.src) } else { (&self.tabs[*i].pdf_path, &self.tabs[*i].src) };
            p.as_deref() == Some(pdf.as_path()) || s.as_ref().map(|s| s.path == path).unwrap_or(false)
        })
    }

    /// Everything a tab must keep before it goes away.
    pub fn save_open_work(&mut self) {
        self.save_note_draft();
        if self.notes_dirty.is_some() {
            self.save_notes();
        }
        if self.src.as_ref().map(|s| s.dirty).unwrap_or(false) {
            self.save_source();
        }
        self.finish_session();
        self.save_side();
    }

    pub fn close_tab(&mut self, id: u64, ctx: &Context) {
        let Some(i) = self.tab_index(id) else { return };
        self.activate(i);
        self.save_open_work();
        if self.gen > 0 {
            let _ = self.engine.tx.send(Cmd::Close { gen: self.gen });
        }
        if self.audience_open && self.audience_tab == id {
            self.close_audience(ctx);
        }
        let mut gone = Tab::blank(0);
        self.swap_state(&mut gone);
        drop(gone);
        self.pending.clear();
        self.tabs.remove(self.active);
        if self.tabs.is_empty() {
            let fresh = self.next_tab_id;
            self.next_tab_id += 1;
            self.tabs.push(Tab::blank(fresh));
            self.active = 0;
            return;
        }
        self.active = self.active.min(self.tabs.len() - 1);
        self.stow();
    }

    /// Time keeps running in a talk while another tab is in front.
    pub fn tick_background_timers(&mut self, dt: f64) {
        let active = self.active;
        for (i, t) in self.tabs.iter_mut().enumerate() {
            if i != active && t.timer_on {
                t.elapsed += dt;
                let slide = t.frozen.unwrap_or(t.cur);
                if let Some(f) = t.deck.slides.get(slide).map(|s| s.frame) {
                    t.session.tick(f, dt as f32);
                }
            }
        }
    }

    // ---------- bookmarks and questions ----------

    /// The colour of the section a slide belongs to.
    pub fn section_color(&self, frame: usize) -> [u8; 3] {
        let at = self.deck.sections.iter().rposition(|s| s.first_frame <= frame);
        let chosen = at.and_then(|i| self.side.section_colors.get(&self.deck.sections[i].title)).copied();
        chosen.unwrap_or_else(|| section_slot_color(at))
    }

    /// Adds a bookmark to the current slide, or removes it if there is one.
    pub fn toggle_bookmark(&mut self) {
        let f = self.frame();
        if self.deck.frames.is_empty() {
            return;
        }
        self.status = if self.side.bookmarks.remove(&f) {
            tfill!(crate::t!("Bookmark removed from slide {0}."), f + 1)
        } else {
            self.side.bookmarks.insert(f);
            tfill!(crate::t!("Bookmarked slide {0}."), f + 1)
        };
        self.save_side();
    }

    /// Marks the current slide as one where a question came up, or clears the mark.
    pub fn toggle_question(&mut self) {
        let f = self.frame();
        if self.deck.frames.is_empty() {
            return;
        }
        if self.side.questions.remove(&f) {
            self.session.questions.retain(|q| *q != f);
            self.status = tfill!(crate::t!("Question mark removed from slide {0}."), f + 1);
        } else {
            self.side.questions.insert(f);
            self.session.questions.push(f);
            self.status = tfill!(crate::t!("Marked a question on slide {0}."), f + 1);
        }
        self.save_side();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_start_with_different_colours() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..SECTION_COLORS.len() {
            assert!(seen.insert(section_slot_color(Some(i))));
        }
        assert_eq!(section_slot_color(Some(SECTION_COLORS.len())), SECTION_COLORS[0]);
        assert_eq!(section_slot_color(None), NO_SECTION);
    }
}
