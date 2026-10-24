# SuniPDF on Ubuntu
[User guide](USER_GUIDE.en.md)

The Ubuntu package targets **Ubuntu 22.04 and 24.04, amd64 (x86_64)**. It installs the desktop application and `sunipdf-cli`. Linux ARM64 is not included in this package. See [current downloads](../README.md#download-0750) and [project status](PROJECT_STATUS.md) for available packages and qualification results.

## Install

Download `SuniPDF-0.75.0-linux-amd64.deb` and its `.sha256` file from the release page. In the download directory:

```sh
sha256sum --check SuniPDF-0.75.0-linux-amd64.deb.sha256
sudo apt install ./SuniPDF-0.75.0-linux-amd64.deb
```

APT installs the declared system dependencies. An Internet connection may be needed for this installation. PDF editing and OCR run locally afterwards.

Start **SuniPDF** from the application menu, or run:

```sh
sunipdf /path/to/document.pdf
sunipdf-cli --help
```

The package registers an application capable of opening PDF files. It does not replace your default PDF reader automatically.

## Available workflows

The Linux application shares the PDF engine, reader, annotations, measurements, quantity tables, page operations, form tools and PDF exports with the Windows application. Existing document permissions, previews and save guards still apply.

OCR uses Ubuntu's local Tesseract executable with the English and French language packages. Fonts for PDF editing are discovered from Linux font directories. Dialogues use the desktop's file portal and its available fallback.

## Platform limits

The following features currently require Windows and are unavailable in the Ubuntu interface:

- Native printing from SuniPDF. Open the saved PDF in another installed application to print it.
- Certificate signing and operating-system certificate trust verification. Inspecting signature structures does not establish cryptographic validity or trust.
- Office-document import through the protected Windows conversion workflow. PDF-to-Office exports remain available within their documented limits.
- Persistent encrypted storage of visual signatures. Drawing, importing and placing a visual signature for the current session remain available.

The application must not alter signed documents through actions prohibited by its existing signature guards. A visual signature is an annotation and does not provide a digital certificate signature.

## Settings, recovery and updates

Preferences and annotation-library settings use `$XDG_CONFIG_HOME/sunipdf`, or `$HOME/.config/sunipdf` when that variable is unset. Recovery uses `$XDG_STATE_HOME/sunipdf/recovery`, or `$HOME/.local/state/sunipdf/recovery`. Only absolute XDG paths are accepted. Active recovery sessions use an operating-system file lock.

Starting with 0.74, choose **Download update** when a newer stable version is available.
SuniPDF downloads and verifies the DEB, then offers **Install update**. Linux asks for
administrator authorization through the desktop's authentication agent. Keep SuniPDF open
while the system package manager works. Your documents stay open; after installation succeeds,
choose **Restart SuniPDF** when ready. Normal save prompts protect changed documents before restart.

If authorization is cancelled or installation fails, the application reports the failure and lets
you retry. Installation requires the official system-wide Ubuntu amd64 package and an available
desktop authentication agent. No APT repository is configured.

**Upgrading from 0.73 or earlier:** install the current DEB once through your package manager to obtain
the new updater and its installed authorization helper. Earlier binaries continue opening the
release page. Manual installation with `sudo apt install ./<downloaded-package>.deb` remains available.

The installed helper independently checks the public release identity, stages verified bytes in
root-owned storage and refuses older or mismatched packages. This still relies on the official
GitHub release channel; it is not an independently signed publisher manifest. See [update trust](SECURITY_MODEL.md#command-line-passwords-and-update-trust).

To remove the application:

```sh
sudo apt remove sunipdf
```

Removing the package preserves your PDFs and per-user preferences and recovery files.

## Validation scope

Package validation installs the same built `.deb` on Ubuntu 22.04 and 24.04 and checks its declared dependencies, installed commands, PDF operations, OCR and a desktop launch under a virtual display. The published release evidence identifies the actual source and completed checks. Virtual-display checks do not qualify every physical GPU, desktop environment, accessibility tool or printer.
