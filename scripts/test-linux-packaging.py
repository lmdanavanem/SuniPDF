#!/usr/bin/env python3
"""Regression checks for Linux archive and executable package boundaries."""
import importlib.util
import io
from pathlib import Path
import struct
import tarfile
import tempfile
import unittest
from unittest.mock import patch


def load(filename):
    spec = importlib.util.spec_from_file_location(filename.replace("-", "_"), Path(__file__).parent / filename)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


fetch = load("fetch-pdfium-linux.py")
package = load("package-deb.py")


class LinuxPackagingTests(unittest.TestCase):
    def test_archive_rejects_traversal_links_duplicates_and_oversize(self):
        for kind in ("traversal", "symlink", "hardlink", "duplicate", "oversize"):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                archive = root / "test.tgz"
                with tarfile.open(archive, "w:gz") as bundle:
                    item = tarfile.TarInfo("../outside" if kind == "traversal" else "lib/libpdfium.so")
                    if kind in ("symlink", "hardlink"):
                        item.type = tarfile.SYMTYPE if kind == "symlink" else tarfile.LNKTYPE
                        item.linkname = "../../outside"
                        bundle.addfile(item)
                    else:
                        item.size = 4
                        bundle.addfile(item, io.BytesIO(b"test"))
                        if kind == "duplicate":
                            bundle.addfile(item, io.BytesIO(b"test"))
                with patch.object(fetch, "MAX_EXPANDED", 3 if kind == "oversize" else 4096):
                    with self.assertRaises(ValueError):
                        fetch.extract(archive, root / "out")
                self.assertFalse((root / "outside").exists())

    def test_archive_regular_files_preserve_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "test.tgz"
            with tarfile.open(archive, "w:gz") as bundle:
                item = tarfile.TarInfo("licenses/example.txt")
                item.size = 7
                bundle.addfile(item, io.BytesIO(b"terms\r\n"))
            fetch.extract(archive, root / "out")
            self.assertEqual((root / "out/licenses/example.txt").read_bytes(), b"terms\r\n")

    def test_wrong_download_hash_never_installs_output(self):
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "pdfium"
            with patch.object(fetch.urllib.request, "urlopen", return_value=io.BytesIO(b"untrusted archive")):
                with self.assertRaisesRegex(ValueError, "checksum"):
                    fetch.fetch(destination)
            self.assertFalse(destination.exists())

    def test_elf_architecture_rejects_windows_32bit_and_arm64(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "binary"
            header = bytearray(20)
            header[:6] = b"\x7fELF\x02\x01"
            struct.pack_into("<H", header, 18, 62)
            path.write_bytes(header)
            package.require_elf_amd64(path)
            for invalid in (b"MZ" + bytes(18), header[:4] + b"\x01" + header[5:], header[:18] + struct.pack("<H", 183)):
                path.write_bytes(invalid)
                with self.assertRaisesRegex(ValueError, "ELF64 amd64"):
                    package.require_elf_amd64(path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
