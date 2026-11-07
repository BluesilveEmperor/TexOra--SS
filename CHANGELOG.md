# Changelog

All notable changes to TeXora. The newest version is first.

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
