# SuniPDF 0.74.0

Published stable. [Downloads](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.74.0) · [Qualification evidence](qa/RELEASE_0.74.0.json).

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

## Qualification

The exact tagged Windows and Ubuntu builds, workspace tests, independent readers and package checks passed. Updates covered 51 portable baselines plus four guards and 49 MSI baselines. Eight public files were downloaded without authentication and verified against their SHA256 records.

The automated release upload received HTTP 403. The owner uploaded the unchanged qualified artifacts; the failed transport and matching hashes remain recorded in the evidence.

Theme persistence, restart and window controls were exercised on Ubuntu 22.04/24.04 under X11/Openbox/Xvfb. Ten Windows DX12 WARP scenes were reviewed offscreen. The installed Ubuntu helper upgraded two actual public DEB releases and checked refusals; the polkit dialog and complete desktop update interaction were not tested end to end.

Windows ARM64 is cross-built. Physical GPUs, Wayland, physical printing and human accessibility remain open. Backlog acceptance statuses are unchanged. Historical CLI corpus counts are not reused as 0.74 results.
