# Known issues

[Français](KNOWN_ISSUES.fr.md)

## Build environment (this machine)

1. ~~No Build Tools~~ **Resolved on 2026-10-13**: Build Tools installed, GNU override removed, tests green under MSVC.
   The GNU + WinLibs fallback remains documented in `scripts/dev-env.ps1` for a machine without admin rights.
2. **Limited memory (14 GB, ~1.5 GB free during the session).** With 16 cores, `cargo` launches 16 `rustc`
   and the system reaches its commit limit: `0xC0000409` aborts and misleading errors
   ("only metadata stub found", ICE). Workaround: `cargo build -j 2` (or `CARGO_BUILD_JOBS=2`)
   and `debug = "line-tables-only"` in the dev profile.
3. (GNU fallback only) **The binutils shipped by rustup are not enough** for `raw-dylib` crates
   (`dlltool` without `as`) and the GNU rustc aborts under clippy on the `windows` crate: full WinLibs required,
   clippy limited to crates without a `windows` dependency.

## Dependencies

4. **wgpu/DX12 disabled**: `wgpu-hal` 30.0.1 does not compile with `gpu-allocator` 0.28 (D010).
   glow/OpenGL backend used. Possible impact: machines without a proper OpenGL driver (VM, ARM64).
   Lead: `cargo update -p gpu-allocator --precise 0.27.x` or a newer eframe.
5. **pdfium-render 0.9.4 targets PDFium build 7881**; newer builds (8100 as of 2026-10-12)
   probably work but are untested. Version pinned in `scripts/fetch-pdfium.ps1`.

11. ~~MSI not built~~ **Resolved on 2026-10-14**: standalone WiX 3.14 binaries (`third_party/wix314`, official zip, no
    installation or admin rights); `scripts/package-msi.ps1` detects them through `WIX_BIN`; x64 MSI produced and validated
    (ICE `smoke.exe`, administrative extraction).
12. **Local ARM64 build blocked by a missing Visual Studio component** (attempted on 2026-10-14): the Rust target
    `aarch64-pc-windows-msvc` is installed and the ARM64 PDFium downloads, but linking fails because
    the Build Tools only have the x64 and x86 tools (`VC\Tools\MSVC\<version>\bin\Hostx64\arm64` missing, ARM64 CRT
    missing). Installation (administrator rights required, refused during the unattended session):
    `& "C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe" modify --installPath
    "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools" --add Microsoft.VisualStudio.Component.VC.Tools.ARM64
    --passive --norestart`, then `scripts\package-portable.ps1 -Arch arm64` (from Git Bash).
    GitHub `windows-latest` runners have this component: `release.yml` builds ARM64 in CI.

## Application

A1. **Fluent icons**: read from `C:\Windows\Fonts\SegoeIcons.ttf` (or `segmdl2.ttf`). On a Windows without these
    fonts (rare case: stripped-down LTSC), the icons are replaced by text symbols, less elegant but legible.
A2. ~~Page-by-page display~~ **Resolved (SUNI-012b)**: continuous scrolling by default, single page as an option (F5).
A3. ~~High-zoom rendering limited to 16,384 px~~ **Resolved for the screen (SUNI-012b)**: tiled rendering up to
    3200% (rendered page side <= 131,072 px). Also fixes a latent crash: a page rendered in one
    piece could exceed the GPU's maximum texture size (`egui_glow` panic if the driver limits to
    8192 px); no texture exceeds `max_texture_side` any more. The 16,384 px limit still holds for
    `Document::render_page`, image export and `sunipdf-cli render` without `--region`.
A4. **Encryption and independent QA**: AES-128/256 is written by lopdf and reopened by PDFium; 40-bit RC4 is covered
    by a fixture. In 0.9.1, the shared helper explicitly writes AES-256 `/Encrypt /Length 256`, previously omitted and
    reported by qpdf, including after re-encryption. Synthetic qpdf checks and qpdf-generated AES fixtures are added
    to CI. Current outcomes are in `PROJECT_STATUS.md`; this is not a real-world/hostile PDF corpus.
A5. **The open dialog (`rfd`) is modal and blocks the UI loop** for the duration of the choice:
    native Windows behaviour, acceptable.
A6. **Content editing: editing an existing text reuses its original font.** If that font
    (often an embedded subset) does not contain a typed character, PDFium substitutes or omits it.
    Workaround: delete the object and add a new one (Helvetica, Latin accents supported).
A7. ~~Rotation in 90° steps, no system font~~ **Resolved (SUNI-031c)**: free rotation and system TrueType
    fonts. Group rotation/resizing is implemented for 0.10.0 with a single undo; validation is tracked in `PROJECT_STATUS.md`. Remaining: an object rotated by an angle that is not a multiple of 90° no longer has corner handles
    (resizing it would distort it); pivot = centre of the bounding box, so an asymmetric path may
    drift slightly with each successive rotation.
A8. **Reordering (send to back, send backward, bring forward) merges the page's content streams** (lopdf) before
    reordering: PDFium draws stream by stream. The PDF remains valid. **Duplicated or pasted objects are
    groups** (form XObjects): not editable as text, colour and opacity not modifiable (Opacity field
    greyed out), bounded to the box of the original page (an object that extended past its page is clipped).
A18. **System fonts embedded whole** (no subsetting): each font adds its size to the PDF
     (0.2 to 1.6 MB for common Windows fonts), once per document; after Undo / Redo,
     the next insertion embeds it again and the old copy remains orphaned until
     "Optimize". Only `.ttf` fonts with TrueType outlines are offered (`.otf` PostScript, `.ttc` and
     fonts whose licence forbids embedding: typed refusal `font_not_embeddable`). List read through
     `reg.exe` (fallback: fonts folder); a non-ASCII name may display badly (OEM code page). Rendering
     verified by PDFium; other-reader visual compatibility remains unverified. qpdf does not validate rendering.
A19. **Crop hides, it does not erase**: the content outside the frame stays in the file (this is not
     redaction; stated in the dialog). Mouse cropping in the preview is implemented for 0.10.0; numeric margins remain available.
A20. **Text on the page**: the input field shows the text with the interface font (Segoe UI, or
     monospace for Courier), not with the PDF font: approximate width and line breaks while
     typing. An existing rotated text is edited in a dialog. No paragraph editing (one line
     = one object).
A21. **Resizing in "Fit" mode**: the content is scaled (`FPDFPage_TransFormWithClip`),
     not the existing annotations or links (permanent note in the dialog).
A22. **Opacity of an existing image not readable**: PDFium 7881 does not return the colour (hence the alpha) of an
     image object; a transparent image keeps the previous value of the Opacity field in the object bar.
     Applying an opacity works (verified at rendering).
A23. **Displayed angle of objects on a rotated page**: an object's angle is that of its content on screen;
     a rectangle drawn in the PDF on a `/Rotate 90` page is therefore reported at 90° (CLI `objects`, Angle
     field). Corner handles remain available (multiple of 90°).
A24. **Object clipboard and system clipboard**: Ctrl+V only pastes objects if the marker written
     by the copy (the objects' text, or "[SuniPDF: n object(s) copied]") is still in the Windows
     clipboard; the Edit menu, the palette and the object bar paste without this condition. The object
     clipboard lives in memory for the session (lost when the application closes).
A9. **Engine tests serialized**: PDFium cannot be reinitialized within the same process after
    `FPDF_DestroyLibrary`; the tests share a single engine that is never destroyed (`tests/engine.rs`).
A10. **Distribution validation**: current tests, CI, package and release evidence are recorded in `PROJECT_STATUS.md`.
     Published 0.10.0 passed eight real portable update/failure/rollback scenarios and two disposable-runner MSI
     baselines covering installation/transactional rollback/upgrade/downgrade rejection/uninstall, preserving user data.
     Interactive flows, ARM64 hardware and physical printers remain separate checks.
A11. **All tools: 14 tools implemented for 0.9.0.** OCR, field creation and certificate signatures now have
     real commands. Automated and hardware coverage is recorded in `PROJECT_STATUS.md`.
A12. **Hub interface verified without automated pixel rendering**: no software renderer nor `egui_kittest`
     in the dependencies. The tests cover the data (table, registry, glyph contrast), tool
     switching, row geometry, the AccessKit tree and the focus ring (egui pass without a window); the
     actual rendering is checked through screenshots (`scripts/dev-screenshot.ps1`). The fallback glyphs (without Segoe
     Fluent Icons) of the new icons are not visually verified (see A1).
A13. **Tool bar in a narrow window**: the label widths are estimated before drawing; below the
     required width, icons only, then compact mode (name and "Leave tool" without label) and
     horizontal scrolling of the buttons (floating scrollbar, on hover; the vertical wheel does not
     scroll, Shift+wheel does). **Scrolling not demonstrated**: at 100% scale and at the minimum window
     width (720 px), the longest bar (Edit) fits in compact mode; scrolling is only
     reached with a larger Windows display scale and has been verified neither by a test nor by a
     screenshot.
A14. **Keyboard**: after a keyboard activation (Enter) of a hub row or button or of the quick
     access (Open, Save, Undo, Redo), focus is released so that the single-letter shortcuts stay
     active; Tab therefore starts again from the beginning. Standard egui buttons (tab bar, menu)
     only show a discreet focus hint; hub rows, cards and buttons have a 2 px ring.
     Esc acts in order (one Esc, one effect; `workspace::EscState::action`, tested): open menu or drop-down
     list, field or search, selected object, active tool. Two Esc received in the same
     frame (impossible from the keyboard, possible by injection) can still produce two effects.
A15. ~~Right rail in a short window: overlap~~ **Fixed (SUNI-012b)**: below ~700 px high, the top
     group (panels, view toggle, search) scrolls (wheel on hover) instead of overlapping the navigation;
     its last icons are then hidden until scrolled. Only whole icons are
     shown and a thin scrollbar, visible without hover, signals the rest (1000 x 560 screenshot).
A16. **CLI help for `stamp` and `page-numbers`**: clap replaces any `{n}` sequence in a help text
     with a line break (`clap_builder` 4.6, no escaping possible); the help therefore shows ": ⏎ = page
     number" instead of `{n}`. The `{n}` and `{total}` variables work; only the help text is
     truncated (raised by the hub review, not addressed).
A17. ~~"Modified" indicator after undo~~ **Fixed (review-fixes, D052)**: save point in
     the history; lost only if one edits after an undo or if the saved snapshot leaves the 50
     levels (the document then stays "modified" until the next save).
A29. ~~Extraction/split removed PDF protection~~ **Resolved in 0.11.0**:
     copies retain original encryption, passwords and permissions through the shared protected writer (D097 replaces
     D046). Output at the resolved source path is refused. CLI reports `encryption_removed: false` and adds `encrypted`.
     Existing document-permission policy is unchanged; current validation belongs in `PROJECT_STATUS.md`.
A30b. **AES-256 and passwords**: the password is SASLprep-normalized before encryption (special spaces
    mapped to spaces, ligatures decomposed), then limited to a valid UTF-8 prefix of 127 bytes. SuniPDF applies
    the same transformation on opening; another reader may require this effective form (for example
    "pass word" for an entry with a non-breaking space). Since 0.8.0, an opening password that becomes empty is
    rejected; restrictions require distinct effective passwords.
A30. **AES-128 and passwords**: only printable Latin-1 is accepted (0x20–0x7E, 0xA1–0xFF except 0xAD;
     accented letters included). PDFDocEncoding-specific symbols outside Latin-1 (€, œ, Œ, ligatures,
     typographic dashes and quotes…) are refused (`password_not_latin1`) although lopdf encodes them: PDFium
     receives the password in UTF-8 and only retries in Latin-1, so SuniPDF would not reopen the file
     (verified by `review_aes128_latin1_passwords_round_trip_through_pdfium`). Choose AES-256 for these
     passwords. AES-128 uses the first 32 encoded bytes, padded according to the PDF format; since 0.8.0,
     owner/user differences are checked on these effective bytes.
A31. **Images → PDF**: JPEGs are embedded as is except for EXIF orientations 5 and 7 (rotation +
     mirror), which are decoded; PNG, TIFF, BMP, GIF and WebP are decoded in memory and stored as compressed pixels
     (Flate), hence heavier than the original. A very large batch of PNGs remains limited by the machine's memory.
     A CMYK JPEG embedded as is is converted to RGB by PDFium at rendering (pure red → 238,29,35):
     normal conversion of a DCT stream, not an inversion.
A32. **Added text**: a tab counts as four spaces (no PDF font has a tab glyph).
     The Comments panel names 17 annotation subtypes; the others (sound, video, 3D, watermark,
     redaction…) are shown as "Other annotation".
A33. **Save point**: "modified" clears when undoing back to the saved state, unless an
     edit followed an undo (the saved state is then unreachable) or the saved snapshot
     has left the 50 history levels (D052).
A34. **Properties and PDF/A**: 0.9.1 preserves PDF/A identification (`pdfaid:part`, `pdfaid:conformance`) and unrelated
     XMP properties/namespaces when updating the four standard editable properties. Malformed XML, DTDs and unreadable
     streams are clearly refused, rather than silently replacing the packet. Preserving identification does not certify
     PDF/A conformance. A dedicated conformance validator and broader PDF/A corpus remain future work; current executed checks
     are recorded in `PROJECT_STATUS.md`.
A25. **Continuous view: limits (SUNI-012b)**.
     - Selection follows PDFium logical order across pages and scrolls near viewport edges; intended order
       of arbitrary untagged columns is not inferred. Text markup keeps per-page rectangular gestures.
     - Scrolling **without animation or inertia** for "go to" and the keyboard (direct jump).
     - **Pinch** (touchpad or touch screen): same path as Ctrl + wheel (`zoom_delta`), untested
       (no test on this machine).
       Ctrl + wheel verified by screenshot (`dev-screenshot.ps1 -Wheel`).
     - A **tile rendering already started is not interrupted**: when scrolling fast, the cancellation latency is
       the duration of one tile (measured: 1.2 ms per 1024² tile on `many-pages.pdf`, more for a heavy page).
     - Tile stitching: glyphs crossing a tile edge may differ by one anti-aliasing pixel
       from the one-piece rendering (121 pixels out of 1.9 million measured, D044).
     - Two-page/Book views share the existing caps; view-only rotation/full screen/per-tab history are implemented for 0.10.0. Editing returns to upright view; native interactive coverage remains separate.
A26. **"Fit" in continuous scrolling on a document of mixed sizes**: the zoom is shared by all the
     pages and fits the **largest** page (D041); a portrait page among landscape pages is therefore
     narrower than the screen. In single page, the fit follows the current page. Two-page/Book fit uses the
     widest spread and tallest page at one shared zoom.
A27. **Very long documents**: positions in `f32`; the zoom is capped so that the total height stays
     <= 8,000,000 logical px ("Maximum zoom reached for this document" message). A 5000-page Letter
     document is thus limited to about 150%.
A28. **View memory**: bounded but notable. Page textures 192 MiB (excluding screen textures), engine
     cache 256 MiB (tiles and thumbnails), 256 thumbnail textures at most in both the sidebar and Organize
     grid. Pending thumbnail requests also have a 256-entry cap; stale results are discarded.
     These caps are regression-tested on a 5,000-page grid. The following measurement predates 0.8.3 and
     covers the reading view: measured in a debug build, 500 pages,
     300 "Page Down" then End then 300 "Page Up": private memory stable around 810-830 MB (WorkingSet
     438 MB) after 175 MB at opening; no growth beyond (NVIDIA OpenGL driver, system copies of the
     textures included). Caps as constants (`view.rs`, `worker.rs`), no user setting.
A35. **Handwritten signature = image without cryptographic proof**: another editor can copy or delete it.
     CLI `add-image` reports `digital_signature: false`. CMS/PFX signing is a separate Use a certificate
     workflow (A44); the application warns before rewriting a signed PDF.
A36. **Signatures saved unencrypted**: PNG in `%APPDATA%\SuniPDF\signatures` (`signature-<ms>.png`,
     `initials-<ms>.png`), protected only by the Windows profile ACLs: anyone with access to the session can
     reuse them. Countermeasure: untick "Save on this computer for reuse" (nothing is written) or delete the thumbnail
     (bin, two clicks). Only the 12 most recent per type are listed; sorting assumes timestamps of
     the same length (13 digits, until 2286).
A37. **Pad and import**: constant-thickness stroke (egui does not pass on pen pressure), raw polyline
     without smoothing; background removal by a **global luminance threshold** with a soft ramp (a very unevenly
     lit photo would need an adaptive threshold: judge on the preview, lower or raise the threshold, or prefer a
     transparent PNG); imported images limited to 16,384 px per side and 512 MiB decoded (refused when reading
     the header, same 512 MiB bound for Insert Image, Replace Image and Images → PDF), then reduced
     to 1600 px per side; the placed date is kept inside the page over an **estimated** width (0.556 em per
     character, Helvetica digits), not measured; no **typed** signature (handwriting font: depends on
     font embedding, A7); initialling every page at once is not offered (Shift+click page by
     page); full library: the dialog body scrolls and its floating bar gets a 10 px gutter
     (the content tightens by as much on the frame following the bar's appearance).
A38. **AcroForm forms**: XFA is read, never filled/flattened; JavaScript, calculations and actions never run.
     0.10.0 implements text/checkbox/radio/combo/list creation, XFDF export and strict bounded JSON/XFDF/FDF value import.
     Import validates every field/value before changing anything; unknown or rejected values refuse the whole GUI import.
     Panel changes remain pending until Apply or Save, then use one undo/reload; Discard restores the current values.
     On-page confirmation is immediate. Space/Enter activate focused widgets; choice popups receive keyboard focus.
     Names are unique, without dots/control characters, numeric rectangles stay inside the page (minimum 4 points).
     Radio buttons are arranged vertically; option labels are exposed in the panel, not inserted as page text.
     FDF supports normal xref tables and text/name/array values only; streams, actions, attachments, rich text and
     FDF export are unsupported. XML DTDs, processing instructions, unknown elements, duplicate names and excessive
     size/depth are refused; referenced document paths are never opened. Input cap: 16 MiB.
     0.11.0 delivers editable-combo free text in the panel/on page; local, CI and packaged checks passed.
     In development for 0.12.0: create editable combos (predefined choices still required) and map radio `/Opt`
     export values separately from appearance states. Malformed/duplicate mappings keep the affected field read-only;
     so do malformed raw `/Kids`, duplicate widget references and non-UTF-8 appearance names. Unchanged read-only
     values can be exported and imported, including literal `Off` or empty strings. Filling another field preserves
     raw `/V` and `/AS`; PDFium may regenerate a missing appearance and remove inferred read-only status once valid.
     No-`/Opt` radios retain state-name values. Validation/publication pending. Existing fields' editable flags cannot
     be changed; mouse-drawn creation is unavailable. Rich text becomes plain text; password fields never stored/exported.
     Partial flattening can leave orphan fields, and non-Link annotations without appearances may disappear. Unicode
     field appearances can use a non-embedded system font, so other-machine rendering can differ. Narrator and
     third-party-reader interoperability remain unverified. Current executed evidence belongs in `PROJECT_STATUS.md`.


A39. **Advanced annotations (annotations-advanced slice, D067–D076)**: **free text in Helvetica WinAnsi** only
    (Latin accents, € and common punctuation; any other character shows as "?" in the appearance, `/Contents` keeps
    the exact text; no bold, italic, rich text `/RC`, CJK or right-to-left) and the wrapping width is computed with the
    Helvetica table (2 % margin); **one ink stroke = one annotation** and every operation (stroke, move, style)
    costs a full snapshot + lopdf + reload round trip: imperceptible on an ordinary PDF, about a second on a file of
    tens of MB; **reply threads and statuses** written according to ISO 32000-1 but **never tested in Acrobat, Foxit
    or Chrome** (replies without icon: empty appearance + `F 4`), no "Marked" state model; **detailed reading** of
    annotations (panel, selection) is requested on demand, per page, and re-reads the whole document; an existing
    link does not change target (delete then recreate), named destinations are not resolved, `/XYZ` destination
    reduced to the page, Launch / JavaScript / GoToR actions are never executed; **resizing and styling reserved for
    SuniPDF's own annotations** (prefixed `/NM`): those from another producer can be moved, commented and deleted
    (highlights and notes can also be recoloured); no eraser, polygon, cloud, stamp, attachment, dashed border; caps:
    64 strokes of 4,000 points, width ≤ 50 pt, link area ≥ 6 pt; **verified in screenshots** (`dev-screenshot.ps1`): ink,
    selection of a SuniPDF annotation (frame, four handles, floating bar) then moving and resizing with the mouse,
    moving another producer's annotation (frame without handles), dialogs (text box, link, address confirmation),
    Comments panel with a thread, light and dark themes; **not replayed in screenshots**: the text box placed after
    its dialog, editing by double-click, keyboard arrows, resizing an ink stroke, a line or a text box (all covered by
    the engine tests and the pure tests of `annot_adv.rs`); synthetic structural qpdf checking is added for 0.9.1
    (A4); reader interoperability remains unestablished.
A40. **Redaction (redaction slice, D077–D082)**: content under an area is **removed** (objects deleted, file rewritten
    without any earlier revision, verified by reading it back), with these limits:
    1. **Granularity**: a text object touched by an area leaves the text layer entirely; what extends beyond the area
       stays visible as an image (200 dpi patch), neither selectable nor searchable. Same for a partly covered image
       or drawing.
    2. **Text in a scanned image without OCR**: invisible to "Find Text to Redact…" (draw the area). An OCR layer
       offset from the image can escape a drawn area: the "Convert redacted pages entirely to images" option removes
       it in every case.
    3. **Embedded fonts**: if the font of the redacted text is still used elsewhere, its glyph subset stays in the file
       (it reveals characters that were used, not their order or position).
    4. **Not changed**: bookmarks (a title may repeat the redacted text), attachments, JavaScript, named destinations,
       content of unmarked pages; a `/Redact` annotation placed by other software is handled as an ordinary
       annotation (removed only when it lies under an area).
    5. **Form fields**: a field with a widget under an area disappears everywhere (all its widgets, all pages); `/XFA`
       is then removed.
    6. **Accessibility tags**: removed for the whole document when "hidden information" is checked (default); kept
       otherwise, with the risk that an alternate text describes the redacted content.
    7. **Digital signatures**: any existing signature becomes invalid (full rewrite).
    8. **Memory and disk**: undo and suspended tabs keep previous states in RAM. Successful redaction immediately
       refreshes the current recovery snapshot (or removes it if refresh fails), but older crash-session copies may
       still contain unredacted content. The original, backups and deleted copies are not securely wiped from storage.
    9. **Remaining assumption**: PDFium object bounds enclose the object's ink. For text they are enlarged by 30 % and
       checked at run time (residual text under an area = error, document unchanged); for images and drawings they
       are geometric.
    10. **Refused documents**: a content stream that does not decode strictly (lopdf) gives `redaction_incomplete`
        without changing anything; retry with "page as image".
    11. **Deleting an object with the Edit tool is not redaction**: PDFium may leave the object orphaned or the
        resource in the file; only the Redact tool sanitises and rewrites the document.
    12. Areas are **cleared whenever the document changes** (rotation, deleted page, undo…); drawn areas have no
        accessibility node of their own (their count is announced in the status bar).
    13. Not verified: real scans/heavily tagged PDFs and pages with tens of thousands of objects (lopdf's
        `prune_objects` is quadratic). Synthetic qpdf checking does not replace this corpus (A4).


## 0.9.0 feature limits

Executed validation results are tracked in `PROJECT_STATUS.md`.

A41. **Recovery**: snapshots every 60 seconds and on tab suspension can miss more recent changes after a forced stop.
     Unconfirmed dialog text is not a PDF edit. Protected snapshots stay encrypted, but filenames/paths and page counts
     are present in JSON metadata. No passwords are saved; restoring requires the password again. Recovery never replaces the original.
A42. **Windows printing**: raster output up to 300 dpi in bounded bands, fitted to the printable area; no vector output
     or professional colour guarantee. The driver handles copies/orientation. A running native operation can delay
     cancellation. Actual Microsoft Print to PDF output was reopened and checked with unsaved edits while source
     bytes/objects stayed unchanged. Physical printers and the interactive printer dialog remain unverified.
A43. **Local OCR**: English/French only, fast Tesseract models; accuracy varies by scan and language. Pages containing
     any extractable text are skipped, even if that text layer is poor. Images capped at 16 Mpx and 8192 px/side,
     TSV at 16 MiB, 50,000 words/page, 120 seconds/page; very large pages refused. No semantic table/column reconstruction.
     Cancellation/failure commits no pages; success is one undo step. Other readers still need interoperability checks.
A44. **Digital signatures**: detached CMS `adbe.pkcs7.detached`/`ETSI.CAdES.detached` over saved bytes;
     incremental PFX/P12 signing into a new copy, no encrypted PDF signing. Cryptographic validity does not establish
     identity trust: chain, revocation, AATL/EUTL, PKCS#11, trusted timestamps and LTV are not implemented. PDF signing time
     is declared by the signer. Editing/rewriting may invalidate signatures. Acrobat/Foxit interoperability and PAdES
     qualification are not established; do not present the result as a qualified signature.

A45. **0.10.0 image compression**: lossy; only supported 8-bit DeviceRGB/DeviceGray images up to 32 million pixels
     are decoded. Mask/soft-mask resource graphs, profiles, custom Decode/DecodeParms and other unsupported images
     remain unchanged. If the document would grow, the exact input bytes are returned. Maximum-side/JPEG controls
     do not guarantee a target file size. Signed or modification-restricted PDFs are refused; output is a new copy.
A46. **0.10.0 batches**: sequential compression or offline English/French OCR, at most 128 PDFs and 256 MiB/input.
     Existing outputs are never replaced; completed files remain if a later file fails or cancellation occurs.
     Password prompts are not provided per batch file; protected files that cannot open without a password are refused.
     Cancel takes effect at engine checkpoints; a codec or native call already running may delay it.
A47. **0.10.0 data-safety/corpus coverage**: injected storage failures and a killed partial-save child exercise original
     preservation. They do not simulate every physical disk/power-loss failure. Four public NIST PDFs supplement the
     synthetic corpus: 31 synthetic qpdf checks and all four real documents (744 pages) passed for 0.10.0.
     They do not establish broad customer/hostile/large-file compatibility; executed evidence is in `PROJECT_STATUS.md`.

## 0.11.0 published changes and limits

These additions are published in [0.11.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0). Local, CI, package and upgrade regression
gates passed; public-download/live-updater checks also passed; evidence and remaining limits are recorded in `PROJECT_STATUS.md`.

A48. **Page-label reading**: labels are optional metadata, not editable page numbering. Blank/unreadable labels,
     labels over 4096 UTF-8 bytes and labels containing control characters fall back to physical indices.
     Exact unique labels take precedence over numeric input; `#N` selects physical page N. Duplicate labels require
     a physical index. No semantic rebasing is performed after structural changes: ranges can stay attached to
     physical positions; extraction can omit labels. CLI indices remain physical and one-based.
A49. **Destination navigation**: internal links/bookmarks support `/XYZ`, `/Fit` and usable `/FitH` positions,
     with per-tab position/zoom history. Other fit modes retain page-only navigation. PDFium cannot distinguish
     a `/FitH` null position from zero; both use the page-only fallback. Unreadable/invalid view details also fall
     back to the page. Missing `/XYZ` axes and zero/missing zoom retain viewport values; explicit zoom is clamped
     to application limits. This does not establish Acrobat/Foxit or native screen-reader interoperability.
A50. **Form-read failures**: an error is kept distinct from an empty form, and the panel offers an explicit retry.
     Pending drafts and staged values are retained while the read fails. A waiting import/clear operation resumes
     only after a successful read; repeated automatic retries are suppressed.
