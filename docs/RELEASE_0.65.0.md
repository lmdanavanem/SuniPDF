# SuniPDF 0.65.0

Status: **development candidate; not published**. This increment adds reviewed links between
indexed local plan sheets and avoids unnecessary Ubuntu rebuilds for verified documentation-only
changes. Local automated and native qualification passed within the scope below;
exact-source CI and package checks remain release gates.

## Reviewed links between plan sheets

Open **File → Plan links**, or find the command under **Organize pages** or in the command palette.
Choose a local plan-set manifest, one source PDF, explicit physical pages and the permitted target
sheets. Confirm that the operation uses saved files. The scan finds exact sheet identifiers in
existing searchable text and reports ambiguous, missing, unsupported and already-linked references.

Approve unique matches explicitly or choose the intended destination for an ambiguous occurrence.
Preparation creates a separate candidate. Source-page previews show the actual linked PDF, and
destination previews render the verified target sheet. Save a new linked copy beside the source
PDF to retain relative destinations. Existing content, annotations and links remain preserved.

Clicking an authored link opens a confirmation workflow. Select the relevant manifest and open
the verified destination in SuniPDF. The target's physical page, relative path, size and fingerprint
must agree with the index. Missing, redirected or changed targets are refused. An existing dirty
tab retains its edits through the existing plan-set navigation choices.

The CLI provides `plan-links inspect`, `preview` and `apply`. Reviews retain source and manifest
hashes, occurrence fingerprints and explicit confirmations. PNG previews can render either the
source candidate or a selected destination. Both the CLI and desktop use the same bounded engine.
See [workflow, credentials and limits](PLAN_LINKS.md).

The first increment processes one indexed source PDF, at most 32 explicit pages and 512 detected
references. It reads existing text, including a previously created OCR layer; it does not perform
OCR automatically. Standard GoToR links carry bounded target metadata. Compatibility with arbitrary
external readers and bulk processing of multiple source PDFs remain separate work.

## Build and test reliability

Ubuntu CI now classifies the exact Git event before building. Only the existing strict documentation
allowlist can omit application builds; malformed or unsupported events take the full build path.
Code changes, tags, manual runs and reusable release workflows retain the full build, tests,
conformance and Ubuntu installation gates. Documentation changes retain link, guide, backlog and
evidence validation. Fifteen classification regressions pass locally.

The adversarial test campaign now creates its report directory when an external Cargo cache is
used. This fixes a report-writing failure in fresh worktrees without changing the mutation checks.

## Local qualification

The final local pipeline passed formatting, workspace Clippy, the complete workspace suite and
desktop/CLI builds. **1,013 test executions passed, none failed and 15 were ignored across 90 suite
results.** English and French catalogues have 1,715 matching keys. The final source fingerprint is
`ddfe8c3ec11d342d68482600c8c24c211f9f4ed52ede6ac09d866f4ec9576473`.

The actual final development CLI passed **30 commands and 329 independent checks**. Coverage includes
all four rotations, nonzero crops, exact identifier boundaries, Unicode paths, ambiguous destinations,
existing links, absent text, stale sources/targets/manifests and output collisions. qpdf and pypdf
inspect the saved outputs. Existing annotation fields are checked exactly, with a 0.001-point absolute
tolerance only for stored rectangle rounding; a changed action or larger movement is rejected.
All protected sources and the tested binary remained unchanged.

These results identify development binaries: desktop SHA256
`36b6f02e2d24f16e995b6140013ed50d646a70a914d50491100fe11e7e51b281`, CLI SHA256
`e271c46162f03dc77aa883d2937bcaf6c922726df3f707631488f061d207016f`.
Native Windows checks passed in isolated English/light and French/dark profiles on those final
binaries. Three explicitly reviewed references were saved and reopened; clicks navigated to the
correct physical pages, including rotated sources and destinations. An ambiguous reference was
manually assigned to revision B and reopened at that destination. A changed target was refused
both before saving and during navigation; a wrong manifest was also refused. Sources without text
showed the OCR prerequisite. Existing links were retained, and cancelling discarded the candidate.

Each of the two native saved copies passed 95 independent pypdf/qpdf checks. A final overlapping
run passed 96 checks, including absence of the refused output; these runs are not added together.
All 16 immutable inputs and the identified executables/runtime remained unchanged, and both QA
instances closed normally. Protected PDFs, dirty-tab cases, cancellation races and source/manifest
changes after preview were not newly exercised manually; their automated checks remain separate.
Some engine errors still use English in the French interface.

Published-package, physical-platform, broader reader and accessibility evidence is not inferred
from these checks. Failed attempts and separate runs are not added to the final passed count.
