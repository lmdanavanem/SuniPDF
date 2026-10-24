# Form value exchange

The Forms tool and `sunipdf-cli form-export` export values as JSON, XFDF or FDF according to the selected extension. Imports use the current form's field definitions and validation. The source PDF is not part of the exchange file.

```text
sunipdf-cli form-export source.pdf --out values.fdf
sunipdf-cli fill-form blank.pdf --values values.fdf --out filled.pdf
```

FDF export includes supported text, checkbox, radio and choice values. It excludes password fields, buttons, signatures, annotations, attachments, actions, scripts and source-file references. Read-only values are included; reimporting the same value does not rewrite a read-only field.

Qualified names become a tree of partial names. Text uses PDF text strings, including Unicode. Multi-selection lists retain their selected array. Checkbox and radio values use their actual appearance-state names. During import, those names remain distinct from text export values until they are matched to the destination field; this matters for radio groups whose `/Opt` export values differ from `/AP` names.

An explicitly selected empty choice remains distinct from clearing a field. FDF imports preserve an empty text string as a selected empty option when that option exists; a null value clears the selection. JSON and XFDF retain their existing empty-string behavior.

Export refuses duplicate names, a terminal name that is also a parent, empty or invalid name components, qualified names longer than 4,096 bytes, more than 12 name levels, more than 10,000 fields, more than 4,096 values in a choice array, and a selected button without one unambiguous appearance state. Button appearance-state names must use ASCII; text and choice export values retain Unicode. It does not infer a selected widget from an ambiguous radio export value.

FDF exchange limits both the serialized file and decoded value/name accounting to 16 MiB. Repeated indirect strings, array storage, null values, map entries and long name prefixes consume the same shared expansion budget before allocation. Existing bounded import accepts ordinary cross-reference tables and value-only text/name/array data; encrypted files, streams, actions and unsupported properties are refused. Strict imports reject unknown fields and invalid values before writing the new PDF.

## Form appearances

Forms that request a global appearance refresh (`NeedAppearances`) are displayed through temporary copies.
This preserves their original button states while viewing, including restricted and signed inputs. Filling
values, creating a field or changing an annotation style refreshes appearances explicitly; a failed refresh
keeps the previous working document. Existing ambiguous button definitions remain refused.

The isolated render copy is limited to 128 MiB before and after preparation, 16,384 widgets and 32 field-parent
levels. Inspection limits decompression to 128 MiB per stream. The copy-size limit applies to forms requesting
refresh, not to ordinary forms. These uncommon forms require copying and preparation for each rendered region;
large documents can therefore render more slowly. This is a preservation safeguard, not a performance claim.

The file structure follows [ISO 32000-1, sections 12.7.4 and 12.7.7.3](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf). This is value exchange, not full annotation or document exchange.
