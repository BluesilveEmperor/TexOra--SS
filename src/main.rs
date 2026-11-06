//! TeXora: a LaTeX-native presenter for Beamer and PDF slides.
#![allow(non_snake_case)] // The program file is named TeXora; see Cargo.toml.
// No black console window behind the app on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod analytics;
mod app;
mod bidi;
mod rtl_edit;
mod cli;
mod deck;
mod latex;
mod notes;
mod pdf;
mod selftest;
mod settings;
mod syscheck;
mod icons;
mod tabs;
mod ui_chrome;
mod ui_panels;
mod ui_present;
mod ui_start;
mod update;

use app::{App, Args};
use std::path::PathBuf;

struct Root {
    app: App,
    audience_shot: bool,
}

impl eframe::App for Root {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let a = &mut self.app;
        a.pump(&ctx);
        a.keys(&ctx);
        if self.audience_shot {
            a.audience_ui(ui);
        } else {
            a.tab_bar(ui);
            a.top_bar(ui);
            if a.left.is_some() {
                egui::Panel::left("left-sidebar")
                    .resizable(true)
                    .default_size(340.0)
                    .size_range(260.0..=640.0)
                    .frame(egui::Frame::new().fill(app::SURFACE).inner_margin(egui::Margin::symmetric(10, 0)))
                    .show(ui, |ui| a.left_sidebar(ui));
            }
            egui::CentralPanel::default().frame(egui::Frame::new().fill(app::BG)).show(ui, |ui| match a.view {
                app::View::Start => a.start_ui(ui),
                app::View::Presenter => a.presenter_ui(ui),
                app::View::Split => a.split_ui(ui),
                app::View::Overview => a.overview_ui(ui),
            });
        }
        a.duration_popup(&ctx);
        a.help_windows(&ctx);
        a.audience_window(&ctx);
        a.screenshot_tick(&ctx);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.app.script_input(raw);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.app.on_exit();
    }
}

/// A picture that cannot be taken must end in an error, never in a program that runs forever.
fn start_screenshot_limit() {
    let seconds = std::env::var("TEXORA_SHOT_LIMIT").ok().and_then(|s| s.parse().ok()).unwrap_or(180);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(seconds));
        eprintln!("No picture after {seconds} seconds, so stopping.");
        std::process::exit(3);
    });
}

const HELP: &str = "TeXora {version}: present PDF and LaTeX Beamer slides

Usage: TeXora [file.pdf | file.tex] [options]

  file.pdf | file.tex      open this presentation
  --version                show the version and exit
  --help, -h               show this list and exit
  --self-test[=file.png]   check this copy works, with no window, and exit (0 means it passed)

For testing and screenshots:
  --screenshot <file.png>  draw about 120 frames, save a picture of the window and exit
  --view <name>            start on: presenter, split, swap, overview,
                           analytics, settings, outline, search, audience, textnotes
  --goto <n>               start on page position n
  --window <W>x<H>         start at this window size
  --script \"<steps>\"       feed in test input, for example \"w:500;k:ArrowRight\"
  --demo                   fill the analytics with invented numbers

Full details: README.md, or ? > Documentation in the app.";

fn parse_args() -> Result<Args, String> {
    let mut a = Args { file: None, screenshot: None, view: None, goto: None, demo: false, script: vec![], window: None };
    let mut it = std::env::args().skip(1);
    while let Some(x) = it.next() {
        if x == "--version" {
            println!("TeXora {}", update::VERSION);
            std::process::exit(0);
        }
        if x == "--self-test" || x.starts_with("--self-test=") {
            let out = x.strip_prefix("--self-test=").map(PathBuf::from);
            std::process::exit(if selftest::run(out) { 0 } else { 1 });
        }
        match x.as_str() {
            "--screenshot" => a.screenshot = Some(PathBuf::from(it.next().ok_or("--screenshot needs a file name")?)),
            "--view" => a.view = Some(it.next().ok_or("--view needs a name")?),
            "--goto" => a.goto = it.next().and_then(|n| n.parse().ok()),
            "--demo" => a.demo = true,
            "--window" => {
                a.window = it.next().and_then(|s| {
                    let (w, h) = s.split_once('x')?;
                    Some([w.parse().ok()?, h.parse().ok()?])
                })
            }
            "--script" => a.script = it.next().unwrap_or_default().split(';').map(str::to_string).collect(),
            "--help" | "-h" => {
                println!("{}", HELP.replace("{version}", update::VERSION));
                std::process::exit(0);
            }
            f => a.file = Some(PathBuf::from(f)),
        }
    }
    Ok(a)
}

/// The Windows package has no console of its own, so text from --help, --version and the tests would
/// vanish. When started from a terminal, borrow that terminal's console instead.
#[cfg(windows)]
fn use_parent_console() {
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(process: u32) -> i32;
    }
    const PARENT: u32 = u32::MAX;
    // SAFETY: AttachConsole takes a plain process id and fails harmlessly when there is no terminal.
    unsafe {
        AttachConsole(PARENT);
    }
}

fn main() -> eframe::Result {
    #[cfg(windows)]
    if std::env::args().len() > 1 {
        use_parent_console();
    }
    let args = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if args.screenshot.is_some() {
        start_screenshot_limit();
    }
    let audience_shot = App::audience_only_shot(&args.view);
    let size = args.window.unwrap_or(if audience_shot { [1100.0, 825.0] } else { [1388.0, 900.0] });
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(if audience_shot { "TeXora Audience View" } else { "TeXora" })
        .with_app_id("texora")
        .with_inner_size(size)
        .with_min_inner_size([820.0, 560.0]);
    if let Some(icon) = icons::window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "TeXora",
        options,
        Box::new(move |cc| Ok(Box::new(Root { app: App::new(cc, args), audience_shot }))),
    )
}
