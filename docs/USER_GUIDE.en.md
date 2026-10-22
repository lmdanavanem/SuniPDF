# SuniPDF — User Guide

SuniPDF is a PDF reader and editor for Windows 10/11. PDF processing stays local: no account, no telemetry,
no document ever leaves your computer.

This guide covers [0.46.0](RELEASE_0.46.0.md). The preceding [0.45.0](RELEASE_0.45.0.md) retains its own evidence. Executed checks and limits are recorded in [Project status](PROJECT_STATUS.md). English remains the default language; French is available in Settings.

## Quantity tables and typed fields

Available in [0.46.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.46.0). [Evidence](qa/RELEASE_0.46.0.json).

1. Open a PDF, then choose **Quantity table** from **All tools**, the measurement controls or the annotation register. Filter the rows and choose **Subtotal groups**; totals cover all matching rows, including other table pages.
2. Choose **Edit row** to enter **Unit price**, **Currency code**, **Price per unit** and **Cost decimal places**. A blank price is missing; `0` is an explicit zero. Counts and saved measurements supply the quantities; ordinary annotations do not acquire an inferred quantity, and angles are not priced.
3. Use **Add custom column** for text, number, date or choice values. Choose **Apply fields** or **Cancel edit**. Each applied row is one Undo step; close the table before using Undo/Redo, and save the PDF to retain the fields.
4. Use **Export CSV** or **Export JSON** and a new filename. Exports include all matching rows and source provenance; copying permission is required. Quantity totals keep units separate, monetary totals keep currency/rounding groups separate, and partial coverage remains visible.

[Full instructions, limits and CLI](QUANTITY_TABLE.md).

## Reuse local annotation templates

The 0.45 annotation library is available within its [qualified scope](RELEASE_0.45.0.md). Click **Library** in the main toolbar, or choose
**Annotation library** in **All tools** or **Add comments**. Its local file is separate from PDF files and settings.

Choose **Create a template**, prepare a supported new annotation draft, then expand **Save to library**.
Enter a unique name and choose **Save template**. Credit, source and licence are optional user declarations;
blank means unknown. Saving a template does not insert it in the PDF. Capture supports text with standard
fonts, polygons/polylines, revision clouds, predefined stamps and PNG image stamps.

Choose **Place on page**, set the scale and click the PDF. Inspect the rendered preview; adjust **Origin X (pt)**,
**Origin Y (pt)** or **Scale** if needed, then request **Preview** again. Choose **Apply to object** to create the
annotations, or **Cancel** to discard the placement. A group is applied as one Undo step; save the PDF to retain it.
To make a group, check at least two entries, choose **Create group**, enter a new name and member offsets, then save.

Import/export, selected-entry deletion, reload and filtering are available in the library. Invalid imports and
conflicting writes preserve its active contents. Library-file changes have no PDF Undo history; export a copy before
deleting templates you want to keep. Measurements, page scales, existing PDF identities and review threads are excluded.
[Full instructions, supported format, bounds and CLI examples](ANNOTATION_LIBRARY.md).

## Measure directly on a PDF

Open a PDF and click **Measurements** beside **All tools** in the main toolbar. You can also choose
**All tools → Measurements**, or the Measurements shortcut in **Add comments**. The button keeps its
text label in narrow windows.

1. Select the unit (`mm`, `cm`, `m`, `in` or `ft`) and 0–4 decimal places. Click **Calibrate this page**,
   then click two different points spanning a known distance. Enter the real **Known distance** and
   click **Confirm scale**. Decimal feet and inches are supported: 10 feet 6 inches is `10.5 ft` or `126 in`.
2. Choose **Length**, **Polyline length**, **Polygon perimeter**, **Polygon area** or **Angle**, then
   click the points directly on the PDF. Length opens the preview after two points. Angle uses three
   points in this order: first arm, vertex, second arm; it does not require a physical calibration.
   Finish a polyline or polygon with **Preview measurement**, **Enter**, or a double-click on its last
   point. Polygons close automatically; do not repeat the first point.
3. Inspect the rendered PDF preview and the value, then click **Apply**. **Cancel** or **Escape** leaves
   the PDF unchanged. Save to keep the annotation and its calibration. **Undo** removes the annotation
   and its measurement data together. The tool stays active for the next measurement.

During drawing, **Backspace** or **Undo point** removes the last point. **Shift** constrains a segment
horizontally or vertically in the displayed page axes; it does not snap to existing PDF lines.
**Escape** or **Cancel drawing** discards the draft. Finish or cancel before changing page, document or tool.

Each page has its own session scale. Ordinary annotation edits and normal tab switches retain confirmed
scales; changes to page geometry, structure or content clear them. A session scale takes priority over the page's
saved default. Changing units converts the confirmed distance. Saved measurements keep their own
geometry and scale; recalibrating new measurements does not change existing ones.

To keep a scale across closing/reopening, click **Save scale in PDF** after calibration, then save the
PDF file. The saved default is reused when valid. **Remove saved scale** removes that page's saved
default, while retaining any explicitly chosen session scale; save the file to retain the removal.
**Undo**/**Redo** restores the document state and reloads its saved default; it does not restore a cleared session scale. Saving/removing a default
requires document-modification permission; measurement creation requires annotation permission.
Rotation and page reordering preserve eligible defaults. Changed crop bounds or decoded page content,
unsupported records and non-default `UserUnit` cause a refusal; recalibrate or remove a supported stale
default. Unsaved session scales are lost on closing. These checks do not authenticate every plan revision.

Click **Saved measurements** in the measurement toolbar to inspect the current page. Reading the list leaves the PDF unchanged. It
shows value/unit, type, annotation index, optional identity, vertex count, precision and known reference
distance for non-angle measurements, in batches of 50. **View on page** selects the actual annotation and opens
its comments. **Use this scale** copies a non-angle annotation's calibration to the page's session for
new measurements, without changing existing records. Reference points outside the page are refused.
Use **Save scale in PDF** separately to keep this default. **Refresh** rereads the current state; close
the inspector before drawing or changing documents.

The CLI exposes `page-calibration`, `set-page-calibration` and `remove-page-calibration`.
`measure --use-page-calibration` uses a valid saved default and converts it to `--unit`; it excludes
manual `--calibration`/`--known-distance`. Writes always require a new output file, even with `--overwrite`.
Use password environment/stdin options for protected files. [Examples and bounds](MEASUREMENTS.md#cli-and-quantities).

In 0.44, choose **Edit measurement** in the inspector to change that annotation's known reference
distance, unit or precision. Unit changes preserve physical size; reference-distance changes recalculate
only this measurement. Angles expose precision only. Request a **Preview**, inspect the rendered
candidate, then **Apply**; **Cancel** changes nothing. One Undo/Redo step restores the previous/replacement
measurement, including its identity and linked comments. Save the PDF to retain the edit. Measured
vertices and reference points remain fixed in this desktop editor; changed or unsupported targets are refused.

**Use saved page scale** discards the temporary override when a valid saved default exists. Otherwise
**Clear temporary scale** discards only the page's temporary scale. Neither control modifies the PDF.
The manual rich-annotation path now requires an explicit calibration instead of supplying a positive default.

The 0.44 CLI adds `edit-measurement`: supply a fresh `--expected-record` from `measurements`, the physical
`--page`, the record's annotation `--index` and a new `--out`. The CLI can replace explicit vertices;
see the measurement guide for calibration and identity requirements. Export requires copying permission.
Permitted creation/editing without it returns `measurement: null` and `export_unavailable: true`.
Resizing content and annotations on measured pages is refused before changes are committed.

Global recalculation, saved-vertex dragging, vector snapping and multiple-scale viewports remain unavailable. Check the known distance and drawing revision; a
calculated value does not establish physical accuracy. See the [measurement guide](MEASUREMENTS.md)
for the open metadata format, permissions and geometry limits.

## Prepare a visual signature

In **Fill & Sign**, optionally enable **Smooth strokes** for a drawn signature or **Reduce paper shadows**
after enabling background removal for an imported photo. Compare the preview before placing it.
Original strokes and source image files are retained. These are visual marks; certificate signing is separate.
The CLI can export a separate PNG with `prepare-signature-image`.
[Instructions, drawing limits and cleanup bounds](VISUAL_SIGNATURE_PREPARATION.md).

## Inspect a saved PDF

Choose **File → Properties** (`Ctrl+D`), expand **Inspect saved PDF**, and choose **Inspect**.
Save edits first; if the file has changed outside the app, reload it. The report displays the saved
revision's SHA256 and declarations of JavaScript, automatic/external actions, embedded files, XFA,
multimedia and signatures. Counts refer to matching dictionaries, including unused objects.
No action is run and no document is changed. An incomplete report retains partial counts explicitly.
Metadata drafts in Properties are applied only by **Apply**, independently of inspection.

For automation, use `sunipdf-cli inspect document.pdf` or provide `--password-env NAME` / `--password-stdin`.
JSON exit `0` means the bounded inventory completed; exit `2` means partial results. Findings do not
establish file safety or PDF/A/PDF/UA compliance and do not verify signature integrity. Payloads are not
inspected. The input limit is 128 MiB; object/traversal limits can also produce a refusal or partial result.

## New editing and document tools

- **Edit → Rich annotations** opens guarded controls for embedded-font FreeText, polygons,
  polylines, clouds, stamps and attachments. Preview before applying. **Erase ink strokes** removes
  whole touched strokes; cancel keeps the PDF unchanged.
- Paragraph editing previews wrapping with the original PDF font. Unsupported glyphs or graphics
  states keep the document and draft intact. **Paste editable** imports individual supported PDF
  objects; the existing grouped paste remains available.
- Link target editing supports existing named destinations and Fit/XYZ/FitH. Blank coordinates or
  zoom keep the reader's current values; coordinates use the target page as displayed. A FitH
  position on a rotated page can use X instead of Y. Unsafe external actions are refused.
- The bookmark panel can add, rename, retarget, move and delete bounded bookmark trees. Page labels
  follow page content during reorganisation; `#N` is always a physical page number.
- The redaction confirmation offers before/after preview, fill and optional replacement text.
  **Save marks** stores visible, unapplied `/Redact` annotations: sensitive content remains present
  until redaction is applied. Saving marks currently requires an empty overlay. Apply uses the exact
  prepared preview and one Undo. Native text outside marks remains selectable only when its safe
  reconstruction can be proved; the report identifies bitmap fallback.
- **Sanitize** writes a separate cleaned copy and verifies supported hidden-data removal. Keep track
  of previous copies, recovery files, backups and Undo separately.
- Conversion tools detect an already-installed supported local LibreOffice. If unavailable, the app
  says so. Standard/Print profiles and bounded cancellable batches use new-copy output. PDF exports
  offer simple editable DOCX text, aligned XLSX string cells and raster PPTX slides, with explicit
  limits in [Office conversion](OFFICE_CONVERSION.md).
- Optional saved signature images use current-user Windows DPAPI. Preserve the original image or
  the Windows profile and DPAPI keys for recovery. Digital-signature verification separately reports
  integrity, Windows trust, revocation and RFC3161 timestamps. Online checks require explicit consent;
  offline/unavailable states are not successful verification.

Examples (pages and bookmark numbers start at 1; annotation indices start at 0):

```powershell
sunipdf-cli bookmark-add input.pdf --title "Chapter 1" --page 2 --out bookmarked.pdf
sunipdf-cli link-destinations input.pdf
sunipdf-cli set-link-target input.pdf --index 0 --goto-page 2 --view xyz --y 300 --zoom 1.5 --out linked.pdf
sunipdf-cli redact input.pdf --text SECRET --color 000000 --out redacted.pdf
sunipdf-cli redact input.pdf --text SECRET --marks-only --out unapplied-marks.pdf
sunipdf-cli sanitize input.pdf --out sanitized.pdf
```

See [rich annotations](RICH_ANNOTATIONS.md), [redaction](REDACTION.md),
[bookmarks and structures](BOOKMARKS_AND_PAGE_STRUCTURE.md), [font embedding](FONT_EMBEDDING.md),
[signature security](SIGNATURE_SECURITY.md) and [sanitization](SECURITY_MODEL.md) for exact bounds.

## Installation

- **Portable**: unzip `SuniPDF-<version>-win-x64-portable.zip` anywhere and run `sunipdf.exe`. Nothing is
  installed system-wide. Preferences use `%APPDATA%\SuniPDF\settings.json`; update downloads use a temporary folder.
- **MSI installer**: double-click `SuniPDF-<version>-win-x64.msi`. SuniPDF appears in the Start menu and in
  "Open with" for PDF files; it does not become the default reader without your consent.
- Uninstall: Windows Settings › Apps, or delete the folder for the portable version.

## Updates and language

English is the default interface language. French is available in Settings; an existing language preference is preserved.
From 0.8.2, the application checks GitHub Releases at startup without sending PDF files. Disable automatic checks in About,
or use **Check for updates** there at any time. A failed automatic check does not block local PDF work.

When a newer version is available, choose **Download update**. After size and SHA256 verification, choose **Close and install**.
Save or discard changes in each open document as usual; cancelling or a failed save cancels installation.
Installed x64 copies use the Windows MSI installer and may show a Windows administrator prompt. Portable x64/ARM64 copies
replace the application files in their own folder and keep a rollback backup. A failed portable replacement restores the old files.
Older versions require one manual upgrade to obtain this feature. The updater never restarts the computer automatically.

## Home screen

The home screen shows a large **Open file** card, one card per available tool (the whole card is clickable)
and your recent files. You can also drop a PDF anywhere in the window, or click **+ Open** in the tab strip.

## All tools

The **All tools** panel on the left (F9, or the button of the same name in the toolbar) lists the tools by
task: Export a PDF, Fill & Sign, Edit a PDF, Create a PDF, Combine files, Organize pages, Measurements, Annotation library, Add
comments, Protect a PDF, Redact a PDF, Compress a PDF, Add a stamp, Scan & OCR, Prepare a form and
Use a certificate. The **Tools** menu and the Ctrl+K palette give the same access.
Without an open document, a tool that needs one first asks for a file, then starts.

- **Export, Create, Combine, Compress, Annotation library** open their dialog directly. The library can be managed without an open PDF.
- **Fill & Sign, Edit, Organize, Measurements, Add comments, Protect, Add a stamp** (marked with a › chevron) show a **tool bar** under
  the toolbar: the tool's buttons, grouped, with their labels when the width allows (otherwise the icon only,
  the label staying in the tooltip). Actions that need a selection (object, text, grid pages) stay greyed
  out without one. Measurements keeps its label visible and exposes its calibration and drawing controls.
- **Leave tool**: button on the right of the bar, **Esc**, clicking the active tool again in the panel, or the
  back arrow (which also reopens the panel). Esc first closes what is closer: an open menu or drop-down
  list, then the search bar, then the selected object, then the tool (one Esc, one effect).
  An active measurement draft is cancelled before leaving its tool.
- Single-letter shortcuts open the matching tool: H, U, X, N, A, D, F open **Add comments**; E, T, I, R, O, L open
  **Edit a PDF**; S opens the **Sign** dialog; F8 opens **Organize pages** with the grid; V goes back to the reading cursor.
- Each tab keeps its own active tool.
- The main toolbar keeps quick access to Open, Save, Undo, Redo, Measurements and Library.

The 0.45 interface adds **Annotation library** alongside **Measurements**, **Scan & OCR**, **Prepare a form** and **Use a certificate**.
Fill & Sign continues to provide form filling and visual signatures; certificate signatures use their own tool.

## Tabs

Each open PDF gets a tab in the top strip. Click a tab to show it, its cross to close it, the home button to
go back to the home screen without closing anything. Ctrl+Tab and Ctrl+Shift+Tab cycle through tabs.
Unsaved changes are flagged with a dot per tab and confirmed one by one when you quit.

## Automatic recovery

Automatic recovery is enabled by default. A modified document gets a recovery snapshot every 60 seconds
and when its tab is suspended. This does not save over the original PDF. If the application stops unexpectedly,
the next startup offers the available snapshots; **Recover documents** also opens the list later.
Choose **Restore**, then **Save as** to keep the recovered work in a new PDF. A protected snapshot requires
its PDF password again: encryption is preserved and passwords are never written to recovery metadata.
The recovery folder is `%LOCALAPPDATA%\SuniPDF\recovery`; metadata includes the original path and page count.
Explicitly discarding changes or saving successfully removes the relevant recovery copy. Recovery covers
completed document edits and valid form drafts, not text still being typed in an unconfirmed dialog.
Disable automatic snapshots with the checkbox in About. **File → Recover documents** reopens the list.

## Reading

By default pages are **stacked**: the mouse wheel, the scroll bar or the keyboard scroll through the whole
document, and the page that fills the screen becomes the current page (right-rail number, status bar,
thumbnail, comments panel; Rotate, Delete and Extract act on it; clicking a page makes it current). **F5**, the
arrows button of the right rail or the View menu switch between **Continuous Scrolling** and **Single Page**;
the choice is remembered. Only visible pages are rendered: a document of several hundred pages opens and
scrolls without rendering every page. The View menu also offers **Two pages** (pairs from the first page) and
**Book view** (cover alone, then pairs). An unpaired last page is centred. Mixed page sizes share one zoom;
fit width/page uses the widest spread and tallest page in spread views. These modes use the same tile and texture caps.

| Action | How |
|---|---|
| Scroll | Mouse wheel; ↑ ↓ arrows (one line); Space / Shift+Space and Page Down / Page Up (one screen; in single-page view, Page Down / Up change page) |
| Next / previous page | → ← arrows, right-rail buttons |
| First / last page | Home / End |
| Go to page | Type the number in the right-rail box, then Enter, or click a thumbnail: the page comes to the top of the screen |
| Zoom | Ctrl + mouse wheel (zooms **around the pointer**; touchpad pinch takes the same path, untested), Ctrl + / Ctrl − (around the centre), Ctrl 0 (actual size), zoom list (25 to 1600%), "Width", "Page"; up to 3200% with the wheel, staying sharp (tiled rendering) |
| Continuous / single page | F5, right-rail button, View menu |
| Thumbnails | **Pages** panel on the left (grid button on the right rail); drag a thumbnail to reorder |
| Bookmarks | F6 or the list button on the right rail |
| Find | Ctrl+F: type, match case, ▲ ▼, matches highlighted |
| Select text | With the reading cursor (V), drag along text, including across pages in either direction; Ctrl+C copies. Double-click selects a word. Drag near a top/bottom edge to scroll automatically |
| Light / dark theme | Moon / sun icon, top right |
| Language | Globe icon, top right (French, English) |

## Annotating

Reading selection follows the PDFium logical character sequence, normalises line endings and keeps word
apostrophes such as `l'amour` and `l’amour`. It does not infer the intended order of arbitrary untagged columns.
Highlight, underline and strikethrough keep their rectangular selection gestures.

The **Add comments** tool brings two families together. Text markup: select text (V), highlight (H), underline (U),
strikethrough (X), note (N); drag a rectangle over text to apply a markup, click the page with the note tool to type a
comment. **Advanced annotations**, real PDF annotations (not page content: other readers show them too):

| Tool | Gesture |
|---|---|
| **Select an annotation** (A) | Click an annotation: frame, handles, floating bar. Drag = move; handles = resize; arrow keys = nudge (Shift: ×10); Delete = remove; double-click or the "Edit the text" button = change its text or comment; Esc = deselect |
| **Freehand drawing** (D) | Draw several strokes on one page; colour, width and opacity apply to the group. Finish drawing = one annotation/Undo; Cancel drawing or Esc before submission = discard the draft; a click = a dot |
| **Free text box** (F) | Click (default box) or drag (chosen box), then type in the anchored multiline page editor. Set size, colour, background and border; choose Add or Cancel. To change existing free text, select it then Edit Text/double-click, and Save or Cancel. The box grows downwards if needed. Helvetica WinAnsi appearance; exact text retained |
| **Rectangle, Ellipse, Line or arrow** (annotations) | Drag; colour, width, fill (rectangle and ellipse), opacity, "Arrow" checkbox |
| **Link** | Draw an area (at least 6 points) then choose "Go to a page" or "Open a web address" (`http`, `https` or `mailto` only) |

**On-page free text — from 0.20.0.** With **Free text box** (F), click the page for a default box or
drag its rectangle, then type directly in the anchored multiline editor. Adjust size, text colour,
background and border with the existing controls (size 6–96 pt, border 0–10 pt). Choose **Add** to create the annotation. To edit one,
select the FreeText annotation and choose **Edit Text**, or double-click it, then choose **Save**.
**Cancel** or Escape discards the draft without changing the PDF; clicking outside or pressing Enter does not submit it.

Confirm or cancel before conflicting page/navigation/document/tab/tool/workspace actions. Zoom keeps the editor bound to
its original page; document revision/tab/tool/workspace changes cancel stale editing. An operation already
opening/saving/mutating the document or another active draft can prevent editing from starting. Validation
or worker failure retains typed text and style for retry. Text must be nonempty, with a finite
rectangle at least twice the font size wide and at least 2 pt high. A successful Add/Save is one Undo step; save the PDF explicitly
afterwards. Unsubmitted editor text stays in memory and is absent from crash recovery.

The box can grow downwards for wrapped text. Rendering still uses Helvetica WinAnsi (Latin accents, € and
common punctuation); unsupported characters appear as "?", while `/Contents` retains the exact text.
Bold/italic, rich text, CJK and right-to-left layout remain unsupported. Locked annotations and malformed
free-text borders keep the existing editing restrictions. This editor does not edit PDF page text objects.

**Grouped drawing — available in 0.18.0.** Draw multiple strokes on one page, then click
**Finish drawing** to create one annotation and one Undo step, or **Cancel drawing** to discard only that draft.
Esc also cancels an unsubmitted draft. Colour, width and opacity restyle the whole draft until Finish captures
them. Finish or cancel before saving/closing, switching document/tool or making a conflicting edit; navigation,
zoom and reading view modes can keep the draft on its original page. A worker error keeps all strokes for retry
or cancellation. While Finish is pending, further drawing, Finish and Cancel are blocked.

At most 64 strokes and 4,000 points per stroke; reaching the point limit stops that stroke and another can be
started. A click is a dot. Nothing auto-commits on idle; unfinished strokes are not PDF content or crash-recovery
snapshots. Existing point simplification, geometry and protection remain. Local/CI/package/update gates passed (D109); native pen/touch remains unverified.

An annotation created by SuniPDF can be resized and restyled ("Apply to annotation" button: the bar's colours are
applied to it); one written by another application can be moved, commented and deleted (a highlight or a note can
also be recoloured). A locked annotation cannot be changed.

**Dashed shapes — available in 0.16.0.** For Rectangle, Ellipse, Line or Arrow annotations, choose
**Dashed border** before drawing. On an editable existing SuniPDF shape, change the checkbox and use
**Apply to annotation**. Checked means 6-point dashes with 3-point gaps; unchecked means solid. Fill stays
continuous and arrowheads stay solid. A loaded custom pattern is retained when selecting, recolouring, moving,
resizing or undoing the shape; changing the checkbox explicitly replaces it with the preset or solid. There is
no custom-pattern editor. Ink, free text, links and text markup do not offer dashed borders. Unsupported or
malformed border styles refuse edits that would regenerate the appearance; very dense patterns/large paths
can also be refused. Save explicitly to keep changes. These additions are included in the published 0.16.0
packages; executed checks and remaining limits are recorded in [Project status](PROJECT_STATUS.md).

An explicitly empty dash array means solid. A malformed border disables Move/Resize/Style on a SuniPDF shape;
its comment can still be edited where normal permissions allow. A free-text comment with a malformed border
cannot be edited because that changes its appearance. Valid subpixel dash patterns are retained in the preview; an
invalid or excessive pattern omits the outline preview rather than displaying a solid line.

**Clickable links.** With the reading cursor, the pointer turns into a hand over a link. An internal link takes you
to the page; an external link first opens a **confirmation dialog that shows the full address** (Open in browser,
Copy address, Cancel): nothing opens without your agreement, and only `http`, `https` and `mailto` addresses are
accepted. Actions that launch a program or a script are never run.

**Edit an existing target — available in 0.15.0.** Use **Select an annotation** (A), select the link and choose
**Edit target**, or double-click it. Choose an `http`, `https` or `mailto` address, or a physical page number.
An existing resolved named/XYZ destination prefills its page. Replacing a page target fits the whole page;
the old detailed position/zoom is not retained, as the dialog explains. An unsupported destination starts with
an empty address, so enter a valid destination explicitly. **Replace destination** keeps the link's area,
appearance, name, comments and replies. Read-only/locked/locked-content links cannot be edited; malformed flags
are refused too. Cancel/Esc keeps the original target. One Ctrl+Z undoes the change; save explicitly to keep it.
Other edits or tab switches close the pending dialog so it cannot use a stale annotation index. Existing signed-PDF
rewrite warnings still apply. These additions are included in the published 0.15.0 packages; executed checks
and remaining limits are recorded in [Project status](PROJECT_STATUS.md).

The **Comments** panel (F7) groups the page's annotations into **threads**: author, date, text, review status (None,
Accepted, Rejected, Cancelled, Completed), indented replies. **Reply** adds a reply with no icon on the page, the
status menu changes the thread's state, the bin deletes the annotation and its replies, a click on a card's title
selects it on the page. The **Author** field (remembered in your preferences, empty by default) is written into the
annotations, replies and statuses created afterwards. Threads and statuses follow the PDF standard but their display
in other PDF readers has not been verified. Every annotation can be undone (Ctrl+Z); a password-protected document
stays protected after annotating, and rotated or cropped pages are respected.

## Editing content

The **Edit a PDF** tool changes the page content itself,
not annotations: select object (E), add text (T), insert image (I), rectangle (R), ellipse (O), line (L).
Everything also works on rotated or cropped pages: objects land where you click and added text is always
upright on screen.

**Selecting.** With the select tool, click a text, image or path (or press on it and drag straight away to
move it). **Shift+click** or **Ctrl+click** adds or removes an object; dragging on an empty area draws a
**selection rectangle** (objects fully inside; **Alt**: objects merely touched); **Ctrl+A** selects every
object of the page. A selected group moves as one (mouse or arrows), is deleted, duplicated, brought to
front or sent to back, and receives the same colours and opacity.

**Transforming.** Drag the selection to move it, pull a corner handle to resize (images keep their
proportions, Shift to free them; other objects: Shift to lock), drag the **round handle** above the frame to
rotate freely (Shift: 15° steps, the angle shows in the status bar) or type an **angle** in the object bar.
Arrows nudge by one point (Shift: ten points), **Del** deletes, Escape deselects. The tool bar also offers
rotate by 90°, duplicate (Ctrl+Shift+D), bring to front (Ctrl+]) and send to back (Ctrl+[).

**Object bar.** Below the selection, a floating bar sets the stroke (colour, width), fill, **opacity**
(0 to 100%) and angle; **Apply to object** applies colours and opacity. With no selection, a shape or image
tool uses it for the next shapes and images. It also carries Copy, Cut, Paste and, for an image, **Replace
image** (same frame, new picture fitted and centred).

**Copy, cut, paste.** Ctrl+C / Ctrl+X / Ctrl+V (or the Edit menu) copy objects from one page to another, and
from one tab to another. Pasted objects form a **group**: it moves, resizes, rotates and deletes, but its
text can no longer be edited as text. Pasting at the same place offsets each copy by 12 points.

**Text.** Click the page with the text tool: a field opens **on the page**, at the zoom size, with a mini bar
(size, colour, font, bold, italic). **Enter** applies, **Shift+Enter** starts a new line, **Esc** cancels,
clicking elsewhere applies. The font list offers Helvetica, Times and Courier, then the **TrueType system
fonts** (search by name, for example "Segoe UI" or "Arial Bold"): they allow Cyrillic, Greek or extended Latin;
the font is **embedded whole** in the PDF (the mini bar shows its size). If the chosen font lacks a typed
character (for example Cyrillic with Helvetica), SuniPDF says so and reopens the field instead of losing the
character. For existing text that is upright on screen, **F2** or double-click opens the same on-page field
(content, size, colour; its original font is kept); rotated text is edited in a dialog.

**Objects panel** (F3, the layers icon of the right rail, or the tool bar button): the current page's objects, front
to back. Click: select (Ctrl: add or remove, Shift: add); hover: orange marker on the page; buttons bring
forward / send backward and delete.

Everything can be undone (Ctrl+Z) and is written into the PDF when saving.

Limits: grouped paste remains a group; use **Paste editable** for supported individual objects.
Eligible static TTF/OTF/TTC faces are subset and reused unless embedding restrictions require a full face.
The ordinary on-page input uses an interface-font preview; paragraph preview uses actual PDF metrics.
Existing text keeps its original font; unsupported replacement glyphs are refused atomically.

**Published in 0.21.1:** unsupported replacement text is refused before changing the PDF,
with the correct on-page or rotated-text draft reopened. Published 0.21.0 packages
still have the original limitation above.

**Flattening fix published in 0.21.1:** non-widget annotations with no usable appearance
retain their raw data, attachments and shared references. Widgets and drawable appearances still flatten;
retained annotations are excluded from the count. Scratch-document failure leaves the active PDF unchanged.
This fix ships in 0.21.1; previous 0.21.0 packages are unchanged.
See [executed checks and limits](PROJECT_STATUS.md).

## Filling a form

When a PDF contains a form (AcroForm fields), a banner "This document contains a form to fill in" appears above
the page with the **Fill form** button (also in the **Fill & Sign** tool, the **Forms** menu and the palette). The
tool highlights every field on the page: blue for a fillable field, **red** when it is required, grey when it is
read-only or not fillable (signature, push button). Fields stay interactive in continuous scrolling, on every
visible page.

- **Checkbox, radio button**: click, or focus the widget and press Space/Enter, to toggle the state; each click is an undo step (Ctrl+Z).
- **Text field**: one click opens an editor placed on the field; **Enter** confirms (Ctrl+Enter in a multi-line
  field, where Enter inserts a line break), **Esc** cancels, **Tab** / **Shift+Tab** confirm and move to the next /
  previous field (its content is selected); clicking elsewhere confirms too. The field's maximum length is
  enforced.
- **Drop-down list, list box**: one click opens the list of choices (checkboxes for a multi-select list;
  "(none)" clears a list that is not required).
- **Existing editable drop-down (0.11.0)**: choose **Enter a value…** in the on-page choice popup to type custom text,
  then confirm like a text field. The fields panel also accepts custom text alongside the listed choices, with the
  same Apply/Discard/Save controls. Ordinary drop-downs remain choice-only.
- The value is drawn by the PDF engine itself (regenerated appearance): what you see is what other readers will
  show, and what Ctrl+S saves.

The **Form fields** panel (Ctrl+Shift+F, button in the tool bar) lists the fields of the current page or of the
whole document, with one control per field (text box, checkbox, radio buttons, list) usable with the keyboard and
a screen reader; clicking a field name brings it on screen. It flags required and read-only fields, an **XFA**
form (read, never modified) and the presence of **scripts** (JavaScript calculations and formats: SuniPDF never
runs them, calculated fields are not updated).

**Forms** menu: **Clear fields** (texts emptied, boxes unchecked, lists deselected; read-only fields and "always
one choice" radio groups are kept), **Export form values...** (`.xfdf` or `.json`), **Import values into the form...** (`.json`, `.xfdf`, or value-only `.fdf`).
The complete GUI import is refused if a field is unknown or a value is invalid; no partial import occurs. An exported
read-only value can be imported unchanged, and passwords are never exported. The fields panel keeps changes pending:
choose **Apply changes** for one undo step, **Discard pending changes** to return to current values, or Save to apply
and save together. On-page editing remains immediate. **Flatten form and annotations…** (all pages or the current page: values, highlights and notes
become fixed content; links — web addresses, jumps to a page — have no appearance and are kept as they are; undo
with Ctrl+Z while the document is open; flattening every page removes the form from the document). A
**password** field is neither fillable nor exported (its value must never be stored). A password-protected
document is filled like any other and stays protected. Ctrl+S confirms the entry in progress first.

**0.11.0:** a failed form read shows an error and a **Retry** button in the fields panel.
Your pending text and panel changes remain available; a read failure is not treated as an empty form.
Retry reads the current fields again before applying a waiting import or clear operation. Nothing is retried in a loop.

**0.12.0:** radio fields with `/Opt` expose and accept their export values in the panel and
JSON/XFDF/FDF, even when the PDF widget's checked-state name differs. CLI `form-fields` retains `on_state` and adds
`widgets[].export_value`. A malformed or ambiguous mapping makes that field read-only; other fields remain fillable.
Radio fields without `/Opt` keep their existing state-name values.

## Signing and initialing

The **Fill & Sign** tool (bar: **Sign (visual signature)…** — key **S** —, **Add Initials…**, **Insert
Today's Date**; also in the Edit menu and the palette) places a **visual signature** on the page. **What this
signature does not guarantee**: it is an image, it proves neither your identity nor the document's integrity, and
anyone who edits the file can copy or remove it. The dialog says so permanently. Use **Use a certificate**
for the separate cryptographic signing workflow described below.

**Draw.** In the dialog, sign in the white pad with the mouse, touchpad or pen (black or blue ink, adjustable
thickness, **Undo last stroke**, **Clear**). **Import an image** takes a transparent PNG as is, or a photo of
your signature on light paper: tick **Remove the light background** and adjust the **threshold** while watching
the preview (lower it if the background remains visible, raise it if strokes disappear). **Place on Page**
closes the dialog; a ghost follows the pointer, a **click** places the signature (centred under the click, kept
inside the page), **Shift+click** places it several times, **Esc** cancels.

**Type — available in 0.13.0.** Choose **Type**, enter your name or initials, choose a standard or installed
TrueType font and black or blue ink, then click **Preview**. Times italic is the default. The preview is rendered
locally before placement; changing the text, font or ink requires another preview. Input must fit on one line of
at most 256 characters. Unsupported glyphs or restricted font embedding are reported as errors. The result is a
transparent image, using the same placement and optional saving as drawing/import.

**Page targets — available in 0.13.0.** Choose **Current page**, **All pages** or **Selected pages** before
placing. For a selection, first select pages in **Organize pages**, then open Sign/Initials; otherwise Selected
pages is disabled. One click places the same mark on each target at the same centre in PDF points. Its physical
size and proportions stay fixed unless a smaller page requires shrinking; placement is clamped inside each
visible page, including rotated/cropped pages. This is not proportional positioning relative to page size.
The batch shows progress; **Cancel** or **Esc** discards the entire batch, checked between pages. One **Ctrl+Z**
undoes the completed batch. Page insertion/deletion/reordering discards a pending dialog or captured targets, so
reopen it after changing the page structure. Save the document explicitly to keep the marks. These additions are
included in the published 0.13.0 packages; executed checks and limits are in [Project status](PROJECT_STATUS.md).

**Reuse.** With **Save on this computer for reuse** (ticked by default), the trimmed signature is stored as a
current-user DPAPI-protected image in `%APPDATA%\SuniPDF\signatures` and then appears in the dialog: click it to start placement; the
trash icon (two clicks) deletes it. Current-user Windows DPAPI encrypts new library entries; legacy PNG migration is verified before removing the old file. No image is uploaded. Anyone able to operate your
Windows session can reuse these files; untick the box to save nothing. **Initials** have their own list and a
smaller default size on the page.

**After placing**, the signature is an ordinary **image object** of the page. Current-page placement selects it right away: drag it,
resize it with the handles (proportions kept), rotate it, duplicate it, delete it, Ctrl+Z undoes it; the page
content stays visible between the strokes (transparent background). Batch placement does not automatically
select all inserted images; select individual objects with Edit a PDF. **Insert Today's Date** places a text
object (`10/15/2026` in English, `15/10/2026` in French), editable with F2. To fill a field with free text, use
the text tool (**T**) of Edit a PDF.

## Stamps, watermarks and page numbers

**Stamp / Watermark…** (**Add a stamp** tool) places a text on every page or on the current page:
size, colour, bold, angle, opacity and position (centre or one of six edges). **Number Pages…** adds a
number from a template: `{n}` is the page number, `{total}` the page count (for example
`Page {n} / {total}`). Each stamp is a page object: select, move, delete and undo it like any other.
Stamp text and page-number templates must fit on one line; CR/LF line breaks are rejected.

## Organizing pages

- **Organize pages** (F8 or the tool of the same name): page grid with multi-selection (click, Ctrl+click,
  Shift+click); the tool bar rotates, deletes, inserts from another PDF, moves forward/backward, splits and
  extracts; drag and drop to reorder.
- From the normal view: Ctrl+R / Ctrl+Shift+R rotate, Ctrl+Del deletes the current page, Ctrl+Shift+I
  inserts pages from another file.
- **Undo / Redo**: Ctrl+Z / Ctrl+Y (up to 50 steps).
- **Split document**: choose pages per file and a destination folder.
- **Extract selected pages**: creates a new PDF with the selection. From a protected document, the new
  file and split parts **retain the original encryption, passwords and permissions in 0.11.0**. An output resolving
  to the original file is refused, including with CLI `--overwrite`; the existing document-permission policy is unchanged.
- **Combine PDF files**: pick files in order, then the output file; it opens afterwards.
- **Insert Blank Page…**: size of the current page, A3, A4, A5, Letter, Legal or custom (portrait or
  landscape), number of pages, before or after the current page or at the end.
- **Crop Pages…**: left, right, top and bottom margins in millimetres, with a preview on the page render; drag a rectangle in this preview to set the crop, or enter margins;
  current page (or grid selection) or all pages; **Restore original frame** undoes a crop. Cropping **hides**
  the edges: content outside the frame stays in the file (this is not redaction).
- **Resize Pages…**: size (A3 to Legal or custom, landscape); the content is either kept as is (anchored
  top-left) or scaled and centred. Existing annotations and links do not follow the scaling.

## Reading controls

Use **View** for view-only quarter-turn rotation and full screen. These controls do not alter PDF page rotation;
starting an editing tool returns to upright view. Back/Forward navigation (Alt+Left/Alt+Right) keeps up to 256 positions
per tab, including page and scroll anchors. View state/history belongs to each tab and is not PDF content.

### 0.11.0 navigation

PDF page labels such as `iv`, `1` or `Annexe-É7` appear alongside physical page indices in thumbnails, the status bar
and accessibility descriptions. Type an exact, case-sensitive, unique label into the page box and press Enter.
A matching label takes priority over a number: if label `1` belongs to physical page 3, entering `1` opens page 3.
Enter `#1` to select physical page 1 explicitly. Without a matching label, a number selects the physical page.
Unknown, duplicate or out-of-range input leaves the current page unchanged and displays an explanation.

Page-label authoring is available in 0.14.0, as described below. There is no
semantic renumbering after inserting, deleting, moving or extracting pages; existing label ranges can remain
tied to physical positions, and extracted files may have no labels. Blank, unreadable, control-containing or
over-4096-byte UTF-8 labels fall back to physical indices.
CLI `info` exposes an optional `pages[].label`; all CLI page numbers remain physical and one-based.

Internal links and bookmarks also follow supported PDF destinations: `/XYZ` positions and zoom, `/Fit` page fit,
and `/FitH` width fit with a usable position. Crop and page rotation are taken into account. An omitted `/XYZ` axis
or zero/omitted zoom keeps the current viewport value; zoom stays within the application's limits. Alt+Left/Alt+Right
returns through the tab's page, position and zoom history, including destinations on the same page. Finish pending
text or form edits before following a destination. Unsupported fit modes, unreadable destination details and
`/FitH` zero/null positions fall back to the target page without applying destination zoom or position.

### Page-label authoring — available in 0.14.0

Open **Pages → Page Labels…**, or the Page Labels command in **Organize pages**. Set **First physical page**
and **Last physical page**: the range includes both ends. Choose decimal, uppercase/lowercase Roman,
uppercase/lowercase letters or prefix only, enter an optional prefix and the starting number, then **Apply Labels**.
Roman examples are `i, ii, iii`; letter numbering goes `A…Z, AA, BB…ZZ`, as specified by PDF. Prefix only repeats
the same text on each target. The starting number defaults to 1 and must be positive; prefix only requires
nonblank text. New prefixes must fit on one line of at most 256 UTF-8 bytes; generated labels are bounded to
4096 bytes. Invalid input or a malformed existing label tree refuses Apply without partial changes.

Existing labels outside the chosen range retain their original numbering. **Reset All Labels** removes all
label rules, including a malformed tree, and returns navigation to physical page indices. Reset does not securely
erase old metadata objects. These changes do not add or change printed page numbers: use Number Pages for that.
Apply/Reset can be undone; save explicitly to keep the changes. Existing signed-document rewrite warnings still
apply to these metadata edits. Undo/Redo of label/content edits retains the view/navigation when page geometry
is unchanged; structural edits still invalidate it.

After inserting, deleting or moving pages, label ranges follow physical positions rather than rebasing by document
meaning. Check them after reorganization. Duplicate labels remain ambiguous; use physical `#N` to navigate.
These additions are included in the published 0.14.0 packages; executed checks and limits are in [Project status](PROJECT_STATUS.md).

## Image compression and batches

Open **Compress a PDF**, choose the maximum image side (128..8192 pixels) and JPEG quality (10..100), then save a
new compressed copy. The result shows actual before/after bytes; cancellation leaves the source untouched. Compression
is lossy, supports bounded 8-bit RGB/Gray images and can yield no gain; masks/profiles/custom decoding stay unchanged.
Signed or modification-restricted PDFs are refused. Existing output files are never replaced.

Choose **Batch processing** for compression or local English/French OCR, select 1..128 PDFs and a destination folder,
then start. Each input is limited to 256 MiB. Progress/cancellation and an individual result are shown for each file.
Copies use `-compressed` or `-ocr` names. Completed copies remain when another input fails or cancellation occurs.
Batch files do not prompt for passwords; files requiring one are reported as errors.

## Converting

- **Create PDF from images** (PNG, JPEG, BMP, GIF, TIFF, WebP): one page per image, 150 dpi; the EXIF
  orientation of photos is honoured. All eight JPEG orientations are embedded without recompression.
- **Export pages as images**: one PNG per page (150 dpi) in the chosen folder.
- **Optimize file size**: lossless recompression; the gain depends on the file and may be small.

**Available in 0.17.0:** all eight JPEG EXIF orientations, including 5 and 7, are embedded directly without
recompression. Before decoding the other formats, Images → PDF checks a cumulative estimated raster budget
of 512 MiB: the sum of the larger of each decoder's announced size and `width × height × 4` bytes. The checks
repeat at loading and reject invalid physical page sizes. Image quality/transparency remain preserved; this
budget estimates raster data, not total/native process memory, and does not limit compressed JPEGs. Existing
UI and CLI `images2pdf` usage is unchanged. An oversized raster batch reports `image_batch_too_large`; choose
fewer or smaller images. Local workspace, qpdf/corpus and independent debug conversion checks passed;
Windows CI, same-source ARM64 compilation, upgrades, staged/public package checks and production updater also
passed. See [Project status](PROJECT_STATUS.md) for evidence and remaining hardware/interoperability limits.

## Security

- **Protect with password**: open password, owner password, permissions (print, copy, edit, annotate),
  AES-256 by default (AES-128 for older readers). Encryption is applied to the file when you save. AES-128
  only accepts Latin-1 passwords (accented letters included, but not €, œ, typographic dashes or quotes);
  keep AES-256 for Cyrillic, Greek, CJK or emoji passwords and for those symbols. Text extraction for
  accessibility (screen readers) is always allowed.
- **Remove protection**: asks for the owner password (or the open password when no distinct owner
  password was set). Restrictions (printing, copying…) require an owner password different from the open
  password.
- Operations on a protected document (metadata, optimisation, object order) **keep** its protection.
- **Remove metadata**: strips the information dictionary, XMP metadata (document, pages, images) and
  application private data (`PieceInfo`). Editing the properties also updates the XMP.
- **Document properties** (Ctrl+D): editable title, author, subject, keywords; version, size, encryption
  and permissions read-only.
- **Redact a PDF** (B key): draw rectangles over what must disappear, or use **Find Text to Redact…** to mark
  every occurrence of a text; a click on an area removes it, **Clear Areas** removes them all. Nothing is deleted
  until you choose **Apply Redactions…** (Ctrl+Shift+B, then the red button or Ctrl+Enter). Text, images,
  drawings, annotations and fields under the areas are then **removed from the document** (not just covered);
  whatever extends beyond an area stays visible as an image. SuniPDF then checks that no text can still be
  extracted under the areas, otherwise nothing is changed: check "Convert redacted pages entirely to images" in
  that case. Ctrl+Z undoes while the document is open; the file only changes when you save (saving with
  unapplied areas asks for confirmation). Limits: text inside a scanned image without OCR cannot be found by
  searching, bookmarks and attachments are not changed, a digital signature becomes invalid (details:
  `docs/KNOWN_ISSUES.en.md` A40). From the command line:
  `sunipdf-cli redact input.pdf --text "John Smith" --rect "1:72,500,272,560" --out redacted.pdf`.

## Saving

Ctrl+S saves to the open file, Ctrl+Shift+S to a new file. Writes are atomic: the original file is never
left half-written (data is flushed to disk before the replacement). A dot after the tab name signals unsaved
changes; undoing back to the saved state clears it; SuniPDF asks before closing or quitting.

## Printing

Choose **File → Print** to open the native Windows printer dialog. Select a printer, page range, current
page or selected pages, copies and printer orientation. SuniPDF prints the current document state,
including applied edits, annotations and form values, as raster pages fitted inside the printer's printable
area. A progress dialog offers cancellation; a driver operation already running may take time to stop.
Output uses up to 300 dpi in bounded raster bands. Printer drivers may apply further scaling or colour changes.
Actual Microsoft Print to PDF output is covered by an automated test. Physical printers and interactive
driver dialogs still require checking on the target machine.

## Creating form fields

Open **Prepare a form** and choose text, checkbox, radio group, drop-down list or list box on the current page.
Use a unique name without dots/control characters and a displayed-page rectangle in PDF points, inside the page and
at least 4 points wide/high. For choices, enter one `export value | display label` per line; export values must be
unique. List boxes can allow multiple selections. A radio group's rectangle contains vertically arranged buttons;
labels appear in the fields panel and are not inserted as PDF page text. Creation is undoable and keeps encryption.

**0.12.0:** for a drop-down, enable **Allow custom text in addition to the listed options** to accept
both predefined choices and typed values. Keep at least one predefined choice with a unique, nonempty export value.
This setting applies only to new drop-downs; changing the flag on an existing field is not offered.

**Available in 0.19.0:** choose **Draw on page** to hide this dialog, drag a rectangle on one
page and reopen it with the field name, choices and flags retained. Review the numeric rectangle, then choose
**Create Field**; drawing alone does not create a field. Rectangles stay inside one page and must be at least
4 PDF points wide/high. Reverse dragging is allowed; a click or tiny rectangle stays in drawing mode so you
can retry. Release outside the page clamps to the original page. **Escape** cancels the builder without writing.
Document revision/tab/tool/workspace changes cancel stale placement; zoom/navigation remain possible, including
release finalization when a Single-page jump hides the original page. In that case the last known rectangle
reopens in the numeric dialog for review; only **Create Field** creates it. Numeric creation remains available.
This adds no new field type or editor for existing fields. Native mouse/pen/touch and Narrator checks remain separate.

XFA creation is unsupported. FDF import accepts normal xref text/name/array values;
streams, actions, attachments and rich text are refused. Referenced document locations are never opened.

## Offline OCR

Open **Scan & OCR**, choose English, French or both, and the current page or all pages, then start recognition.
The embedded Tesseract 5.5.3 / Leptonica 1.87 runtime and fast `eng`/`fra` models run locally; no model download
or PDF upload is required. Pages already containing extractable text are skipped. Recognised words become
an invisible text layer for search and copy, while the existing page appearance is retained.
Recognition targets 300 dpi, reduced for large pages (16 million pixels, 8192 pixels per side); very large
pages are refused. Progress is reported per page. Cancellation or an error leaves the document unchanged;
a successful run is one undo step. Save to keep the result, and proofread recognised text before relying on it.

## Certificate signatures

**Use a certificate → Verify signatures** checks signatures over the saved PDF bytes and shows the result,
certificate subject, issuer, validity dates and SHA256 fingerprint. Results distinguish a valid signed revision,
changes after signing, invalid content, unsupported formats and unverifiable signatures.
CMS integrity alone proves neither identity trust nor revocation. Verification reports those decisions and
supported RFC3161 timestamps separately, offline by default. Creating a signed copy does not add a trusted
timestamp or archival LTV evidence. See [signature security](SIGNATURE_SECURITY.md).
The displayed signing time is the signer's declared time.

Save edits first, then choose **Sign with a certificate**, select a PFX/P12 identity, enter its password and
optional reason/location, and choose a new output PDF. Signing appends a CMS signature revision while preserving
the preceding bytes. It never replaces the source, and encrypted PDFs cannot be signed.
Editing and saving an already signed PDF can invalidate its signatures; confirm the application's rewrite warning
only when that is intended. Private-key bytes and password copies are cleared from application buffers after use;
SuniPDF does not save the password or import the key into a persistent key store.

## Command palette

Ctrl+K opens the palette: type a few letters, pick with ↑ ↓, confirm with Enter. Each command carries
the colour of its tool; entries such as "Edit a PDF" or "Add comments" open the tool.

## Command line

`sunipdf-cli.exe --help` lists the commands: `info`, `render`, `text`, `search`, `merge`, `split`, `extract`,
`rotate`, `insert`, `move`, `images2pdf`, `export-images`, `optimize`, `protect`, `unprotect`,
`properties`, `bookmarks`, `set-metadata`, `strip-metadata`, `stamp`, `page-numbers`, `commands`, for
annotations: `annotations` (JSON list: index, subtype, rectangle in displayed-page points, text, author, style, link, thread), `annotate` (`--kind ink|square|circle|line|arrow|freetext|link`, `--rect left,bottom,right,top`, repeatable `--stroke "x,y x,y …"`, `--text`, `--color`, `--fill`, `--width`, `--opacity`, `--goto-page` or `--url`, `--author`), `reply`, `review-state` (`none|accepted|rejected|cancelled|completed`) and `annotation-delete` (`--thread`), and for
content editing: `objects` (a page's objects, 0-based indices), `add-text` (`--font-name "Arial"` or
`--font-file` for an embedded system font; `\n` for a new line), `set-opacity`, `rotate-object` (free angle),
`copy-objects` (to another page, `--into` for another document), `replace-image`, `crop` (`--unit mm`,
`--reset`), `resize-pages` (`--mode keep|scale`), `insert-blank`, `fonts`, and `add-image`
(`--image signature.png --x 100 --y 520 --width 150`, `--remove-background 200` for a photo on a light
background: an image placed on the page, e.g. a scanned signature; the reply carries
`"digital_signature": false`, it is never a digital signature). Forms: `form-fields` (fields, types, values,
options, widgets with 1-based pages; `--values-only` for the bare `{ name: value }` object), `fill-form`
(`--values file.json|file.xfdf|file.fdf` or `-` for JSON standard input, repeatable `--set name=value` — checkbox: `true`/`false`, list
box: choices separated by `;` —, strict by default: an unknown field or a rejected value is a
`form_field_not_found` / `form_value_invalid` error and nothing is written, `--ignore-unknown` to skip and count,
`--flatten`) and `flatten` (`--pages all` or `1,3-5`; every page = the form is removed). An XFA form returns
`form_xfa`.
Output is JSON. In 0.11.0, `extract` and `split` retain protection: `encryption_removed` stays `false`, and additive
`encrypted` reports whether the copies are encrypted. They refuse a resolved source-path output even with `--overwrite`. Exit codes
distinguish password required (3), invalid file (4), unreadable file (5) and
existing output file (6). No command overwrites an existing file without `--overwrite`. `render` always
writes PNG (`--region x,y,w,h`: an area in pixels of the page rendered at `--scale`, for scales where the
whole page would exceed 16,384 px); `rotate` accepts 90, 180, 270 or -90. Coordinates (`search`, `objects`, `add-text`…) are in
points, origin at the bottom-left of the page **as displayed** (after rotation and cropping).
`add-text` requires finite `--x` and `--y` coordinates. CLI `annotate --kind freetext` accepts
only `--opacity 1`; other annotation opacity values must be finite. Invalid values are rejected
before saving, including when `--overwrite` is supplied.

**Available in 0.13.0:** `add-image --pages all` or `--pages "1,3-5"` places the image once per selected
physical page. This conflicts with explicit `--page`; omitting both keeps the existing single-page default.
`--x`, `--y` and `--width` describe the requested lower-left corner and size in displayed PDF points. Batch
placement fits/clamps that request on each page and keeps the source protection; existing single-page placement
is unchanged. Batch JSON adds `pages` and `count`; its coordinates/size describe the request, not each fitted
rectangle. Example:

```powershell
sunipdf-cli add-image input.pdf --image signature.png --pages "1,3-5" --x 100 --y 40 --width 150 --out initialled.pdf
```

**Available in 0.14.0:** `set-page-labels` accepts `--from` (default 1), `--to` (default last page),
`--style decimal|roman-upper|roman-lower|letters-upper|letters-lower|prefix-only` (default decimal), `--start`
(default 1) and `--prefix` (default empty). `reset-page-labels` removes all label rules. Both take `--out` and
optional `--password`, retain encryption/permissions and use the existing overwrite safeguards. Invalid input
leaves existing output unchanged, including with `--overwrite`. Page arguments remain physical and one-based;
`info` reads back labels through `pages[].label`.

```powershell
sunipdf-cli set-page-labels input.pdf --from 1 --to 4 --style roman-lower --out labelled.pdf
sunipdf-cli set-page-labels labelled.pdf --from 5 --style decimal --prefix "Appendix-" --start 1 --out appendix.pdf
sunipdf-cli info appendix.pdf
sunipdf-cli reset-page-labels appendix.pdf --out unlabelled.pdf
```

**Available in 0.15.0:** `set-link-target` uses `--page` (default physical page 1) and required `--index`
(zero-based annotation index from `annotations`). Supply exactly one of `--goto-page N` or `--url ADDRESS`,
plus `--out` and optional `--password`. A page target uses whole-page Fit; safe addresses are limited to `http`,
`https` and `mailto`. Existing output guards apply, and invalid input leaves the existing output unchanged,
including with `--overwrite`. The original link's rectangle/style/comments and document protection are retained.

```powershell
sunipdf-cli annotations input.pdf --page 1
sunipdf-cli set-link-target input.pdf --page 1 --index 0 --url "https://example.org/updated" --out updated-link.pdf
sunipdf-cli set-link-target updated-link.pdf --page 1 --index 0 --goto-page 2 --out page-link.pdf
```

**Available in 0.16.0:** add `--dashed` to `annotate --kind square|circle|line|arrow` for the fixed `[6,3]`
point pattern. `annotations` returns the actual `dash_pattern` array. Without the flag, existing solid behaviour
stays unchanged. The flag is refused for unsupported annotation kinds; invalid requests leave no output or
preserve an existing output, including with `--overwrite`. Use `sunipdf-cli --help` for the current command list.

```powershell
sunipdf-cli annotate input.pdf --kind square --rect 40,40,240,140 --dashed --out dashed.pdf
sunipdf-cli annotations dashed.pdf --page 1
```

Commands added in 0.9.0:

```powershell
sunipdf-cli add-form-field input.pdf --kind text --name FullName --page 1 --rect 40,40,220,65 --out form.pdf
sunipdf-cli add-form-field form.pdf --kind checkbox --name Consent --rect 30,30,50,50 --out form-with-checkbox.pdf
sunipdf-cli ocr scan.pdf --language eng+fra --pages 1,3-5 --out searchable.pdf
sunipdf-cli signatures saved.pdf
sunipdf-cli sign saved.pdf --certificate identity.pfx --certificate-password-env SUNIPDF_CERT_PASSWORD --out signed-copy.pdf
```

For `sign`, set the named environment variable securely before execution, or use `--certificate-password-stdin`
with piped UTF-8 input instead. These options are mutually exclusive and one is required; one trailing line ending
is removed from stdin. There is no plaintext certificate-password argument. `--reason` and `--location` are optional.
Signature JSON separates integrity from trust and revocation; `identity_trusted` requires the recorded trust policy to succeed. Verification is offline by default; `--allow-network` explicitly permits certificate retrieval for that check. Even `--overwrite` cannot replace the signing input.
`ocr --language` accepts `eng`, `fra` or `eng+fra`; `--pages` defaults to `all`. If no text is added, the output
copies the source bytes exactly.

Commands added in 0.10.0:

```powershell
sunipdf-cli compress-images scan.pdf --max-image-side 1800 --jpeg-quality 75 --out scan-compressed.pdf
sunipdf-cli add-form-field form.pdf --kind combo --name Country --option FR=France --option BE=Belgium --rect 40,40,220,65 --out choices.pdf
sunipdf-cli form-export choices.pdf --out values.xfdf
sunipdf-cli fill-form choices.pdf --values values.xfdf --out filled.pdf
```

`rotate-objects` and `resize-objects` apply a group transformation; use `--help` for the actual coordinate/scale options.
`compress-images` always requires a new copy, including with `--overwrite`.

In the **0.12.0** CLI, add `--editable` to `add-form-field --kind combo` to allow custom text;
other field kinds reject this flag. Predefined choices are still required.

## Current limits

PDF password input since 0.30.1: prefer `--password-env NAME` or `--password-stdin` with piped UTF-8
input. New protection credentials also support `--user-password-env`/`--user-password-stdin` and
`--owner-password-env`/`--owner-password-stdin`. Only one credential may use stdin per invocation;
one trailing line ending is removed. Populate environment variables through a suitable secret source
and clear them afterwards. Literal legacy password arguments remain visible in process/history tools.
Changing existing encryption requires the current owner password. `signatures` prints its complete
JSON report then exits with code 2 if verification fails; a PDF with no signatures remains a successful
listing. [Security behavior and limits](SECURITY_MODEL.md).

Office conversion has the bounded local scope above; editable XFA forms remain unavailable; form scripts are never run. Complex column reading
order is not inferred. Field creation supports text, checkbox, radio, combo and list widgets; FDF interchange has the value-only limits above. OCR accuracy depends on the scan
and language; existing text is not replaced. Windows trust, revocation and supported RFC3161 timestamps have explicit results; archival LTV validation and AATL/EUTL qualification remain unsupported.
Real printer, installation/update and ARM64 hardware validation remain separate from automated tests.
See `docs/KNOWN_ISSUES.md` and `docs/ROADMAP.md`.


## Local comparison and search across selected PDFs (0.31.0)

Use **File → Compare PDF files…** or **Edit → Search in PDF files…**. Save current edits before selecting the files. Compare physical page pairs with adjustable starting pages; select a changed pair for three previews. Search results open their physical page. Both operations are read-only, local and cancellable. Partial results never mean the remaining files are clean. See the [complete guide and limits](LOCAL_REVIEW.md), including protected files, image-only pages, JSON commands and preview resolution.

## Reorder objects by dragging

Open the Objects panel with F3. Rows at the top are in front. Drag a row above or below another row; the blue insertion line shows the destination. Dragging a selected row moves the entire selection as one block, keeping its original relative drawing order. Dragging an unselected row moves that object only.

Escape cancels. Dropping outside a valid row, onto a selected row or at the same position does nothing. A drop into another page/document or after an intervening edit is refused. Selection follows the moved objects. Existing forward/backward buttons and keyboard commands remain available; one completed drag uses one Undo step and supports Redo through the existing worker transaction.

[Guide](OBJECT_ORDER.md).
