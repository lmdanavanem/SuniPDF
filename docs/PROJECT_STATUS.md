# Project status - updated 2026-10-16 (0.21.0 published stable Latest)

[Français](PROJECT_STATUS.fr.md)

Rule: a ticked box = code present **and** command executed with a result observed on this machine
(Windows 11 Pro 26300, Rust 1.99.0, target `x86_64-pc-windows-msvc`, PDFium 7881).

## Main/unreleased P1 fixes

Both existing P1 bugs are implemented and locally validated; **0.21.0 remains the published Latest**.
The release source, packages and its validation below are unchanged. GitHub CI/CodeQL integration checks
are pending; these fixes are not included in the published packages and no version change is claimed.

| P1 item | Implemented behavior | Current state |
|---|---|---|
| Missing glyphs in existing text | Keep the original font; atomically refuse unsupported text and reopen the correct editing draft without changing the PDF | In Progress; local checks passed, CI/CodeQL pending |
| Flattening non-Link annotations without a usable appearance | Preserve raw non-widget annotation graphs, attachments and shared references; keep widget/drawable-appearance flattening, exclude preserved annotations from the count and commit through an atomic scratch document | In Progress; local checks passed, CI/CodeQL pending |

Local evidence: **476 passed tests, zero failures, seven default ignored across 37 suites**; formatting,
all-target Clippy with `-D warnings`, **912 EN/FR keys** and **84 qpdf checks** passed. Four licensed PDFs
(**744 pages**) passed in **60.419 seconds**, sampled peak child memory
**330,194,944 bytes**; all inputs stayed unchanged. The regression first
reproduced accepted unsupported text, the stale add-text draft and lost flattening annotations, then passed
with the fixes. The full native flatten-preservation regression passed separately.

The [GitHub project](https://github.com/users/lmdanavanem/projects/1) retains 42 drafts: Bugs (8), Roadmap (23),
Suggestions (11); 39 Todo, these two In Progress and the 0.21.0 security item Done. APIs, dependencies and
CLI commands remain unchanged. Completion awaits GitHub source checks; no release publication is planned for this lot.

## 0.21.0: Code Scanning remediation (published stable Latest)

Explicit native signer-certificate null checks and malformed-CMS regression; release root
`contents: read` with `contents: write` restricted to Publish (D112). XML 1.0 readers use `ParserConfig`,
retain bounds and reject DTD events; UTF-8/UTF-16 XMP/XFDF regressions passed. Deterministic fixtures
and complete scanning coverage are preserved.

471 tests, zero failures, seven default ignored; formatting/Clippy, 912 EN/FR keys, 75 qpdf checks,
four-document/744-page corpus and helper passed. CI, 19 portable cases (15 upgrades/four guards), 13 MSI
baselines/65 logs, same-source ARM64, six staged/public files, `dist/` copies and production updater verified.
CodeQL fixed #3 and #119; zero alerts remain open. All 115 criticals were individually dismissed with
evidence by `lmdanavanem`: 114 `used in tests`, #13 `false positive`. No folder or query is excluded.

The [GitHub project](https://github.com/users/lmdanavanem/projects/1) contains 42 prioritised drafts: Bugs (8), Roadmap (23), Suggestions (11),
with saved category filters. At 0.21.0 publication the security item was Done; current implementation
status for the other items is tracked in PROJECT_STATUS.md.
See [Code Scanning review](CODE_SCANNING_REVIEW.md)
and [Project status](PROJECT_STATUS.md). The 53 CLI commands are unchanged.

[Published as stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.0) under `lmdanavanem`, release `415338013`,
source `f006b7dd027fd5a3084c2bfffc2fceaa1b66be46`. Local validation: 471 passed tests, zero failures, seven default ignored;
formatting/Clippy with warnings denied, 912 EN/FR keys, 75 qpdf checks, update-helper self-test and four PDFs/744 pages.
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648173) and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957647839/job/117164444654) passed:
19 portable cases, 13 MSI baselines and 65 verified logs. Six staged/public files and `dist/` copies match API/SHA256 digests; complete package checks passed.
The production updater verified the 19,846,878-byte Latest x64 ZIP in 1.76 seconds without installing on the host.
[CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648825) fixed #3/#119 on this source; zero alerts remain open after individual triage of 115 criticals.
Detailed evidence and limits are in [Project status](PROJECT_STATUS.md).

Local corpus: 67.304 seconds, sampled peak child memory 329,601,024 bytes; CI corpus: 32.195 seconds,
sampled peak child memory 320,372,736 bytes. Sources unchanged. ARM64: 891 seconds,
artifact `12071194275`; upload/post-cache verified before cancelling the duplicate workflow.
CodeQL fixed #3 at `2026-10-16T19:58:04Z` and #119 at `2026-10-16T19:49:12Z`; neither was dismissed.
The analysis still contains 115 Rust test findings with verified triage; zero open alerts does not mean zero findings.
Three new CMS/XML regressions passed. XML notices are accurate; the unchanged MIT license matches XML 1.4 byte for byte.
The [GitHub project](https://github.com/users/lmdanavanem/projects/1) has 42 read-back drafts, saved Bugs/Roadmap/Suggestions filters and the 0.21.0 security item Done.
Other items are planned; no repository issue or comment was created.

| Package | Bytes | SHA256 |
|---|---:|---|
| `SuniPDF-0.21.0-win-x64-portable.zip` | 19846878 | `7D5DC36F467970891E5BD094791A8E0B0D8AFFDD7BFC36B106E9EE0409B2A956` |
| `SuniPDF-0.21.0-win-arm64-portable.zip` | 18434585 | `91B99CFADD4E2547A42D915B653A7075EF5485984E1EE650726CDA7C2D5E7F31` |
| `SuniPDF-0.21.0-win-x64.msi` | 19386368 | `4FEAA78595E289D381DAEBEFE0E036E36142A0CCDD54DCC31F34C36730BD6E85` |

MSI ProductCode: `{BFFBDEB1-7C9A-4D74-B97C-5C9E69684FE9}`; version `0.21.0`, language `1033`.

Packages are unsigned; native interactive UI, printers, Narrator, ARM64 hardware, independent readers and exhaustive fuzzing remain separate QA.

## 0.20.0: on-page FreeText creation and editing (published; Latest at the time)

The implemented 0.20.0 lot creates and edits FreeText annotations directly on the page (D111/A39).
Click or draw the existing box, type in an anchored multiline editor and use explicit **Add** / **Save** /
**Cancel** with the existing size, colour, background and border controls. The existing AddFreeText /
UpdateFreeText worker operations keep validation, protection and one Undo step; no new dependency,
annotation type or CLI command is added (53 commands remain).

The draft stays bound to its original page. Zoom remains available; conflicting navigation waits for
confirmation/cancellation. Document revision/tab/tool/workspace changes cancel stale editing. Pending
open/save/mutations and concurrent drafts are guarded; validation or worker failure retains typed text
for retry. Helvetica WinAnsi appearance and downward wrap growth remain unchanged. The editor draft
exists only in memory until confirmation and has no crash recovery.

Implementation and independent review passed. Local workspace: 468 passed tests, zero failures, seven
default ignored; formatting, all-target Clippy with warnings denied and 912 matching EN/FR keys passed.
The 75-check qpdf gate, four-document/744-page real corpus and update-helper self-test passed. Windows CI,
18 portable cases, 12 MSI baselines/60 logs, same-source ARM64, all six staged/public files and the production
updater passed. **0.20.0 was published as stable Latest at the time**; [Project status](PROJECT_STATUS.md) records the executed evidence.

Observed 0.20.0 local evidence so far: 75 qpdf checks passed. Four licensed NIST/NBS documents
(744 pages) passed in 58.260 seconds, with sampled peak child memory of 331,800,576 bytes; source files
stayed unchanged. The update-helper path/checksum/rollback self-test passed. The 468-test workspace includes
eight new desktop regressions and the extended native-worker FreeText create/update, Undo/Redo,
save/reopen, protected-document and failure regression. Headless UI cases cover real mouse rectangle
coordinates, long text with visible Cancel, explicit submission, lifecycle guards, failure retention and
blocked search navigation. Quick rectangle drags now use the release position.

[Published as stable Latest at the time](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.20.0) under `lmdanavanem`,
release `415294715`, on 2026-10-16 at 19:09:50 UTC. Immutable tag/source:
`54054b101282a7688dde778b24090af6d6e1e6a6`.
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38950447258) passed all 32 steps in 751 seconds,
including the same workspace/translation/qpdf gates, 18 real portable cases (14 upgrades and four guards),
and 12 MSI baselines with 60 verified installer logs. Its four-document/744-page corpus took 34.172 seconds,
with sampled peak child memory of 320,299,008 bytes. The [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38950446650/job/117140185977)
passed in 832 seconds, including artifact upload/post-cache; only the duplicate workflow was then cancelled.
ARM64 artifact `12068203388`; the successful main-CI x64 packages and this verified ARM64 artifact were published.

All six staged/public files and `dist/` copies match GitHub API digests and SHA256 sidecars. Complete enhanced
checks passed on staged and unauthenticated public downloads: architecture, English version metadata, seven
exact icon frames, OCR/models/licenses, MSI/ZIP payload, forms, encryption, signatures, compression, labels,
links, dashed shapes, grouped ink, offline French OCR and multiline FreeText geometry/style. The five FreeText
readbacks cover plain PDFs and AES-128/AES-256 with both user and owner passwords, qpdf checks and unchanged
source files. Independent conversion checks passed 35 subprocesses with all thirteen sources unchanged.
The production updater downloaded the public Latest x64 ZIP (19,841,217 bytes) and verified its SHA256 in
1.33 seconds using the already validated workspace test executable, without installing on the host. Real
portable/MSI installation and upgrade scenarios passed separately on the disposable Windows CI runner.

| File | Bytes | SHA256 |
|---|---:|---|
| `SuniPDF-0.20.0-win-x64-portable.zip` | 19841217 | `CBAB56A4FD8AAF15F798B94EDDF88C62785D78C11ED7E26CDC4D81266AD77EAD` |
| `SuniPDF-0.20.0-win-arm64-portable.zip` | 18429703 | `CD1A32354441EA5B992032FD48C4B209EE4A953D9F4AB46E7CAB06ACD58EE040` |
| `SuniPDF-0.20.0-win-x64.msi` | 19378176 | `9D87F80B559EA76DBCF92F6EE8867994B72C82AE8AFC690EE6425ECAB6551B75` |

Three SHA256 sidecars complete the six published files. MSI ProductCode
`{90AA23B4-8929-4B93-A77B-2995BD483DC3}`, language 1033. No new dependency, annotation type or CLI command;
53 commands remain. Packages are unsigned. Native mouse/pen/touch, Narrator, physical printers, ARM64
hardware and third-party readers remain manually unverified. Unsubmitted FreeText stays in memory and
has no crash recovery; Helvetica WinAnsi, plain text and downward wrap growth retain their existing limits.

The [Code Scanning review](CODE_SCANNING_REVIEW.md) covers all 117 open alerts, including all 115 critical
alerts. No production secret was confirmed among those critical findings; 114 are test fixtures and #13 is
intentional empty-password PDF authentication. The high pointer trace is a reviewed false positive; the
medium release-workflow permission declaration remains a future hardening item, with current read-only
repository defaults verified. Alerts were not dismissed; no CMS fuzzing or global security guarantee is claimed.

## 0.19.0: mouse placement of existing form fields (published; Latest at the time)

The selected lot adds **Draw on page** to **Prepare a form** for text, checkbox, radio, drop-down and list-box
fields. The numeric dialog hides while drawing, then reopens with name, choices and flags retained. Drawing
alone does not change the PDF; explicit **Create Field** uses the current validation/protection and one Undo.
Finite rectangles are clamped to the original page, at least four PDF points per dimension. Reverse dragging
is allowed; tiny gestures stay in drawing mode for retry. Escape cancels without writing. Document revisions,
tab/tool/workspace changes cancel stale placement; zoom/navigation remain available, with a release fallback
when the original page is hidden. Crosshair/preview and excluded widget overlays support placement (D110/A38).

Implementation and independent review passed. Local workspace: 460 passed, zero failed, seven default ignored;
909 matching EN/FR keys. Eight new UI/autosave tests and the extended five-type native-worker regression passed.
The local qpdf gate passed 75 checks; the update-helper self-test passed path/checksum/rollback checks.
Four licensed NIST/NBS documents (744 pages) passed the local real-corpus gate in 76.762 seconds, with sampled
peak child memory of 331,276,288 bytes; all source files stayed unchanged.
Formatting (`cargo fmt --all -- --check`) and all-target Clippy with `-D warnings` passed.
[Published as stable Latest at the time](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.19.0) under `lmdanavanem`, release
`415214602`, on 2026-10-16 at 17:35:42 UTC. Immutable tag/source: `903181fee3bba5794b39714106e5814bca753fb3`.
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38938525308) passed all 32 steps in 844 seconds,
including the same test/translation/qpdf gates, 17 portable cases (13 upgrades and four guards), and 11 MSI
baselines with 55 verified installer logs. Its four-document corpus took 47.721 seconds, with sampled peak
child memory of 321,085,440 bytes. The [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38938524836/job/117100003633)
passed in 680 seconds, including upload/post-cache; the duplicate workflow was then cancelled. ARM64 artifact
`12062064636`; successful main-CI x64 packages and this verified ARM64 artifact were published.

All six staged/public files and `dist/` copies match API digests and SHA256 sidecars. Enhanced public package
checks passed for architecture, English version metadata, exact icon frames, OCR/models/licenses, MSI/ZIP
payload, forms, encryption, signatures, compression, labels, links, dashed shapes, grouped ink and offline
French OCR. Independent conversion checks passed 35 subprocesses with all thirteen sources unchanged.
The production updater downloaded the public Latest x64 ZIP (19,837,875 bytes) and verified its SHA256 using
the already validated workspace test binary, without installing on the host. Real portable/MSI update and
install scenarios passed separately on the disposable Windows CI runner.

| File | Bytes | SHA256 |
|---|---:|---|
| `SuniPDF-0.19.0-win-x64-portable.zip` | 19837875 | `C713DC4495D024E0BCBC715A29832FD5C49B164EFE165907496E8A1F9690E3BB` |
| `SuniPDF-0.19.0-win-arm64-portable.zip` | 18428126 | `D40E8EA991674FF790F356BA054D8FADC01CB5C596305FD99E5C33B13C712313` |
| `SuniPDF-0.19.0-win-x64.msi` | 19378176 | `E1CD1E6894B09A107238F96DFEFB5F7075E94D31DCC4FDFE3CCDF01100EE0EEE` |

Three SHA256 sidecars complete the six published files. MSI ProductCode
`{AA82CB16-BEC5-40FD-83D3-C86852E44676}`, language 1033. Core/CLI APIs and 53 commands remain unchanged;
no dependency or field type is added. Packages remain unsigned. Native mouse/pen/touch, Narrator, physical
printers, ARM64 hardware and third-party readers remain manually unverified. The field draft exists only in
memory until **Create Field**. When Single view hides its origin page before release, review the last known
rectangle in the reopened dialog before creating the field.

## 0.18.0: grouped freehand drawings and readable CLI templates (published; Latest at the time)

The desktop groups multiple freehand strokes on one page into one Ink annotation through the existing
core/worker edit. The desktop draft has explicit Finish/Cancel, one undo, bounded strokes/points and guards
against losing unsubmitted strokes when editing, saving, switching documents or closing. Worker failure
retains the draft for retry or cancellation. English remains the default, with matching French controls.
Up to 64 strokes and 4,000 points per stroke are retained, with visible limit messages. Colour, width and
opacity apply to the entire draft. Navigation, zoom and view modes retain its original-page binding;
global pointer-release finalization prevents a stroke becoming stuck after a Single-page jump. Deferred
form changes and recovery markers must complete before drawing can begin. The draft exists only in memory
until Finish: it is not included in crash recovery. Implementation/local validation passed.

[Published as stable Latest at the time](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.18.0) under `lmdanavanem`, release `415140702`,
source `7e52de82244faf9a020ed3ed233176775c007e97`. Local gates passed: 452 workspace tests (zero failed, seven default ignored),
formatting/Clippy with warnings denied, 907 matching EN/FR keys, 75 qpdf checks and the update-helper self-test.
Four licensed NIST/NBS documents (744 pages) passed in 50.796 seconds, sampled peak child memory
329,445,376 bytes; all sources stayed unchanged. Six new headless UI tests, the extended native-worker
grouped Undo/Redo/failure regression and the CLI help regression passed.
[Windows CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082734) and the [same-source ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38926082934/job/117057854520) passed,
with sixteen portable cases, ten MSI baselines and 50 verified MSI logs. The CI corpus took 44.494 seconds,
sampled peak child memory 320,299,008 bytes. All six staged/public files, API digests/sidecars and enhanced
package checks passed: five grouped-Ink readbacks (plain/AES-128/AES-256 with both passwords), four literal-help
checks and existing conversion, forms, labels, links, dashes, compression, OCR and MSI payload checks.
Verified copies are in `dist/`; the production updater verified the 19,834,488-byte Latest x64 ZIP without
installing it on the host. Executed evidence and remaining limits are in `PROJECT_STATUS.md`.

The sixteen portable cases cover twelve upgrades, including 0.17.0, and four guards. Ten MSI baselines:
0.8.0, 0.9.1 and 0.10.0 through 0.17.0; 50 install/rollback/upgrade/downgrade/uninstall logs verified.
The ARM64 job succeeded in 612 seconds, including upload/post-cache; the duplicate workflow was then cancelled.
ARM64 artifact `12056658063`. Published 2026-10-16 at 16:03:33 UTC.

Five independent qpdf readbacks confirm one Ink annotation and three strokes, including a dot, with InkList
lengths `[6,4,2]`, for three plain/AES-128/AES-256 outputs reopened with both passwords. Public conversion checks
include 35 EXIF/transparency/budget/DPI subprocesses and thirteen unchanged sources.

| File | Bytes | SHA256 |
|---|---:|---|
| `SuniPDF-0.18.0-win-x64-portable.zip` | 19834488 | `8D1C76930C7D929D289745DC20897C91418DF63873B6E66B2566CA6AF289A08C` |
| `SuniPDF-0.18.0-win-arm64-portable.zip` | 18422022 | `89A814C2395CAD59D761B02CC8ACC7C9E2629B23FA8D3DB22D5B7C1AB90815FB` |
| `SuniPDF-0.18.0-win-x64.msi` | 19378176 | `D3DABCB44438E4E005D33A5D4544ACD6C3514DE10BF7BB07B745123E4CD7DDC4` |

Three SHA256 sidecars complete the six files; `dist/` copies were independently matched against API,
public and staged files. MSI ProductCode `{2C188A13-8DBB-475A-B48D-78BCC8D058EA}`, language 1033.

Native pen/touch, interactive dialogs, printers, Narrator, ARM64 hardware and third-party readers remain unverified. Packages are unsigned; unfinished ink drafts have no crash recovery.

## 0.17.0: lossless oriented JPEG conversion and raster batch checks (published; Latest at the time)

[SuniPDF 0.17.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.17.0) was stable **Latest** at the time under
`lmdanavanem`, release ID `415074108`, with six assets. Immutable tag/source:
`5785de282e126fb4666b28fb83340d0e9c6e9acf`. [Windows x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916382731)
and the same-source [ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38916383380/job/117024804268)
passed. The duplicate release workflow was cancelled after ARM64 completed upload and post-cache successfully;
the successful main-CI x64 packages and verified ARM64 artifact were used. Earlier releases remain available.

Images to PDF now embeds JPEGs without pixel decoding or recompression for all eight EXIF orientations,
including mirrored rotations 5 and 7. Page dimensions follow the oriented image. The shared core API and
existing desktop/CLI callers are retained; transparency is preserved for decoded image formats.

Before creating any bitmap, the conversion checks all image headers and rejects a batch whose non-JPEG images'
cumulative estimated raster cost exceeds 512 MiB. Each such image charges the greater of its decoder byte count
and width × height × 4, using checked arithmetic. The actual loading decoder is checked again before decoding.
This is not a process-memory ceiling: native PDFium buffers, encoded JPEG metadata, temporary copies and the
serialized PDF remain outside the estimate. Physical page dimensions must be finite and positive.

Budget overflow/excess returns `image_batch_too_large`; the desktop shows an actionable EN/FR error and keeps
the current document and unsaved changes. Existing output guards and atomic file writes remain. No dependency,
option, command or tool was added; there are still 53 CLI commands and 896 matching EN/FR translation keys.

Local validation passed: 445 workspace tests, zero failures, seven default ignored; formatting, Clippy with
warnings denied, 896 matching EN/FR keys, 74 independent qpdf checks and four licensed NIST/NBS documents
(744 pages; 48.187 seconds, peak sampled child working set 330,264,576 bytes). The update-helper self-test passed.
Independent code review found no blocker. Native and CLI tests cover all eight orientations, exact JPEG streams,
asymmetric corners, malformed EXIF fallback, transparency, cumulative/overflow/header bounds, extreme DPI,
unchanged inputs and existing-output guards. The desktop response test keeps the modified document on a refusal.

An independent Python/Pillow/qpdf check passed against the debug CLI: nine exact JPEG stream comparisons,
32 oriented JPEG corners, four PNG corners with a soft mask, three qpdf checks, two header-only budget refusals
before pixel decoding, ten invalid-DPI refusals and positive finite geometry after saving at f32 maximum DPI.
All 13 input hashes stayed unchanged. This is debug-runtime evidence, not released-package or manual GUI proof.

Windows CI passed the same tests/translation/qpdf gates. Its four-document corpus completed in 45.791 seconds,
with peak sampled child working set 321,413,120 bytes. All 15 real portable scenarios passed: upgrades from
0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0, 0.13.0, 0.14.0, 0.15.0 and 0.16.0, plus bad checksum,
wrong architecture, locked-file rollback and the real 120-second close-timeout/concurrent-update guard.
Nine MSI baselines (0.8.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0, 0.13.0, 0.14.0, 0.15.0, 0.16.0) passed installation,
transactional rollback, upgrade, downgrade rejection, payload/user-data checks and uninstall on disposable
Windows runners. All 45 MSI logs have the expected final exit 0 or 1603. Evidence is in the main job log and
downloaded portable/MSI/corpus artifacts.

| Package | Bytes | SHA256 |
|---|---:|---|
| x64 portable ZIP | 19,825,743 | `55D8CBC224D1D6D9582AD9402F51B7F76CEC84C93A0FC90AFE91C43471ACF1D4` |
| ARM64 portable ZIP | 18,413,678 | `CBFA86B5D1EEE8B83A82EE97D4105BCE7539E19A04CB6916938D5678FBB373E4` |
| x64 MSI | 19,369,984 | `C59DE003132EFC077B6D5F0C70B7FC8DAE06616D6F4E4B12DDD45BCB479A603E` |

All six staged files passed checksums, architecture, English version/icon resources, OCR/licenses, MSI/ZIP
payload and existing feature checks. The independent conversion checks passed on the packaged x64 CLI,
including exact JPEG streams, oriented corners, PNG soft masks, budget/DPI refusal and input/output preservation.
All six unauthenticated public downloads match API digests, sidecars and staged hashes; all enhanced checks
passed again on the public files. Verified copies are in `dist/`. The production updater downloaded and
verified the Latest 19,825,743-byte x64 ZIP without installation on the development machine.

Native interactive dialogs, ARM64 hardware, physical printers, Narrator and third-party readers remain unverified;
packages remain unsigned. The 512 MiB raster budget does not cap total native/process memory. Full PDF rewrites
can invalidate digital signatures; the existing save warning applies.

## 0.16.0: dashed shape annotations (published as Latest at that time)

[SuniPDF 0.16.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.16.0) was stable **Latest** at that time under
`lmdanavanem`, release ID `414934190`, with six assets. Immutable tag/source:
`6cd7843734f3924e87033d8b05539d493ccbcb95`. [Windows x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377537)
and the same-source [ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38897377641/job/116961002589)
passed. The duplicate release workflow was cancelled after the ARM64 job completed successfully;
the successful main-CI x64 packages and verified ARM64 artifact were used. Earlier releases remain available.

This release adds a Dashed border control for rectangle, ellipse, line and arrow annotations, and `annotate --dashed`
with the same 6-point dash / 3-point gap preset. Existing solid defaults remain. The core stores the actual
dash array so selecting, recolouring, moving, resizing and Undo/Redo preserve it; only an explicit checkbox
toggle replaces a custom pattern. Border dictionaries and appearance streams use the same pattern and phase 0;
arrowheads remain solid and fills continuous. Unsupported annotation kinds refuse nonempty dash patterns.

Pattern parsing and generated appearances are bounded; malformed or unsupported authored border styles refuse
regenerating edits rather than becoming solid. No new dependency, command or tool is added. English remains
the default, French remains available. The CLI still has 53 commands.

Local validation passed: 439 workspace tests, zero failed, seven default ignored; formatting, Clippy with
warnings denied, 895 matching EN/FR keys, 73 independent qpdf checks and four licensed NIST/NBS documents
(744 pages; 53.542 seconds, peak sampled child working set 331,517,952 bytes). Windows CI passed the same
test/translation/qpdf gates; its real corpus completed in 47.628 seconds with a peak sampled child working set
of 321,306,624 bytes. Native tests cover custom/legacy/default/empty patterns, numeric and density bounds,
rotated/cropped pages, atomic refusals, unchanged forms/links/content/source and both AES-128/256 opening roles.
Real-worker and accessible EN/FR headless checks cover creation, selection, custom-pattern preservation,
restyling, movement, resizing and Undo/Redo. Manual native interaction is not inferred from those tests.

Fourteen real portable scenarios passed: upgrades from 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0,
0.13.0, 0.14.0 and 0.15.0, plus bad checksum, wrong architecture, locked-file rollback and the real 120-second
close-timeout/concurrent-update guard. Eight MSI baselines (0.8.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0, 0.13.0,
0.14.0, 0.15.0) passed installation, transactional rollback, upgrade, downgrade rejection, payload/user-data checks
and uninstall on disposable Windows runners. All 40 MSI logs have the expected final exit 0 or injected-failure
1603. Evidence is in the main job log and downloaded portable/MSI/corpus artifacts.

| Package | Bytes | SHA256 |
|---|---:|---|
| x64 portable ZIP | 19,821,719 | `F03B7BB283A58414F1F26FD707FE120231B1B9502762B5588F75C97DC4526DD0` |
| ARM64 portable ZIP | 18,408,941 | `ECD3BE771391105DDC119E8F5A59415349C1B1EE9C1A393773DCDE9EBC67A75E` |
| x64 MSI | 19,365,888 | `5B4043B271C36C2CBBFBFD3FAA419ED96CB2D151CD32250C43C8DC121FB1328D` |

All six staged files passed checksums, architecture, English version/icon resources, OCR/licenses,
MSI/ZIP payload and existing feature checks. Packaged dashed borders cover four shapes on plain and
owner-opened AES-128/256 files, both passwords, 20 independent qpdf checks, 20 distinct solid/dashed render
pairs and unchanged input hashes. All six unauthenticated public downloads match API digests, sidecars and
staged hashes; the enhanced checks passed again on the public files. Verified copies are in `dist/`.
The production updater downloaded and verified the Latest 19,821,719-byte x64 ZIP without installation.

Native interactive dialogs, ARM64 hardware, physical printing, Narrator and third-party readers remain unverified;
packages remain unsigned. Polygon/polyline creation and richer free-text borders remain future work. Full PDF
rewrites can invalidate digital signatures; the existing save warning applies. Page labels remain positional
after structural edits.

## 0.15.0: editing existing link destinations (published as Latest at that time)

[SuniPDF 0.15.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.15.0) was stable **Latest** at that time under
`lmdanavanem`, release ID `414866395`, with six assets. Immutable tag/source:
`952f8699cac451e7c3004b6d816105d52e41f249`. [Windows x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343488)
and the same-source [ARM64 job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38889343639/job/116934762541)
passed. The duplicate release workflow was cancelled after the ARM64 job completed successfully;
the successful main-CI x64 packages and verified ARM64 artifact were used. Earlier releases remain unchanged.

This release adds `set-link-target` and an Edit target action for existing links. Safe HTTP/HTTPS/mailto addresses
and physical-page fit destinations replace the old `/A`, `/Dest` and `/AA`, retaining the annotation slot,
rectangle, appearance, comments/replies and unrelated properties. Detailed/named destinations are resolved
for prefilling where available; the dialog explicitly describes replacing their view with a page-fit target.
Read-only, locked, locked-content and malformed flag values are refused, including indirect flags.

The desktop reuses its link dialog, records one geometry-preserving Undo/Redo, and closes stale prompts after
edits or tab changes. English remains the default, French remains available; no dependency was added.
The published CLI has 53 commands. Select a link with the annotation selection tool, then choose Edit target
or double-click. Page destinations use full-page fit; precise coordinate/view authoring remains future work.

Local and Windows CI validation passed: **429 workspace tests, zero failed, seven default ignored**, formatting,
Clippy with warnings denied, **894 matching EN/FR keys**, **68 independent qpdf checks** and four licensed
NIST/NBS documents (**744 pages**). Local corpus: 48.891 seconds, peak sampled child working set 328,736,768
bytes. CI corpus: 32.955 seconds, 321,470,464 bytes. Native tests cover direct/indirect annotations, malformed
and locked flags, unsafe/out-of-range targets, unchanged form/content/rendering/source and both AES opening roles.
The test loader supplies the password during parsing to decode encrypted object streams; appearance generation
is completed before taking the strict atomicity baseline. Real-worker and EN/FR headless tests cover Undo/Redo,
actual accessible controls, validation, resolved destination prefill, cancellation and stale prompts.

Thirteen actual portable scenarios passed: upgrades from 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0,
0.13.0 and 0.14.0, plus bad checksum, wrong architecture, locked-file rollback and the real 120-second
close-timeout/concurrent-update guard. Seven MSI baselines (0.8.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0, 0.13.0,
0.14.0) passed installation, transactional rollback, upgrade, downgrade rejection, payload/user-data checks
and uninstall on disposable Windows runners. All 35 MSI logs have the expected final exit 0 or injected-failure
1603. Evidence is in the main job log and downloaded portable/MSI/corpus artifacts.

| Package | Bytes | SHA256 |
|---|---:|---|
| x64 portable ZIP | 19,803,226 | `CFC093316717596EE4BD3944DB6108A5512AC191F2D37782F55B4A389CE7E733` |
| ARM64 portable ZIP | 18,392,243 | `BA09DD4C57F0449AAFF767E7B2E77A894FA298576C3843A33F5334017C2A493D` |
| x64 MSI | 19,353,600 | `119C4725324AF01FC1245D685604C9A0927773BB2D08E48C4FEA70AFF230CCE7` |

All six staged files passed checksums, architecture, English version/icon resources, OCR/licenses,
MSI/ZIP payload and existing feature checks. Packaged link replacement covers plain and owner-opened AES-128/256,
both passwords, stable annotation identity/rectangle, ten qpdf checks, ten identical page PNG comparisons and
unchanged hashes of every input to the retargeting commands. All six unauthenticated public downloads match
API digests, sidecars and staged hashes; the enhanced binary checks passed again on public files. Verified copies
are in `dist/`. The production updater downloaded and verified the Latest 19,803,226-byte x64 ZIP without installation.

Native interactive dialogs, ARM64 hardware, physical printing, Narrator and third-party readers remain unverified;
packages remain unsigned. Replacing actions is not secure erasure of orphaned PDF objects. Full rewrites can
invalidate digital signatures; the existing save warning applies. Page labels remain positional after structural edits.

## 0.14.0: page-label authoring and navigation history (published as Latest at that time)

[SuniPDF 0.14.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.14.0) was stable **Latest** at that time under
`lmdanavanem`, release ID `414812776`, with six verified assets. Immutable tag/source:
`af532888804c196aabe216b8f6a2980c3d1e9e2d`. [Windows x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739746)
and the same-source [ARM64 build job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38883739698/job/116916354626)
passed. The duplicate release x64 build/automatic draft publication was cancelled after ARM64 succeeded;
the successful main-CI x64 packages and ARM64 artifact were used.

Pages / Organize opens Page labels: an inclusive physical range, decimal/Roman/letter numbering, optional prefix
and positive starting number, or prefix-only labels. Labels affect navigation, not printed page content; `#N`
still selects a physical page when labels are ambiguous. The CLI adds `set-page-labels` and `reset-page-labels`,
bringing its published command count to 52. Existing output-overwrite safeguards remain in place.

Range edits retain outside rules and restore their ordinal after the range. New prefixes are limited to 256
UTF-8 bytes and generated labels to 4096 bytes before candidate loading. Decoded number trees have bounded
traversal, references and input lengths; malformed/unsupported rules refuse Apply. Reset all removes the
catalogue entry even for malformed trees; it is not secure erasure. The candidate is loaded and its metadata
refreshed before the active state changes. Existing passwords, encryption, permissions, content, forms and links
use the shared writer. Full rewrites can invalidate digital signatures; the existing save warning remains.
Labels stay positional after reordering/insertion/deletion; semantic rebasing is not implemented (D104).

Undo snapshots retain whether page geometry/order changed. Undo/Redo of label and content edits preserves
reading history, while structural edits still invalidate it. Label dialogs close after tab/document or structural
changes (D105). English remains the default, French remains available; no new dependency was added.

Local and Windows CI validation passed: **420 workspace tests, zero failed, seven default ignored**, formatting,
Clippy with warnings denied, **890 matching EN/FR keys**, **63 independent qpdf checks** and **four licensed
NIST/NBS PDFs (744 pages)**. Local real-corpus time: 53.91 seconds; peak sampled child working set: 332,460,032
bytes. CI corpus: 51.89 seconds, 321,134,592 bytes. Native tests cover six styles, malformed/nested trees,
outside continuation, default/blank labels, protected files, unchanged rendering/text and save/reset/reopen.
Real-worker/headless UI tests cover Apply/Reset, invalid input, stale dialogs and geometry-aware Undo/Redo.

Twelve actual portable scenarios passed: upgrades from 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0
and 0.13.0, plus bad checksum, wrong architecture, locked-file rollback and the real 120-second close-timeout /
concurrent-update guard. MSI installation, transactional rollback, upgrade, downgrade rejection, payload hashes,
uninstall and user-data preservation passed from 0.8.0, 0.9.1, 0.10.0, 0.11.0, 0.12.0 and 0.13.0 on disposable
Windows runners. Evidence includes the main job log, 12 portable results, 30 MSI logs and the real-corpus report.

| Package | Bytes | SHA256 |
|---|---:|---|
| x64 portable ZIP | 19,785,200 | `9A2A085FDCC0E30852A2F72755705CB42EE2F5C866EF9A4E00ADCEEA9C6D6960` |
| ARM64 portable ZIP | 18,382,324 | `20CE7CA3C04495548FA45CADDB568266B52C574D6292A2A35A7DDAE0CAD476D0` |
| x64 MSI | 19,345,408 | `838DA744E862C840B4000678FC6E4757B3EFD85DBDB5910764178334EFFB5FBA` |

All six unauthenticated public downloads match API digests, SHA256 sidecars and staged CI files; verified copies
are in `dist/`. Staged/public binaries passed architecture, English version/metadata, exact icon, licenses/OCR,
MSI/ZIP payload and existing feature checks. New packaged tests cover range labels, reset, Unicode and outside
rules on plain and owner-opened AES-128/256 PDFs, both passwords, qpdf, readable text, identical PNG renders for
both fixture pages and unchanged source hashes. The production updater downloaded and verified the Latest
19,785,200-byte x64 ZIP without installation.

Native interactive dialogs, ARM64 hardware, physical printing, Narrator and third-party readers remain unverified;
packages remain unsigned. Scripted CI installation is not an interactive hardware claim. Metadata rewrites and
undo snapshots add memory cost for large documents.

## 0.13.0: typed and multi-page visual signatures (published as Latest at that time)

[SuniPDF 0.13.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.13.0) was published as stable **Latest** under
`lmdanavanem`, release ID `414763678`, with six verified assets. Immutable tag/source:
`b09259f7bd3b8602f7d2c290d5fd89d50dda5710`. [Windows x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818813)
and the same-source [ARM64 build job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38877818501/job/116897016513)
passed. The duplicate release x64 build/automatic draft publication was cancelled after ARM64 succeeded;
the successful main-CI x64 packages and ARM64 artifact were used. No previous tag or release asset was replaced.

Create a visual signature or initials by typing one line, choosing an existing standard or installed TrueType
font and black/blue ink, then explicitly previewing a native transparent PNG. Times italic is the default.
Text, font, ink and mode changes invalidate old previews; switching documents discards pending responses.
The optional local library reuses its existing PNG format; it remains unencrypted and does not prove identity.
Certificate-based CMS/PFX signing is a separate existing function. Input is bounded to 256 characters and
1600 pixels per side; existing glyph coverage and embedding restrictions are checked. No new dependency was added.

Choose Current page, All pages or a captured Organize selection. A single batch validates all targets, works on
a temporary document and commits once, with cooperative cancellation and one Undo/Redo. Signature previews and
actual placement share fitted geometry: preserve aspect and physical size, shrinking only for a smaller page.
The same displayed-page centre in PDF points is used on every target. Structural edits discard captured targets.
CLI `add-image --pages` is additive; the existing single-page command and JSON remain compatible.
Cancellation/failure leaves no partial page set and clears stale placement hints; explicit saving is still required.

Final local gates passed: **410 workspace tests, zero failed, seven default ignored**, formatting, Clippy with
warnings denied, **870 matching EN/FR keys**, **53 independent qpdf checks** and all **four licensed NIST/NBS
PDFs (744 pages)** in 76.40 seconds. Peak sampled child working set was 331,522,048 bytes. Native core tests cover
typed transparency/font checks, crop/rotation/protected PDFs, cancellation, save/reopen and qpdf. Real worker tests
verify non-mutating preview, progress and single Undo/Redo; headless app tests cover selected-page requests, stale
previews, structural invalidation, close cancellation and success/failure status retention.

Eleven actual portable scenarios passed: upgrades from 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0, 0.11.0 and 0.12.0,
plus bad checksum, wrong architecture, locked-file rollback and the real 120-second close-timeout/concurrent-update
guard. Actual MSI installation, transactional rollback, upgrade, downgrade rejection, payload hashes, uninstall and
user-data preservation passed from 0.8.0, 0.9.1, 0.10.0, 0.11.0 and 0.12.0 on disposable Windows runners.
CI real-corpus evidence recorded 32.09 seconds and peak sampled child working set 320,385,024 bytes.

| Package | Bytes | SHA256 |
|---|---:|---|
| x64 portable ZIP | 19,730,578 | `B5CC0CBAB1A1ABE4CFF346093A984760ECE458C2986CBF650C6EC3769C2A3FDD` |
| ARM64 portable ZIP | 18,320,921 | `C14B8F70E21ED3A811556868A267B1A862B31E55EBF71C6044EFF7C3B06F069D` |
| x64 MSI | 19,292,160 | `0727828749D235CCCD52BE6440C34F980F59FCABC7AB2B1027D2A77228D65C63` |

All six unauthenticated public downloads match API digests, SHA256 sidecars and staged CI files; verified copies
are in `dist/`. Both staged and public binaries passed architecture, English metadata/version, exact seven-frame
icon, licenses/OCR, MSI/ZIP payload and existing forms/compression/grouped-transform/protected-copy checks.
New packaged checks cover fitted signature batches on plain and owner-opened AES-128/256 PDFs, both passwords,
independent qpdf, readable text and unchanged originals. The production updater downloaded and verified the Latest
19,730,578-byte x64 ZIP without installation. Package fitting checks use each original page's dimensions, including the landscape fixture page.

Native interactive dialogs, ARM64 hardware, physical printing, Narrator and third-party readers remain unverified;
packages remain unsigned. Snapshots and the temporary document increase peak memory; native page calls cannot be
cancelled mid-call. Scripted CI installation is not an interactive hardware claim.

## 0.12.0: form creation and radio export compatibility (published as Latest at that time)

[SuniPDF 0.12.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.12.0) was published as stable **Latest** under
`lmdanavanem`, release ID `414674886`, with six verified assets. Immutable tag/source:
`3de5bbb7e9065afb38bfb1fe79a154c53b3602c9`. [Final Windows x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697688)
and the same-source [ARM64 build job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38866697728/job/116861070878)
passed. The duplicate release x64 build/automatic draft publication was cancelled after ARM64 succeeded;
the successful main-CI x64 packages and ARM64 artifact were used.

Create editable drop-downs through Prepare a form or `add-form-field --kind combo --editable`, with predefined
choices and custom text. Existing creation APIs keep their default choice-only behaviour. Radio `/Opt` export
strings remain distinct from widget appearance names and are used consistently in the app, JSON/XFDF and FDF
import. Invisible widgets keep their original indices. CLI information adds `widgets[].export_value`.

Malformed or ambiguous mappings, duplicate widget references and non-UTF-8 appearance names make only the
affected radio field read-only. Exact unchanged exports, including reserved or empty strings, remain safe to
import. Tests check raw `/V`, `/AS` and appearance keys after unrelated edits. PDFium can regenerate missing
appearances; no-`/Opt` radios keep their existing behaviour. Changing existing editable flags remains future work.

Final local gates passed: **398 workspace tests, zero failed, seven default ignored**, formatting, Clippy with
warnings denied, **859 matching EN/FR keys**, **48 independent qpdf checks** and all **four licensed NIST/NBS
documents (744 pages)** in 59.68 seconds. Tests include headless EN/FR authoring, real worker creation/filling with
Undo/Redo, CLI save/reopen, cropped/rotated protected pages, owner-opened AES-128/256 and malformed mapping refusal.
Portable and MSI upgrade gates now include the independently verified 0.11.0 public package baseline.

Ten actual portable scenarios passed: upgrades from 0.8.2, 0.8.3, 0.9.0, 0.9.1, 0.10.0 and 0.11.0, plus bad
checksum, wrong architecture, locked-file rollback and the real 120-second close-timeout/concurrent-update guard.
MSI installation, transactional rollback, upgrade, downgrade rejection, payload hashes, uninstall and user-data
preservation passed from 0.8.0, 0.9.1, 0.10.0 and 0.11.0 on disposable Windows runners.

| Package | Bytes | SHA256 |
|---|---:|---|
| x64 portable ZIP | 19,699,323 | `AD1ECB72A999F1A43602E58AD2634F9E80FD7925C90A90B9AD25FB9B6BDC1B83` |
| ARM64 portable ZIP | 18,290,932 | `567362FB988C9C2C7FF60B67221BAC8D6901E05EF6F6FA25CB8CB1ADE47CD358` |
| x64 MSI | 19,267,584 | `390A20B7B5999635FDC6F2FE99ED4BD25DE53EAB4623B820FEC2F9AF04AA0C89` |

All six unauthenticated public downloads match API digests, SHA256 sidecars and the staged CI files; verified
copies are in `dist/`. Both staged and public binaries passed architecture, English metadata/version, exact icon,
licenses/OCR, MSI/ZIP payload, compression, forms/XFDF, grouped transform and protected-copy checks. The new checks
also cover authored editable combos and a QPDF-generated `/Opt` radio fixture on plain and owner-opened AES-128/256
PDFs, with independent qpdf validation. Original inputs stayed unchanged. The production live updater downloaded
and verified the Latest 19,699,323-byte x64 ZIP without installation.
Native interactive dialogs, ARM64 hardware, physical printing, Narrator and third-party readers remain unverified;
packages remain unsigned. Scripted CI installation is not an interactive hardware claim.

## 0.11.0: navigation, forms and protected copies (published as Latest at that time)

[SuniPDF 0.11.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.11.0) was published as stable **Latest** under `lmdanavanem`,
release ID `414626617`, with six assets. Immutable tag/source: `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`.
[Final Windows x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860176578) and the same-source
[ARM64 build job](https://github.com/lmdanavanem/SuniPDF/actions/runs/38860179447/job/116840434621) passed.
The duplicate release x64 build/automatic publication was cancelled after ARM64 succeeded; the successful main-CI
x64 artifacts and ARM64 artifact were used. The author, tag, asset digests and stable Latest API state were checked.

Nine actual portable scenarios passed: upgrades from 0.8.2, 0.8.3, 0.9.0, 0.9.1 and 0.10.0, plus bad checksum,
wrong architecture, locked-file rollback and the real close-timeout/mutex scenario. Actual MSI installation,
transactional rollback, upgrade, downgrade rejection, payload hashes, uninstall and user-data preservation passed
from 0.8.0, 0.9.1 and 0.10.0 on disposable Windows runners. These are isolated automated tests, not an interactive
hardware claim. All six staged assets passed architecture/version/icon/license/OCR and MSI/ZIP payload checks;
packaged x64 labels, owner-opened AES-128/256 copies and metadata, both passwords, qpdf/readable content, compression,
forms/XFDF, grouped transforms and French OCR passed. The original files stayed unchanged.

| Package | Bytes | SHA256 |
|---|---:|---|
| x64 portable ZIP | 19,687,375 | `6A08A518619D89D450332F8AE1BA31FB336C3DF88910D5471183B25C173B2421` |
| ARM64 portable ZIP | 18,279,249 | `8A6B958F2211BC95666DE10342E3338BBE80170667BBBB1D5DBD012584D04C33` |
| x64 MSI | 19,255,296 | `E99FE4416DA5E55086059884CCBE770EA1D9AB437D8E3BE26D49D696B5C9C4BD` |

All six unauthenticated public downloads match API digests, SHA256 sidecars and the same-source CI packages; verified copies are in `dist/`. Enhanced checks passed on public binaries, including owner-opened AES-128/256 content and metadata. The production live updater downloaded and verified the Latest 19,687,375-byte x64 ZIP without installation. Packages remain unsigned.
ARM64 hardware, physical printers, Narrator, interactive Windows dialogs and third-party readers remain unverified.

- Cached PDF page labels, exact unique-label navigation and explicit `#N` physical positions; thumbnails/status,
  accessibility and additive CLI `info.pages[].label`. Unreadable, control-containing or over-4096-byte labels use
  physical positions. Label authoring and semantic rebasing after structural edits are not implemented.
- Resolved local XYZ/Fit/nonzero-FitH destinations for links and bookmarks, using target CropBox/PDF rotation and
  reading rotation. Same-page and rapid back/forward navigation keep zoom and positions. Text-selection clicks now
  reach links. URI confirmation remains; remote, embedded, Launch and JavaScript actions are not followed.
- Typed form-read failures preserve text, choices, in-place input and pending actions. They block commits based on
  stale values, show Retry, and do not retry continuously. A confirmed empty form is still a valid successful read.

Final local gates for the expanded 0.11.0 source passed: **390 workspace tests, zero failed, seven default ignored**,
formatting, Clippy with warnings denied and **858 matching EN/FR keys**. Independent qpdf checks passed for **42 distinct
PDFs (44 password-specific checks)**, plus malformed-input refusal. The **four licensed NIST/NBS documents, 744 pages**,
passed in 91.59 seconds with peak sampled child working set 329,347,072 bytes. Tests exercise editable combo panel/page
input, ambiguous export labels, first-frame keyboard input, failed reads/retries, eight crop/rotation/encryption cases,
JSON/XFDF round trips, protected extraction/split, both passwords, RC4/AES-128/AES-256, source/collision preservation,
actual worker output and unsaved page rotations. Final Windows CI, same-source ARM64, staged package checks and publication passed; public downloads and the production live updater passed.

Before publication, independent checks of candidate `1714846` caught corrupt AES-128 streams after opening with a
distinct owner password. Those candidate packages were never published. Existing lopdf is now pinned to official
revision `6ce451328c5caa65d50462daaf4ee8db9c85bb8e`, with upstream owner-key/password-encoding fixes and its MIT
license included. The final local results above cover this corrected dependency and both source-opening roles,
including decoded text/streams, metadata rewrites and unprotect. Windows builds and package gates must use this
corrected source `0b7a22b2cf293c2908e8e6c6107c3f89bde01862`; earlier candidate artifacts were excluded.
Historical navigation/form-read slice, before the expanded 0.11.0 source: **383 workspace tests passed, zero failed, seven default ignored**; formatting, Clippy with warnings
denied and **857 EN/FR keys** passed. The explicit qpdf gate passed **34 PDFs** and rejected its malformed samples.
All **four licensed real documents (744 pages)** passed in 63.01 seconds; peak sampled child working set was
328,953,856 bytes. Core/CLI tests cover named/direct/action destinations, rotations/crops, AES-256 and save/reopen;
headless UI tests cover label navigation, link clicks, same-page zoom history, rapid Back/link sequences and failed
form reads. The actual local GUI and CLI development binaries were built successfully. GitHub CI is separate.
The corrected 0.11.0 source is published; historical 0.10.0 files remain unchanged. Native interactive use, Narrator and third-party
readers remain unverified.

The expanded lot also implements custom text in existing editable drop-downs from the fields panel or page,
including typed export-value retention across ambiguous labels, failed reads and retries. Extraction and split parts
retain passwords, the encryption key and permissions. The input cannot be replaced; split without overwrite uses
atomic non-clobber publication. Plain sources stay plain. The existing permission policy is unchanged. Split failures
can leave earlier completed parts, which remain complete and protected. CLI output retains `encryption_removed: false`
and adds `encrypted`. Upgrade checks now include the published 0.10.0 portable and MSI baselines.
Page labels and rotations share one cached metadata pass. Page rotation updates only the affected cache entries;
grouped deletion validates all indices before mutation and refreshes metadata once. A 492-page CLI `info` measurement
on AMD Ryzen 7 6800HS took 2.332/2.848/2.869 seconds in the optimized dev build, versus 2.387/2.455/2.536 seconds
in the published release build. Profiles differ: these are whole-command observations, not an isolated opening-cost
benchmark or a performance guarantee. Eager label reading still adds work when opening or rebuilding document state.

## 0.10.0: requested P1/P2 lot published and verified

The six requested backlog areas are implemented: data-safety/corpus checks, real image compression, reading controls,
form creation/interchange/grouped keyboard editing, mouse cropping/group transformations, and compression/OCR batches.
The CLI source registers 50 subcommands. English remains the default; French remains available.

The complete local workspace recorded **362 tests, zero failures, seven default ignored**. Formatting, Clippy with
warnings denied and all 853 matching EN/FR catalogue keys passed. The two independent corpus gates were then
executed explicitly: **31 synthetic/generated PDFs** passed qpdf, and **four authentic public research PDFs,
744 pages in total**, passed bounded CLI regression checks. Final-source GitHub CI, builds, staged packages and actual
portable/MSI update regressions passed as detailed below. [SuniPDF 0.10.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.10.0)
was published as **Latest** at that time under `lmdanavanem`, with six assets (release ID `414522111`), neither draft nor prerelease.

[Final main x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38848137022) passed for immutable
tag/source `f7510afd314846079bbdd838525cd28372fd3100`, including the workspace, Clippy, independent corpora and actual
update regressions. [ARM64 build](https://github.com/lmdanavanem/SuniPDF/actions/runs/38848138423/job/116803186215)
passed from the same source; ARM64 hardware execution remains unverified. The duplicate release x64 workflow was
cancelled after ARM64 completed; the tested main-CI x64 files and successful ARM64 job provide the release packages.

All **eight real portable scenarios** passed: 0.8.2/0.8.3/0.9.0/0.9.1 to 0.10.0, each with actual application relaunch,
payload/backup hashes and documents/settings preserved; bad checksum, wrong architecture, locked-file rollback and
the real 120-second close guard/concurrent-update mutex also passed. Actual per-machine MSI regression passed from
**both 0.8.0 (French) and 0.9.1 (English)**: installation, injected transactional rollback, upgrade, downgrade rejection,
payload checks, uninstall and user-data preservation. These ran on disposable Windows runners with Mesa 26.2.4
llvmpipe for GUI startup; its test DLLs are never included in distributed packages.

All six staged files passed checksum/sidecar verification; GUI/CLI/PDFium/Tesseract architecture for x64/ARM64,
English executable metadata, seven embedded ICO frames, pinned FR/EN OCR models and all notices were checked.
Actual packaged x64 binaries passed info/render, French OCR, image-compression gain/source preservation,
radio/combo/list creation and XFDF round trips, and grouped rotation/resize rendering. Read-only MSI inspection
confirmed language 1033, compatible upgrade transaction and payload hashes matching the x64 ZIP. The real-install
proof comes from CI above; staged inspection does not itself install anything.

All six public files were downloaded without authentication and matched GitHub API digests, their SHA256 sidecars
and the exact staged files, then copied to `dist/`. Enhanced verification on those public bytes repeated architecture,
English metadata, all seven embedded icon frames, pinned models/notices, actual compression gain, choice forms/XFDF,
group transformations, French OCR and read-only MSI payload/transaction checks. The production live-updater test
passed against the Latest public x64 ZIP: **19,651,167 bytes**, SHA256
`add8ef3f1f385b47aa4e3fb9061f8735c7e31004d408abd853924a8bd9a35742`. HTTPS redirects, size and SHA verification
passed without installing locally; the real installations passed separately in CI. The real corpus also passed in
CI in 33.747 seconds with an observed peak child working set of 333,922,304 bytes; the local measurements below
describe their own run and are not hardware-independent performance guarantees.

| Area | Implemented scope and gate |
|---|---|
| Data safety | Neighbouring NamedTempFile, content sync, closed file handle and atomic rename; injected interrupted/full-disk/write/permission failure tests and a killed partial-save child; original must remain readable and unchanged |
| Independent corpus | Four pinned NIST/NBS publications (66/25/161/492 pages), including a genuine monochrome scan and grayscale photographs; source SHA256/size, text/render, extract, optimize, compression, corrupt-input rejection and unchanged source checks passed |
| Compression | 8-bit DeviceRGB/DeviceGray, JPEG/raw/Flate decoding, <=32 million pixels/image; maximum side 128..8192 pixels, JPEG quality 10..100; mask resource graphs/profiles/non-default decoding retained; no-gain original-byte copy; signed/restricted documents refused |
| Batch UI | Sequential 1..128 PDFs, <=256 MiB/input, compression or local FR/EN OCR, item/file progress, cancellation and individual errors; new atomic copies never overwrite sources or existing files |
| Reading | View rotation without PDF mutation, full screen and 256-entry per-tab history; editing returns to upright view |
| Forms | Radio/combo/list creation, XFDF export, bounded strict JSON/XFDF/FDF value import, atomic grouped panel Apply/Discard, one undo, page Space/Enter and choice-popup focus |
| Editing | Mouse crop preview and grouped rotation/resizing, one undo; save/reopen/render and normal/rotated/cropped/AES-256 geometry checks passed |

The real corpus run took 44.3 seconds; the highest observed child working set was 343,621,632 bytes against a
768 MiB sampled memory guard. Each process is limited to 180 seconds and 2 MiB of captured output. Four photographs
in the 161-page report were recompressed, reducing 9,791,921 bytes to 9,592,202; the other three compression results
retained the original bytes. Three upstream originals already have stale Adobe linearization/hint diagnostics:
their exact qpdf diagnostics are pinned. Rewritten outputs pass without warnings; byte-identical no-gain results
retain only the pinned original baseline. These source warnings are not counted as clean rewritten output.

Real documents exposed stale storage objects, `/Size` bounds and revision offsets in full lopdf rewrites. The shared
writer now removes obsolete storage/linearization objects and `/Prev`/`XRefStm`, recalculates the maximum object ID
and preserves the original encryption key/permissions. Independent qpdf checks and encrypted regression tests passed.
Atomic-save fault injection, killed partial-write preservation, locked destination handling and headless grouped
edit/form/reading controls also passed locally. Native GUI interaction and power-loss hardware durability remain separate.

Hardware and interactive limits remain: ARM64 execution, physical printers, Narrator, real native update/save dialogs,
Acrobat/Foxit compatibility and customer/hostile/very-large corpus coverage. PDF/A identification preservation is not
conformance certification; certificate-chain trust, revocation and trusted timestamps/LTV remain unimplemented.

## 0.9.1 reliability release: previous publication evidence

[SuniPDF 0.9.1](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.1) was published as Latest under
`lmdanavanem`, with six assets and tag/source `c8473ecafe5f88b9dad846677fa610226955b061`.

Implemented: larger application icon (about 91% visible width instead of 50%), preservation of PDF/A/custom XMP,
strict malformed-XMP refusal, explicit AES-256 key length, update-helper restart/error reporting and .NET SHA256 hashing,
pinned QPDF corpus checks and real portable/MSI upgrade regression scripts in Windows CI.

Local workspace: 322 tests passed, zero failed, five default ignored; Clippy, formatting and 804 bilingual keys passed.
The explicit QPDF gate passed on 22 synthetic/generated PDFs and rejected three malformed inputs. XMP preservation,
compressed/encrypted streams and AES round trips are covered. Windows PowerShell 5 passed with Get-FileHash unavailable.

[Final main x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38839336101) passed for source
`c8473ecafe5f88b9dad846677fa610226955b061`, including the workspace, Clippy, qpdf and real 0.9.1 update regressions.
Seven portable scenarios passed: 0.8.2/0.8.3/0.9.0 to 0.9.1 with actual relaunch and payload checks, bad SHA256,
wrong architecture, locked-file rollback, and the real 120-second close guard/concurrent-update mutex.
The actual per-machine MSI regression passed 0.8.0-to-0.9.1 install/transactional rollback/upgrade/downgrade
rejection/uninstall with user-data preservation on a disposable runner.

GUI regression tests use pinned Mesa 26.2.4 llvmpipe on these runners. Its two DLLs exist only in temporary test copies,
never distribution packages; native graphics-driver validation is separate. The earlier
[fast Windows regression](https://github.com/lmdanavanem/SuniPDF/actions/runs/38838856238) also passed against a pinned
public 0.9.0 target on Windows 2022 and latest, with a real MSI scenario.

[ARM64 build](https://github.com/lmdanavanem/SuniPDF/actions/runs/38839338858/job/116775562315) passed from the same source;
hardware execution remains unverified. The six staged files passed SHA256/sidecar, GUI/CLI/PDFium/Tesseract architecture,
English GUI version, pinned eng/fra models and notices checks. All seven expected ICO images are embedded in both
GUI executables; packaged x64 reading/rendering, French OCR and field creation/fill/reopen passed. MSI payloads match
the x64 ZIP; language 1033 and rollback sequence are verified.

All six public assets were downloaded without authentication and matched GitHub API digests, SHA256 sidecars and
the exact staged files, then copied to `dist/`. Actual public x64 binaries passed PDF reading/rendering, French OCR
and form-field creation/fill/reopen; the public MSI payload, language and rollback sequence passed read-only verification.
The explicit live production-updater test passed against the public 0.9.1 x64 ZIP (19,270,287 bytes,
SHA256 `601306861be0c80bda62e568473172504b530d4344635d3bfa28cbf6adbfc082`): production HTTPS/redirections,
size and SHA256 checks passed without installation. The real upgrade/install scenarios above passed separately in CI.
The duplicate release x64 job stopped before its update tests because GitHub rate-limited unauthenticated metadata
requests; the verified x64 packages come from the successful main CI above. The ARM64 job succeeded independently.
After publication, the CI harness was changed to authenticate only fixed GitHub API metadata requests, with redirects
disabled and the token removed before child processes. Anonymous/authenticated offline boundary checks passed;
the full workflow has not been rerun for that test-only change. Published package bytes are unchanged.

Local compilation is limited to one Cargo command with `-j 2`. A workspace rebuild hit Windows paging-file error1455;
sequential crate builds followed by the complete workspace rerun passed. No installation or user profile was modified locally.

Remaining independent checks: real customer/hostile corpus, ARM64 hardware, physical printers, Narrator,
interactive update/save dialogs and Acrobat/Foxit interoperability. PDF/A identification preservation is not conformance certification.
Certificate trust/revocation/trusted timestamps/LTV remain unimplemented.

## 0.9.0 published and validated (previous release)

Implemented: 60-second/tab-suspend recovery; Windows printing; logical text selection across pages;
two-page/cover-first Book views; text/checkbox creation; bundled local FR/EN OCR; CMS/PFX signing and verification.
All 14 tools have commands. The CLI has 46 subcommands. English is the default; French remains available.

| Check | Current result |
|---|---|
| New core integration tests | 10 passed: reading ranges/rotations, form creation, OCR and certificate signatures |
| New end-to-end CLI tests | 3 passed: fields, real OCR, saved-byte signatures and source preservation |
| `cargo test --workspace --locked -j 2` | 318 passed, 0 failed, 4 ignored (two measurements, live updater and native printing), final rerun passed |
| `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | Passed with no warnings |
| `cargo fmt --all -- --check` | Passed |
| Bilingual catalogue check | Passed: 804 matching EN/FR keys |
| Update helper self-test | Passed, including flat OCR payload replacement and rollback |
| Native Microsoft Print to PDF | Explicit ignored test passed using the Windows driver: reopened one-page output contains unsaved edits; all source PDF objects/trailer (except generated ID) and original bytes remain unchanged |
| 0.9.0 publication | [Published as Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.9.0) under `lmdanavanem`; tag/source `dbeafdf6b484c784a0dab26fe6698049ec2e49ca` |
| CI build provenance | [Main x64 CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/38830535213) passed; [ARM64 build](https://github.com/lmdanavanem/SuniPDF/actions/runs/38830543193/job/116748068977) passed. The duplicate release x64 build was cancelled after both artifacts were ready; the automated publisher was skipped and the owner published six verified files |
| Public release files | All six files downloaded and verified against API digests and SHA256 sidecars, then copied to `dist/`: x64/ARM64 ZIP, x64 MSI and their three checksums. GUI/CLI/PDFium/Tesseract architecture, English GUI version, pinned eng/fra models and full OCR notices checked |
| Packaged x64 application/CLI | Actual public binaries passed PDF reading/rendering, French OCR (`Bonjour`, `français`, `bientôt`) and field creation/fill/reopen checks |
| MSI x64 | Built locally from the exact main-CI ZIP; every MSI payload hash matches that ZIP. Language 1033, unrestricted upgrade language and rollback sequence checked. WiX smoke passed with the existing ICE61 same-version-upgrade warning |
| Live production updater | Explicit ignored network test passed against the public 0.9.0 x64 ZIP: production HTTPS/redirection, size and SHA256 checks; no installation performed |

Not established: physical printer output, complete installer/portable update execution, ARM64 hardware,
new native UI visual coverage or interactive printer dialogs, Narrator, Acrobat/Foxit signature compatibility or certificate identity trust.
Trusted timestamps, revocation and LTV are not part of this release.

## Historical implementation record through 0.8.3

The phase details and validation table below retain their original version-specific evidence. Older command/key
counts, CI observations and screenshots are historical; they are not current release test or publication results.

## Phase 0 - Audit and planning: completed (historical)
- [x] `FEATURE_MATRIX.md`, `ARCHITECTURE.md`,
      `TECH_STACK.md`, `ROADMAP.md`, `I18N.md`, `DECISIONS.md`, `KNOWN_ISSUES.md`, `THIRD_PARTY_NOTICES.md`.

## Phase 1 - Foundations: completed (historical; final visual validation by the user recommended)
- [x] Rust workspace (`sunipdf-core`, `sunipdf-i18n`, `sunipdf-desktop`, `sunipdf-cli`), `cargo build` OK.
- [x] PDFium engine integrated (`pdfium-render` 0.9.4, dynamically loaded DLL, 4 search locations).
- [x] Open / display / navigate (keyboard, toolbar, page field) / zoom (fit width, page, steps) / close.
- [x] Protected documents: password dialog, "incorrect password" message.
- [x] Drag and drop of a `.pdf`, opening by command-line argument, window title.
- [x] UI states: empty, engine starting, opening, rendering in progress (spinner), error (banner), engine missing.
- [x] fr-FR / en-US i18n: 76 keys, hot switching, persisted preference (atomic write), light/dark theme.
- [x] Rendering off the UI thread (single engine thread, generations, one render in flight).
- [x] CLI: `info` (JSON), `render` (PNG), `commands` (registry), stable error codes, exit codes 3/4/5.
- [x] **annotations-advanced** slice (advanced annotations, SUNI-021b, D067–D076): engine `sunipdf-core::annot_write` (ink, free text box, rectangle, ellipse, line, arrow, link, reply, review status written by lopdf with an explicit `/AP` stream; `annotation_details` aligned with `/Annots` with each annotation's rights; `move` / `resize` / `set_annotation_style` / `update_free_text` / `set_annotation_contents` / `delete_annotation_thread`; appearance regeneration reserved for `/NM SuniPDF-…` annotations), `appearance` (pure functions: Helvetica widths, wrapping, WinAnsi, stroke simplification and smoothing, stream of each shape), `links` (`page_links` through `annotations()`, `is_safe_external_uri`), `annot::annotations` aligned by index; coordinates in display space, appearances with a rotation `/Matrix` (upright text on a rotated page); encrypted documents stay encrypted; worker (12 `EditOp`, `Request::Annotations { detailed }` and `Links`); interface `annot_adv.rs`: 7 tools in **Add comments** (Select an annotation A, Freehand D, Free text F, Rectangle, Ellipse, Line or arrow, Link), floating bar, text box / link / reply / **external address confirmation** dialogs, link clicks (page jump; address after confirmation), select / move / resize / recolour / delete, Comments panel as threads (status, replies, author remembered in `settings.json`); CLI `annotations`, `annotate`, `reply`, `review-state`, `annotation-delete` (41 commands). 7 commands (97 in the registry), 64 i18n keys (676). Fixture `annotated.pdf` (12 annotations from several producers). Tests: 16 new engine unit tests (`appearance`: widths, wrapping, WinAnsi, numbers, simplification, smoothing, streams; `annot_write`: dates, names, rights, scaling, translation, matrix, thread status; `links`: 14 hostile addresses refused) + 12 engine integration tests (ink round trip to the pixel, rendered shapes, free text that grows and keeps its text, **rotated and cropped pages to the pixel** on `boxes.pdf` with an oriented arrow and upright text compared to an upright page, links without duplicates, faithful reading of `annotated.pdf`, move / style / content / thread deletion without touching other annotations, resized SuniPDF annotations, replies invisible to the pixel, **AES-256 and RC4 encryption kept**, snapshot / undo, unreadable `/Annots` entry) + 7 application unit tests (hit test, resizing, default box, dates, threads, bar, link decision) + 2 CLI unit tests + 3 end-to-end CLI tests (`annotate` ×6 kinds re-read by `annotations`, reply / status / thread deletion, typed errors including dangerous addresses, `already_exists`, encrypted document). Screenshots inspected: Comment bar with 14 buttons, ink drawn with the mouse, Free text box dialog, Link dialog, **external address confirmation** (click on the `annotated.pdf` link), Comments panel as threads (note + reply + Accepted status, light and dark themes), selection of an Acrobat annotation (frame without handles, settings hidden) and of a SuniPDF annotation (frame, four handles, bar), **mouse move and resize** with reselection. Not shipped: see `NEXT_TASKS.md` (SUNI-021b) and `KNOWN_ISSUES.md` A39 (Acrobat / Foxit interoperability not verified).
- [x] **forms** slice (AcroForm forms, "fill" half of Fill & Sign, D061–D066): engine `sunipdf-core::forms` (model read by lopdf: `/Kids` hierarchy with inheritance, multi-widget fields, `/AP /N` states, pages via `/Annots`, lists with export value and label, rectangles in display space via `PageGeometry`; `form_info`, `validate`, strict/tolerant `fill_form` with full validation before any write, `clear_form`, `flatten` with the Print flag set and `/AcroForm` removed, `ensure_appearances`; `DocumentInfo.form`) and `form_data` (JSON export/import/structure, CLI coercion); appearances regenerated by PDFium and **present in the saved bytes** (`(Jean Dupont) Tj` re-read by lopdf); encrypted document filled and kept encrypted; XFA read-only; line breaks kept despite lopdf's PDFDocEncoding table; worker `Request::FormInfo`, `EditOp::{FillForm, ClearForm, FlattenForm}`, `BatchOp::ExportFormData`; interface `forms.rs`: **Fill form** tool (rectangles per page in continuous view, blue / red required / grey read-only, click on checkbox and radio, editor placed on the text field with Enter / Esc / Tab / Shift+Tab and content selection, floating choice list, checkbox popup for multiple choice), **Form fields** panel (Ctrl+Shift+F, one native egui control per field, current page or whole document, XFA and scripts banners, go to field), banner on opening, **Flatten form** dialog (all pages / current page), **Clear fields**, **Export / Import (JSON)**, **Forms** menu, "fill" group of the Fill & Sign tool, per-tab state, widgets excluded from the Comments panel; CLI `form-fields` (`--values-only`), `fill-form` (`--values`, `--set`, `--ignore-unknown`, `--flatten`), `flatten` (36 commands). 6 commands (90 in the registry), 59 i18n keys (611). Tests: 3 engine unit tests (PDFium oracle `lopdf_model_matches_pdfium_enumeration` on the 16 widgets, validation rules, JSON and coercion) + 10 engine integration tests (fields/types/values/widgets, filling re-read by snapshot and by file with pixels, typed errors and atomicity, appearances in the bytes, flattening with a `/F 0` highlight and a three-page document, XFA, encryption kept, 90/180/270 rotation, JSON round trip, clearing), 3 application unit tests (reading order, `forms.kind.*` keys, reset; `forms.rs` keys swept by `ui_keys_used_by_app_exist`), 1 CLI unit test (`--set`) + 1 end-to-end CLI test (`form-fields`, `fill-form --set` re-read, `--values --flatten` then pixel-level rendering, strict refusal without a written file, `--ignore-unknown`, `flatten`, `already_exists`, `form_xfa`). Screenshots inspected: tool + panel (16 rectangles, grey `ref`), ticked checkbox + moved radio + open list, "Canada" chosen (list arrow drawn by PDFium) then Tab from `nom` to `ville` to `commentaire` (content replaced, multiline), Flatten dialog, flattened document (values in the page, empty panel, greyed commands), Ctrl+Z (fields back), banner on opening without the tool, XFA (banner, grey fields), dark theme + English with the list open. Not shipped (D066): field creation, XFDF/FDF.
- [x] **sign-image** slice (visual signature, "sign" half of Fill & Sign, D054–D060): engine `Document::add_image_data` (decoded image, alpha kept through the `/SMask` that PDFium 7881 writes, verified at pixel level: 50% blend, blue intact under the transparent area, persistence after reload and reopening, mask following move and resize) and `signature_image` module (anti-aliased rasterised strokes, light background removed by a ramped threshold, cropping, reading bounded to 16,384 px and 512 MiB decoded, refused before decoding, with EXIF orientation, atomic PNG); worker `EditOp::AddImageData`, `Request::LoadImage` / `Response::ImageLoaded`; interface `sign.rs`: "Handwritten Signature / Initials" dialog (permanent "no cryptographic value" box, local library `%APPDATA%\SuniPDF\signatures` with thumbnails on white paper and two-click deletion, drawing pad with black/blue ink + thickness + undo/clear, image import decoded by the engine thread with preview and threshold, optional saving), ghost placement (click, Shift+click, Esc, crosshair cursor), image object reselected (handles, rotation, Ctrl+Z), today's date by catalogue (`{d}/{m}/{y}`), **Fill & Sign** tool enabled in the hub (Sign / Initials / Date bar), commands in the Edit menu, S shortcut; CLI `add-image` (`--remove-background`, `"digital_signature": false`, 33 commands). 4 commands (84 in the registry), 45 i18n keys (552). Tests: 6 engine unit tests (`signature_image`, including the refusal of a PNG announcing 2 GiB without allocating anything), 2 engine integration tests (`image_alpha_is_preserved_as_soft_mask`, `signature_strokes_to_page_pipeline` with a < 300 KB size guard), 5 application unit tests (bounded placement, date kept within the page, library listing/sorting, date per language, default state; `sign.rs` keys swept by `ui_keys_used_by_app_exist`), 1 CLI unit test + 1 end-to-end CLI test (`add-image` then `render` re-read at pixel level, `already_exists`, empty image). Screenshots inspected (`SUNIPDF_SIGNATURES_DIR`, `SUNIPDF_PICK_FILE`): empty dialog (A), three strokes placed on the blue rectangle and selected (B), Ctrl+Z (C), library with thumbnails from the hub tool (D1) and reuse of a thumbnail placed under the rectangle (D2), date ghost with status message (E1), date placed and selected (E2), Esc (F), import of a photo with the background removed and threshold (G), Initials in dark + English with three strokes (H).
- [x] **review-fixes** slice (fixes for the 39 findings of the adversarial reviews, D046–D053): engine (AES-128 refuses passwords outside printable Latin-1, PDFium round trip tested, accessibility bit always granted, XMP regenerated by `set_metadata`, `strip_metadata` on pages/images/`PieceInfo`, `write_atomic` with `sync_all`, EXIF orientation and JPEGs embedded as is, reliable bring-to-front after reload, stamps rotated within the page, uniform resizing of skewed objects, `add_text` with tabs and the list of missing characters, full text of rotated pages, stable annotation subtypes, `Document::is_encrypted`), interface (`FIELD_SHORTCUTS` / `SHORTCUTS` shortcut tables with Shift read first and a tested invariant, dialogs and editors inside the shortcut guard, Esc of an open list consumed by `dialog::show`, dialog bodies scrolling at a fixed height and fixed-height footer, egui keyboard zoom disabled, save point in the engine thread, atomic cut (copy then delete in the engine thread), marking intent remembered, grid selection emptied after deletion, drop without move ignored, Move Forward/Backward greyed outside valid cases, Apply metadata greyed during loading, properties cache invalidated, Apply greyed without text, translated units and suffixes, `Palette::error` error colour, announcement of the encryption removal on extraction/splitting), CLI (`extract` / `split`: `encryption_removed`). 23 new i18n keys, 1 removed (507). Tests: 6 new engine integration tests (AES-128/accessibility, XMP/PieceInfo, extraction of a protected document, EXIF/JPEG at pixel level, bring-to-front after reload + corner stamps, skewed object + text of rotated pages) + 3 engine unit tests (PDFDocEncoding/XMP, orientation matrices, subtypes), 8 application unit tests (shortcut order with an egui scenario, Esc and open list, contrast ≥ 4.5:1, save point ×2, sizes and decimal separator, protection messages, annotation labels). Screenshots inspected: Document Properties at 720 × 480 (header and buttons visible, scrolling body), Properties in English dark ("1 KB"), Stamp without text (Apply greyed), Comments "Strikethrough", Protect dark with a readable error, Rotate then Ctrl+Z (title without "modified", Save greyed).
- [x] SUNI-012b (continuous-view slice) Continuous view, tiles and anchored zoom: **continuous scrolling by default** (stacked pages, 16 px margin, 12 px gap, soft shadow), **single page** optional (F5, rail button, View menu with the active mode highlighted, palette; `continuous_view` preference), **virtualisation** (only the visible pages create widgets, requests and textures; `Layout` + binary search), **current page tracked** while scrolling with a lock after "go to" (field, thumbnail brought into the panel, bookmarks, First Page/Last Page, search brought to the upper third, including horizontally at high zoom), **tiled rendering** (`sunipdf-core::tiles`, `Document::render_region`: whole pages <= 4096 px / 8 Mpx, otherwise 1024 px tiles, bounded by `max_texture_side`) with **cancellation by replacement** of the wanted list (`SetWanted` / `Tile` / `TileFailed`, epochs), bounded **texture pool** (192 MiB, LRU) and textures of another scale or stale ones displayed during a zoom or an edit (`pages_unchanged`), **anchored zoom** (Ctrl + wheel under the pointer, buttons at the centre, up to 3200%, `MAX_TILED_SIDE_PX` / total height ceilings), keyboard (Space, Next/Previous Page by one screen, arrows by one line), bounded (256) and synchronised thumbnails, tabs keeping their scroll position; Objects Panel F5 → F3; right rail: active toggles highlighted, top group scrolling in a short window; CLI `render --region`; fixture `many-pages.pdf`; 2 commands (80 in the registry), 4 new i18n keys + 4 modified (485); D038–D045. Tests: 7 engine integration tests (regions = exact crop of the full render, rotated page, ×40 zoom beyond 16,384 px, image page at ×50, errors, 120-page fixture, tile stitching at 99.9%) + 1 ignored measurement, 7 engine unit tests (tiles, cache), 13 application tests (layout, 600 virtualised pages, zoom anchor, ceilings, bounded pool, wanted tiles, sending, engine queue, `keeps_pages`, preferences) + 1 ignored measurement, 1 CLI test. Screenshots inspected: continuous opening (S1), last page (S2), single page identical to the previous version (S3), 1600% zoom without a panic (S4) and sharp text at 800% on a search result, "Page 7" search (S5), selection + Ctrl+C on page 2 (S6), page 2 highlight + comments panel (S7), rectangle and object selection with handles on page 2 (S8), Next Page after a click (S9), fit page (S10), First Page (S11), text on page 2, rotation of a page in continuous mode, tabs (scroll restored), Ctrl + wheel (`-Wheel`), View menu, dark theme, short window.
- [x] SUNI-031c (edit-advanced slice) Advanced content editing and rotated pages: engine in **display space** (`geometry.rs`: objects, text, search, highlighting, notes correct on `/Rotate` 90/180/270 and cropped pages, plan hypotheses H1–H7 verified at pixel level), **opacity** (texts, shapes, images; alpha kept on colour change), **free rotation** (handle, Shift 15°, Angle field, oriented frame), **multiple selection** (rectangle, Alt, Shift/Ctrl+click, Ctrl+A, direct grab on drag) and grouped move/delete/duplicate/order/style, **object clipboard** (real Ctrl+C/X/V through the egui Copy/Cut/Paste events, pasting across pages and tabs as an XObject group, duplication without bringing the original to the front), **Objects panel** (F5, F3 since SUNI-012b), **text on the page** (mini-bar, several lines, existing upright text editable in place), embedded **system TrueType fonts** (list via the registry, `fsType` licence, typed refusal of missing characters, including in Helvetica), **replace image**, **blank page**, **crop** (preview), **resize** (keep / fit); contextual object bar under the selection; fixed along the way: dead Ctrl+C, duplication bringing the original to the front, alpha lost on colour change, size of rotated texts (0 to 90°, multiplied on edit), stamps on rotated pages, "ÿ" of non-Latin texts in a standard font; 9 commands (78 in the registry), 83 new i18n keys (481); CLI `objects`, `add-text`, `set-opacity`, `rotate-object`, `copy-objects`, `replace-image`, `crop`, `resize-pages`, `insert-blank`, `fonts` exercised on the fixtures; fixture `boxes.pdf`; 11 integration tests + 8 engine unit tests, 5 application unit tests, 1 CLI test. Screenshots inspected (FR light, EN dark): Objects panel, selection and handles, free rotation, rubber-band rectangle and grouped move, Ctrl+C/Ctrl+V, pasting from one tab to another, text on a rotated page, text on the page + mini-bar, filtered font picker, in-place editing of an existing text (F2), Crop (preview) / Resize / Blank page dialogs, crop applied.
- [x] SUNI-040 (tool-hub slice) "All tools" hub: `workspace.rs` (table of the 14 tools by task, 9 visible because actually implemented: Export, Edit, Create, Combine, Organize, Comment, Protect, Compress, Add a stamp), left panel with coloured chips (F9), contextual bar per tool (grouped buttons, labels depending on the width then compact mode and scrolling, actions greyed without a selection, Leave tool / Esc / back), Tools menu, 6 `hub.*` commands (69 in the registry, `sunipdf-cli commands` verified), single-letter shortcuts that open the tool, active tool per tab, home with fully clickable cards + "Open file" card, quick access Open/Save/Undo/Redo; floating palette removed, object bar and grid bar reduced; chrome polish (filled button readable in dark, contrasted glyphs, fades, focus ring, shadows, active tab underlined, icons); 12 tests (table, registry coverage, contrast ≥ 3.5:1, tool toggling and tracking, egui pass without a window: geometry, AccessKit tree, keyboard focus). Screenshots inspected: home light/dark/English, Comment, Edit (+ rectangle), Organize, Protect, Esc, search + Esc, Highlight by shortcut, Stamp + dark dialog, tabs, palette, narrow window (740 px).
- [x] SUNI-033 Stamps, watermarks and page numbers: "Stamp or Watermark" dialog (text, size, colour, bold, angle, opacity, 7 positions, all pages or current page) and "Number Pages" (templates `{n}`, `{n} / {total}`, `Page {n}`), text objects added to the content, hence editable and undoable; engine `sunipdf-core::stamp` (`stamp_pages`, text opacity in `TextStyle`); CLI `stamp` and `page-numbers` exercised on the fixture (text re-read: "BROUILLON 1/2", "Page 1"); 1 engine test (page centre, 45° rotation, dimmed pixels, numbers at the bottom right, persistence).
- [x] SUNI-032 Visual polish and editing: common dialog frame (`dialog.rs`, icon/title/subtitle header, primary/secondary/danger button footer), About with logo and Navanem credit, tools panel and home with category-coloured icons, tools usable without a document (open, then deferred execution), editing: ±90° rotation, duplication, bring to front / send to back (lopdf content stream merge + reinsertion), arrows to nudge, Esc, proportions (Shift), action buttons in the object bar; 5 commands; 1 engine test (rotation, duplication, Z-order verified on render); engine tests serialised on a single engine.
- [x] SUNI-031b Advanced editing: resizing by corner handles (live preview, dedicated cursors), object properties bar (stroke colour/width, fill; "Apply to object"), Helvetica / Times / Courier fonts + bold / italic, size and colour of existing text (F2), shape preview while drawing, image inserted with its proportions (top-left corner under the click); fix of the drag origin (press position); engine `resize_object`, `set_object_style`, `set_text_size`, `FontFamily`, colour/font info in `ObjectInfo`; 1 additional engine test (resizing, recolouring verified on render, Times italic, ×2 size). Screenshots verified: move, resize, ellipse with bar, F2 dialog.
- [x] SUNI-031 Page content editing: "Edit" category and tools in the palette (Select Object E, Add Text T, Insert Image I, Rectangle R, Ellipse O, Line L), selection of an existing object on click (frame + handles), move by dragging, deletion (Del), modification of an existing object's text (F2 or double-click, original font kept), Helvetica / Helvetica-Bold text addition with size, image insertion (PNG, JPEG, BMP, GIF, TIFF, WebP; 200 pt wide), vector shapes; everything undoable and saved in the content stream (not as annotations); engine `sunipdf-core::edit` (PDFium `FPDFPageObj_*`, content regeneration); 1 engine test (addition, hit-test, modification, move, deletion, image and shapes verified on render, persistence after saving). Remaining limits after SUNI-031b: no object rotation, fonts limited to the 14 standard ones, existing text limited to the characters of its font.
- [x] SUNI-012a Multi-document tabs: several PDFs open (tab bar, Ctrl+Tab / Ctrl+Shift+Tab, closing per tab, home without closing), inactive documents suspended in the engine (bytes + undo history kept) and interface state kept per tab; save confirmation tab by tab on closing; several files accepted as command-line arguments.
- [x] SUNI-030 Windows delivery: original icon generated by `scripts/make_icon.py` (Pillow, no third-party resource), icon + version information + PerMonitorV2 DPI manifest embedded in `sunipdf.exe` (`build.rs`, `winresource`), window icon, `scripts/package-portable.ps1` (portable ZIP + SHA256, x64/arm64), `packaging/wix/SuniPDF.wxs` + `scripts/package-msi.ps1` (WiX v3), `.github/workflows/release.yml` (portable x64 + ARM64, x64 MSI, signing if a certificate is present), FR/EN user guides.
- [x] SUNI-024 / 014d File tools in the interface ("Convert" category, menus, palette): combine PDFs, split ("pages per file" dialog), extract the selection, create a PDF from images (PNG, JPEG, BMP, GIF, TIFF, WebP), export the pages as 150 dpi PNG, lossless optimisation (lopdf recompression, unused objects); tasks on the engine thread with an "in progress" state; CLI `images2pdf`, `export-images`, `optimize`; 1 engine test (images → PDF rendering verified, PNG export, optimisation with bookmarks kept).
- [x] SUNI-022/023 Bookmarks (F6 panel, navigation), document properties (Ctrl+D: editable metadata, version, size, encryption, permissions), AES-128/AES-256 password protection with permissions (dialog + CLI `protect`), protection removal (`unprotect`), metadata removal (`strip-metadata`), `set-metadata`, `properties`, `bookmarks` in the CLI; engine `sunipdf-core::security` (lopdf for writing, PDFium for re-reading); 2 engine tests including an encryption/decryption round trip verified by PDFium.
- [x] SUNI-021 Annotations: text selection by dragging and copy (Ctrl+C), highlight / underline / strike through / note tools (floating palette, shortcuts V H U X N), Comments panel (F7, list per page, deletion), "Annotate" category, all undoable; engine `sunipdf-core::annot` (PDFium generates the appearances); 1 engine test with rendering verification.
- [x] SUNI-014c Snapshot-based Undo/Redo (Ctrl+Z / Ctrl+Y, 50 levels, 512 MB max), insertion of pages from another PDF, moving pages (drag and drop in the thumbnails and in the Organize view, forward/backward buttons), **Organize Pages** view (F8: grid, multiple selection click/Ctrl/Shift, grouped actions), `SUNIPDF_STARTUP_COMMANDS` variable for screenshot automation; CLI `insert`, `move`; 1 engine test (insertion, move, snapshot, encrypted).
- [x] SUNI-020 (UI) Shell: tab bar (home, document, + Open, language, help, theme), toolbar (Menu, All tools / Read / Pages categories, command search), "All tools" panel, home page (cards, persisted recent files, privacy), right rail (thumbnails, search, page, zoom, fit modes, rotation), Ctrl+K command palette, Segoe UI + Segoe Fluent Icons system fonts loaded at run time, reworked light/dark themes. Screenshots verified by `scripts/dev-screenshot.ps1`.
- [x] SUNI-014b (UI) Pages menu (rotate right/left, delete page), Save / Save As (Ctrl+S, Ctrl+Shift+S), "modified" state in the title, Save / Don't Save / Cancel prompt before closing, opening or quitting (including the window's close button). 20 commands in the registry.
- [x] SUNI-014 (engine + CLI) Merge, extraction, split, rotation, page deletion, atomic save (`sunipdf-core::pages`, `write_atomic` shared with the preferences); CLI `merge`, `extract`, `split`, `rotate`; 4 tests.
- [x] SUNI-013 Full-text search (Ctrl+F, search bar, case, next/previous match, highlighting on the page) and text extraction; CLI `text` and `search`; 1 engine test.
- [x] SUNI-011 Capped LRU render cache (256 MB, `sunipdf-core::cache`), pre-rendering of the neighbouring pages, virtualised thumbnails panel (F4), 2 cache tests.
- [x] SUNI-010 Command registry (`sunipdf-core::command`): 57 commands, menus and shortcuts routed by `SuniApp::execute`, 3 tests.
- [x] Tests executed: `cargo test --workspace` → see "Last run".
- [x] Smoke test: `sunipdf.exe two-pages.pdf` starts and stays alive (window not visually inspected by the agent).
- [x] Real screenshots (home, document, palette) verified by the agent through `scripts/dev-screenshot.ps1`; final validation by the user to do.
- [x] MSVC build (reference target): Build Tools installed on the evening of 13/10, `cargo test --workspace` and `cargo clippy --workspace --all-targets -D warnings` green.
- [x] Windows icon / version resources (verified: `(Get-Item sunipdf.exe).VersionInfo` → SuniPDF, SuniWorks, 0.1.0).
- [ ] Historical CI observation (2026-10-14, v0.1.0/v0.2.0): workflows triggered; their outcome was not recorded in this initial checklist. Later 0.8.x release evidence is in the historical table below.
- [x] x64 MSI built locally with the standalone WiX 3.14 binaries: `dist\SuniPDF-0.2.0-win-x64.msi` (11.0 MB) + SHA256 (0.1.0 validated by `smoke.exe` and an administrative extraction; 0.2.0 rebuilt on 14/10, same process).
- [ ] Historical local ARM64 attempt (2026-10-14): missing Visual Studio ARM64 tools prevented linking. Later CI package builds do not prove ARM64 hardware execution (see `KNOWN_ISSUES.md` no. 12).

## Historical 0.8.3 validation (2026-10-16, **MSVC** target, `-j 2`)
| Command | Result |
|---|---|
| `cargo test --workspace --locked -j 2` | 276/276 passed, 3 ignored (2 measurements and 1 live network check): core 57 unit + 84 integration + 7 protection, application 102, i18n 6, CLI 10 unit + 1 `add_image` + 6 `annotations_cli` + 2 `forms_cli` + 1 `redact_cli` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo fmt --all -- --check` | OK |
| `scripts\check-i18n.ps1` | OK: 726 bilingual keys (fr-FR / en-US identical) |
| Live updater download | Explicitly ran the ignored network check after publication: public 0.8.3 x64 ZIP downloaded through production HTTPS/redirect checks, size and SHA256 verified; no installation. |
| Helper `-SelfTest` | Passed: path bounds, checksum rejection, ZIP traversal/limits, architecture, complete copy, rollback and unrelated-file preservation. Full installer launch and real installation not executed. |
| Distribution 0.8.1 | CI and release passed; published under lmdanavanem, six public assets downloaded and SHA256 verified. x64/ARM64 architectures and desktop version checked; x64 CLI version, PDF reading and rendering passed. |
| Distribution 0.8.2 | Published as Latest under lmdanavanem. x64 CI passed; ARM64 build passed. MSI packaged from the verified x64 CI binaries and statically validated; duplicate x64 release build cancelled. Six public assets downloaded and SHA256 verified; English resources, architectures, license contents, CLI reading/rendering and MSI rollback sequence checked. |
| Distribution 0.8.3 | [Published as Latest under lmdanavanem](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.8.3), from `cf79e74`. x64 CI and ARM64 build passed; duplicate x64 release build cancelled. Six public assets downloaded and SHA256 verified, then copied to `dist/`. GUI/CLI versions, x64/ARM64 architectures, package contents and licenses checked. x64 CLI reading/rendering and opacity rejection passed. MSI payload matches the CI ZIP; upgrade removal is inside the rollback transaction. WiX static validation passed with the existing ICE61 same-version-upgrade warning. |
| Continuous view measurements (AMD Ryzen 7 6800HS, 13.7 GB, NVIDIA RTX 3050 Laptop, Windows at 150%) | `SUNIPDF_PDFIUM_DIR=third_party\pdfium\win-x64\bin cargo test -p sunipdf-core --release -j 2 --test engine -- --ignored --nocapture render_scroll_100_pages_timing`: 100 pages of `many-pages.pdf` at 1.5 px/pt as whole pages, 150 to 167 ms in total (1.5 to 1.7 ms per page, first one 4 to 9 ms); 10 pages at 6 px/pt as 200 tiles, 230 to 239 ms (1.2 ms per 1024² tile, first page 24 ms), two runs. `cargo test -p sunipdf-desktop -j 2 -- --ignored --nocapture layout_cost` (debug): `Layout::new` + `visible` 14 µs for 500 pages, 144 µs for 5000 pages per frame. Memory (`target\debug\sunipdf.exe`, `many-500.pdf`, 300 Next Page / End / 300 Previous Page on the keyboard): private 176 MB on opening, plateau 810–827 MB, WorkingSet 438 MB (A28) |
| Distribution 0.8.0 | CI and GitHub release passed; six public files downloaded and SHA256 verified (x64/ARM64 ZIP, x64 MSI and checksums). Binary versions and architectures verified; MSI payload matches the x64 ZIP. WiX validation passed with an ICE61 warning related to `AllowSameVersionUpgrades`. x64 CLI reading and rendering checked; ARM64 not executed. |
| Screenshots `scripts\dev-screenshot.ps1` | annotations-advanced (`two-pages.pdf`, `annotated.pdf`, `-Commands hub.comment,tool.ink` etc.): ink drawn, Free text box / Link / address confirmation dialogs, Comments panel as threads in light and dark, selecting, moving and resizing a SuniPDF annotation (`ann2.pdf`); forms (`acroform.pdf`, `-Commands forms.fill`): 16 highlighted rectangles and a panel with 13 fields, click on the "Accepte" checkbox (tick drawn by PDFium) then on the "carte" radio (moved) then on "Pays" (floating list France / Belgique / Canada with "(none)"), "Canada" chosen and re-read with the regenerated list arrow, click on "Nom" then `Jean{TAB}Paris{TAB}Deux lignes{ENTER}` (Tab selects the content of the next field, Enter inserts a line break in the multiline field), Flatten dialog, flattened with Enter (values in the page, "no fields" panel, greyed tool, status message), Ctrl+Z after flattening (fields and panel back, title intact), "This document contains a form to fill in." banner without the tool, `acroform-xfa.pdf` (XFA banner, grey fields, greyed commands), dark theme + English with the list open (preferences restored); the script targets `GetWindowRect`: a click at (x, y) arrives at (x − 8, y − 31) on the egui side (`RUST_LOG=trace` trace); sign-image: empty Sign dialog (warning box, white pad, Place button greyed), three strokes → signature placed on the blue rectangle and selected (handles, object bar, modified tab, PNG written into `SUNIPDF_SIGNATURES_DIR`), Ctrl+Z (signature gone, title intact), Fill & Sign tool of the hub (Sign / Initials / Date bar, active row) + library with two thumbnails, thumbnail clicked then placed under the rectangle, date ghost under the pointer with a status message, date `15/10/2026` placed and selected, Esc (nothing added), import of a photo (`SUNIPDF_PICK_FILE`) with the background and shadow removed at threshold 200, Initials in dark theme + English; review-fixes (2nd pass): Protect with "abc" typed on opening (lands in the open password field), stamp Position list open (seven entries visible); review-fixes: Document Properties in a 720 × 480 window (header, scrolling body and buttons visible), Properties in English + dark ("File size 1 KB"), Stamp without text (Apply greyed), Strike Through tool + Comments panel ("Strikethrough"), Protect dark + English with a readable error ("Enter at least one password."), Rotate then Ctrl+Z (title without "modified", Save greyed, Redo active); earlier slices (home, document, palette, Organize, annotations, bookmarks, properties, protection, tabs, light/dark/English tool hub…); edit-advanced: Objects panel, selection + rotation handle, free rotation at 77°, rubber-band rectangle + grouped move, keyboard Ctrl+C/Ctrl+V (two offset pastes), Ctrl+A/Ctrl+C/Ctrl+Tab/Ctrl+V from one tab to another, text added upright on page A (`/Rotate 90`) of `boxes.pdf`, on-page editor with Shift+Enter, filtered font picker ("ari"), F2 on an existing text, Crop (preview) / Resize / Blank page dialogs, crop applied, dark theme + English: inspected by the agent |
| `sunipdf-cli` | annotations (`annotations_cli` test): `annotate` ×6 kinds on `two-pages.pdf` re-read by `annotations` (indices, subtypes, rectangles, author, colours), reply + status + `annotation-delete --thread` (3 removed), 10 typed errors (`file:` and `javascript:` addresses refused), `already_exists` (code 6), annotated RC4 document then `password_required` (code 3); forms (`forms_cli` test on the fixtures): `form-fields acroform.pdf` → 13 fields, `kind acroform`, `ville` "Élodie Müller", `pays` combo with options and widget on page 1 rect [150, 436, 350, 456]; `--values-only` → `{ nom: valeur }` without `signature`; `fill-form --set nom=Jean --set accepte=true --set "langues=Français;Allemand"` → `applied 3` re-read by `form-fields`; `fill-form --values values.json --flatten` → `kind none`, `count 0`, pixel-level rendering (> 300 dark pixels in "nom"); `--set inconnu=1` → `form_field_not_found` (code 1) without a file, `--ignore-unknown` → `skipped.unknown ["inconnu"]`, `skipped.invalid[0].field "ref"`; `--set ref=x` → `form_value_invalid`; `flatten` → `annotations ≥ 16`, then existing output → `already_exists` (6); `acroform-xfa.pdf` → `form_xfa`; `two-pages.pdf` → `kind none`. sign-image: `add-image two-pages.pdf --image signature-transparent.png --x 100 --y 520 --width 150` → `height_pt 50`, `digital_signature false`, rendering re-read at pixel level (621 ink pixels, 9,879 blue pixels visible between the strokes in the area, none elsewhere); `--image signature-photo.jpg --remove-background 200` → `background_removed true`, clean rendering (background and shadow removed, 613 ink pixels); `--remove-background 1` → `other` "image is empty after background removal" (code 1); missing image → `io` (5); existing output without `--overwrite` → `already_exists` (6); `--page 7` → `page_out_of_range` (1). review-fixes: `protect --aes128` with "пароль" or "œuvre" → `password_not_latin1` (code 1), with "Été" → file reopened by `properties --password`; `extract` and `split` of an AES-256 file → `"encryption_removed": true` (false on an unencrypted file); `rotate 90` then `text --page 1` → "SuniPDF page 1" (text at y = 700 > 612, lost before); `add-text` "Привет" in Helvetica → `missing_glyphs` with the list of characters. continuous-view: `render --page 1 --scale 40 --region 4000,7000,256,256` → solid blue 256 x 256 PNG, `page_pixels` [24480, 31680]; region outside the page → `other` (code 1); whole page at ×40 → `render_too_large`. edit-advanced: `objects`, `add-text --font-name Arial` (Cyrillic re-read by `text`), `set-opacity`, `rotate-object`, `copy-objects --into`, `crop --unit mm`, `resize-pages --mode scale`, `insert-blank --like`, `info` (rotation), `fonts` (323 fonts), `missing_glyphs` (code 1) and `already_exists` (code 6) refusals exercised on the fixtures; other commands exercised by the earlier slices |

## Not done (do not confuse with "done")
View rotation, complex untagged-column reading order, font subsetting/CFF/TTC (whole TrueType fonts only), editable
object paste, group rotation/resizing, more field types and FDF/XFDF, editable XFA, Office conversion, certificate
trust/revocation/trusted timestamps/LTV, ARM64 hardware validation and broader real-corpus/performance QA.

## Log
- 2026-10-16: 0.9.0 implements the seven requested features. Workspace: 318 passed/4 ignored, Clippy and formatting
  passed, 804 bilingual keys, updater self-test and actual Microsoft Print to PDF output test passed. Published as Latest
  under `lmdanavanem`; all six public files/SHA256, package payloads, French OCR, fields and live updater download verified.
- 2026-10-16: version 0.8.3 completes the four P1/P2 backlog fixes: finite text coordinates, bounded
  Organize/sidebar thumbnail textures and requests with stale/failure handling, single-line stamps and
  page-number templates, and explicit CLI rejection of unsupported free-text opacity.
- 2026-10-16: version 0.8.2 adds startup notifications, verified downloads and explicit installation after
  document-close confirmations. English is the default interface and primary documentation language;
  French and existing preferences are retained. MSI upgrades support rollback.
- 2026-10-15: version 0.8.1 fixes tab close buttons; four pointer tests cover active-tab closing,
  background-tab closing with/without changes, title navigation and refusal during a save.
- 2026-10-15: version 0.8.0, six priority fixes: effective passwords, Form panel drafts, selections during tab
  changes, closing with pending mutations, moving pages while preserving fields/bookmarks/links, and hiding
  password fields in the full JSON listing.
- 2026-10-15: annotations-advanced (advanced annotations, D067–D076): see the ticked box at the top. Deviations from the plan:
  encrypted documents stay encrypted instead of being refused (D067); appearances in display coordinates with a rotation matrix
  (D068); the seven tools join the hub's Comment tool instead of a floating palette (D074). Screenshots: the harness's mouse
  drag is reliable once calibrated (window coordinates = capture pixels / 1.5 at 150 %, first press at the stale position of
  the window centre: precede with a warm-up drag; a click = zero-length drag `x,y,x,y`); a panel that widened by itself every
  frame came from a frame nested in a `horizontal` row (fixed with `outer_margin`). 207 tests, clippy, fmt and i18n (676 keys) green.
- 2026-10-15: sign-image re-verified on HEAD (slice already delivered by e4d3016 → 786e83f): screenshots A (empty
  dialog), B (three strokes placed on the blue rectangle, selected, blue visible between the strokes, PNG written into
  `SUNIPDF_SIGNATURES_DIR`), D (library with one thumbnail) replayed; 10 px gutter for the dialogs'
  scrollbar (A37 reduced). Along the way, the forms review fix found uncommitted was taken over
  (links kept on flattening: deep copy, D062 corrected; debug `eprintln!` removed);
  167 tests, clippy, fmt and i18n (612 keys) green.
- 2026-10-15: forms (AcroForm forms, D061–D066): `acroform*.pdf` fixtures, `forms` / `form_data` engine
  (lopdf reading and writing, PDFium appearances written into the file, flattening, JSON), `form_*` errors,
  CLI `form-fields` / `fill-form` / `flatten`, Fill form tool (per-page overlay, placed editor,
  lists), Form fields panel, banner, Flatten dialog, JSON export/import; 165 tests, 611 keys;
  field creation and XFDF/FDF on the roadmap.
- 2026-10-15: review-fixes, 2nd pass (independent review of the slice): AES-128 limited to the printable Latin-1
  that PDFium reopens (€, œ, typographic dashes and quotes refused; engine round-trip test, A30 and D047
  corrected); atomic cut in the engine thread; de-duplicated list of missing characters; focus in
  Protect; seven stamp positions visible; tabs' "modified" indicator aligned with the engine;
  A34 (XMP regenerated without `pdfaid`). 130 tests, clippy, fmt and i18n green, two screenshots inspected.
- 2026-10-14: review-fixes slice: 39 findings of the adversarial reviews replayed on HEAD then fixed at the root (engine, engine thread, interface, CLI), decisions D046–D053; 129 tests, clippy and fmt green, 507 i18n keys, screenshots inspected (Document Properties 720 × 480 and English dark, Stamp, Comments, Protect dark, undo back to the saved state).
- 2026-10-13: study of the existing readers, PDFium chosen (D005), skeleton → vertical slice 1, GNU environment for lack of MSVC (D008),
  wgpu disabled (D010), fixtures generated (`scripts/make_fixtures.py`), git repository initialised.
- 2026-10-14: SUNI-031 content editing (`edit.rs` engine + "Edit" category, D019); 36 tests, clippy green, screenshots verified.
- 2026-10-14: SUNI-031b advanced editing (handles, colours, fonts), version 0.2.0, `CHANGELOG.md`, `v0.2.0` tag pushed (GitHub Release produced by `release.yml`, to verify in the Actions/Releases tab).
- 2026-10-14: tool-hub slice (SUNI-040): "All tools" hub, contextual bar, chrome polish (D021–D026); 55 tests, clippy and fmt green, screenshots inspected.
- 2026-10-14: hub review fixes: Esc with an open menu closes the menu (single point `workspace::take_escape`, order tested), distinct rotation and move icons, object bar bounded to the document area, palette, quick access focus, absolute path of the recent files; 58 tests, clippy and fmt green, screenshots inspected.
- 2026-10-14: SUNI-032 visual polish (dialogs, home, coloured tools, About) + editing (rotation, duplication, Z-order, keyboard), version 0.3.0, `v0.3.0` tag.
- 2026-10-14: continuous view review fixes: Shift+Space scrolls up one screen instead of down (read before Space, test without a window), stale textures drawn under the up-to-date renders of another scale, scripted commands (`SUNIPDF_STARTUP_COMMANDS`) executed after the document's first frame (3 × zoom out: 33% from the 71% fit), resumed tab without state brought back to the top, right rail in a short window limited to whole icons with a visible scrollbar; 112 tests, clippy and fmt green, screenshots inspected (Shift+Space, 1000 x 560 / 680 and full-height windows).
- 2026-10-14: SUNI-012b continuous view (continuous-view slice): continuous scrolling by default, tiles, anchored zoom, virtualisation, measurements; decisions D038–D045; 111 tests, clippy and fmt green, screenshots inspected.
- 2026-10-14: SUNI-031c advanced editing (edit-advanced slice): display space, opacity, free rotation, multiple selection, object clipboard, Objects panel, text on the page, system fonts, page frames, CLI; decisions D027–D037; 83 tests, clippy and fmt green, screenshots inspected.
