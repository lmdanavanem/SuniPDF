//! Hub « Tous les outils » : regroupe les commandes du registre par tâche (liste),
//! bascule d'outil (fonctions pures, testables sans fenêtre) et barre contextuelle de l'outil actif.
//! Les familles du panneau sont dans `tool_hub.rs`, les cartes d'accueil dans `shell.rs`.

use crate::annotate::{AnnotateState, Tool};
use crate::app::SuniApp;
use crate::editor::EditState;
use crate::icons::{self, glyph};
use crate::organize::OrganizeState;
use crate::theme;
use egui::Color32;
use sunipdf_core::Command;
use sunipdf_i18n::tr;

/// Ordre des outils tel qu'affiché ; `TABLE[w as usize].ws == w` (testé).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Workspace {
    Export,
    FillSign,
    Edit,
    Create,
    Combine,
    Organize,
    Quantity,
    Measure,
    RevisionOverlay,
    Library,
    Comment,
    ScanOcr,
    Protect,
    Redact,
    Compress,
    PrepareForm,
    Stamp,
    Certificate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    /// Un clic lance la commande (boîte de dialogue, sélecteur de fichiers) : pas de barre.
    Direct(Command),
    /// Un clic entre dans l'outil (commande `hub.*` du registre) ; ses commandes remplissent la barre.
    Mode(Command),
}

pub struct WsInfo {
    pub ws: Workspace,
    pub name_key: &'static str,
    pub desc_key: &'static str,
    /// (glyphe Segoe Fluent, repli texte), comme `shell::command_icon`.
    pub icon: (&'static str, &'static str),
    pub color: Color32,
    /// `None` = fonction non implémentée : l'outil n'apparaît nulle part (pas de bouton factice).
    pub entry: Option<Entry>,
    /// Commands grouped by separators. Measure also has a shortcut in Comment.
    pub groups: &'static [&'static [Command]],
}

impl WsInfo {
    pub fn commands(&self) -> impl Iterator<Item = Command> + '_ {
        self.groups.iter().flat_map(|g| g.iter().copied())
    }

    pub fn contains(&self, c: Command) -> bool {
        self.commands().any(|x| x == c)
    }

    fn bar_commands(&self) -> impl Iterator<Item = Command> + '_ {
        self.commands()
            .filter(|command| self.ws != Workspace::Measure || *command != C::ToolMeasure)
    }
}

use Command as C;

/// Segoe Fluent Ruler with a readable fallback when the icon font is absent.
pub const MEASURE_ICON: (&str, &str) = ("\u{ED5E}", "↔");

pub static TABLE: [WsInfo; 18] = [
    WsInfo {
        ws: Workspace::Export,
        name_key: "hub.export.name",
        desc_key: "hub.export.desc",
        icon: (glyph::EXPORT, "↗"),
        color: theme::CAT_VIEW,
        entry: Some(Entry::Direct(C::ExportImages)),
        groups: &[&[C::ExportImages, C::ExportOffice]],
    },
    WsInfo {
        ws: Workspace::FillSign,
        name_key: "hub.fill_sign.name",
        desc_key: "hub.fill_sign.desc",
        icon: (glyph::SIGN, "✍"),
        color: Color32::from_rgb(0x63, 0x66, 0xF1),
        // Moitié « remplir » : formulaires AcroForm ; moitié « signer » : signature visuelle,
        // paraphe, date (texte libre : outil Modifier, touche T).
        entry: Some(Entry::Mode(C::HubFillSign)),
        groups: &[
            &[
                C::ToolFillForm,
                C::ToggleForms,
                C::ClearForm,
                C::FlattenForm,
                C::ExportFormData,
                C::ImportFormData,
            ],
            &[C::SignDocument, C::SignInitials, C::InsertDate],
        ],
    },
    WsInfo {
        ws: Workspace::Edit,
        name_key: "hub.edit.name",
        desc_key: "hub.edit.desc",
        icon: (glyph::EDIT_DOC, "✎"),
        color: theme::CAT_EDIT,
        entry: Some(Entry::Mode(C::HubEdit)),
        groups: &[
            &[
                C::ToolSelectObject,
                C::ToolAddText,
                C::ToolAddImage,
                C::ToolRectangle,
                C::ToolEllipse,
                C::ToolLine,
            ],
            &[
                C::EditObjectText,
                C::DuplicateObject,
                C::RotateObjectLeft,
                C::RotateObjectRight,
                C::BringToFront,
                C::SendToBack,
                C::DeleteObject,
            ],
            &[C::ReplaceImage, C::ToggleObjects],
        ],
    },
    WsInfo {
        ws: Workspace::Create,
        name_key: "hub.create.name",
        desc_key: "hub.create.desc",
        icon: (glyph::ADD, "+"),
        color: theme::CAT_CONVERT,
        entry: Some(Entry::Direct(C::CreateFromImages)),
        groups: &[&[C::CreateFromImages, C::OfficeToPdf]],
    },
    WsInfo {
        ws: Workspace::Combine,
        name_key: "hub.combine.name",
        desc_key: "hub.combine.desc",
        icon: (glyph::COMBINE, "⧉"),
        color: theme::CAT_FILE,
        entry: Some(Entry::Direct(C::CombineFiles)),
        groups: &[&[C::CombineFiles]],
    },
    WsInfo {
        ws: Workspace::Organize,
        name_key: "hub.organize.name",
        desc_key: "hub.organize.desc",
        icon: (glyph::PAGES, "▦"),
        color: theme::CAT_PAGES,
        entry: Some(Entry::Mode(C::HubOrganize)),
        groups: &[
            &[C::OrganizePages],
            &[
                C::RotateCounterClockwise,
                C::RotateClockwise,
                C::DeletePage,
                C::InsertPages,
                C::InsertBlankPage,
                C::MovePageLeft,
                C::MovePageRight,
            ],
            &[C::CropPages, C::ResizePages, C::PageLabels],
            &[C::SplitDocument, C::ExtractPages],
        ],
    },
    WsInfo {
        ws: Workspace::Quantity,
        name_key: "quantity.title",
        desc_key: "quantity.help",
        icon: (glyph::NUMBERS, "#"),
        color: theme::CAT_ANNOTATE,
        entry: Some(Entry::Direct(C::QuantityTable)),
        groups: &[&[C::QuantityTable]],
    },
    WsInfo {
        ws: Workspace::Measure,
        name_key: "hub.measure.name",
        desc_key: "hub.measure.desc",
        icon: MEASURE_ICON,
        color: theme::CAT_ANNOTATE,
        entry: Some(Entry::Mode(C::HubMeasure)),
        groups: &[&[C::ToolMeasure], &[C::QuantityTable]],
    },
    WsInfo {
        ws: Workspace::RevisionOverlay,
        name_key: "overlay.title",
        desc_key: "overlay.description",
        icon: (glyph::PAGES, "◫"),
        color: theme::CAT_PAGES,
        entry: Some(Entry::Direct(C::RevisionOverlay)),
        groups: &[&[C::RevisionOverlay]],
    },
    WsInfo {
        ws: Workspace::Library,
        name_key: "library.title",
        desc_key: "library.description",
        icon: (glyph::ALL_APPS, "▦"),
        color: theme::CAT_ANNOTATE,
        entry: Some(Entry::Direct(C::AnnotationLibrary)),
        groups: &[&[C::AnnotationLibrary]],
    },
    WsInfo {
        ws: Workspace::Comment,
        name_key: "hub.comment.name",
        desc_key: "hub.comment.desc",
        icon: (glyph::COMMENT, "💬"),
        color: theme::CAT_ANNOTATE,
        entry: Some(Entry::Mode(C::HubComment)),
        groups: &[
            &[C::ToolMeasure],
            &[
                C::ToolSelect,
                C::ToolHighlight,
                C::ToolUnderline,
                C::ToolStrikeOut,
                C::ToolNote,
            ],
            &[
                C::ToolSelectAnnot,
                C::ToolInk,
                C::ToolFreeText,
                C::ToolAnnotRect,
                C::ToolAnnotEllipse,
                C::ToolAnnotLine,
                C::ToolLink,
            ],
            &[
                C::RichAnnotations,
                C::AnnotationLibrary,
                C::ToolCount,
                C::AnnotationRegister,
                C::QuantityTable,
                C::EraseInkStrokes,
            ],
            &[C::ToggleComments, C::CopyText],
        ],
    },
    WsInfo {
        ws: Workspace::ScanOcr,
        name_key: "hub.scan_ocr.name",
        desc_key: "hub.scan_ocr.desc",
        icon: (glyph::PICTURE, "🖼"),
        color: Color32::from_rgb(0x06, 0xB6, 0xD4),
        entry: Some(Entry::Mode(C::HubOcr)),
        groups: &[&[C::OcrDocument]],
    },
    WsInfo {
        ws: Workspace::Protect,
        name_key: "hub.protect.name",
        desc_key: "hub.protect.desc",
        icon: (glyph::LOCK, "🔒"),
        color: theme::CAT_SECURITY,
        entry: Some(Entry::Mode(C::HubProtect)),
        groups: &[
            &[C::Protect, C::Unprotect],
            &[C::StripMetadata, C::Sanitize, C::Properties],
        ],
    },
    WsInfo {
        ws: Workspace::Redact,
        name_key: "hub.redact.name",
        desc_key: "hub.redact.desc",
        icon: (glyph::ERASE, "⌫"),
        color: Color32::from_rgb(0x47, 0x55, 0x69),
        // Zones marquées (souris ou recherche), effacées ou appliquées : suppression réelle et
        // vérifiée par le moteur (`sunipdf-core::redact`).
        entry: Some(Entry::Mode(C::HubRedact)),
        groups: &[
            &[C::ToolRedact, C::RedactSearch],
            &[C::RedactClear, C::RedactApply],
        ],
    },
    WsInfo {
        ws: Workspace::Compress,
        name_key: "hub.compress.name",
        desc_key: "hub.compress.desc",
        icon: (glyph::OPTIMIZE, "⤓"),
        color: Color32::from_rgb(0xEA, 0x58, 0x0C),
        entry: Some(Entry::Mode(C::HubCompress)),
        groups: &[&[C::CompressImages], &[C::Optimize, C::ProcessFiles]],
    },
    WsInfo {
        ws: Workspace::PrepareForm,
        name_key: "hub.prepare_form.name",
        desc_key: "hub.prepare_form.desc",
        icon: (glyph::RECT, "▭"),
        color: Color32::from_rgb(0x65, 0xA3, 0x0D),
        entry: Some(Entry::Mode(C::HubPrepareForm)),
        groups: &[&[
            C::ToolAddTextField,
            C::ToolAddCheckbox,
            C::ToolAddRadioField,
            C::ToolAddComboField,
            C::ToolAddListField,
        ]],
    },
    WsInfo {
        ws: Workspace::Stamp,
        name_key: "hub.stamp.name",
        desc_key: "hub.stamp.desc",
        icon: (glyph::STAMP, "◈"),
        color: Color32::from_rgb(0xC0, 0x26, 0xD3),
        entry: Some(Entry::Mode(C::HubStamp)),
        groups: &[&[C::Watermark, C::PageNumbers]],
    },
    WsInfo {
        ws: Workspace::Certificate,
        name_key: "hub.certificate.name",
        desc_key: "hub.certificate.desc",
        icon: (glyph::SHIELD, "◇"),
        color: Color32::from_rgb(0xDB, 0x27, 0x77),
        entry: Some(Entry::Mode(C::HubCertificate)),
        groups: &[&[C::VerifySignatures, C::SignWithCertificate]],
    },
];

impl Workspace {
    pub fn info(self) -> &'static WsInfo {
        &TABLE[self as usize]
    }

    /// Seuls les outils réels : jamais de bouton factice.
    pub fn available() -> impl Iterator<Item = Workspace> {
        TABLE.iter().filter(|i| i.entry.is_some()).map(|i| i.ws)
    }

    /// Commande exécutée par un clic sur la ligne, la carte ou l'entrée de menu.
    pub fn activate(self) -> Option<Command> {
        match self.info().entry? {
            Entry::Direct(c) | Entry::Mode(c) => Some(c),
        }
    }

    pub fn is_mode(self) -> bool {
        matches!(self.info().entry, Some(Entry::Mode(_)))
    }

    /// `Command::HubEdit` -> `Workspace::Edit`.
    pub fn from_command(c: Command) -> Option<Workspace> {
        Self::available().find(|w| w.info().entry == Some(Entry::Mode(c)))
    }

    /// Primary workspace of a command; Measure precedes its shortcut in Comment.
    pub fn owner_of(c: Command) -> Option<Workspace> {
        Self::available().find(|w| w.info().contains(c))
    }
}

impl Tool {
    pub fn from_command(c: Command) -> Option<Tool> {
        Tool::ALL.into_iter().find(|t| t.command() == c)
    }
}

/// Change d'outil. Ferme la grille si l'on quitte Organiser ; abandonne l'outil de dessin s'il
/// n'appartient pas au nouvel outil (retour au curseur de lecture).
pub fn switch(
    a: &mut AnnotateState,
    o: &mut OrganizeState,
    e: &mut EditState,
    to: Option<Workspace>,
) {
    let from = std::mem::replace(&mut a.workspace, to);
    if from == to {
        return;
    }
    if from == Some(Workspace::Measure) {
        a.measure.cancel();
    }
    if from == Some(Workspace::Organize) {
        o.active = false;
    }
    let keep = to.is_some_and(|w| w.info().contains(a.tool.command()));
    if !keep {
        a.tool = Tool::Select;
        a.drag = None;
        a.clear_reading();
        e.selection = None;
        a.adv.selected = None;
        a.adv.ink.clear();
        a.adv.drag = None;
    }
}

/// Garde la barre cohérente quand un outil de dessin ou la grille est lancé hors du hub
/// (raccourci, palette, menu, rail droit).
pub fn follow(a: &mut AnnotateState, o: &mut OrganizeState, e: &mut EditState, command: Command) {
    if let Some(tool) = Tool::from_command(command) {
        let to = if tool == Tool::Select {
            // V = retour au curseur de lecture : on reste dans l'outil s'il propose la sélection de texte.
            a.workspace.filter(|w| w.info().contains(command))
        } else {
            Workspace::owner_of(command).or(a.workspace)
        };
        switch(a, o, e, to);
    } else if command == Command::OrganizePages && o.active {
        switch(a, o, e, Some(Workspace::Organize));
    }
}

/// Effet d'un Échap hors boîtes de dialogue et palette.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Esc {
    /// Ferme le menu ou la liste déroulante ouverts.
    ClosePopup,
    /// Laissé à egui (il vient de retirer le focus du champ) ou à la barre de recherche (elle se
    /// ferme elle-même), ou sans effet.
    Pass,
    Deselect,
    Leave,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct EscState {
    pub popup_open: bool,
    pub focus_last_frame: bool,
    pub find_open: bool,
    pub object_selected: bool,
    pub in_workspace: bool,
}

impl EscState {
    /// Un Échap = un effet, le plus proche d'abord : menu ou liste, champ ou recherche, objet
    /// sélectionné, outil actif (ordre décrit dans les guides utilisateur).
    pub fn action(self) -> Esc {
        if self.popup_open {
            Esc::ClosePopup
        } else if self.focus_last_frame || self.find_open {
            Esc::Pass
        } else if self.object_selected {
            Esc::Deselect
        } else if self.in_workspace {
            Esc::Leave
        } else {
            Esc::Pass
        }
    }
}

/// Consomme Échap selon `s` (à appeler en début d'image, avant tout dessin). Un menu est fermé ici
/// plutôt que par egui : la touche consommée, rien d'autre ne la voit dans la même image.
/// `None` : touche absente, ou laissée à egui ou à la barre de recherche.
pub fn take_escape(ctx: &egui::Context, s: EscState) -> Option<Esc> {
    let action = s.action();
    if action == Esc::Pass
        || !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        return None;
    }
    if action == Esc::ClosePopup {
        egui::Popup::close_all(ctx);
    }
    Some(action)
}

/// Bouton de barre dessiné à la main : fondu au survol, état sélectionné, anneau de focus et
/// libellé AccessKit lisible (un glyphe de la zone privée n'a rien à faire dans un lecteur d'écran).
pub struct BarButton<'a> {
    pub icon: (&'a str, &'a str),
    /// Couleur de l'outil ; `None` = icône neutre (barre d'outils principale).
    pub tint: Option<Color32>,
    pub label: &'a str,
    pub show_label: bool,
    pub tooltip: &'a str,
    pub selected: bool,
    pub enabled: bool,
}

pub const BAR_BUTTON_H: f32 = 34.0;
const ICON_W: f32 = 34.0;
const LABEL_FONT: f32 = 14.0;

pub fn bar_button(ui: &mut egui::Ui, has_icons: bool, b: BarButton<'_>) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    let p = theme::palette(dark);
    let alpha = if b.enabled { 1.0 } else { 0.4 };
    let galley = b.show_label.then(|| {
        ui.painter().layout_no_wrap(
            b.label.to_string(),
            egui::FontId::proportional(LABEL_FONT),
            p.text.gamma_multiply(alpha),
        )
    });
    let w = ICON_W + galley.as_ref().map_or(0.0, |g| g.size().x + 10.0);
    let sense = if b.enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, BAR_BUTTON_H), sense);
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, b.enabled, b.selected, b.label)
    });
    let resp = resp
        .on_hover_text(b.tooltip.to_string())
        .on_disabled_hover_text(b.tooltip.to_string());
    if ui.is_rect_visible(rect) {
        let fade = theme::hover_fade(
            ui,
            resp.id.with("hover"),
            b.enabled && resp.hovered(),
            p.hover,
        );
        let painter = ui.painter();
        let selected_fill = match b.tint {
            Some(t) => theme::badge_fill(t, dark),
            None => p.accent_soft,
        };
        painter.rect_filled(
            rect,
            theme::radius::ROW,
            if b.selected { selected_fill } else { fade },
        );
        let icon_color = match b.tint {
            Some(t) => theme::glyph_color(t, dark),
            None => p.text,
        };
        let (g, f) = b.icon;
        painter.text(
            egui::pos2(rect.left() + ICON_W / 2.0, rect.center().y),
            egui::Align2::CENTER_CENTER,
            if has_icons { g } else { f },
            if has_icons {
                egui::FontId::new(16.0, egui::FontFamily::Name(icons::ICONS.into()))
            } else {
                egui::FontId::proportional(16.0)
            },
            icon_color.gamma_multiply(alpha),
        );
        if let Some(gal) = galley {
            let pos = egui::pos2(
                rect.left() + ICON_W - 4.0,
                rect.center().y - gal.size().y / 2.0,
            );
            painter.galley(pos, gal, p.text);
        }
        if resp.has_focus() {
            theme::focus_ring(painter, rect, &p, theme::radius::ROW);
        }
    }
    if b.enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

impl SuniApp {
    pub(crate) fn leaves_measurement_workspace(&self, command: Command) -> bool {
        self.annotate.workspace == Some(Workspace::Measure)
            && (command == Command::HubLeave
                || Workspace::from_command(command).is_some_and(|ws| ws != Workspace::Measure)
                || Tool::from_command(command).is_some_and(|tool| tool != Tool::Measure))
    }

    pub fn enter_workspace(&mut self, ctx: &egui::Context, ws: Workspace) {
        if self.ink_blocks_command(ws.activate().unwrap_or(Command::HubLeave)) {
            self.ink_blocks_action();
            return;
        }
        self.form_builder.cancel();
        switch(
            &mut self.annotate,
            &mut self.organize,
            &mut self.edit,
            Some(ws),
        );
        // Premier outil de dessin de la liste = outil par défaut (Sélectionner un objet / du texte).
        if let Some(first) = ws
            .info()
            .commands()
            .find(|c| Tool::from_command(*c).is_some() && Workspace::owner_of(*c) == Some(ws))
        {
            self.execute(ctx, first);
        }
        if ws == Workspace::Organize && !self.organize.active {
            self.execute(ctx, Command::OrganizePages);
        }
        if ws == Workspace::ScanOcr {
            self.execute(ctx, Command::OcrDocument);
        }
    }

    pub fn leave_workspace(&mut self) {
        if self.ink_blocks_command(Command::HubLeave) {
            self.ink_blocks_action();
            return;
        }
        self.form_builder.cancel();
        switch(&mut self.annotate, &mut self.organize, &mut self.edit, None);
    }

    /// État lu par Échap en début d'image, avant tout dessin : les menus ouverts sont ceux de
    /// l'image précédente, comme `focus_last_frame`.
    pub fn esc_state(&self, ctx: &egui::Context) -> EscState {
        EscState {
            popup_open: egui::Popup::is_any_open(ctx),
            focus_last_frame: self.focus_last_frame,
            find_open: self.find.open,
            object_selected: self.edit.selection.is_some() || self.annotate.adv.selected.is_some(),
            in_workspace: self.annotate.workspace.is_some(),
        }
    }

    fn command_selected(&self, c: Command) -> bool {
        match c {
            Command::FullScreenReading => self.reading_full_screen,
            Command::OrganizePages => self.organize.active,
            Command::ToggleComments => {
                self.context_panel() == Some(crate::reader_layout::ContextPanel::Comments)
            }
            Command::ToggleObjects => {
                self.context_panel() == Some(crate::reader_layout::ContextPanel::Objects)
            }
            Command::ToggleForms => {
                self.context_panel() == Some(crate::reader_layout::ContextPanel::Forms)
            }
            _ => Tool::from_command(c).is_some_and(|t| t == self.annotate.tool),
        }
    }

    pub fn command_enabled(&self, c: Command) -> bool {
        if self.ink_blocks_command(c) || self.measurement_blocks_panel_command(c) {
            return false;
        }
        let grid_selection = !self.organize.selected.is_empty();
        let single = self
            .edit
            .selection
            .as_ref()
            .and_then(crate::editor::Selection::single);
        match c {
            Command::DeleteObject => {
                self.edit.selection.is_some() || self.annotate.adv.selected.is_some()
            }
            Command::DuplicateObject
            | Command::BringToFront
            | Command::SendToBack
            | Command::CopyObjects
            | Command::CutObjects => self.edit.selection.is_some(),
            Command::RotateObjectLeft | Command::RotateObjectRight => self.edit.selection.is_some(),
            Command::HistoryBack => self.view.reading.history.can_back(),
            Command::HistoryForward => self.view.reading.history.can_forward(),
            Command::PasteObjects | Command::PasteEditableObjects => self.edit.clip.is_some(),
            Command::RedactClear | Command::RedactApply => {
                !self.annotate.redact.marks.is_empty() && !self.annotate.redact.applying
            }
            Command::EditObjectText => {
                single.is_some_and(|i| i.kind == sunipdf_core::ObjectKind::Text)
            }
            Command::ReplaceImage => {
                single.is_some_and(|i| i.kind == sunipdf_core::ObjectKind::Image)
            }
            Command::CopyText => {
                self.annotate
                    .selection
                    .as_ref()
                    .is_some_and(|(_, s)| !s.text.is_empty())
                    || self
                        .annotate
                        .reading
                        .as_ref()
                        .is_some_and(|s| s.pages.iter().any(|(_, page)| !page.text.is_empty()))
            }
            // Sans champ AcroForm, rien à remplir, effacer ni exporter (aplatir vaut aussi pour les
            // annotations : toujours proposé).
            Command::ToolFillForm
            | Command::ClearForm
            | Command::ExportFormData
            | Command::ImportFormData => self
                .document
                .as_ref()
                .is_some_and(|d| d.info.form == sunipdf_core::FormKind::AcroForm),
            // Dans la grille, ces actions portent sur la sélection (sinon sur la page courante).
            Command::RotateClockwise
            | Command::RotateCounterClockwise
            | Command::DeletePage
            | Command::ExtractPages => !self.organize.active || grid_selection,
            // Une seule page à la fois, et pas déjà en bout de document.
            Command::MovePageLeft | Command::MovePageRight => {
                let count = self.document.as_ref().map_or(0, |d| d.info.page_count());
                match self.target_pages()[..] {
                    [page] if c == Command::MovePageLeft => page > 0,
                    [page] => page + 1 < count,
                    _ => false,
                }
            }
            _ => true,
        }
    }

    /// Disposition selon la largeur. Les libellés des outils de dessin passent avant ceux des
    /// actions ; si même les icônes seules ne tiennent pas, la barre devient compacte (nom de l'outil
    /// et libellé « Quitter l'outil » masqués) et les boutons défilent horizontalement.
    fn bar_layout(&self, ui: &egui::Ui, info: &WsInfo) -> BarLayout {
        let lang = self.lang();
        let width = |s: &str, size: f32| {
            ui.painter()
                .layout_no_wrap(
                    s.to_string(),
                    egui::FontId::proportional(size),
                    Color32::WHITE,
                )
                .size()
                .x
        };
        let spacing = ui.spacing().item_spacing.x;
        let count = info.bar_commands().count() as f32;
        // Marges (6 + 6), retour, pastille, nom (semi-gras, ~8 % plus large), un séparateur de 6 px
        // par groupe, bouton « Quitter l'outil », espacements entre ces éléments, 12 px de sécurité.
        let fixed = 12.0
            + ICON_W
            + 28.0
            + width(tr(lang, info.name_key), 15.0) * 1.08
            + info.groups.len() as f32 * (6.0 + spacing)
            + ICON_W
            + width(tr(lang, "hub.leave"), LABEL_FONT)
            + 10.0
            + 4.0 * spacing
            + 12.0;
        let label_w = |tools: bool| -> f32 {
            info.bar_commands()
                .filter(|c| Tool::from_command(*c).is_some() == tools)
                .map(|c| width(tr(lang, c.info().label_key), LABEL_FONT) + 10.0)
                .sum()
        };
        let room = ui.available_width() - fixed - count * (ICON_W + spacing);
        let tools = label_w(true);
        // Libellé de l'outil de dessin actif, affiché même en icônes seules.
        let active_label: f32 = info
            .bar_commands()
            .filter(|c| Tool::from_command(*c) == Some(self.annotate.tool))
            .map(|c| width(tr(lang, c.info().label_key), LABEL_FONT) + 10.0)
            .sum();
        BarLayout {
            tool_labels: tools <= room,
            action_labels: tools + label_w(false) <= room,
            compact: room < active_label,
        }
    }

    /// Barre contextuelle de l'outil actif (entre la barre d'outils et la recherche).
    pub fn workspace_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let Some(ws) = self.annotate.workspace else {
            return;
        };
        let info = ws.info();
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let p = theme::palette(dark);
        let has_icons = self.has_icons;
        let layout = self.bar_layout(ui, info);
        let mut run = None;
        let mut back = false;
        let mut leave = false;
        // Filet de la couleur de l'outil, en haut de la barre.
        let full = ui.max_rect();
        ui.painter().rect_filled(
            egui::Rect::from_min_size(full.min, egui::vec2(full.width(), 2.0)),
            0.0,
            info.color,
        );
        ui.add_space(6.0);
        let leave_tip = format!(
            "{}  ({})",
            tr(lang, "hub.leave"),
            tr(lang, "shortcut.hub_leave")
        );
        // « Quitter l'outil » est placé d'abord, à droite ; le reste se partage la largeur restante.
        egui::containers::Sides::new()
            .shrink_left()
            .height(BAR_BUTTON_H)
            .show(
                ui,
                |ui| {
                    ui.add_space(6.0);
                    back = bar_button(
                        ui,
                        has_icons,
                        BarButton {
                            icon: (glyph::ARROW_LEFT, "‹"),
                            tint: None,
                            label: tr(lang, "hub.back"),
                            show_label: false,
                            tooltip: tr(lang, "hub.back"),
                            selected: false,
                            enabled: true,
                        },
                    )
                    .clicked();
                    crate::shell::badge(ui, has_icons, info.icon, info.color, 28.0, dark);
                    if !layout.compact {
                        ui.label(icons::semibold(tr(lang, info.name_key), 15.0).color(p.text));
                    }
                    egui::ScrollArea::horizontal()
                        .id_salt("workspace-bar")
                        .show(ui, |ui| {
                            // Rangée de la hauteur des boutons : même centre vertical que la pastille.
                            ui.spacing_mut().interact_size.y = BAR_BUTTON_H;
                            ui.horizontal(|ui| {
                                for group in info.groups {
                                    let commands: Vec<_> = group
                                        .iter()
                                        .copied()
                                        .filter(|command| {
                                            ws != Workspace::Measure || *command != C::ToolMeasure
                                        })
                                        .collect();
                                    if commands.is_empty() {
                                        continue;
                                    }
                                    ui.separator();
                                    for c in commands {
                                        if self.bar_command(ui, info, c, layout) {
                                            run = Some(c);
                                        }
                                    }
                                }
                            });
                        });
                },
                |ui| {
                    ui.add_space(6.0);
                    let r = bar_button(
                        ui,
                        has_icons,
                        BarButton {
                            icon: (glyph::CLOSE, "✕"),
                            tint: None,
                            label: tr(lang, "hub.leave"),
                            show_label: !layout.compact,
                            tooltip: &leave_tip,
                            selected: false,
                            enabled: true,
                        },
                    );
                    if r.clicked() {
                        r.surrender_focus();
                        leave = true;
                    }
                },
            );
        if leave {
            run = Some(Command::HubLeave);
        }
        if back {
            self.set_navigation_panel(Some(crate::reader_layout::NavigationPanel::Tools));
            run = Some(Command::HubLeave);
        }
        if let Some(c) = run {
            self.execute(ctx, c);
        }
    }

    /// Un bouton de commande de la barre ; `true` si cliqué.
    fn bar_command(&self, ui: &mut egui::Ui, info: &WsInfo, c: Command, layout: BarLayout) -> bool {
        let lang = self.lang();
        let ci = c.info();
        let label = tr(lang, ci.label_key);
        let shortcut = self.shortcut_text(ci.command);
        let tooltip = if shortcut.is_empty() {
            label.to_string()
        } else {
            format!("{label}  ({shortcut})")
        };
        let is_tool = Tool::from_command(c).is_some();
        let selected = self.command_selected(c);
        let r = bar_button(
            ui,
            self.has_icons,
            BarButton {
                icon: crate::shell::command_icon(c),
                tint: Some(info.color),
                label,
                // L'outil de dessin actif garde son libellé même en icônes seules (sauf compact).
                show_label: if matches!(c, C::ToolMeasure | C::AnnotationLibrary | C::QuantityTable)
                {
                    true
                } else if is_tool {
                    layout.tool_labels || (selected && !layout.compact)
                } else {
                    layout.action_labels
                },
                tooltip: &tooltip,
                selected,
                enabled: self.command_enabled(c),
            },
        );
        if r.clicked() {
            r.surrender_focus();
        }
        r.clicked()
    }
}

#[derive(Clone, Copy)]
struct BarLayout {
    tool_labels: bool,
    action_labels: bool,
    compact: bool,
}
#[cfg(test)]
mod tests {
    use super::*;
    use sunipdf_i18n::Language;

    /// Commandes système, de vue ou de navigation : volontairement hors outils.
    const SYSTEM_COMMANDS: &[Command] = &[
        C::CompareFiles,
        C::SearchFiles,
        C::RotateViewClockwise,
        C::RotateViewCounterClockwise,
        C::FullScreenReading,
        C::HistoryBack,
        C::HistoryForward,
        C::OpenDocument,
        C::CloseDocument,
        C::Quit,
        C::Save,
        C::SaveAs,
        C::Undo,
        C::Redo,
        C::Find,
        C::FirstPage,
        C::PreviousPage,
        C::NextPage,
        C::LastPage,
        C::ZoomIn,
        C::ZoomOut,
        C::ActualSize,
        C::FitWidth,
        C::FitPage,
        C::ViewContinuous,
        C::ViewSinglePage,
        C::ViewTwoPage,
        C::ViewBook,
        C::PrintDocument,
        C::RecoverDocuments,
        C::ToggleThumbnails,
        C::ToggleBookmarks,
        C::ToggleTools,
        C::CommandPalette,
        C::NextTab,
        C::PrevTab,
        C::About,
        C::KeyboardShortcuts,
        // Presse-papiers d'objets : clavier, menu Édition, palette et barre d'objet (comme
        // Annuler / Rétablir), pas dans la barre d'outil (DECISIONS.md).
        C::CopyObjects,
        C::CutObjects,
        C::PasteObjects,
        C::PasteEditableObjects,
        C::SelectAllObjects,
    ];

    const HUB_COMMANDS: &[Command] = &[
        C::HubCompress,
        C::HubFillSign,
        C::HubEdit,
        C::HubOrganize,
        C::HubComment,
        C::HubMeasure,
        C::HubProtect,
        C::HubStamp,
        C::HubRedact,
        C::HubPrepareForm,
        C::HubOcr,
        C::HubCertificate,
        C::HubLeave,
    ];

    #[test]
    fn table_order_matches_enum() {
        for (i, info) in TABLE.iter().enumerate() {
            assert_eq!(info.ws as usize, i, "{:?}", info.ws);
            assert!(std::ptr::eq(info.ws.info(), info));
        }
    }

    #[test]
    fn names_and_descriptions_exist_in_both_catalogs() {
        for info in &TABLE {
            for key in [info.name_key, info.desc_key] {
                for lang in Language::ALL {
                    assert_ne!(tr(lang, key), key, "clé {key} absente en {}", lang.code());
                }
            }
        }
    }

    #[test]
    fn unavailable_tools_are_inert() {
        for info in TABLE.iter().filter(|i| i.entry.is_none()) {
            assert!(info.groups.is_empty(), "{:?}", info.ws);
            assert!(Workspace::available().all(|w| w != info.ws));
        }
    }

    #[test]
    fn available_tools_are_wired() {
        assert_eq!(Workspace::available().count(), 18);
        for ws in Workspace::available() {
            let info = ws.info();
            match info.entry.unwrap() {
                Entry::Mode(c) => {
                    assert!(c.info().id.starts_with("hub."), "{:?}", ws);
                    assert!(c.info().needs_document);
                    assert_eq!(Workspace::from_command(c), Some(ws));
                    assert!(info.commands().count() >= 1, "{:?}", ws);
                    assert!(ws.is_mode());
                }
                Entry::Direct(c) => {
                    assert_eq!(info.commands().next(), Some(c));
                    assert_eq!(Workspace::owner_of(c), Some(ws));
                    assert!(!ws.is_mode());
                }
            }
            assert!(ws.activate().is_some());
        }
    }

    #[test]
    fn measurements_have_a_dedicated_workspace_and_a_comment_shortcut() {
        assert_eq!(Workspace::Measure.activate(), Some(C::HubMeasure));
        assert_eq!(
            Workspace::owner_of(C::ToolMeasure),
            Some(Workspace::Measure)
        );
        assert_eq!(
            Workspace::from_command(C::HubMeasure),
            Some(Workspace::Measure)
        );
        assert_eq!(Workspace::Comment.info().groups[0], &[C::ToolMeasure]);
        assert_eq!(Workspace::Measure.info().groups[0], &[C::ToolMeasure]);
        assert!(Workspace::available().any(|w| w == Workspace::Measure));
    }

    #[test]
    fn measurement_context_shows_one_title_and_keeps_the_quantity_action() {
        assert_eq!(
            Workspace::Measure.info().bar_commands().collect::<Vec<_>>(),
            vec![C::QuantityTable]
        );
        assert!(Workspace::Comment
            .info()
            .bar_commands()
            .any(|c| c == C::ToolMeasure));
        for lang in Language::ALL {
            for width in [640.0, 1280.0] {
                let (mut app, _, _) = crate::app::tests::test_app();
                app.settings.set_language(lang);
                let ctx = egui::Context::default();
                app.has_icons = icons::install(&ctx);
                ctx.enable_accesskit();
                app.execute(&ctx, C::HubMeasure);
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 160.0),
                    )),
                    ..Default::default()
                };
                let mut output = ctx.run_ui(input.clone(), |ui| app.workspace_bar(&ctx, ui));
                output.textures_delta.clear();
                let mut output = ctx.run_ui(input, |ui| app.workspace_bar(&ctx, ui));
                output.textures_delta.clear();
                let title = tr(lang, Workspace::Measure.info().name_key);
                assert_eq!(output.shapes.iter().filter(|shape| {
                    matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == title)
                }).count(), 1);
                let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                assert!(!tree.nodes.iter().any(|(_, node)| {
                    node.role() == egui::accesskit::Role::Button
                        && node.label() == Some(tr(lang, C::ToolMeasure.info().label_key))
                }));
                assert!(tree.nodes.iter().any(|(_, node)| {
                    node.role() == egui::accesskit::Role::Button
                        && node.label() == Some(tr(lang, C::QuantityTable.info().label_key))
                }));
            }
        }
    }

    #[test]
    fn new_hub_tools_open_real_feature_controls() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.execute(&ctx, Workspace::ScanOcr.activate().unwrap());
        assert_eq!(app.annotate.workspace, Some(Workspace::ScanOcr));
        assert!(app.ocr.open);
        app.ocr.open = false;
        app.execute(&ctx, Workspace::PrepareForm.activate().unwrap());
        app.execute(&ctx, C::ToolAddTextField);
        assert!(app.form_builder.visible);
        app.form_builder.visible = false;
        app.execute(&ctx, Workspace::Certificate.activate().unwrap());
        app.execute(&ctx, C::VerifySignatures);
        assert!(app.certificate.visible);
        assert!(requests
            .try_iter()
            .any(|r| matches!(r, crate::worker::Request::VerifySignatures)));
    }

    #[test]
    fn listed_commands_have_unique_owners_except_the_measurement_shortcut() {
        let mut seen = std::collections::HashSet::new();
        for info in &TABLE {
            assert!(info.groups.iter().all(|g| !g.is_empty()));
            for c in info.commands() {
                let _ = c.info(); // panique si la commande n'est pas au registre
                let shared_measure_shortcut = matches!(c, C::ToolMeasure | C::AnnotationLibrary)
                    && info.ws == Workspace::Comment;
                assert!(
                    seen.insert(c)
                        || shared_measure_shortcut
                        || (c == C::QuantityTable
                            && matches!(info.ws, Workspace::Measure | Workspace::Comment)),
                    "{c:?} appartient à deux outils"
                );
            }
        }
    }

    #[test]
    fn every_registry_command_has_a_home() {
        for entry in sunipdf_core::REGISTRY {
            let c = entry.command;
            let owner = Workspace::owner_of(c);
            if SYSTEM_COMMANDS.contains(&c) || HUB_COMMANDS.contains(&c) {
                assert!(owner.is_none(), "{c:?} ne doit appartenir à aucun outil");
            } else {
                assert!(owner.is_some(), "{c:?} n'a pas d'outil dans le hub");
            }
        }
        for c in HUB_COMMANDS.iter().filter(|c| **c != C::HubLeave) {
            assert!(Workspace::from_command(*c).is_some(), "{c:?}");
        }
    }

    fn luminance(c: Color32) -> f64 {
        let lin = |v: u8| {
            let s = v as f64 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Couleur prémultipliée `src` composée sur un fond opaque.
    fn over(src: Color32, dst: Color32) -> Color32 {
        let k = 1.0 - src.a() as f32 / 255.0;
        let mix = |s: u8, d: u8| (s as f32 + d as f32 * k).round().min(255.0) as u8;
        Color32::from_rgb(
            mix(src.r(), dst.r()),
            mix(src.g(), dst.g()),
            mix(src.b(), dst.b()),
        )
    }

    #[test]
    fn tool_glyph_contrast() {
        for dark in [false, true] {
            let surface = theme::palette(dark).surface;
            for info in &TABLE {
                let bg = over(theme::badge_fill(info.color, dark), surface);
                let ratio = contrast(theme::glyph_color(info.color, dark), bg);
                assert!(ratio >= 3.5, "{:?} sombre={dark} : {ratio:.2}:1", info.ws);
            }
        }
    }

    /// Dans une barre, aucune icône n'en répète une autre ni celle d'un bouton d'accès rapide
    /// visible en même temps (Pivoter à gauche reprenait le glyphe d'Annuler).
    #[test]
    fn bar_icons_are_distinct() {
        let quick = [C::OpenDocument, C::Save, C::Undo, C::Redo];
        for info in TABLE.iter().filter(|i| i.ws.is_mode()) {
            let mut seen: Vec<&str> = quick
                .iter()
                .map(|c| crate::shell::command_icon(*c).0)
                .collect();
            for c in info.commands() {
                let g = crate::shell::command_icon(c).0;
                assert!(!seen.contains(&g), "{:?} : icône de {c:?} répétée", info.ws);
                seen.push(g);
            }
        }
    }

    #[test]
    fn switch_resets_drawing_tool() {
        let (mut a, mut o, mut e) = Default::default();
        switch(&mut a, &mut o, &mut e, Some(Workspace::Comment));
        a.tool = Tool::Highlight;
        switch(&mut a, &mut o, &mut e, Some(Workspace::Comment));
        assert_eq!(a.tool, Tool::Highlight);
        switch(&mut a, &mut o, &mut e, Some(Workspace::Edit));
        assert_eq!(a.tool, Tool::Select);
        a.tool = Tool::Rectangle;
        switch(&mut a, &mut o, &mut e, None);
        assert_eq!((a.tool, a.workspace), (Tool::Select, None));
    }

    #[test]
    fn leaving_organize_closes_grid() {
        let (mut a, mut o, mut e): (AnnotateState, OrganizeState, EditState) = Default::default();
        o.active = true;
        switch(&mut a, &mut o, &mut e, Some(Workspace::Organize));
        assert!(o.active);
        switch(&mut a, &mut o, &mut e, None);
        assert!(!o.active);
        o.active = true;
        switch(&mut a, &mut o, &mut e, Some(Workspace::Organize));
        switch(&mut a, &mut o, &mut e, Some(Workspace::Comment));
        assert!(!o.active);
    }

    #[test]
    fn follow_tool_shortcuts() {
        let (mut a, mut o, mut e): (AnnotateState, OrganizeState, EditState) = Default::default();
        // H sans outil actif : ouvre Commenter.
        a.tool = Tool::Highlight;
        follow(&mut a, &mut o, &mut e, C::ToolHighlight);
        assert_eq!(a.workspace, Some(Workspace::Comment));
        assert_eq!(a.tool, Tool::Highlight);
        // V dans Commenter : reste dans l'outil (il propose la sélection de texte).
        a.tool = Tool::Select;
        follow(&mut a, &mut o, &mut e, C::ToolSelect);
        assert_eq!(a.workspace, Some(Workspace::Comment));
        // R depuis Commenter : bascule vers Modifier.
        a.tool = Tool::Rectangle;
        follow(&mut a, &mut o, &mut e, C::ToolRectangle);
        assert_eq!(a.workspace, Some(Workspace::Edit));
        assert_eq!(a.tool, Tool::Rectangle);
        // V dans Modifier : retour à la lecture.
        a.tool = Tool::Select;
        follow(&mut a, &mut o, &mut e, C::ToolSelect);
        assert_eq!(a.workspace, None);
    }

    #[test]
    fn escape_closes_the_nearest_thing_first() {
        let tool = EscState {
            in_workspace: true,
            ..Default::default()
        };
        assert_eq!(EscState::default().action(), Esc::Pass);
        assert_eq!(tool.action(), Esc::Leave);
        let selected = EscState {
            object_selected: true,
            ..tool
        };
        assert_eq!(selected.action(), Esc::Deselect);
        let field = EscState {
            focus_last_frame: true,
            ..selected
        };
        let find = EscState {
            find_open: true,
            ..selected
        };
        assert_eq!(field.action(), Esc::Pass);
        assert_eq!(find.action(), Esc::Pass);
        for s in [tool, selected, field, find] {
            let menu = EscState {
                popup_open: true,
                ..s
            };
            assert_eq!(menu.action(), Esc::ClosePopup, "{s:?}");
        }
    }

    /// Scénario de la revue : outil actif, menu ouvert, Échap. Lu avant le dessin comme dans
    /// `handle_input`, l'état des menus est celui de l'image précédente : Échap ferme le menu et
    /// est consommé (ni la recherche ni l'outil ne le voient) ; l'Échap suivant quitte l'outil.
    #[test]
    fn escape_with_an_open_menu_closes_only_the_menu() {
        let ctx = egui::Context::default();
        let tool = EscState {
            in_workspace: true,
            ..Default::default()
        };
        let esc = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        // Une image : (effet d'Échap, menu affiché, Échap encore visible après `take_escape`).
        let frame = |events: Vec<egui::Event>, open_menu: bool| {
            let mut seen = (None, false, false);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                let s = EscState {
                    popup_open: egui::Popup::is_any_open(ui.ctx()),
                    ..tool
                };
                seen.0 = take_escape(ui.ctx(), s);
                seen.2 = ui.input(|i| i.key_pressed(egui::Key::Escape));
                let menu = ui.menu_button("Menu", |ui| ui.label("…"));
                seen.1 = menu.inner.is_some();
                if open_menu {
                    let id = egui::Popup::default_response_id(&menu.response);
                    egui::Popup::open_id(ui.ctx(), id);
                }
            });
            out.textures_delta.clear();
            seen
        };
        let _ = frame(vec![], true);
        assert_eq!(frame(vec![], false), (None, true, false));
        assert_eq!(
            frame(vec![esc.clone()], false),
            (Some(Esc::ClosePopup), false, false)
        );
        assert_eq!(frame(vec![esc], false), (Some(Esc::Leave), false, false));
    }

    #[test]
    fn follow_organize_grid() {
        let (mut a, mut o, mut e): (AnnotateState, OrganizeState, EditState) = Default::default();
        follow(&mut a, &mut o, &mut e, C::OrganizePages);
        assert_eq!(a.workspace, None);
        o.active = true;
        follow(&mut a, &mut o, &mut e, C::OrganizePages);
        assert_eq!(a.workspace, Some(Workspace::Organize));
    }
}
