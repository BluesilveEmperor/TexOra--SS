//! Records how a talk or rehearsal went, slide by slide, plus what is saved beside each deck.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum InkKind {
    Pen,
    Highlighter,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Stroke {
    pub kind: InkKind,
    pub color: [u8; 3],
    pub width: f32,
    pub points: Vec<[f32; 2]>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub started_unix: u64,
    pub total_secs: f32,
    pub per_frame: Vec<f32>,
    pub visits: Vec<u32>,
    /// Slides marked during the talk because someone asked about them.
    #[serde(default)]
    pub questions: Vec<usize>,
    #[serde(default)]
    pub ink_strokes: u32,
}

impl Session {
    pub fn tick(&mut self, frame: usize, dt: f32) {
        self.grow(frame);
        self.per_frame[frame] += dt;
        self.total_secs += dt;
    }
    pub fn visit(&mut self, frame: usize) {
        self.grow(frame);
        self.visits[frame] += 1;
    }
    fn grow(&mut self, frame: usize) {
        if self.per_frame.len() <= frame {
            self.per_frame.resize(frame + 1, 0.0);
            self.visits.resize(frame + 1, 0);
        }
    }
    /// Plain-language findings, most useful first.
    pub fn insights(&self, talk_frames: usize, target_min: f32) -> Vec<String> {
        let mut out = vec![];
        let times: Vec<(usize, f32)> = self.per_frame.iter().copied().enumerate().take(talk_frames).collect();
        let shown = times.iter().filter(|(_, t)| *t >= 1.0).count();
        if shown == 0 {
            return out;
        }
        if target_min > 0.0 {
            let d = self.total_secs / 60.0 - target_min;
            if d.abs() < 0.5 {
                out.push("You finished on your target time.".to_string());
            } else {
                out.push(format!("You ran {:.1} minutes {} your target.", d.abs(), if d > 0.0 { "over" } else { "under" }));
            }
        }
        let mean = times.iter().map(|t| t.1).sum::<f32>() / shown as f32;
        if let Some((i, t)) = times.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)) {
            if t > mean * 1.5 && t > 30.0 {
                out.push(format!("Slide {} took {} ({:.1} times your average).", i + 1, fmt_time(t), t / mean));
            }
        }
        let rushed: Vec<String> = times.iter().filter(|(_, t)| *t >= 0.5 && *t < 4.0).map(|(i, _)| (i + 1).to_string()).collect();
        if !rushed.is_empty() {
            out.push(format!("Very quick on slide(s) {}. Skip them or say why they matter.", rushed.join(", ")));
        }
        let skipped: Vec<String> = (0..talk_frames)
            .filter(|i| self.per_frame.get(*i).copied().unwrap_or(0.0) < 0.5)
            .map(|i| (i + 1).to_string())
            .collect();
        if !skipped.is_empty() {
            out.push(format!("Not shown: slide(s) {}.", skipped.join(", ")));
        }
        let revisit: Vec<String> =
            self.visits.iter().enumerate().take(talk_frames).filter(|(_, v)| **v > 1).map(|(i, _)| (i + 1).to_string()).collect();
        if !revisit.is_empty() {
            out.push(format!("You came back to slide(s) {}.", revisit.join(", ")));
        }
        if !self.questions.is_empty() {
            let q: Vec<String> = self.questions.iter().map(|f| (f + 1).to_string()).collect();
            out.push(format!("Questions came up on slide(s) {}. Worth a clearer explanation there.", q.join(", ")));
        }
        out
    }
    pub fn csv(&self, titles: &[String]) -> String {
        let mut s = String::from("slide,title,seconds,visits,questions\n");
        for (i, t) in self.per_frame.iter().enumerate() {
            let title = titles.get(i).map(|x| x.replace('"', "\"\"")).unwrap_or_default();
            let q = self.questions.iter().filter(|f| **f == i).count();
            s += &format!("{},\"{}\",{:.1},{},{}\n", i + 1, title, t, self.visits.get(i).copied().unwrap_or(0), q);
        }
        s
    }
}

pub fn fmt_time(secs: f32) -> String {
    let s = secs.max(0.0).round() as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

/// Everything TeXora remembers about one deck, kept in a small file beside it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sidecar {
    pub ink: BTreeMap<String, Vec<Stroke>>,
    pub target_minutes: f32,
    pub sessions: Vec<Session>,
    /// Slides with a bookmark. Each shows in the colour of its section.
    pub bookmarks: BTreeSet<usize>,
    /// Slides where a question came up. These stay even if the timer never ran.
    pub questions: BTreeSet<usize>,
    /// Colours picked for sections, by section title.
    pub section_colors: BTreeMap<String, [u8; 3]>,
    pub notes_mode: crate::deck::NotesMode,
}

impl Sidecar {
    pub fn path_for(pdf: &std::path::Path) -> std::path::PathBuf {
        pdf.with_extension("texora.json")
    }
    pub fn load(pdf: &std::path::Path) -> Self {
        // Decks used before the app was renamed keep their ink and timings.
        std::fs::read_to_string(Self::path_for(pdf))
            .or_else(|_| std::fs::read_to_string(pdf.with_extension("texpress.json")))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, pdf: &std::path::Path) -> Result<(), String> {
        let t = serde_json::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::path_for(pdf), t).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        let mut s = Session::default();
        for (f, t) in [(0, 20.0), (1, 200.0), (2, 2.0)] {
            s.visit(f);
            s.tick(f, t);
        }
        s.visit(1);
        s.questions.push(1);
        s
    }

    #[test]
    fn totals_and_visits_add_up() {
        let s = session();
        assert_eq!(s.total_secs, 222.0);
        assert_eq!(s.visits, vec![1, 2, 1]);
    }

    #[test]
    fn insights_name_the_slow_quick_skipped_and_questioned_slides() {
        let all = session().insights(4, 2.0).join(" | ");
        assert!(all.contains("1.7 minutes over"), "{all}");
        assert!(all.contains("Slide 2 took 03:20"), "{all}");
        assert!(all.contains("Very quick on slide(s) 3"), "{all}");
        assert!(all.contains("Not shown: slide(s) 4"), "{all}");
        assert!(all.contains("Questions came up on slide(s) 2"), "{all}");
    }

    #[test]
    fn backup_slides_do_not_count_as_skipped() {
        let mut s = session();
        s.tick(5, 0.0);
        let all = s.insights(3, 0.0).join(" | ");
        assert!(!all.contains("Not shown"), "{all}");
    }

    #[test]
    fn csv_has_one_row_per_slide_and_escapes_quotes() {
        let c = session().csv(&["A \"b\"".to_string()]);
        assert_eq!(c.lines().count(), 4);
        assert!(c.contains("\"A \"\"b\"\"\""));
    }

    #[test]
    fn ink_timings_and_length_survive_closing_the_deck() {
        let pdf = std::env::temp_dir().join(format!("txp-side-{}.pdf", std::process::id()));
        let mut side = Sidecar { target_minutes: 25.0, ..Default::default() };
        side.ink.insert("3".into(), vec![Stroke { kind: InkKind::Pen, color: [1, 2, 3], width: 0.01, points: vec![[0.1, 0.2]] }]);
        side.sessions.push(session());
        side.save(&pdf).unwrap();
        let back = Sidecar::load(&pdf);
        assert_eq!(back.target_minutes, 25.0);
        assert_eq!(back.ink["3"][0].color, [1, 2, 3]);
        assert_eq!(back.sessions[0], session());
    }

    #[test]
    fn bookmarks_questions_and_colours_survive_closing_the_deck() {
        let pdf = std::env::temp_dir().join(format!("txp-marks-{}.pdf", std::process::id()));
        let mut side = Sidecar::default();
        side.bookmarks.insert(4);
        side.questions.insert(7);
        side.section_colors.insert("Approach".into(), [1, 2, 3]);
        side.save(&pdf).unwrap();
        let back = Sidecar::load(&pdf);
        assert!(back.bookmarks.contains(&4) && back.questions.contains(&7));
        assert_eq!(back.section_colors["Approach"], [1, 2, 3]);
    }

    #[test]
    fn time_format() {
        assert_eq!(fmt_time(65.0), "01:05");
        assert_eq!(fmt_time(3661.0), "1:01:01");
    }
}
