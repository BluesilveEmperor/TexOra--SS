<p align="center">
  <img src="assets/icon.png" alt="TeXora icon" width="128">
</p>

<h1 align="center">TeXora</h1>

<p align="center">
  <b>Present PDF and LaTeX Beamer slides with confidence.</b><br>
  Free for non-commercial use, with its source code open to read, for macOS, Windows and Linux.
</p>

<p align="center">
  <a href="https://github.com/khalidt/TeXora/releases/latest">Download</a> ·
  <a href="#features">Features</a> ·
  <a href="#using-texora">Guide</a> ·
  <a href="#keyboard-shortcuts">Shortcuts</a> ·
  <a href="#building-from-source">Build</a>
</p>

![The presenter view: notes, current and next slide, slide number, elapsed time, duration and clock](screenshots/presenter.png)

TeXora is a presenter for **LaTeX Beamer** and **PDF** slides. You see your notes, the current and
next slide and the time; the room sees only the slide. Open a `.tex` file and TeXora compiles it, then
rebuilds the slides every time you save it in your own editor.

## Screenshots

| Home | Presenter view |
|---|---|
| ![Home page with recent files](screenshots/home.png) | ![Presenter view with notes, current and next slide](screenshots/presenter.png) |

| Slide and notes, side by side | Every slide at a glance |
|---|---|
| ![Split view with the audience slide, notes and next slide](screenshots/split.png) | ![Overview of all slides](screenshots/overview.png) |

The audience window shows only the slide, fullscreen on a projector or second screen:

![The audience window](screenshots/audience.png)

## Features

- **Presenter view**: notes, current and next slide, slide number and overlay step, elapsed time, talk
  duration with pace (ahead or behind), and a clock. Also a large-slide view and a split view.
- **Audience window** (F5): fullscreen on a second display, or press F11 on one display. Black screen
  (B), whiteboard (W) and freeze (F) to look ahead privately.
- **LaTeX Beamer**: open a `.tex` file and it compiles with pdfLaTeX, XeLaTeX or LuaLaTeX (chosen for
  you from `% !TEX program` or the packages used). Edit it in any editor; the slides rebuild when you
  save. Compile errors are shown with their line until fixed.
- **Required packages**: Settings lists every package, theme and program your file needs, marked
  Installed or Needs download, with how to get it.
- **Speaker notes** from PDF notes pages (Beamer's `show notes on second screen`), from `\note{}` in the
  source, from a `.pdfpc` file, or from a Markdown file beside the presentation. **Edit notes** for any
  slide right in the notes panel.
- **Drawing and pointing**: pen, highlighter, eraser, a laser pointer in several colours, and zoom into
  an area. Ink is kept per slide.
- **Zoom** any slide with the wheel or a pinch, around the pointer.
- **Bookmarks and questions**: mark slides in their section's colour, and mark where questions came up.
  Only you see them.
- **Contents and search** in a sidebar, from the PDF outline and text.
- **Analytics**: time on each slide for every rehearsal, compared with the last one.
- **Tabs**: several presentations open at once, each keeping its place, ink and timer.
- **Configurable shortcuts**, a **System check** (? menu) that shows what your computer has, a
  **command-line tool**, and **update notices**: TeXora looks for a new release when it starts and every
  few hours while it is open, and tells you in a small notice with a download for your computer.

## Download and install

Get TeXora from the [latest release](https://github.com/khalidt/TeXora/releases/latest).

| System | Installer | Portable |
|---|---|---|
| macOS, Apple chips (M1 and later) | `TeXora-<version>-mac-arm64.dmg` | `…-mac-arm64.zip` |
| macOS, Intel | `TeXora-<version>-mac-x64.dmg` | `…-mac-x64.zip` |
| Windows 10 and 11 (x64) | `TeXora-<version>-windows-x64-setup.exe` | `…-windows-x64.zip` |
| Debian, Ubuntu and relatives (x64) | `texora_<version>_amd64.deb` | `TeXora-<version>-linux-x64.tar.gz` |
| Debian, Ubuntu and relatives (arm64) | `texora_<version>_arm64.deb` | `TeXora-<version>-linux-arm64.tar.gz` |

- **macOS**: open the `.dmg` and drag **TeXora** to **Applications**. The first time, macOS asks you
  to confirm it; see [Opening TeXora on a Mac the first time](#opening-texora-on-a-mac-the-first-time).
- **Windows**: run the setup program. It installs for you alone without administrator rights (or for
  everyone, if you choose), adds TeXora to the Start menu and to "Open with" for PDF and `.tex`
  files, and appears in *Installed apps* for uninstalling. Windows may warn about an unknown publisher.
- **Linux**: `sudo apt install ./texora_<version>_amd64.deb`, then start **TeXora** from the
  menu or run `texora`. Or unpack the `.tar.gz` anywhere and run `./TeXora`. Needs glibc 2.31 or
  newer (Ubuntu 20.04, Debian 11 or later) and an X11 or Wayland desktop.

### Opening TeXora on a Mac the first time

TeXora is signed *ad hoc*, not with a paid Apple Developer ID, so macOS cannot check who made it and
blocks it the first time it is opened.

On **macOS 15 Sequoia and later**, the old right-click > **Open** shortcut no longer works. Instead:

1. Open **TeXora** once and click **Done** on the warning.
2. Go to **System Settings > Privacy & Security**.
3. Scroll to the **Security** section, find the line saying TeXora was blocked, and click **Open Anyway**.
4. Enter your password or use Touch ID, and confirm.

On **macOS 14 Sonoma and earlier**, right-click (or Control-click) TeXora in Applications, choose
**Open**, then **Open** again.

If you prefer Terminal, this removes the download mark so TeXora opens normally:

```sh
xattr -dr com.apple.quarantine /Applications/TeXora.app
```

You only need to do this once; after that TeXora opens like any other app.

Every file is listed with its checksum in `SHA256SUMS.txt`, and `VALIDATION.txt` shows how each one was
tested before the release.

Nothing else is needed to present PDFs: each package carries the program and its PDF engine. To compile
`.tex` files, install a TeX distribution: **MacTeX** on macOS, **MiKTeX** or TeX Live on Windows,
**TeX Live** on Linux.

## Using TeXora

### Getting started

1. Start TeXora. **Home** shows your recent files and ways to begin.
2. Choose **Open PDF presentation** or **Open LaTeX project**, pick a recent file, or drop a file on the
   window. **Try the sample lecture** opens a ready-made talk to explore.
3. A `.tex` file is compiled for you. Keep editing it in your own LaTeX editor: each time you save, the
   slides rebuild.
4. Press **F5** to show the slides to the room, then move with the arrow keys or **Space**.

Each presentation opens in its own tab, and keeps its place, ink and timer. Hover over any button to
see its name and shortcut.

### The presenter screen

![The presenter screen: notes on the left, current and next slide on the right, timers below](screenshots/presenter.png)

| Area | What it shows |
|---|---|
| **Notes** | Your speaker notes for the slide on screen. Scroll or pinch to change the text size. |
| **Current slide** | What the room sees right now. |
| **Next slide** | What comes next, including the next overlay step. |
| **Slide number** | Slide, page and overlay step, with the section name and a progress bar. Click it and type a number to jump. |
| **Time elapsed** | Starts with the first move. Click the time, or use the play, pause and stop buttons. |
| **Duration** | Click to set how long the talk should take; it then shows the time left and whether you are ahead or behind. |
| **Clock** | The time of day. |

Drag the dividers between the panels to resize them; **View > Fit all panels automatically** resets them.

### The toolbar

The second row of buttons acts on the open presentation. From left to right:

#### Views

| Button | Name | What it does | Key |
|:---:|---|---|---|
| <img src="screenshots/icons/presenter.png" width="28" alt="Presenter"> | **Presenter** | Notes with the current and next slide. |  |
| <img src="screenshots/icons/large-slide.png" width="28" alt="Large slide"> | **Large slide** | A large current slide with the notes beside it. | N |
| <img src="screenshots/icons/split.png" width="28" alt="Slide and notes split"> | **Slide and notes split** | The slide and the notes side by side, with the next slide. | S |
| <img src="screenshots/icons/overview.png" width="28" alt="Slide overview"> | **Slide overview** | Every slide at a glance. Click one to go there. | O |

#### The room's screen

| Button | Name | What it does | Key |
|:---:|---|---|---|
| <img src="screenshots/icons/audience.png" width="28" alt="Audience window"> | **Audience window** | Opens the slides in their own window, fullscreen on a projector or second display. | F5 |
| <img src="screenshots/icons/black.png" width="28" alt="Black screen"> | **Black screen** | Blanks the room's screen. Press again to show the slide. | B |
| <img src="screenshots/icons/whiteboard.png" width="28" alt="Whiteboard"> | **Whiteboard** | Shows a white page to draw on. | W |
| <img src="screenshots/icons/freeze.png" width="28" alt="Freeze"> | **Freeze** | Keeps the room on this slide while you look ahead privately. | F |

#### Notes

| Button | Name | What it does | Key |
|:---:|---|---|---|
| <img src="screenshots/icons/edit-notes.png" width="28" alt="Edit notes"> | **Edit notes** | Opens the notes of this slide for editing, in the notes panel. | E |
| <img src="screenshots/icons/save-notes.png" width="28" alt="Save notes"> | **Save notes** | Shown while editing. Saves your changes. | Cmd/Ctrl+Enter |
| <img src="screenshots/icons/cancel-notes.png" width="28" alt="Cancel"> | **Cancel** | Shown while editing. Leaves the notes as they were. | Esc |

#### Drawing and pointing

| Button | Name | What it does | Key |
|:---:|---|---|---|
| <img src="screenshots/icons/pointer.png" width="28" alt="Pointer"> | **Pointer** | The normal pointer: click to move on, no drawing. | V |
| <img src="screenshots/icons/pen.png" width="28" alt="Pen"> | **Pen** | Draw on the slide. The room sees it too. | D |
| <img src="screenshots/icons/highlighter.png" width="28" alt="Highlighter"> | **Highlighter** | A wide, see-through stroke to mark text. | H |
| <img src="screenshots/icons/laser.png" width="28" alt="Laser pointer"> | **Laser pointer** | A bright dot that follows your pointer, with a short trail. | L |
| <img src="screenshots/icons/eraser.png" width="28" alt="Eraser"> | **Eraser** | Removes the strokes you touch. | X |
| <img src="screenshots/icons/zoom-area.png" width="28" alt="Zoom to area"> | **Zoom to area** | Drag a box on the slide to enlarge that part for everyone. | Z |
| <img src="screenshots/icons/undo.png" width="28" alt="Undo ink"> | **Undo ink** | Takes back the last stroke. | Cmd/Ctrl+Z |
| <img src="screenshots/icons/colour.png" width="28" alt="Colour"> | **Colour** | Chooses the ink colour, or the laser colour while the laser is on. |  |

Ink is kept with each slide, so it is still there when you come back. **C** clears the ink on the slide.

#### Zoom

| Button | Name | What it does | Key |
|:---:|---|---|---|
| <img src="screenshots/icons/zoom-out.png" width="28" alt="Zoom out"> | **Zoom out** | Makes the slide smaller again. | - |
| <img src="screenshots/icons/zoom-in.png" width="28" alt="Zoom in"> | **Zoom in** | Enlarges the slide. The mouse wheel or a pinch zooms around the pointer. | + |
| <img src="screenshots/icons/fit.png" width="28" alt="Fit to page"> | **Fit to page** | Shows the whole slide again. | 0 |

#### Moving through the slides

| Button | Name | What it does | Key |
|:---:|---|---|---|
| <img src="screenshots/icons/first.png" width="28" alt="First slide"> | **First slide** | Goes to the start. | Home |
| <img src="screenshots/icons/previous.png" width="28" alt="Previous"> | **Previous** | One step back. | Left |
| <img src="screenshots/icons/next.png" width="28" alt="Next"> | **Next** | One step on: the next overlay or the next slide. | Right, Space |
| <img src="screenshots/icons/last.png" width="28" alt="Last slide"> | **Last slide** | Goes to the end. | End |

Between **Previous** and **Next**, the box shows the slide number: click it, type a number and press
Enter. **Shift+Right** and **Shift+Left** skip whole slides, and **Backspace** goes back to where you were.

### The buttons at the top right

| Button | Name | What it does | Key |
|:---:|---|---|---|
| <img src="screenshots/icons/bookmark.png" width="28" alt="Bookmark"> | **Bookmark** | Marks this slide in the colour of its section, to find it again quickly. | M |
| <img src="screenshots/icons/question.png" width="28" alt="Question"> | **Question** | Marks that someone asked a question on this slide. Analytics counts them. | Q |
| <img src="screenshots/icons/contents.png" width="28" alt="Contents and bookmarks"> | **Contents and bookmarks** | Opens the sidebar with sections, bookmarks and questions. | T |
| <img src="screenshots/icons/search.png" width="28" alt="Search"> | **Search** | Finds a word or phrase in the slides and notes. | Cmd/Ctrl+F |
| <img src="screenshots/icons/analytics.png" width="28" alt="Analytics"> | **Analytics** | Shows how long you spent on each slide. | Y |
| <img src="screenshots/icons/settings.png" width="28" alt="Settings"> | **Settings** | Text size, LaTeX engine, required packages, notes, updates and shortcuts. | Cmd/Ctrl+, |
| <img src="screenshots/icons/help.png" width="28" alt="Help"> | **Help** | Documentation, system check, command-line tool, shortcuts, updates and About. |  |

These open a sidebar on the left. Click the same button again, or the cross, to close it.

### Speaker notes

TeXora shows notes from any of these:

- **Beamer notes pages**: add `\setbeameroption{show notes on second screen=right}` to your preamble.
- **`\note{...}`** in your `.tex` source.
- A **`.pdfpc`** file beside the PDF.
- A **notes file** saved next to the presentation (see below), with a `## Slide N` heading for each
  slide. You can also write it yourself in any text editor.

**Where notes are saved.** Notes go into a file in the **same folder as the presentation you opened**,
named after it with `.note.md` in place of `.pdf` or `.tex`:

| You opened | Notes are saved as (same folder) |
|---|---|
| `Lecture.pdf` | `Lecture.note.md` |
| `Week 3 - Graphs.pdf` | `Week 3 - Graphs.note.md` |
| `Thesis defense.tex` | `Thesis defense.note.md` |

Keep this file with the presentation when you move or share it, and the notes come along. Notes
saved by earlier versions as `Lecture.notes.md` are still read, and take the new name the next time
you save.
**Settings > Speaker notes** shows the exact file for the presentation that is open.

**Edit notes.** Click <img src="screenshots/icons/edit-notes.png" width="22" alt="Edit notes"> or press
**E**, type, then click <img src="screenshots/icons/save-notes.png" width="22" alt="Save"> or press
**Cmd/Ctrl+Enter** to save; **Esc** cancels. On a slide without notes, **Write notes** in the notes panel
does the same. Your notes are saved in the presentation's notes file (for `Lecture.pdf`, that is
`Lecture.note.md` in the same folder), so they open with it next time. Markdown works: `**bold**`,
`*emphasis*`, `- lists` and `# headings`. Clearing all the text brings back the notes from the PDF or
source.

**Any language.** Notes can be written in any language, including Arabic, Chinese, Japanese and
Korean, and emoji. The fonts come from your computer. Arabic notes are shown right to left, aligned to
the right, both while you type them and when they are shown. Notes are
saved as UTF-8; files saved as UTF-16 or in older encodings are read too.

![Editing the notes of a slide](screenshots/notes-edit.png)

### Presenting to the room

- **F5** opens the audience window. With a second display it goes fullscreen there; on one display,
  press **F11** to make it fullscreen.
- **B** blanks the screen, **W** shows a whiteboard, and **F** freezes the room on one slide while you
  look ahead.
- Ink, the laser pointer and zoom are shown to the room as you use them.

### Contents, bookmarks and search

The **Contents** sidebar lists the sections of your talk, each in its own colour, with your bookmarks and
questions above them. Click any entry to go there; **Back to where I was** returns.

![The contents sidebar](screenshots/contents.png)

### Analytics

Every rehearsal is timed slide by slide. **Analytics** shows the total time, the time on each slide
compared with your previous run, what stands out, and where questions came up. **Save as CSV** keeps
the numbers.

![Analytics: time on each slide](screenshots/analytics.png)

### Settings

**Settings** has sections you can open and close: text size, the LaTeX engine, the packages your file
needs, where your notes are saved, updates, and every keyboard shortcut, which you can change.

![The settings sidebar](screenshots/settings.png)

### Help menu

The **?** button holds **Documentation** (a short guide inside the app), **System check** (what this
computer has for TeXora), **Command-line tool** (the path of your copy, with examples), **Keyboard
shortcuts**, **Check for updates** and **About TeXora**.

### Updates

When a new version is published, TeXora shows a notice in the lower right corner by itself, with
**See what's new**, **Later** and **Skip this version**, and a dot on the **?** button. The update box
offers the package for your computer and the release page; install it over your copy and your settings
stay. **? > Check for updates** asks at any time. Turn the automatic look off in **Settings > Updates**.
TeXora 1.0.0 shows the dot on **?** only. See [CHANGELOG.md](CHANGELOG.md) for what changed in each
version.

## Keyboard shortcuts

All can be changed in Settings. On macOS **Cmd** is used; on Windows and Linux, **Ctrl**.

| Action | Keys | Action | Keys |
|---|---|---|---|
| Next / previous step | Right, Space / Left | Next / previous slide | Shift+Right / Shift+Left |
| First / last slide | Home / End | Go to slide | G |
| Audience window | F5 | Fullscreen | F11 |
| Black / white screen | B / W | Freeze audience | F |
| Overview | O | Split view | S |
| Large slide | N | Back | Backspace |
| Start or pause timer | P | Stop and reset timer | R |
| Set talk duration | U | Bookmark slide | M |
| Mark a question | Q | Contents | T |
| Search | Cmd/Ctrl+F | Analytics | Y |
| Pointer / pen / highlighter | V / D / H | Eraser / laser | X / L |
| Zoom to area | Z | Zoom in / out / fit | + / - / 0 |
| Undo ink / clear ink | Cmd/Ctrl+Z / C | Compile LaTeX | Cmd/Ctrl+B |
| Edit notes | E | Save notes | Cmd/Ctrl+Enter |
| Check required packages | Cmd/Ctrl+K | Open file | Cmd/Ctrl+O |
| Home | Cmd/Ctrl+L | Close tab | Cmd/Ctrl+W |
| Settings | Cmd/Ctrl+, | Cancel | Esc |

## Command line

```sh
TeXora talk.pdf                          # open a presentation
TeXora --self-test                       # check this copy works (exit code 0 means it passed)
TeXora --help                            # every option
```

**? > Command-line tool** in the app shows the exact path of your copy.

## Building from source

Needs [Rust](https://rustup.rs) (stable). The PDF engine (PDFium) is downloaded on the first build.

```sh
git clone https://github.com/khalidt/TeXora.git
cd TeXora
cargo build --release
./target/release/TeXora
cargo test
```

## Project layout

| Path | What it holds |
|---|---|
| `src/` | The app, in Rust with [egui](https://github.com/emilk/egui) and [PDFium](https://pdfium.googlesource.com/pdfium/). |
| `assets/` | App icon (`assets/icon.png`), Home picture, fonts and the sample lecture. |
| `scripts/` | Packaging, installer, release and icon scripts. |
| `installer/` | The Windows installer (Inno Setup). |
| `.github/workflows/` | Continuous tests and the release workflow. |
| `screenshots/` | The pictures in this README. |

## Licence

Copyright 2026 Khalid Alkhaldi. Licensed under [PolyForm Noncommercial 1.0.0](LICENSE)
(<https://polyformproject.org/licenses/noncommercial/1.0.0>): free to use, change and share for any
non-commercial purpose, including personal use, study, teaching, research and non-profit work. Commercial
use needs the author's permission. The app icon and Home picture are the author's artwork and are not
covered by the licence. See [CREDITS.md](CREDITS.md) for the libraries and fonts used.

Made by Khalid Alkhaldi.
