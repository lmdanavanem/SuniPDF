#!/usr/bin/env python3
"""Offline tests for native creation evidence helpers; no desktop qualification."""
import base64
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


spec = importlib.util.spec_from_file_location("native_creation", Path(__file__).with_name("native-plan-creation-fixture.py"))
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)


def qpdf_document(link=None):
    page = {"/Type": "/Page", "/Parent": "2 0 R", "/MediaBox": [0, 0, 600, 800],
            "/Resources": {"/Font": {"/F1": "3 0 R"}}, "/Contents": "5 0 R"}
    objects = {"obj:3 0 R": {"value": {"/Type": "/Font", "/Subtype": "/Type1", "/BaseFont": "/Helvetica"}},
               "obj:4 0 R": {"value": page},
               "obj:5 0 R": {"stream": {"dict": {"/Length": 17},
                                          "data": base64.b64encode(b"synthetic content").decode()}}}
    if link is not None:
        page["/Annots"] = ["8 0 R"]
        objects["obj:8 0 R"] = {"value": link}
    return {"pages": [{"object": "4 0 R", "contents": ["5 0 R"]}], "qpdf": [{"jsonversion": 2}, objects]}


def annotation(target, name="synthetic-link-alpha"):
    return {"/Subtype": "/Link", "/P": "4 0 R", "/NM": "u:" + name, "/Rect": [72, 590, 172, 617],
            "/A": {"/S": "/GoToR", "/D": [1, "/Fit"], "/NewWindow": True,
                    "/F": {"/Type": "/Filespec", "/F": "u:Target.pdf", "/UF": "u:Target.pdf"},
                    "/SuniPDFPlanTarget": "u:" + json.dumps(target)}}


def tsv(lines):
    result = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"
    for block, (y, words) in enumerate(lines, 1):
        x = 10
        for number, word in enumerate(words.split(), 1):
            result += f"5\t1\t{block}\t1\t1\t{number}\t{x}\t{y}\t{len(word) * 7}\t14\t95\t{word}\n"
            x += len(word) * 7 + 8
    return result


class NativeCreationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        sources = [self.root / "Alpha.pdf", self.root / "Beta.pdf"]
        outputs = [self.root / "Alpha-linked.pdf", self.root / "Beta-linked.pdf"]
        target, manifest = self.root / "Target.pdf", self.root / "plan-set.json"
        for path in [*sources, target, manifest, *outputs]:
            path.write_bytes(("offline helper fixture: " + path.name).encode())
        self.fixture = {"sources": sources, "outputs": outputs, "target": target, "manifest": manifest,
                        "sourceIds": ["source-alpha", "source-beta"], "sheetId": "target-two",
                        "protected": {str(path): native.sha(path) for path in [*sources, target, manifest]}}
        self.target = native.expected_target(self.fixture)

    def outputs(self):
        documents = {path: qpdf_document() for path in self.fixture["sources"]}
        documents.update({path: qpdf_document(annotation(self.target, "synthetic-link-" + str(index)))
                          for index, path in enumerate(self.fixture["outputs"])})
        calls = []

        def command(args):
            calls.append(args)
            self.assertEqual(args[0], "qpdf")
            return "no syntax errors" if args[1] == "--check" else json.dumps(documents[args[-1]])

        result = native.verify_outputs(self.fixture, self.root, command)
        self.assertEqual(len(calls), 8)
        return result

    def receipt(self, outputs):
        result = {"kind": "batch", "receipt": {"version": 1, "status": "complete", "failure": None,
                  "remaining": [], "last_input_validation": {"valid": True, "reason": None}, "completed": []}}
        for output in outputs:
            occurrence = {"id": "occurrence-" + output["sourceFileId"], "text": "A-101", "status": "ready",
                          "target_sheet_ids": [self.fixture["sheetId"]]}
            report = {"version": 1, "output_sha256": output["outputSha256"], "excluded_occurrences": [],
                      "inspection": {"manifest_sha256": native.sha(self.fixture["manifest"]),
                                     "source_sha256": output["sourceSha256"], "source_file_id": output["sourceFileId"],
                                     "requested_pages": [1], "scanned_pages": 1, "scan_complete": True,
                                     "truncated": False, "issues": [], "occurrences": [occurrence]},
                      "created": [{"occurrence_id": occurrence["id"], "page": 1, "annotation_index": 0,
                                   "name": output["annotationName"], "rect": output["rect"], "target": self.target}]}
            result["receipt"]["completed"].append({"output": output["output"], "bytes": output["bytes"], "report": report})
        return result

    def test_preparation_only_indexes_originals_and_never_creates_links(self):
        cli = self.root / "cli"
        cli.write_bytes(b"\x7fELF\x02\x01" + bytes(12) + b"\x3e\x00")
        calls = []

        def command(args):
            calls.append([str(item) for item in args])
            self.assertEqual(args[1], "plan-set")
            output = Path(args[-1])
            if args[2] == "create":
                document = {"files": [{"path": Path(path).name, "id": Path(path).stem} for path in args[3:6]],
                            "sheets": [{"file_id": "Target", "page": 2, "id": "sheet-two"}]}
            else:
                self.assertEqual(args[2], "edit-sheet")
                document = json.loads(Path(args[3]).read_text())
            output.write_text(json.dumps(document), encoding="utf-8")
            return "{}"

        fixture = native.prepare(self.root / "prepared", self.root, command, cli)
        self.assertEqual([call[2] for call in calls], ["create", "edit-sheet"])
        self.assertFalse(any(output.exists() for output in fixture["outputs"]))
        self.assertTrue(all(b"/Annots" not in source.read_bytes() and b"/GoToR" not in source.read_bytes()
                            for source in fixture["sources"]))
        report = json.loads((self.root / fixture["fixtureReport"]).read_text())
        self.assertFalse(report["desktopLinkSavingTested"])
        self.assertEqual(report["cliBinarySha256"], native.sha(cli))
        self.assertEqual(report["target"]["physicalPage"], 2)

    def test_ocr_scope_ignores_punctuation_but_not_ambiguous_rows(self):
        text = tsv([(20, "Alpha.pdf · 0 Approved links"), (50, "Beta.pdf · 0 Approved links"),
                    (80, "Alpha.pdf 1 Preview source Choose a destination Preview destination")])
        box = native.label_box(text, "Alpha.pdf", (900, 200), line_contains="Preview source")
        self.assertEqual(box[1], 80)
        self.assertEqual(native.label_box(text, "Beta.pdf", (900, 200), line_contains="0 Approved links")[1], 50)
        with self.assertRaisesRegex(AssertionError, "Ambiguous"):
            native.label_box(text, "Alpha.pdf", (900, 200))
        with self.assertRaises(native.LabelMissing):
            native.label_box(text, "Alpha.pdf", (900, 200), line_contains="1 Approved links")
        with self.assertRaises(AssertionError):
            native.label_box(text, "Beta.pdf", (20, 200))

    def test_ocr_scopes_adjacent_sparse_blocks_to_the_same_baseline_only(self):
        text = tsv([(20, "Alpha.pdf"), (20, "Preview source"), (50, "Beta.pdf"), (80, "Preview source")])
        self.assertEqual(native.label_box(text, "Alpha.pdf", (900, 200), line_contains="Preview source")[1], 20)
        with self.assertRaises(native.LabelMissing):
            native.label_box(text, "Beta.pdf", (900, 200), line_contains="Preview source")

    def test_ocr_expanded_destination_group_is_below_its_source(self):
        text = tsv([(20, "Destination sheets (1)"), (50, "Beta.pdf"), (80, "Destination sheets (1)")])
        with self.assertRaisesRegex(AssertionError, "Ambiguous"):
            native.label_box(text, "Destination sheets 1", (900, 200))
        self.assertEqual(native.label_box(text, "Destination sheets 1", (900, 200), below=64)[1], 80)

    def test_popup_choice_is_scoped_to_the_observed_combo_below_other_file_labels(self):
        text = tsv([(20, "Beta.pdf"), (80, "Beta.pdf")])
        with self.assertRaisesRegex(AssertionError, "Ambiguous"):
            native.label_box(text, "Beta.pdf", (900, 200), exact_line=True)
        self.assertEqual(native.label_box(text, "Beta.pdf", (900, 200), exact_line=True,
                                         within=(0, 60, 200, 120))[1], 80)

    def test_pdf_ink_requires_unique_correct_title_and_enough_coloured_pixels(self):
        for kind, title in (("alpha", "Native source alpha"), ("beta", "Native source beta"),
                            ("destination", "Native destination two")):
            self.assertTrue(native.fixture_visible(kind, 1001, title))
            for text, count in ((title, 1000), ("Preview destination", 9000), (title + " " + title, 9000),
                                (title + " Native wrong first page", 9000), ("Other PDF", 9000)):
                with self.subTest(kind=kind, text=text):
                    self.assertFalse(native.fixture_visible(kind, count, text))

    def test_qpdf_checks_content_resources_destination_and_actual_rectangle(self):
        original = native.pdf_snapshot(qpdf_document())
        saved = qpdf_document(annotation(self.target))
        verified = native.verify_link_page(original, native.pdf_snapshot(saved), self.target)
        self.assertEqual(verified["target"]["page"], 1)
        mutations = [lambda doc: doc["qpdf"][1]["obj:5 0 R"]["stream"].update(data=base64.b64encode(b"changed").decode()),
                     lambda doc: doc["qpdf"][1]["obj:3 0 R"]["value"].update({"/BaseFont": "/Courier"}),
                     lambda doc: doc["qpdf"][1]["obj:4 0 R"]["value"].update({"/MediaBox": [0, 0, 800, 600]}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"]["/A"].update({"/D": [0, "/Fit"]}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"]["/A"].update({"/JS": "u:script"}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"].update({"/Rect": [72, 500, 172, 527]}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"].update({"/Rect": [72, 590, float("nan"), 617]}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"]["/A"]["/F"].update({"/F": "u:Other.pdf"}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"].update({"/P": "6 0 R"}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"].update({"/AA": {}})]
        for mutate in mutations:
            changed = copy.deepcopy(saved)
            mutate(changed)
            with self.subTest(mutation=mutate), self.assertRaises(AssertionError):
                native.verify_link_page(original, native.pdf_snapshot(changed), self.target)

    def test_qpdf_rejects_old_links_and_stale_target_metadata(self):
        saved = native.pdf_snapshot(qpdf_document(annotation(self.target)))
        with self.assertRaisesRegex(AssertionError, "already contains"):
            native.verify_link_page(saved, saved, self.target)
        wrong = dict(self.target, sha256="0" * 64)
        with self.assertRaises(AssertionError):
            native.verify_link_page(native.pdf_snapshot(qpdf_document()), saved, wrong)

    def test_actual_files_and_native_receipt_are_bound_to_independent_objects(self):
        outputs = self.outputs()
        receipt = self.root / "receipt.json"
        receipt.write_text(json.dumps(self.receipt(outputs)), encoding="utf-8")
        result = native.verify_receipt(self.fixture, receipt, outputs)
        self.assertEqual(result["outputs"], 2)
        self.assertEqual(result["sha256"], native.sha(receipt))
        self.fixture["outputs"][1].write_bytes(b"replaced after saving")
        with self.assertRaises(AssertionError):
            native.verify_receipt(self.fixture, receipt, outputs)

    def test_receipt_rejects_missing_copy_wrong_hash_page_identity_or_incomplete_outcome(self):
        outputs = self.outputs()
        base = self.receipt(outputs)
        mutations = [lambda value: value["receipt"]["completed"].pop(),
                     lambda value: value["receipt"].update(status="failed"),
                     lambda value: value["receipt"].update(last_input_validation={"valid": False, "reason": "changed"}),
                     lambda value: value["receipt"]["completed"][0]["report"].update(output_sha256="0" * 64),
                     lambda value: value["receipt"]["completed"][0]["report"]["created"][0].update(page=2),
                     lambda value: value["receipt"]["completed"][0]["report"]["created"][0].update(name="another-name"),
                     lambda value: value["receipt"]["completed"][0]["report"]["inspection"].update(source_file_id="wrong"),
                     lambda value: value["receipt"]["completed"][0]["report"]["inspection"].update(manifest_sha256="0" * 64)]
        path = self.root / "receipt.json"
        for mutate in mutations:
            value = copy.deepcopy(base)
            mutate(value)
            path.write_text(json.dumps(value), encoding="utf-8")
            with self.subTest(mutation=mutate), self.assertRaises(AssertionError):
                native.verify_receipt(self.fixture, path, outputs)

    def test_changed_original_refuses_output_qualification(self):
        self.fixture["sources"][1].write_bytes(b"changed original")
        with self.assertRaisesRegex(AssertionError, "Original inputs changed"):
            native.verify_outputs(self.fixture, self.root, lambda args: self.fail("QPDF must not run for changed inputs"))


if __name__ == "__main__":
    unittest.main()
