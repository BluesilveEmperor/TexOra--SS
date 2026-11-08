# Credits

TeXora is original code by Khalid Alkhaldi, licensed under PolyForm Noncommercial 1.0.0 (see `LICENSE`). The
bundled sample lecture and the five presentation templates are neutral placeholder text written for it.

- PDFium, prebuilt by bblanchon/pdfium-binaries: BSD-3-Clause and Apache-2.0. Downloaded into `vendor/` when building.
- md-5 (names video posters): MIT or Apache-2.0. GIFs are decoded by the image crate.
- No video decoder, ffmpeg, VLC, libvlc or codec library is included. MP4 playback uses VLC or ffmpeg
  installed on your computer, started as separate programs. Every library inside TeXora, with its
  licence, is listed in `THIRD-PARTY-LICENSES.md`.
- egui and eframe: MIT or Apache-2.0.
- pdfium-render: MIT or Apache-2.0.
- ureq, serde, serde_json, image, rfd, display-info, dirs: MIT or Apache-2.0 style licences.
- Inter font (Rasmus Andersson): SIL Open Font License 1.1, in `assets/fonts/LICENSE-Inter.txt`.
- Hack font (bundled with egui): MIT. Ubuntu Light font (bundled with egui): Ubuntu Font Licence.
- The app icon (`assets/icon.png`), a hummingbird above a presentation screen, and the Home background
  (`assets/home-background.png`) are artwork by the project's owner, not covered by the licence.
- Beamer and LaTeX are used through your own TeX installation and are not included.
