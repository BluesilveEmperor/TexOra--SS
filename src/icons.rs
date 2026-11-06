//! One drawn icon set, so every control has the same line weight and size on every computer.
use crate::app::{INK, LEAF_DARK, LEAF_SOFT, MUTED};
use egui::{epaint::CubicBezierShape, pos2, vec2, Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

#[cfg(test)]
mod tests {
    #[test]
    fn app_icon_is_a_square_picture() {
        let (rgba, w, h) = super::icon_pixels().expect("icon reads");
        assert_eq!(w, h);
        assert!(w >= 512, "{w}");
        assert_eq!(rgba.len(), (w * h * 4) as usize);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Home,
    New,
    Folder,
    Pdf,
    Save,
    Play,
    Pause,
    Stop,
    Search,
    Bookmark,
    Chart,
    Gear,
    Grid,
    Code,
    Presenter,
    LargeSlide,
    Split,
    Windows,
    Black,
    White,
    Freeze,
    Pointer,
    Pen,
    Marker,
    Laser,
    Eraser,
    ZoomBox,
    Plus,
    Minus,
    Fit,
    Undo,
    Close,
    Check,
    Warn,
    Trash,
    Spark,
    Note,
    Wrap,
    Refresh,
    Copy,
    Help,
    First,
    Prev,
    Next,
    Last,
    List,
    Ask,
}

pub fn paint(p: &Painter, r: Rect, icon: Icon, c: Color32) {
    let u = r.width() / 24.0;
    let at = |x: f32, y: f32| r.min + vec2(x, y) * u;
    let s = Stroke::new((1.7 * u).max(1.2), c);
    let line = |pts: &[(f32, f32)]| {
        let v: Vec<Pos2> = pts.iter().map(|q| at(q.0, q.1)).collect();
        for w in v.windows(2) {
            p.line_segment([w[0], w[1]], s);
        }
    };
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32, round: f32| {
        p.rect_stroke(Rect::from_min_max(at(x0, y0), at(x1, y1)), CornerRadius::same((round * u) as u8), s, StrokeKind::Middle);
    };
    let fill = |x0: f32, y0: f32, x1: f32, y1: f32, round: f32| {
        p.rect_filled(Rect::from_min_max(at(x0, y0), at(x1, y1)), CornerRadius::same((round * u) as u8), c);
    };
    let tri = |pts: &[(f32, f32)]| {
        let v: Vec<Pos2> = pts.iter().map(|q| at(q.0, q.1)).collect();
        p.add(egui::Shape::convex_polygon(v, c, Stroke::NONE));
    };
    match icon {
        Icon::Home => {
            line(&[(3.5, 11.5), (12.0, 4.0), (20.5, 11.5)]);
            line(&[(6.0, 10.0), (6.0, 20.0), (18.0, 20.0), (18.0, 10.0)]);
        }
        Icon::New | Icon::Pdf => {
            line(&[(14.0, 3.0), (6.0, 3.0), (6.0, 21.0), (19.0, 21.0), (19.0, 8.0), (14.0, 3.0)]);
            line(&[(14.0, 3.0), (14.0, 8.0), (19.0, 8.0)]);
            if icon == Icon::New {
                line(&[(12.5, 11.0), (12.5, 17.0)]);
                line(&[(9.5, 14.0), (15.5, 14.0)]);
            }
        }
        Icon::Folder => {
            line(&[(3.0, 19.0), (3.0, 6.0), (9.5, 6.0), (11.5, 8.5), (21.0, 8.5), (21.0, 19.0), (3.0, 19.0)]);
        }
        Icon::Save => {
            line(&[(4.0, 4.0), (16.5, 4.0), (20.0, 7.5), (20.0, 20.0), (4.0, 20.0), (4.0, 4.0)]);
            line(&[(8.0, 4.0), (8.0, 9.0), (15.0, 9.0), (15.0, 4.0)]);
            rect(8.0, 13.5, 16.0, 20.0, 0.0);
        }
        Icon::Play => tri(&[(7.0, 4.5), (20.0, 12.0), (7.0, 19.5)]),
        Icon::Pause => {
            fill(6.5, 4.5, 10.5, 19.5, 1.0);
            fill(13.5, 4.5, 17.5, 19.5, 1.0);
        }
        Icon::Stop => fill(5.5, 5.5, 18.5, 18.5, 2.0),
        Icon::Search => {
            p.circle_stroke(at(10.5, 10.5), 6.5 * u, s);
            line(&[(15.5, 15.5), (20.5, 20.5)]);
        }
        Icon::Bookmark => line(&[(6.5, 3.5), (17.5, 3.5), (17.5, 20.5), (12.0, 16.0), (6.5, 20.5), (6.5, 3.5)]),
        Icon::Spark => {
            let star = |cx: f32, cy: f32, big: f32| {
                let k = big * 0.24;
                let pts = [(0.0, -big), (k, -k), (big, 0.0), (k, k), (0.0, big), (-k, k), (-big, 0.0), (-k, -k)];
                for i in 0..8 {
                    let (a, b) = (pts[i], pts[(i + 1) % 8]);
                    p.line_segment([at(cx + a.0, cy + a.1), at(cx + b.0, cy + b.1)], s);
                }
            };
            star(10.5, 13.0, 8.0);
            star(18.5, 5.5, 3.2);
        }
        Icon::Chart => {
            fill(5.0, 12.0, 8.5, 20.0, 1.0);
            fill(10.5, 6.0, 14.0, 20.0, 1.0);
            fill(16.0, 9.5, 19.5, 20.0, 1.0);
        }
        Icon::Gear => {
            p.circle_stroke(at(12.0, 12.0), 3.2 * u, s);
            p.circle_stroke(at(12.0, 12.0), 7.2 * u, s);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let (x, y) = (a.cos(), a.sin());
                p.line_segment([at(12.0 + x * 7.2, 12.0 + y * 7.2), at(12.0 + x * 10.0, 12.0 + y * 10.0)], Stroke::new(2.4 * u, c));
            }
        }
        Icon::Grid => {
            for (x, y) in [(4.0, 4.0), (13.5, 4.0), (4.0, 13.5), (13.5, 13.5)] {
                rect(x, y, x + 6.5, y + 6.5, 1.0);
            }
        }
        Icon::Code => {
            line(&[(8.5, 6.5), (3.5, 12.0), (8.5, 17.5)]);
            line(&[(15.5, 6.5), (20.5, 12.0), (15.5, 17.5)]);
            line(&[(13.5, 5.0), (10.5, 19.0)]);
        }
        Icon::Presenter => {
            rect(3.0, 4.5, 21.0, 19.5, 1.5);
            line(&[(15.0, 4.5), (15.0, 19.5)]);
            line(&[(15.0, 12.0), (21.0, 12.0)]);
            line(&[(6.0, 9.0), (12.0, 9.0)]);
            line(&[(6.0, 13.0), (12.0, 13.0)]);
        }
        Icon::LargeSlide => {
            rect(3.0, 5.0, 21.0, 19.0, 1.5);
            fill(6.0, 8.0, 14.0, 10.0, 0.5);
            line(&[(6.5, 14.0), (17.5, 14.0)]);
        }
        Icon::Split => {
            rect(3.0, 4.5, 21.0, 19.5, 1.5);
            line(&[(13.5, 4.5), (13.5, 19.5)]);
            line(&[(16.0, 9.0), (19.0, 9.0)]);
            line(&[(16.0, 12.5), (19.0, 12.5)]);
        }
        Icon::Windows => {
            line(&[(8.0, 7.5), (8.0, 3.5), (21.0, 3.5), (21.0, 14.5), (17.0, 14.5)]);
            line(&[(8.0, 6.8), (21.0, 6.8)]);
            rect(3.0, 9.5, 17.0, 20.5, 1.0);
            line(&[(3.0, 12.8), (17.0, 12.8)]);
        }
        Icon::Black => {
            rect(3.0, 5.0, 21.0, 19.0, 1.5);
            fill(5.5, 7.5, 18.5, 16.5, 0.5);
        }
        Icon::White => rect(3.0, 5.0, 21.0, 19.0, 1.5),
        Icon::Freeze => {
            for i in 0..3 {
                let a = i as f32 * std::f32::consts::FRAC_PI_3 + std::f32::consts::FRAC_PI_2;
                let (x, y) = (a.cos() * 9.0, a.sin() * 9.0);
                line(&[(12.0 - x, 12.0 - y), (12.0 + x, 12.0 + y)]);
                for side in [-1.0, 1.0] {
                    let (px, py) = (x * 0.68 * side, y * 0.68 * side);
                    let (qx, qy) = (-y * 0.26, x * 0.26);
                    line(&[(12.0 + px, 12.0 + py), (12.0 + px * 1.28 + qx, 12.0 + py * 1.28 + qy)]);
                    line(&[(12.0 + px, 12.0 + py), (12.0 + px * 1.28 - qx, 12.0 + py * 1.28 - qy)]);
                }
            }
        }
        Icon::Pointer => line(&[(6.0, 3.5), (6.0, 19.0), (10.5, 15.0), (13.5, 21.0), (16.0, 20.0), (13.0, 14.0), (18.5, 13.5), (6.0, 3.5)]),
        Icon::Pen => {
            line(&[(4.0, 20.0), (5.0, 15.5), (16.5, 4.0), (20.0, 7.5), (8.5, 19.0), (4.0, 20.0)]);
            line(&[(14.0, 6.5), (17.5, 10.0)]);
        }
        Icon::Marker => {
            line(&[(9.0, 13.0), (16.0, 4.0), (20.0, 8.0), (11.0, 15.0), (9.0, 13.0)]);
            line(&[(9.0, 13.0), (6.5, 17.5), (8.5, 19.5), (11.0, 15.0)]);
            line(&[(3.5, 21.0), (20.5, 21.0)]);
        }
        Icon::Laser => {
            p.circle_filled(at(12.0, 12.0), 3.0 * u, c);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let (x, y) = (a.cos(), a.sin());
                line(&[(12.0 + x * 6.0, 12.0 + y * 6.0), (12.0 + x * 9.5, 12.0 + y * 9.5)]);
            }
        }
        Icon::Eraser => {
            line(&[(8.0, 20.0), (3.5, 15.5), (14.0, 5.0), (20.5, 11.5), (12.0, 20.0), (8.0, 20.0)]);
            line(&[(8.5, 10.0), (15.0, 16.5)]);
        }
        Icon::ZoomBox => {
            p.circle_stroke(at(10.5, 10.5), 6.5 * u, s);
            line(&[(15.5, 15.5), (20.5, 20.5)]);
            line(&[(8.0, 10.5), (13.0, 10.5)]);
            line(&[(10.5, 8.0), (10.5, 13.0)]);
        }
        Icon::Plus => {
            line(&[(12.0, 5.0), (12.0, 19.0)]);
            line(&[(5.0, 12.0), (19.0, 12.0)]);
        }
        Icon::Minus => line(&[(5.0, 12.0), (19.0, 12.0)]),
        Icon::Fit => {
            for (x, y, dx, dy) in [(4.0, 4.0, 1.0, 1.0), (20.0, 4.0, -1.0, 1.0), (4.0, 20.0, 1.0, -1.0), (20.0, 20.0, -1.0, -1.0)] {
                line(&[(x + dx * 5.0, y), (x, y), (x, y + dy * 5.0)]);
            }
            rect(8.5, 8.5, 15.5, 15.5, 0.5);
        }
        Icon::Undo => {
            line(&[(8.5, 5.0), (4.0, 9.5), (8.5, 14.0)]);
            let curve = [at(4.0, 9.5), at(14.0, 8.0), at(21.0, 11.0), at(16.0, 19.0)];
            p.add(CubicBezierShape::from_points_stroke(curve, false, Color32::TRANSPARENT, s));
        }
        Icon::Close => {
            line(&[(6.0, 6.0), (18.0, 18.0)]);
            line(&[(18.0, 6.0), (6.0, 18.0)]);
        }
        Icon::Check => line(&[(4.5, 12.5), (9.5, 17.5), (19.5, 6.5)]),
        Icon::Warn => {
            line(&[(12.0, 4.0), (21.0, 19.5), (3.0, 19.5), (12.0, 4.0)]);
            line(&[(12.0, 9.5), (12.0, 14.5)]);
            p.circle_filled(at(12.0, 17.0), 1.0 * u, c);
        }
        Icon::Trash => {
            line(&[(4.0, 7.0), (20.0, 7.0)]);
            line(&[(9.0, 7.0), (9.5, 4.0), (14.5, 4.0), (15.0, 7.0)]);
            line(&[(6.0, 7.0), (7.0, 20.5), (17.0, 20.5), (18.0, 7.0)]);
        }
        Icon::First => {
            line(&[(6.0, 5.0), (6.0, 19.0)]);
            line(&[(18.0, 5.0), (10.0, 12.0), (18.0, 19.0)]);
        }
        Icon::Prev => line(&[(16.0, 4.5), (8.0, 12.0), (16.0, 19.5)]),
        Icon::Next => line(&[(8.0, 4.5), (16.0, 12.0), (8.0, 19.5)]),
        Icon::Last => {
            line(&[(18.0, 5.0), (18.0, 19.0)]);
            line(&[(6.0, 5.0), (14.0, 12.0), (6.0, 19.0)]);
        }
        Icon::List => {
            for y in [6.0, 12.0, 18.0] {
                p.circle_filled(at(4.5, y), 1.3 * u, c);
                line(&[(8.5, y), (20.5, y)]);
            }
        }
        Icon::Ask => {
            line(&[(3.5, 4.0), (20.5, 4.0), (20.5, 16.5), (11.0, 16.5), (6.5, 20.5), (6.5, 16.5), (3.5, 16.5), (3.5, 4.0)]);
            let curve = [at(9.3, 8.2), at(9.8, 5.6), at(14.8, 5.8), at(14.5, 8.6)];
            p.add(CubicBezierShape::from_points_stroke(curve, false, Color32::TRANSPARENT, s));
            line(&[(14.5, 8.6), (12.0, 10.5), (12.0, 11.8)]);
            p.circle_filled(at(12.0, 14.1), 0.9 * u, c);
        }
        Icon::Wrap => {
            line(&[(3.5, 6.0), (20.5, 6.0)]);
            line(&[(3.5, 18.0), (9.0, 18.0)]);
            let curve = [at(3.5, 12.0), at(22.0, 11.0), at(22.0, 18.0), at(13.5, 18.0)];
            p.add(CubicBezierShape::from_points_stroke(curve, false, Color32::TRANSPARENT, s));
            line(&[(15.5, 15.5), (13.0, 18.0), (15.5, 20.5)]);
        }
        Icon::Refresh => {
            let curve = [at(19.5, 9.0), at(16.0, 3.0), at(6.0, 3.5), at(4.5, 11.0)];
            p.add(CubicBezierShape::from_points_stroke(curve, false, Color32::TRANSPARENT, s));
            let curve = [at(4.5, 15.0), at(8.0, 21.0), at(18.0, 20.5), at(19.5, 13.0)];
            p.add(CubicBezierShape::from_points_stroke(curve, false, Color32::TRANSPARENT, s));
            line(&[(20.0, 4.0), (19.5, 9.0), (14.5, 8.5)]);
            line(&[(4.0, 20.0), (4.5, 15.0), (9.5, 15.5)]);
        }
        Icon::Copy => {
            rect(9.0, 9.0, 20.0, 20.0, 1.5);
            line(&[(15.0, 9.0), (15.0, 4.0), (4.0, 4.0), (4.0, 15.0), (9.0, 15.0)]);
        }
        Icon::Note => {
            line(&[(12.0, 4.0), (4.0, 4.0), (4.0, 20.0), (20.0, 20.0), (20.0, 12.0)]);
            line(&[(7.5, 9.0), (11.0, 9.0)]);
            line(&[(7.5, 16.0), (15.5, 16.0)]);
            line(&[(10.0, 13.5), (10.7, 11.0), (17.8, 3.9), (20.1, 6.2), (13.0, 13.2), (10.0, 13.5)]);
        }
        Icon::Help => {
            p.circle_stroke(at(12.0, 12.0), 9.0 * u, s);
            let curve = [at(8.8, 9.6), at(9.2, 5.6), at(15.6, 6.0), at(15.2, 9.8)];
            p.add(CubicBezierShape::from_points_stroke(curve, false, Color32::TRANSPARENT, s));
            let curve = [at(15.2, 9.8), at(14.8, 11.8), at(12.0, 11.6), at(12.0, 14.0)];
            p.add(CubicBezierShape::from_points_stroke(curve, false, Color32::TRANSPARENT, s));
            p.circle_filled(at(12.0, 17.0), 1.1 * u, c);
        }
    }
}

/// The app's icon. Replace assets/icon.png, then run scripts/make-icons.py, to change it everywhere.
pub const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

fn icon_pixels() -> Option<(Vec<u8>, u32, u32)> {
    let picture = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png).ok()?.to_rgba8();
    let (width, height) = picture.dimensions();
    Some((picture.into_raw(), width, height))
}

/// The picture for the window, Dock and taskbar.
pub fn window_icon() -> Option<egui::IconData> {
    icon_pixels().map(|(rgba, width, height)| egui::IconData { rgba, width, height })
}

/// Draws the app's icon inside the interface, from the same picture.
pub fn logo(ui: &Ui, r: Rect) {
    let key = egui::Id::NULL.with("texora-icon");
    let ctx = ui.ctx().clone();
    let stored: Option<Option<egui::TextureHandle>> = ctx.data(|d| d.get_temp(key));
    let texture = stored.unwrap_or_else(|| {
        let made = icon_pixels().map(|(rgba, w, h)| {
            let picture = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let smooth = egui::TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear));
            ctx.load_texture("texora-icon", picture, smooth)
        });
        ctx.data_mut(|d| d.insert_temp(key, made.clone()));
        made
    });
    if let Some(t) = texture {
        let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        ui.painter().image(t.id(), r, whole, Color32::WHITE);
    }
}

/// A filled bookmark ribbon. The notch means it is drawn as two halves.
pub fn ribbon(p: &Painter, r: Rect, c: Color32) {
    let m = r.center().x;
    let notch = r.bottom() - r.height() * 0.24;
    for side in [r.left(), r.right()] {
        let pts = vec![pos2(side, r.top()), pos2(m, r.top()), pos2(m, notch), pos2(side, r.bottom())];
        let pts = if side < m { pts } else { pts.into_iter().rev().collect() };
        p.add(egui::Shape::convex_polygon(pts, c, Stroke::NONE));
    }
}

/// Square icon button with the same hover, active and disabled states everywhere.
pub fn icon_button(ui: &mut Ui, icon: Icon, active: bool, tip: &str) -> Response {
    sized_button(ui, icon, active, tip, 32.0, 18.0)
}

pub fn sized_button(ui: &mut Ui, icon: Icon, active: bool, tip: &str, size: f32, glyph: f32) -> Response {
    let enabled = ui.is_enabled();
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), if enabled { Sense::click() } else { Sense::hover() });
    if ui.is_rect_visible(rect) {
        let hover = resp.hovered() && enabled;
        let bg = if active {
            LEAF_SOFT
        } else if hover {
            Color32::from_rgb(234, 233, 229)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 7.0, bg);
        let c = if !enabled {
            Color32::from_gray(190)
        } else if active {
            LEAF_DARK
        } else if hover {
            INK
        } else {
            Color32::from_rgb(70, 70, 68)
        };
        paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(glyph)), icon, c);
    }
    if tip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tip)
    }
}

/// Button with an icon and a label, for places with room to explain themselves.
pub fn labelled_button(ui: &mut Ui, icon: Icon, label: &str, primary: bool) -> Response {
    let font = egui::FontId::new(14.0, egui::FontFamily::Name("medium".into()));
    let text = if primary { Color32::WHITE } else { INK };
    let g = ui.painter().layout_no_wrap(label.to_string(), font, text);
    let size = vec2(g.size().x + 44.0, 36.0);
    let enabled = ui.is_enabled();
    let (rect, resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    let hover = resp.hovered() && enabled;
    let bg = match (primary, hover) {
        (true, false) => crate::app::LEAF,
        (true, true) => LEAF_DARK,
        (false, false) => Color32::from_rgb(236, 235, 231),
        (false, true) => LEAF_SOFT,
    };
    ui.painter().rect_filled(rect, 8.0, if enabled { bg } else { Color32::from_gray(232) });
    let c = if !enabled { MUTED } else { text };
    paint(ui.painter(), Rect::from_center_size(pos2(rect.left() + 21.0, rect.center().y), Vec2::splat(17.0)), icon, c);
    ui.painter().galley(pos2(rect.left() + 34.0, rect.center().y - g.size().y / 2.0), g, c);
    resp
}
