# Page decorations

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
