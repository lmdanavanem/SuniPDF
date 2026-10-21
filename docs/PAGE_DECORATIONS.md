# Page decorations

## Reusable presets (0.39.0)

In Watermark or Page numbers, choose a saved preset to load its settings into the dialog. Edit the draft, inspect its settings and click the usual Apply button to change the PDF. Choosing a preset does not change the document. Cancel discards the pending document operation.

To keep a configuration, enter a new name and click **Save preset**. Select an existing preset and click **Delete preset** to remove that preference. These explicit buttons save preferences immediately; cancelling the document dialog afterwards does not undo an explicit preset save/delete. Names are unique ignoring case; an existing name is refused, so use a new name or delete it first. Failed writes preserve the active preferences.

Up to 16 presets with names of 64 characters are stored locally in the existing settings file. Text watermarks, image watermarks, page numbers and six-zone headers/footers are supported. Page selection is deliberately not saved: check all/current page each time. Text is limited to one line and 256 characters per zone/template. Image presets retain the original file path; keep that file available. Choosing a preset does not decode the image; Apply reports a missing/invalid image and preserves the document. Neither image bytes nor signing identities are stored by this feature.

Older settings retain defaults; malformed preset entries are skipped without resetting other valid preferences. Settings files above 1 MiB are refused on load and defaults are used. Writes above that limit are refused before replacing the existing preferences. Presets are desktop preferences; no CLI preset import/export, cloud synchronization, macro or credential store is added. The packaged CLI gate retains the image/header operations; headless preference/dialog tests cover persistence and cancellation. Native Windows interaction remains separate qualification.

## Headers and footers (0.38.0)

Open **Pages → Page numbers**, then choose **Headers and footers**. Fill any of the six zones: header left/center/right and footer left/center/right. Set text size and margin; choose all pages or the current page. `{n}` means the absolute page number and `{total}` the full document page count. One Undo removes the whole operation.

```powershell
sunipdf-cli headers-footers input.pdf --header-left "Report" --header-right "Internal" --footer-center "{n} / {total}" --pages 1-3 --size 10 --margin 28 --out decorated.pdf
```

Each zone has one third of the available width, with an 8-point gap between columns. Text is aligned to its zone and fitted to displayed rotation/crop coordinates. Text that exceeds its column or half the usable page height is refused; text is never silently shrunk or truncated. All selected pages are committed together, with at most six editable foreground text objects per page. Existing content can overlap these zones; inspect before saving.

Limits: 256 characters on one line per zone, no control characters, 4–72-point text, 0–1000-point margins and at most 1000 selected pages. At least one non-empty zone is required. Modification restrictions, allowed encryption, credential inputs and copy-only CLI output use the same guards described below. Empty zones are omitted. No automatic date, section counter, first/odd/even page rule or PDF tagging is added. Font support follows the existing PDF text engine; complex-script shaping is not newly qualified.

## Image watermarks (0.37.0)

Open **Pages → Watermark**, choose **Image** and select a PNG, JPEG, BMP, GIF, TIFF or WebP file. Set width as a percentage of the displayed page, opacity, angle, position and margin. Apply to every page or the current page. One Undo removes the whole operation. Save the PDF to keep the result.

The watermark is an editable foreground image, with its original transparency multiplied by the chosen opacity. It keeps its proportions and fits inside both page dimensions, including rotation and cropping. Foreground content may cover existing content; inspect the result before saving. Existing text watermarks remain available.

```powershell
sunipdf-cli image-watermark input.pdf --image logo.png --pages 1-3 --width-percent 35 --opacity 0.25 --angle=-45 --position center --out marked.pdf
```

Positions: `center`, `top-left`, `top-center`, `top-right`, `bottom-left`, `bottom-center`, `bottom-right`. Pages are 1-based; duplicate selections are applied once. `--password-env NAME` and `--password-stdin` support protected documents without credentials in the command line. The command always writes a new copy; an existing destination or the original path is refused even with `--overwrite`.

The image is decoded locally, including EXIF orientation. Limits are 64 MiB encoded, 4096 pixels per side, four million image pixels, 1000 selected pages and 64 million image pixels across selected pages. Invalid settings, fully transparent images and modification-restricted documents are refused. The in-memory document is replaced only when every selected page succeeds. Allowed AES-128/AES-256 protection and both credentials are preserved.

The generated-fixture gate checks CLI geometry, independent rendered pixels, protection, refusals and qpdf. It does not qualify native picker/keyboard/Narrator interaction, customer documents, physical printing or PDF/A/PDF/UA conformance. No background layer, tiled image, animation, signature verification or DRM is implied.

## Image Stamp annotations and atomic text edits (0.40.0)

Open **Comment → Rich annotations**, choose **Image stamp**, select a local image and set the displayed-page rectangle and opacity. Preview the actual candidate before Apply; changing any input requires a fresh preview. Cancel/Escape preserves the document. One Undo removes the annotation. The image keeps its proportions and alpha and is embedded into a real Stamp annotation, so the original image file is no longer needed after Apply. Common annotation tools support moving, replies and thread deletion; generic resize/restyle is unavailable. Recreate it to choose a different rectangle or image.

```powershell
sunipdf-cli image-stamp input.pdf --image logo.png --rect 40,40,240,240 --opacity 0.5 --page 1 --author "Reviewer" --out stamped.pdf
```

Creation requires annotation permission. The command writes a new copy, refusing originals and existing destinations even with `--overwrite`; environment/stdin credential inputs remain available. Allowed AES-128/AES-256 protection and both credentials are retained. Images are decoded locally with EXIF orientation and bounded to 64 MiB encoded, 4096 pixels per side and four million pixels. Fully transparent, oversized or invalid images are refused. No source path or image filename is embedded. This is a visual annotation, not a cryptographic signature or verified identity.

Text watermarks and page numbering now preflight all selected pages and margins and commit only when every result fits. Non-finite/negative margins, out-of-range pages or text that exceeds the displayed page margins are refused atomically, without silent shrinking or truncation. Duplicate page selections are applied once.

New [calibrated local measurements](MEASUREMENTS.md) use the same guarded rich-annotation preview flow. Cloud/collaboration are deferred; local tools remain free and open source.
