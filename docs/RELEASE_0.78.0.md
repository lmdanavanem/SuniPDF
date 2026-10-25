# SuniPDF 0.78.0

Published stable on 2026-10-25. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.78.0) · [Qualification evidence](qa/RELEASE_0.78.0.json).

## A quieter Glass workspace

The Glass style uses a continuous neutral workspace in Light and Dark modes.
Docked bars meet cleanly, controls use smaller corners, and subtle depth is
reserved for floating surfaces. The welcome screen gives priority to opening a
file, with compact tool families instead of a large repeated card grid.

The Theme settings still provide Classic, Light, Dark, System and reduced effects.
The material is drawn within the application and does not blur the desktop behind
the window. Visual qualification includes the actual home screen as well as PDF
reading and settings.

## Return to the source plan

This increment adds an explicit return after following a verified link to another
plan. The source stays open in memory, including its unsaved edits and Undo/Redo
history. Returning restores the source reading position and view settings.

The return belongs to the latest successful linked-document navigation in the
current session. Closing or changing the source invalidates it. Cancelled and
failed navigation must leave the previous return intact. Existing page history
remains local to each document.

## Qualification

The tagged source passed all 11 release jobs. Windows qualification includes
16 offscreen interface scenes (3 Home and 13 PDF/settings scenes), 55 portable upgrade baselines plus
4 guards, and 53 MSI upgrade baselines. Ubuntu 22.04 and 24.04
each passed 22 installed runtime cases, including 6 plan-link cases.
Apple Silicon passed 7 installed checks on macOS 15.7.9.

Independent PDF validation passed the recorded 12-fixture corpus. The public
DEB contains the same CLI bytes as that conformance run. All ten public downloads
were checked anonymously against their SHA256 values, verified 2026-10-25T02:42:58.071611+00:00.

Visual review covers 16 recorded images. Windows captures use offscreen software
rendering; Ubuntu interaction uses the recorded X11/Openbox/Xvfb environment.
Capture identities, review methods and exact-source limits are retained in
[qualification evidence](qa/RELEASE_0.78.0.json).

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
