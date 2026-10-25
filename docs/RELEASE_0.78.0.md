# SuniPDF 0.78.0

Development candidate. Published packages remain on the version identified in
[Project status](PROJECT_STATUS.md) until qualification and publication finish.

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

## Qualification status

Implementation, source tests and installed application interaction are in progress.
Native evidence must distinguish the prepared synthetic link from actual clicks in
the installed reader. Dirty documents, failed transitions, stable tab identities
and view restoration require separate regression coverage. No complete AEC-017,
external-reader interoperability or physical-device qualification is claimed.
