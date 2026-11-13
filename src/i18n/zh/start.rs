//! The Chinese text for the start page (`src/ui_start.rs`).
//!
//! These are the words of the home page: the cards that open a presentation, the list of recent
//! files, the welcome box and the picker of the Beamer templates.
//!
//! The text of the program is the English text, and the table here says what each of those
//! strings is in Chinese. The entries follow the order in which the strings appear in the file.
//!
//! Two of the strings here are also in other parts of the program, so their Chinese text is
//! written down there instead, and the lookup finds them all the same:
//!
//! - `PDF presentation`: the name of the file type in the file dialog (`src/i18n/zh/chrome.rs`).
//! - `LaTeX source`: the name of the file type in the file dialog (`src/i18n/zh/app.rs`).
//!
//! The word `Cancel` is written down here, so no other part of the program writes it down again.
//!
//! A message with values in it has them as `{0}`, `{1}`, ... , in the order they are given.
pub const TABLE: &[(&str, &str)] = &[
    ("Today", "今天"),
    ("Yesterday", "昨天"),
    ("{0} days ago", "{0} 天前"),
    ("{0} months ago", "{0} 个月前"),
    ("Present PDF and LaTeX Beamer slides with confidence", "自信地演示 PDF 与 LaTeX Beamer 幻灯片"),
    ("Get started", "开始使用"),
    ("New LaTeX presentation", "新建 LaTeX 演示文稿"),
    ("Start from a ready Beamer template", "从现成的 Beamer 模板开始"),
    ("Open PDF presentation", "打开 PDF 演示文稿"),
    ("Present any PDF, with notes if it has them", "演示任意 PDF，若其中有备注则一并显示"),
    ("Open LaTeX project", "打开 LaTeX 项目"),
    ("Compile a .tex file, edit it and present", "编译 .tex 文件，编辑它并演示"),
    ("Try the sample lecture", "试用示例演讲"),
    ("See notes, overlays and citations at work", "看看备注、覆盖层和引用的实际效果"),
    ("Recent files", "最近文件"),
    ("File not found", "未找到文件"),
    ("Remove from this list", "从此列表中移除"),
    ("Welcome to TeXora", "欢迎使用 TeXora"),
    ("Your recent presentations will appear here.", "你最近的演示文稿会显示在这里。"),
    ("See notes, the next slide and your time while the room sees only the slide.", "观众只看到幻灯片的同时，你能看到备注、下一张幻灯片和已用时间。"),
    ("Edit LaTeX beside a live preview. Slides rebuild as you type.", "在实时预览旁编辑 LaTeX，幻灯片随输入而重新生成。"),
    ("Rehearse, then see where the time went on each slide.", "先排练，再看看每张幻灯片上的时间去了哪里。"),
    ("Choose a style. Each template is a complete Beamer file with speaker notes, ready to edit. The animated ones show animations, GIF frames and video clips that play in TeXora.", "选择一种样式。每个模板都是完整的 Beamer 文件，带演讲者备注，可直接编辑。带动画的模板展示了动画、GIF 帧和在 TeXora 中播放的视频片段。"),
    ("Classic", "经典"),
    ("Animated", "动画"),
    ("You choose where to save it next. Change its colours later on the \\definecolor lines at the top.", "接下来选择保存位置。配色稍后可以在顶部的 \\definecolor 各行中更改。"),
    ("Create with {0}…", "使用 {0} 创建…"),
    ("Cancel", "取消"),
];
