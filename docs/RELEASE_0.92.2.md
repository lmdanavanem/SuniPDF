# SuniPDF 0.92.2

By Emanuel de Almeida.

Candidate under qualification. [Project status](PROJECT_STATUS.md).

## Clipboard copy on Windows

Text copied from SuniPDF (Ctrl+C and the copy commands) now reaches the Windows clipboard, so it
can be pasted into other applications. Earlier builds kept the copied text inside the application
because the system clipboard support was not compiled into the Windows build. Fixes #157. A
regression test now checks that copied text reaches the Windows clipboard.

## Author name

The author name is spelled Emanuel de Almeida in the package metadata, the executable's copyright
resource, the About dialog and the documentation.

See the [changelog](../CHANGELOG.md) and [known limitations](KNOWN_ISSUES.md).
