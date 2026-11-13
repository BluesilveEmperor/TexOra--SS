//! The Chinese text for video.
//!
//! These are the words of the video panel (`src/ui_video.rs`) and of the messages the video
//! tools give (`src/video.rs`).
//!
//! A message with a value in it has the value as `{0}`, `{1}`, ... , in the order they are given.
//!
//! Two of the strings here are also in other parts of the program, so their Chinese text is
//! written down there instead, and the lookup finds them all the same:
//!
//! - `Auto`: also the name of the LaTeX engine's own choice (`src/latex.rs`).
//! - `It is a folder, not a file.`: also in the preparation of the videos (`src/video_prep.rs`).
//!
//! The names of the programs (`ffmpeg`, `ffplay`, `VLC`) and the file formats (`MP4`, `GIF`)
//! stay in English, as they are written everywhere else.
pub const TABLE: &[(&str, &str)] = &[
    // The words over a clip, and the reasons a clip cannot play (`src/video.rs`).
    ("This video needs VLC or ffmpeg installed on your computer.", "此视频需要电脑上安装 VLC 或 ffmpeg。"),
    ("Video not found: Place the video in the same folder as your .tex file or in a subfolder. If it's in a subfolder, include the folder path in \\includevideo.", "找不到视频：请把视频放在 .tex 文件所在的文件夹中，或它的子文件夹中。如果在子文件夹中，请在 \\includevideo 里带上该文件夹的路径。"),
    ("Not played: {0}\n{1}", "未播放：{0}\n{1}"),
    ("Cannot play {0}\n{1}", "无法播放 {0}\n{1}"),
    ("Not a video: {0}\nUse a video file (such as MP4) or a GIF.", "不是视频：{0}\n请使用视频文件（如 MP4）或 GIF。"),
    ("Not played: {0}\nVideo playing is turned off in Settings > Video.", "未播放：{0}\n设置 > 视频里已关闭视频播放。"),
    // The name of the player, as it is chosen in the settings.
    ("Off", "关闭"),
    // What the tools say when they do not answer.
    ("it did not answer within {0} seconds", "在 {0} 秒内没有响应"),
    ("No program at this path.", "此路径下没有程序。"),
    ("It started but did not say its version.", "程序启动了，但没有报告版本。"),
    ("TeXora only passes full paths to ffmpeg.", "TeXora 只向 ffmpeg 传递完整路径。"),
    ("TeXora only passes full paths to VLC.", "TeXora 只向 VLC 传递完整路径。"),
    ("it is not a video file VLC may open.", "它不是 VLC 可以打开的视频文件。"),
    ("ffmpeg could not be run: {0}.", "无法运行 ffmpeg：{0}。"),
    ("ffmpeg could not be started: {0}.", "无法启动 ffmpeg：{0}。"),
    ("TeXora only passes full paths to ffplay.", "TeXora 只向 ffplay 传递完整路径。"),
    ("The GIF could not be read ({0}).", "无法读取 GIF（{0}）。"),
    ("No picture could be read from it.", "无法从中读取画面。"),
    // The buttons that lead to the programs, over a clip that needs one (`src/ui_video.rs`).
    ("Get VLC", "获取 VLC"),
    ("Get ffmpeg", "获取 ffmpeg"),
    ("Copy install command", "复制安装命令"),
    ("Copied \"{0}\". Paste it into Terminal and run it yourself; TeXora never runs it.", "已复制“{0}”。请把它粘贴到终端里自己运行；TeXora 从不运行它。"),
    // What a click on a clip says.
    ("Would play {0} in VLC.", "将在 VLC 中播放 {0}。"),
    ("{0} is playing in VLC.", "{0} 正在 VLC 中播放。"),
    ("VLC could not be started: {0}", "无法启动 VLC：{0}"),
    // The settings of the video panel.
    ("GIFs play by themselves. MP4 and other videos play through VLC or ffmpeg installed on your computer: ffmpeg plays them inside the slide, VLC in its own window. TeXora contains no video decoder and never installs one.", "GIF 会自行播放。MP4 等视频通过电脑上安装的 VLC 或 ffmpeg 播放：ffmpeg 在幻灯片内播放，VLC 在自己的窗口中播放。TeXora 不含视频解码器，也不会安装。"),
    ("Video player", "视频播放器"),
    ("found", "已找到"),
    ("set by you", "由你设置"),
    ("not found", "未找到"),
    ("Test", "测试"),
    ("{0} works: {1}", "{0} 可用：{1}"),
    ("{0} did not work: {1}", "{0} 无法使用：{1}"),
    ("{0} was not found on this computer.", "在这台电脑上没有找到 {0}。"),
    ("Path to the program (leave empty to look for it)", "程序的路径（留空则自动查找）"),
    ("Videos play inside the slide through ffmpeg ({0}).", "视频通过 ffmpeg（{0}）在幻灯片内播放。"),
    ("Videos open in VLC ({0}). Install ffmpeg to play them inside the slide and to make their posters.", "视频在 VLC（{0}）中打开。安装 ffmpeg 后就能在幻灯片内播放，并生成它们的封面图。"),
    ("Video playing is off. GIFs still play.", "视频播放已关闭。GIF 仍会播放。"),
    ("Play sound", "播放声音"),
    ("Videos play their sound too. A clip with the mute option stays silent.", "视频也会播放声音。带 mute 选项的片段保持静音。"),
    ("Videos play without sound.", "视频播放时没有声音。"),
    ("Sound plays in VLC, with the picture, in VLC's own window.", "声音在 VLC 自己的窗口中与画面一起播放。"),
    ("Sound plays through ffplay ({0}), which comes with ffmpeg.", "声音通过 ffplay（{0}）播放，它随 ffmpeg 一起安装。"),
    ("No sound inside the slide: ffplay, which comes with ffmpeg, was not found. Install ffmpeg, or choose VLC, which plays sound in its own window.", "幻灯片内没有声音：没有找到随 ffmpeg 一起安装的 ffplay。请安装 ffmpeg，或改用 VLC，它在自己的窗口里播放声音。"),
];
