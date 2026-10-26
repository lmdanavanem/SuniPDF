#!/usr/bin/env python3
"""Regression checks for Linux archive and executable package boundaries."""
import importlib.util
import ast
import gzip
import hashlib
import io
from pathlib import Path
import re
import struct
import tarfile
import tempfile
import unittest
from unittest.mock import Mock, patch


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
    def test_portal_activation_session_inherits_the_virtual_display_as_the_test_user(self):
        path = Path(__file__).with_name("test-deb.py")
        tree = ast.parse(path.read_text(encoding="utf-8"))
        lifecycle = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == "lifecycle")
        launches = [node for node in ast.walk(lifecycle) if isinstance(node, ast.Call)
                    and isinstance(node.func, ast.Name) and node.func.id == "command"
                    and any(isinstance(part, ast.Constant) and part.value == "dbus-run-session"
                            for part in ast.walk(node))]
        self.assertEqual(len(launches), 1)
        command = Mock()
        environment = ["env", "HOME=/home/sunipdf-test", "XDG_RUNTIME_DIR=/home/sunipdf-test/.runtime",
                       "GITHUB_SHA=" + "1" * 40]
        evidence = Path("offline-evidence")
        namespace = {"command": command, "ACCOUNT": "sunipdf-test", "environment": environment,
                     "__file__": str(path), "version": "0.91.0", "evidence": evidence}
        exec(compile(ast.Expression(launches[0]), str(path), "eval"), namespace)
        arguments = command.call_args.args[0]
        self.assertEqual(arguments[:4], ["runuser", "-u", "sunipdf-test", "--"])
        self.assertEqual(arguments[4:4 + len(environment)], environment)
        display = arguments.index("xvfb-run")
        bus = arguments.index("dbus-run-session")
        self.assertLess(display, bus, "Portal activation must inherit DISPLAY/XAUTHORITY from Xvfb")
        self.assertEqual(arguments[display:bus], ["xvfb-run", "-a", "-s", "-screen 0 1400x1000x24 -dpi 96"])
        self.assertEqual(arguments[bus:], ["dbus-run-session", "--", "python3", str(path), "--runtime",
                                          "--version", "0.91.0", "--evidence", evidence])
        self.assertEqual(command.call_args.kwargs, {"timeout": 1200})

    def test_plan_fixture_records_the_binary_separately_from_the_installed_launcher(self):
        with tempfile.TemporaryDirectory() as directory:
            launcher = Path(directory) / "sunipdf-cli"
            binary = Path(directory) / "payload"
            launcher.write_bytes(b'#!/bin/sh\nexec /usr/lib/sunipdf/sunipdf-cli "$@"\n')
            elf = b"\x7fELF\x02\x01" + bytes(12) + b"\x3e\x00" + b"fixture payload"
            binary.write_bytes(elf)
            with patch.object(plan_fixture, "INSTALLED_CLI", launcher), \
                    patch.object(plan_fixture, "INSTALLED_CLI_BINARY", binary):
                receipt = plan_fixture.cli_provenance(launcher)
                self.assertEqual(receipt["cliSha256"], hashlib.sha256(launcher.read_bytes()).hexdigest())
                self.assertEqual(receipt["cliBinarySha256"], hashlib.sha256(elf).hexdigest())
                self.assertNotEqual(receipt["cliSha256"], receipt["cliBinarySha256"])
                self.assertEqual(receipt["launcherPath"], str(launcher))
                self.assertEqual(receipt["binaryPath"], str(binary))
                binary.write_bytes(elf + b"changed")
                changed = plan_fixture.cli_provenance(launcher)
                self.assertEqual(changed["cliSha256"], receipt["cliSha256"])
                self.assertNotEqual(changed["cliBinarySha256"], receipt["cliBinarySha256"])
            direct = plan_fixture.cli_provenance(binary)
            self.assertEqual(direct["cliSha256"], direct["cliBinarySha256"])
            self.assertEqual(direct["launcherPath"], direct["binaryPath"])

    def test_plan_fixture_does_not_infer_installed_binary_for_an_arbitrary_cli(self):
        with tempfile.TemporaryDirectory() as directory:
            cli = Path(directory) / "fake-cli"
            for content in (b"fake CLI", b'#!/bin/sh\nexec /usr/lib/sunipdf/sunipdf-cli "$@"\n'):
                with self.subTest(content=content):
                    cli.write_bytes(content)
                    receipt = plan_fixture.cli_provenance(cli)
                    self.assertEqual(receipt["cliSha256"], hashlib.sha256(content).hexdigest())
                    self.assertIsNone(receipt["cliBinarySha256"])
                    self.assertIsNone(receipt["binaryPath"])

    def test_plan_fixture_rejects_changed_launcher_and_missing_or_non_elf_payload(self):
        with tempfile.TemporaryDirectory() as directory:
            launcher = Path(directory) / "sunipdf-cli"
            binary = Path(directory) / "payload"
            with patch.object(plan_fixture, "INSTALLED_CLI", launcher), \
                    patch.object(plan_fixture, "INSTALLED_CLI_BINARY", binary):
                launcher.write_bytes(b'#!/bin/sh\nexec /tmp/other-cli "$@"\n')
                with self.assertRaisesRegex(ValueError, "launcher"):
                    plan_fixture.cli_provenance(launcher)
                launcher.write_bytes(b'#!/bin/sh\nexec /usr/lib/sunipdf/sunipdf-cli "$@"\n')
                with self.assertRaisesRegex(ValueError, "regular ELF"):
                    plan_fixture.cli_provenance(launcher)
                for content in (b"#!/bin/sh\nexit 0\n", b"\x7fELF", b"MZ" + bytes(18),
                                b"\x7fELF\x02\x01" + bytes(12) + b"\xb7\x00"):
                    with self.subTest(content=content):
                        binary.write_bytes(content)
                        with self.assertRaisesRegex(ValueError, "ELF64 amd64"):
                            plan_fixture.cli_provenance(launcher)

    def test_source_refusal_allows_only_the_observed_pdf_ocr_confusion(self):
        for token in ("PDF", "POF", "pdf", "pof"):
            text = f"engine errar: Plan links: source {token} changed since the preview, preview ik again"
            self.assertTrue(installed.plan_error_is_visible("changed-source", 500, text))
            self.assertFalse(installed.plan_error_is_visible("changed-target", 500, text))
            self.assertFalse(installed.plan_error_is_visible("missing-target", 500, text))
            self.assertFalse(installed.plan_error_is_visible("changed-source", 0, text))
        for text in ("source PXF changed since the preview", "source P0F changed since the preview",
                     "destination POF changed since the preview", "source POF unchanged since the preview",
                     "source POF changed before the preview", "source fingerprint changed",
                     "source POF not found", "source POF changed since the previewing"):
            with self.subTest(text=text):
                self.assertFalse(installed.plan_error_is_visible("changed-source", 500, text))

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

    def test_plan_source_location_uses_both_unique_pdf_anchors_and_actual_image_scale(self):
        header = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"
        title = "".join(f"5\t1\t1\t1\t1\t{index + 1}\t{30 + 90 * index}\t60\t60\t30\t95\t{word}\n"
                        for index, word in enumerate("Native source document".split()))
        reference = "5\t1\t2\t1\t1\t1\t30\t360\t90\t30\t95\tA-101\n"
        self.assertEqual(installed.plan_source_location(header + title + reference, (900, 1200), 3),
                         {"Native source document": [50, 25], "A-101": [25, 125]})
        for incomplete in (title, reference, title + reference.replace("A-101", "A-102")):
            with self.subTest(text=incomplete), self.assertRaises(installed.ThemeLabelMissing):
                installed.plan_source_location(header + incomplete, (900, 1200), 3)
        duplicate = reference.replace("\t1\t2\t1\t1\t1\t", "\t1\t3\t1\t1\t1\t")
        with self.assertRaises(AssertionError):
            installed.plan_source_location(header + title + reference + duplicate, (900, 1200), 3)

    def test_plan_source_return_rejects_different_scroll_zoom_or_missing_anchor(self):
        before = {"Native source document": [300, 250], "A-101": [250, 450]}
        self.assertTrue(installed.plan_location_restored(before, before))
        self.assertTrue(installed.plan_location_restored(before, {"Native source document": [303, 247], "A-101": [249, 452]}))
        for after in ({"Native source document": [300, 260], "A-101": [250, 460]},
                      {"Native source document": [300, 250], "A-101": [250, 470]},
                      {"Native source document": [300, 250]},
                      {"Native source document": [300, 250], "A-101": [250]},
                      {"Native source document": [300, 250], "A-101": [float("nan"), 450]}):
            with self.subTest(after=after):
                self.assertFalse(installed.plan_location_restored(before, after))

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
