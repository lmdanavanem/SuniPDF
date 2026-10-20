# SuniPDF 0.34.0

[Français](RELEASE_0.34.0.fr.md)

## Changes

- Inspect the saved PDF from the Properties dialog or the new `inspect` JSON command. Seven bounded
  dictionary inventories identify JavaScript, automatic/external actions, embedded files, XFA, multimedia
  and signature declarations without running actions, exposing payloads or changing source bytes.
- Preserve partial results with an explicit incomplete status and CLI exit `2`. Support existing
  environment/stdin credential inputs and encrypted files. Display the saved revision's SHA256.
- Reject stale UI responses after edits, tab changes and closing. Require saving before inspecting;
  never mistake an unsaved metadata draft or a rewritten PDFium snapshot for the saved revision.
- Stream the saved-file fingerprint shared with signature verification instead of allocating another
  complete file buffer; reject changed lengths and hashes before using the captured revision.
- Add independently written PDF/xref fixtures and a fourteen-scenario packaged-CLI regression gate.
  Add the actual public 0.33.0 ZIP/MSI to upgrade baselines, with verified hashes/product properties.

[Usage and bounds](PDF_INSPECTION.md). No new runtime dependency is added. English remains the default,
with matching French strings.

## Validation

[0.34.0 published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.34.0) under `lmdanavanem`, source `521618f4aab6cd17f7f829bc739bb3f7a93c9c5e`.

[Main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342371499) and [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39342385646) passed: **600 tests, zero failures**, 10 default ignored, **1108 EN/FR keys**, **14 packaged inspection cases**, **26 portable scenarios**, **20 MSI baselines/100 logs**. CodeQL Rust/Python/Actions passed with zero open alerts. Existing qpdf, independent-reader, PDF-corpus, mutation and Office gates remain included.

Six public files and `dist/` copies verified by SHA256. The preserved 0.32.0 harness, with its unchanged updater module verified, downloaded and checked the **26,741,242-byte** x64 ZIP without installation on the workstation.

Structural inventory without a safety verdict or signature/PDF/A/PDF/UA validation. Packages remain unsigned; native/hardware qualification and 1.0 criteria remain open. Projects edits stay deferred at the owner’s request, with no new synchronization claimed.

[Detailed evidence](qa/RELEASE_0.34.0.json).

## Limits

Dictionary findings are not a safety verdict, malware scan, signature verification or conformance check.
Native UI/Narrator, hardware printing/GPU/ARM64, independent update authenticity, code signing, broad
hostile corpora, coverage-guided fuzzing and named third-party reader qualification remain open under
the [1.0 criteria](RELEASE_1.0_CRITERIA.md). Projects edits remain deferred at the owner's request.
