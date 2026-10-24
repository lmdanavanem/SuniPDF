# Deducted areas and constant-depth volumes

This increment adds holes to polygon area measurements and volumes calculated from a net area and an explicitly entered physical depth. It uses the same calibration, preview, editing, history and quantity infrastructure as other measurements. The published 0.63.0 release passed source and public-package checks, with separate native development-build workflow evidence. See [release qualification](RELEASE_0.63.0.md) for evidence and limits.

## Calculation and units

The outer polygon defines the gross area. Each interior ring defines one deduction, in its input order:

```text
net area = gross area - sum(deductions)
volume = net area * physical depth
```

For example, a 10 m by 10 m outer boundary with a 2 m by 5 m hole gives 100 m² gross, 10 m² deducted and 90 m² net. A depth of 0.2 m gives 18 m³. Entering the depth as 20 cm produces the same physical volume.

The result unit and depth unit are independent. Supported length units are `mm`, `cm`, `m`, `in` and `ft`; area results use squared units and volume results use cubic units. Changing the result unit converts the calibration and leaves the depth input unchanged. Explicit recalibration changes the measured area; it does not reinterpret the physical depth. Changing the depth itself requires an explicit new value and unit.

Depth must be finite and between `1e-9` and `1e9` in its stated unit. The calculated result must be finite and between `1e-9` and `1e12` in its result unit. Display precision accepts 0–4 decimal places. Stored values retain calculation precision; the label is rounded for display.

## Drawing and editing

In Measurements, choose polygon area or volume and confirm the scale. Draw the outer boundary first, then add and select the deduction rings. A volume also requires a depth and its unit. Review the gross area, total deductions, net area and volume before applying the candidate. The PDF changes only after Apply succeeds.

The geometry editor treats the outer boundary and each deduction as separate selectable rings. Removing a deduction changes the net area; it does not remove the outer boundary. Translations and rotations must move every ring together. A depth-only change preserves the polygon geometry. An unfinished or invalid ring cannot be committed.

Preview is bound to the source document. A changed source requires a fresh preview. A successful operation is one history entry, so Undo and Redo restore the annotation and its metadata together. Save the PDF to retain the result on disk. The native check exercised deduction vertex insertion, Preview/Apply, Undo/Redo and save/reopen; it did not establish every pointer gesture or external-reader behavior.

Appearances draw the outer and interior outlines, without a filled polygon or opaque label background. They do not conceal the plan inside a deduction. The volume label is a numerical quantity, not a rendered 3D solid.

## Geometry limits

Each measurement supports at most 16 deduction rings, 128 vertices per ring and 512 vertices in total, including the outer boundary. Each ring needs at least three distinct vertices. Do not repeat the starting point to close a ring; closure is automatic.

All rings must be simple and nondegenerate. Holes must lie strictly inside the outer boundary. Touching an outer edge, touching another hole, intersections, overlaps and nested holes are refused. The net area must be strictly positive. Vertex order may be clockwise or counterclockwise.

Coordinates use the displayed page after crop and rotation, with a bottom-left origin. Vertices and calibration points must remain inside the page. The shared engine also validates the geometry recovered from PDF storage. A narrow edge that collapses after coordinate conversion, a changed rounded label or excessive precision loss is refused before any annotation is written. The engine does not clip a polygon or silently discard an invalid hole.

## CLI examples

The following examples use a reference of 10 m per 100 page points. Use a PDF large enough to contain all listed points, and choose a new destination for each command.

```powershell
sunipdf-cli measure input.pdf --kind area --points "40,80 140,80 140,180 40,180" --hole "60,100 80,100 80,150 60,150" --calibration "40,40 140,40" --known-distance 10 --unit m --out net-area.pdf
sunipdf-cli measure input.pdf --kind volume --points "40,80 140,80 140,180 40,180" --hole "60,100 80,100 80,150 60,150" --depth 20 --depth-unit cm --calibration "40,40 140,40" --known-distance 10 --unit m --out volume.pdf
sunipdf-cli measurements volume.pdf --page 1
```

Repeat `--hole` for additional deductions. A volume can also have no holes. `--use-page-calibration` selects a supported saved page scale instead of `--calibration` and `--known-distance`. Page numbers start at 1.

`edit-measurement` requires the expected full record, its annotation index and a new output path. Omitting `--hole` or `--depth` preserves the existing input. Supplying any `--hole` replaces the complete hole list; `--clear-holes` explicitly removes it. `--depth` requires `--depth-unit`. Converting a volume to an area requires `--kind area --clear-depth`; an incompatible retained unit price causes the edit to be refused. Changing `--unit` alone never changes the physical depth.

An ordinary unit or precision edit cannot reactivate a stale or legacy measurement. Explicit recalibration against the current document is required. Source paths, aliases and existing output files are refused for writes. Use `--password-env` or `--password-stdin` for protected inputs. Annotation permission is required and permitted encryption is retained. When copying is forbidden, a permitted annotation write does not export the measurement geometry or result.

## Saved records and quantities

Deducted areas and volumes use private measurement metadata version 3. Ordinary measurements continue to use version 2; older version 1 and 2 records remain readable under their existing validity rules. Unknown schemas, malformed depth values and unsupported geometry are refused rather than guessed.

Readable JSON includes `holes`, `depth`, `area` with `gross`, `deductions` and `net`, plus `area_unit`. Volume results use `kind: "volume"` and a cubic unit such as `m3`. Current records provide `value`, `label` and `area`. Unverified or stale records expose their historical result separately as `historical_value`, `historical_label` and `historical_area`; the current-value fields are null.

The quantity table supports `mm3`, `cm3`, `m3`, `in3` and `ft3` as volume units. Unit-price conversion must preserve the physical dimension: a price per cubic metre can follow a conversion to cubic centimetres, but cannot silently become a price per square metre. Compatible edits preserve quantity fields and prices. Edits or recalibrations that exceed pricing bounds are refused before saving.

Only current measurements contribute quantities, line costs and quantity-dependent formulas. Historical inputs and stored prices remain visible for review, while inactive values are excluded from totals rather than represented as zero. Explicit recalibration restores eligible quantities without clearing the user's stored fields.

## Scope and validation

This feature covers polygon deductions and a single constant depth. It does not implement variable depth, sloped solids, automatic material classification, automatic recognition of openings, a 3D model, or persistent rectangular constraints during later polygon edits. Other PDF applications can display the saved appearance; editable measurement interoperability requires separate qualification.

The final source workspace and CLI corpus passed regression cases for 90 m² / 18 m³ calculations, depth-unit equivalence, invalid topology, rotation and crop, storage precision, encrypted permissions, incompatible prices, recalibration, history and reopening. A native deduction edit was saved and reopened, then checked independently: 65 assertions confirmed retained geometry, physical depth, quantity, appearance and source preservation. The downloaded release CLI also passed the plan/form/volume corpus (20 commands / 680 assertions). This is bounded synthetic-document evidence; private project plans, physical measurement accuracy and editable interoperability remain unqualified. [Qualification scope](RELEASE_0.63.0.md).
