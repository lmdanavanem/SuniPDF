# SuniPDF 0.45.0 — reusable annotation libraries

[Français](RELEASE_0.45.0.fr.md) · [Library guide](ANNOTATION_LIBRARY.md)

Status: **in development**. This scope is being implemented and has not been qualified or published.

## Scope

A visible local annotation library lets users save supported drafts, name entries, reuse them, import/export a documented JSON file and remove entries explicitly. Supported templates include shapes, text using standard fonts, polylines/polygons, revision clouds, predefined stamps and embedded PNG images. A flat group contains at most 16 primitives.

Placement uses an explicit origin and scale on the current page, followed by Preview and Apply. Applying a group is one Undo step; its members remain individual PDF annotations with new identities. The library entry remains available; deleting it later does not remove annotations already placed.

The versioned file contains its image data and declared provenance. Import validates types and resource bounds before committing a merged library; conflicts or failures preserve its previous state. Scripts, external paths, PDF identities, review threads and measurement calibrations are excluded. Provenance and licence text are user declarations, not independently verified grants.

CLI management operates on explicit library files. Placing an entry in a PDF requires annotation permission and a new output copy; original files and existing destinations remain protected. Supported encryption is retained.

## Qualification

Core, UI, CLI, independent PDF readers, packaging and public-download results will be recorded after execution against the release source. No passing counts or published assets are asserted here.

## Remaining limits

This first library does not capture arbitrary existing PDF annotations, preserve arbitrary imported appearances or nest groups. It does not transfer measurements, page scales, attached files or executable actions. AEC-006 remains partial until the broader acceptance criteria are demonstrated. Native/hardware and 1.0 qualification gates remain separate.
