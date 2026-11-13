//! The Chinese text for the command-line tool (`src/cli.rs`).
//!
//! The commands themselves are not translated: they are what the reader types. Only the words
//! that explain them are.
//!
//! A message with a value in it has the value as `{0}`, `{1}`, ... , in the order they are given.
pub const TABLE: &[(&str, &str)] = &[
    ("every option", "全部选项"),
    ("which version this is", "查看当前版本"),
    ("check this copy works", "检查这份副本能否正常工作"),
    ("open a presentation", "打开演示文稿"),
    ("To type just TeXora from any folder, run this once in PowerShell, then open a new terminal:", "要在任意文件夹中直接输入 TeXora，请在 PowerShell 中运行一次以下命令，然后打开新的终端："),
    ("In Command Prompt, leave out the & at the start: \"{0}\" --help", "在 Command Prompt 中，请去掉开头的 &：\"{0}\" --help"),
    ("To type just texora instead, add this line to {0}, then open a new terminal:", "要直接输入 texora，请把这一行加入 {0}，然后打开新的终端："),
    ("This is the program inside {0}. If you move the app, open this box again for the new path.", "这是 {0} 里的程序。如果移动了应用，请重新打开此窗口，以得到新的路径。"),
    ("macOS is running this copy from a temporary place, because it was opened straight from where it was downloaded. Move TeXora.app into Applications, open it from there, then open this box again for a path that lasts.", "macOS 正在从临时位置运行这份副本，因为它是直接从下载的位置打开的。请把 TeXora.app 移到 Applications 文件夹中，从那里打开它，然后重新打开此窗口，以得到长期有效的路径。"),
    ("This copy is running from a temporary folder. Move it somewhere permanent, open it from there, then open this box again.", "这份副本正在从临时文件夹中运行。请把它移到固定的位置，从那里打开，然后重新打开此窗口。"),
];
