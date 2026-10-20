# SuniPDF 0.35.0

## Changes

- Optional smoothing for drawn visual signatures, using the same curve geometry for preview and placement while retaining the original points for Undo and comparison.
- Optional local paper-shadow reduction for imported signatures, with a preview bound to every cleanup setting and placement using those exact prepared pixels.
- A `prepare-signature-image` command to create a separate PNG using keep, global-threshold or adaptive-background modes, without opening a PDF.
- Explicit drawing budgets and input/output guards, plus independent packaged image, EXIF, PDF-rendering and qpdf regression evidence.
- Upgrade checks retain previous release baselines and add the published 0.34.0 packages.

[Usage and limits](VISUAL_SIGNATURE_PREPARATION.md).

## Validation

[0.35.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.35.0) under `lmdanavanem`, source `aab39beaffa88a7a970441f647498920e432bf27`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347959736) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39347970752) passed: **606 tests, zero failures**, 10 default ignored, **1114 EN/FR keys**, **20 image cases**, **14 PDF-inspection cases**, **27 portable scenarios**, **21 MSI baselines/105 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Existing qpdf, independent readers, fonts, DX12 software rendering, PDF, mutation and Office gates remain included.

Six public files and `dist/` copies verified by SHA256; image and inspection suites were also rerun on the downloaded CLI. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,752,605-byte** x64 ZIP without installing it on the workstation.

Visual signatures provide no identity proof; cleanup may remove faint ink and needs preview review. Native pointer/tablet/Narrator interaction and customer photos remain unqualified. Packages remain unsigned, ARM64 is cross-built only and 1.0 criteria stay open. Projects edits remain deferred at the owner’s request, with no fresh synchronization claimed.

[Detailed evidence](qa/RELEASE_0.35.0.json).

## Limits

Visual signatures do not authenticate an identity or provide cryptographic integrity. Cleanup targets thin dark ink on light paper and may remove faint marks; review the preview. Native mouse/tablet/Narrator interaction and customer-photo coverage still need qualification.

Windows packages remain unsigned without an independent update-publication key. ARM64 is cross-built without execution on ARM64 hardware. Physical printing, broader hostile corpus/fuzzing coverage, named third-party reader checks and PDF/A/PDF/UA conformance remain open under the [1.0 criteria](RELEASE_1.0_CRITERIA.md). Projects edits remain deferred at the owner's request.
