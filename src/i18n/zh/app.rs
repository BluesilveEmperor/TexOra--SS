//! The Chinese text for the application state and everything that is not drawing (`src/app.rs`).
//!
//! The English text of the program is the key of the translation, and the table here says what
//! each of those strings is in Chinese. The entries follow the order in which the strings appear
//! in the file.
//!
//! Two of the strings here are also in other parts of the program, so their Chinese text is
//! written down there instead, and the lookup finds them all the same:
//!
//! - `Compiling your slides…`: also in the presenter screen (`src/i18n/zh/present.rs`).
//! - `Slides`: also in the presenter screen (`src/i18n/zh/present.rs`).
//!
//! The three sentences about a PDF that will not open are the keys of the message the reader
//! sees, so they are translated here as well.
//!
//! A message with values in it has them as `{0}`, `{1}`, ... , in the order they are given.
pub const TABLE: &[(&str, &str)] = &[
    ("Could not open that file.", "无法打开该文件。"),
    ("TeXora opens .pdf and .tex files, not {0}.", "TeXora 打开 .pdf 和 .tex 文件，而不是 {0}。"),
    ("This PDF has no pages.", "该 PDF 没有页面。"),
    ("Slides refreshed.", "幻灯片已刷新。"),
    ("Could not save this deck's data: {0}", "无法保存该演示文稿的数据：{0}"),
    ("Notes for slide {0} saved in {1}, in the slide's \\note. The slides rebuild in the background.", "第 {0} 张幻灯片的备注已保存到 {1} 的 \\note 中。幻灯片正在后台重新编译。"),
    ("Notes for slide {0} saved in {1}, beside the presentation.", "第 {0} 张幻灯片的备注已保存到演示文稿旁的 {1} 中。"),
    ("Could not save notes: {0}", "无法保存备注：{0}"),
    ("Could not save: {0}", "无法保存：{0}"),
    ("Open a .tex file to compile.", "打开 .tex 文件以进行编译。"),
    ("Compiling…", "正在编译…"),
    (", no errors, 1 warning", "，无错误，1 个警告"),
    (", no errors, {0} warnings", "，无错误，{0} 个警告"),
    ("Compiled with {0} in {1}s{2}", "使用 {0} 编译，耗时 {1} 秒{2}"),
    ("see the log", "查看日志"),
    ("Could not compile: {0}", "无法编译：{0}"),
    ("Could not open {0}: {1}", "无法打开 {0}：{1}"),
    ("{0} is playing in your video player.", "{0} 正在你的视频播放器中播放。"),
    ("Would play {0}.", "将播放 {0}。"),
    ("The clip {0} was not found. Keep it in the same folder as the presentation.", "未找到片段 {0}。请把它与演示文稿放在同一个文件夹里。"),
    ("That link is not a web address, so it was not opened.", "该链接不是网址，因此没有打开。"),
    ("Could not open {0}", "无法打开 {0}"),
    ("No source position here. Compile from TeXora so it can link slides to source.", "此处没有对应的源码位置。请从 TeXora 编译，这样幻灯片才能与源码链接。"),
    ("Language changed.", "语言已更改。"),
    ("Enter a slide number from 1 to {0}.", "请输入 1 到 {0} 之间的幻灯片编号。"),
    ("LaTeX source", "LaTeX 源文件"),
    ("Could not save {0} beside the presentation.", "无法在演示文稿旁保存 {0}。"),
    ("Could not create that file: {0}", "无法创建该文件：{0}"),
    ("One display found. Press F11 for fullscreen slides.", "只找到一个显示屏。按 F11 全屏显示幻灯片。"),
    ("{0} was used by \"{1}\", which now has no such key.", "{0} 原本是 \"{1}\" 的快捷键，现在这个操作没有该快捷键了。"),
    // The keys of the message about a PDF that will not open.
    ("That PDF is protected by a password, which TeXora cannot open yet.", "该 PDF 受密码保护，TeXora 暂时还无法打开。"),
    ("That file could not be found. It may have been moved or renamed.", "找不到该文件。它可能已被移动或重命名。"),
    ("That file could not be opened. It may be damaged, or not really a PDF.", "无法打开该文件。它可能已损坏，也可能并不是 PDF 文件。"),
];
