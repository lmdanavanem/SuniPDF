# SuniPDF 0.88.0 ? candidate

This candidate adds reusable capture of supported saved rich-text annotations. Publication and package qualification are pending.

Capture native text using the twelve standard Helvetica, Times and Courier font variants, either alone or with supported shapes and stamps. Text, font, color, border, fill, opacity and displayed-page geometry are preserved through the existing version-1 annotation library. Each placement creates fresh annotation identities and one Undo step.

The saved appearance and its resources must match a regenerated appearance before capture is offered. Foreign or embedded font resources, altered appearances, legacy simple-text annotations and unsupported annotation semantics remain refused. Geometry that would change the text box dimensions during normalization, grouping or placement is refused rather than changing line wrapping.

The independent CLI corpus covers rotated and cropped pages, standard-font text, mixed groups, exact rendered pixels, saved appearance resources and unchanged original files. Runtime results have not yet been recorded for this candidate.

[Annotation-library guide](ANNOTATION_LIBRARY.md) ? [Project status](PROJECT_STATUS.md)
