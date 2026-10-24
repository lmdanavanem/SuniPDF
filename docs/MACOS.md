# SuniPDF on macOS
[User guide](USER_GUIDE.en.md)

The macOS package targets **macOS 12 or later on Apple silicon (arm64)**. It contains the desktop application and `sunipdf-cli`. Intel Macs are not included in this package. See the [releases page](https://github.com/lmdanavanem/SuniPDF/releases) for available downloads.

## Install

Download `SuniPDF-<version>-macos-arm64.dmg` and its `.sha256` file from the release page. In the download directory:

```sh
shasum -a 256 -c SuniPDF-<version>-macos-arm64.dmg.sha256
```

Open the DMG and drag **SuniPDF** to **Applications**. The application is ad hoc signed but not notarized: on first launch, macOS blocks it once. Open **System Settings > Privacy & Security** and choose **Open Anyway**.

The command-line interface is inside the application bundle:

```sh
/Applications/SuniPDF.app/Contents/MacOS/sunipdf-cli --help
```

PDFs open from the Finder, and **SuniPDF > Use SuniPDF as Default PDF App** makes it the default PDF application. It does not replace your default PDF reader automatically.

## Available workflows

The macOS application shares the PDF engine, reader, annotations, measurements, quantity tables, page operations, form tools and PDF exports with the Windows and Linux applications. Existing document permissions, previews and save guards still apply.

The menu bar offers the same commands as the in-app **Menu**. Shortcuts use **⌘** where Windows uses Ctrl, **⌘⌫** deletes a page, **⌦** (fn+⌫) deletes the selected object and **⌃Tab** switches tabs. Fonts for PDF editing are discovered from the macOS font folders.

## Platform limits

The following features currently require Windows or Linux and are unavailable in the macOS interface:

- Native printing from SuniPDF. Open the saved PDF in another application, such as Preview, to print it.
- Certificate signing and operating-system certificate trust verification. Inspecting signature structures does not establish cryptographic validity or trust.
- OCR. Searchable text cannot be added to scanned pages on macOS yet.
- Office-document import through the protected Windows conversion workflow. PDF-to-Office exports remain available within their documented limits.
- Persistent encrypted storage of visual signatures. Drawing, importing and placing a visual signature for the current session remain available.
- In-app updates. Download the new DMG from the releases page and replace the application.

The application must not alter signed documents through actions prohibited by its existing signature guards. A visual signature is an annotation and does not provide a digital certificate signature.

## Settings and recovery

Preferences, annotation-library settings and recovery files use `~/Library/Application Support/SuniPDF`. Active recovery sessions use an operating-system file lock. Replacing or removing the application preserves your PDFs, preferences and recovery files.

## Validation scope

Package validation builds the DMG on macOS 15, verifies its checksum, code signature and license inventory, then copies the application out of the image as a user would, runs the bundled command-line interface against a fixture with the bundled PDFium, and starts the desktop application with a PDF. These checks do not qualify every Mac model, display, accessibility tool or printer.
