# SuniPDF 0.45.0 — reusable annotation libraries

[Français](RELEASE_0.45.0.fr.md) · [Library guide](ANNOTATION_LIBRARY.md)

Status: published as stable Latest [0.45.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.45.0), source `51f07a6b27733e4bbfd46bcbc45bc2a82d4886c8`, on 2026-10-22. [Previous release](RELEASE_0.44.0.md).

## Scope

A visible local annotation library lets users save supported drafts, name entries, reuse them, import/export a documented JSON file and remove entries explicitly. Supported templates include shapes, text using standard fonts, polylines/polygons, revision clouds, predefined stamps and embedded PNG images. A flat group contains at most 16 primitives.

Placement uses an explicit origin and scale on the current page, followed by Preview and Apply. Applying a group is one Undo step; its members remain individual PDF annotations with new identities. The library entry remains available; deleting it later does not remove annotations already placed.

The versioned file contains its image data and declared provenance. Import validates types and resource bounds before committing a merged library; conflicts or failures preserve its previous state. Scripts, external paths, PDF identities, review threads and measurement calibrations are excluded. Provenance and licence text are user declarations, not independently verified grants.

CLI management operates on explicit library files. Placing an entry in a PDF requires annotation permission and a new output copy; original files and existing destinations remain protected. Supported encryption is retained.

## Qualification

Exact source passed: [ci](https://github.com/lmdanavanem/SuniPDF/actions/runs/39732620304) · [release](https://github.com/lmdanavanem/SuniPDF/actions/runs/39732631044) · [codeql](https://github.com/lmdanavanem/SuniPDF/actions/runs/39732620287). 724 tests passed, 10 default ignored. 1302 EN/FR keys. 37 portable scenarios. 31 MSI baselines. 155 MSI logs. No open Code Scanning alerts. Six public downloads and dist copies verified by SHA256. 24 packaged CLI calls verify strict library imports, named templates and groups, selected export, removal, placement, fresh identities, source preservation and explicit refusals. [Evidence](qa/RELEASE_0.45.0.json).

## Remaining limits

This first library does not capture arbitrary existing PDF annotations, preserve arbitrary imported appearances or nest groups. It does not transfer measurements, page scales, attached files or executable actions. AEC-006 remains partial until the broader acceptance criteria are demonstrated. Native/hardware and 1.0 qualification gates remain separate.
