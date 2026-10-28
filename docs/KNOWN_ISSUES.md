# Known limitations

Current scope: [SuniPDF 0.91.2](RELEASE_0.91.2.md). This page lists supported boundaries
and remaining qualification gaps. See [project status](PROJECT_STATUS.md) for executed
checks and the [roadmap](ROADMAP.md) for planned work.

## Register workflow qualification

The new register-batch corpus covers 113 checks, 35 commands, 18 cases and 16 refusals; the saved-text corpus covers 388 checks, 45 commands, eight cases and four refusals. Actual plain PDF graphs, saved text glyph mappings and rendered pixels are independently checked. Encrypted register output retains independently inspected QPDF objects and permission dictionaries; ephemeral passwords are not retained for a second independent decryption. Four additional offscreen dialog scenes qualify recorded rendering and control states, not native mouse/keyboard, Undo/Redo interaction or physical accessibility. Exact field filtering adds no numeric ranges or regular expressions and has no separate packaged corpus. Foreign, embedded or arbitrary text fonts, unsupported semantics and incompatible text-box geometry remain refused. All affected backlog cards retain their remaining acceptance criteria. Rectangle rotation does not create persistent geometric constraints. Saved annotation capture retains its supported native appearance and image bounds. Overlay setups retain source hashes and configuration, not PDF data, rendered rasters or credentials; automatic registration and layered PDF output remain open. Formula conversion supports only the existing geometric unit families; each conversion rounds to six decimal places and sub-micro values can round to zero. Source quantities keep their exact unit guard. Every affected backlog card retains its remaining acceptance criteria. Circles and ellipses use 128 editable polygon vertices, about 0.0402% below ideal analytic area before coordinate rounding. Segments use two stored endpoints and linear quantities; none of these shapes has a persistent analytic constraint. The ellipse corpus verifies 327 checks, 62 commands, 27 cases and 19 refusals across eight PDFs/1024 vertices. The segment corpus verifies 359 checks, 67 commands, 29 cases and 20 refusals across nine PDFs/18 endpoints. Both retain actual PDF/QPDF, appearance and stored-quantity checks across four cropped rotations and five units. No new native shape mouse/keyboard, Undo/Redo, physical survey or external-reader qualification is claimed. Independent X/Y calibration, permanent constraints, analytic curves and broader interoperability remain open; AEC-008 remains In progress.

[Current release evidence](RELEASE_0.91.2.md).

## Platforms, installation and updates

- macOS targets Apple Silicon, with a binary-checked macOS 12 minimum and installed execution on macOS 15.7.9. The DMG is ad hoc signed, not notarized. Physical Macs, interactive Finder opening and clipboard interoperability remain unqualified. OCR, native printing, certificate signing, protected Office import, persistent encrypted visual-signature storage and automatic update installation are unavailable. See [macOS](MACOS.md).

- Windows packages are unsigned. Update downloads use GitHub HTTPS and same-channel SHA256;
  an independent release-signing key and production publisher certificate are not configured.
- Windows ARM64 is cross-built and inspected, without execution on physical ARM64 hardware.
- Automated Windows UI evidence uses offscreen rendering. Physical GPU/window behavior,
  touch/pen input, Narrator and interactive installer/updater dialogs still need native checks.
- Ubuntu supports 22.04/24.04 amd64. Its checks use Xvfb/software rendering and removal/reinstall
  of the same candidate, plus a real upgrade between pinned public releases through the candidate helper. Physical desktops remain unqualified.
- Ubuntu versions through 0.73 open the release page and require manual package installation.
  Version 0.74 adds direct downloads and authorized installation; one manual upgrade is needed
  to obtain that flow. Graphical authentication-agent behavior needs native qualification.
  Native printing, certificate signing/trust checks, protected Office import and persistent
  encrypted visual-signature storage remain Windows-only. See [Ubuntu](UBUNTU.md).
- Windows printing rasterizes at up to 300 dpi. Microsoft Print to PDF output has been checked;
  physical printers, interactive dialogs, vector output and professional color fidelity are unqualified.

## Measurements, quantities and large documents

- Calibrated values depend on the chosen reference and drawing revision. Verify a second known
  dimension. Private real-project plan qualification is deferred at the owner's request.
- Snapping uses eligible vector endpoints and straight-line intersections. Raster scans and
  text/image bounds are not snap geometry; curves contribute endpoints only. Unsupported clips,
  excessive geometry or dense intersections can decline snapping and require manual points.
- Named regions require explicit selection for ambiguous overlaps or crossing geometry.
  Changing a region affects future drawing; saved measurements retain their independent scales.
- Resource-dependent legacy version-1 page scales require explicit recalibration or removal.
  Supported stale region records can be reset; unknown/malformed records are not overwritten.
- Batch recalibration is bounded to 512 pages, 4096 records and 128 MiB per document; regions
  remain unchanged. Regions are limited to 64 per page and measurements to 128 vertices.
  Reference points must fit every selected page. Full guards: [Plan workspace](PLAN_WORKSPACE.md).
- Search and inventory cancellation is cooperative between pages. An in-progress PDF engine
  call can delay it. Very long documents can have a lower maximum zoom to bound layout coordinates.
- Render and thumbnail caches are bounded, but native memory and driver allocations are additional.
  CI model benchmarks do not establish complete frame latency or performance on physical devices.

## Formula and saved-group limits

Typed numeric fields retain their declared units. Report formulas accept explicit `convert(expression, unit)` for matching length, area and volume dimensions; counts, angles and dimensionless values cannot be converted. The source quantity retains its exact declared unit. Every conversion rounds half away from zero at six decimal places; sub-micro values can round to zero. Missing inputs, mismatches and overflow remain explicit errors. These calculations do not modify the PDF, stored prices or source quantities. [Formula limits](QUANTITY_TABLE.md).

Saved-group capture accepts reproducible native shapes, supported standard and image stamps with verified RGB/optional alpha resources, and supported native rich-text FreeText using twelve standard font/style variants when appearance and resources reproduce exactly. The existing 2 MiB appearance-graph budget still applies. Legacy simple text, foreign appearances, embedded or arbitrary fonts, replies and extra semantics remain refused; incompatible text geometry is refused to preserve wrapping. Unsupported selected members are never silently dropped. [Library limits](ANNOTATION_LIBRARY.md). Remaining native interaction and independent-reader criteria stay open.

## Editing, annotations and forms

- Complex shaping, bidirectional paragraph editing, variable/CFF2/color fonts and arbitrary
  editable Form XObjects are unsupported. Missing glyphs or unsafe embedding are refused.
  See [font embedding](FONT_EMBEDDING.md) and the [feature matrix](FEATURE_MATRIX.md).
- Paragraph editing is bounded reflow, not general document-layout reconstruction. Complex
  untagged columns may not follow intended reading order during selection/copying.
- Grouped paste can create a non-editable Form XObject. Explicit editable paste accepts only
  supported native objects. Some non-orthogonal resize and unknown-opacity cases remain limited.
- FreeText supports a uniform style, not arbitrary rich-text spans or foreign rich appearances.
  Replies, review states and appearance preservation need broader independent-reader testing.
  See [rich annotations](RICH_ANNOTATIONS.md).
- XFA is read-only; PDF JavaScript, calculations and active actions are not executed. FDF import
  accepts bounded value-only structures; FDF export and changing existing editable-field flags
  are unsupported. Malformed fields remain read-only and flattening can leave orphan fields.
- Some form appearances can depend on system fonts. Keyboard behavior has automated coverage;
  screen-reader and broader cross-reader behavior require separate qualification.
- Detailed engine refusal reasons have no stable error code yet. The desktop shows them in
  English inside a translated sentence, including annotation-library capture refusals.
  Overlay, register-field, saved-filter and library messages raised by the desktop are fully translated.
- Listing library capture candidates fully verifies native text up to a fixed budget per page
  (about 64 KiB of text). Larger pages defer that check to capture, which still refuses an
  appearance that does not reproduce.
- Overlay setups must be saved to a folder on a local drive; mapped network drives are refused.

## Redaction, saving and recovery

- Cropping or deleting an object is not redaction. Persistent redaction marks leave the content
  present until Apply. Read the [redaction guarantees and limits](REDACTION.md) before applying.
- Complex fonts, OCR, images and nested graphics can use raster-patch fallback, losing selection
  in the affected remainder. Image-only text is not discoverable by text search. Misaligned OCR
  may require converting the entire redacted page to an image.
- Hidden-data cleanup removes supported metadata and accessibility structure. Turning it off
  can retain hidden duplicates of redacted content. A form field intersecting a redaction is
  removed with all its widgets; existing digital signatures are invalidated by the rewrite.
- Undo, original files, backups and older recovery sessions can retain unredacted data; deletion
  is not secure erasure. Bounded residual checks do not replace a broader hostile-document corpus.
- Recovery snapshots normally occur every 60 seconds and on tab suspension. More recent edits,
  unconfirmed dialog text and unfinished drawing drafts may be lost after a forced stop.
- Protected recovery PDFs stay encrypted, but JSON recovery metadata includes paths/page counts.
  Passwords are not saved and must be supplied again. Recovery never replaces the original.
- Storage-failure regressions cover injected failures, not every physical disk or power-loss case.

## Signatures, security and conformance

- A visual signature is an image, without cryptographic proof. Image cleanup can remove faint
  ink; review the preview. Pen pressure is unsupported. Windows per-user encrypted signature
  storage depends on the user profile and does not prevent reuse by someone in that session.
- Certificate signing and trust are distinct from visual signatures. Encrypted signing is refused;
  network trust retrieval requires explicit consent, and unknown revocation never means trusted.
- Timestamp creation, authenticated archival LTV, qualified-signature assessment, AATL/EUTL and
  PKCS#11 are unsupported. See [signature security](SIGNATURE_SECURITY.md).
- PDF permissions are application controls, not DRM. Saved-file inspection is bounded inventory,
  not exhaustive payload analysis or proof of authenticity.
- Preserving PDF/A identification metadata does not establish conformance. Recorded veraPDF
  and PDFium/Poppler checks cover specified fixtures and operations only. Human accessibility
  and broader reader interoperability remain open; no PDF/A converter or PDF/UA tagging editor is supplied.

## OCR, conversion and batches

- Local OCR supports English/French with fast models. It skips pages with any extractable text,
  including poor text layers, and does not reconstruct semantic tables/columns. Very large inputs
  or pages exceeding resource/time bounds are refused.
- Office import needs a supported local converter on Windows. Editable exports support bounded
  layouts; PDF-to-PPTX uses raster pages. Arbitrary Office layout reconstruction is unavailable.
- Lossy compression changes only supported image streams, may leave unsupported images untouched,
  and cannot guarantee a target size. Signed or modification-restricted documents are refused.
- Compression/OCR batches are sequential and bounded. Existing outputs are preserved; successful
  earlier files remain if a later file fails. Native calls can delay cancellation.

The remaining [1.0 acceptance gates](RELEASE_1.0_CRITERIA.md) are open. Historical bugs and
old validation counts are retained in the [status archive](archive/PROJECT_STATUS_PRE_0.60.md),
not repeated here as current failures.
