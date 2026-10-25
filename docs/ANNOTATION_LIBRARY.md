# Annotation libraries

[0.61 release scope](RELEASE_0.61.0.md) Â· [User guide](USER_GUIDE.en.md)

## Desktop

Open **Tools â†’ Review & measure â†’ Annotation library**. The library is stored locally, separately from application preferences. You can manage it without an open PDF; creating a draft, capturing saved annotations or placing a template requires an open document.

The original template workflow shipped in [0.45.0](RELEASE_0.45.0.md); [its evidence](qa/RELEASE_0.45.0.json) does not qualify the new saved-annotation capture workflow.

1. Choose **Create a template** to start a new annotation draft. Supported capture includes text using a standard font, polygons/polylines, revision clouds, predefined stamps and PNG image stamps. Choose the type and prepare its text, geometry and style in the rich-annotation dialog.
2. Expand **Save to library**, enter a unique **Name**, then choose **Save template**. **Credit**, **Source or provenance** and **License or permission to reuse** are optional declarations; blank values remain unknown. Saving a template leaves the PDF unchanged and keeps the draft open. Close it with **Cancel** when you only want the template.
3. Reopen the library and choose **Place on page**. Set **Scale** in the placement bar, then click the page to set the template's lower-left origin. A frame follows the pointer; the complete template must fit the page. The click requests a rendered preview. In **Place annotation template**, you can refine **Origin X (pt)**, **Origin Y (pt)** and **Scale**; after changing them, request **Preview** again. Inspect the result, then choose **Apply to object**. **Cancel** or **Escape** discards the pending placement. Applying creates one Undo step; save the PDF to keep it.
4. To make a group, check at least two entries and choose **Create group**. Enter a new name, set each member's horizontal/vertical offset in points and choose **Save template**. **Cancel** abandons the group draft. A flat group contains at most 16 primitives; its source entries remain available. Placement gives each member a new PDF annotation identity.

**Import library** merges a validated file into the active library. Duplicate IDs or names, invalid entries or exceeded limits refuse the complete import. **Export library** and **Export selection** write a new JSON file; choose a filename that does not already exist. **Delete selected** followed by **Confirm deletion** removes the selected templates; **Cancel** leaves them in place. Deleting a template does not remove annotations already placed in PDFs. **Filter templates** searches names. **Reload library** rereads the local file.

Failed reads, imports and saves preserve the active library. If another instance has changed its file, reload before saving; a save already in progress must finish before another instance can write. PDF Undo/Redo applies to placed annotations, not library-file changes. Export a copy before deleting templates you want to keep.

Scale is a uniform size multiplier: `1` keeps the template size. It is independent of measurement calibration. The desktop normalizes the template's local bounds to a lower-left origin of `(0, 0)` before placement. Finish or cancel a pending placement before switching documents or tools. If its document changes, choose the template again.

## Capture a group from an existing page

1. Open the source PDF at the desired page, then open the library and choose **Capture from current page**.
2. Select 1â€“16 supported saved annotations. Native SuniPDF rectangles, ellipses, lines/arrows, polygons, polylines and clouds are eligible when their geometry, style and appearance can be reproduced. The [0.85 candidate](RELEASE_0.85.0.md) also accepts supported native standard stamps and image stamps with verified appearances. Unsupported rows stay visible but disabled; hover for the refusal reason. Nothing is selected automatically.
3. Enter a unique name and declare **Source or provenance**. Credit and licence remain optional declarations. Choose **Save selected annotations as a group**. This saves a local library entry without modifying the PDF.
4. Reuse it through **Place on page**, preview and apply. Relative displayed-page geometry, stroke/fill colors, width, dashes and opacity are retained. Each placed member gets a fresh identity, and the complete placement has one Undo step.

Capture requires copying permission and supports documents up to 128 MiB and source pages with at most 512 annotations. It refuses changed source fingerprints, duplicates, clipped/incompatible geometry and unsupported members as a complete operation. A document/tab change invalidates an outstanding capture. Cancel and start capture again to refresh the selection.

The 0.82 candidate accepts the native Approved, Draft, Confidential and Rejected stamps. Native image stamps require a verified RGB image, optional matching alpha mask and reproducible aspect-fit appearance. Capture produces a metadata-free embedded PNG. Its template opacity is 1 because the saved pixels already contain the original opacity; this prevents applying transparency twice. Mixed supported shapes, stamps and images may be captured as one group.

The [0.88 candidate](RELEASE_0.88.0.md) also captures native rich-text FreeText annotations using the twelve standard Helvetica, Times and Courier font variants. Text, font style, color, fill, border and opacity must reproduce the saved appearance exactly. The appearance and its font resources are validated against a regenerated annotation; no source font resources are imported.

Legacy simple-text annotations, foreign/custom appearances, embedded or altered fonts, review threads, measurements, page scales and extra annotation semantics remain excluded. Normalization, group offsets and placement must preserve text-box dimensions exactly at the requested scale; incompatible coordinates are refused to protect line wrapping. Supported text templates can still be created from new drafts as described above. The library never silently drops an unsupported selected member, and these additions do not change the revision-transfer workflow's supported annotation types.

Canonical appearance inspection retains its 2 MiB graph budget, including decoded resources. An image may satisfy the general PNG limits yet still exceed this stricter saved-appearance capture budget; the candidate is then refused explicitly.

## CLI example

Save this single entry as a UTF-8 JSON file named `draft-entry.json`:

```json
{
  "id": "review-draft",
  "name": "Review draft",
  "provenance": { "author": null, "source": null, "license": null },
  "primitives": [
    { "kind": "stamp", "rect": [0, 0, 160, 50], "stamp": "draft" }
  ]
}
```

```powershell
sunipdf-cli annotation-library add --entry draft-entry.json --out review-library.json
sunipdf-cli annotation-library inspect review-library.json
sunipdf-cli annotation-library place review-library.json --id review-draft --file plan.pdf --page 1 --origin "40,40" --scale 1 --out plan-reviewed.pdf
sunipdf-cli annotation-library export review-library.json --id review-draft --out selected-library.json
sunipdf-cli annotation-library remove review-library.json --id review-draft --out remaining-library.json
```

Pages start at 1. The origin uses displayed-page coordinates in PDF points, measured from the page's lower-left corner, including rotation/crop handling. CLI placement transforms each local coordinate as `origin + coordinate Ã— scale`; a manually authored entry can therefore retain local padding. Scale is explicit and uniform. Geometry and styles must still fit the page and supported annotation limits after scaling. Existing destinations are refused, including with `--overwrite`; choose a new output filename. Protected input uses the existing `--password-stdin` or `--password-env` options. Placement requires annotation permission and retains supported encryption.

To capture saved annotations, first inspect their availability and fingerprints. This example captures the first supported record; repeat `--annotation INDEX:FINGERPRINT` to capture a group:

```powershell
$inventory = sunipdf-cli annotation-library capture-candidates plan.pdf --page 1 | ConvertFrom-Json
$record = $inventory.annotations | Where-Object { $_.fingerprint } | Select-Object -First 1
if ($null -eq $record) { throw "No supported annotations on this page" }
$selection = "{0}:{1}" -f $record.index, $record.fingerprint
sunipdf-cli annotation-library capture plan.pdf --page 1 --annotation $selection --id captured-plan --name "Captured plan symbol" --source "Locally authored plan" --out captured-library.json
```

Capture indices start at 0, exactly as returned by `capture-candidates`. The fingerprint must still match the saved annotation. Capture writes a new version-1 library and leaves the original PDF unchanged; it does not merge into an existing destination. Import the resulting library in the desktop to merge it explicitly.

## Open file format and limits

The library envelope is `{"format":"sunipdf-annotation-library","version":1,"entries":[...]}`. Each entry has an ASCII ID, a unique name, declared provenance and a flat list of primitives. Primitive kinds are `shape`, `free_text`, `vertices`, `stamp` and `image`. They contain explicit geometry and styles. Images contain PNG bytes in base64, dimensions and SHA256; they do not retain an external file path.

Files are limited to 16 MiB, 64 entries and 16 primitives per entry. Individual PNG inputs are limited to 4 MiB, 4096 pixels per side and four million pixels. Each entry and the complete library are also limited to 12 MiB of encoded image data and sixteen million image pixels. Names and IDs must be unique; unsupported versions, fields, actions, malformed images and invalid geometry are refused. Local placement supports documents up to 128 MiB. A refused group does not leave partly inserted members behind.

Library data is local and self-contained. Provenance text is preserved as supplied; it is not proof of ownership or an independently verified licence. The library does not import scripts, attachments, measurement scales, PDF object identities, replies or executable actions. Members remain separate PDF annotations after placement; available editing controls depend on their type. They are not kept as a linked group. Nested groups and capture of arbitrary PDF annotations remain open work.
