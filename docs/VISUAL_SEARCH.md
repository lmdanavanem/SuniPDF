# Visual symbol search

This workflow requires version 0.71 or later. See [project status](PROJECT_STATUS.md)
for the currently published release and the scope of its qualification.

## Find and review symbols

1. Open a PDF and go to the page containing the symbol to use as a template.
2. Choose **Tools → Review & measure → Find symbols**.
3. Use **Fit page** or **Template zoom** to locate the symbol. Enable **Pan image**
   to move around an enlarged page, then disable it before dragging a rectangle
   around one symbol, including a small surrounding margin.
4. Enter the physical pages to search, such as `1,3-5`. Set the minimum similarity,
   rendering resolution and whether to include quarter-turn rotations.
5. Start the search. Review the proposed matches and their previews. Results start
   unselected; a similar appearance is not proof that two objects have the same meaning.
6. Select the matches to count, enter their group and confirm the reviewed selection.
   Apply to add count markers. One Undo removes the whole batch; save the PDF to keep it.

The search uses page content only, excluding annotation and form-widget appearances.
Existing count markers therefore do not become new template matches. Candidate previews
include existing annotations so previously counted objects remain visible during review.
Existing counts are retained; repeating a search does not automatically deduplicate
objects already counted. The original template occurrence can appear in the results
when its page is searched.

Changing the template or settings invalidates the previous review. Editing or switching
documents also invalidates results. A search can be cancelled; cancelled or limited
results retain their incomplete status even if some matches are accepted.
Closing the tool, including with Escape, discards its search settings and results.
Reopening it starts a new search on the current page. Count markers already applied
remain in the document and can be undone through the document's Undo command.

## What similarity means

The matcher compares the overall grayscale image and local pattern agreement at the
same physical scale, with exact rotations of 0, 90, 180 and 270 degrees when requested.
A higher threshold accepts less raster difference. Local missing or changed details
can lower the score even when the surrounding outline matches. A score is an image
similarity measure, not a probability or a validated
recognition rate. Automatic size changes, arbitrary-angle matching and matching across
different files are outside this workflow.

Blank or almost uniform templates are refused. Close, overlapping matches are reduced
to one candidate. Dense or overlapping symbols, background changes and noisy scans can
still produce missed or false matches. Inspect each accepted result. Accepted count
markers are separate from the number of candidates and from search completeness.

## Limits

- Select 1–16 distinct pages from the current document.
- The desktop offers 36, 72 and 144 dpi (0.5, 1 and 2 raster pixels per displayed PDF
  point). Each page is limited to 4,096 pixels per side and 8 Mi pixels overall;
  reduce the fixed resolution for larger sheets. The CLI accepts 0.25–2 pixels per point.
- A template must occupy 8–128 pixels on each side at the selected resolution.
- The desktop stops at 256 candidates or 100 million pixel comparisons. The CLI
  exposes limits up to 512 candidates and 500 million comparisons. A limit reached
  before the search finishes is reported explicitly.
- Source snapshots are limited to 128 MiB. Creating count markers requires copying and
  annotation permissions and is refused for digitally signed documents.

Raster matching advances in bounded chunks between foreground work. A single PDF page
render remains non-preemptive. Finishing this bounded search does not establish that
every real-world object on a drawing has been found.

## Command line

Inspect a known template and create a new review file with no candidates accepted:

```powershell
sunipdf-cli visual-search inspect plan.pdf --template-page 1 --template-rect 40,80,72,112 --pages 1,3-5 --rotations 0,90,180,270 --similarity 90 --review-out review.json
```

The output contains candidate IDs, rectangles, rotation, similarity and coverage. Core
inspection pages and `options.pages` use zero-based indices; `template_page` in the review
file and command-line page inputs use physical one-based page numbers.

Edit `selected_ids` in the review file to contain only reviewed IDs, then set `group`,
`marker_size` and optional `author`. Do not alter the recorded source or inspection
fingerprints. An incomplete search also requires an explicit `allow_partial: true`.

```powershell
sunipdf-cli visual-search apply plan.pdf --review review.json --out counted-plan.pdf
```

Apply repeats the search on the exact saved source and validates the results before
creating the selected markers. Unknown or duplicate IDs, changed sources and changed
inspection results are refused. Outputs must be new files, including when `--overwrite`
is supplied. An incomplete search returns exit code 2 with a JSON report; check `published`
to distinguish an inspected partial result from a deliberately accepted partial output.

The PDF preview, runtime corpus and native interaction remain separate qualification
scopes. See the [annotation register](MARKUP_REGISTER.md) for reviewing saved counts.
