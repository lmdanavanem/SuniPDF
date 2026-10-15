//! Liens cliquables : lecture des zones de lien d'une page et filtre des adresses externes.
//!
//! La lecture passe par `page.annotations()` (indice d'annotation, comme `/Annots`) et jamais par
//! `page.links()` de pdfium-render 0.9.4 : `FPDFLink_Enumerate` y reçoit un indice d'annotation et
//! renvoie des doublons. Les actions autres que « aller à une page » et « ouvrir une adresse » ne
//! sont jamais exécutées.

use crate::annot_write::LinkTarget;
use crate::geometry::geometry_of;
use crate::{Document, EngineError, PointRect};
use pdfium_render::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct LinkInfo {
    /// Indice de l'annotation dans la page (celui de `annotations` et `annotation_details`).
    pub index: usize,
    /// Zone cliquable, espace d'affichage.
    pub rect: PointRect,
    pub target: LinkTarget,
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

fn target_of(link: &PdfLink<'_>) -> LinkTarget {
    if let Some(p) = link.destination().and_then(|d| d.page_index().ok()) {
        return LinkTarget::Page(p as usize);
    }
    let Some(action) = link.action() else {
        return LinkTarget::Unsupported;
    };
    if let Some(p) = action
        .as_local_destination_action()
        .and_then(|a| a.destination().ok())
        .and_then(|d| d.page_index().ok())
    {
        return LinkTarget::Page(p as usize);
    }
    if let Some(uri) = action.as_uri_action().and_then(|a| a.uri().ok()) {
        return LinkTarget::Uri(uri);
    }
    LinkTarget::Unsupported
}

impl Document<'_> {
    /// Zones de lien de la page, dans l'ordre de `/Annots`.
    pub fn page_links(&self, page_index: usize) -> Result<Vec<LinkInfo>, EngineError> {
        let page = self.page_handle(page_index)?;
        let geo = geometry_of(&page)?;
        let count = self.page_count();
        let annotations = page.annotations();
        let mut out = Vec::new();
        for index in 0..annotations.len() {
            let Ok(a) = annotations.get(index) else {
                continue;
            };
            let Some(l) = a.as_link_annotation() else {
                continue;
            };
            let Ok(r) = a.bounds() else { continue };
            let target = match l.link() {
                Ok(link) => match target_of(&link) {
                    LinkTarget::Page(p) if p >= count => LinkTarget::Unsupported,
                    t => t,
                },
                Err(_) => LinkTarget::Unsupported,
            };
            out.push(LinkInfo {
                index,
                rect: geo.rect_to_display(PointRect {
                    left: r.left().value,
                    bottom: r.bottom().value,
                    right: r.right().value,
                    top: r.top().value,
                }),
                target,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
