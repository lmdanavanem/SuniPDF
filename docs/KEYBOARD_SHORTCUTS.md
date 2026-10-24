# Keyboard shortcuts

Open **Help → Keyboard shortcuts** (also available in the command palette). Filter the list, select a command, enter a combination such as `Ctrl+Shift+J`, and choose **Assign**. **Save** applies the entire draft and writes preferences atomically. **Cancel** or Escape discards it.

**Disable** removes all shortcuts for that command, including aliases. **Restore command default** restores its original bindings; **Restore all defaults** clears every override. Conflicts are refused; disable or reassign the other command first. Restoring a default that conflicts with an override leaves Save unavailable until the conflict is resolved. Menus, the command palette, home screen and command tooltips show the effective shortcuts.

This first version customizes commands that already have entries in the desktop shortcut table. It does not add arbitrary macros or new command bindings to previously unbound commands. One override replaces all default aliases for its command.

Custom combinations use `Ctrl`, optional `Shift`, and a letter, digit, F2–F12, Tab, Delete, Plus, Equals, Minus, OpenBracket or CloseBracket. Clipboard/select-all, text Undo/Redo, Ctrl+F4/F6, and Ctrl+Shift+Delete are reserved. Navigation, Escape, F5 view toggling, F11 fullscreen, Alt+Left/Right history, object nudging and Windows keys keep their existing dedicated behavior. `Plus` accepts keyboard layouts that report Shift as part of the plus key. Alt/AltGr combinations are not captured.

When a text widget has focus, only Open, Save, Save As, Print, Find, Forms and the command palette can use their Ctrl shortcuts. Text Undo/Redo and ordinary typing stay with the widget. Other dialogs suspend command shortcuts. The palette accepts only its own close/toggle command while focused.

Overrides use stable command IDs in `%APPDATA%\SuniPDF\settings.json`, under `shortcuts`. A value is the combination string; `null` disables the command. Old preferences without this field retain defaults. Unknown IDs, invalid combinations and collisions make the entire override set inactive; the app uses defaults and the editor asks you to restore them. A failed preferences write leaves the active settings unchanged.

Headless egui input and dialog tests exercise dispatch, conflict/refusal, focus, displayed bindings, cancellation and serialization. They do not qualify a native Windows keyboard layout, Narrator or physical input device. Explorer integration remains separate work.
