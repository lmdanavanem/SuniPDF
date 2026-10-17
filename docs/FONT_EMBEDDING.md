# Embedded font formats and subsets

[Version française](FONT_EMBEDDING.fr.md)

Added text uses a selected installed face, not a filename extension as proof of
compatibility. Static TrueType outlines in TTF/OTF and explicitly selected TTC/OTC
faces use PDFium's TrueType CID loader. Static CFF1 faces use its Type1 CID loader.
Variable fonts, CFF2, color-font tables, missing outlines, damaged faces and web-font
containers are refused. Format-specific runtime and independent-reader QA must pass
before a format is described as validated in a release.

The implementation uses Allsorts 0.17.0 with its Rust flate2 backend. Its subsetter
generates a Unicode cmap and includes dependent composite glyphs; SuniPDF does not
implement a glyph-rewriting algorithm. The Minimal profile keeps OS/2 and required
SFNT tables. Cmap mappings, font size and glyph availability are checked before
PDFium changes the page. The API is `TextStyle.font_file` with zero-based
`font_face_index`; collection faces have distinct entries in the font picker.

Embedding rights are checked on the selected face's OS/2.fsType before any subset:
restricted, preview/print-only and bitmap-only outline embedding is refused.
The no-subsetting bit keeps the entire selected face. For a collection, the library
extracts that complete face with all its glyphs. Reserved rights bits are refused;
the existing least-restrictive interpretation for older mixed usage flags is kept.
These flags do not replace the font owner's license terms.

A subset contains requested glyphs, composite dependencies and a small editing
repertoire from U+0020–U+024F and U+0370–U+052F when present in the source. This keeps
normal Latin, Greek and Cyrillic edits useful without embedding unrelated large
glyph ranges. An existing subset does not acquire new characters automatically:
unsupported edits raise MissingGlyphs and preserve the document. Add Text can select
the original system face for a new repertoire. A standalone face is kept intact if
its subset would be larger.

Input is bounded to 32 MiB, 64 collection faces, 65,536 text characters and 4,096 selected
subset glyphs. A font file is read with a byte limit as well as a metadata check.
Prepared bytes are compared exactly before reusing a cached PDF font. After Undo,
Redo or reopening, existing page fonts are examined and an identical program is
reused. Different face selections or different glyph coverage may need separate
programs; stale native handles are never retained across restore.

Regression fixtures record their exact source hashes, full license and original
copyright notices in `crates/sunipdf-core/tests/fixtures/fonts`. Tests cover Unicode,
size reduction, complete no-subset embedding, selected TTC faces, refusal and
save/reopen/resource reuse. Engine tests write representative PDFs to
`target/font-embedding-evidence` for QPDF and independent-reader rendering/extraction.
Writing those files or passing PDFium tests alone does not constitute other-reader
validation.

References: [Microsoft embedding rights](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fstype),
[OpenType containers](https://learn.microsoft.com/en-us/typography/opentype/spec/otff),
[Allsorts source](https://github.com/yeslogic/allsorts),
[PDFium font API](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/fpdf_edit.h).
