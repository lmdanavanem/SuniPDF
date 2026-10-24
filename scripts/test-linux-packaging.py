#!/usr/bin/env python3
"""Regression checks for Linux archive and executable package boundaries."""
import importlib.util
import gzip
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
installed = load("test-deb.py")


class LinuxPackagingTests(unittest.TestCase):
    def test_checkbox_target_accepts_unique_word_beside_merged_glyph_only(self):
        header = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"
        first = "5\t1\t1\t1\t1\t1\t20\t30\t50\t14\t88\t\u2610Reduce\n"
        second = "5\t1\t1\t1\t1\t2\t74\t30\t90\t14\t95\ttransparency\n"
        self.assertEqual(installed.theme_label_center(header + first + second, "transparency", (200, 80)), (119, 37))
        duplicate = second.replace("\t1\t1\t1\t1\t2\t", "\t1\t2\t1\t1\t2\t")
        for text, bounds in ((header + first, (200, 80)), (header + first + second, (160, 80)),
                             (header + first + second + duplicate, (200, 80)),
                             (header + second.replace("transparency", "transparencies"), (200, 80))):
            with self.subTest(text=text, bounds=bounds), self.assertRaises(AssertionError):
                installed.theme_label_center(text, "transparency", bounds)

    def test_theme_label_targets_require_unique_complete_visible_text(self):
        header = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"
        first = "5\t1\t1\t1\t1\t1\t20\t30\t40\t14\t95\tLiquid\n"
        second = "5\t1\t1\t1\t1\t2\t65\t30\t30\t14\t95\tGlass\n"
        text = header + first + second
        self.assertEqual(installed.theme_label_center(text, "Liquid Glass", (120, 80)), (57, 37))
        dark = first.replace("Liquid", "Dark")
        description = (first.replace("\t1\t1\t1\t1\t1\t", "\t1\t2\t1\t1\t1\t").replace("Liquid", "Follow")
                       + second.replace("\t1\t1\t1\t1\t2\t", "\t1\t2\t1\t1\t2\t").replace("Glass", "dark"))
        self.assertEqual(installed.theme_label_center(header + dark + description, "Dark", (120, 80)), (40, 37))
        for bad, bounds in ((header + first, (120, 80)), (text, (80, 80)),
                            (text + first.replace("\t1\t1\t1\t1\t1\t", "\t1\t2\t1\t1\t1\t")
                             + second.replace("\t1\t1\t1\t1\t2\t", "\t1\t2\t1\t1\t2\t"), (120, 80))):
            with self.subTest(bounds=bounds, text=bad):
                with self.assertRaises(AssertionError):
                    installed.theme_label_center(bad, "Liquid Glass", bounds)

    def test_native_fixture_mask_excludes_dark_ui_text_and_non_blue_pixels(self):
        pixels = [(0, 0, 255), (30, 40, 220), (0, 0, 0), (40, 50, 60),
                  (255, 255, 255), (255, 0, 0), (70, 0, 255), (0, 70, 255), (0, 0, 180)]
        mask, blue = installed.blue_fixture_mask(pixels)
        self.assertEqual(mask, bytes([0, 0] + [255] * 7))
        self.assertEqual(blue, 2)
        self.assertEqual(installed.blue_fixture_mask([(0, 0, 0)] * 3000), (bytes([255]) * 3000, 0))

    def test_native_fixture_requires_both_blue_area_and_correct_pdf_text(self):
        self.assertTrue(installed.fixture_text_is_visible(2001, "SuniPDF page 1"))
        for count, text in [(0, "SuniPDF page 1"), (2000, "SuniPDF page 1"),
                            (3000, "SuniPDF"), (3000, "SuniPDF page 2"),
                            (3000, "SuniPDF page 10"), (3000, "")]:
            with self.subTest(count=count, text=text):
                self.assertFalse(installed.fixture_text_is_visible(count, text))

    def test_distribution_identity_preserves_diagnostics(self):
        content = 'NAME="Ubuntu"\nID=ubuntu\nVERSION_ID="24.04"\n'
        with patch.object(Path, "read_text", return_value=content):
            identity = installed.distribution_identity()
        self.assertEqual(identity["distro"], {"id": "ubuntu", "version_id": "24.04"})
        self.assertEqual(identity["osRelease"], content)

    def test_documentation_preserves_license_and_limits_pdfium_exceptions(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            package.package_documentation(stage, {"version": "0.51.0", "repository": "https://github.com/lmdanavanem/SuniPDF"})
            notice = (stage / "usr/share/doc/sunipdf/copyright").read_text(encoding="utf-8")
            self.assertIn((package.ROOT / "LICENSE-MIT").read_text(encoding="utf-8"), notice)
            self.assertIn("/usr/share/common-licenses/Apache-2.0", notice)
            overrides = (stage / "usr/share/lintian/overrides/sunipdf").read_text(encoding="utf-8")
            active = [line for line in overrides.splitlines() if line and not line.startswith("#")]
            self.assertEqual(active, [
                "sunipdf: embedded-library freetype usr/lib/sunipdf/libpdfium.so",
                "sunipdf: embedded-library lcms2 usr/lib/sunipdf/libpdfium.so",
                "sunipdf: embedded-library openjpeg usr/lib/sunipdf/libpdfium.so",
                "sunipdf: unstripped-binary-or-object usr/lib/sunipdf/libpdfium.so",
            ])
            for command in ("sunipdf", "sunipdf-cli"):
                manual = gzip.decompress((stage / "usr/share/man/man1" / (command + ".1.gz")).read_bytes()).decode("utf-8")
                self.assertIn(f'.TH {command.upper()} 1 "" "SuniPDF 0.51.0"', manual)
                self.assertIn(".SH SYNOPSIS", manual)
                self.assertIn(".SH SEE ALSO", manual)

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
