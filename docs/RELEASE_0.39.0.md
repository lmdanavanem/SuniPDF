# SuniPDF 0.39.0

## Changes

- Local named presets for text/image watermarks, page numbers and six-zone headers/footers.
- Load a draft, save under a unique name or delete explicitly; at most 16 presets.
- Atomic preference activation, bounded settings loading and isolation of malformed preset entries.
- Preserve current document/page scope and retain independent package, security and migration regression.
- Office version probes use headless startup and a cancellable 30-second cold-start budget; conversion and security guards remain enforced.

[Usage and limits](PAGE_DECORATIONS.md).

## Validation

Final-source Windows CI, CodeQL, package qualification and publication remain pending. Headless regressions cover four modes, backward compatibility, invalid values, persistence failure and cancellation. Native Windows interaction remains unqualified.

## Limits

Image presets reference the original path; keep the file available. Scope is checked each time. Explicit preset save/delete persists independently of cancelling the document operation. No macro, cloud sync, CLI preset import/export or signing-identity storage. Packages remain unsigned and ARM64 is cross-built only. Other [1.0 criteria](RELEASE_1.0_CRITERIA.md) stay open. Projects edits remain deferred at the owner's request.
