//! The Beamer templates offered for a new presentation. Each is a complete .tex file in
//! assets/templates/, with two preview pictures made from it (its title slide and a content slide).
//! To change one, edit its .tex file and run scripts/make-template-previews.sh.
//! The animated templates also bring files of their own (pictures and a video clip), written into a
//! `media` folder beside the new presentation.

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    /// Shape and look, in a few words.
    pub tags: &'static str,
    pub tex: &'static str,
    pub title_png: &'static [u8],
    pub content_png: &'static [u8],
    /// Files the presentation needs, by path relative to it, such as `media/clip.mp4`.
    pub files: &'static [(&'static str, &'static [u8])],
}

macro_rules! template {
    ($id:literal, $name:literal, $about:literal, $tags:literal) => {
        template!($id, $name, $about, $tags, &[])
    };
    ($id:literal, $name:literal, $about:literal, $tags:literal, $files:expr) => {
        Template {
            id: $id,
            name: $name,
            about: $about,
            tags: $tags,
            tex: include_str!(concat!("../assets/templates/", $id, ".tex")),
            title_png: include_bytes!(concat!("../assets/templates/", $id, "-1.png")),
            content_png: include_bytes!(concat!("../assets/templates/", $id, "-5.png")),
            files: $files,
        }
    };
}

/// The pictures of the Flipbook template: the frames of a GIF, played as one animation.
const FRAMES: &[(&str, &[u8])] = &[
    ("media/frame-1.png", include_bytes!("../assets/templates/media/frame-1.png")),
    ("media/frame-2.png", include_bytes!("../assets/templates/media/frame-2.png")),
    ("media/frame-3.png", include_bytes!("../assets/templates/media/frame-3.png")),
    ("media/frame-4.png", include_bytes!("../assets/templates/media/frame-4.png")),
    ("media/frame-5.png", include_bytes!("../assets/templates/media/frame-5.png")),
    ("media/frame-6.png", include_bytes!("../assets/templates/media/frame-6.png")),
    ("media/frame-7.png", include_bytes!("../assets/templates/media/frame-7.png")),
    ("media/frame-8.png", include_bytes!("../assets/templates/media/frame-8.png")),
    ("media/frame-9.png", include_bytes!("../assets/templates/media/frame-9.png")),
    ("media/frame-10.png", include_bytes!("../assets/templates/media/frame-10.png")),
    ("media/frame-11.png", include_bytes!("../assets/templates/media/frame-11.png")),
    ("media/frame-12.png", include_bytes!("../assets/templates/media/frame-12.png")),
    ("media/frame-13.png", include_bytes!("../assets/templates/media/frame-13.png")),
    ("media/frame-14.png", include_bytes!("../assets/templates/media/frame-14.png")),
    ("media/frame-15.png", include_bytes!("../assets/templates/media/frame-15.png")),
    ("media/frame-16.png", include_bytes!("../assets/templates/media/frame-16.png")),
    ("media/frame-17.png", include_bytes!("../assets/templates/media/frame-17.png")),
    ("media/frame-18.png", include_bytes!("../assets/templates/media/frame-18.png")),
    ("media/frame-19.png", include_bytes!("../assets/templates/media/frame-19.png")),
    ("media/frame-20.png", include_bytes!("../assets/templates/media/frame-20.png")),
    ("media/frame-21.png", include_bytes!("../assets/templates/media/frame-21.png")),
    ("media/frame-22.png", include_bytes!("../assets/templates/media/frame-22.png")),
    ("media/frame-23.png", include_bytes!("../assets/templates/media/frame-23.png")),
    ("media/frame-24.png", include_bytes!("../assets/templates/media/frame-24.png")),
    ("media/frame-25.png", include_bytes!("../assets/templates/media/frame-25.png")),
    ("media/frame-26.png", include_bytes!("../assets/templates/media/frame-26.png")),
    ("media/frame-27.png", include_bytes!("../assets/templates/media/frame-27.png")),
    ("media/frame-28.png", include_bytes!("../assets/templates/media/frame-28.png")),
    ("media/frame-29.png", include_bytes!("../assets/templates/media/frame-29.png")),
    ("media/frame-30.png", include_bytes!("../assets/templates/media/frame-30.png")),
];

/// The video clip of the Media Studio template.
const CLIP: &[(&str, &[u8])] = &[("media/clip.mp4", include_bytes!("../assets/templates/media/clip.mp4"))];

pub const TEMPLATES: [Template; 8] = [
    template!("classic", "Classic Green", "Madrid theme with a calm green palette and a full information bar.", "16:9 · light · green"),
    template!("midnight", "Midnight", "Dark slides with amber highlights, easy on the eyes in a dim room.", "16:9 · dark · amber"),
    template!("minimal", "Minimal White", "Clean white slides, dark text and a single blue accent.", "16:9 · light · blue"),
    template!("conference", "Conference", "Berlin theme in deep wine red, with section progress along the top.", "16:9 · light · red"),
    template!("sunrise", "Sunrise", "Warm coral and teal on a soft cream page, in the classic 4:3 shape.", "4:3 · warm · coral"),
    template!("motion", "Motion Lab", "Dark slides with an orbit, a pendulum and a countdown drawn with TikZ: looping, back and forth, on a click and at changing speeds.", "16:9 · dark · animated"),
    template!("flipbook", "Flipbook", "Paper-white slides that play the frames of a GIF and bar charts that grow. The pictures come with it.", "4:3 · light · animated", FRAMES),
    template!("media", "Media Studio", "Violet slides with a video clip, a clip packed into the PDF, and charts that draw themselves. The clip comes with it.", "16:9 · light · video", CLIP),
];

/// Where the animated templates start in the list.
pub const FIRST_ANIMATED: usize = 5;

/// Writes the files template `t` needs into `dir`, leaving any that are already there alone.
/// Returns the names of those it could not write.
pub fn write_files(dir: &std::path::Path, t: &Template) -> Vec<String> {
    let mut failed = vec![];
    for (name, bytes) in t.files {
        let to = dir.join(name);
        if to.exists() {
            continue;
        }
        let ok = to.parent().map(|p| std::fs::create_dir_all(p).is_ok()).unwrap_or(true) && std::fs::write(&to, bytes).is_ok();
        if !ok {
            failed.push(name.to_string());
        }
    }
    failed
}

/// The template with this id, or the first one.
pub fn by_id(id: &str) -> usize {
    TEMPLATES.iter().position(|t| t.id == id).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_templates_with_previews() {
        let mut themes = std::collections::HashSet::new();
        for t in &TEMPLATES {
            assert!(t.tex.contains("\\begin{document}") && t.tex.contains("\\end{document}"), "{}", t.id);
            assert!(t.tex.contains("\\note{"), "{} has speaker notes", t.id);
            let theme = t.tex.lines().find(|l| l.starts_with("\\usetheme")).unwrap_or("");
            let colours: Vec<&str> = t.tex.lines().filter(|l| l.starts_with("\\definecolor")).collect();
            assert!(!colours.is_empty(), "{} sets its colours", t.id);
            themes.insert(format!("{theme}{}", colours.join("")));
            for png in [t.title_png, t.content_png] {
                let p = image::load_from_memory_with_format(png, image::ImageFormat::Png).expect("preview reads");
                assert_eq!(p.width(), 400, "{}", t.id);
            }
        }
        assert_eq!(themes.len(), TEMPLATES.len(), "every template looks different");
        for t in &TEMPLATES[FIRST_ANIMATED..] {
            assert!(t.tex.contains("\\usepackage{animate}"), "{} is animated", t.id);
            for (name, bytes) in t.files {
                let stem = name.split('-').next().unwrap_or(name);
                assert!(t.tex.contains(stem), "{} uses {name}", t.id);
                assert!(!bytes.is_empty(), "{name}");
            }
        }
        assert_eq!(by_id("sunrise"), 4);
        assert_eq!(by_id("nope"), 0);
    }

    #[test]
    fn every_template_compiles() {
        if crate::latex::find_tool("pdflatex").is_none() {
            return;
        }
        for t in &TEMPLATES {
            let dir = std::env::temp_dir().join(format!("texora-template-test-{}-{}", std::process::id(), t.id));
            std::fs::create_dir_all(&dir).unwrap();
            let tex = dir.join("talk.tex");
            std::fs::write(&tex, t.tex).unwrap();
            assert!(write_files(&dir, t).is_empty(), "{} files", t.id);
            let r = crate::latex::compile(&tex, crate::latex::Engine::PdfLatex);
            assert!(r.ok, "{} did not compile: {:?}", t.id, r.problems);
            assert!(r.problems.iter().all(|p| !p.is_error), "{}", t.id);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
