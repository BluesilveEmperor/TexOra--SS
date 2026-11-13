//! The Chinese text for the chrome (`src/ui_chrome.rs`).
//!
//! These are the words of the top bar, the tab strip, the menus and the windows that sit over
//! everything: the command-line tool, the system check, the About box and the update boxes.
//!
//! The English text of the program is the key of the translation, and the table here says what
//! each of those strings is in Chinese. The entries follow the order in which the strings appear
//! in the file.
//!
//! Twenty-five of the strings here are also in other parts of the program, so their Chinese text
//! is written down there instead, and the lookup finds them all the same:
//!
//! - `Home`, `Fit to page`, `Zoom in`, `Zoom out`, `Slide and notes split`, `Slide overview`,
//!   `LaTeX studio`, `Edit notes for this slide`, `Contents and bookmarks`, `Search`,
//!   `Analytics`, `Fullscreen`, `Black screen`, `Freeze audience screen`, `Set talk duration`,
//!   `Start or pause timer`, `Stop and reset timer`, `Bookmark this slide`,
//!   `Mark a question on this slide`, `Go to slide number`, `Compile LaTeX`,
//!   `Check required packages` and `Open audience window`: the labels of the actions
//!   (`src/i18n/zh/settings.rs`).
//! - `LaTeX source`: the name of the file type in the file dialog (`src/i18n/zh/app.rs`).
//! - `Present`: also the word on the presenter screen, which is written down in the LaTeX studio
//!   (`src/i18n/zh/studio.rs`) along with `Notes`.
//!
//! A message with values in it has them as `{0}`, `{1}`, ... , in the order they are given.
pub const TABLE: &[(&str, &str)] = &[
    // The top bar: the menu button, the tab strip and the help button.
    ("Menu", "菜单"),
    ("Go to Home", "回到主页"),
    ("About this version", "关于此版本"),
    ("Close TeXora", "关闭 TeXora"),
    ("Close tab", "关闭标签页"),
    ("Ink", "墨迹"),
    ("Help: an update is available", "帮助：有可用更新"),
    ("Help", "帮助"),
    ("Update available: {0}", "有可用更新：{0}"),
    ("Check for updates…", "检查更新…"),
    ("Documentation", "文档"),
    ("System check", "系统检查"),
    ("What this computer has for TeXora, and what is optional", "这台电脑为 TeXora 提供了什么，以及哪些是可选的"),
    ("Command-line tool", "命令行工具"),
    ("Where this copy's command-line tool is, with examples", "这份副本的命令行工具在哪里，附有示例"),
    ("Keyboard shortcuts", "快捷键"),
    ("About TeXora", "关于 TeXora"),
    // The window that shows where the command-line tool is.
    ("Copied", "已复制"),
    ("Copy", "复制"),
    ("Copy to the clipboard", "复制到剪贴板"),
    ("The command-line tool is TeXora itself. This copy is here; run it in {0}:", "命令行工具就是 TeXora 本身。这份副本就在这里；在 {0} 中运行："),
    ("Program file:", "程序文件："),
    ("Copy path", "复制路径"),
    ("Examples", "示例"),
    ("Every option is listed by --help, and in the README on the project page.", "所有选项都列在 --help 中，也在项目页面的 README 里。"),
    ("Close", "关闭"),
    // The window that says what this computer has for TeXora.
    ("TeXora carries everything it needs to run: the program, its PDF engine, fonts and short guide. The rest is optional, for one feature each.", "TeXora 自带运行所需的一切：程序本身、PDF 引擎、字体和简明指南。其余都是可选的，每样对应一个功能。"),
    // The About box.
    ("Version {0}", "版本 {0}"),
    ("Website", "网站"),
    ("Developer", "开发者"),
    // The window that checks for updates.
    ("Check for updates", "检查更新"),
    ("You have TeXora {0}.", "你当前的版本是 TeXora {0}。"),
    ("Checking…", "正在检查…"),
    ("You're up to date.", "你已经是最新版本。"),
    ("TeXora {0} is available.", "TeXora {0} 已发布。"),
    ("Download {0}", "下载 {0}"),
    ("The package for this computer. Install it over the one you have; your settings stay.", "适用于这台电脑的软件包。覆盖安装你现有的那份；设置会保留。"),
    ("Open download page", "打开下载页面"),
    // The little box that says a new version is ready.
    ("Update available", "有可用更新"),
    ("TeXora {0} is ready to download. You have {1}.", "TeXora {0} 可以下载了。你当前的版本是 {1}。"),
    ("See what's new", "看看有什么新内容"),
    ("Later", "稍后"),
    ("Skip this version", "跳过此版本"),
    // The notes buttons in the top bar.
    ("Save notes", "保存备注"),
    ("Cancel, keep the notes as they were", "取消，保持备注原样"),
    // The slide number box, and the two ways of showing the presenter screen.
    ("Click, type a slide number and press Enter (G)", "点击，输入幻灯片编号并按回车（G）"),
    ("Presenter: notes with current and next slide", "演讲者视图：当前幻灯片和下一张幻灯片的备注"),
    ("Large slide with notes beside it", "大幻灯片，旁边是备注"),
    // The colour of the ink and of the laser pointer.
    ("Laser colour", "激光笔颜色"),
    ("Ink colour", "墨迹颜色"),
    // The menus.
    ("File", "文件"),
    ("New LaTeX presentation…", "新建 LaTeX 演示文稿…"),
    ("Open PDF…", "打开 PDF…"),
    ("PDF presentation", "PDF 演示文稿"),
    ("Open LaTeX project…", "打开 LaTeX 项目…"),
    ("Open recent", "最近打开"),
    ("Nothing yet", "还没有"),
    ("Open the sample lecture", "打开示例演讲"),
    ("View", "视图"),
    ("Presenter", "演讲者视图"),
    ("Large slide", "大幻灯片"),
    ("Fit all panels automatically", "自动适应所有面板"),
    // How the notes are shown.
    ("Speaker notes", "演讲者备注"),
    ("Automatic", "自动"),
    ("PDF notes page", "PDF 备注页"),
    ("Text notes", "文本备注"),
    ("PDF notes layout", "PDF 备注布局"),
    ("Detect", "自动检测"),
    ("Notes on right half", "备注在右半边"),
    ("Notes on left half", "备注在左半边"),
    ("Every second page", "每隔一页"),
    ("No notes pages", "没有备注页"),
    // The menu that presents.
    ("Close audience window", "关闭观众窗口"),
    ("Audience screen", "观众屏幕"),
    ("One display found. Connect a projector, then press F5.", "只找到一个显示屏。连接投影仪后按 F5。"),
    ("Display {0}", "显示器 {0}"),
    ("White screen", "白屏"),
    // The menu of the tools.
    ("Tools", "工具"),
    ("Settings and shortcuts", "设置与快捷键"),
];
