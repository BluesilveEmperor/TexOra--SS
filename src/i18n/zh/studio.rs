//! The Chinese text for studio.
//!
//! These are the words of the LaTeX studio: the toolbar, the message panel with its problems and
//! packages, the source editor with its folding and the preview of the slides.
//!
//! The LaTeX itself is not here: the code in the editor, the names of the commands and the
//! messages the compiler writes belong to the document, not to the interface, and stay as they are.
pub const TABLE: &[(&str, &str)] = &[
    // The Problems tab, with the errors and the warnings counted apart.
    ("Problems (0)", "问题（0）"),
    ("Problems: 1 warning", "问题：1 个警告"),
    ("Problems: {0} warnings", "问题：{0} 个警告"),
    ("Problems: 1 error", "问题：1 个错误"),
    ("Problems: {0} errors", "问题：{0} 个错误"),
    ("Problems: 1 error, 1 warning", "问题：1 个错误，1 个警告"),
    ("Problems: 1 error, {0} warnings", "问题：1 个错误，{0} 个警告"),
    ("Problems: {0} errors, 1 warning", "问题：{0} 个错误，1 个警告"),
    ("Problems: {0} errors, {1} warnings", "问题：{0} 个错误，{1} 个警告"),
    // What a fold arrow's tip names.
    ("this {0}", "{0}"),
    ("the preamble", "导言区"),
    ("this {…} group", "{…} 组"),
    // The toolbar.
    ("Compile", "编译"),
    ("Save", "保存"),
    ("Check packages", "检查所需的软件包"),
    ("Rebuild on save", "保存时重新编译"),
    ("Follow cursor", "跟随光标"),
    ("Show the slide for the line you are on", "显示光标所在行对应的幻灯片"),
    ("Wrap text", "自动换行"),
    ("Wrap long lines at the editor's width instead of scrolling sideways", "在编辑器宽度处换行，而不是横向滚动"),
    // The message panel.
    ("Ready to compile", "准备编译"),
    ("Hide", "隐藏"),
    ("Show", "显示"),
    ("Copy this text", "复制此文本"),
    ("Wrap long lines", "长行自动换行"),
    ("Full log", "完整日志"),
    ("Packages ({0} missing)", "软件包（{0} 个缺失）"),
    ("Compile to see the full log here.", "编译后可在此查看完整日志。"),
    // The problems, under their own headings.
    ("No problems.", "没有问题。"),
    ("Error", "错误"),
    ("Warning", "警告"),
    ("Errors ({0})", "错误（{0}）"),
    ("Warnings ({0})", "警告（{0}）"),
    ("these stop the slides from being built", "这些会阻止幻灯片生成"),
    ("these do not stop the build, but are worth a look", "这些不会阻止构建，但值得一看"),
    ("line {0}: ", "第 {0} 行："),
    ("Click to go to this line", "点击跳转到该行"),
    // The packages the document needs.
    ("Open a .tex file to check what it needs.", "打开 .tex 文件以查看它需要哪些依赖。"),
    ("Checking packages…", "正在检查软件包…"),
    ("All {0} requirements found.", "所有 {0} 项依赖均已找到。"),
    ("Missing {0}: {1}", "缺少 {0}：{1}"),
    ("Everything checked ({0})", "全部已检查（{0}）"),
    ("Found {0}: {1}", "已找到 {0}：{1}"),
    // The editor with nothing open.
    ("No LaTeX source is open", "未打开 LaTeX 源码"),
    ("Open a .tex file to edit it here and watch the slides update.", "打开 .tex 文件以在此编辑，并实时查看幻灯片更新。"),
    ("Open LaTeX file…", "打开 LaTeX 文件…"),
    // Folding the blocks of the source.
    ("Unfold", "展开"),
    ("Fold", "折叠"),
    ("⋯ {0} lines", "⋯ {0} 行"),
    // The preview of the slides.
    ("No slides yet. Compile to see them here.", "还没有幻灯片。编译后可在此查看。"),
    ("Slide {0} (page {1} of {2})", "幻灯片 {0}（第 {1} 页，共 {2} 页）"),
    ("Double-click for source. Wheel to zoom.", "双击跳转到源码。滚轮缩放。"),
];
