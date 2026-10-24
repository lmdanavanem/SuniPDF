# Prepare a visual signature

Visual signatures are pictures of handwriting. They do not authenticate an identity or verify document integrity. Certificate signatures remain a separate function.

## Draw

Open **Fill & Sign**, choose a signature or initials, then draw. **Smooth strokes** is optional and initially off. Toggle it to compare the same raw strokes with a curved preview. Undo and Clear continue to operate on the original strokes. Preview and placement use the same curve geometry, including single-point dots and endpoints.

A draft accepts at most 20,000 sampled points and 256 strokes. When a limit is reached, placement is disabled and an explanation is shown. Undo a stroke or clear the draft before continuing; a truncated draft is not silently accepted as a finished signature.

## Import a photo or scan

Choose an image in the signature dialog. Enable **Remove background**, then optionally **Reduce paper shadows** for dark, thin ink on light paper with uneven illumination. Adjust the threshold and inspect the preview before placing it. The preview is reduced for display, while placement uses the same prepared full-resolution pixels. Changing any cleanup option invalidates the previous preview.

The original decoded image is retained for changing settings. The source file is not edited. Existing image decoding, orientation correction and a 1600-pixel working-size limit still apply. Cleanup can remove faint gray marks or affect broad strokes and textured backgrounds; disable it when the preview is unsuitable. It is not a general photo cutout tool.

## Command line

```powershell
sunipdf-cli prepare-signature-image photo.jpg --background adaptive --threshold 200 --out signature.png
sunipdf-cli prepare-signature-image scan.png --background threshold --out signature.png
sunipdf-cli prepare-signature-image handwriting.png --background keep --out signature.png
```

`keep` is the default. `threshold` uses the global luminance threshold; `adaptive` adjusts for local paper brightness first. The threshold accepts 60–250. PNG output is required, transparent padding is trimmed with a six-pixel margin, and an empty result is refused. The command runs without opening a PDF or initializing the rendering engine. It returns JSON describing the output and explicitly reports `cryptographic_signature: false`.

The input must be a regular file of at most 64 MiB at the size precheck; decoder dimension and allocation limits also apply. An existing output requires `--overwrite`. Replacing the input path is refused even with that flag. Failures use the existing JSON error and exit-code conventions. A PNG requested with this command is a normal unencrypted user file; the application's optional DPAPI-protected signature library is a separate storage path.

## Qualification

Core and desktop tests cover ink/alpha preservation, blank and oversized images, raw-stroke preservation, drawing limits and preview-setting binding. The packaged x64 CLI is additionally checked with generated images using hash-pinned, test-only Pillow: pixel preservation, shadow cleanup, all eight JPEG EXIF orientations, input/output guards, PDF placement/rendering and independent qpdf structure validation. The image reader is not shipped as an application dependency.

These checks do not qualify native pointer/tablet interaction, Narrator, real customer photos or ARM64 hardware. Release evidence records which checks actually ran.
