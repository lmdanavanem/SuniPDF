//! Flux d'apparence (`/AP`) des annotations écrites par SuniPDF : fonctions pures, sans PDFium ni
//! lopdf, testables seules. Tout est dessiné en coordonnées **locales d'affichage** (origine au coin
//! bas-gauche du rectangle de l'annotation tel qu'on le voit, Y vers le haut, en points) ; le
//! `/Matrix` du flux (calculé par `annot_write`) ramène ces axes dans l'espace utilisateur de la page,
//! si bien qu'un texte écrit sur une page pivotée reste droit à l'écran.

use crate::annot_write::{AnnotStyle, FreeTextStyle};

/// Largeurs Helvetica (1/1000 em) des caractères ASCII 0x20..=0x7E.
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278,
    278, // espace .. /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 0-9
    278, 278, 584, 584, 584, 556, 1015, // : .. @
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667,
    611, 722, 667, 944, 667, 667, 611, // A-Z
    278, 278, 278, 469, 556, 333, // [ .. `
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500,
    278, 556, 500, 722, 500, 500, 500, // a-z
    334, 260, 334, 584, // { .. ~
];

/// Largeur d'un caractère en 1/1000 em. Hors ASCII : 556 (une lettre accentuée vaut sa lettre de
/// base à ±10 % près ; le renvoi à la ligne garde une marge).
pub fn helvetica_width(c: char) -> u16 {
    match c as u32 {
        0x20..=0x7E => HELVETICA[c as usize - 0x20],
        _ => 556,
    }
}

pub fn text_width(s: &str, size_pt: f32) -> f32 {
    s.chars().map(|c| helvetica_width(c) as f32).sum::<f32>() * size_pt / 1000.0
}

/// Renvoi à la ligne à `max_width_pt` : sauts de ligne conservés, mots plus longs que la ligne
/// coupés. Une marge de 2 % absorbe l'imprécision de la table hors ASCII.
pub fn wrap_text(text: &str, max_width_pt: f32, size_pt: f32) -> Vec<String> {
    let max = (max_width_pt * 0.98).max(size_pt);
    let mut lines = Vec::new();
    for paragraph in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ') {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if text_width(&candidate, size_pt) <= max {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            // Mot trop long : coupé caractère par caractère.
            let mut piece = String::new();
            for c in word.chars() {
                piece.push(c);
                if text_width(&piece, size_pt) > max && piece.chars().count() > 1 {
                    piece.pop();
                    lines.push(std::mem::take(&mut piece));
                    piece.push(c);
                }
            }
            line = piece;
        }
        lines.push(line);
    }
    lines
}

/// Octet WinAnsi d'un caractère (`None` si absent de la page de codes).
fn winansi(c: char) -> Option<u8> {
    let u = c as u32;
    Some(match u {
        0x20..=0x7E | 0xA0..=0xFF => u as u8,
        0x20AC => 0x80,
        0x201A => 0x82,
        0x0192 => 0x83,
        0x201E => 0x84,
        0x2026 => 0x85,
        0x2020 => 0x86,
        0x2021 => 0x87,
        0x02C6 => 0x88,
        0x2030 => 0x89,
        0x0160 => 0x8A,
        0x2039 => 0x8B,
        0x0152 => 0x8C,
        0x017D => 0x8E,
        0x2018 => 0x91,
        0x2019 => 0x92,
        0x201C => 0x93,
        0x201D => 0x94,
        0x2022 => 0x95,
        0x2013 => 0x96,
        0x2014 => 0x97,
        0x02DC => 0x98,
        0x2122 => 0x99,
        0x0161 => 0x9A,
        0x203A => 0x9B,
        0x0153 => 0x9C,
        0x017E => 0x9E,
        0x0178 => 0x9F,
        _ => return None,
    })
}

/// Contenu d'une chaîne littérale PDF (entre parenthèses) : `\ ( )` échappés, octets hors ASCII
/// imprimable en `\ddd`, caractère absent de WinAnsi remplacé par `?`.
pub(crate) fn winansi_literal(line: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(line.len());
    for c in line.chars() {
        let b = winansi(c).unwrap_or(b'?');
        match b {
            b'\\' | b'(' | b')' => {
                out.push(b'\\');
                out.push(b);
            }
            0x20..=0x7E => out.push(b),
            _ => out.extend_from_slice(format!("\\{b:03o}").as_bytes()),
        }
    }
    out
}

/// Nombre PDF : 3 décimales au plus, sans zéros finaux ni exposant.
pub(crate) fn fmt_num(v: f32) -> String {
    let v = if v.is_finite() { v } else { 0.0 };
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.into()
    }
}

fn color_ops(rgb: (u8, u8, u8), stroke: bool) -> String {
    format!(
        "{} {} {} {}",
        fmt_num(rgb.0 as f32 / 255.0),
        fmt_num(rgb.1 as f32 / 255.0),
        fmt_num(rgb.2 as f32 / 255.0),
        if stroke { "RG" } else { "rg" }
    )
}

/// Apparence prête à être emballée dans un flux : taille locale, contenu, opacité, police utilisée.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ap {
    pub w: f32,
    pub h: f32,
    pub content: Vec<u8>,
    pub opacity: f32,
    pub uses_font: bool,
}

fn header(s: &AnnotStyle, with_fill: bool) -> String {
    let mut c = String::from("q /GS gs ");
    if with_fill {
        if let Some(f) = s.fill_rgb {
            c.push_str(&color_ops(f, false));
            c.push(' ');
        }
    }
    c.push_str(&color_ops(s.rgb, true));
    c.push_str(&format!(" {} w ", fmt_num(s.width_pt)));
    if !s.dash_pattern.is_empty() {
        let pattern = s
            .dash_pattern
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        c.push_str(&format!("[{pattern}] 0 d "));
    }
    c
}

fn paint_op(s: &AnnotStyle) -> &'static str {
    match (s.fill_rgb.is_some(), s.width_pt > 0.0) {
        (true, true) => "B",
        (true, false) => "f",
        (false, true) => "S",
        (false, false) => "n",
    }
}

fn ap(w: f32, h: f32, content: String, s: &AnnotStyle) -> Ap {
    Ap {
        w,
        h,
        content: content.into_bytes(),
        opacity: s.opacity.clamp(0.0, 1.0),
        uses_font: false,
    }
}

pub(crate) fn square_ap(w: f32, h: f32, s: &AnnotStyle) -> Ap {
    let lw = s.width_pt;
    let mut c = header(s, true);
    c.push_str(&format!(
        "{} {} {} {} re {} Q",
        fmt_num(lw / 2.0),
        fmt_num(lw / 2.0),
        fmt_num((w - lw).max(0.0)),
        fmt_num((h - lw).max(0.0)),
        paint_op(s)
    ));
    ap(w, h, c, s)
}

pub(crate) fn circle_ap(w: f32, h: f32, s: &AnnotStyle) -> Ap {
    const K: f32 = 0.552_284_8;
    let lw = s.width_pt;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (rx, ry) = (((w - lw) / 2.0).max(0.0), ((h - lw) / 2.0).max(0.0));
    let n = fmt_num;
    let mut c = header(s, true);
    c.push_str(&format!(
        "{} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c h {} Q",
        n(cx + rx), n(cy),
        n(cx + rx), n(cy + K * ry), n(cx + K * rx), n(cy + ry), n(cx), n(cy + ry),
        n(cx - K * rx), n(cy + ry), n(cx - rx), n(cy + K * ry), n(cx - rx), n(cy),
        n(cx - rx), n(cy - K * ry), n(cx - K * rx), n(cy - ry), n(cx), n(cy - ry),
        n(cx + K * rx), n(cy - ry), n(cx + rx), n(cy - K * ry), n(cx + rx), n(cy),
        paint_op(s)
    ));
    ap(w, h, c, s)
}

/// Milieu de deux points.
fn mid(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

/// Trait lissé : courbes passant par les milieux des segments, chaque sommet servant de point de
/// contrôle (courbe quadratique écrite en cubique) ; premier et dernier points exacts.
/// `n` points d'échantillonnage par courbe pour l'aperçu à l'écran.
pub fn smooth_polyline(points: &[(f32, f32)], n: usize) -> Vec<(f32, f32)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut out = vec![points[0]];
    let mut cur = points[0];
    for i in 1..points.len() - 1 {
        let (ctrl, end) = (points[i], mid(points[i], points[i + 1]));
        for s in 1..=n {
            let t = s as f32 / n as f32;
            let u = 1.0 - t;
            out.push((
                u * u * cur.0 + 2.0 * u * t * ctrl.0 + t * t * end.0,
                u * u * cur.1 + 2.0 * u * t * ctrl.1 + t * t * end.1,
            ));
        }
        cur = end;
    }
    out.push(points[points.len() - 1]);
    out
}

/// Traits en coordonnées locales, lissés ; un trait d'un seul point donne un point rond (`1 J`).
pub(crate) fn ink_ap(w: f32, h: f32, strokes: &[Vec<(f32, f32)>], s: &AnnotStyle) -> Ap {
    let mut c = header(s, false);
    c.push_str("1 J 1 j ");
    for stroke in strokes {
        let Some(first) = stroke.first() else {
            continue;
        };
        c.push_str(&format!("{} {} m ", fmt_num(first.0), fmt_num(first.1)));
        match stroke.len() {
            1 => c.push_str(&format!("{} {} l ", fmt_num(first.0), fmt_num(first.1))),
            2 => c.push_str(&format!(
                "{} {} l ",
                fmt_num(stroke[1].0),
                fmt_num(stroke[1].1)
            )),
            len => {
                let mut cur = *first;
                for i in 1..len - 1 {
                    let (ctrl, end) = (stroke[i], mid(stroke[i], stroke[i + 1]));
                    // Quadratique (cur, ctrl, end) → cubique : points à 2/3 vers le contrôle.
                    let c1 = (
                        cur.0 + 2.0 / 3.0 * (ctrl.0 - cur.0),
                        cur.1 + 2.0 / 3.0 * (ctrl.1 - cur.1),
                    );
                    let c2 = (
                        end.0 + 2.0 / 3.0 * (ctrl.0 - end.0),
                        end.1 + 2.0 / 3.0 * (ctrl.1 - end.1),
                    );
                    c.push_str(&format!(
                        "{} {} {} {} {} {} c ",
                        fmt_num(c1.0),
                        fmt_num(c1.1),
                        fmt_num(c2.0),
                        fmt_num(c2.1),
                        fmt_num(end.0),
                        fmt_num(end.1)
                    ));
                    cur = end;
                }
                let last = stroke[len - 1];
                c.push_str(&format!("{} {} l ", fmt_num(last.0), fmt_num(last.1)));
            }
        }
        c.push_str("S ");
    }
    c.push('Q');
    ap(w, h, c, s)
}

/// Longueur de la pointe d'une flèche.
pub(crate) fn arrow_head(width_pt: f32) -> f32 {
    (4.0 * width_pt).max(8.0)
}

pub(crate) fn line_ap(
    w: f32,
    h: f32,
    p1: (f32, f32),
    p2: (f32, f32),
    arrow: bool,
    s: &AnnotStyle,
) -> Ap {
    let mut c = header(s, false);
    c.push_str("1 J 1 j ");
    c.push_str(&format!(
        "{} {} m {} {} l S ",
        fmt_num(p1.0),
        fmt_num(p1.1),
        fmt_num(p2.0),
        fmt_num(p2.1)
    ));
    if arrow {
        if !s.dash_pattern.is_empty() {
            c.push_str("[] 0 d ");
        }
        let (dx, dy) = (p2.0 - p1.0, p2.1 - p1.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (ux, uy) = (dx / len, dy / len);
        let hl = arrow_head(s.width_pt).min(len);
        // Demi-angle 25° : tan = 0,466.
        let (nx, ny) = (-uy * hl * 0.466, ux * hl * 0.466);
        let (bx, by) = (p2.0 - ux * hl, p2.1 - uy * hl);
        c.push_str(&format!(
            "{} {} m {} {} l {} {} l S ",
            fmt_num(bx + nx),
            fmt_num(by + ny),
            fmt_num(p2.0),
            fmt_num(p2.1),
            fmt_num(bx - nx),
            fmt_num(by - ny)
        ));
    }
    c.push('Q');
    ap(w, h, c, s)
}

/// Marge intérieure du texte libre.
pub(crate) fn free_text_pad(border_pt: f32) -> f32 {
    2.0 + border_pt
}

/// Hauteur nécessaire à `lines` lignes de `size_pt`.
pub(crate) fn free_text_needed_height(lines: usize, size_pt: f32, border_pt: f32) -> f32 {
    lines.max(1) as f32 * 1.2 * size_pt + 2.0 * free_text_pad(border_pt)
}

pub(crate) fn free_text_ap(w: f32, h: f32, lines: &[String], s: &FreeTextStyle) -> Ap {
    let pad = free_text_pad(s.border_pt);
    let mut c = String::from("q /GS gs ");
    if let Some(f) = s.fill_rgb {
        c.push_str(&format!(
            "{} 0 0 {} {} re f ",
            color_ops(f, false),
            fmt_num(w),
            fmt_num(h)
        ));
    }
    if s.border_pt > 0.0 {
        let bw = s.border_pt;
        c.push_str(&format!(
            "{} {} w {} {} {} {} re S ",
            color_ops(s.rgb, true),
            fmt_num(bw),
            fmt_num(bw / 2.0),
            fmt_num(bw / 2.0),
            fmt_num((w - bw).max(0.0)),
            fmt_num((h - bw).max(0.0))
        ));
    }
    c.push_str(&format!(
        "q 0 0 {} {} re W n BT /Helv {} Tf {} {} {} Td ",
        fmt_num(w),
        fmt_num(h),
        fmt_num(s.size_pt),
        color_ops(s.rgb, false),
        fmt_num(pad),
        fmt_num(h - pad - 0.8 * s.size_pt),
    ));
    let mut bytes = c.into_bytes();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            bytes.extend_from_slice(format!("0 {} Td ", fmt_num(-1.2 * s.size_pt)).as_bytes());
        }
        bytes.push(b'(');
        bytes.extend(winansi_literal(line));
        bytes.extend_from_slice(b") Tj ");
    }
    bytes.extend_from_slice(b"ET Q Q");
    Ap {
        w,
        h,
        content: bytes,
        opacity: 1.0,
        uses_font: true,
    }
}

/// Simplification de polyligne (Ramer-Douglas-Peucker, pile explicite : pas de récursion sur un
/// tracé de milliers de points).
pub fn simplify_polyline(points: &[(f32, f32)], epsilon: f32) -> Vec<(f32, f32)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0usize, points.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let (mut far, mut far_d) = (a, 0.0f32);
        for i in a + 1..b {
            let d = dist_to_segment(points[i], points[a], points[b]);
            if d > far_d {
                (far, far_d) = (i, d);
            }
        }
        if far_d > epsilon {
            keep[far] = true;
            stack.push((a, far));
            stack.push((far, b));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

fn dist_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-12 {
        return ((p.0 - a.0).powi(2) + (p.1 - a.1).powi(2)).sqrt();
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    ((p.0 - (a.0 + t * dx)).powi(2) + (p.1 - (a.1 + t * dy)).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> AnnotStyle {
        AnnotStyle {
            rgb: (255, 0, 0),
            fill_rgb: None,
            width_pt: 2.0,
            opacity: 1.0,
            dash_pattern: Vec::new(),
        }
    }

    fn text(ap: &Ap) -> String {
        String::from_utf8(ap.content.clone()).unwrap()
    }

    #[test]
    fn helvetica_table_matches_the_measured_width() {
        let all: String = (0x20u8..=0x7E).map(|b| b as char).collect();
        let w = text_width(&all, 12.0);
        assert!((w - 600.7).abs() < 0.5, "{w}");
        assert_eq!(HELVETICA.len(), 95);
    }

    #[test]
    fn wraps_words_breaks_long_ones_and_keeps_newlines() {
        let lines = wrap_text("aaa bbb ccc", text_width("aaa bbb", 10.0) + 1.0, 10.0);
        assert_eq!(lines, ["aaa bbb", "ccc"]);
        let long = wrap_text(&"W".repeat(30), 60.0, 10.0);
        assert!(long.len() > 3 && long.iter().all(|l| text_width(l, 10.0) <= 60.0));
        assert_eq!(wrap_text("a\n\nb", 500.0, 10.0), ["a", "", "b"]);
        assert_eq!(wrap_text("", 500.0, 10.0), [""]);
    }

    #[test]
    fn winansi_escapes_and_replaces() {
        assert_eq!(winansi_literal("é("), b"\\351\\(");
        assert_eq!(winansi_literal("a\\b)"), b"a\\\\b\\)");
        assert_eq!(winansi_literal("€"), b"\\200");
        assert_eq!(winansi_literal("✓"), b"?");
    }

    #[test]
    fn numbers_are_short() {
        assert_eq!(fmt_num(1.0), "1");
        assert_eq!(fmt_num(0.123_456), "0.123");
        assert_eq!(fmt_num(-0.0001), "0");
        assert_eq!(fmt_num(f32::NAN), "0");
        assert_eq!(fmt_num(12.5), "12.5");
    }

    #[test]
    fn simplification_keeps_corners_and_drops_collinear_points() {
        let line: Vec<(f32, f32)> = (0..50).map(|i| (i as f32, 2.0 * i as f32)).collect();
        assert_eq!(simplify_polyline(&line, 0.4).len(), 2);
        let mut bent = line.clone();
        bent.extend((1..50).map(|i| (49.0 + i as f32, 98.0)));
        let s = simplify_polyline(&bent, 0.4);
        assert_eq!(s.len(), 3);
        assert_eq!(s[1], (49.0, 98.0));
        assert_eq!(simplify_polyline(&[(0.0, 0.0)], 1.0).len(), 1);
    }

    #[test]
    fn smoothing_keeps_the_ends_and_stays_inside_the_hull() {
        let pts = [(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)];
        let s = smooth_polyline(&pts, 4);
        assert_eq!((s[0], *s.last().unwrap()), (pts[0], pts[2]));
        assert_eq!(s.len(), 6);
        assert!(s
            .iter()
            .all(|p| p.1 <= 10.0 && p.1 >= 0.0 && p.0 >= 0.0 && p.0 <= 20.0));
        assert_eq!(smooth_polyline(&pts[..2], 4), pts[..2].to_vec());
    }

    #[test]
    fn shape_streams_use_the_expected_operators() {
        let sq = text(&square_ap(100.0, 50.0, &style()));
        assert!(sq.contains(" re S Q") && !sq.contains(" rg "), "{sq}");
        let mut filled = style();
        filled.fill_rgb = Some((0, 255, 0));
        assert!(text(&square_ap(100.0, 50.0, &filled)).contains(" re B Q"));
        filled.width_pt = 0.0;
        assert!(text(&square_ap(100.0, 50.0, &filled)).contains(" re f Q"));
        assert_eq!(
            text(&circle_ap(100.0, 50.0, &style()))
                .matches(" c ")
                .count(),
            4
        );
        let plain = text(&line_ap(
            10.0,
            10.0,
            (0.0, 0.0),
            (10.0, 10.0),
            false,
            &style(),
        ));
        let arrow = text(&line_ap(
            10.0,
            10.0,
            (0.0, 0.0),
            (10.0, 10.0),
            true,
            &style(),
        ));
        assert_eq!(plain.matches(" l ").count(), 1);
        assert_eq!(arrow.matches(" l ").count(), 3);
        let dot = text(&ink_ap(5.0, 5.0, &[vec![(2.0, 2.0)]], &style()));
        assert!(dot.contains("2 2 m 2 2 l S"), "{dot}");
        let two = text(&ink_ap(5.0, 5.0, &[vec![(0.0, 0.0), (4.0, 4.0)]], &style()));
        assert!(
            two.contains("0 0 m 4 4 l S") && !two.contains(" c "),
            "{two}"
        );
        // Trois points ou plus : courbes lissées, extrémités exactes.
        let curve = text(&ink_ap(
            20.0,
            20.0,
            &[vec![(0.0, 0.0), (10.0, 10.0), (20.0, 0.0), (30.0, 10.0)]],
            &style(),
        ));
        assert_eq!(curve.matches(" c ").count(), 2, "{curve}");
        assert!(
            curve.contains("0 0 m") && curve.contains("30 10 l S"),
            "{curve}"
        );
    }

    #[test]
    fn dashed_shapes_preserve_fill_and_solid_arrowheads() {
        let mut dashed = style();
        dashed.dash_pattern = vec![6.25, 3.0, 0.123_456];
        let sq = text(&square_ap(100.0, 50.0, &dashed));
        assert!(
            sq.contains("[6.25 3 0.123456] 0 d ") && sq.contains(" re S Q"),
            "{sq}"
        );
        dashed.fill_rgb = Some((0, 255, 0));
        assert!(text(&square_ap(100.0, 50.0, &dashed)).contains(" re B Q"));
        assert!(text(&circle_ap(100.0, 50.0, &dashed)).contains("[6.25 3 0.123456] 0 d "));
        let arrow = text(&line_ap(
            100.0,
            50.0,
            (0.0, 0.0),
            (100.0, 50.0),
            true,
            &dashed,
        ));
        assert!(
            arrow.contains(" l S [] 0 d ") && arrow.matches(" l ").count() == 3,
            "{arrow}"
        );
        assert!(!text(&square_ap(100.0, 50.0, &style())).contains(" d "));
    }

    #[test]
    fn free_text_stream_carries_each_line() {
        let st = FreeTextStyle {
            size_pt: 12.0,
            rgb: (0, 0, 255),
            fill_rgb: Some((255, 255, 0)),
            border_pt: 1.0,
        };
        let ap = free_text_ap(100.0, 40.0, &["un".into(), "é".into()], &st);
        let t = text(&ap);
        assert!(
            ap.uses_font && t.contains("(un) Tj") && t.contains("(\\351) Tj"),
            "{t}"
        );
        assert!(t.contains("0 -14.4 Td"));
        assert_eq!(free_text_needed_height(3, 10.0, 0.0), 3.0 * 12.0 + 4.0);
    }
}
