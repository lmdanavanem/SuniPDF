# SuniPDF 0.82.0

Development candidate; publication and package qualification are pending.

## Capture saved stamps and images into the annotation library

Select supported native stamps and image stamps already saved in a PDF, together
with supported shapes, and capture them as one reusable library entry. The
existing placement workflow provides a rendered preview, fresh annotation
identities and one Undo step for the complete group.

Capture checks the native appearance and resource graph before accepting a
member. Standard stamps retain their stamp kind. Verified image pixels become a
self-contained PNG, preserving transparency and the opacity already applied to
the saved appearance. Unsupported custom appearances remain explicit refusals.

The existing version-1 library format and CLI capture commands remain compatible.
Provenance is supplied explicitly; no private source path or ownership claim is
inferred. Capturing a library entry leaves the PDF unchanged. Changed fingerprints,
missing copying permission and unsupported selected members refuse the operation.

## Qualification and limits

Source, platform, round-trip and package results will be recorded after testing.
The existing bounded capture and image budgets remain in force; larger or
unrecognized appearance graphs are refused.

Saved FreeText capture remains unsupported because arbitrary font/resource graphs
cannot yet be reproduced safely. The revision-transfer workflow keeps its
existing supported annotation types. This increment does not establish native
workflow or universal external-reader qualification. AEC-006 remains In progress.

[Annotation library](ANNOTATION_LIBRARY.md) · [Roadmap](ROADMAP.md).
