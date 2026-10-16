# SuniPDF — User Guide

SuniPDF is a PDF reader and editor for Windows 10/11. PDF processing stays local: no account, no telemetry,
no document ever leaves your computer.

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
task: Export a PDF, Fill & Sign, Edit a PDF, Create a PDF, Combine files, Organize pages, Add
comments, Protect a PDF, Redact a PDF, Compress a PDF, Add a stamp, Scan & OCR, Prepare a form and
Use a certificate. The **Tools** menu and the Ctrl+K palette give the same access.
Without an open document, a tool that needs one first asks for a file, then starts.

- **Export, Create, Combine, Compress** open their dialog directly.
- **Fill & Sign, Edit, Organize, Add comments, Protect, Add a stamp** (marked with a › chevron) show a **tool bar** under
  the toolbar: the tool's buttons, grouped, with their labels when the width allows (otherwise the icon only,
  the label staying in the tooltip). Actions that need a selection (object, text, grid pages) stay greyed
  out without one.
- **Leave tool**: button on the right of the bar, **Esc**, clicking the active tool again in the panel, or the
  back arrow (which also reopens the panel). Esc first closes what is closer: an open menu or drop-down
  list, then the search bar, then the selected object, then the tool (one Esc, one effect).
- Single-letter shortcuts open the matching tool: H, U, X, N, A, D, F open **Add comments**; E, T, I, R, O, L open
  **Edit a PDF**; S opens the **Sign** dialog; F8 opens **Organize pages** with the grid; V goes back to the reading cursor.
- Each tab keeps its own active tool.
- The main toolbar keeps quick access to Open, Save, Undo and Redo.

All 14 tools are available in 0.9.0, including **Scan & OCR**, **Prepare a form** and **Use a certificate**.
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
scrolls without loading everything. The View menu also offers **Two pages** (pairs from the first page) and
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
| **Freehand drawing** (D) | Drag on the page: smoothed stroke (one stroke = one annotation, a click = a dot); colour, width and opacity in the floating bar |
| **Free text box** (F) | Click (default box) or drag (chosen box), then type in the dialog: size, colour, background, border. The box grows downwards if the text is taller. Helvetica font: characters outside the Latin alphabet show as "?", the exact text stays in the annotation |
| **Rectangle, Ellipse, Line or arrow** (annotations) | Drag; colour, width, fill (rectangle and ellipse), opacity, "Arrow" checkbox |
| **Link** | Draw an area (at least 6 points) then choose "Go to a page" or "Open a web address" (`http`, `https` or `mailto` only) |

An annotation created by SuniPDF can be resized and restyled ("Apply to annotation" button: the bar's colours are
applied to it); one written by another application can be moved, commented and deleted (a highlight or a note can
also be recoloured) but is never rewritten, so nothing of its fidelity is lost. A locked annotation cannot be changed.

**Clickable links.** With the reading cursor, the pointer turns into a hand over a link. An internal link takes you
to the page; an external link first opens a **confirmation dialog that shows the full address** (Open in browser,
Copy address, Cancel): nothing opens without your agreement, and only `http`, `https` and `mailto` addresses are
accepted. Actions that launch a program or a script are never run.

The **Comments** panel (F7) groups the page's annotations into **threads**: author, date, text, review status (None,
Accepted, Rejected, Cancelled, Completed), indented replies. **Reply** adds a reply with no icon on the page, the
status menu changes the thread's state, the bin deletes the annotation and its replies, a click on a card's title
selects it on the page. The **Author** field (remembered in your preferences, empty by default) is written into the
annotations, replies and statuses created afterwards. Threads and statuses follow the PDF standard but their display
in Acrobat or Foxit has not been verified. Every annotation can be undone (Ctrl+Z); a password-protected document
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

Limits: a pasted or duplicated group can no longer be edited as text; group rotation/resizing is available in 0.10.0 with one undo step; `.otf` and `.ttc` fonts are not offered; fonts are embedded whole
(no subsetting); the on-page field previews text with the interface font (approximate widths); edited text
keeps its original font, which may not contain every character you type.

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

## Signing and initialing

The **Fill & Sign** tool (bar: **Sign (handwritten signature)…** — key **S** —, **Add Initials…**, **Insert
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

**Reuse.** With **Save on this computer for reuse** (ticked by default), the trimmed signature is written as a
PNG in `%APPDATA%\SuniPDF\signatures` and then appears in the dialog, on a white card: one click places it, the
trash icon (two clicks) deletes it. Nothing is encrypted or sent over the Internet: anyone with access to your
Windows session can reuse these files; untick the box to save nothing. **Initials** have their own list and a
smaller default size on the page.

**After placing**, the signature is an ordinary **image object** of the page, selected right away: drag it,
resize it with the handles (proportions kept), rotate it, duplicate it, delete it, Ctrl+Z undoes it; the page
content stays visible between the strokes (transparent background). **Insert Today's Date** places a text
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
  file (like the parts of a split) **is not encrypted**: the status bar says so; open it and run **Protect**
  if needed.
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
  orientation of photos is honoured and JPEG files are embedded without recompression.
- **Export pages as images**: one PNG per page (150 dpi) in the chosen folder.
- **Optimize file size**: lossless recompression; the gain depends on the file and may be small.

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
Drawing fields with the mouse and XFA creation are unsupported. FDF import accepts normal xref text/name/array values;
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
This proves neither certificate identity trust nor revocation status; no trusted timestamp or LTV is provided.
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
Output is JSON (`extract` and `split` report `encryption_removed` when the source was protected); exit codes
distinguish password required (3), invalid file (4), unreadable file (5) and
existing output file (6). No command overwrites an existing file without `--overwrite`. `render` always
writes PNG (`--region x,y,w,h`: an area in pixels of the page rendered at `--scale`, for scales where the
whole page would exceed 16,384 px); `rotate` accepts 90, 180, 270 or -90. Coordinates (`search`, `objects`, `add-text`…) are in
points, origin at the bottom-left of the page **as displayed** (after rotation and cropping).
`add-text` requires finite `--x` and `--y` coordinates. CLI `annotate --kind freetext` accepts
only `--opacity 1`; other annotation opacity values must be finite. Invalid values are rejected
before saving, including when `--overwrite` is supplied.

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
Signature JSON always reports `identity_trusted: false`. Even `--overwrite` cannot replace the signing input.
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

## Current limits

Office conversion and editable XFA forms are unavailable; form scripts are never run. Complex column reading
order is not inferred. Field creation supports text, checkbox, radio, combo and list widgets; FDF interchange has the value-only limits above. OCR accuracy depends on the scan
and language; existing text is not replaced. Certificate trust, revocation, trusted timestamps and LTV are unchecked.
Real printer, installation/update and ARM64 hardware validation remain separate from automated tests.
See `docs/KNOWN_ISSUES.md` and `docs/ROADMAP.md`.
