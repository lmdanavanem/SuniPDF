# SuniPDF — User Guide

SuniPDF is a PDF reader and editor for Windows 10/11. It works entirely offline: no account, no telemetry,
no document ever leaves your computer.

## Installation

- **Portable**: unzip `SuniPDF-<version>-win-x64-portable.zip` anywhere and run `sunipdf.exe`. Nothing is
  written outside your preferences (`%APPDATA%\SuniPDF\settings.json`).
- **MSI installer**: double-click `SuniPDF-<version>-win-x64.msi`. SuniPDF appears in the Start menu and in
  "Open with" for PDF files; it does not become the default reader without your consent.
- Uninstall: Windows Settings › Apps, or delete the folder for the portable version.

## Home screen

The home screen shows a large **Open file** card, one card per available tool (the whole card is clickable)
and your recent files. You can also drop a PDF anywhere in the window, or click **+ Open** in the tab strip.

## All tools

The **All tools** panel on the left (F9, or the button of the same name in the toolbar) lists the tools by
task, like PDFCraft: Export a PDF, Edit a PDF, Create a PDF, Combine files, Organize pages, Add comments,
Protect a PDF, Compress a PDF, Add a stamp. The **Tools** menu and the Ctrl+K palette give the same access.
Without an open document, a tool that needs one first asks for a file, then starts.

- **Export, Create, Combine, Compress** open their dialog directly.
- **Edit, Organize, Add comments, Protect, Add a stamp** (marked with a › chevron) show a **tool bar** under
  the toolbar: the tool's buttons, grouped, with their labels when the width allows (otherwise the icon only,
  the label staying in the tooltip). Actions that need a selection (object, text, grid pages) stay greyed
  out without one.
- **Leave tool**: button on the right of the bar, **Esc**, clicking the active tool again in the panel, or the
  back arrow (which also reopens the panel). Esc first closes what is closer: an open menu or drop-down
  list, then the search bar, then the selected object, then the tool (one Esc, one effect).
- Single-letter shortcuts open the matching tool: H, U, X, N open **Add comments**; E, T, I, R, O, L open
  **Edit a PDF**; F8 opens **Organize pages** with the grid; V goes back to the reading cursor.
- Each tab keeps its own active tool.
- The main toolbar keeps quick access to Open, Save, Undo and Redo.

PDFCraft tools that have no real feature in SuniPDF yet (fill & sign, OCR, redaction, form preparation,
certificate) are not shown: they will come with their feature.

## Tabs

Each open PDF gets a tab in the top strip. Click a tab to show it, its cross to close it, the home button to
go back to the home screen without closing anything. Ctrl+Tab and Ctrl+Shift+Tab cycle through tabs.
Unsaved changes are flagged with a dot per tab and confirmed one by one when you quit.

## Reading

By default pages are **stacked**: the mouse wheel, the scroll bar or the keyboard scroll through the whole
document, and the page that fills the screen becomes the current page (right-rail number, status bar,
thumbnail, comments panel; Rotate, Delete and Extract act on it; clicking a page makes it current). **F5**, the
arrows button of the right rail or the View menu switch between **Continuous Scrolling** and **Single Page**;
the choice is remembered. Only visible pages are rendered: a document of several hundred pages opens and
scrolls without loading everything.

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
| Select text | Drag a rectangle over the text (reading cursor, V), then Ctrl+C |
| Light / dark theme | Moon / sun icon, top right |
| Language | Globe icon, top right (French, English) |

## Annotating

The **Add comments** tool offers: select text (V), highlight (H), underline (U), strikethrough (X),
note (N), the Comments panel and copying the selected text. Drag a rectangle over text to apply a markup; click the page with the note
tool to type a comment. The **Comments** panel (F7) lists the annotations of the current page and lets you
delete them. Every annotation can be undone (Ctrl+Z).

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

Limits: a pasted or duplicated group can no longer be edited as text; rotating or resizing a group of several
objects is not offered yet (moving only); `.otf` and `.ttc` fonts are not offered; fonts are embedded whole
(no subsetting); the on-page field previews text with the interface font (approximate widths); edited text
keeps its original font, which may not contain every character you type.

## Stamps, watermarks and page numbers

**Stamp / Watermark…** (**Add a stamp** tool) places a text on every page or on the current page:
size, colour, bold, angle, opacity and position (centre or one of six edges). **Number Pages…** adds a
number from a template: `{n}` is the page number, `{total}` the page count (for example
`Page {n} / {total}`). Each stamp is a page object: select, move, delete and undo it like any other.

## Organizing pages

- **Organize pages** (F8 or the tool of the same name): page grid with multi-selection (click, Ctrl+click,
  Shift+click); the tool bar rotates, deletes, inserts from another PDF, moves forward/backward, splits and
  extracts; drag and drop to reorder.
- From the normal view: Ctrl+R / Ctrl+Shift+R rotate, Ctrl+Del deletes the current page, Ctrl+Shift+I
  inserts pages from another file.
- **Undo / Redo**: Ctrl+Z / Ctrl+Y (up to 50 steps).
- **Split document**: choose pages per file and a destination folder.
- **Extract selected pages**: creates a new PDF with the selection.
- **Combine PDF files**: pick files in order, then the output file; it opens afterwards.
- **Insert Blank Page…**: size of the current page, A3, A4, A5, Letter, Legal or custom (portrait or
  landscape), number of pages, before or after the current page or at the end.
- **Crop Pages…**: left, right, top and bottom margins in millimetres, with a preview on the page render;
  current page (or grid selection) or all pages; **Restore original frame** undoes a crop. Cropping **hides**
  the edges: content outside the frame stays in the file (this is not redaction).
- **Resize Pages…**: size (A3 to Legal or custom, landscape); the content is either kept as is (anchored
  top-left) or scaled and centred. Existing annotations and links do not follow the scaling.

## Converting

- **Create PDF from images** (PNG, JPEG, BMP, GIF, TIFF, WebP): one page per image, 150 dpi.
- **Export pages as images**: one PNG per page (150 dpi) in the chosen folder.
- **Optimize file size**: lossless recompression; the gain depends on the file and may be small.

## Security

- **Protect with password**: open password, owner password, permissions (print, copy, edit, annotate),
  AES-256 by default (AES-128 for older readers). Encryption is applied to the file when you save.
- **Remove protection**: asks for the owner password (or the open password when no distinct owner
  password was set). Restrictions (printing, copying…) require an owner password different from the open
  password.
- Operations on a protected document (metadata, optimisation, object order) **keep** its protection.
- **Remove metadata**: strips the information dictionary and XMP metadata.
- **Document properties** (Ctrl+D): editable title, author, subject, keywords; version, size, encryption
  and permissions read-only.

## Saving

Ctrl+S saves to the open file, Ctrl+Shift+S to a new file. Writes are atomic: the original file is never
left half-written. A dot after the tab name signals unsaved changes; SuniPDF asks before closing or quitting.

## Command palette

Ctrl+K opens the palette: type a few letters, pick with ↑ ↓, confirm with Enter. Each command carries
the colour of its tool; entries such as "Edit a PDF" or "Add comments" open the tool.

## Command line

`sunipdf-cli.exe --help` lists the commands: `info`, `render`, `text`, `search`, `merge`, `split`, `extract`,
`rotate`, `insert`, `move`, `images2pdf`, `export-images`, `optimize`, `protect`, `unprotect`,
`properties`, `bookmarks`, `set-metadata`, `strip-metadata`, `stamp`, `page-numbers`, `commands`, and for
content editing: `objects` (a page's objects, 0-based indices), `add-text` (`--font-name "Arial"` or
`--font-file` for an embedded system font; `\n` for a new line), `set-opacity`, `rotate-object` (free angle),
`copy-objects` (to another page, `--into` for another document), `replace-image`, `crop` (`--unit mm`,
`--reset`), `resize-pages` (`--mode keep|scale`), `insert-blank`, `fonts`.
Output is JSON; exit codes distinguish password required (3), invalid file (4), unreadable file (5) and
existing output file (6). No command overwrites an existing file without `--overwrite`. `render` always
writes PNG (`--region x,y,w,h`: an area in pixels of the page rendered at `--scale`, for scales where the
whole page would exceed 16,384 px); `rotate` accepts 90, 180, 270 or -90. Coordinates (`search`, `objects`, `add-text`…) are in
points, origin at the bottom-left of the page **as displayed** (after rotation and cropping).

## Current limits

No two-page view or view rotation; a text selection stays on its page (no selection across two pages, no
auto-scroll when dragging at the edge); in continuous scrolling, "Width" and "Page" fit the largest page of the
document. No OCR, no Office conversion, no digital signatures, no direct printing. See `docs/ROADMAP.md`.
