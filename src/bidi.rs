//! Right-to-left writing (Arabic, Hebrew, Persian, Urdu) in the notes view.
//!
//! The text engine lays out a line from left to right in the order it was typed. It shapes each word
//! of Arabic or Hebrew correctly, but puts the words themselves in the wrong order and a sentence's
//! punctuation on the wrong side. Here a line is first wrapped, then put in the order it is seen,
//! following the Unicode bidirectional algorithm, before it is drawn.
use unicode_bidi::{bidi_class, BidiClass, BidiInfo, Level};

/// True when the text has right-to-left letters anywhere.
pub fn has_rtl(s: &str) -> bool {
    s.chars().any(|c| matches!(bidi_class(c), BidiClass::R | BidiClass::AL | BidiClass::RLE | BidiClass::RLO | BidiClass::RLI))
}

/// True when a paragraph reads right to left: its first letter is Arabic, Hebrew or similar.
pub fn starts_rtl(s: &str) -> bool {
    for c in s.chars() {
        match bidi_class(c) {
            BidiClass::R | BidiClass::AL => return true,
            BidiClass::L => return false,
            _ => {}
        }
    }
    false
}

/// A piece of a line, in the order it is seen from left to right, with the style it is drawn in.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub text: String,
    pub style: usize,
}

/// Breaks a paragraph into lines no wider than `max`. `chars` holds each character with its style,
/// and `width` measures a piece of text in a style. Words stay whole; spaces end a line.
pub fn wrap(chars: &[(char, usize)], max: f32, mut width: impl FnMut(&str, usize) -> f32) -> Vec<Vec<(char, usize)>> {
    // Words with the spaces after them, kept in typing order.
    let mut words: Vec<Vec<(char, usize)>> = vec![];
    for &(c, s) in chars {
        let new_word = match words.last() {
            None => true,
            Some(w) => w.last().map(|x| x.0.is_whitespace()).unwrap_or(false) && !c.is_whitespace(),
        };
        if new_word {
            words.push(vec![]);
        }
        words.last_mut().unwrap().push((c, s));
    }
    let measure = |w: &[(char, usize)], width: &mut dyn FnMut(&str, usize) -> f32| -> (f32, f32) {
        let text: String = w.iter().map(|x| x.0).collect();
        let full = width(&text, w[0].1);
        let bare = width(text.trim_end(), w[0].1);
        (full, bare)
    };
    let mut lines: Vec<Vec<(char, usize)>> = vec![];
    let mut line: Vec<(char, usize)> = vec![];
    let mut used = 0.0;
    for w in words {
        let (full, bare) = measure(&w, &mut width);
        if !line.is_empty() && used + bare > max {
            lines.push(std::mem::take(&mut line));
            used = 0.0;
        }
        used += full;
        line.extend(w);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Brackets and similar signs face the other way inside right-to-left text.
fn mirror(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        _ => c,
    }
}

/// A piece of a line in the order it is seen: the characters `range` of the line (in typing order),
/// drawn as one piece, read right to left when `rtl`.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub range: std::ops::Range<usize>,
    pub rtl: bool,
}

/// The pieces of one line, from left to right as they are seen. `rtl` is the paragraph's direction.
pub fn visual_runs(line: &[(char, usize)], rtl: bool) -> Vec<Run> {
    if line.is_empty() {
        return vec![];
    }
    let text: String = line.iter().map(|x| x.0).collect();
    let info = BidiInfo::new(&text, Some(if rtl { Level::rtl() } else { Level::ltr() }));
    let Some(para) = info.paragraphs.first() else {
        return vec![Run { range: 0..line.len(), rtl }];
    };
    // One level per character, with trailing spaces set back to the paragraph's level (rule L1).
    let per_byte = info.reordered_levels(para, 0..text.len());
    let levels: Vec<Level> = text.char_indices().map(|(i, _)| per_byte[i]).collect();

    // Pieces the text engine can draw as they are: a run of left-to-right text, or one right-to-left
    // word (which it shapes correctly by itself), or a single sign between right-to-left words.
    let letter = |c: char| matches!(bidi_class(c), BidiClass::R | BidiClass::AL | BidiClass::NSM);
    let mut runs: Vec<(Run, Level)> = vec![];
    for (i, &(c, style)) in line.iter().enumerate() {
        let level = levels[i];
        let joins = match runs.last() {
            Some((r, l)) if *l == level && line[r.range.start].1 == style => {
                level.is_ltr() || (letter(c) && letter(line[i - 1].0))
            }
            _ => false,
        };
        if joins {
            runs.last_mut().unwrap().0.range.end = i + 1;
        } else {
            runs.push((Run { range: i..i + 1, rtl: level.is_rtl() }, level));
        }
    }

    // Rule L2: from the highest level down to the lowest odd one, reverse every run at or above it.
    let highest = runs.iter().map(|p| p.1.number()).max().unwrap_or(0);
    let lowest_odd = runs.iter().map(|p| p.1.number()).filter(|n| n % 2 == 1).min();
    if let Some(lowest_odd) = lowest_odd {
        for k in (lowest_odd..=highest).rev() {
            let mut i = 0;
            while i < runs.len() {
                if runs[i].1.number() >= k {
                    let start = i;
                    while i < runs.len() && runs[i].1.number() >= k {
                        i += 1;
                    }
                    runs[start..i].reverse();
                } else {
                    i += 1;
                }
            }
        }
    }
    runs.into_iter().map(|p| p.0).collect()
}

/// The text of a run as it is drawn: brackets inside right-to-left text face the other way.
pub fn run_text(chars: impl Iterator<Item = char>, rtl: bool) -> String {
    chars.map(|c| if rtl { mirror(c) } else { c }).collect()
}

/// Puts one line in the order it is seen. `rtl` is the paragraph's direction.
pub fn visual(line: &[(char, usize)], rtl: bool) -> Vec<Piece> {
    visual_runs(line, rtl)
        .into_iter()
        .map(|r| Piece { text: run_text(line[r.range.clone()].iter().map(|x| x.0), r.rtl), style: line[r.range.start].1 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(s: &str) -> Vec<(char, usize)> {
        s.chars().map(|c| (c, 0)).collect()
    }

    fn seen(s: &str, rtl: bool) -> String {
        visual(&plain(s), rtl).into_iter().map(|p| p.text).collect::<Vec<_>>().join("|")
    }

    #[test]
    fn directions_are_found() {
        assert!(has_rtl("Hello مرحبا") && !has_rtl("Hello, привет, 你好"));
        assert!(starts_rtl("  مرحبا Hello") && starts_rtl("12: שלום") && !starts_rtl("Hello مرحبا"));
    }

    #[test]
    fn arabic_words_run_right_to_left() {
        // Typed: "the first" "word" — seen from the left: the last word first, the colon on the left.
        assert_eq!(seen("العربية: مرحبا بكم", true), "بكم| |مرحبا| |:|العربية");
    }

    #[test]
    fn hebrew_inside_english_keeps_the_english_order() {
        assert_eq!(seen("Say שלום עולם now", false), "Say |עולם| |שלום| now");
    }

    #[test]
    fn numbers_and_latin_inside_arabic_read_left_to_right() {
        assert_eq!(seen("الشريحة 12 فيها PDF", true), "PDF| |فيها| |12| |الشريحة");
    }

    #[test]
    fn brackets_face_the_right_way() {
        assert_eq!(seen("مرحبا (عالم)", true), "(|عالم|)| |مرحبا");
    }

    #[test]
    fn lines_wrap_before_they_are_reordered() {
        let w = |s: &str, _| s.chars().count() as f32;
        let lines = wrap(&plain("واحد اثنان ثلاثة"), 11.0, w);
        let text: Vec<String> = lines.iter().map(|l| l.iter().map(|x| x.0).collect()).collect();
        assert_eq!(text, ["واحد اثنان ", "ثلاثة"]);
    }

    #[test]
    fn styles_stay_with_their_words() {
        let mut chars = plain("مرحبا ");
        chars.extend("بكم".chars().map(|c| (c, 1)));
        let v = visual(&chars, true);
        assert_eq!(v[0], Piece { text: "بكم".into(), style: 1 });
        assert_eq!(v.last().unwrap().text, "مرحبا");
    }
}
