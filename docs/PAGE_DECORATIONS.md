# Image watermarks

Open **Pages → Watermark**, choose **Image** and select a PNG, JPEG, BMP, GIF, TIFF or WebP file. Set width as a percentage of the displayed page, opacity, angle, position and margin. Apply to every page or the current page. One Undo removes the whole operation. Save the PDF to keep the result.

The watermark is an editable foreground image, with its original transparency multiplied by the chosen opacity. It keeps its proportions and fits inside both page dimensions, including rotation and cropping. Foreground content may cover existing content; inspect the result before saving. Existing text watermarks remain available.

```powershell
sunipdf-cli image-watermark input.pdf --image logo.png --pages 1-3 --width-percent 35 --opacity 0.25 --angle=-45 --position center --out marked.pdf
```

Positions: `center`, `top-left`, `top-center`, `top-right`, `bottom-left`, `bottom-center`, `bottom-right`. Pages are 1-based; duplicate selections are applied once. `--password-env NAME` and `--password-stdin` support protected documents without credentials in the command line. The command always writes a new copy; an existing destination or the original path is refused even with `--overwrite`.

The image is decoded locally, including EXIF orientation. Limits are 64 MiB encoded, 4096 pixels per side, four million image pixels, 1000 selected pages and 64 million image pixels across selected pages. Invalid settings, fully transparent images and modification-restricted documents are refused. The in-memory document is replaced only when every selected page succeeds. Allowed AES-128/AES-256 protection and both credentials are preserved.

The generated-fixture gate checks CLI geometry, independent rendered pixels, protection, refusals and qpdf. It does not qualify native picker/keyboard/Narrator interaction, customer documents, physical printing or PDF/A/PDF/UA conformance. No background layer, tiled image, animation, signature verification or DRM is implied.
