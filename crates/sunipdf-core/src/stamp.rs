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
        if stamp.text.contains('\n') || stamp.text.contains('\r') {
            return Err(EngineError::Other(
                "stamp text must be a single line".into(),
            ));
        }
        if stamp.text.trim().is_empty() {
            return Err(EngineError::Other("empty stamp text".into()));
        }
        if !stamp.angle.is_finite() {
            return Err(EngineError::Other("invalid angle".into()));
        }
        if !stamp.margin_pt.is_finite() || stamp.margin_pt < 0.0 {
            return Err(EngineError::Other("invalid stamp margin".into()));
        }
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        let total = self.page_count();
        let mut targets: Vec<usize> = match pages {
            Some(list) => list.to_vec(),
            None => (0..total).collect(),
        };
        targets.sort_unstable();
        targets.dedup();
        if targets.is_empty() || targets.len() > 1000 {
            return Err(EngineError::Other(
                "select between 1 and 1000 stamp pages".into(),
            ));
        }
        for &page in &targets {
            self.page_size(page)?;
        }
        let mut scratch = Document {
            inner: self
                .pdfium
                .load_pdf_from_byte_vec(self.snapshot()?, self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
            form_render_need_appearances: Default::default(),
        };
        for &page in &targets {
            if page >= total {
                return Err(EngineError::PageOutOfRange(page));
            }
            let text = stamp
                .text
                .replace("{n}", &(page + 1).to_string())
                .replace("{total}", &total.to_string());
            let size = scratch.page_size(page)?;
            scratch.add_text(page, 0.0, 0.0, &text, &stamp.style)?;
            let index = scratch.page_objects(page)?.len() - 1;
            // Pivoter d'abord : la position se calcule sur la boîte englobante du texte pivoté,
            // sinon un tampon à 45° dans un coin déborde de la page.
            if stamp.angle != 0.0 {
                scratch.rotate_object(page, index, -stamp.angle.rem_euclid(360.0))?;
            }
            let b = scratch.object_info(page, index)?.bounds;
            let (w, h) = (b.right - b.left, b.top - b.bottom);
            let m = stamp.margin_pt;
            if !w.is_finite()
                || !h.is_finite()
                || w + 2.0 * m > size.width_pt + 0.02
                || h + 2.0 * m > size.height_pt + 0.02
            {
                return Err(EngineError::Other(
                    "stamp does not fit within the page margins".into(),
                ));
            }
            let (x, y) = match stamp.position {
                StampPosition::Center => ((size.width_pt - w) / 2.0, (size.height_pt - h) / 2.0),
                StampPosition::TopLeft => (m, size.height_pt - m - h),
                StampPosition::TopCenter => ((size.width_pt - w) / 2.0, size.height_pt - m - h),
                StampPosition::TopRight => (size.width_pt - m - w, size.height_pt - m - h),
                StampPosition::BottomLeft => (m, m),
                StampPosition::BottomCenter => ((size.width_pt - w) / 2.0, m),
                StampPosition::BottomRight => (size.width_pt - m - w, m),
            };
            scratch.move_object(page, index, x - b.left, y - b.bottom)?;
        }
        self.form_render_need_appearances.set(None);
        self.inner = scratch.inner;
        self.font_cache.clear();
        Ok(targets.len())
    }
}
