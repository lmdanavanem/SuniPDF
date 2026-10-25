"""Offline oracle mutation tests; these synthetic graphs never qualify a product binary."""
import base64
import cmath
import copy
import importlib.util
import json
import math
from pathlib import Path
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location("ellipse_corpus", Path(__file__).with_name("test-dimensioned-ellipses.py"))
CORPUS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CORPUS)


def fixture_graph():
    objects = {"obj:3 0 R": {"value": {"/Type": "/Font", "/Subtype": "/Type1", "/BaseFont": "/Helvetica"}}}
    pages = []
    for index, rotation in enumerate(CORPUS.ROTATIONS):
        page, content = f"{4 + index * 2} 0 R", f"{5 + index * 2} 0 R"
        pages.append({"object": page, "contents": [content]})
        objects["obj:" + page] = {"value": {"/Type": "/Page", "/Parent": "2 0 R", "/MediaBox": [0, 0, 1000, 800],
            "/CropBox": list(CORPUS.CROP), "/Rotate": rotation,
            "/Resources": {"/Font": {"/F1": "3 0 R"}}, "/Contents": content}}
        objects["obj:" + content] = {"stream": {"dict": {"/Length": len(CORPUS.page_content(index))},
            "data": base64.b64encode(CORPUS.page_content(index)).decode()}}
    return {"pages": pages, "qpdf": [{"jsonversion": 2}, objects]}


def calibrated_graph():
    value = fixture_graph()
    page = value["qpdf"][1]["obj:4 0 R"]["value"]
    # Deliberately synthetic immutable strings: only the real CLI generates runtime provenance.
    page["/SuniPDFPageCalibration"] = "u:offline-page-scale"
    page["/SuniPDFMeasurementViewports"] = "u:offline-region-scale"
    return value


def shape_graph(case, baseline=None):
    graph = copy.deepcopy(baseline or calibrated_graph())
    rotation = CORPUS.ROTATIONS[case["page"] - 1]
    refs, known, factor = CORPUS.scale_reference(case)
    # An independent complex-plane parametrization creates these offline test fixtures.
    orientation = cmath.exp(-1j * math.radians(case["angle"] % 360))
    displayed = []
    for index in range(128):
        unit = cmath.exp(index * math.tau * 1j / 128)
        p = complex(unit.real * case["x"] * factor / 2, unit.imag * case["y"] * factor / 2) * orientation
        displayed.append((CORPUS.f32(p.real + CORPUS.CENTER[0]), CORPUS.f32(p.imag + CORPUS.CENTER[1])))
    points = [CORPUS.user_point(p, rotation) for p in displayed]
    displayed = [CORPUS.display_point(p, rotation) for p in points]
    metadata = {"version": 2, "coordinate_system": "pdf-user-space", "kind": "area", "points": points,
                "calibration": [CORPUS.user_point(p, rotation) for p in refs], "known_distance": known,
                "unit": case["unit"], "precision": 4}
    value = CORPUS.polygon_area(displayed) * (known / math.dist(*refs)) ** 2
    label = f"{value:.4f} {case['unit']}2"
    width = len(label) * 6.2 + 8
    left = min(p[0] for p in displayed) - 6
    bottom = min(p[1] for p in displayed) - 22
    right = max(max(p[0] for p in displayed) + 6, left + width)
    top = max(p[1] for p in displayed) + 6
    corners = [CORPUS.user_point(p, rotation) for p in ((left, bottom), (right, bottom), (right, top), (left, top))]
    rect = [min(p[0] for p in corners), min(p[1] for p in corners), max(p[0] for p in corners), max(p[1] for p in corners)]
    drawing = "q /GS gs 0.05 0.25 0.75 RG 1.25 w 1 J 1 j\n"
    for index, (x, y) in enumerate(displayed):
        drawing += f"{x - left:.4f} {y - bottom:.4f} {'m' if index == 0 else 'l'}\n"
    drawing += f"h\nS\n1 1 1 rg 2 1 {width - 2:.4f} 14 re f\n"
    drawing += f"0.05 0.25 0.75 rg BT /Helv 10 Tf 4 4 Td ({label}) Tj ET Q\n"
    matrix = {0: [1, 0, 0, 1, 0, 0], 90: [0, 1, -1, 0, 0, 0],
              180: [-1, 0, 0, -1, 0, 0], 270: [0, -1, 1, 0, 0, 0]}[rotation]
    page = graph["pages"][case["page"] - 1]["object"]
    graph["qpdf"][1]["obj:" + page]["value"]["/Annots"] = ["30 0 R"]
    graph["qpdf"][1]["obj:30 0 R"] = {"value": {
        "/Type": "/Annot", "/P": page, "/Subtype": "/Stamp", "/Name": "/SuniPDFMeasurement",
        "/Rect": rect, "/NM": "u:SuniPDF-" + case["name"], "/T": "u:Ellipse qualification",
        "/SuniPDFMeasurement": "u:" + json.dumps(metadata), "/Contents": "u:" + label,
        "/AP": {"/N": "31 0 R"}}}
    graph["qpdf"][1]["obj:31 0 R"] = {"stream": {"dict": {
        "/Type": "/XObject", "/Subtype": "/Form", "/Matrix": matrix,
        "/BBox": [0, 0, right - left, top - bottom], "/Resources": {
            "/ExtGState": {"/GS": {"/Type": "/ExtGState", "/CA": 1, "/ca": 1}},
            "/Font": {"/Helv": {"/Type": "/Font", "/Subtype": "/Type1",
                      "/BaseFont": "/Helvetica", "/Encoding": "/WinAnsiEncoding"}}}},
        "data": base64.b64encode(drawing.encode()).decode()}}
    measurement = {"kind": "area", "unit": case["unit"] + "2", "value": value, "version": 2,
                   "validity": "current", "points": displayed, "holes": [], "depth": None, "label": label}
    result = {"page": case["page"], "index": 0, "shape": case["shape"], "segments": 128,
              "approximation": "polygonal_area", "persistent_shape_constraint": False, "original_unchanged": True,
              "encrypted": False, "export_unavailable": False, "rotation_degrees": case["angle"] % 360,
              "measurement": measurement}
    table = {"summary": {"quantity_rows": 1}, "rows": [{"measurement_validity": "current",
              "quantity": {"unit": case["unit"] + "2", "value": value}}]}
    return graph, result, table


def verify(case, graph, result, table):
    checks = []
    def check(name, passed):
        checks.append(name)
        if not passed:
            raise AssertionError(name)
    proof = CORPUS.verify_saved(check, case, CORPUS.qpdf_pages(calibrated_graph()),
                               CORPUS.qpdf_pages(graph), result, table)
    return proof, checks


class OfflineTests(unittest.TestCase):
    def test_all_eight_cropped_rotation_and_unit_oracles(self):
        for case in CORPUS.cases():
            with self.subTest(case=case["name"]):
                proof, checks = verify(case, *shape_graph(case))
                self.assertEqual(len(checks), 22)
                self.assertEqual(proof["vertices"], 128)
                self.assertGreater(proof["polygon_value"], 0)

    def test_vertices_shape_count_and_rotation_tampering(self):
        case = CORPUS.cases()[1]
        for mutation in ("vertex", "count", "rotate", "unit", "calibration", "analytic-value"):
            graph, result, table = shape_graph(case)
            annotation = graph["qpdf"][1]["obj:30 0 R"]["value"]
            metadata = json.loads(annotation["/SuniPDFMeasurement"][2:])
            if mutation == "vertex":
                metadata["points"][17][0] += .1
            elif mutation == "count":
                metadata["points"].pop()
            elif mutation == "rotate":
                metadata["points"] = metadata["points"][1:] + metadata["points"][:1]
            elif mutation == "unit":
                metadata["unit"] = "m"
            elif mutation == "calibration":
                metadata["known_distance"] *= 1.001
            else:
                result["measurement"]["value"] = math.pi * case["x"] * case["y"] / 4
                table["rows"][0]["quantity"]["value"] = result["measurement"]["value"]
            annotation["/SuniPDFMeasurement"] = "u:" + json.dumps(metadata)
            with self.subTest(mutation=mutation), self.assertRaises((AssertionError, ValueError)):
                verify(case, graph, result, table)

    def test_actual_appearance_tampering_cannot_hide_behind_metadata(self):
        case = CORPUS.cases()[3]
        for mutation in ("disconnected", "matrix", "bbox", "rectangle", "hidden-stroke", "font", "encoding", "extra-operator", "label"):
            graph, result, table = shape_graph(case)
            annotation = graph["qpdf"][1]["obj:30 0 R"]["value"]
            appearance = graph["qpdf"][1]["obj:31 0 R"]["stream"]
            drawing = base64.b64decode(appearance["data"]).decode()
            if mutation == "disconnected":
                drawing = drawing.replace(" l\n", " m\n", 1)
            elif mutation == "matrix":
                appearance["dict"]["/Matrix"][0] = 2
            elif mutation == "bbox":
                appearance["dict"]["/BBox"][2] += 1
            elif mutation == "rectangle":
                annotation["/Rect"][0] += 1
            elif mutation == "hidden-stroke":
                appearance["dict"]["/Resources"]["/ExtGState"]["/GS"]["/CA"] = 0
            elif mutation == "font":
                appearance["dict"]["/Resources"]["/Font"]["/Helv"]["/BaseFont"] = "/Courier"
            elif mutation == "encoding":
                appearance["dict"]["/Resources"]["/Font"]["/Helv"]["/Encoding"] = {"/Differences": [49, "/nine"]}
            elif mutation == "extra-operator":
                drawing = "0 0 0 0 0 0 cm\n" + drawing
            else:
                drawing = drawing.replace(" ft2)", " cm2)")
            appearance["data"] = base64.b64encode(drawing.encode()).decode()
            with self.subTest(mutation=mutation), self.assertRaises((AssertionError, ValueError)):
                verify(case, graph, result, table)

    def test_wrong_page_owner_changed_source_and_quantity(self):
        case = CORPUS.cases()[0]
        for mutation in ("owner", "content", "resources", "crop", "extra-annotation", "quantity", "stale"):
            graph, result, table = shape_graph(case)
            objects = graph["qpdf"][1]
            if mutation == "owner":
                objects["obj:30 0 R"]["value"]["/P"] = "6 0 R"
            elif mutation == "content":
                objects["obj:5 0 R"]["stream"]["data"] = base64.b64encode(b"q Q\n").decode()
            elif mutation == "resources":
                objects["obj:3 0 R"]["value"]["/BaseFont"] = "/Times-Roman"
            elif mutation == "crop":
                objects["obj:4 0 R"]["value"]["/CropBox"][0] += 1
            elif mutation == "extra-annotation":
                objects["obj:6 0 R"]["value"]["/Annots"] = ["30 0 R"]
            elif mutation == "quantity":
                table["rows"][0]["quantity"]["value"] += 1
            else:
                table["rows"][0]["measurement_validity"] = "stale"
            with self.subTest(mutation=mutation), self.assertRaises((AssertionError, ValueError)):
                verify(case, graph, result, table)

    def test_strict_json_and_bounded_graph(self):
        for value in ('{"a":1,"a":2}', '{"a":NaN}', '{"a":Infinity}'):
            with self.assertRaises(ValueError):
                CORPUS.strict_json(value)
        graph = fixture_graph()
        graph["qpdf"][1]["obj:3 0 R"]["value"]["/recursive"] = "3 0 R"
        with self.assertRaisesRegex(ValueError, "recursive"):
            CORPUS.qpdf_pages(graph)


class SimulatedCorpus(CORPUS.Corpus):
    """Exercises orchestration only. Never launches a process or writes outside a temporary fixture."""
    def __init__(self, directory):
        directory.mkdir()
        cli, qpdf = directory.parent / "offline-cli", directory.parent / "offline-qpdf"
        cli.write_bytes(b"not an executable")
        qpdf.write_bytes(b"also not an executable")
        super().__init__(cli, qpdf, directory, "0.90.0", "0" * 40, 600)
        self.graphs = {}
        self.tables = {}

    def execute(self, name, arguments, tool=None, reject=False, parse=True, parser_refusal=False):
        command = [str(tool or self.cli), *map(str, arguments)]
        self.report["commands"].append({"name": name, "argv": command, "expected_refusal": reject,
                                        "exit_code": 2 if parser_refusal else int(reject)})
        answer = b""
        if reject:
            pass
        elif name == "version":
            answer = b"sunipdf-cli 0.90.0\n"
        elif name == "qpdf-version":
            answer = b"qpdf version offline-simulation\n"
        elif name in ("save-page-scale", "save-region"):
            Path(arguments[-1]).write_bytes(b"offline generated fixture " + name.encode())
            answer = {"out": str(arguments[-1])}
        elif name == "uncompress-source":
            Path(arguments[-1]).write_bytes(b"offline qdf Original ellipse fixture page 1")
        elif name.endswith("-check"):
            pass
        elif name.endswith("-quantity"):
            answer = self.tables[name.removesuffix("-quantity")]
        elif name.endswith("-json"):
            label = name.removesuffix("-json")
            if label == "source":
                answer = fixture_graph()
            elif label == "qualified":
                answer = calibrated_graph()
            elif label == "stale":
                answer = calibrated_graph()
                answer["qpdf"][1]["obj:5 0 R"]["stream"]["data"] = base64.b64encode(b"Modified content").decode()
            else:
                answer = self.graphs[label]
        else:
            case = next(c for c in CORPUS.cases() if c["name"] == name)
            graph, answer, table = shape_graph(case)
            self.graphs[name], self.tables[name] = graph, table
            output = Path(arguments[arguments.index("--out") + 1])
            output.write_bytes(b"offline saved polygon " + name.encode())
            answer["out"] = str(output)
        for suffix in ("stdout", "stderr"):
            data = answer if isinstance(answer, bytes) else json.dumps(answer).encode()
            self.write(name + "." + suffix, data if suffix == "stdout" else b"", "offline_simulated_transcript")
        self.check(name + ": bounded expected process outcome", True)
        if reject:
            self.check(name + ": explicit refusal diagnostic", True)
        return answer


class OrchestrationTests(unittest.TestCase):
    def test_exact_plan_counts_without_running_any_product(self):
        with tempfile.TemporaryDirectory(prefix="dimensioned-ellipse-offline-") as temporary:
            run = SimulatedCorpus(Path(temporary).resolve() / "evidence")
            CORPUS.run_corpus(run)
            run.finish()
            self.assertEqual(run.report["counts"], {key: CORPUS.PLANNED[key]
                             for key in ("checks", "commands", "refusals", "cases", "outputs", "inventory_files")})
            self.assertTrue(all(item["unchanged"] for item in run.report["protected_integrity"]))


if __name__ == "__main__":
    unittest.main()
