# Accessibility: actual state

SuniPDF exposes its interface to assistive technologies through AccessKit (egui). This document describes what is
named and keyboard-testable, and what is not. **Manual verification with Windows Narrator: NOT
DONE** (no test session with a screen reader has been run on this machine).

## Common principle

- Every hand-drawn button carries a readable AccessKit label (`widget_info`), never a glyph from the icon
  font (`workspace::bar_button`, `shell::tool_row`).
- Single-letter shortcuts are cut off as soon as an input field has the keyboard; Escape follows a single order
  (`workspace::EscState`).
- Bit 10 (extraction for accessibility) is always granted by Protect (D050).

## AcroForm forms (forms slice)

| Control | Accessible name (key) | Keyboard |
|---|---|---|
| Field rectangle on the page (Fill tool) | `a11y.forms.widget` ("Field {name}, type {kind}") or `a11y.forms.widget_value` | Click only; no activation by Space / Enter (roadmap) |
| Editor placed on a text field | `a11y.forms.editor` ("Editing field {name}") | Enter confirms (Ctrl+Enter in multiline), Escape cancels, Tab / Shift+Tab confirm and move to the next / previous text or list field (content selected) |
| Floating choice list | `a11y.forms.option` ("{name}: {option}") per entry | Click, Escape closes |
| "Form fields" panel (Ctrl+Shift+F) | title `forms.panel.title`, one native egui control per field (text box, checkbox `a11y.forms.checked` / `unchecked`, radio buttons, drop-down list, list box boxes), each attached to the field label by `labelled_by` | Tab between controls, Space, arrows, Enter; Escape restores the input of a text box |
| Banners (form present, XFA, scripts), Flatten dialog | translated texts, `dialog::show` (Enter / Escape) | Tab |

Limits: the page is an image (texture); its text is not exposed to screen readers, only the field rectangles
and the panel are. Reading the tagged structure (Tagged PDF): out of scope. Contrast of
highlights: `theme::BLUE` / `theme::ERROR` at 12% fill and 100% outline (22% and 1.5 px on hover),
verified on captures in light and dark themes (no automatic ratio measurement for these outlines).
