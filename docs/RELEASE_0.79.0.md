# SuniPDF 0.79.0

Development candidate; publication and package qualification are pending.

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

Automated, native and downloaded-package results will be recorded after they
complete. No complete AEC-016/AEC-017, physical hardware or external-reader
interoperability claim follows from implementation alone.

Transfers retain their existing supported annotation types and alignment limits.
They create copies of incoming PDF revisions with supported annotations added;
they do not replace page content or transfer unsupported semantic records.

[Transfer guide](REVISION_TRANSFER.md) · [Plan links](PLAN_LINKS.md) · [Roadmap](ROADMAP.md).
