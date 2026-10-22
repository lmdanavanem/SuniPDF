# Annotation libraries

[Français](ANNOTATION_LIBRARY.fr.md) · [0.45 scope and qualification status](RELEASE_0.45.0.md)

## Desktop

Open **Annotation library** from the main toolbar, **All tools**, or **Add comments**. The library is stored locally, separately from application preferences.

1. Choose **Create template** to start a new annotation draft. Supported draft capture includes text in a standard font, polygons/polylines, revision clouds, predefined stamps and PNG image stamps.
2. In **Save to library**, enter a unique name and save the draft. Author, source and licence are optional declarations; missing information stays unknown. Saving a template does not apply it to the PDF.
3. In the library, choose **Place**, set the scale and click the page to position the template. Review **Preview**, then **Apply**. Cancel discards the placement; Apply creates one Undo step.
4. To make a group, select at least two entries, choose **Group**, set their offsets and save under a new name. A group is flat and contains at most 16 primitives. Placement gives every member a new PDF annotation identity.

Import merges a validated library into the active library. Duplicate IDs or names and invalid entries refuse the complete import; they do not partly replace valid data. Export writes all or selected entries to a new JSON file. Delete removes the selected library entries after the explicit delete action; it does not remove annotations already placed in PDFs. Reload reads the saved local file again.

Ordinary shapes can also be imported through the versioned file format. Capturing an arbitrary selected PDF annotation, its review thread or its imported appearance is outside this first version. Measurement calibrations are never silently reused through a library.

## CLI example

Save this single entry as `draft-entry.json`:

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
sunipdf-cli annotation-library place review-library.json --id review-draft --file plan.pdf --page 1 --origin 40,40 --scale 1 --out plan-reviewed.pdf
sunipdf-cli annotation-library export review-library.json --id review-draft --out selected-library.json
sunipdf-cli annotation-library remove review-library.json --id review-draft --out remaining-library.json
```

Pages start at 1. The origin is the displayed page's lower-left coordinate in PDF points, including rotation/crop handling. Scale is explicit and uniform. Geometry and styles must still fit the page and supported annotation limits after scaling. Existing destinations are refused, including with `--overwrite`; choose a new output filename. Protected input uses the existing `--password-stdin` or `--password-env` options. Placement requires annotation permission and retains supported encryption.

## Open file format and limits

The library envelope is `{"format":"sunipdf-annotation-library","version":1,"entries":[...]}`. Each entry has an ASCII ID, a unique name, declared provenance and a flat list of primitives. Primitive kinds are `shape`, `free_text`, `vertices`, `stamp` and `image`. They contain explicit geometry and styles. Images contain PNG bytes in base64, dimensions and SHA256; they do not retain an external file path.

Files are limited to 16 MiB, 64 entries and 16 primitives per entry. Individual PNG inputs are limited to 4 MiB and four million pixels; additional library-wide image budgets apply. Names and IDs must be unique; unsupported versions, fields, actions, malformed images and invalid geometry are refused. Local placement supports documents up to 128 MiB. A refused group does not leave partly inserted members behind.

Library data is local and self-contained. Provenance text is preserved as supplied; it is not proof of ownership or an independently verified licence. The library does not import scripts, attachments, measurement scales, PDF object identities, replies or executable actions. Members remain individually editable PDF annotations after placement; nested groups and general PDF annotation capture remain open work.
