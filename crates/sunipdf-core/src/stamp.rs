//! Tampons, filigranes et numéros de page : texte apposé sur chaque page via les objets de page
//! (`edit.rs`), avec position, angle et opacité. `{n}` et `{total}` sont remplacés par page.

use crate::{Document, EngineError, TextStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StampPosition {
    #[default]
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl StampPosition {
    pub const ALL: [StampPosition; 7] = [
        StampPosition::Center,
        StampPosition::TopLeft,
        StampPosition::TopCenter,
        StampPosition::TopRight,
        StampPosition::BottomLeft,
        StampPosition::BottomCenter,
        StampPosition::BottomRight,
    ];

    /// Identifiant stable (CLI, préférences) : `center`, `top-left`, `bottom-center`…
    pub fn id(self) -> &'static str {
        match self {
            StampPosition::Center => "center",
            StampPosition::TopLeft => "top-left",
            StampPosition::TopCenter => "top-center",
            StampPosition::TopRight => "top-right",
            StampPosition::BottomLeft => "bottom-left",
            StampPosition::BottomCenter => "bottom-center",
            StampPosition::BottomRight => "bottom-right",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.id() == id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stamp {
    /// Texte ; `{n}` = numéro de page (1-based), `{total}` = nombre de pages.
    pub text: String,
    pub style: TextStyle,
    /// Angle en degrés, sens trigonométrique (45 = diagonale montante).
    pub angle: f32,
    pub position: StampPosition,
    /// Marge depuis les bords pour les positions non centrées.
    pub margin_pt: f32,
}

impl Default for Stamp {
    fn default() -> Self {
        Self {
            text: String::new(),
            style: TextStyle {
                size_pt: 48.0,
                bold: true,
                rgb: (0x80, 0x80, 0x80),
                opacity: 0.3,
                ..Default::default()
            },
            angle: 45.0,
            position: StampPosition::Center,
            margin_pt: 28.0,
        }
    }
}

impl Stamp {
    /// Numéros de page : petit texte opaque, horizontal, en bas au centre par défaut.
    pub fn page_numbers(template: &str, position: StampPosition) -> Self {
        Self {
            text: template.to_string(),
            style: TextStyle {
                size_pt: 10.0,
                ..Default::default()
            },
            angle: 0.0,
            position,
            margin_pt: 28.0,
        }
    }
}

impl Document<'_> {
    /// Appose le tampon sur les pages listées (toutes si `None`) ; renvoie le nombre de pages traitées.
    /// Chaque page reçoit un objet texte (annulable, déplaçable, supprimable comme tout objet).
    pub fn stamp_pages(
        &mut self,
        pages: Option<&[usize]>,
        stamp: &Stamp,
    ) -> Result<usize, EngineError> {
        if stamp.text.trim().is_empty() {
            return Err(EngineError::Other("empty stamp text".into()));
        }
        if !stamp.angle.is_finite() {
            return Err(EngineError::Other("invalid angle".into()));
        }
        let total = self.page_count();
        let targets: Vec<usize> = match pages {
            Some(list) => list.to_vec(),
            None => (0..total).collect(),
        };
        for &page in &targets {
            if page >= total {
                return Err(EngineError::PageOutOfRange(page));
            }
            let text = stamp
                .text
                .replace("{n}", &(page + 1).to_string())
                .replace("{total}", &total.to_string());
            let size = self.page_size(page)?;
            self.add_text(page, 0.0, 0.0, &text, &stamp.style)?;
            let index = self.page_objects(page)?.len() - 1;
            let b = self.object_info(page, index)?.bounds;
            let (w, h) = (b.right - b.left, b.top - b.bottom);
            let m = stamp.margin_pt;
            let (x, y) = match stamp.position {
                StampPosition::Center => ((size.width_pt - w) / 2.0, (size.height_pt - h) / 2.0),
                StampPosition::TopLeft => (m, size.height_pt - m - h),
                StampPosition::TopCenter => ((size.width_pt - w) / 2.0, size.height_pt - m - h),
                StampPosition::TopRight => (size.width_pt - m - w, size.height_pt - m - h),
                StampPosition::BottomLeft => (m, m),
                StampPosition::BottomCenter => ((size.width_pt - w) / 2.0, m),
                StampPosition::BottomRight => (size.width_pt - m - w, m),
            };
            self.move_object(page, index, x - b.left, y - b.bottom)?;
            if stamp.angle != 0.0 {
                self.rotate_object(page, index, -stamp.angle)?;
            }
        }
        Ok(targets.len())
    }
}
