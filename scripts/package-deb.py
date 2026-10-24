#!/usr/bin/env python3
"""Package previously built Linux amd64 binaries as a verified Ubuntu .deb."""
import argparse
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
TARGET = "x86_64-unknown-linux-gnu"
MAINTAINER = "Emanuel DE ALMEIDA <287339505+lmdanavanem@users.noreply.github.com>"
RUNTIME_DEPENDENCIES = (
    "libx11-6", "libx11-xcb1", "libxcb1", "libxcursor1", "libxrandr2", "libxi6",
    "libxkbcommon0", "libxkbcommon-x11-0", "libwayland-client0", "libwayland-cursor0",
    "libwayland-egl1", "libegl1", "libgl1", "libvulkan1", "libdbus-1-3",
    "xdg-desktop-portal", "xdg-desktop-portal-gtk | xdg-desktop-portal-gnome | xdg-desktop-portal-kde",
    "zenity", "tesseract-ocr (>= 4)", "tesseract-ocr-eng", "tesseract-ocr-fra",
    "pkexec", "python3", "ca-certificates",
)


def run(arguments, **kwargs):
    return subprocess.check_output([str(value) for value in arguments], text=True, **kwargs).strip()


def module(name):
    specification = importlib.util.spec_from_file_location(name.replace("-", "_"), ROOT / "scripts" / (name + ".py"))
    result = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(result)
    return result


def require_elf_amd64(path):
    with path.open("rb") as source:
        header = source.read(20)
    if len(header) != 20 or header[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", header, 18)[0] != 62:
        raise ValueError("Expected a little-endian ELF64 amd64 binary: " + str(path))


def copy(source, target, executable=False):
    if source.is_symlink() or not source.is_file():
        raise ValueError("Expected a regular package source: " + str(source))
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    target.chmod(0o755 if executable else 0o644)


def write(path, text, executable=False):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")
    path.chmod(0o755 if executable else 0o644)


def package_documentation(stage, manifest):
    doc = stage / "usr/share/doc/sunipdf"
    mit_license = (ROOT / "LICENSE-MIT").read_text(encoding="utf-8")
    write(doc / "copyright", "SuniPDF\nSource: " + manifest["repository"] + "\n\n"
          "The application is available under MIT OR Apache-2.0, at your option.\n\n"
          + mit_license + "\n"
          "Alternatively, the application may be used under the Apache License, Version 2.0.\n"
          "On Debian and Ubuntu, its complete text is available in\n"
          "/usr/share/common-licenses/Apache-2.0.\n\n"
          "Full dependency, native-library and embedded-font license and copyright notices\n"
          "are in licenses/dependency-licenses.txt. The installed payload inventory is in\n"
          "licenses/dependency-inventory.json. Tesseract and its English/French models\n"
          "are supplied by Ubuntu packages; their copyright files are in /usr/share/doc.\n")
    write(stage / "usr/share/lintian/overrides/sunipdf", """# The private PDFium library is the checksum-pinned upstream distribution.
# Its bundled FreeType, Little CMS and OpenJPEG are upstream build components;
# replacing them individually would change the qualified PDFium build. Their
# versions and license notices are retained in the installed license inventory.
# These exceptions apply only to the private PDFium file, not other binaries.
sunipdf: embedded-library freetype usr/lib/sunipdf/libpdfium.so
sunipdf: embedded-library lcms2 usr/lib/sunipdf/libpdfium.so
sunipdf: embedded-library openjpeg usr/lib/sunipdf/libpdfium.so
# Preserve the exact upstream bytes (including symbols) so the downloaded,
# packaged and installed SHA-256 checks can all verify the same PDFium binary.
sunipdf: unstripped-binary-or-object usr/lib/sunipdf/libpdfium.so
""")
    manuals = {
        "sunipdf": r""".SH NAME
sunipdf \- local PDF reader, editor and measurement workspace
.SH SYNOPSIS
.B sunipdf
.RI [ PDF-files ...]
.SH DESCRIPTION
Open PDF documents in the desktop workspace to read, annotate, edit and measure
them locally. English and French OCR use the installed Tesseract packages.
.SH FILES
.I ${XDG_CONFIG_HOME:-$HOME/.config}/sunipdf
stores application preferences.
.PP
.I ${XDG_STATE_HOME:-$HOME/.local/state}/sunipdf/recovery
stores recovery data.
.SH SEE ALSO
.BR sunipdf-cli (1)
.PP
The user guide and Ubuntu platform limitations are installed in
.IR /usr/share/doc/sunipdf/ .
""",
        "sunipdf-cli": r""".SH NAME
sunipdf-cli \- process PDF documents locally from the command line
.SH SYNOPSIS
.B sunipdf-cli
.RI [ options ] " command " [ arguments ]
.SH DESCRIPTION
Use the same local PDF engine as the desktop application for document inspection,
rendering, extraction, OCR, editing, annotations and measurements.
.PP
Run
.B sunipdf-cli \-\-help
to list commands, or
.B sunipdf-cli command \-\-help
for a command's arguments. Existing output files are protected unless the
.B \-\-overwrite
option is explicitly supplied.
.SH OPTIONS
.TP
.B \-h, \-\-help
Show usage and available commands.
.TP
.B \-V, \-\-version
Show the installed version.
.TP
.B \-\-overwrite
Allow replacing existing output files.
.SH SEE ALSO
.BR sunipdf (1)
.PP
The user guide and Ubuntu platform limitations are installed in
.IR /usr/share/doc/sunipdf/ .
""",
    }
    for name, content in manuals.items():
        path = stage / "usr/share/man/man1" / (name + ".1.gz")
        path.parent.mkdir(parents=True, exist_ok=True)
        header = f'.TH {name.upper()} 1 "" "SuniPDF {manifest["version"]}" "User Commands"\n'
        path.write_bytes(gzip.compress((header + content).encode("utf-8"), mtime=0))
        path.chmod(0o644)


def shared_dependencies(stage, temporary):
    working = temporary / "shlibs"
    write(working / "debian/control", "Source: sunipdf\nSection: graphics\nPriority: optional\nMaintainer: " + MAINTAINER + "\n\nPackage: sunipdf\nArchitecture: amd64\nDescription: Local PDF reader and editor\n")
    binaries = [stage / "usr/lib/sunipdf" / name for name in ("sunipdf", "sunipdf-cli", "libpdfium.so")]
    output = run(["dpkg-shlibdeps", "-O", *["-e" + str(path) for path in binaries]], cwd=working)
    match = re.search(r"^shlibs:Depends=(.+)$", output, re.M)
    if not match:
        raise ValueError("No ELF shared-library dependency evidence was generated")
    return match[1]


def package_update_helper(stage):
    # Normalize the executable shebang even when packaging a Windows checkout.
    write(stage / "usr/lib/sunipdf/install-update",
          (ROOT / "packaging/linux/install-update").read_text(encoding="utf-8"), executable=True)
    write(stage / "usr/share/polkit-1/actions/org.sunipdf.update.policy",
          (ROOT / "packaging/linux/org.sunipdf.update.policy").read_text(encoding="utf-8"))


def package(binary_dir, output):
    if sys.platform != "linux" or run(["dpkg", "--print-architecture"]) != "amd64":
        raise ValueError("Build the Debian package on a Linux amd64 runner")
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]
    version = manifest["version"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("Expected a stable three-part release version")
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    filename = f"SuniPDF-{version}-linux-amd64.deb"
    artifact = output / filename
    if artifact.exists() or artifact.with_suffix(".deb.sha256").exists():
        raise ValueError("Package output already exists; use a fresh output directory")
    pdfium = ROOT / "third_party/pdfium/linux-x64"
    module("fetch-pdfium-linux").verify(pdfium)
    with tempfile.TemporaryDirectory(prefix="sunipdf-deb-", dir=output) as directory:
        temporary = Path(directory)
        stage = temporary / "stage"
        doc = stage / "usr/share/doc/sunipdf"
        for name in ("sunipdf", "sunipdf-cli"):
            source = binary_dir / name
            require_elf_amd64(source)
            copy(source, stage / "usr/lib/sunipdf" / name, executable=True)
            write(stage / "usr/bin" / name, '#!/bin/sh\nexec /usr/lib/sunipdf/' + name + ' "$@"\n', executable=True)
        require_elf_amd64(pdfium / "lib/libpdfium.so")
        copy(pdfium / "lib/libpdfium.so", stage / "usr/lib/sunipdf/libpdfium.so")
        package_update_helper(stage)
        copy(ROOT / "assets/linux/sunipdf.desktop", stage / "usr/share/applications/sunipdf.desktop")
        run(["desktop-file-validate", stage / "usr/share/applications/sunipdf.desktop"])
        for size in (16, 24, 32, 48, 64, 128, 256, 512):
            copy(ROOT / f"assets/icon/sunipdf-{size}.png", stage / f"usr/share/icons/hicolor/{size}x{size}/apps/sunipdf.png")
        guides = module("package-guides")
        for name in guides.GUIDES:
            copy(ROOT / "docs" / name, doc / name)
        run([sys.executable, ROOT / "scripts/package-guides.py", "--source-root", ROOT, "--stage", doc, "--tag", "v" + version])
        # Atomic guide replacement creates private temporary files; the installed
        # documentation must remain readable by every desktop user.
        for name in guides.GUIDES:
            (doc / name).chmod(0o644)
        for name in ("UBUNTU.md",):
            source = ROOT / "docs" / name
            prepared, _ = guides.rewrite_guide(source.read_text(encoding="utf-8"), source, ROOT, "v" + version)
            write(doc / name, prepared)
        (doc / "changelog.gz").write_bytes(gzip.compress((ROOT / "CHANGELOG.md").read_bytes(), mtime=0))
        changelog_date = run(["git", "show", "-s", "--format=%aD", "HEAD"], cwd=ROOT)
        debian_changelog = f"sunipdf ({version}) stable; urgency=medium\n\n  * Package the local PDF desktop and CLI for Ubuntu amd64.\n\n -- {MAINTAINER}  {changelog_date}\n"
        (doc / "changelog.Debian.gz").write_bytes(gzip.compress(debian_changelog.encode("utf-8"), mtime=0))
        package_documentation(stage, manifest)
        write(doc / "README", "SuniPDF " + version + " for Ubuntu amd64\n\nLaunch sunipdf [PDF files...] or sunipdf-cli --help.\nInstall using: sudo apt install ./" + filename + "\nOCR uses locally installed Tesseract with English and French models.\nPreferences: ${XDG_CONFIG_HOME:-$HOME/.config}/sunipdf\nUpdating or removing this package does not remove user documents or preferences.\n")
        metadata = temporary / "metadata"
        metadata.mkdir()
        for name, extra in (("all", []), ("linux", ["--filter-platform", TARGET])):
            write(metadata / (name + ".json"), run(["cargo", "metadata", "--locked", "--format-version", "1", *extra], cwd=ROOT) + "\n")
        for name, command in (("rust", ["rustc", "-Vv"]), ("cargo", ["cargo", "-Vv"])):
            write(metadata / (name + ".txt"), run(command) + "\n")
        dependencies = shared_dependencies(stage, temporary)
        write(doc / "build-provenance.json", json.dumps({"schema": 1, "sourceRevision": run(["git", "rev-parse", "HEAD"], cwd=ROOT), "target": TARGET, "elfDependencies": dependencies, "runtimeDependencies": RUNTIME_DEPENDENCIES, "osRelease": Path("/etc/os-release").read_text(), "cc": run(["cc", "--version"])}, indent=2) + "\n")
        license_tool = ROOT / "scripts/license-inventory.py"
        run([sys.executable, license_tool, "--linux", "--stage", stage, "--metadata", metadata / "all.json", "--linux-metadata", metadata / "linux.json", "--rust-sysroot", run(["rustc", "--print", "sysroot"]), "--rust-version-file", metadata / "rust.txt", "--cargo-version-file", metadata / "cargo.txt"])
        installed_size = (sum(path.stat().st_size for path in stage.rglob("*") if path.is_file()) + 1023) // 1024
        write(stage / "DEBIAN/control", f"Package: sunipdf\nVersion: {version}\nArchitecture: amd64\nMaintainer: {MAINTAINER}\nSection: graphics\nPriority: optional\nInstalled-Size: {installed_size}\nHomepage: {manifest['homepage']}\nDepends: {dependencies}, " + ", ".join(RUNTIME_DEPENDENCIES) + "\nRecommends: fonts-dejavu-core\nDescription: Local PDF reader, editor and measurement workspace\n Read, annotate, edit and measure PDF documents locally.\n Includes a command-line interface and offline English/French OCR support.\n")
        for name in ("postinst", "postrm"):
            write(stage / "DEBIAN" / name, "#!/bin/sh\nset -e\nif command -v update-desktop-database >/dev/null 2>&1; then\n    update-desktop-database -q /usr/share/applications\nfi\nif command -v gtk-update-icon-cache >/dev/null 2>&1; then\n    gtk-update-icon-cache -q -t /usr/share/icons/hicolor\nfi\nexit 0\n", executable=True)
        for directory_path in stage.rglob("*"):
            if directory_path.is_dir():
                directory_path.chmod(0o755)
        run(["dpkg-deb", "-Zxz", "--root-owner-group", "--build", stage, artifact])
        extracted = temporary / "extracted"
        run(["dpkg-deb", "--extract", artifact, extracted])
        run([sys.executable, license_tool, "--linux", "--verify", "--stage", extracted])
    checksum = hashlib.sha256(artifact.read_bytes()).hexdigest()
    write(output / (filename + ".sha256"), checksum + "  " + filename + "\n")
    print(artifact)
    return artifact


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary-dir", type=Path, default=ROOT / "target" / TARGET / "release")
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    package(args.binary_dir.resolve(), args.output)
