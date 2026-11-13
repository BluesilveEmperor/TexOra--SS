//! The Chinese text for the tools of the program (`src/syscheck.rs`, `src/analytics.rs`,
//! `src/update.rs`, `src/tabs.rs` and `src/video_prep.rs`).
//!
//! These are the words of the system check, of the findings of the analytics, of the update
//! check, of the tabs with their bookmarks and question marks, and of the messages of the
//! preparation of the videos.
//!
//! The text of the program is the English text, and the table here says what each of those
//! strings is in Chinese. The entries follow the order in which the strings appear in the files.
//!
//! Two of the strings here are also in other parts of the program, so their Chinese text is
//! written down there instead, and the lookup finds them all the same:
//!
//! - `Download TeXora again and keep its files together: {0} must stay beside the TeXora program.`:
//!   also in the presenter screen (`src/i18n/zh/present.rs`).
//! - `Home`: also the label of the action that goes to the start page
//!   (`src/i18n/zh/settings.rs`).
//!
//! The string `It is a folder, not a file.` is written down here, and the video tools use it too
//! (`src/video.rs`).
//!
//! A message with values in it has them as `{0}`, `{1}`, ... , in the order they are given.
pub const TABLE: &[(&str, &str)] = &[
    // The system check (`src/syscheck.rs`).
    ("Its file, {0}, is missing from beside the TeXora program", "它的文件 {0} 不在 TeXora 程序旁边"),
    ("It could not start: {0}", "它无法启动：{0}"),
    ("Install MacTeX from https://tug.org/mactex (or BasicTeX for a smaller download).", "从 https://tug.org/mactex 安装 MacTeX（或安装体积更小的 BasicTeX）。"),
    ("Install MiKTeX from https://miktex.org, or TeX Live from https://tug.org/texlive.", "从 https://miktex.org 安装 MiKTeX，或从 https://tug.org/texlive 安装 TeX Live。"),
    ("Install TeX Live from your package manager, for example: sudo apt install texlive-latex-extra (Ubuntu, Debian) or sudo dnf install texlive-scheme-medium (Fedora).", "用软件包管理器安装 TeX Live，例如：sudo apt install texlive-latex-extra（Ubuntu、Debian）或 sudo dnf install texlive-scheme-medium（Fedora）。"),
    ("TeXora and its PDF engine", "TeXora 及其 PDF 引擎"),
    ("Included in TeXora and working", "已包含在 TeXora 中且工作正常"),
    ("Showing every presentation", "显示所有演示文稿"),
    ("Not found", "未找到"),
    ("Found: {0}", "已找到：{0}"),
    ("Compiling .tex files in the LaTeX studio. PDF presentations do not need it.", "在 LaTeX 工作室中编译 .tex 文件。PDF 演示文稿不需要它。"),
    ("Web browser", "网页浏览器"),
    ("Your default browser", "你的默认浏览器"),
    ("The website and update downloads", "网站和更新下载"),
    ("Found", "已找到"),
    ("Jumping between a slide and its source in the studio", "在工作室中在幻灯片与它的源码之间跳转"),
    ("It comes with TeX Live, MacTeX and MiKTeX.", "它随 TeX Live、MacTeX 和 MiKTeX 一同提供。"),
    // The findings of the analytics (`src/analytics.rs`).
    ("You finished on your target time.", "你正好在目标时间内讲完了。"),
    ("You ran {0} minutes over your target.", "你比目标时间超出了 {0} 分钟。"),
    ("You ran {0} minutes under your target.", "你比目标时间少用了 {0} 分钟。"),
    ("Slide {0} took {1} ({2} times your average).", "第 {0} 张幻灯片用了 {1}（是平均值的 {2} 倍）。"),
    ("Very quick on slide(s) {0}. Skip them or say why they matter.", "第 {0} 张幻灯片讲得很快。可以跳过它们，或说明它们为什么重要。"),
    ("Not shown: slide(s) {0}.", "未展示：第 {0} 张幻灯片。"),
    ("You came back to slide(s) {0}.", "你回到了第 {0} 张幻灯片。"),
    ("Questions came up on slide(s) {0}. Worth a clearer explanation there.", "第 {0} 张幻灯片上有人提问，那里值得讲得更清楚些。"),
    // The update check (`src/update.rs`).
    ("The update information could not be read.", "无法读取更新信息。"),
    ("The update information has no version number.", "更新信息里没有版本号。"),
    ("GitHub is busy right now. Try again later.", "GitHub 现在正忙，请稍后再试。"),
    ("GitHub answered with an error ({0}).", "GitHub 返回了错误（{0}）。"),
    ("Could not reach GitHub. Check your internet connection.", "无法连接到 GitHub。请检查你的网络连接。"),
    // The tabs and their marks (`src/tabs.rs`).
    ("Bookmark removed from slide {0}.", "已从第 {0} 张幻灯片移除书签。"),
    ("Bookmarked slide {0}.", "已为第 {0} 张幻灯片添加书签。"),
    ("Question mark removed from slide {0}.", "已从第 {0} 张幻灯片移除疑问标记。"),
    ("Marked a question on slide {0}.", "已在第 {0} 张幻灯片上标记疑问。"),
    // The preparation of the videos (`src/video_prep.rs`).
    ("No file name was given.", "没有给出文件名。"),
    ("Web addresses are not played. Save the video in the presentation's folder.", "不播放网络地址。请把视频保存在演示文稿所在的文件夹中。"),
    ("Only files inside the presentation's folder are played. Write the path from that folder.", "只播放演示文稿文件夹内的文件。路径请从该文件夹开始写。"),
    ("The path leads out of the presentation's folder (\"..\").", "该路径指向演示文稿文件夹之外（“..”）。"),
    ("It is a link to a file outside the presentation's folder.", "它是指向演示文稿文件夹之外文件的链接。"),
    ("It is a folder, not a file.", "它是文件夹，不是文件。"),
    ("posterat={0} is not used: {1} has one poster, taken at {2}% as asked on line {3}.", "posterat={0} 没有被使用：{1} 只有一个封面图，在 {2}% 处截取，如第 {3} 行所要求的。"),
    ("Its poster could not be saved.", "它的封面图无法保存。"),
    ("No poster for {0}: posters need ffmpeg, so a plain box shows. The clip plays in VLC when clicked.", "{0} 没有封面图：封面图需要 ffmpeg，所以只会显示一个空白方块。点击后该片段在 VLC 中播放。"),
];
