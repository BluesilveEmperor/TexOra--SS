# Getting started

To use TeXora from a terminal, choose **? > Command-line tool**. To see what this computer has for TeXora, choose **? > System check**.

Open a PDF or a LaTeX file from the start page, or drop one on the window. Choose **New LaTeX presentation** to pick one of eight templates, each with its own colours and style: Classic Green, Midnight, Minimal White, Conference and Sunrise, and three animated ones, Motion Lab, Flipbook and Media Studio, whose slides show animations, GIF frames and a video clip playing. Write your presentation in your own LaTeX editor: TeXora rebuilds the slides every time you save it. Recent files stay on the start page.

## Giving a talk

- Press **F5** to open the audience window. With a second display it goes fullscreen there. With one display, press **F11**.
- Move with the arrow keys, Space or Page Up and Page Down. Add **Shift** to skip the overlay steps of a slide.
- The buttons around the slide number go to the first, previous, next and last slide.
- To go to a slide, click the number in the toolbar, or press **G**, then type it and press Enter.
- Press **O** for the overview, and **Backspace** to return to where you were.
- The row at the top shows whether you are ready: how many displays there are, whether a duration is set and whether notes were found. Beside it is the time of day. **Present** opens the audience window, like **F5**; while it is open the row shows what the audience sees.
- Under the main panel are the timer, then the arrows that go back and on, with the slide number and overlay step between them.
- The line at the bottom shows your talk's sections in their colours, with a black mark where you are. Click a section to go to its first slide.

## Views

- **Presenter**: your notes with the current and next slide.
- **Large slide**: the slide fills the main area, with notes beside it and the drawing tools at its left.
- **Split (S)**: slide and notes side by side in one window. Drag the divider to resize them.
- **Overview (O)**: every slide at once.

Every panel fits its space by itself. Drag a divider between panels to resize them, in any view. Double-click a divider to fit it automatically again, or choose **Fit all panels automatically** in the View menu.

## Tabs

Each presentation has its own tab at the top, as in a browser. Switching keeps your place, notes, drawing and timer, and a running timer keeps counting. Home closes when you open a file; the TeXora icon at the top left brings it back. Press the **x** on a tab, or **Cmd/Ctrl+W**, to close it; your work is saved first. Closing your last presentation returns to Home, and closing Home with nothing else open closes TeXora. The audience window stays on its presentation while you use another tab.

## Bookmarks and questions

- Press the bookmark button, or **M**, to bookmark the current slide. Press it again to remove it.
- Press the question button, or **Q**, to mark a slide where someone asked something.

Marks show on the slide for you only, never for the audience, and bookmarks also show on the section line at the bottom. They are saved with the presentation. Open **Contents (T)** to see them all and jump to one. Each section has its own bookmark colour: click the coloured dot beside a section to change it.

## Audience screen

- **B** shows a black screen.
- **W** shows a white screen you can write on.
- **F** freezes the audience on one slide while you look ahead privately.

## Timer

The timer is under the main panel, left of the slide number. Click the duration beside the elapsed time (**Set duration** until there is one) to set the length of your talk; a label at the bottom then shows the time left, or how far ahead or behind you are. When you are behind, a button at the bottom right offers to skip the next slides that would make up the time. Use the play, pause and stop buttons, or click the elapsed time itself to pause and resume. Stop sets the clock back to zero and saves the run for Analytics. The time of day is in the top row, beside **Present**.

## Zoom

Turn the mouse wheel over a slide, or pinch on a trackpad, to zoom around the pointer. This works on the current slide, the next slide and a PDF notes page, in every view. For the current slide you can also use the toolbar buttons or the keys **+**, **-** and **0**. Drag to move around an enlarged slide, and double-click to fit it again. Over text notes, the same gestures change the text size.

## Drawing and pointing

- **D** pen, **H** marker, **X** eraser.
- **L** laser pointer, with colours in the toolbar.
- **Z** then drag to zoom into an area.
- **V** returns to the normal pointer.

## Speaker notes

Notes come from the notes pages in your PDF, from `\note{}` in your LaTeX, or from a notes file in the same folder as the presentation, named after it: for `Lecture.pdf` it is `Lecture.note.md` (Settings > Speaker notes shows the exact file). Notes saved as `Lecture.notes.md` by earlier versions are read too. In that file, each `## Slide N` heading starts a slide's notes.

To edit the current slide's notes, press the notes button beside the pointer, or **E**. Type, then press the tick or **Cmd/Ctrl+Enter** to save, or the cross or **Esc** to cancel. On a slide without notes, **Write notes** does the same. For a PDF, your notes are saved in that file (for `Lecture.pdf`, `Lecture.note.md` beside it). For a `.tex` presentation they are saved in the source itself, as `\note{...}` in the slide, so the presenter and the LaTeX studio always show the same notes. A note shows at once when you save it; the slides rebuild in the background. To keep the .tex file unchanged, untick **Also save in the .tex file** under the editor (or in **Settings > Speaker notes**): notes then go into the `.note.md` file, as for a PDF. Either way they open with the presentation next time. Saving with no text removes the slide's note. Notes can be in any language; Arabic is typed and shown right to left. They are saved as UTF-8. The View menu chooses which notes are shown. Text notes support **bold**, *emphasis*, lists and headings. To make PDF notes pages, add this to your Beamer preamble:

```
\setbeameroption{show notes on second screen=right}
```

## LaTeX studio

Open a `.tex` file, then press **Cmd/Ctrl+E**, the studio button in the toolbar, or **View > LaTeX studio**. The source is on the left, the slide and its notes on the right, and the compile messages along the bottom.

- Slides rebuild shortly after you stop typing. Turn this off with the **Live** box.
- **Compile** (Cmd/Ctrl+B) builds now; **Save** (Cmd/Ctrl+S) saves the source.
- Double-click a slide to find its source. Moving the cursor shows the matching slide (**Follow cursor**).
- **Cmd/Ctrl+/** comments out the current or selected lines, or uncomments them if they all are. **Cmd/Ctrl+]** comments and **Cmd/Ctrl+[** uncomments. Change these in Settings.
- The panel at the bottom holds problems, packages and the full log. Drag its top edge to resize it.
- **Check packages**, also in the Tools menu, lists anything your file needs that is missing, and how to get it. **Settings > Required LaTeX packages** shows the same list.
- You can also keep editing in your own LaTeX editor: each time you save, the slides rebuild.

TeXora needs TeX Live, MacTeX or MiKTeX installed to compile. It picks pdfLaTeX, XeLaTeX or LuaLaTeX by itself, or you can choose one. PDF presentations need nothing else.

## Animations and videos

Animations made with the LaTeX **animate** package play on the slide, as in Adobe Acrobat. Those set to `autoplay` start when their slide is shown and stop when you move on; `loop` and `palindrome` work too, at the animation's own speed. Click an animation to play or pause it. In the presenter view, the bar under it has first, previous, play/pause, next and last frame buttons, and the audience sees the same frame.

Video clips added with `\movie` (multimedia) or `\includemedia` (media9) show a play button on your screen. Click the clip to play it in your computer's video player; clips set to start by themselves open when their slide is shown while the audience window is open. Keep a `\movie` file in the same folder as the presentation.

## After the talk

**Analytics (Y)** shows the time spent on each slide, what stands out, and how the run compares with the previous one. Questions you marked are counted here too.

## Updates

TeXora looks for a new version when it starts and every few hours while it stays open. When one is out, a notice appears in the lower right corner by itself: **See what's new** shows what changed and a download for this computer, **Later** hides it until TeXora starts again, and **Skip this version** hides it until an even newer one comes out. A dot on the **?** button stays either way. The notice never appears while the audience window is open. Choose **Check for updates** in the **?** menu to ask at any time. You can turn the automatic look off in **Settings**.

## Keyboard shortcuts

Every shortcut is listed in **Settings**. Click one, then press the new key to change it.
