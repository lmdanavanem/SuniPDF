//! Registre central des commandes. Données pures (aucune dépendance UI) : l'application, la CLI,
//! la future palette de commandes et l'automatisation lisent le même tableau.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    OpenDocument,
    CloseDocument,
    Quit,
    FirstPage,
    PreviousPage,
    NextPage,
    LastPage,
    ZoomIn,
    ZoomOut,
    ActualSize,
    FitWidth,
    FitPage,
    ToggleThumbnails,
    Find,
    Save,
    SaveAs,
    RotateClockwise,
    RotateCounterClockwise,
    DeletePage,
    ToggleTools,
    CommandPalette,
    Undo,
    Redo,
    InsertPages,
    MovePageLeft,
    MovePageRight,
    OrganizePages,
    CopyText,
    ToolSelect,
    ToolHighlight,
    ToolUnderline,
    ToolStrikeOut,
    ToolNote,
    ToggleComments,
    ToggleBookmarks,
    Properties,
    Protect,
    Unprotect,
    StripMetadata,
    CombineFiles,
    SplitDocument,
    ExtractPages,
    CreateFromImages,
    ExportImages,
    Optimize,
    About,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    File,
    Edit,
    View,
    Pages,
    Security,
    Convert,
    Help,
}

#[derive(Debug)]
pub struct CommandInfo {
    pub command: Command,
    /// Identifiant stable (CLI, automatisation, raccourcis configurables).
    pub id: &'static str,
    /// Clé i18n du libellé.
    pub label_key: &'static str,
    /// Clé i18n du raccourci affiché, s'il existe.
    pub shortcut_key: Option<&'static str>,
    pub needs_document: bool,
    pub menu: Menu,
}

macro_rules! cmd {
    ($c:ident, $id:literal, $label:literal, $shortcut:expr, $doc:literal, $menu:ident) => {
        CommandInfo {
            command: Command::$c,
            id: $id,
            label_key: $label,
            shortcut_key: $shortcut,
            needs_document: $doc,
            menu: Menu::$menu,
        }
    };
}

/// Ordre = ordre d'affichage dans les menus.
pub const REGISTRY: &[CommandInfo] = &[
    cmd!(
        OpenDocument,
        "file.open",
        "menu.file.open",
        Some("shortcut.open"),
        false,
        File
    ),
    cmd!(
        CloseDocument,
        "file.close",
        "menu.file.close",
        Some("shortcut.close"),
        true,
        File
    ),
    cmd!(
        Save,
        "file.save",
        "menu.file.save",
        Some("shortcut.save"),
        true,
        File
    ),
    cmd!(
        SaveAs,
        "file.save_as",
        "menu.file.save_as",
        Some("shortcut.save_as"),
        true,
        File
    ),
    cmd!(
        CombineFiles,
        "file.combine",
        "menu.file.combine",
        None,
        false,
        File
    ),
    cmd!(
        Properties,
        "file.properties",
        "menu.file.properties",
        Some("shortcut.properties"),
        true,
        File
    ),
    cmd!(
        Quit,
        "app.quit",
        "menu.file.quit",
        Some("shortcut.quit"),
        false,
        File
    ),
    cmd!(
        ZoomIn,
        "view.zoom_in",
        "menu.view.zoom_in",
        Some("shortcut.zoom_in"),
        true,
        View
    ),
    cmd!(
        ZoomOut,
        "view.zoom_out",
        "menu.view.zoom_out",
        Some("shortcut.zoom_out"),
        true,
        View
    ),
    cmd!(
        ActualSize,
        "view.actual_size",
        "menu.view.actual_size",
        Some("shortcut.actual_size"),
        true,
        View
    ),
    cmd!(
        FitWidth,
        "view.fit_width",
        "menu.view.fit_width",
        None,
        true,
        View
    ),
    cmd!(
        FitPage,
        "view.fit_page",
        "menu.view.fit_page",
        None,
        true,
        View
    ),
    cmd!(
        FirstPage,
        "nav.first_page",
        "menu.view.first_page",
        Some("shortcut.first_page"),
        true,
        View
    ),
    cmd!(
        PreviousPage,
        "nav.previous_page",
        "menu.view.previous_page",
        Some("shortcut.previous_page"),
        true,
        View
    ),
    cmd!(
        NextPage,
        "nav.next_page",
        "menu.view.next_page",
        Some("shortcut.next_page"),
        true,
        View
    ),
    cmd!(
        LastPage,
        "nav.last_page",
        "menu.view.last_page",
        Some("shortcut.last_page"),
        true,
        View
    ),
    cmd!(
        ToggleThumbnails,
        "view.thumbnails",
        "menu.view.thumbnails",
        Some("shortcut.thumbnails"),
        false,
        View
    ),
    cmd!(
        OrganizePages,
        "view.organize",
        "menu.view.organize",
        Some("shortcut.organize"),
        true,
        View
    ),
    cmd!(
        ToggleTools,
        "view.tools",
        "menu.view.tools",
        Some("shortcut.tools"),
        false,
        View
    ),
    cmd!(
        CommandPalette,
        "app.palette",
        "menu.view.palette",
        Some("shortcut.palette"),
        false,
        View
    ),
    cmd!(
        RotateClockwise,
        "pages.rotate_cw",
        "menu.pages.rotate_cw",
        Some("shortcut.rotate_cw"),
        true,
        Pages
    ),
    cmd!(
        RotateCounterClockwise,
        "pages.rotate_ccw",
        "menu.pages.rotate_ccw",
        Some("shortcut.rotate_ccw"),
        true,
        Pages
    ),
    cmd!(
        DeletePage,
        "pages.delete",
        "menu.pages.delete",
        Some("shortcut.delete_page"),
        true,
        Pages
    ),
    cmd!(
        SplitDocument,
        "pages.split",
        "menu.pages.split",
        None,
        true,
        Pages
    ),
    cmd!(
        ExtractPages,
        "pages.extract",
        "menu.pages.extract",
        None,
        true,
        Pages
    ),
    cmd!(
        InsertPages,
        "pages.insert",
        "menu.pages.insert",
        Some("shortcut.insert"),
        true,
        Pages
    ),
    cmd!(
        MovePageLeft,
        "pages.move_left",
        "menu.pages.move_left",
        None,
        true,
        Pages
    ),
    cmd!(
        MovePageRight,
        "pages.move_right",
        "menu.pages.move_right",
        None,
        true,
        Pages
    ),
    cmd!(
        CopyText,
        "edit.copy",
        "menu.edit.copy",
        Some("shortcut.copy"),
        true,
        Edit
    ),
    cmd!(
        ToolSelect,
        "tool.select",
        "menu.edit.tool_select",
        Some("shortcut.tool_select"),
        true,
        Edit
    ),
    cmd!(
        ToolHighlight,
        "tool.highlight",
        "menu.edit.tool_highlight",
        Some("shortcut.tool_highlight"),
        true,
        Edit
    ),
    cmd!(
        ToolUnderline,
        "tool.underline",
        "menu.edit.tool_underline",
        Some("shortcut.tool_underline"),
        true,
        Edit
    ),
    cmd!(
        ToolStrikeOut,
        "tool.strikeout",
        "menu.edit.tool_strikeout",
        Some("shortcut.tool_strikeout"),
        true,
        Edit
    ),
    cmd!(
        ToolNote,
        "tool.note",
        "menu.edit.tool_note",
        Some("shortcut.tool_note"),
        true,
        Edit
    ),
    cmd!(
        ToggleBookmarks,
        "view.bookmarks",
        "menu.view.bookmarks",
        Some("shortcut.bookmarks"),
        true,
        View
    ),
    cmd!(
        Protect,
        "security.protect",
        "menu.security.protect",
        None,
        true,
        Security
    ),
    cmd!(
        Unprotect,
        "security.unprotect",
        "menu.security.unprotect",
        None,
        true,
        Security
    ),
    cmd!(
        StripMetadata,
        "security.strip_metadata",
        "menu.security.strip_metadata",
        None,
        true,
        Security
    ),
    cmd!(
        CreateFromImages,
        "convert.images_to_pdf",
        "menu.file.create_images",
        None,
        false,
        Convert
    ),
    cmd!(
        ExportImages,
        "convert.export_images",
        "menu.file.export_images",
        None,
        true,
        Convert
    ),
    cmd!(
        Optimize,
        "convert.optimize",
        "menu.file.optimize",
        None,
        true,
        Convert
    ),
    cmd!(
        ToggleComments,
        "view.comments",
        "menu.view.comments",
        Some("shortcut.comments"),
        true,
        View
    ),
    cmd!(
        Undo,
        "edit.undo",
        "menu.edit.undo",
        Some("shortcut.undo"),
        true,
        Edit
    ),
    cmd!(
        Redo,
        "edit.redo",
        "menu.edit.redo",
        Some("shortcut.redo"),
        true,
        Edit
    ),
    cmd!(
        Find,
        "edit.find",
        "menu.edit.find",
        Some("shortcut.find"),
        true,
        Edit
    ),
    cmd!(About, "help.about", "menu.help.about", None, false, Help),
];

impl Command {
    pub fn info(self) -> &'static CommandInfo {
        REGISTRY
            .iter()
            .find(|c| c.command == self)
            .expect("toute commande est enregistrée")
    }

    pub fn from_id(id: &str) -> Option<Command> {
        REGISTRY.iter().find(|c| c.id == id).map(|c| c.command)
    }

    pub fn in_menu(menu: Menu) -> impl Iterator<Item = &'static CommandInfo> {
        REGISTRY.iter().filter(move |c| c.menu == menu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique_and_round_trip() {
        let ids: HashSet<&str> = REGISTRY.iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), REGISTRY.len());
        for info in REGISTRY {
            assert_eq!(Command::from_id(info.id), Some(info.command));
            assert_eq!(info.command.info().id, info.id);
        }
        assert_eq!(Command::from_id("nope"), None);
    }

    #[test]
    fn every_command_variant_is_registered() {
        // Si une variante est ajoutée sans entrée dans REGISTRY, `info()` panique : on l'exerce ici.
        for info in REGISTRY {
            let _ = info.command.info();
        }
        assert_eq!(Command::in_menu(Menu::File).count(), 7);
    }
}
