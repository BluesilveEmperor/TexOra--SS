//! The left sidebar: contents and bookmarks, search, analytics, settings and the short guide.
use crate::analytics::{fmt_time, Session};
use crate::app::*;
use crate::icons::{self, labelled_button, sized_button, Icon};
use crate::settings::{default_keys, Action};
use crate::tabs::{section_slot_color, SECTION_COLORS};
use crate::ui_present::{engine_combo, markdown};
use egui::*;

const GUIDE: &str = include_str!("../assets/guide.md");

fn strong(s: &str, size: f32) -> RichText {
    RichText::new(s).size(size).family(FontFamily::Name("medium".into())).color(INK)
}

/// A named group with the same spacing everywhere.
fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(strong(title, 14.5));
}

impl App {
    pub fn left_sidebar(&mut self, ui: &mut Ui) {
        let Some(tab) = self.left else { return };
        // Whatever is inside must fit the sidebar; the sidebar never widens to suit its contents.
        let width = ui.available_width();
        ui.set_max_width(width);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (t, icon, tip) in [
                (LeftTab::Bookmarks, Icon::List, "Contents and bookmarks"),
                (LeftTab::Search, Icon::Search, "Search"),
                (LeftTab::Analytics, Icon::Chart, "Analytics"),
                (LeftTab::Settings, Icon::Gear, "Settings"),
                (LeftTab::Docs, Icon::Help, "Documentation"),
            ] {
                if sized_button(ui, icon, tab == t, tip, 32.0, 17.0).clicked() {
                    self.left = Some(t);
                }
            }
            ui.add_space(6.0);
            let title = match tab {
                LeftTab::Bookmarks => "Contents",
                LeftTab::Search => "Search",
                LeftTab::Analytics => "Analytics",
                LeftTab::Settings => "Settings",
                LeftTab::Docs => "Documentation",
            };
            ui.label(strong(title, 15.5));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if sized_button(ui, Icon::Close, false, "Close sidebar", 28.0, 14.0).clicked() {
                    self.left = None;
                    let _ = self.st.save();
                }
            });
        });
        ui.separator();
        match tab {
            LeftTab::Bookmarks => self.bookmarks_tab(ui),
            LeftTab::Search => self.search_tab(ui),
            LeftTab::Analytics => self.analytics_tab(ui),
            LeftTab::Settings => self.settings_tab(ui),
            LeftTab::Docs => {
                ScrollArea::vertical().id_salt("guide").auto_shrink([false, false]).show(ui, |ui| markdown(ui, GUIDE, 14.0));
            }
        }
    }

    fn bookmarks_tab(&mut self, ui: &mut Ui) {
        let mut go = None;
        let mut back = false;
        let mut drop_mark = None;
        let mut drop_ask = None;
        let mut recolor = None;
        ui.add_enabled_ui(!self.history.is_empty(), |ui| {
            back = labelled_button(ui, Icon::Undo, "Back to where I was", false).clicked();
        });
        ui.add_space(4.0);
        let outline = self.doc.as_ref().map(|d| d.outline.clone()).unwrap_or_default();
        let cur_page = self.deck.slides.get(self.cur).map(|s| s.page).unwrap_or(0);
        let titles: Vec<String> = self.deck.frames.iter().map(|f| f.title.clone()).collect();
        // PDFs without LaTeX source only have "Slide N" as a title, so it is not repeated.
        let name = |f: usize| match titles.get(f).filter(|t| !t.starts_with("Slide ")) {
            Some(t) => format!("{}  {t}", f + 1),
            None => format!("Slide {}", f + 1),
        };
        let first_of = |f: usize| self.deck.frames.get(f).map(|x| x.first);
        ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            section(ui, "Bookmarks");
            if self.side.bookmarks.is_empty() {
                ui.label(RichText::new("Press the bookmark button to mark the current slide.").size(13.0).color(MUTED));
            }
            for f in self.side.bookmarks.iter().copied() {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(14.0, 20.0), Sense::hover());
                    let [red, green, blue] = self.section_color(f);
                    icons::ribbon(ui.painter(), Rect::from_center_size(r.center(), vec2(10.0, 15.0)), Color32::from_rgb(red, green, blue));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if sized_button(ui, Icon::Close, false, "Remove bookmark", 24.0, 11.0).clicked() {
                            drop_mark = Some(f);
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            if ui.add(Button::new(RichText::new(name(f)).size(13.5)).frame(false).truncate()).clicked() {
                                go = first_of(f);
                            }
                        });
                    });
                });
            }
            ui.add_space(4.0);
            section(ui, "Questions");
            if self.side.questions.is_empty() {
                ui.label(RichText::new("Press the question button when someone asks about a slide.").size(13.0).color(MUTED));
            }
            for f in self.side.questions.iter().copied() {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(14.0, 20.0), Sense::hover());
                    icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(14.0, 14.0)), Icon::Ask, WARN);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if sized_button(ui, Icon::Close, false, "Remove question mark", 24.0, 11.0).clicked() {
                            drop_ask = Some(f);
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            if ui.add(Button::new(RichText::new(name(f)).size(13.5)).frame(false).truncate()).clicked() {
                                go = first_of(f);
                            }
                        });
                    });
                });
            }
            ui.add_space(4.0);
            ui.separator();
            section(ui, "Contents");
            if outline.is_empty() {
                ui.label(RichText::new("This PDF has no contents list.").size(13.0).color(MUTED));
            }
            for (i, o) in outline.iter().enumerate() {
                let here = cur_page >= o.page && outline.get(i + 1).map(|n| cur_page < n.page).unwrap_or(true);
                ui.horizontal(|ui| {
                    ui.add_space(o.depth as f32 * 16.0);
                    // Each section's colour dot chooses the colour of its bookmarks.
                    if let Some(at) = self.deck.sections.iter().position(|s| s.title == o.title) {
                        let color = self.side.section_colors.get(&o.title).copied().unwrap_or_else(|| section_slot_color(Some(at)));
                        let (r, resp) = ui.allocate_exact_size(vec2(16.0, 20.0), Sense::click());
                        ui.painter().circle_filled(r.center(), 5.5, Color32::from_rgb(color[0], color[1], color[2]));
                        let resp = resp.on_hover_text("Colour for this section's bookmarks");
                        Popup::menu(&resp).show(|ui| {
                            ui.horizontal(|ui| {
                                for c in SECTION_COLORS {
                                    let (rr, b) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
                                    ui.painter().circle_filled(rr.center(), 9.0, Color32::from_rgb(c[0], c[1], c[2]));
                                    if c == color {
                                        ui.painter().circle_stroke(rr.center(), 11.0, Stroke::new(1.5, INK));
                                    }
                                    if b.clicked() {
                                        recolor = Some((o.title.clone(), c));
                                        ui.close();
                                    }
                                }
                            });
                        });
                    }
                    let t = RichText::new(&o.title).color(if here { LEAF_DARK } else { INK });
                    if ui.add(Button::selectable(here, t)).clicked() {
                        go = self.deck.slide_for_page(o.page);
                    }
                });
            }
            if let Some(b) = self.deck.backup_start {
                ui.add_space(6.0);
                ui.separator();
                if ui.button(format!("Backup slides (from slide {})", b + 1)).clicked() {
                    go = self.deck.frames.get(b).map(|f| f.first);
                }
            }
        });
        if back {
            let c = ui.ctx().clone();
            self.run(Action::Back, &c);
        }
        let changed = drop_mark.is_some() || drop_ask.is_some() || recolor.is_some();
        if let Some(f) = drop_mark {
            self.side.bookmarks.remove(&f);
        }
        if let Some(f) = drop_ask {
            self.side.questions.remove(&f);
            self.session.questions.retain(|q| *q != f);
        }
        if let Some((title, c)) = recolor {
            self.side.section_colors.insert(title, c);
        }
        if changed {
            self.save_side();
        }
        if let Some(s) = go {
            self.jump(s);
        }
    }

    fn search_tab(&mut self, ui: &mut Ui) {
        let mut go = None;
        let r = ui.add(TextEdit::singleline(&mut self.search_q).hint_text("Word or phrase").desired_width(f32::INFINITY));
        if self.focus_search {
            self.focus_search = false;
            r.request_focus();
        }
        if r.changed() {
            self.run_search();
        }
        let groups = group_hits(&self.deck, self.doc.as_ref(), &self.hits, &self.search_q);
        ui.label(RichText::new(format!("{} place(s) in slides and notes", groups.len())).size(12.5).color(MUTED));
        ui.separator();
        let cur = self.frame();
        ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            for g in &groups {
                let t = RichText::new(&g.text).size(13.5).color(if g.frame == cur { LEAF_DARK } else { INK });
                if ui.add(Button::new(t).frame(false).wrap()).clicked() {
                    go = Some(g.slide);
                }
            }
        });
        if let Some(s) = go {
            self.jump(s);
        }
    }

    fn analytics_tab(&mut self, ui: &mut Ui) {
        let cur = self.session.clone();
        let previous = self.side.sessions.last().cloned();
        let (session, prev, live) = if cur.total_secs > 0.0 { (cur, previous, true) } else { (previous.clone().unwrap_or_default(), None, false) };
        let titles: Vec<String> = self.deck.frames.iter().map(|f| f.title.clone()).collect();
        let talk = self.deck.talk_frames();
        if session.total_secs <= 0.0 {
            ui.label("Nothing recorded yet. Start the timer and go through your slides; TeXora notes the time on each one.");
            if !self.side.questions.is_empty() {
                let list: Vec<String> = self.side.questions.iter().map(|f| (f + 1).to_string()).collect();
                ui.label(format!("Questions marked on slide(s) {}.", list.join(", ")));
            }
            return;
        }
        ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                stat(ui, "Total time", &fmt_time(session.total_secs));
                let shown = session.per_frame.iter().filter(|t| **t >= 1.0).count();
                stat(ui, "Slides shown", &format!("{shown} of {talk}"));
                stat(ui, "Average", &fmt_time(session.total_secs / shown.max(1) as f32));
                stat(ui, "Questions", &self.side.questions.len().to_string());
            });
            ui.label(RichText::new(if live { "This run" } else { "Last saved run" }).size(12.5).color(MUTED));
            section(ui, "Seconds on each slide");
            time_chart(ui, &session, prev.as_ref(), &titles, talk);
            section(ui, "What stands out");
            for i in session.insights(talk, self.side.target_minutes) {
                ui.label(RichText::new(i).size(13.5));
            }
            ui.add_space(6.0);
            CollapsingHeader::new("Table").show(ui, |ui| {
                Grid::new("analytics-table").striped(true).show(ui, |ui| {
                    ui.strong("Slide");
                    ui.strong("Time");
                    ui.strong("Visits");
                    ui.end_row();
                    for (i, t) in session.per_frame.iter().enumerate() {
                        ui.label((i + 1).to_string()).on_hover_text(titles.get(i).cloned().unwrap_or_default());
                        ui.label(fmt_time(*t));
                        ui.label(session.visits.get(i).copied().unwrap_or(0).to_string());
                        ui.end_row();
                    }
                });
            });
            if labelled_button(ui, Icon::Save, "Save as CSV", false).clicked() {
                if let Some(p) = rfd::FileDialog::new().set_file_name("talk-analytics.csv").save_file() {
                    if let Err(e) = std::fs::write(p, session.csv(&titles)) {
                        self.status = format!("Could not save: {e}");
                    }
                }
            }
        });
    }

    fn settings_tab(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            ui.label(RichText::new("Click a heading to open or close its section.").size(12.0).color(MUTED));
            // Each section remembers whether it is open while TeXora runs.
            let group = |ui: &mut Ui, title: &str, open: bool, body: &mut dyn FnMut(&mut Ui)| {
                ui.add_space(4.0);
                CollapsingHeader::new(strong(title, 14.5)).id_salt(("settings", title)).default_open(open).show(ui, |ui| body(ui));
            };
            group(ui, "Text size", true, &mut |ui| {
                Grid::new("sizes").num_columns(2).show(ui, |ui| {
                    ui.label("Interface");
                    if ui.add(Slider::new(&mut self.st.ui_scale, 0.8..=1.6).step_by(0.05).fixed_decimals(2)).changed() {
                        ui.ctx().set_zoom_factor(self.st.ui_scale);
                    }
                    ui.end_row();
                    ui.label("Speaker notes");
                    ui.add(Slider::new(&mut self.st.notes_size, 14.0..=48.0).step_by(1.0).suffix(" pt"));
                    ui.end_row();
                    ui.label("LaTeX editor");
                    ui.add(Slider::new(&mut self.st.editor_size, 11.0..=22.0).step_by(1.0).suffix(" pt"));
                    ui.end_row();
                });
            });
            group(ui, "LaTeX", true, &mut |ui| {
                ui.horizontal(|ui| {
                    ui.label("Engine");
                    engine_combo(ui, &mut self.st.engine);
                });
                ui.checkbox(&mut self.st.auto_compile, "Rebuild when the source changes");
                ui.checkbox(&mut self.st.live_edit, "Live editing: save and rebuild as you type");
            });
            // Tools > Check required packages opens this section.
            let force = std::mem::take(&mut self.show_packages).then_some(true);
            ui.add_space(4.0);
            CollapsingHeader::new(strong("Required LaTeX packages", 14.5)).id_salt(("settings", "Required LaTeX packages")).default_open(false).open(force).show(ui, |ui| self.required_packages(ui));
            group(ui, "Video", false, &mut |ui| self.video_settings(ui));
            group(ui, "Speaker notes", false, &mut |ui| {
                // A LaTeX presentation keeps its notes in the source, as \note{} in each slide.
                let place = match (&self.src, self.st.notes_in_tex) {
                    (Some(s), true) => Some(s.path.clone()),
                    _ => self.deck_path.as_ref().map(|p| crate::notes::notes_file(p)),
                };
                ui.label(RichText::new("TeXora shows notes from the PDF's notes pages, from \\note{} in the LaTeX source, or from a Markdown file beside the presentation with one ## Slide N section per slide. Write that file in any text editor; TeXora reads it when the presentation opens.").size(12.5));
                if self.src.is_some() {
                    if ui.checkbox(&mut self.st.notes_in_tex, "Save notes in the .tex file, as \\note{...}").changed() {
                        self.save_layout();
                    }
                    let where_to = if self.st.notes_in_tex {
                        "This presentation's notes are saved in its LaTeX source, as \\note{...} in each slide, so the presenter and the LaTeX studio always show the same notes. A new note shows at once; the slides rebuild in the background:"
                    } else {
                        "This presentation's notes are saved in the Markdown file beside it; the .tex source is not changed. A note there shows instead of the slide's \\note:"
                    };
                    ui.label(RichText::new(where_to).size(12.5).color(LEAF_DARK));
                }
                match place {
                    Some(p) => {
                        ui.add(Label::new(RichText::new(p.display().to_string()).monospace().size(11.5).color(INK)).wrap().selectable(true));
                        ui.horizontal(|ui| {
                            if ui.small_button("Copy path").clicked() {
                                ui.ctx().copy_text(p.display().to_string());
                                self.status = "Copied.".to_string();
                            }
                            if ui.small_button("Show in folder").clicked() {
                                if let Some(dir) = p.parent() {
                                    let _ = crate::app::open_folder(dir);
                                }
                            }
                        });
                    }
                    None => {
                        ui.label(RichText::new("Open a presentation to see where its notes are saved.").size(12.5).color(MUTED));
                    }
                }
            });
            group(ui, "Updates", false, &mut |ui| {
                ui.checkbox(&mut self.st.check_updates, "Look for updates by itself (at start and every few hours)");
            });
            group(ui, "Keyboard shortcuts", false, &mut |ui| {
                ui.label(RichText::new("Click a key, then press the new one. Esc cancels.").size(12.5).color(MUTED));
                let keys = self.st.keys.clone();
                let column = ((ui.available_width() - 20.0) / 2.0).max(80.0);
                Grid::new("keys").striped(true).num_columns(2).max_col_width(column).show(ui, |ui| {
                    for (a, ks) in keys {
                        ui.label(a.label());
                        let txt = if self.capture == Some(a) {
                            "Press a key…".to_string()
                        } else {
                            ks.iter().map(|k| k.text().replace("Arrow", "")).collect::<Vec<_>>().join(", ")
                        };
                        if ui.add(Button::new(txt).wrap()).clicked() {
                            self.capture = Some(a);
                        }
                        ui.end_row();
                    }
                });
                if ui.button("Restore default keys").clicked() {
                    self.st.keys = default_keys();
                    let _ = self.st.save();
                }
            });
        });
    }

    /// Every package, class, theme and program the open LaTeX source needs, and whether it is here.
    fn required_packages(&mut self, ui: &mut Ui) {
        if self.src.is_none() {
            ui.label(RichText::new("Open a LaTeX presentation to see the packages it needs.").size(13.0).color(MUTED));
            return;
        }
        let checking = self.req_rx.is_some();
        ui.horizontal(|ui| {
            if ui.add_enabled(!checking, Button::new(if self.reqs.is_empty() { "Check now" } else { "Check again" })).clicked() {
                self.check_packages();
            }
            if ui.button("Open in LaTeX studio").clicked() {
                let c = ui.ctx().clone();
                self.run(Action::CheckPackages, &c);
            }
        });
        if checking {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Checking what is installed…").size(13.0).color(MUTED));
            });
        }
        if let Some(w) = &self.req_warning {
            ui.label(RichText::new(w).size(13.0).color(BAD));
        }
        if self.reqs.is_empty() {
            if !checking {
                ui.label(RichText::new("Not checked yet.").size(13.0).color(MUTED));
            }
            return;
        }
        let missing = self.reqs.iter().filter(|r| !r.found).count();
        let summary = if missing == 0 {
            RichText::new(format!("All {} are installed.", self.reqs.len())).color(LEAF_DARK)
        } else {
            RichText::new(format!("{missing} of {} need downloading.", self.reqs.len())).color(BAD)
        };
        ui.label(summary.size(13.5));
        let mut list = self.reqs.clone();
        list.sort_by_key(|r| r.found);
        // One row per requirement across the whole width: name and kind on the left, status on the right.
        for (i, r) in list.iter().enumerate() {
            let fill = if i % 2 == 0 { Color32::from_rgb(244, 243, 240) } else { Color32::TRANSPARENT };
            Frame::new().fill(fill).corner_radius(4.0).inner_margin(Margin::symmetric(6, 4)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (text, color) = if r.found { ("Installed", LEAF_DARK) } else { ("Needs download", BAD) };
                    let status = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(13.0), color);
                    let room = (ui.available_width() - status.size().x - 12.0).max(60.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(room);
                        ui.add(Label::new(RichText::new(&r.name).size(13.5).color(INK)).wrap());
                        ui.label(RichText::new(r.kind.label()).size(11.5).color(MUTED));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(text).size(13.0).color(color));
                    });
                });
                if !r.found && !r.hint.is_empty() {
                    ui.add(Label::new(RichText::new(&r.hint).monospace().size(11.5).color(MUTED)).wrap());
                }
            });
        }
    }

}

fn stat(ui: &mut Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.label(RichText::new(label).size(12.0).color(MUTED));
        ui.label(RichText::new(value).size(20.0).color(INK));
    });
    ui.add_space(8.0);
}

pub struct HitGroup {
    pub slide: usize,
    pub frame: usize,
    pub in_notes: bool,
    pub text: String,
}

/// One result per slide, rather than one per overlay step or repeated footer.
pub fn group_hits(deck: &crate::deck::Deck, doc: Option<&crate::pdf::DocInfo>, hits: &[crate::pdf::SearchHit], q: &str) -> Vec<HitGroup> {
    let split = deck.slides.first().map(|s| s.region != crate::pdf::Region::Full).unwrap_or(false);
    let needle = q.trim().to_lowercase();
    let mut out: Vec<HitGroup> = vec![];
    for h in hits {
        let Some(si) = deck.slide_for_page(h.page) else { continue };
        let s = &deck.slides[si];
        let in_notes = split && h.rects.first().map(|r| r[0] >= 0.5).unwrap_or(false);
        if out.iter().any(|g| g.frame == s.frame && g.in_notes == in_notes) {
            continue;
        }
        let page = doc.and_then(|d| d.pages.get(h.page));
        let text = page.map(|p| {
            if !split {
                &p.text_full
            } else if in_notes {
                &p.text_right
            } else {
                &p.text_left
            }
        });
        let line = text.and_then(|t| t.lines().find(|l| l.to_lowercase().contains(&needle))).unwrap_or(h.snippet.as_str());
        let short: String = line.trim().chars().take(70).collect();
        let place = if in_notes { "notes" } else { "slide" };
        out.push(HitGroup { slide: si, frame: s.frame, in_notes, text: format!("Slide {} {place}: {short}", s.frame + 1) });
    }
    out
}

/// One bar per slide, with a tick for the previous run when there is one. Hover shows the numbers.
fn time_chart(ui: &mut Ui, s: &Session, prev: Option<&Session>, titles: &[String], talk: usize) {
    let width = ui.available_width().max(220.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 170.0), Sense::hover());
    let p = ui.painter_at(rect);
    let n = s.per_frame.len().max(talk).max(1);
    let plot = Rect::from_min_max(rect.min + vec2(32.0, 18.0), rect.max - vec2(4.0, 18.0));
    let prev_max = prev.map(|x| x.per_frame.iter().copied().fold(0.0, f32::max)).unwrap_or(0.0);
    let max = s.per_frame.iter().copied().fold(1.0, f32::max).max(prev_max);
    let step = [10.0, 30.0, 60.0, 120.0, 300.0, 600.0].into_iter().find(|v| max / v <= 4.0).unwrap_or(1200.0);
    let y_of = |v: f32| plot.bottom() - v / (max * 1.05) * plot.height();
    let mut v = 0.0;
    while v <= max {
        let y = y_of(v);
        p.hline(plot.x_range(), y, Stroke::new(1.0, Color32::from_gray(232)));
        p.text(pos2(plot.left() - 6.0, y), Align2::RIGHT_CENTER, format!("{v:.0}"), FontId::proportional(11.0), MUTED);
        v += step;
    }
    let slot = plot.width() / n as f32;
    let bar = (slot - 2.0).clamp(2.0, 22.0);
    let hover = resp.hover_pos();
    let mut tip = None;
    let every = ((26.0 / slot).ceil() as usize).max(1);
    for i in 0..n {
        let t = s.per_frame.get(i).copied().unwrap_or(0.0);
        let cx = plot.left() + slot * (i as f32 + 0.5);
        let r = Rect::from_min_max(pos2(cx - bar / 2.0, y_of(t)), pos2(cx + bar / 2.0, plot.bottom()));
        let hot = hover.map(|h| (h.x - cx).abs() <= slot / 2.0).unwrap_or(false);
        let fill = if i >= talk { Color32::from_gray(170) } else { LEAF };
        let fill = if hot { LEAF_DARK } else { fill };
        p.rect_filled(r, CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, fill);
        if let Some(pv) = prev.and_then(|x| x.per_frame.get(i)) {
            p.hline(r.x_range(), y_of(*pv), Stroke::new(2.0, INK));
        }
        if (i + 1) % every == 0 || every == 1 {
            p.text(pos2(cx, plot.bottom() + 3.0), Align2::CENTER_TOP, (i + 1).to_string(), FontId::proportional(11.0), MUTED);
        }
        if hot {
            let title = titles.get(i).cloned().unwrap_or_default();
            let mut text = format!("Slide {}: {title}\n{}", i + 1, fmt_time(t));
            if let Some(pv) = prev.and_then(|x| x.per_frame.get(i)) {
                text += &format!(" (previous run {})", fmt_time(*pv));
            }
            tip = Some(text);
        }
    }
    if prev.is_some() {
        let x = plot.right() - 136.0;
        p.rect_filled(Rect::from_min_size(pos2(x, rect.top() + 2.0), vec2(10.0, 10.0)), 2.0, LEAF);
        p.text(pos2(x + 14.0, rect.top() + 7.0), Align2::LEFT_CENTER, "This run", FontId::proportional(11.5), MUTED);
        p.hline((x + 70.0)..=(x + 82.0), rect.top() + 7.0, Stroke::new(2.0, INK));
        p.text(pos2(x + 86.0, rect.top() + 7.0), Align2::LEFT_CENTER, "Previous", FontId::proportional(11.5), MUTED);
    }
    if let Some(t) = tip {
        resp.on_hover_text(t);
    }
}
