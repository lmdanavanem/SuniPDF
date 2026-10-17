//! Liens cliquables : lecture des zones de lien d'une page et filtre des adresses externes.
//!
//! La lecture passe par `page.annotations()` (indice d'annotation, comme `/Annots`) et jamais par
//! `page.links()` de pdfium-render 0.9.4 : `FPDFLink_Enumerate` y reçoit un indice d'annotation et
//! renvoie des doublons. Les actions autres que « aller à une page » et « ouvrir une adresse » ne
//! sont jamais exécutées.

use crate::annot_write::LinkTarget;
use crate::geometry::geometry_of;
use crate::{Document, EngineError, PageGeometry, PointRect, Rotation};
use pdfium_render::prelude::*;

/// An internal destination, with coordinates in the target page's display space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Destination {
    pub page: usize,
    pub view: DestinationView,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DestinationView {
    /// Missing axes and zoom retain the current viewport values.
    Xyz {
        x: Option<f32>,
        y: Option<f32>,
        zoom: Option<f32>,
    },
    FitPage,
    /// Fit the displayed page width, with an optional transformed PDF `/FitH` position.
    FitWidth {
        x: Option<f32>,
        y: Option<f32>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkInfo {
    /// Indice de l'annotation dans la page (celui de `annotations` et `annotation_details`).
    pub index: usize,
    /// Zone cliquable, espace d'affichage.
    pub rect: PointRect,
    pub target: LinkTarget,
    pub destination: Option<Destination>,
}

/// Longueur maximale d'une adresse acceptée.
const MAX_URI_LEN: usize = 2048;

/// Adresse externe qu'on accepte d'ouvrir (après confirmation de l'utilisateur) : `http`, `https`
/// ou `mailto`, ASCII imprimable sans espace. Refuse `file:`, `javascript:`, `ms-*:`, les chemins
/// Windows et UNC, `data:` et tout caractère de contrôle. `webbrowser::open` ne filtre rien :
/// cette liste d'autorisation est la seule barrière.
pub fn is_safe_external_uri(uri: &str) -> bool {
    let uri = uri.trim();
    if uri.is_empty() || uri.len() > MAX_URI_LEN || !uri.bytes().all(|b| (0x21..=0x7E).contains(&b))
    {
        return false;
    }
    let Some((scheme, rest)) = uri.split_once(':') else {
        return false;
    };
    match scheme.to_ascii_lowercase().as_str() {
        "http" | "https" => rest
            .strip_prefix("//")
            .and_then(|r| r.chars().next())
            .is_some_and(|c| !matches!(c, '/' | '\\' | '?' | '#')),
        "mailto" => !rest.is_empty(),
        _ => false,
    }
}

fn display_axes(geo: PageGeometry, x: Option<f32>, y: Option<f32>) -> (Option<f32>, Option<f32>) {
    let b = geo.bbox;
    match geo.rotation {
        Rotation::R0 => (x.map(|v| v - b.left), y.map(|v| v - b.bottom)),
        Rotation::R90 => (y.map(|v| v - b.bottom), x.map(|v| b.right - v)),
        Rotation::R180 => (x.map(|v| b.right - v), y.map(|v| b.top - v)),
        Rotation::R270 => (y.map(|v| b.top - v), x.map(|v| v - b.left)),
    }
}

fn destination_view(
    geo: PageGeometry,
    settings: PdfDestinationViewSettings,
) -> Option<DestinationView> {
    let finite = |v: Option<f32>| v.is_none_or(f32::is_finite);
    match settings {
        PdfDestinationViewSettings::SpecificCoordinatesAndZoom(x, y, zoom) => {
            let (x, y) = (x.map(|v| v.value), y.map(|v| v.value));
            if !finite(x) || !finite(y) || !finite(zoom) || zoom.is_some_and(|v| v < 0.0) {
                return None;
            }
            let (x, y) = display_axes(geo, x, y);
            (finite(x) && finite(y)).then_some(DestinationView::Xyz {
                x,
                y,
                zoom: zoom.filter(|v| *v > 0.0),
            })
        }
        PdfDestinationViewSettings::FitPageToWindow => Some(DestinationView::FitPage),
        PdfDestinationViewSettings::FitPageHorizontallyToWindow(y) => {
            let y = y.map(|v| v.value);
            // shortcut: PDFium collapses FitH null to zero; use page-only fallback until it exposes nullness.
            if !finite(y) || y == Some(0.0) {
                return None;
            }
            let (x, y) = display_axes(geo, None, y);
            (finite(x) && finite(y)).then_some(DestinationView::FitWidth { x, y })
        }
        // shortcut: other fit modes retain page navigation until their viewport behavior is supported.
        _ => None,
    }
}

impl Document<'_> {
    pub(crate) fn resolve_local_action(
        &self,
        action: &PdfAction<'_>,
    ) -> (Option<usize>, Option<Destination>) {
        // PDFium's generic destination getter also accepts remote actions; discriminate first.
        action
            .as_local_destination_action()
            .and_then(|a| a.destination().ok())
            .map(|d| self.resolve_destination(&d))
            .unwrap_or((None, None))
    }

    pub(crate) fn resolve_destination(
        &self,
        destination: &PdfDestination<'_>,
    ) -> (Option<usize>, Option<Destination>) {
        let Some(page) = destination
            .page_index()
            .ok()
            .and_then(|page| usize::try_from(page).ok())
            .filter(|page| *page < self.page_count())
        else {
            return (None, None);
        };
        let view = self
            .page_geometry(page)
            .ok()
            .zip(destination.view_settings().ok())
            .and_then(|(geo, settings)| destination_view(geo, settings));
        (Some(page), view.map(|view| Destination { page, view }))
    }

    /// Zones de lien de la page, dans l'ordre de `/Annots`.
    pub fn page_links(&self, page_index: usize) -> Result<Vec<LinkInfo>, EngineError> {
        let pdf = crate::security::load(&self.snapshot()?, self.password())?;
        let resolver = crate::link_destination::Resolver::new(&pdf);
        let raw_annotations = resolver.page_annotations(page_index)?;
        let page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let annotations = page.annotations();
        let mut out = Vec::new();
        for index in 0..annotations.len() {
            let Ok(a) = annotations.get(index) else {
                continue;
            };
            if a.as_link_annotation().is_none() {
                continue;
            }
            let Ok(r) = a.bounds() else { continue };
            let raw_annotation = raw_annotations.get(index).and_then(|entry| *entry);
            let (precise_target, destination) = raw_annotation
                .and_then(|annotation| {
                    resolver
                        .link(annotation, &|page| self.page_geometry(page))
                        .ok()
                })
                .unwrap_or((LinkTarget::Unsupported, None));
            // Navigation callers keep their legacy Page match; editors read the precise target in AnnotDetail.
            let target = destination
                .map(|destination| destination.page)
                .or_else(|| {
                    raw_annotation.and_then(|annotation| resolver.fallback_page(annotation))
                })
                .map_or(precise_target, LinkTarget::Page);
            out.push(LinkInfo {
                index,
                rect: geo.rect_to_display(PointRect {
                    left: r.left().value,
                    bottom: r.bottom().value,
                    right: r.right().value,
                    top: r.top().value,
                }),
                target,
                destination,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_axes_preserve_missing_values_and_reject_invalid_numbers() {
        for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
            let geo = PageGeometry {
                bbox: PointRect {
                    left: 100.0,
                    bottom: 50.0,
                    right: 500.0,
                    top: 650.0,
                },
                rotation,
            };
            let settings = |x: Option<f32>, y: Option<f32>, zoom: Option<f32>| {
                PdfDestinationViewSettings::SpecificCoordinatesAndZoom(
                    x.map(PdfPoints::new),
                    y.map(PdfPoints::new),
                    zoom,
                )
            };
            let expected = display_axes(geo, None, Some(400.0));
            assert_eq!(
                destination_view(geo, settings(None, Some(400.0), Some(0.0))),
                Some(DestinationView::Xyz {
                    x: expected.0,
                    y: expected.1,
                    zoom: None
                })
            );
            for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                assert_eq!(
                    destination_view(geo, settings(Some(invalid), None, None)),
                    None
                );
                assert_eq!(
                    destination_view(geo, settings(None, Some(invalid), None)),
                    None
                );
                assert_eq!(
                    destination_view(geo, settings(None, None, Some(invalid))),
                    None
                );
            }
            assert_eq!(
                destination_view(geo, settings(None, None, Some(-1.0))),
                None
            );
            assert_eq!(
                destination_view(
                    geo,
                    PdfDestinationViewSettings::FitPageHorizontallyToWindow(Some(PdfPoints::new(
                        0.0
                    )))
                ),
                None
            );
        }
    }

    #[test]
    fn accepts_only_web_and_mail_addresses() {
        for ok in [
            "https://a.b/c?d=1",
            "http://x",
            "mailto:a@b.c",
            "HTTPS://X.ORG",
            "  https://example.org  ",
        ] {
            assert!(is_safe_external_uri(ok), "{ok}");
        }
        for bad in [
            "file:///C:/x",
            "javascript:alert(1)",
            "ms-msdt:id",
            "C:\\Windows\\calc.exe",
            "\\\\srv\\share",
            "https:///x",
            "https://a b",
            "https://a\tb",
            "https://a\u{7f}b",
            "https://é.org",
            "data:text/html,x",
            "mailto:",
            "",
            "example.org",
        ] {
            assert!(!is_safe_external_uri(bad), "{bad:?}");
        }
        let long = format!("https://a.b/{}", "x".repeat(MAX_URI_LEN));
        assert!(!is_safe_external_uri(&long));
    }
}
