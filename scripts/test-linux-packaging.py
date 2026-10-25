#!/usr/bin/env python3
"""Regression checks for Linux archive and executable package boundaries."""
import importlib.util
import gzip
import io
from pathlib import Path
import re
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
plan_fixture = load("native-plan-link-fixture.py")


class LinuxPackagingTests(unittest.TestCase):
    def test_native_error_mask_accepts_accessible_red_but_rejects_pdf_blue_and_neutral_text(self):
        pixels = [(255, 0, 0), (180, 35, 50), (185, 28, 28), (255, 167, 174),
                  (0, 0, 255), (0, 0, 0), (40, 40, 40), (255, 255, 255),
                  (220, 210, 215), (127, 0, 0), (180, 135, 80), (180, 70, 145)]
        self.assertEqual(installed.native_error_mask(pixels), (bytes([0] * 4 + [255] * 8), 4))
        message = "engine error: Plan set: source fingerprint changed; explicitly refresh this file"
        self.assertTrue(installed.plan_error_is_visible("changed-target", 20, message))
        self.assertFalse(installed.plan_error_is_visible("changed-target", 19, message))
        self.assertFalse(installed.plan_error_is_visible("changed-target", 500, "Destination verified"))
        self.assertFalse(installed.plan_error_is_visible("missing-target", 500, message))

    def test_plan_navigation_requires_unique_colored_pdf_title(self):
        for kind, title, wrong in (("source", "Native source document", "Native destination two"),
                                   ("destination", "Native destination two", "Native wrong first page")):
            with self.subTest(kind=kind):
                self.assertTrue(installed.plan_fixture_visible(kind, 501, title))
                self.assertTrue(installed.plan_fixture_visible(kind, 501, title.replace(" ", "\n")))
                for pixels, text in ((500, title), (5000, wrong), (5000, "Page 2"),
                                     (5000, "Preview destination"), (5000, title + "fold"),
                                     (5000, (title + "\n") * 2)):
                    self.assertFalse(installed.plan_fixture_visible(kind, pixels, text))

    def test_plan_navigation_refusal_requires_the_specific_observed_error(self):
        errors = {"changed-target": "Plan set: Source fingerprint changed; accept the revision explicitly before navigation",
                  "missing-target": "I/O error: No such file or directory (os error 2)",
                  "changed-source": "Plan links: source PDF changed since the preview; preview it again"}
        for kind, expected in errors.items():
            with self.subTest(kind=kind):
                self.assertTrue(installed.plan_error_visible(kind, expected))
                for other, text in errors.items():
                    if other != kind:
                        self.assertFalse(installed.plan_error_visible(kind, text))
                self.assertFalse(installed.plan_error_visible(kind, "Destination verified for this preview"))
                self.assertFalse(installed.plan_error_visible(kind, "Unable to open this document"))

    def test_plan_link_input_target_requires_a_complete_unambiguous_placeholder(self):
        header = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"
        rows = "".join(f"5\t1\t1\t1\t1\t{index + 1}\t{20 + 60 * index}\t30\t50\t14\t95\t{word}\n"
                       for index, word in enumerate("Paste a manifest path".split()))
        self.assertEqual(installed.theme_label_center(header + rows, "Paste a manifest path", (280, 80)), (135, 37))
        with self.assertRaises(installed.ThemeLabelMissing):
            installed.theme_label_center(header + rows, "Load manifest", (280, 80))
        with self.assertRaises(AssertionError):
            installed.theme_label_center(header + rows, "Paste a manifest path", (220, 80))

    def test_synthetic_plan_pdf_has_correct_offsets_streams_and_distinct_pages(self):
        pages = [b"0 0.6 0 rg\nBT /F1 28 Tf 72 700 Td (Native wrong first page) Tj ET\n",
                 b"1 0 0 rg\nBT /F1 28 Tf 72 700 Td (Native destination two) Tj ET\n"]
        data = plan_fixture.pdf(pages)
        xref = int(re.search(rb"startxref\n(\d+)\n%%EOF", data).group(1))
        self.assertEqual(data[xref:xref + 5], b"xref\n")
        entries = data[xref:].splitlines()[3:10]
        self.assertEqual(len(entries), 7)
        for index, entry in enumerate(entries, 1):
            offset = int(entry.split()[0])
            self.assertTrue(data[offset:].startswith(f"{index} 0 obj\n".encode()))
        self.assertIn(b"/Kids [4 0 R 6 0 R] /Count 2", data)
        for page in pages:
            self.assertIn(b"/Length " + str(len(page)).encode() + b" >>\nstream\n" + page + b"\nendstream", data)

    def test_split_reader_evidence_rejects_other_page_and_interface_text(self):
        for color, right, wrong in (("blue", "1", "2"), ("red", "2", "1")):
            with self.subTest(color=color):
                self.assertTrue(installed.reader_fixture_visible(color, 501, "SuniPDF page " + right))
                self.assertFalse(installed.reader_fixture_visible(color, 501, "SuniPDF page " + wrong))
                self.assertFalse(installed.reader_fixture_visible(color, 500, "SuniPDF page " + right))
                self.assertFalse(installed.reader_fixture_visible(color, 10000, "Page " + right))
                self.assertFalse(installed.reader_fixture_visible(color, 10000, "SuniPDF page " + right + "0"))
                self.assertFalse(installed.reader_fixture_visible(color, 10000, ("SuniPDF page " + right + "\n") * 2))

    def test_split_reader_masks_distinguish_page_ink(self):
        pixels = [(0, 0, 255), (255, 0, 0), (0, 0, 0), (255, 255, 255),
                  (120, 120, 255), (255, 120, 120), (190, 200, 220)]
        self.assertEqual(installed.colored_fixture_mask(pixels, "blue"), (bytes([0, 255, 255, 255, 0, 255, 255]), 2))
        self.assertEqual(installed.colored_fixture_mask(pixels, "red"), (bytes([255, 0, 255, 255, 255, 0, 255]), 2))

    def test_ocr_quotes_are_literal_and_cannot_swallow_later_control_rows(self):
        header = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"
        quote = '5\t1\t1\t1\t1\t1\t10\t10\t20\t14\t0\t"em\n'
        target = "5\t1\t2\t1\t1\t1\t74\t30\t90\t14\t95\ttransparency\n"
        self.assertEqual(installed.theme_label_center(header + quote + target, "transparency", (200, 80)), (119, 37))
        with self.assertRaises(installed.ThemeLabelMissing):
            installed.theme_label_center(header + quote, "transparency", (200, 80))
        with self.assertRaises(AssertionError) as ambiguous:
            installed.theme_label_center(header + target + target.replace("\t1\t2\t1\t1\t1\t", "\t1\t3\t1\t1\t1\t"), "transparency", (200, 80))
        self.assertNotIsInstance(ambiguous.exception, installed.ThemeLabelMissing)

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
