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


def shared_dependencies(stage, temporary):
    working = temporary / "shlibs"
    write(working / "debian/control", "Source: sunipdf\nSection: graphics\nPriority: optional\nMaintainer: " + MAINTAINER + "\n\nPackage: sunipdf\nArchitecture: amd64\nDescription: Local PDF reader and editor\n")
    binaries = [stage / "usr/lib/sunipdf" / name for name in ("sunipdf", "sunipdf-cli", "libpdfium.so")]
    output = run(["dpkg-shlibdeps", "-O", *["-e" + str(path) for path in binaries]], cwd=working)
    match = re.search(r"^shlibs:Depends=(.+)$", output, re.M)
    if not match:
        raise ValueError("No ELF shared-library dependency evidence was generated")
    return match[1]


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
        copy(ROOT / "assets/linux/sunipdf.desktop", stage / "usr/share/applications/sunipdf.desktop")
        run(["desktop-file-validate", stage / "usr/share/applications/sunipdf.desktop"])
        for size in (16, 24, 32, 48, 64, 128, 256, 512):
            copy(ROOT / f"assets/icon/sunipdf-{size}.png", stage / f"usr/share/icons/hicolor/{size}x{size}/apps/sunipdf.png")
        for name in ("USER_GUIDE.en.md", "USER_GUIDE.fr.md"):
            copy(ROOT / "docs" / name, doc / name)
        run([sys.executable, ROOT / "scripts/package-guides.py", "--source-root", ROOT, "--stage", doc, "--tag", "v" + version])
        guides = module("package-guides")
        for name in ("UBUNTU.md", "UBUNTU.fr.md"):
            source = ROOT / "docs" / name
            prepared, _ = guides.rewrite_guide(source.read_text(encoding="utf-8"), source, ROOT, "v" + version)
            write(doc / name, prepared)
        (doc / "changelog.gz").write_bytes(gzip.compress((ROOT / "CHANGELOG.md").read_bytes(), mtime=0))
        changelog_date = run(["git", "show", "-s", "--format=%aD", "HEAD"], cwd=ROOT)
        debian_changelog = f"sunipdf ({version}) stable; urgency=medium\n\n  * Package the local PDF desktop and CLI for Ubuntu amd64.\n\n -- {MAINTAINER}  {changelog_date}\n"
        (doc / "changelog.Debian.gz").write_bytes(gzip.compress(debian_changelog.encode("utf-8"), mtime=0))
        write(doc / "copyright", "SuniPDF\nCopyright Emanuel DE ALMEIDA and contributors\nSource: " + manifest["repository"] + "\n\nThe application is available under MIT OR Apache-2.0.\nFull source, native-library and embedded-font license texts are in licenses/dependency-licenses.txt.\nThe complete installed payload inventory is in licenses/dependency-inventory.json.\nTesseract and its English/French models are supplied by Ubuntu packages; their copyright files are in /usr/share/doc.\n")
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
        run(["dpkg-deb", "--root-owner-group", "--build", stage, artifact])
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
