# SuniPDF 0.44.0 — editing saved measurements

[Français](RELEASE_0.44.0.fr.md) · [Measurement guide](MEASUREMENTS.md)

Status: published as stable Latest [0.44.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.44.0), source `7e95078127beb81b90a9eac32e6fde0fd8e9bf50`. The preceding release and its evidence remain [0.43.0](RELEASE_0.43.0.md) · [0.43.0 evidence](qa/RELEASE_0.43.0.json).

## Audit before this release

| Finding | Delivered correction |
|---|---|
| The manual measurement dialog supplied an arbitrary 10 m reference over 100 PDF points. | No implicit positive reference distance; a valid explicit calibration is required before preview/application. |
| Saved measurements could be inspected or reused, but not edited. | An explicit editor changes the selected measurement's reference distance, unit and precision through Preview then Apply. |
| The inspector copied the complete page list and generated result JSON every frame. | Records and calculated labels are shared in memory; pagination remains bounded to 50 rows. |
| A temporary scale could override the saved default without a direct way back. | Use saved page scale or Clear temporary scale changes only the current page's session state. |
| Measurement exports did not consistently check copying permission, including details returned by mutation commands. | Read/export commands require copying permission. Permitted creation/editing without it returns `measurement: null` and `export_unavailable: true`. |
| Resizing content and annotations changed measurement rectangles without updating their private geometry. | That resize mode refuses pages containing supported or recognized imported measurements before committing changes. |
| Replacing an existing measurement needs stronger checks than displaying it. | Editing requires an unchanged expected record, stable unique identity, supported metadata/appearance and annotation permission. Locked, shared, stale or externally changed targets are refused. |

## Desktop

Open **Measurements → Saved measurements → Edit measurement**. Change the known reference distance, unit or 0–4 decimal precision. A unit change converts the reference distance physically; a distance change recalculates only this measurement. Angle editing exposes precision only. The measured vertices and reference points remain fixed in the desktop editor.

Inspect the rendered **Preview**, then select **Apply**. Replacement retains annotation identity, author, replies and register fields. **Cancel** leaves the PDF unchanged; successful application uses one Undo/Redo step. Save the PDF to retain the change. Page defaults and other measurements are not recalculated.

**Use saved page scale** discards a temporary override when a valid default exists. Otherwise **Clear temporary scale** removes only the temporary calibration. Neither action modifies the PDF or another page's session scale.

## CLI

`edit-measurement` requires `--page`, `--index` and `--expected-record`, a complete record obtained from `measurements`. It can change kind, vertices, calibration, unit or precision. Explicit calibration requires both `--calibration` and `--known-distance`; `--use-page-calibration` selects the saved page default. Converting an angle into a scaled measurement requires an explicit physical calibration. Angles can be created without a page scale.

Writes require a new `--out` copy, including with `--overwrite`. Existing destinations and original files remain protected. Supported encryption and permissions are retained. Detailed JSON export requires copying permission; annotation permission remains independently required for editing. [Examples and bounds](MEASUREMENTS.md#cli-and-quantities).

## Validation and remaining limits

The validation scope includes real egui control interactions, unit conversion, preview/cancel, stale targets, replacement without duplication, Undo/Redo, save/reopen, encrypted copies, restricted exports and independent qpdf checks. Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39718850233) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39718862137) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39718850516). 702 tests passed, 10 default ignored. 1250 EN/FR keys. 36 portable scenarios. 30 MSI baselines. 150 MSI logs. No open Code Scanning alerts. Six public downloads and dist copies verified by SHA256. 21 packaged CLI calls verify conversion, precision, recalibration, geometry, stale refusal, identity, page defaults, reopening and an angle without calibration. Final value: 40 m; other measurements and originals preserved. [Evidence](qa/RELEASE_0.44.0.json).

Desktop vertex dragging, batch recalculation, vector snapping, multiple-scale viewports, volumes and real-plan dimensional qualification remain open. Editing is limited to supported measurements in documents up to 128 MiB; unknown or externally modified structures are preserved by refusal. Resizing measured pages is not silently repaired. Local tools remain free and open source. Signing, native ARM64, native accessibility, physical printing and 1.0 acceptance gates remain open.

The planned local sequence is 0.44 editing and corrective work, 0.45 a reusable annotation library, 0.46 quantity columns/totals/export, and 0.47 overlays/revision review. Each stage needs its own scope and evidence; this release does not complete the broader AEC cards. [Roadmap](ROADMAP.md).
