//! Theme, the top toolbar and the application menus.
use crate::app::*;
use crate::deck::NotesMode;
use crate::icons::{self, icon_button, logo, sized_button, Icon};
use crate::settings::Action;
use crate::ui_present::{markdown, sid};
use crate::update;
use egui::*;

const ABOUT_TEXT: &str = include_str!("../assets/about.md");

pub fn apply_theme(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in [
        ("Inter", include_bytes!("../assets/fonts/inter-400.ttf").as_slice()),
        ("InterMedium", include_bytes!("../assets/fonts/inter-500.ttf").as_slice()),
        ("InterBold", include_bytes!("../assets/fonts/inter-600.ttf").as_slice()),
    ] {
        fonts.font_data.insert(name.to_string(), std::sync::Arc::new(FontData::from_static(bytes)));
    }
    // Inter here covers Latin text. Hack comes next so arrows and maths signs in notes
    // answers show as themselves, not as empty boxes.
    let mut fallback = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    fallback.extend(fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    let mut body = vec!["Inter".to_string()];
    body.extend(fallback.iter().cloned());
    fonts.families.insert(FontFamily::Proportional, body);
    for (family, face) in [("medium", "InterMedium"), ("strong", "InterBold")] {
        let mut list = vec![face.to_string()];
        list.extend(fallback.iter().cloned());
        fonts.families.insert(FontFamily::Name(family.into()), list);
    }
    ctx.set_fonts(fonts);
    ctx.set_theme(Theme::Light);
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
    ctx.all_styles_mut(|s| {
        use FontFamily::{Monospace, Proportional};
        s.text_styles = [
            (TextStyle::Small, FontId::new(12.5, Proportional)),
            (TextStyle::Body, FontId::new(15.0, Proportional)),
            (TextStyle::Button, FontId::new(14.5, Proportional)),
            (TextStyle::Heading, FontId::new(21.0, FontFamily::Name("strong".into()))),
            (TextStyle::Monospace, FontId::new(14.0, Monospace)),
        ]
        .into();
        s.spacing.item_spacing = vec2(8.0, 7.0);
        s.spacing.button_padding = vec2(10.0, 5.0);
        s.spacing.interact_size.y = 28.0;
        s.spacing.menu_margin = Margin::same(8);
        let v = &mut s.visuals;
        v.panel_fill = BG;
        v.window_fill = SURFACE;
        v.extreme_bg_color = Color32::WHITE;
        v.faint_bg_color = Color32::from_rgb(240, 239, 235);
        v.selection.bg_fill = Color32::from_rgb(190, 220, 180);
        v.selection.stroke.color = LEAF_DARK;
        v.hyperlink_color = LEAF;
        v.window_corner_radius = CornerRadius::same(10);
        v.menu_corner_radius = CornerRadius::same(8);
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open, &mut v.widgets.noninteractive] {
            w.corner_radius = CornerRadius::same(6);
        }
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(236, 235, 231);
        v.widgets.inactive.bg_fill = Color32::from_rgb(232, 231, 227);
        v.widgets.hovered.weak_bg_fill = LEAF_SOFT;
        v.widgets.hovered.bg_fill = LEAF_SOFT;
        v.widgets.active.weak_bg_fill = Color32::from_rgb(205, 226, 196);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    });
}

fn divider(ui: &mut Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(9.0, 26.0), Sense::hover());
    ui.painter().vline(r.center().x, r.y_range(), Stroke::new(1.0, LINE));
}

impl App {
    fn tip(&self, a: Action) -> String {
        match self.st.keys.iter().find(|(x, _)| *x == a).and_then(|(_, k)| k.first()) {
            Some(k) => format!("{} ({})", a.label(), k.text()),
            None => a.label().to_string(),
        }
    }

    fn act(&mut self, ui: &mut Ui, icon: Icon, active: bool, a: Action) {
        let tip = self.tip(a);
        if icon_button(ui, icon, active, &tip).clicked() {
            let ctx = ui.ctx().clone();
            self.run(a, &ctx);
        }
    }

    /// The top row, as in a browser: menus, tabs, then the tools that belong to the whole window.
    pub fn tab_bar(&mut self, ui: &mut Ui) {
        let strip = Color32::from_rgb(232, 231, 227);
        let frame = Frame::new().fill(strip).inner_margin(Margin::symmetric(8, 0));
        Panel::top("tab-bar").exact_size(38.0).frame(frame).show_separator_line(false).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                ui.style_mut().interaction.selectable_labels = false;
                let w = ui.available_width();
                let has_doc = self.doc.is_some();

                let (mark, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
                logo(ui, mark.shrink(2.0));
                if resp.on_hover_text(self.tip(Action::StartPage)).clicked() {
                    self.go_home();
                }
                ui.add_space(5.0);
                if w > 1000.0 {
                    // A menu bar: while one menu is open, pointing at another opens it.
                    // The bar fills the width it is given, so it gets the width its menus used last time.
                    let key = sid("menu-bar-width");
                    let width: f32 = ui.data(|d| d.get_temp(key)).unwrap_or(250.0);
                    ui.allocate_ui(vec2(width, 32.0), |ui| {
                        MenuBar::new().style(|_: &mut Style| {}).ui(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = 1.0;
                            let start = ui.cursor().left();
                            self.menus(ui);
                            let used = ui.cursor().left() - start;
                            ui.data_mut(|d| d.insert_temp(key, used.max(40.0)));
                        });
                    });
                } else {
                    ui.menu_button("Menu", |ui| self.menus(ui));
                }
                ui.add_space(8.0);

                // Room left for the tabs: the buttons on the right and the version at the end.
                let room = (ui.available_width() - 8.0 * 32.0 - 100.0).max(120.0);
                self.tab_strip(ui, room);
                if sized_button(ui, Icon::Plus, false, "Go to Home", 28.0, 14.0).clicked() {
                    self.go_home();
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    // The version, at the right end of the title row; a click opens About.
                    let version = Label::new(RichText::new(format!("v{}", crate::update::VERSION)).size(12.5).color(MUTED)).sense(Sense::click());
                    if ui.add(version).on_hover_cursor(CursorIcon::PointingHand).on_hover_text("About this version").clicked() {
                        self.about_open = true;
                    }
                    ui.add_space(6.0);
                    self.help_menu(ui);
                    let settings = self.left == Some(LeftTab::Settings);
                    self.act(ui, Icon::Gear, settings, Action::Settings);
                    ui.add_enabled_ui(has_doc, |ui| {
                        self.act(ui, Icon::Chart, self.left == Some(LeftTab::Analytics), Action::Analytics);
                        self.act(ui, Icon::Search, self.left == Some(LeftTab::Search), Action::Search);
                        self.act(ui, Icon::List, self.left == Some(LeftTab::Bookmarks), Action::Outline);
                        self.act(ui, Icon::Ask, self.side.questions.contains(&self.frame()), Action::MarkQuestion);
                        self.bookmark_button(ui);
                    });
                });
            });
        });
    }

    fn tab_strip(&mut self, ui: &mut Ui, room: f32) {
        let n = self.tabs.len().max(1);
        let width = (room / n as f32).clamp(76.0, 200.0);
        let mut pick = None;
        for i in 0..self.tabs.len() {
            let id = self.tabs[i].id;
            let here = i == self.active;
            let home = self.tab_is_home(i);
            let title = self.tab_title(i);
            let (rect, resp) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
            let shape = CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 };
            let cell = Rect::from_min_max(rect.min + vec2(1.0, 2.0), pos2(rect.right() - 1.0, ui.max_rect().bottom()));
            if here {
                ui.painter().rect_filled(cell, shape, SURFACE);
            } else if resp.hovered() {
                ui.painter().rect_filled(cell, shape, Color32::from_rgb(242, 241, 238));
            }
            let color = if here { INK } else { MUTED };
            let glyph = Rect::from_center_size(pos2(rect.left() + 17.0, cell.center().y), Vec2::splat(15.0));
            icons::paint(ui.painter(), glyph, if home { Icon::Home } else { Icon::Pdf }, color);
            let room_for_text = ((width - 54.0) / 7.2).max(3.0) as usize;
            let shown: String = if title.chars().count() > room_for_text {
                title.chars().take(room_for_text.saturating_sub(1)).chain(['…']).collect()
            } else {
                title.clone()
            };
            ui.painter().text(pos2(rect.left() + 30.0, cell.center().y), Align2::LEFT_CENTER, shown, FontId::proportional(13.5), color);
            let button = Rect::from_center_size(pos2(rect.right() - 15.0, cell.center().y), Vec2::splat(22.0));
            let mut slot = ui.new_child(UiBuilder::new().max_rect(button));
            let tip = if self.closing_ends_app(id) { "Close TeXora" } else { "Close tab" };
            if sized_button(&mut slot, Icon::Close, false, tip, 22.0, 11.0).clicked() {
                self.close_request = Some(id);
                ui.ctx().request_repaint();
            } else if resp.on_hover_text(title).clicked() {
                pick = Some(i);
            }
        }
        if let Some(i) = pick {
            self.activate(i);
        }
    }

    /// Adds a bookmark to the current slide, in the colour of its section.
    fn bookmark_button(&mut self, ui: &mut Ui) {
        let f = self.frame();
        let on = self.side.bookmarks.contains(&f);
        let tip = self.tip(Action::AddBookmark);
        let resp = icon_button(ui, Icon::Bookmark, on, &tip);
        if on {
            let [r, g, b] = self.section_color(f);
            let area = Rect::from_center_size(resp.rect.center(), vec2(11.0, 16.0));
            icons::ribbon(ui.painter(), area, Color32::from_rgb(r, g, b));
        }
        if resp.clicked() {
            let c = ui.ctx().clone();
            self.run(Action::AddBookmark, &c);
        }
    }

    /// The second row: everything that acts on the open presentation.
    pub fn top_bar(&mut self, ui: &mut Ui) {
        let frame = Frame::new().fill(SURFACE).inner_margin(Margin::symmetric(8, 4)).stroke(Stroke::new(1.0, LINE));
        Panel::top("top-bar").exact_size(42.0).frame(frame).show_separator_line(false).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                ui.style_mut().interaction.selectable_labels = false;
                let w = ui.available_width();
                let has_doc = self.doc.is_some();
                let presenting = matches!(self.view, View::Presenter | View::Split);

                ui.add_enabled_ui(has_doc, |ui| {
                    self.view_group(ui);
                    if presenting {
                        divider(ui);
                        self.act(ui, Icon::Windows, self.audience_open, Action::Present);
                        self.act(ui, Icon::Black, self.screen == Screen::Black, Action::Blackout);
                        self.act(ui, Icon::White, self.screen == Screen::White, Action::Whiteboard);
                        self.act(ui, Icon::Freeze, self.frozen.is_some(), Action::Freeze);
                        divider(ui);
                        self.note_buttons(ui);
                        if w > 960.0 {
                            self.ink_group(ui);
                        } else {
                            ui.menu_button("Ink", |ui| {
                                ui.horizontal(|ui| self.ink_group(ui));
                            });
                        }
                    }
                    if self.view != View::Overview {
                        divider(ui);
                        self.zoom_group(ui);
                    }
                    if has_doc {
                        divider(ui);
                        self.nav_group(ui);
                    }
                });
            });
        });
    }

    /// First, previous, the slide number, next and last.
    fn nav_group(&mut self, ui: &mut Ui) {
        let last = self.deck.slides.len().saturating_sub(1);
        let (start, end) = (self.cur == 0, self.cur >= last);
        ui.add_enabled_ui(!start, |ui| {
            self.act(ui, Icon::First, false, Action::First);
            self.act(ui, Icon::Prev, false, Action::Prev);
        });
        self.slide_box(ui);
        ui.add_enabled_ui(!end, |ui| {
            self.act(ui, Icon::Next, false, Action::Next);
            self.act(ui, Icon::Last, false, Action::Last);
        });
    }

    /// The "?" menu at the far right of the toolbar.
    fn help_menu(&mut self, ui: &mut Ui) {
        let newer = match &self.update_notice {
            Some(update::Outcome::Available { version, .. }) => Some(version.clone()),
            _ => None,
        };
        let tip = if newer.is_some() { "Help: an update is available" } else { "Help" };
        let resp = icon_button(ui, Icon::Help, self.about_open || self.left == Some(LeftTab::Docs), tip);
        if newer.is_some() {
            ui.painter().circle_filled(resp.rect.right_top() + vec2(-8.0, 8.0), 4.5, LEAF);
        }
        Popup::menu(&resp).show(|ui| {
            ui.set_min_width(190.0);
            if let Some(v) = &newer {
                if ui.button(RichText::new(format!("Update available: {v}")).color(LEAF_DARK)).clicked() {
                    self.update_result = self.update_notice.clone();
                    self.update_open = true;
                    ui.close();
                }
            }
            if ui.button("Check for updates…").clicked() {
                let c = ui.ctx().clone();
                self.check_updates(&c);
                ui.close();
            }
            if ui.button("Documentation").clicked() {
                self.left = Some(LeftTab::Docs);
                ui.close();
            }
            if ui.button("System check").on_hover_text("What this computer has for TeXora, and what is optional").clicked() {
                self.syscheck_open = true;
                ui.close();
            }
            if ui.button("Command-line tool").on_hover_text("Where this copy's command-line tool is, with examples").clicked() {
                self.cli_open = true;
                ui.close();
            }
            if ui.button("Keyboard shortcuts").clicked() {
                self.left = Some(LeftTab::Settings);
                ui.close();
            }
            ui.separator();
            if ui.button("About TeXora").clicked() {
                self.about_open = true;
                ui.close();
            }
        });
    }

    /// Where the command-line tool is: it is this program, so the box shows this copy's own path,
    /// found at run time and written for this system's terminal.
    fn cli_window(&mut self, ctx: &Context) {
        let help = crate::cli::cli_help(crate::cli::Os::current(), &crate::cli::this_program());
        let flash = sid("cli-copied");
        let recent: Option<(String, f64)> = ctx.data(|d| d.get_temp(flash));
        let now = ctx.input(|i| i.time);
        let mut copied: Option<String> = None;
        let row = |ui: &mut Ui, text: &str, copied: &mut Option<String>| {
            let just = recent.as_ref().is_some_and(|(t, at)| t == text && now - at < 1.5);
            Frame::new().fill(Color32::from_rgb(242, 241, 237)).corner_radius(6).inner_margin(Margin::symmetric(8, 5)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let room = ui.available_width() - 76.0;
                    ui.scope(|ui| {
                        ui.set_max_width(room);
                        ui.add(Label::new(RichText::new(text).monospace().size(12.5).color(INK)).wrap().selectable(true));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = if just { RichText::new("Copied").color(LEAF_DARK) } else { RichText::new("Copy") };
                        if ui.small_button(label).on_hover_text("Copy to the clipboard").clicked() {
                            *copied = Some(text.to_string());
                        }
                    });
                });
            });
        };
        let resp = Modal::new(sid("cli")).show(ctx, |ui| {
            ui.set_width(620.0);
            ui.label(RichText::new("Command-line tool").size(20.0).family(FontFamily::Name("strong".into())).color(INK));
            ui.add_space(4.0);
            if let Some(w) = &help.warning {
                ui.label(RichText::new(w).size(13.5).color(WARN));
                ui.add_space(4.0);
            }
            ui.label(RichText::new(format!("The command-line tool is TeXora itself. This copy is here; run it in {}:", help.terminal)).size(14.0));
            ui.add_space(4.0);
            row(ui, &help.run, &mut copied);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Program file:").size(12.0).color(MUTED));
                ui.label(RichText::new(&help.path).size(12.0).color(MUTED).monospace());
                if ui.small_button("Copy path").clicked() {
                    copied = Some(help.path.clone());
                }
            });
            if let Some(n) = &help.note {
                ui.label(RichText::new(n).size(12.5).color(MUTED));
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Examples").size(14.5).family(FontFamily::Name("medium".into())).color(INK));
            for (cmd, what) in &help.examples {
                ui.label(RichText::new(*what).size(12.5).color(MUTED));
                row(ui, cmd, &mut copied);
            }
            ui.add_space(8.0);
            ui.label(RichText::new(&help.setup_text).size(13.5));
            row(ui, &help.setup_line, &mut copied);
            ui.add_space(6.0);
            ui.label(RichText::new("Every option is listed by --help, and in the README on the project page.").size(12.5).color(MUTED));
            ui.add_space(6.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.button("Close").clicked()).inner
        });
        if let Some(t) = copied {
            ctx.copy_text(t.clone());
            ctx.data_mut(|d| d.insert_temp(flash, (t.clone(), now)));
            self.cli_copied = Some(t);
            ctx.request_repaint_after(std::time::Duration::from_millis(1600));
        }
        if resp.inner || resp.should_close() {
            self.cli_open = false;
        }
    }

    /// What this computer offers TeXora. Nothing listed is needed to start it.
    fn syscheck_window(&mut self, ctx: &Context) {
        use crate::syscheck::{checks, Facts, Status};
        let facts = Facts {
            os: crate::cli::Os::current(),
            pdf_engine_error: self.engine_error.clone(),
            tex_tools: ["pdflatex", "xelatex", "lualatex", "synctex"].iter().map(|t| (*t, crate::latex::find_tool(t).is_some())).collect(),
        };
        let list = checks(&facts);
        let resp = Modal::new(sid("syscheck")).show(ctx, |ui| {
            ui.set_width(620.0);
            ui.label(RichText::new("System check").size(20.0).family(FontFamily::Name("strong".into())).color(INK));
            ui.add_space(2.0);
            ui.label(RichText::new("TeXora carries everything it needs to run: the program, its PDF engine, fonts and short guide. The rest is optional, for one feature each.").size(13.5).color(MUTED));
            ui.add_space(8.0);
            ScrollArea::vertical().max_height((ctx.content_rect().height() - 220.0).max(200.0)).show(ui, |ui| {
                for c in &list {
                    Frame::new().fill(Color32::from_rgb(246, 245, 242)).corner_radius(6).inner_margin(Margin::symmetric(10, 7)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_top(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                            match c.status {
                                Status::Ready => icons::paint(ui.painter(), r, Icon::Check, LEAF),
                                Status::Problem => icons::paint(ui.painter(), r, Icon::Warn, BAD),
                                Status::Optional => {
                                    ui.painter().line_segment([r.left_center() + vec2(4.0, 0.0), r.right_center() - vec2(4.0, 0.0)], Stroke::new(2.0, MUTED));
                                }
                                Status::Info => {
                                    ui.painter().circle_filled(r.center(), 3.0, MUTED);
                                }
                            }
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(c.name).size(14.5).family(FontFamily::Name("medium".into())).color(INK));
                                    let color = match c.status {
                                        Status::Ready => LEAF_DARK,
                                        Status::Problem => BAD,
                                        _ => MUTED,
                                    };
                                    ui.label(RichText::new(&c.found).size(13.0).color(color));
                                });
                                ui.add(Label::new(RichText::new(c.used_for).size(12.5).color(MUTED)).wrap());
                                if let Some(h) = &c.how {
                                    ui.add(Label::new(RichText::new(h).size(12.5).color(INK)).wrap().selectable(true));
                                }
                            });
                        });
                    });
                    ui.add_space(4.0);
                }
            });
            ui.add_space(6.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.button("Close").clicked()).inner
        });
        if resp.inner || resp.should_close() {
            self.syscheck_open = false;
        }
    }

    /// The About and update boxes, which sit over everything and dim the rest.
    pub fn help_windows(&mut self, ctx: &Context) {
        if self.about_open {
            let resp = Modal::new(sid("about")).show(ctx, |ui| {
                ui.set_width(460.0);
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let (r, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
                    logo(ui, r);
                    ui.add_space(4.0);
                    ui.label(RichText::new("TeXora").size(21.0).family(FontFamily::Name("strong".into())).color(INK));
                    ui.label(RichText::new(format!("Version {}", update::VERSION)).size(13.0).color(MUTED));
                });
                ui.add_space(6.0);
                markdown(ui, ABOUT_TEXT, 15.0);
                ui.add_space(6.0);
                Grid::new("about-facts").num_columns(2).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
                    ui.label(RichText::new("Website").size(14.0).color(MUTED));
                    ui.hyperlink_to(RichText::new(update::WEBSITE.trim_start_matches("https://")).size(14.0), update::WEBSITE);
                    ui.end_row();
                    ui.label(RichText::new("Developer").size(14.0).color(MUTED));
                    ui.label(RichText::new(update::DEVELOPER).size(14.0).color(INK));
                    ui.end_row();
                });
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.button("Close").clicked()).inner
            });
            if resp.inner || resp.should_close() {
                self.about_open = false;
            }
        }
        if let Some(rx) = &self.update_rx {
            if let Ok(o) = rx.try_recv() {
                // A newer version found by hand also lights the dot on the ? button.
                if matches!(o, update::Outcome::Available { .. } | update::Outcome::UpToDate) {
                    self.update_notice = Some(o.clone());
                }
                self.update_result = Some(o);
                self.update_rx = None;
            }
        }
        self.update_toast(ctx);
        if self.cli_open {
            self.cli_window(ctx);
        }
        if self.syscheck_open {
            self.syscheck_window(ctx);
        }
        if self.update_open {
            let resp = Modal::new(sid("updates")).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.label(RichText::new("Check for updates").size(18.0).family(FontFamily::Name("medium".into())).color(INK));
                ui.label(RichText::new(format!("You have TeXora {}.", update::VERSION)).color(MUTED));
                ui.add_space(6.0);
                let mut download = None;
                match &self.update_result {
                    None => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Checking…");
                        });
                    }
                    Some(update::Outcome::UpToDate) => {
                        ui.label(RichText::new("You're up to date.").color(LEAF_DARK));
                    }
                    Some(update::Outcome::Failed(m)) => {
                        ui.label(RichText::new(m).color(BAD));
                    }
                    Some(update::Outcome::Available { version, url, notes, download: file }) => {
                        ui.label(RichText::new(format!("TeXora {version} is available.")).color(LEAF_DARK));
                        if !notes.is_empty() {
                            ScrollArea::vertical().max_height(220.0).show(ui, |ui| markdown(ui, notes, 13.5));
                        }
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if let Some((name, link)) = file {
                                let b = Button::new(RichText::new(format!("Download {name}")).color(Color32::WHITE)).fill(LEAF);
                                if ui.add(b).on_hover_text("The package for this computer. Install it over the one you have; your settings stay.").clicked() {
                                    download = Some(link.clone());
                                }
                            }
                            if !url.is_empty() && ui.button("Open download page").clicked() {
                                download = Some(url.clone());
                            }
                        });
                    }
                }
                ui.add_space(8.0);
                let close = ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.button("Close").clicked()).inner;
                (close, download)
            });
            let close = resp.inner.0 || resp.should_close();
            if let Some(u) = resp.inner.1 {
                self.open_external(&u);
            }
            if close {
                self.update_open = false;
            }
        }
    }

    /// Shown by itself in the lower right when a newer version exists, without any click. "Later" hides
    /// it until the next start; "Skip this version" until an even newer one comes out. Never shown while
    /// the audience window is open, so it cannot interrupt a talk; the dot on ? stays either way.
    fn update_toast(&mut self, ctx: &Context) {
        self.update_toast = false;
        let Some(update::Outcome::Available { version, .. }) = &self.update_notice else { return };
        let version = version.clone();
        if self.update_later || self.update_open || self.audience_open || self.st.skipped_update == version {
            return;
        }
        self.update_toast = true;
        let mut action = 0;
        Area::new(sid("update-toast")).order(Order::Foreground).anchor(Align2::RIGHT_BOTTOM, vec2(-18.0, -92.0)).show(ctx, |ui| {
            Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, LINE))
                .corner_radius(10)
                .inner_margin(Margin::same(14))
                .shadow(Shadow { offset: [0, 4], blur: 18, spread: 0, color: Color32::from_black_alpha(40) })
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.5, LEAF);
                        ui.label(RichText::new("Update available").size(15.5).family(FontFamily::Name("medium".into())).color(INK));
                    });
                    ui.label(RichText::new(format!("TeXora {version} is ready to download. You have {}.", update::VERSION)).size(13.0).color(MUTED));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.add(Button::new(RichText::new("See what's new").color(Color32::WHITE)).fill(LEAF)).clicked() {
                            action = 1;
                        }
                        if ui.button("Later").clicked() {
                            action = 2;
                        }
                        if ui.button("Skip this version").clicked() {
                            action = 3;
                        }
                    });
                });
        });
        match action {
            1 => {
                self.update_result = self.update_notice.clone();
                self.update_open = true;
                self.update_later = true;
            }
            2 => self.update_later = true,
            3 => {
                self.st.skipped_update = version;
                let _ = self.st.save();
            }
            _ => {}
        }
    }

    /// Edit notes, then a tick to save or a cross to cancel, in the same place.
    fn note_buttons(&mut self, ui: &mut Ui) {
        if self.note_draft.is_some() {
            if icon_button(ui, Icon::Check, true, "Save notes").clicked() {
                self.save_note_draft();
            }
            if icon_button(ui, Icon::Close, false, "Cancel, keep the notes as they were").clicked() {
                self.note_draft = None;
            }
        } else {
            self.act(ui, Icon::Note, false, Action::EditNotes);
        }
    }

    /// The current slide number. Click it, type a number, press Enter.
    fn slide_box(&mut self, ui: &mut Ui) {
        let id = sid("slide-number");
        let total = self.deck.frames.len();
        let typing = ui.ctx().memory(|m| m.has_focus(id));
        if !typing {
            self.jump_text = (self.frame() + 1).to_string();
        }
        ui.add_space(3.0);
        let edit = TextEdit::singleline(&mut self.jump_text)
            .id(id)
            .desired_width(34.0)
            .align(Align2::CENTER_CENTER)
            .char_limit(5)
            .margin(vec2(2.0, 3.0));
        let resp = ui.add(edit).on_hover_text("Click, type a slide number and press Enter (G)");
        if self.focus_jump {
            self.focus_jump = false;
            resp.request_focus();
        }
        if resp.gained_focus() || (typing && resp.clicked()) {
            let mut st = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
            let all = text::CCursorRange::two(text::CCursor::new(0), text::CCursor::new(self.jump_text.chars().count()));
            st.cursor.set_char_range(Some(all));
            TextEdit::store_state(ui.ctx(), id, st);
        }
        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            let t = self.jump_text.clone();
            self.go_to_slide(&t);
        }
        ui.add_space(3.0);
        ui.label(RichText::new(format!("/ {total}")).size(13.0).color(MUTED));
        ui.add_space(3.0);
    }

    fn view_group(&mut self, ui: &mut Ui) {
        let (v, swap) = (self.view, self.swap);
        for (icon, large, tip) in [
            (Icon::Presenter, false, "Presenter: notes with current and next slide"),
            (Icon::LargeSlide, true, "Large slide with notes beside it"),
        ] {
            if icon_button(ui, icon, v == View::Presenter && swap == large, tip).clicked() {
                self.view = View::Presenter;
                self.swap = large;
            }
        }
        self.act(ui, Icon::Split, v == View::Split, Action::SplitView);
        self.act(ui, Icon::Grid, v == View::Overview, Action::Overview);
        self.act(ui, Icon::Code, v == View::Studio, Action::Studio);
    }

    fn ink_group(&mut self, ui: &mut Ui) {
        for (tool, icon, a) in [
            (Tool::Pointer, Icon::Pointer, Action::Pointer),
            (Tool::Pen, Icon::Pen, Action::Pen),
            (Tool::Highlighter, Icon::Marker, Action::Highlighter),
            (Tool::Laser, Icon::Laser, Action::Laser),
            (Tool::Eraser, Icon::Eraser, Action::Eraser),
            (Tool::Zoom, Icon::ZoomBox, Action::ZoomTool),
        ] {
            self.act(ui, icon, self.tool == tool, a);
        }
        self.act(ui, Icon::Undo, false, Action::Undo);
        let laser = self.tool == Tool::Laser;
        let cur = if laser { self.st.laser_color } else { self.st.ink_color };
        let (r, resp) = ui.allocate_exact_size(vec2(30.0, 34.0), Sense::click());
        ui.painter().circle_filled(r.center(), 8.5, Color32::from_rgb(cur[0], cur[1], cur[2]));
        ui.painter().circle_stroke(r.center(), 10.5, Stroke::new(1.0, LINE));
        let resp = resp.on_hover_text(if laser { "Laser colour" } else { "Ink colour" });
        Popup::menu(&resp).show(|ui| {
            ui.horizontal(|ui| {
                for c in PALETTE {
                    let (rr, b) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
                    ui.painter().circle_filled(rr.center(), 10.0, Color32::from_rgb(c[0], c[1], c[2]));
                    if c == cur {
                        ui.painter().circle_stroke(rr.center(), 12.0, Stroke::new(1.5, INK));
                    }
                    if b.clicked() {
                        if laser {
                            self.st.laser_color = c;
                        } else {
                            self.st.ink_color = c;
                        }
                        ui.close();
                    }
                }
            });
        });
    }

    fn zoom_group(&mut self, ui: &mut Ui) {
        let level = self.zoom_level();
        ui.add_enabled_ui(level > 1.0, |ui| self.act(ui, Icon::Minus, false, Action::ZoomOut));
        let text = RichText::new(format!("{:.0}%", level * 100.0)).size(13.0).color(MUTED);
        if ui.add_sized([46.0, 28.0], Button::new(text).frame(false)).on_hover_text("Fit to page").clicked() {
            self.zoom = None;
        }
        ui.add_enabled_ui(level < MAX_ZOOM, |ui| self.act(ui, Icon::Plus, false, Action::ZoomIn));
        self.act(ui, Icon::Fit, level <= 1.0, Action::ZoomFit);
    }

    fn item(&mut self, ui: &mut Ui, label: &str, a: Action) {
        let key = self.st.keys.iter().find(|(x, _)| *x == a).and_then(|(_, k)| k.first()).map(|k| k.text()).unwrap_or_default();
        if ui.add(Button::new(label).shortcut_text(key)).clicked() {
            let c = ui.ctx().clone();
            self.run(a, &c);
            ui.close();
        }
    }

    fn menus(&mut self, ui: &mut Ui) {
        ui.menu_button("File", |ui| {
            if ui.button("New LaTeX presentation…").clicked() {
                self.new_presentation_dialog();
                ui.close();
            }
            if ui.button("Open PDF…").clicked() {
                self.open_dialog("PDF presentation", &["pdf"]);
                ui.close();
            }
            if ui.button("Open LaTeX project…").clicked() {
                self.open_dialog("LaTeX source", &["tex"]);
                ui.close();
            }
            ui.menu_button("Open recent", |ui| {
                let recents = self.st.recent.clone();
                if recents.is_empty() {
                    ui.label(RichText::new("Nothing yet").color(MUTED));
                }
                for p in recents {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    if ui.button(name).clicked() {
                        self.open_path(&p);
                        ui.close();
                    }
                }
            });
            ui.separator();
            self.item(ui, "Home", Action::StartPage);
            self.item(ui, "Close tab", Action::CloseTab);
            if ui.button("Open the sample lecture").clicked() {
                if let Some(p) = ensure_sample() {
                    self.open_path(&p);
                }
                ui.close();
            }
        });
        ui.menu_button("View", |ui| {
            if ui.button("Presenter").clicked() {
                self.view = View::Presenter;
                self.swap = false;
                ui.close();
            }
            if ui.button("Large slide").clicked() {
                self.view = View::Presenter;
                self.swap = true;
                ui.close();
            }
            self.item(ui, "Slide and notes split", Action::SplitView);
            self.item(ui, "Slide overview", Action::Overview);
            self.item(ui, "LaTeX studio", Action::Studio);
            ui.separator();
            self.item(ui, "Zoom in", Action::ZoomIn);
            self.item(ui, "Zoom out", Action::ZoomOut);
            self.item(ui, "Fit to page", Action::ZoomFit);
            if ui.button("Fit all panels automatically").clicked() {
                self.st.layout = Default::default();
                self.save_layout();
                ui.close();
            }
            self.item(ui, "Edit notes for this slide", Action::EditNotes);
            ui.separator();
            self.item(ui, "Contents and bookmarks", Action::Outline);
            self.item(ui, "Search", Action::Search);
            self.item(ui, "Analytics", Action::Analytics);
            ui.separator();
            ui.menu_button("Speaker notes", |ui| {
                ui.radio_value(&mut self.notes_view, NotesView::Auto, "Automatic");
                ui.radio_value(&mut self.notes_view, NotesView::Pdf, "PDF notes page");
                ui.radio_value(&mut self.notes_view, NotesView::Text, "Text notes");
            });
            ui.menu_button("PDF notes layout", |ui| {
                let before = self.side.notes_mode;
                ui.radio_value(&mut self.side.notes_mode, NotesMode::Auto, "Detect");
                ui.radio_value(&mut self.side.notes_mode, NotesMode::Right, "Notes on right half");
                ui.radio_value(&mut self.side.notes_mode, NotesMode::Left, "Notes on left half");
                ui.radio_value(&mut self.side.notes_mode, NotesMode::Interleaved, "Every second page");
                ui.radio_value(&mut self.side.notes_mode, NotesMode::None, "No notes pages");
                if before != self.side.notes_mode {
                    self.rebuild_deck();
                    self.cache.clear();
                }
            });
        });
        ui.menu_button("Present", |ui| {
            let lbl = if self.audience_open { "Close audience window" } else { "Open audience window" };
            self.item(ui, lbl, Action::Present);
            self.item(ui, "Fullscreen", Action::Fullscreen);
            ui.menu_button("Audience screen", |ui| {
                if self.displays.len() < 2 {
                    ui.label(RichText::new("One display found. Connect a projector, then press F5.").color(MUTED));
                }
                let names: Vec<String> = self
                    .displays
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        let n = if d.friendly_name.is_empty() { format!("Display {}", i + 1) } else { d.friendly_name.clone() };
                        format!("{n} ({}x{})", d.width, d.height)
                    })
                    .collect();
                for (i, n) in names.into_iter().enumerate() {
                    ui.radio_value(&mut self.display, i, n);
                }
            });
            ui.separator();
            self.item(ui, "Black screen", Action::Blackout);
            self.item(ui, "White screen", Action::Whiteboard);
            self.item(ui, "Freeze audience screen", Action::Freeze);
            ui.separator();
            self.item(ui, "Set talk duration", Action::SetDuration);
            self.item(ui, "Start or pause timer", Action::ToggleTimer);
            self.item(ui, "Stop and reset timer", Action::ResetTimer);
            ui.separator();
            self.item(ui, "Bookmark this slide", Action::AddBookmark);
            self.item(ui, "Mark a question on this slide", Action::MarkQuestion);
            self.item(ui, "Go to slide number", Action::Jump);
        });
        ui.menu_button("Tools", |ui| {
            self.item(ui, "Compile LaTeX", Action::Compile);
            self.item(ui, "Check required packages", Action::CheckPackages);
            ui.separator();
            self.item(ui, "Settings and shortcuts", Action::Settings);
        });
    }
}
