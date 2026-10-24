//! Validated user overrides for the existing application command shortcuts.

use crate::shortcut_defaults::DEFAULTS;
use egui::{Key, Modifiers};
use std::collections::BTreeMap;
use sunipdf_core::Command;
use sunipdf_i18n::tr;

pub type Bindings = BTreeMap<String, Option<String>>;
type Binding = (Modifiers, Key, Command);

#[derive(Debug, PartialEq)]
enum Invalid {
    Syntax,
    Reserved,
    Conflict(Command),
    Stored,
}

fn text(m: Modifiers, key: Key) -> String {
    format!(
        "{}{}{}",
        if m.command || m.ctrl { "Ctrl+" } else { "" },
        if m.shift { "Shift+" } else { "" },
        match key {
            Key::Plus => "Plus",
            Key::Equals => "Equals",
            Key::Minus => "Minus",
            Key::OpenBracket => "OpenBracket",
            Key::CloseBracket => "CloseBracket",
            _ => key.name(),
        }
    )
}

/// Displayed shortcut; macOS uses its modifier symbols, the stored syntax stays `text`.
fn display(m: Modifiers, key: Key) -> String {
    if cfg!(target_os = "macos") {
        let key = match key {
            Key::Backspace => "⌫",
            Key::Delete => "⌦",
            _ => key.symbol_or_name(),
        };
        format!(
            "{}{}{}{key}",
            if m.ctrl { "⌃" } else { "" },
            if m.shift { "⇧" } else { "" },
            if m.command { "⌘" } else { "" }
        )
    } else {
        text(m, key)
    }
}

/// ⌘ and ⌃ are distinct keys on a Mac; elsewhere Ctrl is the command modifier.
fn command_matches(pressed: Modifiers, binding: Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        pressed.command == binding.command && pressed.ctrl == binding.ctrl
    } else {
        (pressed.command || pressed.ctrl) == (binding.command || binding.ctrl) && !pressed.mac_cmd
    }
}

fn parse(value: &str) -> Result<(Modifiers, Key), Invalid> {
    if value.len() > 64 {
        return Err(Invalid::Syntax);
    }
    let parts: Vec<_> = value.split('+').map(str::trim).collect();
    let (shift, name) = match parts.as_slice() {
        [ctrl, key] if ctrl.eq_ignore_ascii_case("ctrl") => (false, *key),
        [ctrl, shift, key]
            if ctrl.eq_ignore_ascii_case("ctrl") && shift.eq_ignore_ascii_case("shift") =>
        {
            (true, *key)
        }
        _ => return Err(Invalid::Syntax),
    };
    let key = Key::ALL
        .iter()
        .copied()
        .find(|k| text(Modifiers::NONE, *k).eq_ignore_ascii_case(name))
        .ok_or(Invalid::Syntax)?;
    let name = key.name();
    let allowed = (name.len() == 1 && name.as_bytes()[0].is_ascii_alphanumeric())
        || matches!(
            key,
            Key::F2
                | Key::F3
                | Key::F4
                | Key::F5
                | Key::F6
                | Key::F7
                | Key::F8
                | Key::F9
                | Key::F10
                | Key::F11
                | Key::F12
                | Key::Tab
                | Key::Delete
                | Key::Plus
                | Key::Equals
                | Key::Minus
                | Key::OpenBracket
                | Key::CloseBracket
        );
    if !allowed {
        return Err(Invalid::Syntax);
    }
    if matches!(key, Key::A | Key::C | Key::X | Key::V | Key::Z | Key::Y)
        || (shift && key == Key::Delete)
        || matches!(key, Key::F4 | Key::F6)
    {
        return Err(Invalid::Reserved);
    }
    let shift = shift && key != Key::Plus;
    Ok((
        Modifiers::COMMAND.plus(if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        }),
        key,
    ))
}

fn resolve(bindings: &Bindings) -> Result<Vec<Binding>, Invalid> {
    if bindings.len() > DEFAULTS.len() {
        return Err(Invalid::Stored);
    }
    let mut result: Vec<_> = DEFAULTS
        .iter()
        .copied()
        .filter(|(_, _, c)| !bindings.contains_key(c.info().id))
        .collect();
    for (id, value) in bindings {
        let command = Command::from_id(id)
            .filter(|c| DEFAULTS.iter().any(|b| b.2 == *c))
            .ok_or(Invalid::Stored)?;
        if let Some(value) = value {
            let (m, key) = parse(value)?;
            if let Some((_, _, other)) = result.iter().find(|(m2, k2, _)| *m2 == m && *k2 == key) {
                return Err(Invalid::Conflict(*other));
            }
            result.push((m, key, command));
        }
    }
    Ok(result)
}

fn effective(bindings: &Bindings) -> Vec<Binding> {
    resolve(bindings).unwrap_or_else(|_| DEFAULTS.to_vec())
}

/// Effective shortcuts that stay active in a text field (macOS menu key equivalents).
#[cfg(target_os = "macos")]
pub fn field_bindings(bindings: &Bindings) -> Vec<Binding> {
    effective(bindings)
        .into_iter()
        .filter(|b| field_allowed(b.2))
        .collect()
}

fn field_allowed(c: Command) -> bool {
    matches!(
        c,
        Command::OpenDocument
            | Command::Save
            | Command::SaveAs
            | Command::PrintDocument
            | Command::CommandPalette
            | Command::ToggleForms
            | Command::Find
    )
}

pub fn consume(input: &mut egui::InputState, bindings: &Bindings, field: bool) -> Vec<Command> {
    let mut commands = Vec::new();
    for (m, key, command) in effective(bindings) {
        if field && (!(m.command || m.ctrl) || !field_allowed(command)) {
            continue;
        }
        let mut seen = false;
        input.events.retain(|event| {
            if let egui::Event::Key {
                key: k,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                // Exact modifiers prevent an extra Shift from triggering Save or another command.
                if *k == key
                    && (modifiers.shift == m.shift || key == Key::Plus)
                    && modifiers.alt == m.alt
                    && command_matches(*modifiers, m)
                {
                    seen = true;
                    return false;
                }
            }
            true
        });
        if seen {
            commands.push(command);
        }
    }
    commands
}

#[derive(Default)]
pub struct State {
    pub draft: Option<Draft>,
}

pub struct Draft {
    bindings: Bindings,
    selected: Command,
    filter: String,
    value: String,
    error: Option<String>,
}

impl Draft {
    pub fn new(bindings: &Bindings) -> Self {
        Self {
            bindings: bindings.clone(),
            selected: Command::OpenDocument,
            filter: String::new(),
            value: String::new(),
            error: None,
        }
    }
}

pub fn text_for(bindings: &Bindings, lang: sunipdf_i18n::Language, command: Command) -> String {
    if DEFAULTS.iter().any(|b| b.2 == command) {
        return effective(bindings)
            .into_iter()
            .filter(|b| b.2 == command)
            .map(|(m, k, _)| display(m, k))
            .collect::<Vec<_>>()
            .join(" / ");
    }
    command
        .info()
        .shortcut_key
        .map(|k| tr(lang, k))
        .unwrap_or("")
        .into()
}

impl crate::app::SuniApp {
    pub fn shortcut_text(&self, command: Command) -> String {
        text_for(&self.settings.shortcuts, self.lang(), command)
    }

    pub fn shortcut_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.shortcuts.draft.take() else {
            return;
        };
        let lang = self.lang();
        let mut save = false;
        let mut close = false;
        let valid = std::cell::Cell::new(resolve(&draft.bindings).is_ok());
        crate::dialog::show(
            ctx,
            crate::dialog::Spec::new(
                "keyboard_shortcuts",
                tr(lang, "shortcuts.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(640.0),
            |ui| {
                ui.label(tr(lang, "shortcuts.scope"));
                if resolve(&draft.bindings).is_err() {
                    ui.colored_label(crate::theme::ERROR, tr(lang, "shortcuts.invalid"));
                }
                let label = ui.label(tr(lang, "shortcuts.search"));
                ui.text_edit_singleline(&mut draft.filter)
                    .labelled_by(label.id);
                egui::ScrollArea::vertical()
                    .id_salt("command_shortcuts")
                    .max_height(170.0)
                    .show(ui, |ui| {
                        for info in sunipdf_core::REGISTRY {
                            if !DEFAULTS.iter().any(|b| b.2 == info.command)
                                || !tr(lang, info.label_key)
                                    .to_lowercase()
                                    .contains(&draft.filter.to_lowercase())
                            {
                                continue;
                            }
                            let shortcuts = effective(&draft.bindings)
                                .into_iter()
                                .filter(|b| b.2 == info.command)
                                .map(|(m, k, _)| display(m, k))
                                .collect::<Vec<_>>()
                                .join(" / ");
                            let shortcuts = if shortcuts.is_empty() {
                                tr(lang, "shortcuts.unassigned").into()
                            } else {
                                shortcuts
                            };
                            if ui
                                .selectable_label(
                                    draft.selected == info.command,
                                    format!("{} — {shortcuts}", tr(lang, info.label_key)),
                                )
                                .clicked()
                            {
                                draft.selected = info.command;
                                draft.value = draft
                                    .bindings
                                    .get(info.id)
                                    .and_then(|v| v.clone())
                                    .unwrap_or_default();
                                draft.error = None;
                            }
                        }
                    });
                ui.separator();
                ui.strong(tr(lang, draft.selected.info().label_key));
                let label = ui.label(tr(lang, "shortcuts.binding"));
                ui.text_edit_singleline(&mut draft.value)
                    .labelled_by(label.id);
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr(lang, "shortcuts.assign")).clicked() {
                        let result = parse(&draft.value).and_then(|(m, k)| {
                            let mut proposed = draft.bindings.clone();
                            proposed.insert(draft.selected.info().id.into(), Some(text(m, k)));
                            resolve(&proposed)?;
                            Ok(proposed)
                        });
                        match result {
                            Ok(bindings) => {
                                draft.bindings = bindings;
                                draft.error = None;
                            }
                            Err(error) => {
                                draft.error = Some(match error {
                                    Invalid::Conflict(c) => format!(
                                        "{}: {}",
                                        tr(lang, "shortcuts.conflict"),
                                        tr(lang, c.info().label_key)
                                    ),
                                    Invalid::Reserved => tr(lang, "shortcuts.reserved").into(),
                                    Invalid::Syntax => tr(lang, "shortcuts.syntax").into(),
                                    Invalid::Stored => tr(lang, "shortcuts.invalid").into(),
                                });
                            }
                        }
                    }
                    if ui.button(tr(lang, "shortcuts.disable")).clicked() {
                        draft.bindings.insert(draft.selected.info().id.into(), None);
                        draft.error = None;
                    }
                    if ui.button(tr(lang, "shortcuts.reset")).clicked() {
                        draft.bindings.remove(draft.selected.info().id);
                        draft.error = None;
                    }
                });
                if ui.button(tr(lang, "shortcuts.reset_all")).clicked() {
                    draft.bindings.clear();
                    draft.error = None;
                }
                valid.set(resolve(&draft.bindings).is_ok());
                if let Some(error) = &draft.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
            },
            |ui| {
                ui.add_enabled_ui(valid.get(), |ui| {
                    save = crate::dialog::primary(ui, tr(lang, "dialog.unsaved.save"));
                });
                close = ui.button(tr(lang, "dialog.unsaved.cancel")).clicked()
                    || ui.input(|i| i.key_pressed(Key::Escape));
            },
        );
        if save && !close && resolve(&draft.bindings).is_ok() {
            let mut proposed = self.settings.clone();
            proposed.shortcuts = draft.bindings.clone();
            if proposed.save().is_ok() {
                self.settings = proposed;
                close = true;
            } else {
                draft.error = Some(tr(lang, "shortcuts.save_failed").into());
            }
        }
        if !close {
            self.shortcuts.draft = Some(draft);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plus_shift_layouts_and_custom_alias_collisions_are_consistent() {
        let b = Bindings::new();
        for m in [Modifiers::COMMAND, Modifiers::COMMAND | Modifiers::SHIFT] {
            assert_eq!(press(&b, Key::Plus, m, false), vec![Command::ZoomIn]);
        }
        let b = Bindings::from([(
            Command::OpenDocument.info().id.into(),
            Some("Ctrl+Plus".into()),
        )]);
        assert!(matches!(
            resolve(&b),
            Err(Invalid::Conflict(Command::ZoomIn))
        ));
        assert_eq!(parse("ctrl+shift+plus"), parse("Ctrl+Plus"));
    }

    #[test]
    fn shortcut_modal_escape_discards_draft_and_does_not_write_preferences() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        let original = app.settings.clone();
        app.execute(&ctx, Command::KeyboardShortcuts);
        assert!(app.dialog_open());
        assert!(!app.palette.open);
        app.shortcuts
            .draft
            .as_mut()
            .unwrap()
            .bindings
            .insert(Command::Save.info().id.into(), Some("Ctrl+J".into()));
        for language in sunipdf_i18n::Language::ALL {
            app.settings.set_language(language);
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                app.shortcut_dialog(ui.ctx())
            });
            out.textures_delta.clear();
            assert!(app.dialog_open());
        }
        app.settings = original.clone();
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| app.shortcut_dialog(ui.ctx()),
        );
        out.textures_delta.clear();
        assert!(!app.dialog_open());
        assert_eq!(app.settings, original);
        assert!(requests.try_iter().next().is_none());
    }

    #[test]
    fn displayed_shortcuts_follow_overrides_disabled_commands_and_invalid_fallback() {
        let (mut app, _, _) = crate::app::tests::test_app();
        for language in sunipdf_i18n::Language::ALL {
            app.settings.set_language(language);
            app.settings
                .shortcuts
                .insert(Command::Save.info().id.into(), Some("Ctrl+J".into()));
            assert_eq!(
                app.shortcut_text(Command::Save),
                if cfg!(target_os = "macos") {
                    "⌘J"
                } else {
                    "Ctrl+J"
                }
            );
            app.settings
                .shortcuts
                .insert(Command::Save.info().id.into(), None);
            assert_eq!(app.shortcut_text(Command::Save), "");
            app.settings
                .shortcuts
                .insert("unknown.command".into(), None);
            assert_eq!(
                app.shortcut_text(Command::Save),
                if cfg!(target_os = "macos") {
                    "⌘S"
                } else {
                    "Ctrl+S"
                }
            );
            app.settings.shortcuts.clear();
        }
    }

    fn press(bindings: &Bindings, key: Key, modifiers: Modifiers, field: bool) -> Vec<Command> {
        let ctx = egui::Context::default();
        let mut commands = vec![];
        let mut out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                ..Default::default()
            },
            |ui| {
                commands = ui.ctx().input_mut(|i| consume(i, bindings, field));
            },
        );
        out.textures_delta.clear();
        commands
    }

    #[test]
    fn override_replaces_all_default_aliases_and_can_be_disabled_or_reset() {
        let mut b = Bindings::new();
        b.insert(
            Command::ZoomIn.info().id.into(),
            Some("Ctrl+Shift+J".into()),
        );
        assert_eq!(
            press(&b, Key::J, Modifiers::COMMAND | Modifiers::SHIFT, false),
            vec![Command::ZoomIn]
        );
        assert!(press(&b, Key::Plus, Modifiers::COMMAND, false).is_empty());
        assert!(press(&b, Key::Equals, Modifiers::COMMAND, false).is_empty());
        b.insert(Command::ZoomIn.info().id.into(), None);
        assert!(press(&b, Key::J, Modifiers::COMMAND | Modifiers::SHIFT, false).is_empty());
        b.clear();
        assert_eq!(
            press(&b, Key::Equals, Modifiers::COMMAND, false),
            vec![Command::ZoomIn]
        );
    }

    #[test]
    fn collisions_reserved_keys_and_invalid_stored_settings_fail_closed() {
        for value in [
            "Ctrl+S",
            "Ctrl+Shift+S",
            "Ctrl+C",
            "Ctrl+Shift+Delete",
            "Alt+F4",
            "F11",
            "Ctrl+Escape",
            "Ctrl+J+K",
        ] {
            let b = Bindings::from([(Command::OpenDocument.info().id.into(), Some(value.into()))]);
            assert!(resolve(&b).is_err(), "{value}");
            assert_eq!(
                press(&b, Key::O, Modifiers::COMMAND, false),
                vec![Command::OpenDocument]
            );
        }
        let b = Bindings::from([("unknown.command".into(), None)]);
        assert!(resolve(&b).is_err());
        let b = Bindings::from([
            (
                Command::OpenDocument.info().id.into(),
                Some("Ctrl+J".into()),
            ),
            (Command::Save.info().id.into(), Some("ctrl+j".into())),
        ]);
        assert!(matches!(resolve(&b), Err(Invalid::Conflict(_))));
    }

    #[test]
    fn exact_shift_and_input_focus_preserve_typing_and_text_undo() {
        let mut b = Bindings::new();
        assert_eq!(
            press(&b, Key::S, Modifiers::COMMAND | Modifiers::SHIFT, true),
            vec![Command::SaveAs]
        );
        assert!(press(&b, Key::J, Modifiers::COMMAND | Modifiers::SHIFT, false).is_empty());
        b.insert(Command::Save.info().id.into(), Some("Ctrl+J".into()));
        assert_eq!(
            press(&b, Key::J, Modifiers::COMMAND, true),
            vec![Command::Save]
        );
        assert!(press(&b, Key::J, Modifiers::COMMAND | Modifiers::SHIFT, true).is_empty());
        b.insert(Command::Undo.info().id.into(), Some("Ctrl+Shift+U".into()));
        assert!(press(&b, Key::U, Modifiers::COMMAND | Modifiers::SHIFT, true).is_empty());
        assert!(press(&b, Key::T, Modifiers::NONE, true).is_empty());
    }

    #[test]
    fn settings_roundtrip_preserves_disabled_and_rebound_commands_and_old_preferences() {
        let mut s = crate::settings::Settings::default();
        s.shortcuts
            .insert(Command::Save.info().id.into(), Some("Ctrl+J".into()));
        s.shortcuts.insert(Command::ToolInk.info().id.into(), None);
        let saved = serde_json::to_vec(&s).unwrap();
        let back: crate::settings::Settings = serde_json::from_slice(&saved).unwrap();
        assert_eq!(s, back);
        assert!(resolve(&back.shortcuts).is_ok());
        let old: crate::settings::Settings =
            serde_json::from_str(r#"{"dark_theme":true,"language":"fr-FR"}"#).unwrap();
        assert!(old.dark_theme && old.shortcuts.is_empty());
        assert_eq!(old.language(), sunipdf_i18n::Language::French);
    }
}
