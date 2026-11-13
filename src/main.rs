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
mod fold;
mod latex;
mod media;
mod notes;
mod pdf;
mod selftest;
mod settings;
mod syscheck;
mod icons;
mod i18n;
mod tabs;
mod templates;
mod ui_chrome;
mod ui_panels;
mod ui_present;
mod ui_start;
mod ui_studio;
mod update;
mod video;
mod video_prep;
mod ui_video;

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
                app::View::Studio => a.studio_ui(ui),
            });
        }
        a.duration_popup(&ctx);
        a.help_windows(&ctx);
        a.template_picker(&ctx);
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

/// What `--help` prints, in English.
const HELP_EN: &str = "TeXora {version}: present PDF and LaTeX Beamer slides

Usage: TeXora [file.pdf | file.tex] [options]

  file.pdf | file.tex      open this presentation
  --version                show the version and exit
  --help, -h               show this list and exit
  --self-test[=file.png]   check this copy works, with no window, and exit (0 means it passed)

For testing and screenshots:
  --screenshot <file.png>  draw about 120 frames, save a picture of the window and exit
  --view <name>            start on: presenter, split, swap, overview, studio,
                           analytics, settings, outline, search, audience, textnotes
  --goto <n>               start on page position n
  --window <W>x<H>         start at this window size
  --script \"<steps>\"       feed in test input, for example \"w:500;k:ArrowRight\"
  --demo                   fill the analytics with invented numbers
  --new <file.tex>         create a presentation from a Beamer template and open it
  --template <name>        with --new: classic, midnight, minimal, conference, sunrise,
                           motion, flipbook or media

Full details: README.md, or ? > Documentation in the app.";

/// The same help, in Chinese. The commands and their names stay as they are: they are what the
/// reader types. Only the words that explain them change.
const HELP_ZH: &str = "TeXora {version}：演示 PDF 与 LaTeX Beamer 幻灯片

用法：TeXora [file.pdf | file.tex] [选项]

  file.pdf | file.tex      打开这个演示文稿
  --version                显示版本号并退出
  --help, -h               显示这份列表并退出
  --self-test[=file.png]   不打开窗口，检查这份副本能否正常工作，然后退出（0 表示通过）

用于测试和截图：
  --screenshot <file.png>  绘制约 120 帧，保存窗口的截图并退出
  --view <name>            从以下视图启动：presenter, split, swap, overview, studio,
                           analytics, settings, outline, search, audience, textnotes
  --goto <n>               从第 n 页开始
  --window <W>x<H>         以这个窗口大小启动
  --script \"<steps>\"       输入测试操作，例如 \"w:500;k:ArrowRight\"
  --demo                   用虚构的数字填满数据分析
  --new <file.tex>         用 Beamer 模板新建演示文稿并打开
  --template <name>        与 --new 一起使用：classic, midnight, minimal, conference, sunrise,
                           motion, flipbook 或 media

完整说明见 README.md，或在应用中选择 ? > 文档。";

fn parse_args() -> Result<Args, String> {
    let mut a = Args { file: None, screenshot: None, view: None, goto: None, demo: false, new_deck: None, template: None, script: vec![], window: None };
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
            "--new" => a.new_deck = it.next().map(PathBuf::from),
            "--template" => a.template = it.next(),
            "--window" => {
                a.window = it.next().and_then(|s| {
                    let (w, h) = s.split_once('x')?;
                    Some([w.parse().ok()?, h.parse().ok()?])
                })
            }
            "--script" => a.script = it.next().unwrap_or_default().split(';').map(str::to_string).collect(),
            "--help" | "-h" => {
                // The help follows the language the interface is set to.
                let code = crate::settings::Settings::load().language;
                let help = match crate::i18n::Lang::from_code(&code) {
                    crate::i18n::Lang::Chinese => HELP_ZH,
                    crate::i18n::Lang::English => HELP_EN,
                };
                println!("{}", help.replace("{version}", update::VERSION));
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
        .with_title(if audience_shot { "TeXora Audience View".to_string() } else { format!("TeXora {}", update::VERSION) })
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
