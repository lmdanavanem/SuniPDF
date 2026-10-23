# Conformance and independent-reader checks

SuniPDF's conformance gate tests a bounded, licensed set of original and rewritten PDFs. It does not certify arbitrary edited files, provide a PDF/UA authoring system, or replace the human checks required for accessible documents.

## Corpus and attribution

The additional corpus contains 12 unmodified fixtures from the [veraPDF test corpus](https://github.com/veraPDF/veraPDF-corpus/tree/377728596647e40edf16003d68d620773ce96ec5), attributed to the veraPDF Consortium and licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). The [manifest](../scripts/conformance-corpus.json) pins the upstream revision, original paths, byte counts, SHA-256 digests, profiles and intended negative rules. Downloads include an attribution file. Derived PDFs are kept separately in the evidence directory and identified by operation.

| Profile | Positive and negative pairs |
| --- | --- |
| PDF/A-1b | Composite fonts; form XObjects |
| PDF/A-2b | Subset font embedding; interactive forms |
| PDF/UA-1 | Form structure; language tags |

These are atomic upstream test files, not a representative collection of production documents. An upstream `pass` filename describes the intended rule test; the gate independently requires the whole selected machine-validation profile to pass. An unexpected baseline outcome is a failed gate and must be investigated. The existing public PDF and large-plan regression suites remain separate and continue to run.

## Checks and evidence

The gate uses the SHA-256-pinned veraPDF 1.30.3 Greenfield installer from the [official release site](https://software.verapdf.org/releases/1.30/verapdf-greenfield-1.30.3-installer.zip). It explicitly selects `1b`, `2b` or `ua1`; it never relies on metadata to select a default profile. See the official [installation](https://docs.verapdf.org/install/) and [validation documentation](https://docs.verapdf.org/cli/validation/).

Each positive source is checked before and after these operations:

1. Lossless optimization. The evidence records whether bytes changed, because optimization can legitimately return the original file.
2. A real title-metadata edit.
3. A 90-degree page rotation, then a 270-degree rotation restoring the original orientation.
4. Metadata removal, which must be rejected as nonconforming by the selected profile.

Negative fixtures must fail their intended rule. A missing validator, truncated report, wrong file or profile, parse error, timeout, inconsistent summary, or unexpected conformance result fails the gate. The validator's exit code alone is never treated as proof. Reports must contain exactly one complete job for the tested file, valid rule counts, and no processing failures. Original fixture hashes are checked before and after processing.

Every tested PDF also undergoes `qpdf --check`. Positive sources and all rewritten files require a clean result. Two negative upstream fixtures have known structural warnings, recorded verbatim in the manifest; only those exact warnings are accepted for those unchanged negative sources, and the report identifies them as warnings rather than clean syntax checks. Unexpected warnings fail.

The first page is rendered independently by PDFium through SuniPDF and by Poppler. Fractional page dimensions can produce raster canvases differing by one pixel because the readers round differently. Only that one-pixel canvas difference is permitted: the smaller canvas is padded with white on its trailing edge without rescaling the content, and both original sizes are recorded. Larger geometry differences fail. Appearance comparison allows two pixels of antialiasing displacement, requires at least 70% mutual dark-pixel coverage, and limits the dark-pixel ratio to 0.5–2.0 and RGB mean absolute error to 12. Dark pixels normally use luminance below 160; one invalid form fixture contains only a light-gray widget border and explicitly uses 240, as documented in the manifest. Positive fixtures keep the default threshold.

Rewritten pages are also compared with their original Poppler rendering using exact raster geometry; the intermediate 90-degree rotation is compared across readers in its own orientation. This is first-page appearance evidence, not a proof that every page, annotation interaction, or assistive-technology behavior is preserved.

`report.json` records all cases and failures, source/output hashes, byte changes, tool versions, executable hashes and appearance measurements. Raw validation XML, tool output, rewritten PDFs and rendered images remain beside it for diagnosis. Failure in one case does not hide the remaining cases. Downloads, file counts, file sizes, raster dimensions, tool output and subprocess runtimes are bounded.

## Running the gate

Python-only checks can run without a native application build:

```sh
python scripts/fetch-conformance.py --self-test
python scripts/test-conformance.py --self-test
```

The Linux CI runner provides Java 17 or later, qpdf, Poppler's `pdftoppm`, Pillow, and a built SuniPDF CLI with its PDFium runtime. The validator installer deliberately refuses execution outside a Linux GitHub Actions runner.

```sh
python scripts/fetch-conformance.py --out target/conformance-corpus --install-verapdf target/qa-tools/verapdf
python scripts/test-conformance.py --cli target/release/sunipdf-cli --corpus target/conformance-corpus --verapdf target/qa-tools/verapdf/verapdf --qpdf qpdf --pdftoppm pdftoppm --out target/conformance-evidence
```

The output and validator installation directories must be new. Keep the evidence artifact even when the command fails. A successful result applies only to that source revision, toolchain, corpus and operation matrix; broader qualification requires additional evidence.

## Remaining qualification

Human PDF/UA review remains open: meaningful reading order, appropriate alternative text, heading semantics, usable table associations, language changes, keyboard navigation and assistive-technology behavior require inspection of real documents. PDF/A-1a, PDF/A-2a/2u, PDF/A-3, PDF/A-4, PDF/UA-2 and conversion of arbitrary PDFs into those formats are outside this gate. User-supplied production plans remain deferred and must not be published as test fixtures.

See the [roadmap](ROADMAP.md) and [release criteria](RELEASE_1.0_CRITERIA.md) for the broader qualification work.
