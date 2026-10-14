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

The home screen lists the available tools, your recent files and an **Open file** button. You can also drop
a PDF anywhere in the window, or click **+ Open** in the tab strip.

## Tabs

Each open PDF gets a tab in the top strip. Click a tab to show it, its cross to close it, the home button to
go back to the home screen without closing anything. Ctrl+Tab and Ctrl+Shift+Tab cycle through tabs.
Unsaved changes are flagged with a dot per tab and confirmed one by one when you quit.

## Reading

| Action | How |
|---|---|
| Next / previous page | Page Down / Page Up, ← →, right-rail buttons |
| First / last page | Home / End |
| Go to page | Type the number in the right-rail box, then Enter |
| Zoom | Ctrl + / Ctrl −, Ctrl 0 (actual size), zoom list, "Width", "Page" |
| Thumbnails | **Pages** panel on the left (grid button on the right rail); drag a thumbnail to reorder |
| Bookmarks | F6 or the list button on the right rail |
| Find | Ctrl+F: type, match case, ▲ ▼, matches highlighted |
| Select text | Arrow tool in the floating palette, drag a rectangle, then Ctrl+C |
| Light / dark theme | Moon / sun icon, top right |
| Language | Globe icon, top right (French, English) |

## Annotating

The floating palette at the top left of the page offers: select (V), highlight (H), underline (U),
strikethrough (X), note (N). Drag a rectangle over text to apply a markup; click the page with the note
tool to type a comment. The **Comments** panel (F7) lists the annotations of the current page and lets you
delete them. Every annotation can be undone (Ctrl+Z).

## Editing content

The **Edit** category (and the second half of the floating palette) changes the page content itself,
not annotations: select object (E), add text (T), insert image (I), rectangle (R), ellipse (O), line (L).
With the select tool, click a text, image or path: a blue frame appears; drag it to move the object,
pull a corner handle to resize it, press **Del** to delete it, **F2** (or double-click) to edit a text
(content, size, colour). Click the page with the text tool to type new text (Helvetica / Times / Courier,
bold, italic, size, colour), with the image tool to pick a file (PNG, JPEG, BMP, GIF, TIFF, WebP; the
clicked point becomes the top-left corner), and drag a rectangle to draw a shape. The bar next to the
palette sets stroke colour, stroke width and fill for new shapes; **Apply to object** recolours the
selected object. Everything can be undone (Ctrl+Z) and is written into the PDF when saving.

With an object selected, the bar also offers: rotate left / right (90°), duplicate (Ctrl+Shift+D),
bring to front (Ctrl+]) or send to back (Ctrl+[), delete. Arrow keys nudge the object by one point
(Shift: ten points), Escape deselects it. Images keep their proportions when resized (Shift to free
them); other objects resize freely (Shift to lock).

Limits: no free-angle rotation; fonts limited to the 14 standard fonts; edited text keeps its original
font, which may not contain every character you type.

## Stamps, watermarks and page numbers

**Stamp / Watermark…** (Pages and Edit categories) places a text on every page or on the current page:
size, colour, bold, angle, opacity and position (centre or one of six edges). **Number Pages…** adds a
number from a template: `{n}` is the page number, `{total}` the page count (for example
`Page {n} / {total}`). Each stamp is a page object: select, move, delete and undo it like any other.

## Organizing pages

- **Organize pages** (F8): page grid with multi-selection (click, Ctrl+click, Shift+click), rotate, delete,
  insert from another PDF, move forward/backward, drag and drop.
- From the normal view: Ctrl+R / Ctrl+Shift+R rotate, Ctrl+Del deletes the current page, Ctrl+Shift+I
  inserts pages from another file.
- **Undo / Redo**: Ctrl+Z / Ctrl+Y (up to 50 steps).
- **Split document**: choose pages per file and a destination folder.
- **Extract selected pages**: creates a new PDF with the selection.
- **Combine PDF files**: pick files in order, then the output file; it opens afterwards.

## Converting

- **Create PDF from images** (PNG, JPEG, BMP, GIF, TIFF, WebP): one page per image, 150 dpi.
- **Export pages as images**: one PNG per page (150 dpi) in the chosen folder.
- **Optimize file size**: lossless recompression; the gain depends on the file and may be small.

## Security

- **Protect with password**: open password, owner password, permissions (print, copy, edit, annotate),
  AES-256 by default (AES-128 for older readers). Encryption is applied to the file when you save.
- **Remove protection**: available when the document was opened with its password.
- **Remove metadata**: strips the information dictionary and XMP metadata.
- **Document properties** (Ctrl+D): editable title, author, subject, keywords; version, size, encryption
  and permissions read-only.

## Saving

Ctrl+S saves to the open file, Ctrl+Shift+S to a new file. Writes are atomic: the original file is never
left half-written. A dot after the tab name signals unsaved changes; SuniPDF asks before closing or quitting.

## Command palette

Ctrl+K opens the palette: type a few letters, pick with ↑ ↓, confirm with Enter.

## Command line

`sunipdf-cli.exe --help` lists the commands: `info`, `render`, `text`, `search`, `merge`, `split`, `extract`,
`rotate`, `insert`, `move`, `images2pdf`, `export-images`, `optimize`, `protect`, `unprotect`,
`properties`, `bookmarks`, `set-metadata`, `strip-metadata`, `commands`. Output is JSON; exit codes
distinguish password required (3), invalid file (4) and unreadable file (5).

## Current limits

Single-page view; no OCR, no Office conversion, no digital signatures, no
direct printing. See `docs/ROADMAP.md`.
