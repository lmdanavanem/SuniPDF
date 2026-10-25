# SuniPDF 0.79.0

Published stable on 2026-10-25. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.79.0) · [Qualification evidence](qa/RELEASE_0.79.0.json).

## Reviewed multi-file annotation transfers

Prepare and review a transfer between saved old/new PDFs, choose a new output,
and add it to a queue. Continue reviewing other file pairs before saving the
queue. Up to 16 pairs and 160 MiB of prepared copies are retained in memory.

Every mapped page requires an actual preview and explicit review. Source and
output checks run across the queue before publication; each output retains its
own no-overwrite commit. Completed copies remain reported if later work fails
or is cancelled. Closing guards queued work instead of silently discarding it.

The CLI adds batch preview/apply through a bounded versioned manifest. Individual
recipes still bind source hashes, page mappings and annotation fingerprints.
Optional per-file environment-variable names support protected inputs without
putting password values in manifests or process arguments.

## More direct plan-link creation

Page selection accepts bounded ranges and explicit indexed pages. Destination
filters and selection actions reduce repetitive input without approving link
occurrences. Invalid page/source/destination choices show an explanation.

Completed copies retain their save receipts. Opening a completed copy checks its
recorded byte count and SHA256 first; receipt export creates a new JSON file.

## Qualification and limits

The tagged source passed all 11 release jobs. Windows qualification includes
16 offscreen interface scenes (3 Home and 13 PDF/settings scenes), 56 portable upgrade baselines plus
4 guards, and 54 MSI upgrade baselines. Ubuntu 22.04 and 24.04
each passed 22 installed runtime cases, including 6 plan-link cases.
Apple Silicon passed 7 installed checks on macOS 15.7.9.

Independent PDF validation passed the recorded 12-fixture corpus. The public
DEB contains the same CLI bytes as that conformance run. All ten public downloads
were checked anonymously against their SHA256 values, verified 2026-10-25T05:43:29.006968+00:00.

Visual review covers 16 recorded images. Windows captures use offscreen software
rendering; Ubuntu interaction uses the recorded X11/Openbox/Xvfb environment.
Capture identities, review methods and exact-source limits are retained in
[qualification evidence](qa/RELEASE_0.79.0.json).

Independent packaged-CLI corpora:

- Reviewed transfer batches: 69 checks across 27 recorded commands.

Each output is committed separately; the queue is not a transaction across files. Transfers retain the existing supported native annotation types and do not replace incoming page content. Native queue creation/review/saving and complete plan-link creation retain separate qualification gates.

Native plan-link navigation uses synthetic PDFs prepared by the installed CLI.
It covers preview/cancel, changed or missing input refusals and the correct
physical destination page. Desktop link creation, desktop link saving,
dirty-destination interaction and broader external-reader interoperability
remain open. These results do not establish complete AEC-017 acceptance.

Native return-to-source qualification covers one return to the observed
source position. Restoring both split panes, repeated returns and dirty
document transitions retain separate automated or unqualified scopes.
Home review covers the visible headings and compact Open action; content
below the recorded viewport is not visually qualified. Native theme
checks verify neutral chrome and saved preferences, without establishing
a visual difference between reduced-effects modes on flat docked bars.

Windows packages remain unsigned, and Windows ARM64 is cross-built without
native hardware execution. The Apple Silicon app is ad hoc signed, not
notarized; macOS 12 is a binary-checked deployment minimum. Physical Macs,
interactive Finder opening and clipboard interoperability remain unqualified.
Ubuntu updater-helper checks do not establish graphical authorization or a
complete desktop updater interaction. Physical GPUs, Wayland, printers and
human accessibility remain separate gates.

Full backlog cards remain open until all stated acceptance criteria are evidenced.

[Transfer guide](REVISION_TRANSFER.md) · [Plan links](PLAN_LINKS.md) · [Roadmap](ROADMAP.md).
