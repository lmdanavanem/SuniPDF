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

Release qualification is in progress. Exact main/tag CI, CodeQL, packaged regressions, public downloads
and updater results will be recorded after verification; 0.33.0 remains the public stable release.

## Limits

Dictionary findings are not a safety verdict, malware scan, signature verification or conformance check.
Native UI/Narrator, hardware printing/GPU/ARM64, independent update authenticity, code signing, broad
hostile corpora, coverage-guided fuzzing and named third-party reader qualification remain open under
the [1.0 criteria](RELEASE_1.0_CRITERIA.md). Projects edits remain deferred at the owner's request.
