# SuniPDF 0.76.0

Published stable on 2026-10-24 at 23:37:40 UTC:
[download packages](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.76.0).
Tagged source: `343103c38569345423b51d40406be19e17ee7914`.
See [qualification evidence](qa/RELEASE_0.76.0.json) and [Project status](PROJECT_STATUS.md).

## Two locations in one PDF

Choose **Reading options > Split document view**, or find **Split document view**
with **Ctrl+K** (**Command+K** on macOS). Two panes show the same open PDF. Each keeps its
own page, zoom, scroll position, reading layout, orientation and navigation history.
The panes appear side by side when space permits and stack in a narrow document area.

Click a pane or its heading to activate it. Page controls, search navigation and
editing tools follow that pane. **Switch active view** also works through the
command palette. The first click inside an inactive pane focuses it; click again
to use an editing tool. Finish or cancel unfinished input before switching views.

Enable **Synchronize views** to follow the active pane's page, zoom, scroll position
and view rotation. Turn synchronization off to navigate independently again.

Both panes share the same document and Undo/Redo history. **Close split view**
retains the active location and leaves the PDF open. The workspace survives tab
switches during the current session; it is not saved in the PDF or restored after
restarting SuniPDF. Use document tabs for other files: this increment covers two
views of one PDF.

[Full reader workflow](READER_WORKSPACE.md#two-independent-views).

## Rendering and editing safeguards

Both panes use a shared, bounded tile cache. Visible-page demand is combined before
rendering, and synchronized positions reuse cached page layouts. Document changes
invalidate affected rendering and clamp pane locations to valid pages.

Pending edits, unfinished form input and object-selection requests prevent a
conflicting pane switch. Opening another tab preserves that tab's reader workspace
without sharing its document selection. Splitting, synchronizing and closing the
split do not modify the PDF.

## Executed qualification

All 11 jobs in the [tagged release run](https://github.com/lmdanavanem/SuniPDF/actions/runs/40140749094)
passed. The published release is marked Latest. Its ten public files were downloaded
without authentication and checked against their recorded SHA256 values.

| Area | Recorded coverage |
|---|---|
| Windows reader | 13 offscreen scenes, including three split-view scenes; interaction, focus, draft and lifecycle regressions |
| Windows upgrades | 53 portable baselines plus four guards, with all 57 checks passing; 51 MSI baselines, including published 0.75 packages |
| Ubuntu 22.04 and 24.04 | 16 installed runtime cases per distribution, including independent pages, synchronized pages and closing the split while retaining the PDF |
| Apple Silicon | Seven installed checks on macOS 15, covering package integrity, binary requirements, PDF rendering, edit/save/reopen and desktop launch |
| Conformance | 12 pinned fixtures within the recorded validator profiles and operations; the downloaded DEB CLI matches the conformance-tested binary |

Ten images received visual review: three Windows split captures by exact RGBA
comparison with directly reviewed PR captures, three native split captures from
each Ubuntu distribution reviewed directly, and one Mac PDF-display capture reviewed
directly. The other ten Windows scenes retain automated checks. The evidence records
the capture source and review method; this is not physical-device qualification.

Native Ubuntu split-reader checks use the installed application through its command
palette under X11/Openbox/Xvfb, with software rendering and color-isolated PDF-text
checks. They cover the recorded two-page fixture, not arbitrary project plans or
native pointer editing. Mac package checks do not establish split-reader interaction,
Finder interaction or clipboard interoperability on a physical Mac.

## Platform and feature limits

Windows packages remain unsigned. The Apple Silicon application is ad hoc signed,
not notarized, and macOS update installation remains manual. Ubuntu supports in-app
installation; the packaged updater helper is tested within the
[documented limits](UBUNTU.md#settings-recovery-and-updates). Graphical authorization
and complete desktop updater interaction were not exercised.
See [macOS](MACOS.md), [Ubuntu](UBUNTU.md) and [known issues](KNOWN_ISSUES.md).

Physical-device, Wayland, ARM64 hardware and human accessibility qualification
retain their separate gates. This reader increment does not close the remaining
plan-link, drawing or interoperability criteria, and makes no general PDF/A/PDF/UA
conformance or overall performance claim.
