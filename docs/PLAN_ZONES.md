# Rooms and zones

Rooms and zones give annotations a named location on a plan page. A zone is a rectangle with a
stable identifier and an optional parent. Its membership is derived from the current annotation
geometry, rather than stored as an unchecked annotation label. Zones do not infer room boundaries,
measure floor area or replace measurement calibration.

## Define and review zones

Navigate to the page, then open **Rooms and zones** from **Tools → Review & measure**, the command
palette, or the annotation register's **Manage rooms and zones** action. Draw a rectangle on its
actual rendering or enter its bounds, and give it a name. The interface creates a stable ID and
preserves it when renamed; JSON input can specify IDs explicitly. An optional parent creates a
hierarchy such as `Building / Floor / Room`.

Bounds are displayed-page points, after the page's crop and rotation, with the origin at the
**bottom left**. Each rectangle must have positive width and height and remain inside the displayed
page. A child must be contained by its parent. IDs are unique within a page; the same ID on another
page is a different zone. Cycles and missing parents are refused. The limits are 64 zones per page
and eight hierarchy levels.

Review the before/after assignments before applying the replacement. Editing the zone list
invalidates the prepared preview. Applying is one document edit and supports Undo/Redo; save the
document to retain it. Reopen the saved PDF to inspect the stored hierarchy and the newly derived
assignments.

Zone metadata is separate from page scales and calibrated measurement regions. Changing a room
name or zone rectangle does not change a measurement's unit, scale, price or formula.

## Understand assignment and coverage

A row is assigned only when its whole annotation bounds fit one unambiguous deepest zone. A child
takes precedence over its containing parent. The register and quantity table share this rule.

| Status | Meaning |
| --- | --- |
| `assigned` | The complete annotation bounds fit one unambiguous deepest zone. |
| `boundary` | The bounds touch or cross a zone boundary; a center-point guess is not used. |
| `overlap` | Several incompatible zones could receive the annotation. |
| `unassigned` | Known bounds are outside the defined zones. |
| `geometry_unknown` | The annotation cannot provide supported, trustworthy bounds. |
| `stale` | Saved zone geometry no longer matches the page content or geometry fingerprint. |

Unfiltered reports retain unresolved rows and coverage counts. A zone filter requires an existing,
current zone. Coverage examines every row on that page before business filters. Filtering is
refused if the page contains boundary, overlap or unknown-geometry rows, because silently omitting
them would make its counts or quantities misleading. Resolve the
ambiguity before exporting a filtered result. Selecting a parent can include descendants explicitly;
each row is counted once. Quantity formulas and monetary totals use the same filtered rows and
retain their existing measurement-validity requirements.

## Changed pages and protected files

The saved page content and displayed geometry are fingerprinted. If they change, historical zones
are shown as stale. Review their location against the current page and explicitly reset the stale
geometry before replacing them. A reset is a new review decision, not evidence that an old room
boundary remains correct. Annotation-only edits recompute membership.

Existing modification permissions and signed-document guards apply. Passwords can be supplied
through the normal protected-file controls. A new copy of a protected source retains encryption;
passwords are not part of the zone JSON or exported reports.

## Command line

The JSON file is a complete replacement for one page, not a patch. An empty `zones` array removes
that page's zones. The input is bounded to 128 KiB and uses strict version 1:

```json
{
  "version": 1,
  "zones": [
    {"id": "floor-1", "name": "Floor 1", "parent_id": null, "bounds": [20, 20, 580, 780]},
    {"id": "room-101", "name": "Room 101", "parent_id": "floor-1", "bounds": [40, 40, 240, 280]}
  ]
}
```

Choose bounds that fit the actual displayed page, then inspect and preview:

```text
sunipdf-cli plan-zones inspect plan.pdf --page 1
sunipdf-cli plan-zones preview plan.pdf --page 1 --zones zones.json --preview-out page.png
sunipdf-cli plan-zones apply plan.pdf --page 1 --zones zones.json --out plan-with-zones.pdf
```

The PNG is a rendering of the actual candidate PDF page. Zone metadata is not painted into the
PDF's page content. Preview JSON reports the before/after zones and assignments. Page numbers are
physical and start at 1. `--expected-source-sha256` can pin an inspection or preview's source;
`--reset-stale` explicitly permits replacement after reviewing changed page geometry.

CLI PDF and PNG outputs must be new files, including when global `--overwrite` is supplied.
Publication rechecks the canonical source and its hash immediately before writing. Input PDFs are
bounded to 128 MiB. Use global `--password-env` or `--password-stdin` for protected sources.

Register and quantity commands accept `--zone PAGE:ID`, optional `--zone-descendants`, and
`--zone-status`. The descendant option requires a zone selection. JSON retains the assignment
object; CSV exports the zone status, ID and hierarchy path. These are derived locations, not a
certificate that every markup on a plan has been reviewed.
