# SuniPDF 0.89.0 ? candidate

This candidate adds an exact custom-field criterion to the annotation register and its reports. Publication and package qualification are pending.

Filter annotations by a named text field, for example `Room = A-101`, together with the existing review, author, page and zone criteria. The comparison is literal and case-sensitive. An empty stored value is distinct from an absent field. This operation changes the selection only and leaves the PDF unchanged.

The criterion can be saved with reusable filters and carried into selection-bound exports. Presets containing it use version2; older version1 presets remain supported and keep their existing representation when the criterion is absent.

This is a text-field filter. It does not add numeric range expressions, regular expressions or automatic selection for bulk edits. Changing filters clears an outstanding bulk-edit selection and preview.

[Annotation register](MARKUP_REGISTER.md) ? [Saved filters](REGISTER_FILTERS.md) ? [Project status](PROJECT_STATUS.md)
