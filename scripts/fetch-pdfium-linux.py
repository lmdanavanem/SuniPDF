#!/usr/bin/env python3
"""Fetch and verify the pinned PDFium Linux amd64 distribution."""
import argparse
import hashlib
from pathlib import Path, PurePosixPath
import shutil
import tarfile
import tempfile
import urllib.request

BUILD = "7881"
URL = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/7881/pdfium-linux-x64.tgz"
ARCHIVE_SHA256 = "1470e21b8b4a3b4ad7f85684e2da11d94f3b69a86d81dee11b9b6709d927ac1d"
LIBRARY_SHA256 = "f728930966f503652b92acc89b9374a2eeca00ce42e26dccd3e4b5c5161b2d64"
MAX_DOWNLOAD = 16 * 1024 * 1024
MAX_EXPANDED = 64 * 1024 * 1024


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify(directory):
    if sha(directory / "lib/libpdfium.so") != LIBRARY_SHA256:
        raise ValueError("PDFium Linux library checksum mismatch")
    if (directory / "VERSION").read_text().splitlines() != ["MAJOR=151", "MINOR=0", "BUILD=7881", "PATCH=0"]:
        raise ValueError("Unexpected PDFium version")
    if not (directory / "LICENSE").is_file() or not list((directory / "licenses").glob("*.txt")):
        raise ValueError("PDFium legal notices are missing")


def extract(archive, directory):
    with tarfile.open(archive, "r:gz") as bundle:
        members = bundle.getmembers()
        size = 0
        seen = set()
        for member in members:
            name = PurePosixPath(member.name)
            if (name.is_absolute() or ".." in name.parts or "\\" in member.name
                    or member.issym() or member.islnk() or not (member.isdir() or member.isfile())):
                raise ValueError("Unsafe PDFium archive member")
            normalized = name.as_posix()
            if normalized in seen:
                raise ValueError("Duplicate PDFium archive member")
            seen.add(normalized)
            size += member.size
            if size > MAX_EXPANDED:
                raise ValueError("PDFium archive exceeds its expanded size bound")
        for member in members:
            destination = directory.joinpath(*PurePosixPath(member.name).parts)
            if member.isdir():
                destination.mkdir(parents=True, exist_ok=True)
            else:
                destination.parent.mkdir(parents=True, exist_ok=True)
                with bundle.extractfile(member) as source, destination.open("wb") as output:
                    shutil.copyfileobj(source, output)


def fetch(destination):
    destination = destination.resolve()
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        raise ValueError("PDFium output already exists; use a fresh directory for verified extraction")
    with tempfile.TemporaryDirectory(prefix="pdfium-linux-", dir=destination.parent) as temporary:
        temporary = Path(temporary)
        archive = temporary / "pdfium-linux-x64.tgz"
        request = urllib.request.Request(URL, headers={"User-Agent": "SuniPDF-build"})
        with urllib.request.urlopen(request, timeout=120) as response, archive.open("wb") as output:
            total = 0
            while chunk := response.read(1024 * 1024):
                total += len(chunk)
                if total > MAX_DOWNLOAD:
                    raise ValueError("PDFium download exceeds its size bound")
                output.write(chunk)
        if sha(archive) != ARCHIVE_SHA256:
            raise ValueError("PDFium Linux archive checksum mismatch")
        stage = temporary / "payload"
        stage.mkdir()
        extract(archive, stage)
        verify(stage)
        shutil.move(str(stage), str(destination))
    return destination


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path(__file__).resolve().parents[1] / "third_party/pdfium/linux-x64")
    arguments = parser.parse_args()
    print(fetch(arguments.output))
