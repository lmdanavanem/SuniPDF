# Reader controls and tools
[User guide](USER_GUIDE.en.md)

The 0.60 plan workspace adds resizable docked/floating tables, cached page layouts and measurement scale tools. This page retains the reading controls introduced in 0.50. [0.60](PLAN_WORKSPACE.md).

Available in [0.50.0](RELEASE_0.50.0.md). [Evidence](qa/RELEASE_0.50.0.json).

## Find a tool

Choose **Tools** in the main toolbar, or press **F9**. The left dock switches between **Tools**, **Pages** and **Bookmarks**, showing one panel at a time. Its close button hides the dock; use the toolbar or navigation commands to reopen it.

The tools panel groups 18 tools into five families:

| Family | Tools |
|---|---|
| Review & measure | Add comments, Measurements, Quantity table, Annotation library, Revision overlay, Add a stamp |
| Edit & organize | Edit a PDF, Organize pages, Redact a PDF |
| Forms & signatures | Fill & Sign, Prepare a form, Use a certificate |
| Create & convert | Create a PDF, Combine files, Export a PDF, Scan & OCR, Compress a PDF |
| Security | Protect a PDF |

Use **Search tools** to filter by the current language's tool name, description or family. All words in the search must match. **Clear search** restores the list. A tool that needs a PDF asks you to open one when necessary. The command palette remains available with **Ctrl+K**.

Open measurements through **Tools → Review & measure → Measurements**. The same family contains **Annotation library**, **Quantity table** and **Revision overlay**. Measurements and the library no longer have separate buttons in the main toolbar.

## Measure with the side panel

Selecting **Measurements** opens its settings on the right, beside the PDF. Four sections group the controls:

- **Create measurement**: choose the type, unit and decimal places.
- **Page scale**: calibrate the current page and manage its session or saved scale.
- **Drawing**: read the current instructions, undo a point, preview or cancel a draft.
- **Saved data**: inspect saved measurements and open the quantity table.

Scroll the panel when necessary. Measurement controls occupy this side panel rather than an extra horizontal workspace bar. Choose **Saved measurements** under **Saved data** to inspect or edit a supported saved record. See [calibrated measurements](MEASUREMENTS.md) and [saved geometry editing](MEASUREMENT_GEOMETRY.md) for calculation and editing limits.

The right side shows one context panel at a time, including Comments, Objects, forms and measurement settings. While a measurement draft is active, opening another context panel is blocked with guidance to finish or cancel the draft. Explicitly leaving the measurement tool or changing tools cancels its local draft; confirmed page scales are preserved.

## Page and zoom controls

The reading controls are below the PDF. Use the previous/next buttons or enter a page number or supported page label and press **Enter**. The total page count appears beside the field.

Use **Zoom Out**, the zoom list or **Zoom In**. The percentage, including an intermediate value such as **149%**, stays on one line. The list provides presets from 25% to 1600%; **Page** and **Width** fit the displayed page or spread. Existing keyboard and mouse-wheel zoom controls remain available.

**Reading options** contains Single Page, Continuous Scrolling, Two Pages, Book View, clockwise/counterclockwise view rotation and Full-screen reading. Wider layouts also show direct clockwise-rotation and full-screen buttons. In a narrower window, whole control groups move to another row and the options menu retains these commands.

View rotation changes reading orientation only. To change the PDF's page rotation, use **Organize pages** and save the document. The right icon rail opens Pages, Bookmarks, Comments, Objects and Find; page navigation and zoom use the bottom controls. The status area shows readiness or operation messages.

## Scope

The 0.50 release reorganized reading, navigation and measurement operations. The [0.60 workspace](PLAN_WORKSPACE.md) adds eligible vector snapping, scale regions and batch recalibration. PDF permissions and draft/preview guards still apply.

Offscreen rendering and automated interaction checks can verify the tested layouts. They do not establish native-window, physical-device, screen-reader or ARM64 hardware qualification. Publication evidence records which checks actually ran.
