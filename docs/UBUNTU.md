# SuniPDF on Ubuntu
[User guide](USER_GUIDE.en.md)

The Ubuntu package targets **Ubuntu 22.04 and 24.04, amd64 (x86_64)**. It installs the desktop application and `sunipdf-cli`. Linux ARM64 is not included in this package. See the [release page](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.61.0) for available downloads and qualification results.

## Install

Download `SuniPDF-0.61.0-linux-amd64.deb` and its `.sha256` file from the release page. In the download directory:

```sh
sha256sum --check SuniPDF-0.61.0-linux-amd64.deb.sha256
sudo apt install ./SuniPDF-0.61.0-linux-amd64.deb
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

The update notification opens the official release page through **Download update**. Download the new `.deb`, verify its checksum, then install it with `sudo apt install ./<downloaded-package>.deb`. The Linux application does not run the Windows installer or silently elevate privileges. This distribution does not configure an APT update repository.

To remove the application:

```sh
sudo apt remove sunipdf
```

Removing the package preserves your PDFs and per-user preferences and recovery files.

## Validation scope

Package validation installs the same built `.deb` on Ubuntu 22.04 and 24.04 and checks its declared dependencies, installed commands, PDF operations, OCR and a desktop launch under a virtual display. The published release evidence identifies the actual source and completed checks. Virtual-display checks do not qualify every physical GPU, desktop environment, accessibility tool or printer.
