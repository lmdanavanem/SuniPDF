# SuniPDF 0.74.0

Candidate under qualification. The current published version and completed checks
remain in [project status](PROJECT_STATUS.md).

## Settings and themes

Open **Menu → Settings → Theme** or search for **Theme settings** with **Ctrl+K**.
Choose the familiar **Classic** style or **Liquid Glass**, with independent
**System**, **Light** and **Dark** color modes. Preview cards show each style;
changes apply immediately and are saved for the next launch.

Liquid Glass uses frosted surfaces, soft highlights and rounded controls throughout
the application chrome. **Reduce transparency** provides opaque surfaces and
removes the reflected highlights. These are lightweight in-application materials,
not operating-system backdrop blur. PDF content and annotation colors are unchanged.

Existing light/dark preferences migrate to Classic without changing the user's
appearance. A failed preference write keeps the prior theme and shows an error.
The panel supports keyboard navigation, Escape and scrolling in short windows.

[Appearance guide](APPEARANCE.md) · [Roadmap](ROADMAP.md).

## Direct Ubuntu updates

The application downloads and verifies newer stable DEB packages, requests normal Linux
administrator authorization and keeps documents open during installation. It reports cancellation
or failure and offers retry; a successful installation offers a restart with existing save guards.

An installed helper independently checks the official release metadata and package identity,
stages verified bytes as root and uses APT without removing other packages or allowing downgrades.
GitHub remains the distribution trust boundary. Users on 0.73 or earlier need one manual installation
of 0.74 to obtain this flow. [Ubuntu installation and updates](UBUNTU.md).

## Qualification scope

Source, rendered-interface, installed Ubuntu interaction and published-package
checks are being recorded separately. This document does not yet qualify a
published 0.74.0 package. Native Windows interaction, Wayland and physical GPU
behavior require their own evidence. Backlog acceptance statuses remain unchanged.
