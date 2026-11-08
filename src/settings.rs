//! Saved preferences and the configurable keyboard map.
use crate::latex::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Action {
    Next,
    Prev,
    NextFrame,
    PrevFrame,
    First,
    Last,
    Back,
    Jump,
    ToggleTimer,
    ResetTimer,
    Present,
    Fullscreen,
    Blackout,
    Whiteboard,
    Freeze,
    Overview,
    Studio,
    SwapNotes,
    Outline,
    Search,
    Analytics,
    Settings,
    Pointer,
    Pen,
    Highlighter,
    Eraser,
    Laser,
    ZoomTool,
    Undo,
    ClearInk,
    MarkQuestion,
    Open,
    Compile,
    Cancel,
    SplitView,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    StartPage,
    SetDuration,
    CheckPackages,
    Save,
    ToggleComment,
    Comment,
    Uncomment,
    EditNotes,
    AddBookmark,
    CloseTab,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::Next => "Next step",
            Action::Prev => "Previous step",
            Action::NextFrame => "Next slide (skip overlays)",
            Action::PrevFrame => "Previous slide (skip overlays)",
            Action::First => "First slide",
            Action::Last => "Last slide",
            Action::Back => "Back to where I was",
            Action::Jump => "Go to slide number",
            Action::ToggleTimer => "Start or pause timer",
            Action::ResetTimer => "Stop and reset timer",
            Action::Present => "Open audience window",
            Action::Fullscreen => "Fullscreen",
            Action::Blackout => "Black screen",
            Action::Whiteboard => "Whiteboard",
            Action::Freeze => "Freeze audience screen",
            Action::Overview => "Slide overview",
            Action::Studio => "LaTeX studio",
            Action::SwapNotes => "Large slide view",
            Action::Outline => "Contents and bookmarks",
            Action::Search => "Search",
            Action::Analytics => "Analytics",
            Action::Settings => "Settings",
            Action::Pointer => "Pointer",
            Action::Pen => "Pen",
            Action::Highlighter => "Highlighter",
            Action::Eraser => "Eraser",
            Action::Laser => "Laser pointer",
            Action::ZoomTool => "Zoom to area",
            Action::Undo => "Undo ink",
            Action::ClearInk => "Clear ink on slide",
            Action::MarkQuestion => "Mark a question on this slide",
            Action::Open => "Open file",
            Action::Compile => "Compile LaTeX",
            Action::Cancel => "Cancel or close",
            Action::SplitView => "Slide and notes split",
            Action::ZoomIn => "Zoom in",
            Action::ZoomOut => "Zoom out",
            Action::ZoomFit => "Fit to page",
            Action::StartPage => "Home",
            Action::SetDuration => "Set talk duration",
            Action::CheckPackages => "Check required packages",
            Action::Save => "Save source",
            Action::ToggleComment => "Comment or uncomment lines (LaTeX studio)",
            Action::Comment => "Comment out lines (LaTeX studio)",
            Action::Uncomment => "Uncomment lines (LaTeX studio)",
            Action::EditNotes => "Edit notes for this slide",
            Action::AddBookmark => "Bookmark this slide",
            Action::CloseTab => "Close this tab",
        }
    }
    /// Keys that keep working while typing, because they hold a modifier or close things.
    pub fn works_while_typing(self) -> bool {
        matches!(self, Action::Save | Action::ToggleComment | Action::Comment | Action::Uncomment | Action::Compile | Action::Search | Action::Open | Action::Cancel | Action::Studio | Action::CheckPackages | Action::StartPage | Action::CloseTab)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct KeyBind {
    pub key: String,
    #[serde(default)]
    pub cmd: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}

impl KeyBind {
    pub fn new(key: &str) -> Self {
        KeyBind { key: key.into(), cmd: false, shift: false, alt: false }
    }
    pub fn cmd(key: &str) -> Self {
        KeyBind { cmd: true, ..Self::new(key) }
    }
    pub fn shift(key: &str) -> Self {
        KeyBind { shift: true, ..Self::new(key) }
    }
    pub fn modifiers(&self) -> egui::Modifiers {
        let mut m = egui::Modifiers::NONE;
        if self.cmd {
            m = m | egui::Modifiers::COMMAND;
        }
        if self.shift {
            m = m | egui::Modifiers::SHIFT;
        }
        if self.alt {
            m = m | egui::Modifiers::ALT;
        }
        m
    }
    pub fn text(&self) -> String {
        let cmd = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
        let mut s = String::new();
        if self.cmd {
            s += cmd;
            s += "+";
        }
        if self.shift {
            s += "Shift+";
        }
        if self.alt {
            s += "Alt+";
        }
        s + &self.key
    }
}

pub fn default_keys() -> Vec<(Action, Vec<KeyBind>)> {
    use Action::*;
    let k = KeyBind::new;
    vec![
        (Next, vec![k("ArrowRight"), k("ArrowDown"), k("PageDown"), k("Space")]),
        (Prev, vec![k("ArrowLeft"), k("ArrowUp"), k("PageUp")]),
        (NextFrame, vec![KeyBind::shift("ArrowRight"), KeyBind::shift("ArrowDown")]),
        (PrevFrame, vec![KeyBind::shift("ArrowLeft"), KeyBind::shift("ArrowUp")]),
        (First, vec![k("Home")]),
        (Last, vec![k("End")]),
        (Back, vec![k("Backspace")]),
        (Jump, vec![k("G")]),
        (ToggleTimer, vec![k("P")]),
        (ResetTimer, vec![k("R")]),
        (Present, vec![k("F5")]),
        (Fullscreen, vec![k("F11")]),
        (Blackout, vec![k("B")]),
        (Whiteboard, vec![k("W")]),
        (Freeze, vec![k("F")]),
        (Overview, vec![k("O")]),
        (SwapNotes, vec![k("N")]),
        (Outline, vec![k("T")]),
        (Search, vec![KeyBind::cmd("F")]),
        (Analytics, vec![k("Y")]),
        (Settings, vec![KeyBind::cmd("Comma")]),
        (Pointer, vec![k("V")]),
        (Pen, vec![k("D")]),
        (Highlighter, vec![k("H")]),
        (Eraser, vec![k("X")]),
        (Laser, vec![k("L")]),
        (ZoomTool, vec![k("Z")]),
        (Undo, vec![KeyBind::cmd("Z")]),
        (ClearInk, vec![k("C")]),
        (MarkQuestion, vec![k("Q")]),
        (Open, vec![KeyBind::cmd("O")]),
        (Compile, vec![KeyBind::cmd("B")]),
        (Cancel, vec![k("Escape")]),
        (SplitView, vec![k("S")]),
        (ZoomIn, vec![k("Plus"), k("Equals")]),
        (ZoomOut, vec![k("Minus")]),
        (ZoomFit, vec![k("0")]),
        (StartPage, vec![KeyBind::cmd("L")]),
        (SetDuration, vec![k("U")]),
        (CheckPackages, vec![KeyBind::cmd("K")]),
        (Studio, vec![KeyBind::cmd("E")]),
        (Save, vec![KeyBind::cmd("S")]),
        (ToggleComment, vec![KeyBind::cmd("/")]),
        (Comment, vec![KeyBind::cmd("]")]),
        (Uncomment, vec![KeyBind::cmd("[")]),
        (EditNotes, vec![k("E")]),
        (AddBookmark, vec![k("M")]),
        (CloseTab, vec![KeyBind::cmd("W")]),
    ]
}

/// Where the dividers sit in each view, as a share of the space. Empty means fit automatically.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub presenter_col: Option<f32>,
    pub presenter_row: Option<f32>,
    pub large_col: Option<f32>,
    pub large_row: Option<f32>,
    pub split_col: Option<f32>,
    pub split_row: Option<f32>,
    /// The LaTeX studio's slide and notes preview, as a share of the width beside the editor.
    pub studio_col: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub keys: Vec<(Action, Vec<KeyBind>)>,
    pub engine: Engine,
    pub auto_compile: bool,
    pub live_edit: bool,
    pub recent: Vec<PathBuf>,
    pub ink_color: [u8; 3],
    pub laser_color: [u8; 3],
    /// Multiplies the size of the whole interface.
    pub ui_scale: f32,
    pub notes_size: f32,
    /// For a .tex presentation, notes written in the presenter also go into the source as \note{...}
    /// (and the slides rebuild). Off: they go into the .note.md file beside the presentation.
    pub notes_in_tex: bool,
    pub editor_size: f32,
    /// The LaTeX studio's editor wraps long lines at its width instead of scrolling sideways.
    pub editor_wrap: bool,
    /// What plays videos that are not GIFs: "auto", "ffmpeg", "vlc" or "off". Only tools the person
    /// installed are used; TeXora contains none.
    pub video_player: String,
    /// Where ffmpeg and VLC are, when not found by themselves; empty means "find it".
    pub ffmpeg_path: String,
    pub vlc_path: String,
    /// Look for a newer version when TeXora starts.
    pub check_updates: bool,
    /// A newer version the speaker chose to skip: its update notice is not shown again.
    pub skipped_update: String,
    /// The template chosen last for a new presentation.
    pub last_template: String,
    pub layout: Layout,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            keys: default_keys(),
            engine: Engine::Auto,
            auto_compile: true,
            live_edit: true,
            recent: vec![],
            ink_color: [214, 40, 40],
            laser_color: [235, 30, 30],
            ui_scale: 1.0,
            notes_size: 26.0,
            notes_in_tex: true,
            editor_size: 14.0,
            editor_wrap: false,
            video_player: "auto".to_string(),
            ffmpeg_path: String::new(),
            vlc_path: String::new(),
            check_updates: true,
            skipped_update: String::new(),
            last_template: String::new(),
            layout: Layout::default(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("TeXora")
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("TeXora")
}

impl Settings {
    fn path() -> PathBuf {
        std::env::var_os("TEXORA_CONFIG").map(PathBuf::from).unwrap_or_else(|| config_dir().join("settings.json"))
    }
    pub fn load() -> Self {
        let text = std::fs::read_to_string(Self::path()).ok();
        let parsed = text.as_deref().map(serde_json::from_str::<Settings>);
        // A settings file TeXora cannot read (for example one left by another program) is set aside,
        // never overwritten.
        if let Some(Err(_)) = &parsed {
            let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let _ = std::fs::rename(Self::path(), Self::path().with_file_name(format!("settings.unreadable-{stamp}.json")));
        }
        let mut s: Settings = parsed.and_then(Result::ok).unwrap_or_default();
        // Actions added by later versions get their default keys.
        for (a, k) in default_keys() {
            if !s.keys.iter().any(|(x, _)| *x == a) {
                s.keys.push((a, k));
            }
        }
        s
    }
    pub fn save(&self) -> Result<(), String> {
        let p = Self::path();
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&p, text).map_err(|e| e.to_string())?;
        // Only this user may read it.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
    /// Which action is already using this key, if any.
    pub fn conflict(&self, b: &KeyBind, except: Action) -> Option<Action> {
        self.keys.iter().find(|(a, ks)| *a != except && ks.contains(b)).map(|(a, _)| *a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keys_do_not_clash_and_all_exist() {
        let d = default_keys();
        assert_eq!(d.len(), 48);
        let mut seen = std::collections::HashSet::new();
        for (a, ks) in &d {
            assert!(!ks.is_empty(), "{a:?}");
            for k in ks {
                assert!(seen.insert(format!("{k:?}")), "{a:?} {k:?}");
                assert!(egui::Key::from_name(&k.key).is_some(), "unknown key {}", k.key);
            }
        }
    }

    #[test]
    fn conflict_finds_the_other_action() {
        let s = Settings::default();
        assert_eq!(s.conflict(&KeyBind::new("B"), Action::Pen), Some(Action::Blackout));
        assert_eq!(s.conflict(&KeyBind::new("B"), Action::Blackout), None);
    }
}
