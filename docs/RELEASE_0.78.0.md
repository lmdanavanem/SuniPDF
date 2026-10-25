# SuniPDF 0.78.0

Development candidate. Published packages remain on the version identified in
[Project status](PROJECT_STATUS.md) until qualification and publication finish.

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
