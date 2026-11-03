# SuniPDF 0.92.1

By Emmanuel Almeida.

Prepared for publication. Tagged package checks and public download verification are pending.
[Project status](PROJECT_STATUS.md).

## Window at startup

Open **Menu → Settings → Theme** and choose **Window at startup**. **Last state** (the default)
reopens SuniPDF maximized if it was maximized when closed; **Maximized** and **Windowed** always
open that way. A windowed start is centered on the screen. Ubuntu Wayland sessions choose the
position themselves. The choice applies at the next start. [Theme guide](APPEARANCE.md).

## Taskbar icon after Windows updates

The Start Menu shortcut installed by the MSI now uses the icon embedded in `sunipdf.exe`.
Earlier versions pointed it to an installer copy that each update removed, so taskbar pins
showed a blank document icon. A pin that is already blank must be unpinned and pinned again once
after installing this version. The real MSI upgrade regression now checks the shortcut icon.

## Build reliability

Continuous integration removes evidence folders restored with its build cache before running
the packaged regressions, which had refused to reuse an existing output folder.

See the [changelog](../CHANGELOG.md) and [known limitations](KNOWN_ISSUES.md).
