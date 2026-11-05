//! The Beamer templates offered for a new presentation. Each is a complete .tex file in
//! assets/templates/, with two preview pictures made from it (its title slide and a content slide).
//! To change one, edit its .tex file and run scripts/make-template-previews.sh.

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    /// Shape and look, in a few words.
    pub tags: &'static str,
    pub tex: &'static str,
    pub title_png: &'static [u8],
    pub content_png: &'static [u8],
}

macro_rules! template {
    ($id:literal, $name:literal, $about:literal, $tags:literal) => {
        Template {
            id: $id,
            name: $name,
            about: $about,
            tags: $tags,
            tex: include_str!(concat!("../assets/templates/", $id, ".tex")),
            title_png: include_bytes!(concat!("../assets/templates/", $id, "-1.png")),
            content_png: include_bytes!(concat!("../assets/templates/", $id, "-5.png")),
        }
    };
}

pub const TEMPLATES: [Template; 5] = [
    template!("classic", "Classic Green", "Madrid theme with a calm green palette and a full information bar.", "16:9 · light · green"),
    template!("midnight", "Midnight", "Dark slides with amber highlights, easy on the eyes in a dim room.", "16:9 · dark · amber"),
    template!("minimal", "Minimal White", "Clean white slides, dark text and a single blue accent.", "16:9 · light · blue"),
    template!("conference", "Conference", "Berlin theme in deep wine red, with section progress along the top.", "16:9 · light · red"),
    template!("sunrise", "Sunrise", "Warm coral and teal on a soft cream page, in the classic 4:3 shape.", "4:3 · warm · coral"),
];

/// The template with this id, or the first one.
pub fn by_id(id: &str) -> usize {
    TEMPLATES.iter().position(|t| t.id == id).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_different_templates_with_previews() {
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
        assert_eq!(themes.len(), 5, "every template looks different");
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
            let r = crate::latex::compile(&tex, crate::latex::Engine::PdfLatex);
            assert!(r.ok, "{} did not compile: {:?}", t.id, r.problems);
            assert!(r.problems.iter().all(|p| !p.is_error), "{}", t.id);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
