//! Sélection de texte et annotations (surlignage, soulignement, barré, note).
//! PDFium génère lui-même l'apparence de ces annotations au rendu.

use crate::geometry::geometry_of;
use crate::{Document, EngineError, PointRect};
use pdfium_render::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkupKind {
    Highlight,
    Underline,
    StrikeOut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationInfo {
    pub index: usize,
    /// Sous-type PDF en identifiant stable minuscule (`highlight`, `strikeout`, `text`, `link`…,
    /// `other` pour les sous-types rares) : jamais le nom de variante d'une bibliothèque.
    pub kind: &'static str,
    pub contents: Option<String>,
    pub rect: PointRect,
}

/// Identifiants de sous-types connus de l'interface (clés `comments.kind.*`).
pub const ANNOTATION_KINDS: [&str; 18] = [
    "text",
    "link",
    "freetext",
    "line",
    "square",
    "circle",
    "polygon",
    "polyline",
    "highlight",
    "underline",
    "squiggly",
    "strikeout",
    "stamp",
    "ink",
    "popup",
    "fileattachment",
    "widget",
    "other",
];

fn kind_id(t: PdfPageAnnotationType) -> &'static str {
    use PdfPageAnnotationType as T;
    match t {
        T::Text => "text",
        T::Link => "link",
        T::FreeText => "freetext",
        T::Line => "line",
        T::Square => "square",
        T::Circle => "circle",
        T::Polygon => "polygon",
        T::Polyline => "polyline",
        T::Highlight => "highlight",
        T::Underline => "underline",
        T::Squiggly => "squiggly",
        T::Strikeout => "strikeout",
        T::Stamp => "stamp",
        T::Ink => "ink",
        T::Popup => "popup",
        T::FileAttachment => "fileattachment",
        T::Widget => "widget",
        _ => "other",
    }
}

/// Texte sélectionné et ses rectangles de ligne (points PDF).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextSelection {
    pub text: String,
    pub rects: Vec<PointRect>,
}

/// A point in displayed page coordinates, after rotation and cropping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadingPoint {
    pub page: usize,
    pub x: f32,
    pub y: f32,
}

/// An index in PDFium's logical character sequence (not a UTF-8 byte offset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextPosition {
    pub page: usize,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadingSelection {
    pub anchor: TextPosition,
    pub focus: TextPosition,
    pub pages: Vec<(usize, TextSelection)>,
}

fn to_pdf_rect(r: PointRect) -> PdfRect {
    PdfRect::new(
        PdfPoints::new(r.bottom.min(r.top)),
        PdfPoints::new(r.left.min(r.right)),
        PdfPoints::new(r.bottom.max(r.top)),
        PdfPoints::new(r.left.max(r.right)),
    )
}

fn from_pdf_rect(r: PdfRect) -> PointRect {
    PointRect {
        left: r.left().value,
        bottom: r.bottom().value,
        right: r.right().value,
        top: r.top().value,
    }
}

/// Fusionne des rectangles de caractères en rectangles de ligne (recouvrement vertical > 50 %).
fn merge_lines(chars: &[PointRect]) -> Vec<PointRect> {
    let mut lines: Vec<PointRect> = Vec::new();
    for c in chars {
        if let Some(last) = lines.last_mut() {
            let overlap = c.top.min(last.top) - c.bottom.max(last.bottom);
            let height = (c.top - c.bottom).min(last.top - last.bottom).max(0.01);
            if overlap / height > 0.5 {
                last.left = last.left.min(c.left);
                last.right = last.right.max(c.right);
                last.top = last.top.max(c.top);
                last.bottom = last.bottom.min(c.bottom);
                continue;
            }
        }
        lines.push(*c);
    }
    lines
}

impl Document<'_> {
    /// Selects a logical character range, including both endpoints, or the word at `anchor`.
    /// PDFium's character order is retained; arbitrary multi-column order is not inferred.
    pub fn reading_selection(
        &self,
        anchor: ReadingPoint,
        focus: ReadingPoint,
        word: bool,
    ) -> Result<ReadingSelection, EngineError> {
        let locate = |point: ReadingPoint| -> Result<TextPosition, EngineError> {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(EngineError::Other("invalid reading coordinates".into()));
            }
            let page = self.page_handle(point.page)?;
            let geo = geometry_of(&page)?;
            let text = page.text()?;
            let mut closest = None;
            for ch in text.chars().iter() {
                if ch.unicode_char().is_none_or(char::is_control) {
                    continue;
                }
                let Ok(bounds) = ch.loose_bounds() else {
                    continue;
                };
                let bounds = from_pdf_rect(bounds);
                let crop = geo.bbox;
                if bounds.right <= crop.left
                    || bounds.left >= crop.right
                    || bounds.top <= crop.bottom
                    || bounds.bottom >= crop.top
                {
                    continue;
                }
                let r = geo.rect_to_display(bounds);
                if r.right <= r.left || r.top <= r.bottom {
                    continue;
                }
                let dx = point.x - point.x.clamp(r.left, r.right);
                let dy = point.y - point.y.clamp(r.bottom, r.top);
                let distance = dx * dx + dy * dy;
                if closest.is_none_or(|(_, d)| distance < d) {
                    closest = Some((ch.index(), distance));
                }
            }
            Ok(TextPosition {
                page: point.page,
                index: closest.map_or(0, |(index, _)| index),
            })
        };
        let anchor = locate(anchor)?;
        let focus = if word { anchor } else { locate(focus)? };
        let (mut start, mut end) = (anchor.min(focus), anchor.max(focus));
        if word {
            let page = self.page_handle(start.page)?;
            let text = page.text()?;
            let chars: Vec<char> = text
                .chars()
                .iter()
                .map(|c| c.unicode_char().unwrap_or('\0'))
                .collect();
            let part = |i: usize| {
                chars[i].is_alphanumeric()
                    || chars[i] == '_'
                    || (matches!(chars[i], '\'' | '’')
                        && i > 0
                        && i + 1 < chars.len()
                        && chars[i - 1].is_alphanumeric()
                        && chars[i + 1].is_alphanumeric())
            };
            if start.index < chars.len() && part(start.index) {
                while start.index > 0 && part(start.index - 1) {
                    start.index -= 1;
                }
                while end.index + 1 < chars.len() && part(end.index + 1) {
                    end.index += 1;
                }
            }
        }
        let mut pages = Vec::new();
        for page_index in start.page..=end.page {
            let page = self.page_handle(page_index)?;
            let geo = geometry_of(&page)?;
            let text = page.text()?;
            let chars = text.chars();
            let first = if page_index == start.page {
                start.index
            } else {
                0
            };
            let last = if page_index == end.page {
                end.index.saturating_add(1)
            } else {
                chars.len()
            };
            let mut selection = TextSelection::default();
            let mut boxes = Vec::new();
            for ch in chars.iter().skip(first).take(last.saturating_sub(first)) {
                let bounds = ch.loose_bounds().ok().map(from_pdf_rect);
                if ch.unicode_char().is_some_and(|c| !c.is_control())
                    && bounds.is_some_and(|r| {
                        r.right <= geo.bbox.left
                            || r.left >= geo.bbox.right
                            || r.top <= geo.bbox.bottom
                            || r.bottom >= geo.bbox.top
                    })
                {
                    continue;
                }
                if let Some(c) = ch.unicode_char().filter(|c| *c != '\0') {
                    selection.text.push(c);
                }
                if let Some(r) = bounds {
                    if r.right > r.left && r.top > r.bottom {
                        boxes.push(r);
                    }
                }
            }
            selection.text = selection.text.replace("\r\n", "\n").replace('\r', "\n");
            selection.rects = merge_lines(&boxes)
                .into_iter()
                .map(|r| geo.rect_to_display(r))
                .collect();
            pages.push((page_index, selection));
        }
        Ok(ReadingSelection {
            anchor,
            focus,
            pages,
        })
    }

    /// Texte contenu dans un rectangle d'affichage, avec ses rectangles de ligne (affichage).
    pub fn text_in_rect(
        &self,
        page_index: usize,
        rect: PointRect,
    ) -> Result<TextSelection, EngineError> {
        let page = self.page_handle(page_index)?;
        // Les boîtes de caractères de PDFium sont en espace utilisateur.
        let geo = geometry_of(&page)?;
        let text = page.text()?;
        // ponytail: parcours de tous les caractères de la page (précis, O(n)) plutôt que
        // `chars_inside_rect`, imprécis sur les bords.
        let (l, r) = (rect.left.min(rect.right), rect.left.max(rect.right));
        let (b, t) = (rect.bottom.min(rect.top), rect.bottom.max(rect.top));
        let all = text.chars();
        let display = |bb: PdfRect| geo.rect_to_display(from_pdf_rect(bb));
        let chars = all.iter().filter(|ch| {
            ch.loose_bounds().is_ok_and(|bb| {
                let bb = display(bb);
                let cx = (bb.left + bb.right) / 2.0;
                let cy = (bb.bottom + bb.top) / 2.0;
                cx >= l && cx <= r && cy >= b && cy <= t
            })
        });
        let mut out = String::new();
        let mut boxes = Vec::new();
        let mut last_bottom: Option<f32> = None;
        for ch in chars {
            let Ok(b) = ch.loose_bounds() else { continue };
            // Lignes reconstituées en espace utilisateur (sens d'écriture du texte), puis converties.
            let b = from_pdf_rect(b);
            if let Some(lb) = last_bottom {
                // Changement de ligne : saut de ligne dans le texte copié.
                if (lb - b.bottom).abs() > (b.top - b.bottom) * 0.5 && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            last_bottom = Some(b.bottom);
            if let Some(c) = ch.unicode_char() {
                if c != '\r' {
                    out.push(c);
                }
            }
            boxes.push(b);
        }
        Ok(TextSelection {
            text: out.trim().to_string(),
            rects: merge_lines(&boxes)
                .into_iter()
                .map(|r| geo.rect_to_display(r))
                .collect(),
        })
    }

    /// Ajoute une annotation de texte (surlignage, soulignement, barré) couvrant `rects`.
    pub fn add_markup(
        &mut self,
        page_index: usize,
        kind: MarkupKind,
        rects: &[PointRect],
        rgb: (u8, u8, u8),
    ) -> Result<(), EngineError> {
        if rects.is_empty() {
            return Err(EngineError::Other("empty selection".into()));
        }
        let mut page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let rects: Vec<PointRect> = rects.iter().map(|r| geo.rect_to_user(*r)).collect();
        let union = rects.iter().fold(rects[0], |u, r| PointRect {
            left: u.left.min(r.left),
            bottom: u.bottom.min(r.bottom),
            right: u.right.max(r.right),
            top: u.top.max(r.top),
        });
        let color = PdfColor::new(rgb.0, rgb.1, rgb.2, 255);
        // Chaque type expose son propre `attachment_points_mut` : macro plutôt que trait objet.
        macro_rules! markup {
            ($annotation:expr) => {{
                let mut a = $annotation;
                a.set_bounds(to_pdf_rect(union))?;
                a.set_stroke_color(color)?;
                for r in &rects {
                    a.attachment_points_mut().create_attachment_point_at_end(
                        PdfQuadPoints::new_from_values(
                            r.left, r.top, r.right, r.top, r.left, r.bottom, r.right, r.bottom,
                        ),
                    )?;
                }
            }};
        }
        let annotations = page.annotations_mut();
        match kind {
            MarkupKind::Highlight => markup!(annotations.create_highlight_annotation()?),
            MarkupKind::Underline => markup!(annotations.create_underline_annotation()?),
            MarkupKind::StrikeOut => markup!(annotations.create_strikeout_annotation()?),
        }
        Ok(())
    }

    /// Ajoute une note (annotation `Text`) dont le coin haut-gauche est en `(x, y)` (affichage).
    pub fn add_note(
        &mut self,
        page_index: usize,
        x: f32,
        y: f32,
        text: &str,
    ) -> Result<(), EngineError> {
        let mut page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let mut note = page.annotations_mut().create_text_annotation(text)?;
        note.set_bounds(to_pdf_rect(geo.rect_to_user(PointRect {
            left: x,
            bottom: y - 20.0,
            right: x + 20.0,
            top: y,
        })))?;
        note.set_stroke_color(PdfColor::new(0xF5, 0x9E, 0x0B, 255))?;
        Ok(())
    }

    /// Annotations de la page, une ligne par entrée de `/Annots` (l'indice est celui de PDFium) :
    /// une entrée illisible donne `kind = "other"` au lieu d'interrompre la liste.
    pub fn annotations(&self, page_index: usize) -> Result<Vec<AnnotationInfo>, EngineError> {
        let page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let annotations = page.annotations();
        let mut out = Vec::new();
        for index in 0..annotations.len() {
            let Ok(a) = annotations.get(index) else {
                out.push(AnnotationInfo {
                    index,
                    kind: "other",
                    contents: None,
                    rect: PointRect {
                        left: 0.0,
                        bottom: 0.0,
                        right: 0.0,
                        top: 0.0,
                    },
                });
                continue;
            };
            let rect = a
                .bounds()
                .map(|r| geo.rect_to_display(from_pdf_rect(r)))
                .unwrap_or(PointRect {
                    left: 0.0,
                    bottom: 0.0,
                    right: 0.0,
                    top: 0.0,
                });
            out.push(AnnotationInfo {
                index,
                kind: kind_id(a.annotation_type()),
                contents: a.contents().filter(|c| !c.is_empty()),
                rect,
            });
        }
        Ok(out)
    }

    pub fn delete_annotation(
        &mut self,
        page_index: usize,
        index: usize,
    ) -> Result<(), EngineError> {
        let mut page = self.page_handle(page_index)?;
        {
            let annotations = page.annotations_mut();
            let annotation = annotations
                .get(index as PdfPageAnnotationIndex)
                .map_err(|_| EngineError::Other(format!("annotation {index} not found")))?;
            annotations.delete_annotation(annotation)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_chars_into_lines() {
        let r = |l, b, rr, t| PointRect {
            left: l,
            bottom: b,
            right: rr,
            top: t,
        };
        let lines = merge_lines(&[
            r(0.0, 0.0, 5.0, 10.0),
            r(5.0, 0.5, 10.0, 10.0),
            r(0.0, 20.0, 5.0, 30.0),
        ]);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].right, 10.0);
        assert_eq!(lines[1].top, 30.0);
    }

    #[test]
    fn annotation_kinds_are_stable_ids_known_to_the_ui() {
        assert_eq!(kind_id(PdfPageAnnotationType::Strikeout), "strikeout");
        assert_eq!(kind_id(PdfPageAnnotationType::Text), "text");
        assert_eq!(kind_id(PdfPageAnnotationType::RichMedia), "other");
        for t in [
            PdfPageAnnotationType::Highlight,
            PdfPageAnnotationType::Widget,
            PdfPageAnnotationType::Ink,
            PdfPageAnnotationType::Unknown,
        ] {
            assert!(ANNOTATION_KINDS.contains(&kind_id(t)), "{t:?}");
        }
    }
}
