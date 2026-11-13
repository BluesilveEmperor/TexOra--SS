<p align="center">
  <img src="assets/icon.png" alt="TeXora icon" width="112">
</p>

<h1 align="center">TeXora</h1>

<p align="center">
  A presenter for <span> L<sup>A</sup>T<sub>E</sub>X </span> Beamer and PDF slides:<br>
  private notes, audience screen, live LaTeX editor, rehearsal analytics, animation and video.<br>
  macOS, Windows and Linux.
</p>

<p align="center">
  <a href="https://github.com/BluesilveEmperor/TexOra--SS/releases/latest"><b>Download</b></a> ·
  <a href="#features">Features</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#guide">Guide</a> ·
  <a href="#keyboard-shortcuts">Shortcuts</a> ·
  <a href="#building-from-source">Build</a>
</p>


<p align="center">
  <img src="screenshots/tour.gif" alt="A tour of TeXora: Home, presenter view, large slide, LaTeX studio with folding, analytics, contents, settings, drawing on a slide and the audience window" width="400">
  <img src="screenshots/media-demo.gif" alt="An MP4 video and an animated GIF playing inside a slide, then two animations made with the LaTeX animate package" width="400">
</p>

TeXora shows the speaker the notes, the current and next slide and the time, while the audience sees
only the slide. It opens PDF files and LaTeX Beamer sources; a `.tex` file is compiled on opening and
can be edited in the built-in LaTeX studio or in any other editor.

## Features

|                          |                                                                                                                                                |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| **Presenter view**       | Notes, current and next slide, overlay step, elapsed time, pace against a set duration, and a clock. Large-slide and split views.              |
| **Audience window**      | Fullscreen on a projector or second display (F5). Black screen, whiteboard and freeze.                                                         |
| **LaTeX studio**         | Source editor with code folding and block guides, beside a slide and notes preview; errors and warnings listed apart; SyncTeX links. |
| **LaTeX Beamer**         | Compiles with pdfLaTeX, XeLaTeX or LuaLaTeX, chosen from `% !TEX program` or the packages used. Rebuilds when the file is saved in any editor. |
| **Required packages**    | Lists every package, theme and program a file needs, with how to install the missing ones.                                                     |
| **Speaker notes**        | From Beamer notes pages, `\note{}`, `.pdfpc` files or a Markdown file; editable in the app, in any language.                                   |
| **Animations and video** | `animate` package animations, GIFs and video clips play inside the slide ([see it](#animations)); video uses an installed ffmpeg or VLC. |
| **Drawing and pointing** | Pen, highlighter, eraser, laser pointer and zoom to an area. Ink is kept per slide.                                                            |
| **Navigation**           | Contents, bookmarks, question marks and full-text search in a sidebar. Tabs for several presentations.                                         |
| **Analytics**            | Time on each slide for every rehearsal, compared with the previous one.                                                                        |
| **Tools**                | Configurable shortcuts, a system check, a command-line interface and update notices.                                                           |
| **Languages**            | English and Simplified Chinese, chosen in Settings; the change is instant and is remembered.                                                  |

## Download and install

Each system has an installer and a portable package in the [latest release](https://github.com/BluesilveEmperor/TexOra--SS/releases/latest).

| System                                 | Installer                                | Portable                              |
| -------------------------------------- | ---------------------------------------- | ------------------------------------- |
| macOS, Apple silicon (M1 and later)    | `TeXora-<version>-mac-arm64.dmg`         | `…-mac-arm64.zip`                     |
| macOS, Intel                           | `TeXora-<version>-mac-x64.dmg`           | `…-mac-x64.zip`                       |
| Windows 10 and 11 (x64)                | `TeXora-<version>-windows-x64-setup.exe` | `…-windows-x64.zip`                   |
| Debian, Ubuntu and derivatives (x64)   | `texora_<version>_amd64.deb`             | `TeXora-<version>-linux-x64.tar.gz`   |
| Debian, Ubuntu and derivatives (arm64) | `texora_<version>_arm64.deb`             | `TeXora-<version>-linux-arm64.tar.gz` |

- **macOS**: open the `.dmg` and drag **TeXora** to **Applications**. macOS asks for confirmation the
  first time; see [First launch on macOS](#first-launch-on-macos).
- **Windows**: run the setup program. It installs per user without administrator rights (or for all
  users), adds Start menu and "Open with" entries for PDF and `.tex` files, and can be removed from
  *Installed apps*. Windows may warn about an unknown publisher.
- **Linux**: `sudo apt install ./texora_<version>_amd64.deb`, then start **TeXora** from the menu or
  run `texora`. The `.tar.gz` runs from any folder with `./TeXora`. Requires glibc 2.31 or newer
  (Ubuntu 20.04, Debian 11 or later) and an X11 or Wayland desktop.

**Requirements.** Presenting PDF files needs nothing else: each package includes the PDF engine.

| To                        | Install                                                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Presenting `PDF` files    | nothing                                                                                                                           |
| Compile `.tex` files      | A TeX distribution: **MacTeX** (macOS), **MiKTeX** or TeX Live (Windows), **TeX Live** (Linux)                                    |
| Play MP4 and other videos | **ffmpeg** (plays inside the slide, with sound through ffplay) or **VLC** (plays in its own window). Optional; GIFs need neither. |

Every release file is listed with its checksum in `SHA256SUMS.txt`; `VALIDATION.txt` records how each
file was tested.

### First launch on macOS

TeXora is signed *ad hoc* rather than with an Apple Developer ID, so macOS blocks the first launch.

<details>
<summary>Steps to open TeXora the first time</summary>
<br>

**macOS 15 Sequoia and later**

1. Open **TeXora** and click **Done** on the warning.
2. Open **System Settings > Privacy & Security**.
3. In the **Security** section, find the entry for TeXora and click **Open Anyway**.
4. Confirm with the password or Touch ID.

**macOS 14 Sonoma and earlier**: right-click (or Control-click) TeXora in Applications, choose
**Open**, then **Open** again.

**Terminal** (any version): remove the download mark.

```sh
xattr -dr com.apple.quarantine /Applications/TeXora.app
```

This is needed only once.

</details>

## Quick start

1. Start TeXora. **Home** lists recent files and ways to begin.
2. Choose **New LaTeX presentation**, **Open PDF presentation** or **Open LaTeX project**, pick a
   recent file, or drop a file on the window. **Try the sample lecture** opens a complete example talk.
3. A `.tex` file is compiled on opening. Edit it in the **LaTeX studio** (Cmd/Ctrl+E) or in another
   editor.
4. Press **F5** to open the audience window, then move through the slides with the arrow keys or
   **Space**.

Each presentation opens in its own tab, with its own position, ink and timer. Every button shows its
name and shortcut on hover.

<details>
<summary><b>Templates</b> for new LaTeX presentations</summary>
<br>

Each template is a complete Beamer file with speaker notes.

| Template                                                    | Contents                                                                                   |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Classic Green, Midnight, Minimal White, Conference, Sunrise | Static themes                                                                              |
| Motion Lab                                                  | Animations drawn with TikZ                                                                 |
| Flipbook                                                    | The frames of a GIF and charts that grow                                                   |
| Media Studio                                                | A video clip and charts that draw themselves                                               |
| Video Guide                                                 | `\includevideo` explained slide by slide, with a video and a GIF playing inside the slides |

Flipbook, Media Studio and Video Guide save their pictures and clips in a `media` folder beside the
new file.

</details>

## Guide

### Presenter screen

The left panel holds the speaker notes; the right panel shows the current slide (what the audience
sees) and the next slide. The timer, slide number and sections bar run along the bottom.

<details>
<summary>Areas of the presenter screen</summary>
<br>

| Area                  | Content                                                                                                                                                        |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Top row**           | Open files; readiness (displays, duration, notes) or, while presenting, what the audience sees; the time of day; **Present**, which opens the audience window. |
| **Notes**             | Speaker notes for the slide on screen. Scroll or pinch to change the text size.                                                                                |
| **Current slide**     | What the audience sees.                                                                                                                                        |
| **Next slide**        | The next slide or overlay step.                                                                                                                                |
| **Timer**             | Elapsed time (click to pause or resume), talk duration (click to set), and play, pause and stop buttons.                                                       |
| **Previous and next** | One step back or forward, with the slide number and overlay step.                                                                                              |
| **Sections**          | The talk's sections in their colours, sized by number of slides. A black line marks the position; ribbons mark bookmarks. Click a section to go there.         |
| **Pace**              | Time left, or time ahead, behind or over. When behind, a button offers to skip slides to recover the time.                                                     |

**Large slide** view (**N**) shows the slide across the main panel with the drawing tools beside it.
Drag the dividers to resize panels; **View > Fit all panels automatically** resets them.

</details>

### Toolbar

The second row of buttons acts on the open presentation: views, the audience screen, notes, drawing,
zoom and navigation.

<details>
<summary>All toolbar buttons</summary>
<br>

**Views**

| Button                                                                     | Name                  | Action                                            | Key        |
|:--------------------------------------------------------------------------:| --------------------- | ------------------------------------------------- | ---------- |
| <img src="screenshots/icons/presenter.png" width="24" alt="Presenter">     | Presenter             | Notes with the current and next slide             |            |
| <img src="screenshots/icons/large-slide.png" width="24" alt="Large slide"> | Large slide           | A large current slide with notes beside it        | N          |
| <img src="screenshots/icons/split.png" width="24" alt="Split">             | Slide and notes split | Slide and notes side by side, with the next slide | S          |
| <img src="screenshots/icons/overview.png" width="24" alt="Overview">       | Slide overview        | Every slide at a glance; click one to go there    | O          |
| <img src="screenshots/icons/studio.png" width="24" alt="LaTeX studio">     | LaTeX studio          | Edit the `.tex` source beside the slide           | Cmd/Ctrl+E |

**Audience screen**

| Button                                                                      | Name            | Action                                                           | Key |
|:---------------------------------------------------------------------------:| --------------- | ---------------------------------------------------------------- | --- |
| <img src="screenshots/icons/audience.png" width="24" alt="Audience window"> | Audience window | Slides in their own window, fullscreen on a second display       | F5  |
| <img src="screenshots/icons/black.png" width="24" alt="Black screen">       | Black screen    | Blanks the audience screen; press again to restore               | B   |
| <img src="screenshots/icons/whiteboard.png" width="24" alt="Whiteboard">    | Whiteboard      | A white page to draw on                                          | W   |
| <img src="screenshots/icons/freeze.png" width="24" alt="Freeze">            | Freeze          | Holds the audience on this slide while the presenter moves ahead | F   |

**Notes**

| Button                                                                   | Name       | Action                               | Key            |
|:------------------------------------------------------------------------:| ---------- | ------------------------------------ | -------------- |
| <img src="screenshots/icons/edit-notes.png" width="24" alt="Edit notes"> | Edit notes | Opens this slide's notes for editing | E              |
| <img src="screenshots/icons/save-notes.png" width="24" alt="Save notes"> | Save notes | Saves the edited notes               | Cmd/Ctrl+Enter |
| <img src="screenshots/icons/cancel-notes.png" width="24" alt="Cancel">   | Cancel     | Discards the changes                 | Esc            |

**Drawing and pointing**

| Button                                                                     | Name          | Action                                            | Key        |
|:--------------------------------------------------------------------------:| ------------- | ------------------------------------------------- | ---------- |
| <img src="screenshots/icons/pointer.png" width="24" alt="Pointer">         | Pointer       | Normal pointer; click to advance                  | V          |
| <img src="screenshots/icons/pen.png" width="24" alt="Pen">                 | Pen           | Draws on the slide, also on the audience screen   | D          |
| <img src="screenshots/icons/highlighter.png" width="24" alt="Highlighter"> | Highlighter   | A wide, transparent stroke                        | H          |
| <img src="screenshots/icons/laser.png" width="24" alt="Laser pointer">     | Laser pointer | A bright dot with a short trail                   | L          |
| <img src="screenshots/icons/eraser.png" width="24" alt="Eraser">           | Eraser        | Removes the strokes it touches                    | X          |
| <img src="screenshots/icons/zoom-area.png" width="24" alt="Zoom to area">  | Zoom to area  | Drag a box to enlarge that part for everyone      | Z          |
| <img src="screenshots/icons/undo.png" width="24" alt="Undo ink">           | Undo ink      | Removes the last stroke                           | Cmd/Ctrl+Z |
| <img src="screenshots/icons/colour.png" width="24" alt="Colour">           | Colour        | Ink colour, or laser colour while the laser is on |            |

Ink stays with each slide. **C** clears the ink on the current slide.

**Zoom**

| Button                                                               | Name        | Action                                                            | Key |
|:--------------------------------------------------------------------:| ----------- | ----------------------------------------------------------------- | --- |
| <img src="screenshots/icons/zoom-out.png" width="24" alt="Zoom out"> | Zoom out    | Makes the slide smaller                                           | -   |
| <img src="screenshots/icons/zoom-in.png" width="24" alt="Zoom in">   | Zoom in     | Enlarges the slide; the wheel or a pinch zooms around the pointer | +   |
| <img src="screenshots/icons/fit.png" width="24" alt="Fit to page">   | Fit to page | Shows the whole slide                                             | 0   |

**Navigation**

| Button                                                               | Name        | Action                         | Key          |
|:--------------------------------------------------------------------:| ----------- | ------------------------------ | ------------ |
| <img src="screenshots/icons/first.png" width="24" alt="First slide"> | First slide | Goes to the start              | Home         |
| <img src="screenshots/icons/previous.png" width="24" alt="Previous"> | Previous    | One step back                  | Left         |
| <img src="screenshots/icons/next.png" width="24" alt="Next">         | Next        | The next overlay step or slide | Right, Space |
| <img src="screenshots/icons/last.png" width="24" alt="Last slide">   | Last slide  | Goes to the end                | End          |

Click the slide number between **Previous** and **Next** to type a slide number. **Shift+Right** and
**Shift+Left** skip whole slides; **Backspace** returns to the previous position.

**Top right**

| Button                                                                 | Name                   | Action                                                                    | Key        |
|:----------------------------------------------------------------------:| ---------------------- | ------------------------------------------------------------------------- | ---------- |
| <img src="screenshots/icons/bookmark.png" width="24" alt="Bookmark">   | Bookmark               | Marks the slide in its section's colour                                   | M          |
| <img src="screenshots/icons/question.png" width="24" alt="Question">   | Question               | Records a question on this slide, counted in Analytics                    | Q          |
| <img src="screenshots/icons/contents.png" width="24" alt="Contents">   | Contents and bookmarks | Sidebar with sections, bookmarks and questions                            | T          |
| <img src="screenshots/icons/search.png" width="24" alt="Search">       | Search                 | Finds text in slides and notes                                            | Cmd/Ctrl+F |
| <img src="screenshots/icons/analytics.png" width="24" alt="Analytics"> | Analytics              | Time spent on each slide                                                  | Y          |
| <img src="screenshots/icons/settings.png" width="24" alt="Settings">   | Settings               | Text size, LaTeX, packages, video, notes, updates and shortcuts           | Cmd/Ctrl+, |
| <img src="screenshots/icons/help.png" width="24" alt="Help">           | Help                   | Documentation, system check, command-line tool, shortcuts, updates, About |            |

These buttons open a sidebar on the left; the same button or the cross closes it.

</details>

### Speaker notes

Notes come from Beamer notes pages (`\setbeameroption{show notes on second screen=right}`), from
`\note{...}` in the source, from a `.pdfpc` file, or from a Markdown notes file beside the
presentation. **Edit notes** (**E**) edits them in place; **Cmd/Ctrl+Enter** saves and **Esc** cancels.

<details>
<summary>Where notes are saved, formatting and languages</summary>
<br>

| Opened file          | Notes saved in                                                                                                            |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `Lecture.pdf`        | `Lecture.note.md` in the same folder                                                                                      |
| `Thesis defense.tex` | `Thesis defense.tex`, as `\note{...}` in each frame (or `Thesis defense.note.md` with **Also save in the .tex file** off) |

- **PDF**: keep the `.note.md` file with the PDF when moving or sharing it. Each slide has a
  `## Slide N` heading; the file can also be written in any text editor. Files named
  `Lecture.notes.md` by earlier versions are still read and renamed on the next save.
- **LaTeX**: the source is the single place for notes, so the presenter and the studio always agree.
  A note saved in the presenter is written into the frame's `\note{...}`, shows at once, and the
  slides rebuild. **Settings > Speaker notes** shows where the current notes are stored.
- **Formatting**: Markdown `**bold**`, `*emphasis*`, `- lists` and `# headings`; in a `.tex` file
  these become `\textbf`, `\emph` and lists. Clearing all text removes the slide's note.
- **Languages**: any language and emoji, using the computer's fonts. Arabic and Hebrew are laid out
  right to left while typing and when shown. Notes are saved as UTF-8; UTF-16 and older encodings are
  read.

</details>

### Custom classes and themes

Any Beamer presentation works, including custom Beamer classes and themes. LaTeX
builds the slides, so every theme, colour scheme, template and package behaves as in any other editor;
TeXora reads the frames from Beamer's own `.nav` file.

<details>
<summary>What is supported, and the limits</summary>
<br>

- **Own class or theme files**: `\documentclass{acmedeck}` or `\usetheme{Acme}` with `acmedeck.cls` or
  `beamerthemeAcme.sty` in the presentation's folder. The package check counts them as found.
- **Every frame form**: `\begin{frame}…\end{frame}`, the command form `\frame{…}`, and frame
  environments defined around a frame, in the file or in a local `.sty`/`.cls`, for example
  `\newenvironment{customframe}[1]{\begin{frame}[environment=customframe]{#1}}{\end{frame}}`.
  Their `\note{…}`, titles, overlays, folding and slide-to-source links all work.
- **`\againframe{label}`** counts as a slide of its own: it shows the note of the frame it repeats, and
  a note written there goes into that frame.
- **Limits**: frames kept in separate files (`\input`, `\include`) or made by a macro cannot be
  matched to the source one for one. TeXora still presents everything (slides, overlays, PDF notes
  pages and `.note.md` notes), but does not read `\note{…}` from the source and saves edited notes in
  the `.note.md` file, so no note lands on the wrong slide.
</details>

### LaTeX studio

**View > LaTeX studio** (**Cmd/Ctrl+E**) shows the source on the left, the slide and its notes on the
right, and compile messages at the bottom. Typing never saves or rebuilds by itself: **Compile**
(**Cmd/Ctrl+B**) builds the slides, and **Save** (**Cmd/Ctrl+S**) rebuilds when **Rebuild on save** is on.

<details>
<summary>Studio controls and editing</summary>
<br>

| Control             | Action                                                                              |
| ------------------- | ----------------------------------------------------------------------------------- |
| **Compile**         | Saves the source and builds the slides. **Cmd/Ctrl+B**                              |
| **Save**            | Saves the source; rebuilds if **Rebuild on save** is on. **Cmd/Ctrl+S**             |
| **Engine**          | pdfLaTeX, XeLaTeX or LuaLaTeX; **Auto** reads `% !TEX program` or the packages used |
| **Check packages**  | Lists every package, theme and program the file needs, and how to get missing ones  |
| **Rebuild on save** | Rebuilds whenever the file is saved, in the studio or another editor                |
| **Follow cursor**   | Shows the slide for the line under the cursor                                       |
| **Wrap text**       | Wraps long lines at the editor's width                                              |
| **Present**         | Returns to the presenter view                                                       |

- **Folding**: an arrow beside the line numbers folds an environment (`\begin{…}` to `\end{…}`), a
  section up to the next heading of its level, the preamble, or a multi-line `{…}` group such as a long
  `\note`. A folded block shows "⋯ N lines"; the arrow or that label opens it, as does moving the
  cursor into it. The file always keeps every line.
- **Block guide**: a green bar beside the line numbers marks the block containing the cursor; its
  first and last lines are highlighted.
- **Errors and warnings**: the Problems tab counts them separately and lists errors first. Errors stop
  the build; warnings do not. Line numbers are red for errors and orange for warnings.
- The editor keeps the keyboard while the studio is open, so typing never triggers presenter
  shortcuts. Other text fields, such as Search, receive their own input.
- **Cmd/Ctrl+/** comments or uncomments the current or selected lines; **Cmd/Ctrl+]** always comments
  and **Cmd/Ctrl+[** always uncomments.
- Double-click the slide preview to jump to its source. The **<** and **>** buttons move between
  slides; the wheel zooms.
- The bottom panel has **Problems**, **Packages** and **Full log** tabs. Click a problem to go to its
  line. Drag the panel's top edge to resize it.
- Drag the divider between source and preview to resize; double-click it to split evenly.
- A file that fails to compile on opening opens in the studio with its problems listed.
- The first time a file is opened, an untouched copy is kept as `<name>.tex.texora-backup`.
- Slide and source links use SyncTeX, included with TeX Live, MacTeX and MiKTeX.

</details>

### Animations

<p align="center">
  <img src="screenshots/media-demo.gif" alt="An MP4 video and an animated GIF playing inside a slide, then two animations made with the LaTeX animate package" width="760"><br>
  <sub>An MP4 video and a GIF playing inside the slide, then two <code>animate</code> package animations.</sub>
</p>

Animations made with the LaTeX `animate` package (`animateinline` or `\animategraphics`) play on the
slide. Click to play or pause; in the presenter view a bar under the animation steps through frames.

<details>
<summary>Supported settings and using a GIF with <code>animate</code></summary>
<br>

| Setting      | Behaviour                                                                           |
| ------------ | ----------------------------------------------------------------------------------- |
| `autoplay`   | Starts when the slide is shown and stops on leaving it; otherwise waits for a click |
| `loop`       | Restarts from the first frame                                                       |
| `palindrome` | Plays forward, then backward                                                        |
| Frame rate   | Follows the source, including changes with `\newframe[rate]`                        |

Other `animate` options are untested. The audience sees the same frame as the presenter.

LaTeX cannot include a `.gif` directly. Split it into pictures with [ffmpeg](https://ffmpeg.org) and
play them with `\animategraphics`, or use `\includevideo` (below), which plays GIFs without splitting.

```sh
mkdir frames
ffmpeg -i anim.gif frames/f%d.png      # add -vf "fps=10,scale=640:-1" to shrink a large GIF
ls frames | wc -l                      # number of frames
```

```latex
\usepackage{animate}
\animategraphics[autoplay,loop,width=0.5\textwidth]{10}{frames/f}{1}{30}   % 30 = number of frames
```

</details>

### Videos

TeXora includes the LaTeX package `texoravideo` and finds it when compiling. GIFs play by themselves;
MP4 and other formats play through ffmpeg or VLC installed on the computer. TeXora contains no video
decoder and never downloads or installs one.

```latex
\usepackage{texoravideo}
\hypersetup{hidelinks}
...
\includevideo[width=0.5\textwidth]{clip.mp4}
\includevideo[height=5cm][loop,autoplay]{media/wave.gif}
```

<details>
<summary>Options, players, sound and safety</summary>
<br>

**Options.** The first optional argument goes to `\includegraphics`. The second takes:

| Option               | Effect                                                              |
| -------------------- | ------------------------------------------------------------------- |
| `autoplay`           | Starts when the slide is shown                                      |
| `loop`               | Restarts at the end                                                 |
| `mute`               | Plays without sound (in the slide and in VLC)                       |
| `posterat=<percent>` | Takes the poster from that point of the clip (default: first frame) |
| `poster=<image>`     | Uses an image as the poster                                         |

**Files.** The video must be in the `.tex` file's folder or a folder inside it, written with its folder:
`{videos/clip.mp4}`. Names may contain spaces and accents, but not `%` or `#`. Before compiling,
TeXora makes one poster per video in a `texora-video` folder, updated only when the clip changes, so
every PDF viewer shows the poster; only TeXora plays the clip.

**Playback.** Click to play or pause; another slide stops the clip, while overlay steps of the same
slide keep it playing. **Settings > Video** chooses the player (Auto, ffmpeg, VLC or Off), each with a
path field and a **Test** button. ffmpeg plays inside the slide; VLC plays in its own window. Without
a player, the clip shows "This video needs VLC or ffmpeg installed on your computer." with **Get VLC**,
**Get ffmpeg** and, on macOS and Linux, **Copy install command**. TeXora only copies the command; it
never runs it.

**Sound.** With ffmpeg, the sound plays through ffplay, part of every ffmpeg install. It starts,
pauses and loops with the picture and stops on leaving the slide. The picture follows the sound's own
clock, so the two stay in sync (within about 5 ms in tests). **Settings > Video > Play sound** turns
sound on or off. GIFs have no sound. With VLC only (no ffmpeg), VLC plays the sound itself in its own
window; **Play sound** and `mute` apply there too.

**Problems.** A missing file, a file that is not a video, or a path outside the presentation's folder
(absolute paths, `..`, links leading out, web addresses) shows a message over the poster and under
Problems.

**Safety.**

- ffmpeg, ffplay and VLC run as separate programs with an argument list, never through a shell, and
  end when the slide changes.
- ffmpeg and ffplay read only local files (`-protocol_whitelist file`) in real video formats, so a
  playlist or concat list disguised as a video cannot reach other files or the web.
- VLC receives the full path after `--`, and only files whose content is a video.
- The program and its arguments come only from Settings. A presentation, or any file in its folder,
  cannot set them.

</details>

<details>
<summary>Clips that open in an external video player (<code>\movie</code>, media9)</summary>
<br>

Beamer's `\movie` (`multimedia` package) and media9's `\includemedia` show a play button in the
presenter view. A click opens the clip in the system's video player, which can be moved to the
projector; some players start paused.

```latex
\usepackage{multimedia}
\movie[width=4cm,height=2.25cm]{\fbox{Play}}{clip.mp4}
```

- A media9 clip set to `activate=pageopen` opens when its slide is shown while the audience window is
  open.
- Keep a `\movie` file in the presentation's folder. Clips embedded by media9 are extracted by TeXora.
- `\movie[externalviewer]`, `\href{run:...}` and `\href{file:...}` links are not opened.
- These clips do not play inside the slide; use `\includevideo` for that.

</details>

### Presenting

- **F5** opens the audience window, fullscreen on a second display; on a single display, **F11**
  makes it fullscreen.
- **B** blanks the screen, **W** shows a whiteboard, **F** freezes the audience view.
- Ink, the laser pointer and zoom appear on the audience screen as they are used.

### Contents, analytics and settings

- **Contents** (**T**) lists the talk's sections in colour, with bookmarks and questions; **Back to
  where I was** returns to the previous position.
- **Analytics** (**Y**) times every rehearsal slide by slide, compares it with the previous run and
  exports CSV.
- **Settings** covers text size, the LaTeX engine, required packages, video, notes, updates and every
  keyboard shortcut.
- **Settings > Language** chooses the language of the interface: English or Simplified Chinese. The
  whole interface, the built-in guide and the `--help` text change at once, and the choice is
  remembered.
- The **?** menu has **Documentation**, **System check**, **Command-line tool**, **Keyboard
  shortcuts**, **Check for updates** and **About TeXora**.

<details>
<summary>Updates</summary>
<br>

When a new version is published, a notice appears in the lower right corner with **See what's new**,
**Later** and **Skip this version**, and a dot appears on the **?** button. The update box links the
package for the current system and the release page; installing over the existing copy keeps the
settings. **? > Check for updates** checks at any time; **Settings > Updates** turns automatic checks
off. See [CHANGELOG.md](CHANGELOG.md) for the changes in each version.

</details>

## Keyboard shortcuts

On macOS, **Cmd** is used; on Windows and Linux, **Ctrl**. All shortcuts can be changed in Settings.
In the LaTeX studio, single-key shortcuts are inactive while editing.

<details>
<summary>All default shortcuts</summary>
<br>

| Action                      | Keys                | Action                  | Keys                     |
| --------------------------- | ------------------- | ----------------------- | ------------------------ |
| Next / previous step        | Right, Space / Left | Next / previous slide   | Shift+Right / Shift+Left |
| First / last slide          | Home / End          | Go to slide             | G                        |
| Audience window             | F5                  | Fullscreen              | F11                      |
| Black / white screen        | B / W               | Freeze audience         | F                        |
| Overview                    | O                   | Split view              | S                        |
| Large slide                 | N                   | Back                    | Backspace                |
| Start or pause timer        | P                   | Stop and reset timer    | R                        |
| Set talk duration           | U                   | Bookmark slide          | M                        |
| Mark a question             | Q                   | Contents                | T                        |
| Search                      | Cmd/Ctrl+F          | Analytics               | Y                        |
| Pointer / pen / highlighter | V / D / H           | Eraser / laser          | X / L                    |
| Zoom to area                | Z                   | Zoom in / out / fit     | + / - / 0                |
| Undo ink / clear ink        | Cmd/Ctrl+Z / C      | Edit notes / save notes | E / Cmd/Ctrl+Enter       |
| LaTeX studio                | Cmd/Ctrl+E          | Compile LaTeX           | Cmd/Ctrl+B               |
| Save source                 | Cmd/Ctrl+S          | Check required packages | Cmd/Ctrl+K               |
| Toggle comment (studio)     | Cmd/Ctrl+/          | Comment / uncomment     | Cmd/Ctrl+] / Cmd/Ctrl+[  |
| Open file                   | Cmd/Ctrl+O          | Home                    | Cmd/Ctrl+L               |
| Close tab                   | Cmd/Ctrl+W          | Settings                | Cmd/Ctrl+,               |
| Cancel                      | Esc                 |                         |                          |

</details>

## Command line

```sh
TeXora talk.pdf          # open a presentation (PDF or .tex)
TeXora --self-test       # check this copy; exit code 0 means it passed
TeXora --help            # all options
```

**? > Command-line tool** shows the path of the installed copy.

## Building from source

Requires [Rust](https://rustup.rs) (stable). The PDF engine (PDFium) is downloaded on the first build.

```sh
git clone https://github.com/BluesilveEmperor/TexOra--SS.git
cd TeXora
cargo build --release
./target/release/TeXora
cargo test
```

<details>
<summary>Project layout</summary>
<br>

| Path                 | Content                                                                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `src/`               | The application, in Rust with [egui](https://github.com/emilk/egui) and [PDFium](https://pdfium.googlesource.com/pdfium/) |
| `assets/`            | Icon, Home picture, fonts, sample lecture, templates and the `texoravideo` LaTeX package                                  |
| `scripts/`           | Packaging, installer, release and icon scripts                                                                            |
| `installer/`         | Windows installer (Inno Setup)                                                                                            |
| `.github/workflows/` | Continuous tests and the release workflow                                                                                 |
| `screenshots/`       | Pictures used in this README                                                                                              |

</details>

## Licence

Copyright 2026 Khalid Alkhaldi. Licensed under [PolyForm Noncommercial 1.0.0](LICENSE): free to use,
modify and share for any non-commercial purpose, including personal use, study, teaching, research and
non-profit work. Commercial use requires the author's permission. The app icon and Home picture are the
author's artwork and are not covered by the licence. Libraries and fonts are listed in
[CREDITS.md](CREDITS.md) and [THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md).
