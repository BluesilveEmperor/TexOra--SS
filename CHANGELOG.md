# Changelog

All notable changes to TeXora. The newest version is first.

## 1.5.1

### Fixed

- **Editing in LaTeX Studio.** After a click on Save, Compile, a checkbox or the slide, the editor lost the keyboard, so the next letters ran presenter shortcuts instead of typing: **B** blacked out the slide and the arrows changed slides and moved the cursor. The keyboard now stays with the editor; another text box (such as Search) still gets its own typing.
- **No more rebuilding as you type.** The **Live** box is gone: typing never saves or rebuilds by itself. Press **Compile** (Cmd/Ctrl+B), or **Save** (Cmd/Ctrl+S) with **Rebuild on save** on.
- A file opened from a terminal with a short path (`texora talk.tex`) now compiles and checks its packages; before, both failed with "No such file or directory".
- Videos play with their sound. The sound comes from ffplay, which is part of every ffmpeg install (TeXora finds it beside your ffmpeg), and follows the picture: it starts, pauses and starts again from the same place, starts again with a `loop` clip, and stops when you show another slide. TeXora still contains no decoder of its own.
- **Settings > Video > Play sound** turns video sound on or off (on by default), and the `mute` option of `\includevideo` now keeps one clip silent. GIFs have no sound.
- ffplay runs with the same safety as ffmpeg: no window, no shell, local files only and real video formats only.
- The Video Guide template's sample video has a soft chord, so you can hear sound working.
- **Sound with VLC only.** Without ffmpeg, a clip opened in VLC plays its sound in VLC (ffplay is not needed). The `mute` option and **Settings > Video > Play sound** off now also keep VLC silent, and Settings > Video says where the sound plays.
- **Picture and sound in sync.** The picture now follows the sound's own clock (ffplay reports it), so it waits the moment ffplay takes to start instead of running ahead. Measured with a clip that flashes and beeps each second: the picture was 730 ms ahead of the sound, now within about 5 ms, also after pause and play. After a pause the sound starts again exactly where the picture is (before, it could start seconds early, from the last key frame).

## 1.5.0

### Security

- ffmpeg opens a clip with `-nostdin -protocol_whitelist file` and a list of real video formats only, so a file that is really a playlist or a concat list cannot make ffmpeg read other files or web addresses. Tested with such files: plain ffmpeg follows them, TeXora's ffmpeg refuses them.
- VLC gets the clip's full path after `--`, so a file name starting with `-` is never read as an option, and only files that really are videos (from their first bytes, not their name): never a playlist.
- Which player runs, and where it is, comes only from your own Settings. A presentation, its `.tex` file, a PDF or anything in its folder can never set the program or add arguments; this is tested.

### New

- Videos and GIFs in your slides: `\usepackage{texoravideo}` and `\includevideo[width=0.5\textwidth]{clip.mp4}`. TeXora ships the LaTeX package and finds it by itself when it compiles. A second optional argument takes `autoplay`, `loop`, `posterat=<percent>` and `poster=<image>`.
- **GIFs play by themselves.** **MP4 playback uses VLC or ffmpeg installed on your computer**: with ffmpeg the clip plays inside the slide, with VLC in VLC's own window. TeXora contains no video decoder and never downloads or installs one.
- Settings > Video chooses the player (Auto, ffmpeg, VLC or Off), with a path and a **Test** button for each.
- A video that needs a player when none is installed says "This video needs VLC or ffmpeg installed on your computer.", with **Get VLC**, **Get ffmpeg** and **Copy install command**. TeXora only opens the pages and copies the command; it never runs it.
- Before compiling, TeXora makes each clip's poster in a `texora-video` folder (again only when the clip changes; a video's poster needs ffmpeg), so every PDF viewer shows the poster; a click in TeXora plays or pauses the clip, and another slide stops it and its ffmpeg.
- A missing file, a file that is not a video or a path outside the presentation's folder shows a message over the poster and under Problems.
- A video can be in the `.tex` file's folder or in any folder inside it; a video in a folder is written with the folder name, as in `\includevideo{videos/clip.mp4}`. "Video not found" says this in plain words.
- One poster per video, however often it is used; posters no longer used are removed.
- A new template, **Video Guide**, explains `\includevideo` slide by slide: what you need, a video, a looping GIF, every option, posters, overlays and what to do when a clip does not play. Its sample video and GIF are saved in a `media` folder beside the new file.

### Fixed

- Check packages finds packages, classes, themes and TikZ libraries of your own that sit beside the `.tex` file (or in a folder you name, as in `\usepackage{styles/house}`), as LaTeX does, and no longer reports them as missing or suggests `tlmgr install` for them (#3).
- The control bar under an animation (first, previous, play/pause, next, last) works again: its buttons now get the click, instead of the slide underneath taking it.

## 1.4.1

### Fixed

- In the LaTeX studio, the slide and notes preview keeps the width you drag it to, instead of jumping back when you let go.
- The black line at the right edge of the notes page is gone, in the presenter screen and the LaTeX studio; a faint frame shows the page's edge instead.
- The presenter screen no longer leaves an empty grey strip beside the notes page: the Notes column fits the page, and the slides take the rest.
- In the LaTeX studio, a header wider than the preview no longer pushes the slide past the window's edge.

### Improved

- Double-click the edge between the source and the preview to split them half and half again.
- The LaTeX studio's preview grows with its width: on a wide panel the notes move beside the slide, so the slide can use the full height.
- The notes page in the studio gets a box of its own shape, so it fills it with no empty margin.
- In the presenter screen, widening the slide column makes the current slide bigger; in the large slide view, the notes page and the next slide share their column at the same width.

### New

- **Wrap text** in the LaTeX studio wraps long lines at the editor's width; line numbers still count real lines.

## 1.4.0

### Improved

- A new look for the presenter screen, with the slide, notes and next slide on cards.
- A top row shows the open files, whether you are ready to present, and a **Present** button.
- The drawing tools sit beside the slide in the large slide view.
- The timer, then previous and next buttons with the slide number and overlay step, sit under the main panel.
- The talk's sections are shown as a coloured line with your place and your bookmarks; click one to go there.
- The time of day sits in the top row, beside **Present**; the pace shows under the sections.
- A note saved with Edit Note shows at once, without waiting for LaTeX to rebuild the slides.
- For a `.tex` presentation, **Also save in the .tex file** chooses whether notes go into the source as `\note{...}` or into the `.note.md` file.

### New

- When you fall behind, a button offers to skip slides to catch up.
- **New LaTeX presentation** on Home and in the File menu: choose one of eight templates.
- Three animated templates: Motion Lab, Flipbook and Media Studio, with animations, GIF frames and a video clip.
- The sample lecture opens with a 20-minute duration, a bookmark and example points in its notes.

## 1.3.0

### New

- Comment out or uncomment lines in the LaTeX studio with **Cmd/Ctrl+/**.
- With several lines selected, all of them are commented or uncommented at once.
- **Cmd/Ctrl+]** comments and **Cmd/Ctrl+[** uncomments; all three keys can be changed in Settings.
- Animations made with the LaTeX `animate` package now play, as in Adobe Acrobat.
- Animations start by themselves when set to (`autoplay`), loop or play back and forth, at their own frame rate.
- Click an animation to play or pause it; the presenter view has first, previous, play/pause, next and last frame buttons.
- Video clips (`\movie` or media9) show a play button and open in your video player; clips set to start by themselves do so during the talk.

## 1.2.0

### New

- Added LaTeX studio.
- Edit the `.tex` source beside a live preview of the slide and its notes.
- Slides rebuild shortly after you stop typing, or when you save.
- Problems with their line numbers, required packages and the full log in one panel.
- Double-click a slide to jump to its source; the slide follows the cursor as you move through the source.
- Open it with the studio button, **View > LaTeX studio** or **Cmd/Ctrl+E**; save with **Cmd/Ctrl+S**.
- **Check packages** opens the package list in the studio.

### Fixed

- Notes now stay the same in the presenter and the LaTeX studio.
- For a `.tex` presentation, a note edited in the presenter is saved into that slide's `\note{...}` in the source.
- A `\note` edited in the studio shows in the presenter, even when a `.note.md` file already exists.
- After a rebuild, the presenter stays on the slide you were on.
- A note being edited stays with its own presentation when you switch tabs.

## 1.1.0

### New

- Added Edit Note feature.
- Edit the notes of any slide in the notes panel: press **E** or click the edit button.
- Save with the tick button or **Cmd/Ctrl+Enter**; cancel with the cross or **Esc**.
- Notes file is saved in the same folder as the opened presentation, using the presentation's filename followed by `.note.md`.
- **Write notes** button on slides that have no notes yet.
- Notes can be written in any language: Arabic, Chinese, Japanese, Korean and more.
- Arabic notes read right to left, with words, numbers and punctuation in the right order.
- Typing Arabic in the note editor runs from the right edge, with words in reading order.
- Emoji show in colour.

### Fixed

- Text in Arabic, Chinese, Japanese, Korean and emoji no longer shows as empty boxes.
- Notes files saved as UTF-16 (for example by Windows Notepad) or in older encodings now open correctly.

## 1.0.1

### New

- Update notice appears by itself when a new version is out.
- The notice offers **See what's new**, **Later** and **Skip this version**.
- The update box offers the download for your computer.
- TeXora checks for updates at start and every few hours while open.

### Fixed

- The bin button in **Recent files** now removes the file at once, without flickering.

### Changed

- **New LaTeX presentation** on Home is switched off.

## 1.0.0

- First public release.
- Presenter and audience views, LaTeX Beamer compiling and speaker notes.
- Drawing tools, laser pointer, zoom, bookmarks, contents, search and analytics.
- Installers for macOS, Windows and Linux.
