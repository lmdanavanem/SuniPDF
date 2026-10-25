# SuniPDF 0.84.0

Development candidate; publication and package qualification are pending.

## Desktop plan-link workflow

This increment exercises link creation and saving through the installed desktop
application. The qualification uses synthetic source drawings and a separate
destination PDF, with explicit candidate review and new output copies.

Source fixtures and an indexed plan manifest are prepared separately. Creating
the actual links, reviewing them and saving the copies must occur through the
desktop controls. Independent checks retain source hashes and inspect the saved
link destinations. Existing navigation, cancellation and changed-source checks
remain separate cases.

## Ubuntu update verification

The installer verifies the private staged package's local size and SHA256 before
requesting the public release metadata. Corrupt downloads fail locally without
using a GitHub API request. The official release identity must still match before
the DEB is parsed or installed. A network refusal leaves the installation intact
and removes the temporary staged copy.

## Qualification and limits

Results will be recorded only after the exact-source package checks finish on
Ubuntu 22.04 and 24.04. The recorded environment uses X11, Openbox and software
rendering. It does not establish behavior on physical GPUs, Wayland, private
project drawings or every external PDF reader.

[Plan links](PLAN_LINKS.md) · [Roadmap](ROADMAP.md).
