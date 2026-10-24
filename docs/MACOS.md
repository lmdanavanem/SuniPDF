# SuniPDF on macOS
[User guide](USER_GUIDE.en.md)

The macOS package targets **Apple Silicon (arm64)**, with a macOS 12 deployment minimum checked against every bundled binary. Qualification runs on macOS 15; older systems and individual Mac models have not been tested. It contains the desktop application and `sunipdf-cli`. Download the [0.76.0 Apple Silicon DMG](https://github.com/lmdanavanem/SuniPDF/releases/download/v0.76.0/SuniPDF-0.76.0-macos-arm64.dmg) and see its [qualification record](qa/RELEASE_0.76.0.json).

## Install

Download `SuniPDF-<version>-macos-arm64.dmg` and its `.sha256` file from the release page. In the download directory:

```sh
shasum -a 256 -c SuniPDF-<version>-macos-arm64.dmg.sha256
```

Open the DMG and drag **SuniPDF** to **Applications**. The application is ad hoc signed but not notarized, so macOS may block the first launch. After attempting to open it, use **System Settings > Privacy & Security > Open Anyway** if that option is offered.

The command-line interface is inside the application bundle:

```sh
/Applications/SuniPDF.app/Contents/MacOS/sunipdf-cli --help
```

PDFs open from the Finder, and **SuniPDF > Use SuniPDF as Default PDF App** makes it the default PDF application. It does not replace your default PDF reader automatically.

## Available workflows

The macOS application shares the PDF engine, reader, annotations, measurements, quantity tables, page operations, form tools and PDF exports with the Windows and Linux applications. Existing document permissions, previews and save guards still apply.

The menu bar offers the same commands as the in-app **Menu**. Shortcuts use **⌘** where Windows uses Ctrl, **⌘⌫** deletes a page, **⌦** (fn+⌫) deletes the selected object and **⌃Tab** switches tabs. Fonts for PDF editing are discovered from the macOS font folders.

## Reader views in 0.76

Version 0.76 adds two views of the same PDF. Use **Reading
options > Split document view**, or find the command with **Command+K**. Page, zoom and
editing controls follow the active pane; **Synchronize views** links page, zoom,
scroll position and view rotation. The workspace lasts for the current session.
See the [reader workflow](READER_WORKSPACE.md#two-independent-views) and
[release qualification](RELEASE_0.76.0.md).

## Platform limits

The following features currently require Windows or Linux and are unavailable in the macOS interface:

- Native printing from SuniPDF. Open the saved PDF in another application, such as Preview, to print it.
- Certificate signing and operating-system certificate trust verification. Inspecting signature structures does not establish cryptographic validity or trust.
- OCR. Searchable text cannot be added to scanned pages on macOS yet.
- Office-document import through the protected Windows conversion workflow. PDF-to-Office exports remain available within their documented limits.
- Persistent encrypted storage of visual signatures. Drawing, importing and placing a visual signature for the current session remain available.
- Automatic update installation. The application can notify you about a newer release and open its official download page. Download the new DMG, quit SuniPDF after saving your documents and replace the application in Applications.

The application must not alter signed documents through actions prohibited by its existing signature guards. A visual signature is an annotation and does not provide a digital certificate signature.

## Settings and recovery

Preferences, annotation-library settings and recovery files use `~/Library/Application Support/SuniPDF`. Active recovery sessions use an operating-system file lock. Replacing or removing the application preserves your PDFs, preferences and recovery files.

## Validation scope

Package validation builds the DMG on macOS 15, verifies its checksum, ad hoc code signature, license inventory, arm64 binaries, deployment minimum and library dependencies. It then copies the application out of the image, opens and renders a PDF with the packaged CLI, saves and reopens a rotated copy and starts the desktop application with a PDF. The source PDF must remain unchanged. The user guide is included in the app resources with links pinned to the release tag.

These checks do not establish notarization, interactive Finder opening, clipboard interoperability, physical Mac testing or compatibility with every display, accessibility tool and printer. Their scope is recorded in the release evidence.
