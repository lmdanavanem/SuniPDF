# Project status

**0.64.0 is published as stable Latest, with source, package and public-download verification complete.**
Updated 2026-10-24. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.64.0) ·
[Evidence](qa/RELEASE_0.64.0.json) · [Release scope](RELEASE_0.64.0.md).

## Qualification

Release `422528678` was published at `2026-10-24T05:42:14Z` from
`71854ef09886c94519775c345de1b45078c9d706` through [PR #41](https://github.com/lmdanavanem/SuniPDF/pull/41).
Its tree matches the qualified local source. The local suite recorded 994 passed, 0 failed and 15 ignored
results across 88 suites; formatting, Clippy and desktop/CLI builds passed. The local transfer CLI recorded
14 commands / 94 independent assertions.

The exact release source passed all four workflows. Windows recorded 994 passed tests
and 15 ignored results; Linux recorded 974 passed and 10 ignored per run,
with no failures. All 1,670 English/French keys matched. Portable upgrades covered
44 baselines and 4 guards (48 scenarios); MSI lifecycle checks covered
42 baselines and 210 logs. Ubuntu 22.04/24.04 installation and the pinned conformance corpus passed.

Eight public assets and checksum sidecars were downloaded anonymously and verified against GitHub digests.
The downloaded Windows CLI passed 82 process commands and 1,062 assertions with its bundled runtime.
Its three corpora cover provenance/quantities, plans/forms/volumes and revision transfer. Packaged files,
test helpers and protected inputs remained unchanged. The DEB's embedded CLI matched both conformance runs;
that additional payload inspection executed no program. See [machine-readable evidence](qa/RELEASE_0.64.0.json).

The final development build also passed native English/light and French/dark workflows: natural sorting,
filtering, grouping, physical-page navigation, explicit matches, both destination previews, four transferred
annotations over two pages, Save new copy and reopening. Ambiguous/unmatched sheets remained explicit;
closing a candidate wrote no output, and changing a scratch destination after preview refused save.
Independent inspection of the saved native output passed 27 assertions with qpdf and pypdf.

These native checks used a development build with an explicitly supplied PDFium runtime. They do not
establish distributed-GUI execution or local installer qualification. Dirty-tab guards, cancellation races
and two-anchor alignment retain automated/CLI evidence; they were not repeated in the native run.
Detailed engine refusals can remain English inside the French interface.

Code scanning has zero open alerts after individual review of two test-only credentials and one false-positive
PDF identity digest. The original failed aggregate checks are retained in history; scanners remain enabled.

## Delivered scope and tracking

- [Plan sets](PLAN_SETS.md): separate indexed PDFs, stable sheet identities, metadata, confined references,
  fingerprints, explicit refresh/relink, persistent manifest copies, filtering, grouping, natural sorting and
  guarded physical-page navigation. AEC-015 is **Done for these stated criteria**; replacement and links are separate cards.
- [Reviewed annotation transfer](REVISION_TRANSFER.md): explicit revision/page matching, supported native shapes,
  bounded alignment, destination previews and atomic fresh-copy publication. AEC-016 remains **In progress**
  for its broader batch scope; excluded semantic records are not silently transferred.
- [Volumes](MEASUREMENT_VOLUMES.md) and [FDF](FORM_DATA.md) retain their published 0.63 capabilities and limits.
  AEC-009 remains Done for polygon deductions and constant physical depth.

AEC-017 reviewed cross-document links, AEC-014 reviewed revision outlines and AEC-010 named zones
are development work planned for a combined 0.67 release after qualification. They remain In progress
and are not included in the 0.64 downloads. The
[GitHub project](https://github.com/users/lmdanavanem/projects/1), [backlog](AEC_BACKLOG.md) and
[roadmap](ROADMAP.md) retain broader acceptance criteria; no whole P1/P2 category is closed.

## Earlier evidence

[0.63](RELEASE_0.63.0.md) retains its 963-test local qualification, 68-command / 968-assertion downloaded CLI
run and separate native plan/FDF/volume evidence. [0.62](RELEASE_0.62.0.md), [0.61](RELEASE_0.61.0.md),
[0.60](RELEASE_0.60.0.md) and [earlier status history](archive/PROJECT_STATUS_PRE_0.60.md) remain available.
An earlier bounded benchmark found a 23 ms optimization/output regression; no general UI performance gain is claimed.

## Remaining gates

Windows packages remain unsigned. Physical ARM64, printers, production code signing, independent
update authenticity, human accessibility and private-project plans remain unqualified. Ubuntu checks use
containers and software rendering. Specified synthetic/licensed corpus results do not establish universal
PDF/A, PDF/UA, external-reader compatibility or survey accuracy. Ignored tests are not passed checks;
separate runs and earlier failures are not added to the final counts.

See [known issues](KNOWN_ISSUES.md) and [1.0 acceptance criteria](RELEASE_1.0_CRITERIA.md).
