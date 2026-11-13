//! Video and GIFs inside slides.
//!
//! TeXora contains no video decoder. GIFs are decoded by the `image` crate. Every other format (MP4,
//! MOV, WebM...) is played through a video tool the person installed on their own computer:
//! - **ffmpeg**: TeXora starts it as a separate program and reads plain RGBA pictures from it, so the
//!   clip plays inside the slide; posters are made the same way.
//! - **VLC**: the clip opens in VLC's own window.
//!
//! TeXora never downloads, installs, ships or updates ffmpeg or VLC. They are started with a list of
//! arguments, never through a shell, and the file is passed as `file:<absolute path>` so its name can
//! never be read as an option or as another kind of input.

/// A translated text with its {0}, {1}… placeholders filled in, in the order they are given.
///
/// `format!` wants its format string to be a literal, but a translated one is only known when the
/// program runs, so the placeholders are put in by hand: the text first, the values after it.
macro_rules! tfill {
    ($text:expr $(, $arg:expr)* $(,)?) => {{
        let mut text = String::from($text);
        let args: Vec<String> = vec![$(format!("{}", $arg)),*];
        for (i, arg) in args.iter().enumerate() {
            let key = format!("{{{i}}}");
            text = text.replace(key.as_str(), arg);
        }
        text
    }};
}

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Why a clip cannot be shown. The text is what TeXora writes over the poster.
#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// No such file.
    Missing,
    /// The path leads outside the presentation's folder, or is a web address, or is not a file.
    Refused(String),
    /// A video file that could not be read, with the reason.
    Broken(String),
    /// Not a video or GIF at all.
    NotVideo,
    /// A video (not a GIF), and neither ffmpeg nor VLC was found.
    NeedsPlayer,
    /// Video playing is turned off in Settings.
    PlayerOff,
}

/// The words over a clip that needs a video tool, also shown under Problems.
pub const NEEDS_PLAYER: &str = "This video needs VLC or ffmpeg installed on your computer.";

/// The same words in the language of the interface, for the places that show them over a clip.
pub fn needs_player_text() -> &'static str {
    crate::t!(NEEDS_PLAYER)
}

impl Problem {
    pub fn message(&self, name: &str) -> String {
        match self {
            Problem::Missing => crate::t!("Video not found: Place the video in the same folder as your .tex file or in a subfolder. If it's in a subfolder, include the folder path in \\includevideo.").to_string(),
            Problem::Refused(why) => tfill!(crate::t!("Not played: {0}\n{1}"), name, why),
            Problem::Broken(why) => tfill!(crate::t!("Cannot play {0}\n{1}"), name, why),
            Problem::NotVideo => tfill!(crate::t!("Not a video: {0}\nUse a video file (such as MP4) or a GIF."), name),
            Problem::NeedsPlayer => needs_player_text().to_string(),
            Problem::PlayerOff => tfill!(crate::t!("Not played: {0}\nVideo playing is turned off in Settings > Video."), name),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Gif,
    /// Played through ffmpeg.
    External,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub kind: Kind,
    pub width: u32,
    pub height: u32,
    /// Seconds; 0 when unknown (a GIF before it has been played through).
    pub duration: f64,
    /// Whether the clip has a sound track (a GIF never has).
    pub has_audio: bool,
}

/// One picture, RGBA.
#[derive(Clone, Debug)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// A picture and when it is shown, in seconds from the start.
#[derive(Clone, Debug)]
pub struct Frame {
    pub at: f64,
    pub picture: Picture,
}

/// Frames of a clip, one after another. A stream is opened and read on one thread (the player's own).
pub trait Stream {
    fn info(&self) -> &Info;
    /// The next frame, or None at the end.
    fn next_frame(&mut self) -> Option<Frame>;
    /// Starts again at `seconds` (a GIF goes back to its start and skips ahead).
    fn seek(&mut self, seconds: f64);
}

// ---------- the video tools the person installed ----------

/// Which video tool plays clips that are not GIFs (Settings > Video).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Choice {
    /// ffmpeg if found, else VLC.
    #[default]
    Auto,
    Ffmpeg,
    Vlc,
    Off,
}

impl Choice {
    pub const ALL: [Choice; 4] = [Choice::Auto, Choice::Ffmpeg, Choice::Vlc, Choice::Off];

    pub fn from_setting(s: &str) -> Choice {
        match s {
            "ffmpeg" => Choice::Ffmpeg,
            "vlc" => Choice::Vlc,
            "off" => Choice::Off,
            _ => Choice::Auto,
        }
    }

    pub fn setting(self) -> &'static str {
        match self {
            Choice::Auto => "auto",
            Choice::Ffmpeg => "ffmpeg",
            Choice::Vlc => "vlc",
            Choice::Off => "off",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Choice::Auto => crate::t!("Auto"),
            Choice::Ffmpeg => "ffmpeg",
            Choice::Vlc => "VLC",
            Choice::Off => crate::t!("Off"),
        }
    }
}

/// What plays a clip that is not a GIF.
#[derive(Clone, Debug, PartialEq)]
pub enum Backend {
    /// Inside the slide, through this ffmpeg.
    Ffmpeg(PathBuf),
    /// In VLC's own window.
    Vlc(PathBuf),
    /// Nothing found.
    None,
    Off,
}

#[derive(Clone, Debug, Default)]
pub struct Tools {
    pub choice: Choice,
    pub ffmpeg: Option<PathBuf>,
    pub vlc: Option<PathBuf>,
    /// ffplay, which comes with ffmpeg, plays a clip's sound beside the picture ffmpeg gives.
    pub ffplay: Option<PathBuf>,
    /// Settings > Video > Play sound.
    pub sound: bool,
}

static TOOLS: RwLock<Option<Tools>> = RwLock::new(None);

/// Takes the Settings: the choice, and the paths typed in (empty means "find it").
pub fn configure(choice: &str, ffmpeg_path: &str, vlc_path: &str, sound: bool) {
    let pick = |typed: &str, find: fn() -> Option<PathBuf>| {
        let t = typed.trim();
        if t.is_empty() {
            find()
        } else {
            Some(PathBuf::from(t)).filter(|p| p.is_file())
        }
    };
    let ffmpeg = pick(ffmpeg_path, find_ffmpeg);
    let ffplay = find_ffplay(ffmpeg.as_deref());
    let t = Tools { choice: Choice::from_setting(choice), ffmpeg, vlc: pick(vlc_path, find_vlc), ffplay, sound };
    if let Ok(mut g) = TOOLS.write() {
        *g = Some(t);
    }
}

pub fn tools() -> Tools {
    if let Some(t) = TOOLS.read().ok().and_then(|g| g.clone()) {
        return t;
    }
    configure("auto", "", "", true);
    TOOLS.read().ok().and_then(|g| g.clone()).unwrap_or_default()
}

pub fn backend() -> Backend {
    let t = tools();
    match t.choice {
        Choice::Off => Backend::Off,
        Choice::Ffmpeg => t.ffmpeg.map(Backend::Ffmpeg).unwrap_or(Backend::None),
        Choice::Vlc => t.vlc.map(Backend::Vlc).unwrap_or(Backend::None),
        Choice::Auto => t.ffmpeg.map(Backend::Ffmpeg).or(t.vlc.map(Backend::Vlc)).unwrap_or(Backend::None),
    }
}

/// The folders a program may be in, beyond PATH: where Homebrew, MacPorts, Linux packages and the
/// usual Windows installers put it.
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    for d in ["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin", "/usr/bin", "/snap/bin", "C:/ffmpeg/bin", "C:/Program Files/ffmpeg/bin", "C:/ProgramData/chocolatey/bin"] {
        dirs.push(PathBuf::from(d));
    }
    if let Some(h) = dirs::home_dir() {
        dirs.push(h.join("scoop/shims"));
        dirs.push(h.join("AppData/Local/Microsoft/WinGet/Links"));
        dirs.push(h.join(".local/bin"));
    }
    dirs
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// ffmpeg installed on this computer, if any.
pub fn find_ffmpeg() -> Option<PathBuf> {
    let name = exe("ffmpeg");
    search_dirs().into_iter().map(|d| d.join(&name)).find(|p| p.is_file())
}

/// ffplay, which comes with ffmpeg: beside the ffmpeg in use first (the same install), then the usual
/// places.
pub fn find_ffplay(ffmpeg: Option<&Path>) -> Option<PathBuf> {
    let name = exe("ffplay");
    let beside = ffmpeg.and_then(|f| f.parent()).map(|d| d.join(&name)).filter(|p| p.is_file());
    beside.or_else(|| search_dirs().into_iter().map(|d| d.join(&name)).find(|p| p.is_file()))
}

/// VLC installed on this computer, if any.
pub fn find_vlc() -> Option<PathBuf> {
    let mut places: Vec<PathBuf> = vec![];
    if cfg!(target_os = "macos") {
        places.push(PathBuf::from("/Applications/VLC.app/Contents/MacOS/VLC"));
        if let Some(h) = dirs::home_dir() {
            places.push(h.join("Applications/VLC.app/Contents/MacOS/VLC"));
        }
    }
    if cfg!(windows) {
        places.push(PathBuf::from("C:/Program Files/VideoLAN/VLC/vlc.exe"));
        places.push(PathBuf::from("C:/Program Files (x86)/VideoLAN/VLC/vlc.exe"));
    }
    let name = exe("vlc");
    places.extend(search_dirs().into_iter().map(|d| d.join(&name)));
    places.into_iter().find(|p| p.is_file())
}

/// A command for a tool: no input, and no console window on Windows.
fn tool_command(program: &Path) -> Command {
    let mut c = Command::new(program);
    c.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

/// Runs a tool and returns what it wrote (output, errors), stopping it after `limit`.
fn run_for(mut c: Command, limit: Duration) -> Result<(String, String), String> {
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = c.spawn().map_err(|e| e.to_string())?;
    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let a = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_string(&mut s);
        }
        s
    });
    let b = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(e) = err.as_mut() {
            let _ = e.read_to_string(&mut s);
        }
        s
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(tfill!(crate::t!("it did not answer within {0} seconds"), limit.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok((a.join().unwrap_or_default(), b.join().unwrap_or_default()))
}

/// Settings > Video > Test: runs the tool with its version option and returns its first line.
pub fn test_tool(program: &Path, vlc: bool) -> Result<String, String> {
    if !program.is_file() {
        return Err(crate::t!("No program at this path.").into());
    }
    let mut c = tool_command(program);
    c.arg(if vlc { "--version" } else { "-version" });
    let (out, err) = run_for(c, Duration::from_secs(8))?;
    let text = if out.trim().is_empty() { err } else { out };
    text.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string).ok_or_else(|| crate::t!("It started but did not say its version.").into())
}

/// A path as the tools read it: without the `\\?\` that Windows puts in front of canonical paths.
fn plain(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s)
}

/// The input argument for ffmpeg: always the file protocol, so no name is read as another kind of input.
fn ffmpeg_input(path: &Path) -> String {
    format!("file:{}", plain(path))
}

/// The only kinds of file ffmpeg may read as a clip: real video containers. Playlists (HLS, concat,
/// image sequences, SDP...) are left out, since a "video" that is really a playlist could make ffmpeg
/// read other files or web addresses.
pub const FFMPEG_FORMATS: &str = "mov,mp4,m4a,3gp,3g2,mj2,matroska,webm,avi,ogg,flv,mpeg,mpegts,asf";

/// The options that open a clip in ffmpeg, safely, up to and including `-i <file>`:
/// - `-nostdin`: never reads the keyboard;
/// - `-protocol_whitelist file`: only local files, never web addresses or other protocols, also for
///   anything the file itself points to;
/// - `-format_whitelist`: only real video containers (see `FFMPEG_FORMATS`);
/// - the file as `file:<absolute path>`, so no name is read as an option or another kind of input.
///
/// Only the path and the start time come from the presentation; the program is the person's own,
/// from Settings, and no other argument can come from a project.
pub fn ffmpeg_input_args(path: &Path, start: Option<f64>) -> Result<Vec<String>, Problem> {
    if !path.is_absolute() {
        return Err(Problem::Refused(crate::t!("TeXora only passes full paths to ffmpeg.").into()));
    }
    let mut a: Vec<String> = ["-hide_banner", "-nostdin", "-protocol_whitelist", "file", "-format_whitelist", FFMPEG_FORMATS].iter().map(|s| s.to_string()).collect();
    if let Some(t) = start {
        a.push("-ss".into());
        a.push(format!("{:.3}", t.max(0.0)));
    }
    a.push("-i".into());
    a.push(ffmpeg_input(path));
    Ok(a)
}

/// The arguments that open a clip in VLC: its full path, after `--` so it is never read as an option.
/// `silent`: the clip's `mute` option, or Settings > Video > Play sound off. VLC plays the sound
/// itself, so without ffmpeg (and ffplay) a clip opened in VLC is still heard.
pub fn vlc_args(path: &Path, repeat: bool, silent: bool) -> Result<Vec<String>, String> {
    if !path.is_absolute() {
        return Err(crate::t!("TeXora only passes full paths to VLC.").into());
    }
    let mut a = vec!["--play-and-exit".to_string()];
    if repeat {
        a.push("--loop".into());
    }
    if silent {
        a.push("--no-audio".into());
    }
    a.push("--".into());
    a.push(plain(path));
    Ok(a)
}

/// Opens a clip in VLC's own window. No shell is involved. Only a real video file is handed over (see
/// `video_magic`): VLC would also play a playlist, which can point anywhere.
pub fn open_in_vlc(vlc: &Path, path: &Path, repeat: bool, silent: bool) -> Result<(), String> {
    if !head(path).is_ok_and(|h| video_magic(&h)) {
        return Err(crate::t!("it is not a video file VLC may open.").into());
    }
    let mut c = tool_command(vlc);
    c.args(vlc_args(path, repeat, silent)?).stdout(Stdio::null()).stderr(Stdio::null());
    c.spawn().map(|_| ()).map_err(|e| e.to_string())
}

// ---------- what a file is ----------

/// Whether a file's first bytes are those of a real video container (MP4/MOV, Matroska/WebM, AVI, Ogg,
/// FLV, MPEG). Its name does not count: a playlist renamed `.mp4` is text, and fails this.
fn video_magic(head: &[u8]) -> bool {
    (head.len() >= 8 && &head[4..8] == b"ftyp")
        || head.starts_with(&[0x1A, 0x45, 0xDF, 0xA3])
        || (head.starts_with(b"RIFF") && head.len() >= 12 && &head[8..12] == b"AVI ")
        || head.starts_with(b"OggS")
        || head.starts_with(b"FLV")
        || head.starts_with(&[0x00, 0x00, 0x01, 0xBA])
        || head.starts_with(&[0x00, 0x00, 0x01, 0xB3])
        || (head.first() == Some(&0x47) && head.len() >= 2 && head[1] & 0x80 == 0)
}

/// Whether a file looks like a video, from its first bytes or else its name, to say what is needed
/// when no video tool is there to ask.
fn looks_like_video(path: &Path, head: &[u8]) -> bool {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    video_magic(head) || ["mp4", "m4v", "mov", "mkv", "webm", "avi", "ogv", "flv", "mpg", "mpeg", "ts", "wmv", "3gp"].contains(&ext.as_str())
}

fn head(path: &Path) -> Result<Vec<u8>, Problem> {
    let mut f = File::open(path).map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { Problem::Missing } else { Problem::Broken(e.to_string()) })?;
    let mut h = [0u8; 16];
    let n = f.read(&mut h).map_err(|e| Problem::Broken(e.to_string()))?;
    Ok(h[..n].to_vec())
}

fn is_gif(h: &[u8]) -> bool {
    h.starts_with(b"GIF87a") || h.starts_with(b"GIF89a")
}

/// Whether this clip is a GIF (played by TeXora itself).
pub fn is_gif_file(path: &Path) -> bool {
    head(path).map(|h| is_gif(&h)).unwrap_or(false)
}

/// Whether a clip can be shown, without playing it: a GIF always; another video when a video tool is
/// there (ffmpeg is asked whether it can read it).
pub fn check(path: &Path) -> Result<(), Problem> {
    if !path.exists() {
        return Err(Problem::Missing);
    }
    if !path.is_file() {
        return Err(Problem::Refused(crate::t!("It is a folder, not a file.").into()));
    }
    let h = head(path)?;
    if is_gif(&h) {
        return Ok(());
    }
    match backend() {
        Backend::Ffmpeg(ff) => probe(&ff, path).map(|_| ()),
        // VLC gets only a real video file, never a playlist (see `open_in_vlc`).
        Backend::Vlc(_) if !video_magic(&h) => Err(Problem::NotVideo),
        Backend::Vlc(_) => Ok(()),
        _ if !looks_like_video(path, &h) => Err(Problem::NotVideo),
        Backend::Off => Err(Problem::PlayerOff),
        Backend::None => Err(Problem::NeedsPlayer),
    }
}

/// Opens a clip for playing inside the slide: a GIF by TeXora, another video through ffmpeg.
pub fn open(path: &Path) -> Result<Box<dyn Stream>, Problem> {
    if !path.exists() {
        return Err(Problem::Missing);
    }
    if !path.is_file() {
        return Err(Problem::Refused(crate::t!("It is a folder, not a file.").into()));
    }
    let h = head(path)?;
    if is_gif(&h) {
        return Ok(Box::new(GifStream::open(path)?));
    }
    match backend() {
        Backend::Ffmpeg(ff) => Ok(Box::new(FfmpegStream::open(&ff, path, 0.0)?)),
        _ if !looks_like_video(path, &h) => Err(Problem::NotVideo),
        Backend::Off => Err(Problem::PlayerOff),
        // VLC plays in its own window (`open_in_vlc`); playing inside the slide needs ffmpeg.
        Backend::Vlc(_) | Backend::None => Err(Problem::NeedsPlayer),
    }
}

// ---------- ffmpeg ----------

/// What ffmpeg says about a file's video: its size (upright), frames per second and length.
#[derive(Clone, Debug, PartialEq)]
pub struct Probe {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: f64,
    pub has_audio: bool,
}

/// Reads what `ffmpeg -i <file>` writes about a file. None when it has no video stream.
pub fn parse_probe(text: &str) -> Option<Probe> {
    let duration = text.lines().find_map(|l| {
        let rest = l.trim().strip_prefix("Duration:")?;
        let t = rest.trim().split(',').next()?.trim();
        let p: Vec<f64> = t.split(':').filter_map(|x| x.parse().ok()).collect();
        (p.len() == 3).then(|| p[0] * 3600.0 + p[1] * 60.0 + p[2])
    });
    let line = text.lines().find(|l| l.contains("Stream #") && l.contains("Video:"))?;
    let size = line
        .split(|c: char| c == ' ' || c == ',' || c == '[' || c == ']' || c == '(' || c == ')')
        .filter(|t| !t.starts_with("0x"))
        .find_map(|t| {
            let (a, b) = t.split_once('x')?;
            let (w, h) = (a.parse::<u32>().ok()?, b.parse::<u32>().ok()?);
            (w > 0 && h > 0).then_some((w, h))
        })?;
    let rate = |unit: &str| {
        line.split(',').find_map(|part| {
            let v = part.trim().strip_suffix(unit)?.trim();
            let (num, k) = match v.strip_suffix('k') {
                Some(n) => (n, 1000.0),
                None => (v, 1.0),
            };
            num.parse::<f64>().ok().map(|f| f * k).filter(|f| *f > 0.0 && *f < 1000.0)
        })
    };
    let fps = rate("fps").or_else(|| rate("tbr")).unwrap_or(25.0).clamp(1.0, 60.0);
    // ffmpeg turns a phone video upright as it decodes it, so its size is then the other way round.
    let turned = text.lines().any(|l| l.contains("rotation of") && (l.contains("90.00") || l.contains("270.00")));
    let (width, height) = if turned { (size.1, size.0) } else { size };
    let has_audio = text.lines().any(|l| l.contains("Stream #") && l.contains("Audio:"));
    Some(Probe { width, height, fps, duration: duration.unwrap_or(0.0), has_audio })
}

/// Asks ffmpeg what a file holds.
pub fn probe(ffmpeg: &Path, path: &Path) -> Result<Probe, Problem> {
    let mut c = tool_command(ffmpeg);
    c.args(ffmpeg_input_args(path, None)?);
    let (_, err) = run_for(c, Duration::from_secs(15)).map_err(|e| Problem::Broken(tfill!(crate::t!("ffmpeg could not be run: {0}."), e)))?;
    parse_probe(&err).ok_or(Problem::NotVideo)
}

/// The size pictures are read at: the video's own, made smaller to fit 1280 x 720 (in even numbers,
/// as ffmpeg needs).
pub fn read_size(w: u32, h: u32) -> (u32, u32) {
    let scale = (1280.0 / w as f64).min(720.0 / h as f64).min(1.0);
    let even = |v: f64| ((v / 2.0).round() as u32 * 2).max(2);
    (even(w as f64 * scale), even(h as f64 * scale))
}

/// A video played by ffmpeg, which runs as a separate program and writes RGBA pictures to TeXora.
pub struct FfmpegStream {
    info: Info,
    ffmpeg: PathBuf,
    path: PathBuf,
    fps: f64,
    size: (u32, u32),
    child: Option<Child>,
    start: f64,
    n: u64,
}

impl FfmpegStream {
    pub fn open(ffmpeg: &Path, path: &Path, from: f64) -> Result<FfmpegStream, Problem> {
        let p = probe(ffmpeg, path)?;
        let size = read_size(p.width, p.height);
        let info = Info { kind: Kind::External, width: size.0, height: size.1, duration: p.duration, has_audio: p.has_audio };
        let mut s = FfmpegStream { info, ffmpeg: ffmpeg.to_path_buf(), path: path.to_path_buf(), fps: p.fps, size, child: None, start: 0.0, n: 0 };
        s.start_at(from)?;
        Ok(s)
    }

    fn start_at(&mut self, seconds: f64) -> Result<(), Problem> {
        self.end_process();
        let mut c = tool_command(&self.ffmpeg);
        c.args(["-loglevel", "error"]).args(ffmpeg_input_args(&self.path, Some(seconds))?);
        c.args(["-an", "-sn", "-dn", "-vf"]).arg(format!("scale={}:{}:flags=bilinear", self.size.0, self.size.1));
        c.arg("-r").arg(format!("{:.3}", self.fps));
        c.args(["-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"]);
        c.stdout(Stdio::piped()).stderr(Stdio::null());
        let child = c.spawn().map_err(|e| Problem::Broken(tfill!(crate::t!("ffmpeg could not be started: {0}."), e)))?;
        (self.child, self.start, self.n) = (Some(child), seconds.max(0.0), 0);
        Ok(())
    }

    /// Ends the ffmpeg process, if one is running.
    fn end_process(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for FfmpegStream {
    /// The ffmpeg process goes with the stream, so another slide ends it.
    fn drop(&mut self) {
        self.end_process();
    }
}

impl Stream for FfmpegStream {
    fn info(&self) -> &Info {
        &self.info
    }

    fn next_frame(&mut self) -> Option<Frame> {
        let (w, h) = self.size;
        let out = self.child.as_mut()?.stdout.as_mut()?;
        let mut data = vec![0u8; (w * h * 4) as usize];
        if out.read_exact(&mut data).is_err() {
            self.end_process();
            return None;
        }
        let at = self.start + self.n as f64 / self.fps;
        self.n += 1;
        Some(Frame { at, picture: Picture { width: w, height: h, data } })
    }

    fn seek(&mut self, seconds: f64) {
        let _ = self.start_at(seconds);
    }
}

// ---------- sound ----------

/// The arguments that play a clip's sound with ffplay from `start` seconds: no window (`-nodisp`), no
/// picture or subtitles, and the same safety as for ffmpeg: local files only, real video formats only,
/// the file as `file:<full path>`. ffplay ends by itself at the end of the sound (`-autoexit`).
pub fn ffplay_args(path: &Path, start: f64) -> Result<Vec<String>, Problem> {
    if !path.is_absolute() {
        return Err(Problem::Refused(crate::t!("TeXora only passes full paths to ffplay.").into()));
    }
    // -stats: ffplay reports where the sound is, which the picture follows.
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-stats", "-nodisp", "-autoexit", "-vn", "-sn", "-protocol_whitelist", "file", "-format_whitelist", FFMPEG_FORMATS].iter().map(|s| s.to_string()).collect();
    a.push("-ss".into());
    a.push(format!("{:.3}", start.max(0.0)));
    // -ss alone lands on the picture's key frame before `start`, seconds early; the trim drops the
    // sound before `start`, so it starts exactly where the picture does.
    if start > 0.0 {
        a.push("-af".into());
        a.push(format!("atrim=start={start:.3}"));
    }
    a.push("-i".into());
    a.push(ffmpeg_input(path));
    Ok(a)
}

/// Plays a clip's sound beside its picture, through ffplay (which comes with ffmpeg). TeXora decodes
/// no sound itself: ffplay is a separate program from the person's own ffmpeg install. It is started
/// when the clip plays, from the clip's position, and ended on pause, at the end and when the slide
/// changes, so the sound always follows the picture.
pub struct SoundPlayer {
    ffplay: PathBuf,
    path: PathBuf,
    child: Option<Child>,
    /// The sound's own clock, read from what ffplay reports while it plays: a new one for each start,
    /// so a stopped ffplay can never move the clock of the next.
    clock: std::sync::Arc<std::sync::Mutex<SoundClock>>,
}

/// Where the sound is, as ffplay reports it about every 30 ms (the time heard now, its output delay
/// taken into account). Between reports it is carried on with the computer's clock; each report
/// nudges the estimate, so the 10 ms rounding of the reports evens out.
#[derive(Default)]
pub struct SoundClock {
    /// Clip seconds minus `since` seconds, once ffplay has reported a time.
    offset: Option<f64>,
    since: Option<Instant>,
}

impl SoundClock {
    pub fn report(&mut self, clip: f64, now: Instant) {
        let since = *self.since.get_or_insert(now);
        let off = clip - now.duration_since(since).as_secs_f64();
        self.offset = Some(match self.offset {
            // A jump (a new start, or the sound held up) is taken at once; small differences are evened out.
            Some(o) if (off - o).abs() < 0.05 => o + (off - o) * 0.2,
            _ => off,
        });
    }

    pub fn at(&self, now: Instant) -> Option<f64> {
        Some(self.offset? + now.duration_since(self.since?).as_secs_f64())
    }
}

/// The sound time in one line of ffplay's report ("  12.34 M-A: ..."), when it has one yet.
pub fn parse_ffplay_clock(line: &str) -> Option<f64> {
    let (t, rest) = line.trim_start().split_once(char::is_whitespace)?;
    if !rest.trim_start().starts_with("M-A") {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

impl SoundPlayer {
    /// A sound player for a clip, when it has a sound track, sound is on and ffplay is installed.
    pub fn for_clip(path: &Path, info: &Info, mute: bool) -> Option<SoundPlayer> {
        let t = tools();
        if mute || !t.sound || !info.has_audio || info.kind != Kind::External {
            return None;
        }
        Some(SoundPlayer { ffplay: t.ffplay?, path: path.to_path_buf(), child: None, clock: Default::default() })
    }

    /// Plays from `seconds` into the clip, ending any sound already playing.
    pub fn play_from(&mut self, seconds: f64) {
        self.stop();
        let Ok(args) = ffplay_args(&self.path, seconds) else { return };
        let mut c = tool_command(&self.ffplay);
        c.args(args).stdout(Stdio::null()).stderr(Stdio::piped());
        self.clock = Default::default();
        self.child = c.spawn().ok();
        // ffplay writes where the sound is to its error output, each report ending in a carriage return.
        if let Some(err) = self.child.as_mut().and_then(|c| c.stderr.take()) {
            let clock = self.clock.clone();
            let _ = std::thread::Builder::new().name("ffplay-clock".into()).spawn(move || {
                let mut line = Vec::new();
                for b in BufReader::new(err).bytes() {
                    let Ok(b) = b else { break };
                    if b == b'\r' || b == b'\n' {
                        if let Some(t) = parse_ffplay_clock(&String::from_utf8_lossy(&line)) {
                            if let Ok(mut c) = clock.lock() {
                                c.report(t, Instant::now());
                            }
                        }
                        line.clear();
                    } else if line.len() < 200 {
                        line.push(b);
                    }
                }
            });
        }
    }

    /// Seconds into the clip of the sound heard now, once ffplay has started playing it.
    pub fn clock(&self) -> Option<f64> {
        self.clock.lock().ok()?.at(Instant::now())
    }

    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    /// Whether ffplay is playing now (it ends by itself at the end of the sound).
    #[cfg(test)]
    pub fn playing(&mut self) -> bool {
        self.child.as_mut().is_some_and(|c| matches!(c.try_wait(), Ok(None)))
    }
}

impl Drop for SoundPlayer {
    /// The sound goes with the clip: another slide ends it.
    fn drop(&mut self) {
        self.stop();
    }
}

// ---------- GIF ----------

pub struct GifStream {
    info: Info,
    path: PathBuf,
    frames: Option<image::Frames<'static>>,
    at: f64,
    /// A frame read ahead while seeking, handed out next.
    pending: Option<Frame>,
}

impl GifStream {
    pub fn open(path: &Path) -> Result<GifStream, Problem> {
        let mut s = GifStream { info: Info { kind: Kind::Gif, width: 0, height: 0, duration: 0.0, has_audio: false }, path: path.to_path_buf(), frames: None, at: 0.0, pending: None };
        s.restart()?;
        Ok(s)
    }

    fn restart(&mut self) -> Result<(), Problem> {
        use image::AnimationDecoder;
        use image::ImageDecoder;
        let f = File::open(&self.path).map_err(|e| Problem::Broken(e.to_string()))?;
        let d = image::codecs::gif::GifDecoder::new(BufReader::new(f)).map_err(|e| Problem::Broken(tfill!(crate::t!("The GIF could not be read ({0})."), e)))?;
        let (w, h) = d.dimensions();
        (self.info.width, self.info.height) = (w, h);
        self.frames = Some(d.into_frames());
        self.at = 0.0;
        self.pending = None;
        Ok(())
    }
}

impl Stream for GifStream {
    fn info(&self) -> &Info {
        &self.info
    }

    fn next_frame(&mut self) -> Option<Frame> {
        if let Some(f) = self.pending.take() {
            return Some(f);
        }
        let f = self.frames.as_mut()?.next()?.ok()?;
        let (num, den) = f.delay().numer_denom_ms();
        // Browsers show a GIF frame with no delay for a tenth of a second; so does TeXora.
        let ms = if den == 0 { 100.0 } else { num as f64 / den as f64 };
        let ms = if ms < 20.0 { 100.0 } else { ms };
        let buf = f.into_buffer();
        let frame = Frame { at: self.at, picture: Picture { width: buf.width(), height: buf.height(), data: buf.into_raw() } };
        self.at += ms / 1000.0;
        self.info.duration = self.info.duration.max(self.at);
        Some(frame)
    }

    fn seek(&mut self, seconds: f64) {
        let _ = self.restart();
        // Frames before the time are decoded and dropped; a GIF has no key frames to jump to.
        if seconds > 0.0 {
            while let Some(f) = self.next_frame() {
                if self.at > seconds {
                    self.pending = Some(f);
                    break;
                }
            }
        }
    }
}

// ---------- posters ----------

/// The picture `fraction` of the way into the clip (0 is the first frame), as RGBA. A GIF's poster is
/// made by TeXora; another video's needs ffmpeg.
pub fn poster(path: &Path, fraction: f64) -> Result<(u32, u32, Vec<u8>), Problem> {
    let h = head(path)?;
    let f = fraction.clamp(0.0, 0.99);
    let frame = if is_gif(&h) {
        let mut s = GifStream::open(path)?;
        if f > 0.0 {
            // A GIF's length is known only once read through.
            while s.next_frame().is_some() {}
            let target = s.info().duration * f;
            s.seek(target);
        }
        s.next_frame()
    } else {
        let ff = match (backend(), tools().ffmpeg) {
            (Backend::Ffmpeg(ff), _) => ff,
            // VLC plays the clip, but only ffmpeg can make its poster.
            (Backend::Vlc(_), Some(ff)) => ff,
            _ if !looks_like_video(path, &h) => return Err(Problem::NotVideo),
            (Backend::Off, _) => return Err(Problem::PlayerOff),
            _ => return Err(Problem::NeedsPlayer),
        };
        let p = probe(&ff, path)?;
        FfmpegStream::open(&ff, path, p.duration * f)?.next_frame()
    };
    let f = frame.ok_or_else(|| Problem::Broken(crate::t!("No picture could be read from it.").into()))?;
    Ok((f.picture.width, f.picture.height, f.picture.data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/test/video").join(name)
    }

    #[test]
    fn gif_is_built_in() {
        let mut g = open(&clip("test.gif")).unwrap();
        let f = g.next_frame().unwrap();
        assert_eq!((f.picture.width, f.picture.height), (320, 180));
        let mut n = 1;
        while g.next_frame().is_some() {
            n += 1;
        }
        assert_eq!(n, 20);
        assert!((g.info().duration - 2.0).abs() < 0.05);
        let (gw, _, _) = poster(&clip("test.gif"), 0.25).unwrap();
        assert_eq!(gw, 320);
    }

    #[test]
    fn ffmpeg_output_is_read() {
        let text = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'file:/x/test.mp4':\n  Duration: 00:00:04.00, start: 0.000000, bitrate: 941 kb/s\n  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(progressive), 640x360 [SAR 1:1 DAR 16:9], 937 kb/s, 30 fps, 30 tbr, 15360 tbn (default)\n";
        assert_eq!(parse_probe(text), Some(Probe { width: 640, height: 360, fps: 30.0, duration: 4.0, has_audio: false }));
        let with_sound = format!("{text}  Stream #0:1[0x2](und): Audio: aac (LC) (mp4a / 0x6134706D), 44100 Hz, mono, fltp, 124 kb/s (default)\n");
        assert!(parse_probe(&with_sound).unwrap().has_audio);
        let phone = "  Duration: 00:01:02.50, start: 0.000000\n  Stream #0:0: Video: hevc (Main) (hvc1 / 0x31637668), yuv420p(tv, bt709), 1920x1080, 29.97 fps, 29.97 tbr\n      displaymatrix: rotation of -90.00 degrees\n";
        let p = parse_probe(phone).unwrap();
        assert_eq!((p.width, p.height, p.duration), (1080, 1920, 62.5));
        assert!((p.fps - 29.97).abs() < 1e-9);
        assert_eq!(parse_probe("Input #0, mp3\n  Stream #0:0: Audio: mp3, 44100 Hz\n"), None);
        assert_eq!(read_size(1920, 1080), (1280, 720));
        assert_eq!(read_size(640, 360), (640, 360));
        assert_eq!(read_size(1080, 1920), (406, 720));
    }

    #[test]
    fn ffmpeg_and_vlc_arguments_are_safe() {
        // ffmpeg: no shell, no keyboard, local files only, real video containers only, and the file
        // as `file:<full path>` right after -i, even when its name starts with "-".
        // A full path on this system (on Windows it starts with a drive, such as C:\).
        let talk = std::env::temp_dir().join("talk");
        let odd = talk.join("-y -i x.mp4");
        let a = ffmpeg_input_args(&odd, Some(2.0)).unwrap();
        let at = |s: &str| a.iter().position(|x| x == s).unwrap();
        assert!(at("-nostdin") < at("-i") && at("-protocol_whitelist") < at("-i") && at("-format_whitelist") < at("-i"));
        assert_eq!(a[at("-protocol_whitelist") + 1], "file");
        assert!(!FFMPEG_FORMATS.split(',').any(|f| ["concat", "hls", "applehttp", "image2", "sdp", "rtp", "rtsp", "tee", "data", "lavfi"].contains(&f)), "no playlist formats");
        assert_eq!(a.last().unwrap(), &format!("file:{}", plain(&odd)));
        assert!(a.last().unwrap().ends_with("-y -i x.mp4"), "the name stays one argument");
        assert_eq!(a[a.len() - 2], "-i");
        assert!(ffmpeg_input_args(Path::new("clip.mp4"), None).is_err(), "only full paths");
        // VLC: the full path after "--", so a name starting with "-" is never an option.
        let named = talk.join("--extraintf=http.mp4");
        let v = vlc_args(&named, true, false).unwrap();
        assert_eq!(v, vec!["--play-and-exit".to_string(), "--loop".to_string(), "--".to_string(), plain(&named)]);
        assert!(!v[3].starts_with('-'), "a full path never starts with '-'");
        assert!(vlc_args(Path::new("--extraintf=http.mp4"), false, false).is_err(), "only full paths");
        // VLC plays the sound itself; a muted clip, or sound turned off, gets none.
        let quiet = vlc_args(&named, false, true).unwrap();
        assert_eq!(quiet, vec!["--play-and-exit".to_string(), "--no-audio".to_string(), "--".to_string(), plain(&named)]);
    }

    #[test]
    fn ffplay_arguments_are_safe() {
        let clip = std::env::temp_dir().join("talk").join("-x clip.mp4");
        let a = ffplay_args(&clip, 1.5).unwrap();
        let at = |s: &str| a.iter().position(|x| x == s).unwrap();
        for o in ["-nodisp", "-autoexit", "-vn"] {
            assert!(a.contains(&o.to_string()), "{o}");
        }
        assert_eq!(a[at("-protocol_whitelist") + 1], "file");
        assert_eq!(a[at("-format_whitelist") + 1], FFMPEG_FORMATS);
        assert_eq!(a[at("-ss") + 1], "1.500");
        assert_eq!(a.last().unwrap(), &format!("file:{}", plain(&clip)));
        assert!(ffplay_args(Path::new("clip.mp4"), 0.0).is_err(), "only full paths");
    }

    /// With the real ffplay: the sound starts with the clip, from where it is, and stops on pause.
    #[test]
    fn sound_plays_and_stops_with_the_clip() {
        let (Some(ff), Some(_)) = (find_ffmpeg(), find_ffplay(find_ffmpeg().as_deref())) else { return };
        let s = FfmpegStream::open(&ff, &clip("test.mp4"), 0.0).unwrap();
        assert!(s.info().has_audio, "the test clip has a 440 Hz tone");
        if !tools().sound {
            return;
        }
        assert!(SoundPlayer::for_clip(&clip("test.mp4"), s.info(), true).is_none(), "mute: no sound");
        let mut gif = GifStream::open(&clip("test.gif")).unwrap();
        gif.next_frame();
        assert!(SoundPlayer::for_clip(&clip("test.gif"), gif.info(), false).is_none(), "a GIF has no sound");
        let mut p = SoundPlayer::for_clip(&clip("test.mp4"), s.info(), false).expect("sound");
        let asked = Instant::now();
        p.play_from(1.0);
        std::thread::sleep(Duration::from_millis(600));
        assert!(p.playing(), "ffplay plays the sound");
        // ffplay reports where the sound is, starting from exactly where it was asked to. A busy
        // computer can take a while to start it, so the first report is waited for.
        while p.clock().is_none() && asked.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(20));
        }
        let heard = p.clock().expect("ffplay reports its sound clock");
        let most = 1.05 + asked.elapsed().as_secs_f64();
        assert!((0.95..most).contains(&heard), "sound clock {heard} after starting at 1.0");
        p.stop();
        assert!(!p.playing(), "and stops on pause");
        // Played from near the end, it ends by itself with the clip.
        p.play_from(3.8);
        let start = Instant::now();
        while p.playing() && start.elapsed() < Duration::from_secs(6) {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(!p.playing(), "ffplay ends at the end of the sound");
    }

    #[test]
    fn the_sound_clock_is_read_from_ffplay() {
        assert_eq!(parse_ffplay_clock("  12.34 M-A:  0.000 fd=   0 aq=   16KB vq=    0KB sq=    0B "), Some(12.34));
        assert_eq!(parse_ffplay_clock("  -0.02 M-A:  0.000 fd=   0"), Some(-0.02));
        assert_eq!(parse_ffplay_clock("    nan M-A:    nan fd=   0"), None, "no time before the sound starts");
        assert_eq!(parse_ffplay_clock("    nan    :  0.000 fd=   0"), None);
        assert_eq!(parse_ffplay_clock("[aac @ 0x1] error"), None);
        // Reports rounded to 10 ms even out, and the clock runs on between them.
        let t0 = Instant::now();
        let mut c = SoundClock::default();
        assert!(c.at(t0).is_none());
        for i in 0..40 {
            let now = t0 + Duration::from_millis(30 * i);
            let exact = 5.0 + 0.030 * i as f64;
            c.report((exact * 100.0).round() / 100.0, now);
        }
        let later = t0 + Duration::from_millis(30 * 39 + 15);
        let want = 5.0 + 0.030 * 39.0 + 0.015;
        assert!((c.at(later).unwrap() - want).abs() < 0.006, "clock {} vs {want}", c.at(later).unwrap());
        // A jump (a new place in the clip) is taken at once.
        c.report(9.0, later);
        assert!((c.at(later).unwrap() - 9.0).abs() < 1e-6);
    }

    #[test]
    fn the_sound_starts_exactly_where_the_picture_does() {
        let clip = std::path::PathBuf::from(plain(&std::env::temp_dir().join("clip.mp4")));
        let a = ffplay_args(&clip, 20.0).unwrap();
        let at = a.iter().position(|x| x == "-af").expect("a trim after a seek");
        assert_eq!(a[at + 1], "atrim=start=20.000");
        assert!(at < a.iter().position(|x| x == "-i").unwrap(), "options come before the file");
        assert!(!ffplay_args(&clip, 0.0).unwrap().contains(&"-af".to_string()), "nothing to trim from the start");
        assert!(a.contains(&"-stats".to_string()), "ffplay reports its clock");
    }

    #[test]
    fn a_playlist_named_like_a_video_is_not_a_video() {
        let dir = std::env::temp_dir().join(format!("texora-playlist-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let hls = dir.join("clip.mp4");
        std::fs::write(&hls, "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXTINF:4,\nhttp://example.com/x.ts\n#EXT-X-ENDLIST\n").unwrap();
        assert!(!video_magic(&head(&hls).unwrap()), "VLC never gets a playlist");
        assert!(video_magic(&head(&clip("test.mp4")).unwrap()));
        assert!(video_magic(&head(&clip("hevc.mp4")).unwrap()));
        assert!(!video_magic(&head(&clip("test.txt")).unwrap()));
        assert!(open_in_vlc(Path::new("/nonexistent/vlc"), &hls, false, false).unwrap_err().contains(crate::t!("it is not a video file VLC may open.")));
    }

    /// With the real ffmpeg: files that are really a concat list or an HLS playlist, named `.mp4`, are
    /// refused. Without the whitelists, ffmpeg would follow them to another file.
    #[test]
    fn ffmpeg_never_follows_a_playlist_inside_a_file() {
        let Some(ff) = find_ffmpeg() else { return };
        let dir = std::env::temp_dir().join(format!("texora-crafted-{}", std::process::id())).canonicalize().unwrap_or_else(|_| {
            let d = std::env::temp_dir().join(format!("texora-crafted-{}", std::process::id()));
            std::fs::create_dir_all(&d).unwrap();
            d.canonicalize().unwrap()
        });
        std::fs::create_dir_all(&dir).unwrap();
        // The "other file" a crafted clip tries to reach: a real video beside it.
        let secret = dir.join("private.mp4");
        std::fs::copy(clip("test.mp4"), &secret).unwrap();
        // A concat list named like a video, and a playlist that points to the web.
        let concat = dir.join("near.mp4");
        std::fs::write(&concat, "ffconcat version 1.0\nfile 'private.mp4'\n").unwrap();
        let web = dir.join("web.m3u8");
        std::fs::write(&web, "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXTINF:4,\nhttp://127.0.0.1:9/x.ts\n#EXT-X-ENDLIST\n").unwrap();
        // Plain ffmpeg (as of version 9) does follow both: the danger is real.
        let plain = |f: &Path| {
            let mut c = tool_command(&ff);
            c.args(["-hide_banner", "-nostdin", "-i"]).arg(ffmpeg_input(f));
            run_for(c, Duration::from_secs(15)).map(|(_, e)| e).unwrap_or_default()
        };
        let (near, far) = (plain(&concat), plain(&web));
        assert!(near.contains("Video:") || near.contains("Unsafe"), "plain ffmpeg reads the list: {near}");
        assert!(far.contains("http://") || far.contains("Invalid data"), "plain ffmpeg goes to the web: {far}");
        // TeXora's way refuses both, and the same for its posters and playing.
        for f in [&concat, &web] {
            assert_eq!(probe(&ff, f).err(), Some(Problem::NotVideo), "{}", f.display());
            assert!(FfmpegStream::open(&ff, f, 0.0).is_err());
        }
        // A real clip still plays.
        assert!(probe(&ff, &secret).is_ok());
    }

    #[test]
    fn tool_arguments_are_safe() {
        assert_eq!(ffmpeg_input(Path::new("/a b/-i x.mp4")), "file:/a b/-i x.mp4");
        assert_eq!(plain(Path::new(r"\\?\C:\talk\clip.mp4")), r"C:\talk\clip.mp4");
        assert_eq!(plain(Path::new(r"\\?\UNC\server\share\clip.mp4")), r"\\server\share\clip.mp4");
        assert_eq!(Choice::from_setting("vlc").setting(), "vlc");
        assert_eq!(Choice::from_setting("anything"), Choice::Auto);
    }

    #[test]
    fn files_are_told_apart_without_a_tool() {
        let h = head(&clip("test.mp4")).unwrap();
        assert!(looks_like_video(&clip("test.mp4"), &h));
        let t = head(&clip("test.txt")).unwrap();
        assert!(!looks_like_video(&clip("test.txt"), &t));
        assert_eq!(check(&clip("does-not-exist.mp4")), Err(Problem::Missing));
        assert!(is_gif_file(&clip("test.gif")) && !is_gif_file(&clip("test.mp4")));
    }

    /// Runs only where ffmpeg is installed, as on the computer that builds TeXora.
    #[test]
    fn videos_play_through_installed_ffmpeg() {
        let Some(ff) = find_ffmpeg() else { return };
        let mut s = FfmpegStream::open(&ff, &clip("test.mp4"), 0.0).expect("opens");
        assert_eq!((s.info().width, s.info().height), (640, 360));
        assert!((s.info().duration - 4.0).abs() < 0.1);
        let (mut n, mut last) = (0, -1.0);
        while let Some(f) = s.next_frame() {
            assert!(f.at > last);
            last = f.at;
            n += 1;
        }
        assert_eq!(n, 120, "4 s at 30 frames per second");
        s.seek(2.0);
        assert!((s.next_frame().unwrap().at - 2.0).abs() < 0.01);
        // Other formats play too, through the person's ffmpeg.
        assert_eq!(FfmpegStream::open(&ff, &clip("hevc.mp4"), 0.0).unwrap().info().width, 320);
        assert_eq!(probe(&ff, &clip("test.txt")).err(), Some(Problem::NotVideo));
        assert!(test_tool(&ff, false).unwrap().starts_with("ffmpeg version"));
    }
}
