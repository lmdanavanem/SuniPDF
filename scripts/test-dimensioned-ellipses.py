"""Qualify dimensioned polygons through a packaged CLI and independent decoded PDF geometry."""
import argparse
import base64
import copy
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
import struct
import subprocess
import time


SEGMENTS = 128
ROTATIONS = (0, 90, 180, 270)
CROP = (100.25, 50.5, 900.25, 650.5)
CENTER = (300.125, 300.375)
METRES = {"mm": .001, "cm": .01, "m": 1.0, "in": .0254, "ft": .3048}
PROCESS_SECONDS = 60
MAX_TRANSCRIPT_BYTES = 4 * 1024 * 1024
MAX_ARTIFACT_BYTES = 16 * 1024 * 1024
MAX_TOTAL_BYTES = 96 * 1024 * 1024
SCOPE = ("Original synthetic cropped PDFs, packaged CLI and independent QPDF decoding. "
         "Eight saved 128-segment polygons, their actual appearance paths and reopened quantities. "
         "No analytic ellipse area substitution, native desktop, external reader, physical survey or PDF/A qualification.")
# The same command plan is used on every platform; these totals are checked before success.
PLANNED = {"commands": 62, "refusals": 19, "cases": 27, "outputs": 8,
           "vertices": 1024, "rotations": 4, "units": 5, "checks": 327, "inventory_files": 140}


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def strict_json(data):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate JSON key: " + key)
            result[key] = value
        return result
    def constant(value):
        raise ValueError("non-finite JSON: " + value)
    return json.loads(data, object_pairs_hook=pairs, parse_constant=constant)


def pdf_bytes(objects):
    data, offsets = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n"), []
    for index, body in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f"{index} 0 obj\n".encode() + body + b"\nendobj\n")
    xref = len(data)
    data.extend(f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode())
    for offset in offsets:
        data.extend(f"{offset:010} 00000 n \n".encode())
    data.extend(f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    return bytes(data)


def page_content(index):
    return f"BT /F1 16 Tf 140 600 Td (Original ellipse fixture page {index + 1}) Tj ET\n".encode()


def fixture_pdf():
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>", b"",
               b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"]
    kids = []
    for index, rotation in enumerate(ROTATIONS):
        page = len(objects) + 1
        kids.append(f"{page} 0 R")
        objects.append((f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1000 800] "
                        f"/CropBox [{' '.join(map(str, CROP))}] /Rotate {rotation} "
                        f"/Resources << /Font << /F1 3 0 R >> >> /Contents {page + 1} 0 R >>").encode())
        content = page_content(index)
        objects.append(b"<< /Length " + str(len(content)).encode() + b" >>\nstream\n" + content + b"endstream")
    objects[1] = f"<< /Type /Pages /Kids [{' '.join(kids)}] /Count 4 >>".encode()
    return pdf_bytes(objects)


def user_point(point, rotation):
    x, y = map(f32, point)
    left, bottom, right, top = CROP
    return tuple(map(f32, {0: (left + x, bottom + y), 90: (right - y, bottom + x),
                          180: (right - x, top - y), 270: (left + y, top - x)}[rotation]))


def display_point(point, rotation):
    x, y = map(f32, point)
    left, bottom, right, top = CROP
    return tuple(map(f32, {0: (x - left, y - bottom), 90: (y - bottom, right - x),
                          180: (right - x, top - y), 270: (top - y, x - left)}[rotation]))


def polygon_area(points):
    x0, y0 = points[0]
    return abs(math.fsum((points[i][0] - x0) * (points[i + 1][1] - y0)
                        - (points[i][1] - y0) * (points[i + 1][0] - x0)
                        for i in range(1, len(points) - 1))) / 2


def cases():
    # Full diameters are literal physical inputs, independently parsed expectations follow.
    return [
        {"name": "ellipse-r0", "shape": "ellipse", "page": 1, "unit": "m", "dx": "4", "dy": "6",
         "x": 4, "y": 6, "angle": 0, "scale": "page"},
        {"name": "ellipse-r90", "shape": "ellipse", "page": 2, "unit": "cm", "dx": "400", "dy": "600",
         "x": 400, "y": 600, "angle": 33.5, "scale": "local"},
        {"name": "ellipse-r180", "shape": "ellipse", "page": 3, "unit": "mm", "dx": "4000", "dy": "6000",
         "x": 4000, "y": 6000, "angle": -27, "scale": "local"},
        {"name": "ellipse-r270", "shape": "ellipse", "page": 4, "unit": "ft", "dx": "6' 6\"", "dy": "4'",
         "x": 6.5, "y": 4, "angle": 90, "scale": "local"},
        {"name": "circle-r0", "shape": "circle", "page": 1, "unit": "m", "dx": "4", "dy": "4",
         "x": 4, "y": 4, "angle": 90, "scale": "region"},
        {"name": "circle-r90", "shape": "circle", "page": 2, "unit": "in", "dx": "78 3/4 in", "dy": "78 3/4 in",
         "x": 78.75, "y": 78.75, "angle": 360, "scale": "local"},
        {"name": "circle-r180", "shape": "circle", "page": 3, "unit": "m", "dx": "4", "dy": "4",
         "x": 4, "y": 4, "angle": -90, "scale": "local"},
        {"name": "circle-r270", "shape": "circle", "page": 4, "unit": "cm", "dx": "400", "dy": "400",
         "x": 400, "y": 400, "angle": 393.5, "scale": "local"},
    ]


def scale_reference(case):
    points = ((220, 220), (270, 220)) if case["scale"] == "region" else ((50, 50), (150, 50))
    known = 10 / METRES[case["unit"]]
    return points, known, math.dist(*points) / known


def expected_points(case):
    _, _, points_per_unit = scale_reference(case)
    angle = math.radians(case["angle"] % 360)
    result = []
    for index in range(SEGMENTS):
        theta = math.tau * index / SEGMENTS
        x = case["x"] * points_per_unit * math.cos(theta) / 2
        y = case["y"] * points_per_unit * math.sin(theta) / 2
        result.append((CENTER[0] + x * math.cos(angle) + y * math.sin(angle),
                       CENTER[1] - x * math.sin(angle) + y * math.cos(angle)))
    return result


def qpdf_pages(value):
    if len(value["pages"]) != 4:
        raise ValueError("expected exactly four actual PDF pages")
    objects = value["qpdf"][1]
    if len(objects) > 4096:
        raise ValueError("excessive PDF graph")
    def resolve(item, active=()):
        if isinstance(item, str) and re.fullmatch(r"\d+ \d+ R", item):
            if item in active or len(active) >= 32:
                raise ValueError("recursive PDF graph")
            entry = objects["obj:" + item]
            if "stream" in entry:
                stream = entry["stream"]
                data = base64.b64decode(stream["data"], validate=True)
                if len(data) > MAX_TRANSCRIPT_BYTES:
                    raise ValueError("excessive decoded PDF stream")
                return {"dictionary": resolve({key: val for key, val in stream["dict"].items()
                        if key not in ("/Length", "/Filter", "/DecodeParms")}, active + (item,)),
                        "decoded": data.hex()}
            return resolve(entry["value"], active + (item,))
        if isinstance(item, dict):
            return {key: resolve(val, active) for key, val in item.items()}
        if isinstance(item, list):
            return [resolve(val, active) for val in item]
        return item
    pages = []
    for page in value["pages"]:
        attrs = objects["obj:" + page["object"]]["value"]
        annotations = attrs.get("/Annots", [])
        if isinstance(annotations, str):
            annotations = objects["obj:" + annotations]["value"]
        normalized = []
        for entry in annotations:
            annotation = copy.deepcopy(objects["obj:" + entry]["value"] if isinstance(entry, str) else entry)
            if annotation.pop("/P", None) != page["object"]:
                raise ValueError("annotation does not reference its owning PDF page")
            normalized.append(resolve(annotation))
        pages.append({"attributes": resolve({key: val for key, val in attrs.items()
                      if key not in ("/Parent", "/Annots", "/Contents")}),
                      "contents": [resolve(ref) for ref in page["contents"]], "annotations": normalized})
    return pages


def pdf_text(value):
    if isinstance(value, str) and value.startswith("u:"):
        return value[2:]
    if isinstance(value, str) and value.startswith("b:"):
        data = bytes.fromhex(value[2:])
        return data[2:].decode("utf-16-be") if data.startswith(b"\xfe\xff") else data.decode("ascii")
    raise ValueError("expected a PDF text string")


def appearance_points(annotation):
    appearance = annotation["/AP"]["/N"]
    dictionary = appearance["dictionary"]
    drawing = bytes.fromhex(appearance["decoded"]).decode("ascii")
    number = r"[-+]?(?:\d+(?:\.\d*)?|\.\d+)"
    pattern = (r"q /GS gs 0\.05 0\.25 0\.75 RG 1\.25 w 1 J 1 j\n"
               + rf"((?:{number} {number} [ml]\n){{128}})h\nS\n"
               + rf"1 1 1 rg 2 1 {number} 14 re f\n"
               + r"0\.05 0\.25 0\.75 rg BT /Helv 10 Tf 4 4 Td \(([^()\\]*)\) Tj ET Q\n")
    match = re.fullmatch(pattern, drawing)
    if match is None:
        raise ValueError("appearance is not the complete expected 128-edge stroked polygon and label")
    rows = [line.split() for line in match[1].splitlines()]
    if [row[2] for row in rows] != ["m"] + ["l"] * 127:
        raise ValueError("appearance contour is disconnected")
    drawn = [(float(row[0]), float(row[1])) for row in rows]
    if dictionary["/Subtype"] != "/Form":
        raise ValueError("appearance is not a PDF Form")
    a, b, c, d, e, f = dictionary["/Matrix"]
    transform = lambda p: (a * p[0] + c * p[1] + e, b * p[0] + d * p[1] + f)
    bx0, by0, bx1, by1 = dictionary["/BBox"]
    if not (bx1 > bx0 and by1 > by0):
        raise ValueError("invalid appearance BBox")
    corners = [transform(p) for p in ((bx0, by0), (bx1, by0), (bx1, by1), (bx0, by1))]
    minimum = [min(p[axis] for p in corners) for axis in (0, 1)]
    maximum = [max(p[axis] for p in corners) for axis in (0, 1)]
    x0, y0, x1, y1 = annotation["/Rect"]
    if not (all(math.isfinite(v) for v in [a, b, c, d, e, f, *minimum, *maximum, x0, y0, x1, y1])
            and maximum[0] > minimum[0] and maximum[1] > minimum[1] and x1 > x0 and y1 > y0):
        raise ValueError("invalid mapped appearance bounds")
    mapped = [((p[0] - minimum[0]) * (x1 - x0) / (maximum[0] - minimum[0]) + x0,
               (p[1] - minimum[1]) * (y1 - y0) / (maximum[1] - minimum[1]) + y0)
              for p in map(transform, drawn)]
    return mapped, match[2], dictionary


def close_points(actual, expected, tolerance=.002):
    return (len(actual) == len(expected) and all(len(p) == len(q) == 2
            and all(math.isfinite(a) and abs(a - b) <= tolerance for a, b in zip(p, q))
            for p, q in zip(actual, expected)))


def verify_saved(check, case, original, saved, result, table):
    name, page_index, unit = case["name"], case["page"] - 1, case["unit"]
    rotation = ROTATIONS[page_index]
    check(name + ": all four source pages retained", len(original) == len(saved) == 4)
    for index, (old, new) in enumerate(zip(original, saved)):
        check(name + f": page {index + 1} exact content resources and geometry",
              new["contents"] == old["contents"] and new["attributes"] == old["attributes"])
        check(name + f": page {index + 1} annotation isolation",
              len(new["annotations"]) == (1 if index == page_index else 0) and not old["annotations"])
    annotation = saved[page_index]["annotations"][0]
    check(name + ": ordinary measurement with fresh identity", annotation["/Subtype"] == "/Stamp"
          and annotation["/Name"] == "/SuniPDFMeasurement" and pdf_text(annotation["/NM"]).startswith("SuniPDF-")
          and pdf_text(annotation["/T"]) == "Ellipse qualification")
    metadata = strict_json(pdf_text(annotation["/SuniPDFMeasurement"]))
    points = metadata["points"]
    check(name + ": actual version2 calibrated polygon", metadata["version"] == 2
          and metadata["coordinate_system"] == "pdf-user-space" and metadata["kind"] == "area"
          and metadata["unit"] == unit and metadata["precision"] == 4 and len(points) == SEGMENTS
          and not metadata.get("holes") and metadata.get("depth") is None)
    expected = expected_points(case)
    displayed = [display_point(p, rotation) for p in points]
    check(name + ": every actual vertex matches independently requested diameters and clockwise angle",
          close_points(points, [user_point(p, rotation) for p in expected], .001)
          and close_points(displayed, expected, .001) and len(set(map(tuple, points))) == SEGMENTS)
    references, known, points_per_unit = scale_reference(case)
    calibration = [display_point(p, rotation) for p in metadata["calibration"]]
    check(name + ": real saved calibration and mixed-unit conversion",
          close_points(calibration, references, 0) and math.isclose(metadata["known_distance"], known, rel_tol=1e-12))
    check(name + ": both physical diameters", math.isclose(math.dist(displayed[0], displayed[64]), case["x"] * points_per_unit, abs_tol=.001)
          and math.isclose(math.dist(displayed[32], displayed[96]), case["y"] * points_per_unit, abs_tol=.001))
    area_points = polygon_area(displayed)
    polygon_value = area_points * (metadata["known_distance"] / math.dist(*calibration)) ** 2
    regular_value = SEGMENTS / 8 * case["x"] * case["y"] * math.sin(math.tau / SEGMENTS)
    analytic_value = math.pi * case["x"] * case["y"] / 4
    check(name + ": independent sampled area rather than analytic ellipse",
          math.isclose(polygon_value, regular_value, rel_tol=2.1e-5)
          and abs(polygon_value - analytic_value) > analytic_value * .0003)
    measurement = result["measurement"]
    check(name + ": exact explicit shape receipt", result["page"] == case["page"] and result["index"] == 0
          and result["shape"] == case["shape"] and result["segments"] == SEGMENTS
          and result["approximation"] == "polygonal_area" and result["persistent_shape_constraint"] is False
          and result["original_unchanged"] is True and result["encrypted"] is False
          and result["export_unavailable"] is False and result["rotation_degrees"] == case["angle"] % 360)
    check(name + ": CLI result matches actual saved polygon", measurement["kind"] == "area"
          and measurement["unit"] == unit + "2" and measurement["version"] == 2
          and measurement["validity"] == "current" and close_points(measurement["points"], displayed, .0001)
          and math.isclose(measurement["value"], polygon_value, rel_tol=1e-9)
          and measurement["holes"] == [] and measurement["depth"] is None)
    rows = table["rows"]
    check(name + ": reopened quantity includes only this saved polygon",
          table["summary"]["quantity_rows"] == len(rows) == 1 and rows[0]["measurement_validity"] == "current"
          and rows[0]["quantity"]["unit"] == unit + "2"
          and math.isclose(rows[0]["quantity"]["value"], polygon_value, rel_tol=1e-9))
    mapped, label, appearance = appearance_points(annotation)
    check(name + ": real complete appearance matches all saved vertices", close_points(mapped, points, .002))
    matrix = {0: [1, 0, 0, 1, 0, 0], 90: [0, 1, -1, 0, 0, 0],
              180: [-1, 0, 0, -1, 0, 0], 270: [0, -1, 1, 0, 0, 0]}[rotation]
    check(name + ": appearance axes match page rotation", appearance["/Matrix"] == matrix)
    check(name + ": visible label matches measured polygon", label == pdf_text(annotation["/Contents"])
          == measurement["label"] and label == f"{polygon_value:.4f} {unit}2")
    resources = appearance["/Resources"]
    check(name + ": visible opaque stroke and standard label resources",
          resources == {"/ExtGState": {"/GS": {"/Type": "/ExtGState", "/CA": 1, "/ca": 1}},
                        "/Font": {"/Helv": {"/Type": "/Font", "/Subtype": "/Type1",
                                  "/BaseFont": "/Helvetica", "/Encoding": "/WinAnsiEncoding"}}})
    return {"area_points_squared": area_points, "polygon_value": polygon_value, "unit": unit + "2",
            "annotation_identity": pdf_text(annotation["/NM"]), "vertices": len(points)}


class Corpus:
    def __init__(self, cli, qpdf, directory, version, source, seconds):
        self.cli, self.qpdf, self.directory = cli, qpdf, directory
        self.started, self.seconds = time.monotonic(), seconds
        self.roles, self.protected = {}, {}
        self.report = {"schema": "sunipdf-dimensioned-ellipse-qualification", "version": 1, "status": "running",
            "source_revision": source.lower(), "expected_cli_version": version, "cli_sha256": sha(cli),
            "qpdf_sha256": sha(qpdf), "script_sha256": sha(Path(__file__)), "scope": SCOPE,
            "started_utc": datetime.now(timezone.utc).isoformat(), "fixture_license": "CC0-1.0",
            "native_desktop_tested": False, "planned_coverage": PLANNED,
            "limits": {"total_seconds": seconds, "process_seconds": PROCESS_SECONDS,
                       "transcript_bytes_per_stream": MAX_TRANSCRIPT_BYTES, "artifact_bytes": MAX_ARTIFACT_BYTES,
                       "total_artifact_bytes": MAX_TOTAL_BYTES},
            "checks": [], "commands": [], "cases": [], "sources": [], "outputs": [],
            "inventory_excludes": ["report.json"]}

    def check(self, name, passed):
        self.report["checks"].append({"name": name, "passed": bool(passed)})
        if not passed:
            raise AssertionError(name)

    def record(self, path, role):
        if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_ARTIFACT_BYTES:
            raise ValueError("missing, linked or excessive artifact: " + str(path))
        name = path.relative_to(self.directory).as_posix()
        if name in self.roles:
            raise ValueError("artifact already retained: " + name)
        self.roles[name], self.protected[path] = role, sha(path)
        if sum(p.stat().st_size for p in self.protected) > MAX_TOTAL_BYTES:
            raise ValueError("total artifact bound exceeded")
        return path

    def write(self, name, value, role):
        path = self.directory / name
        with path.open("xb") as stream:
            stream.write(value if isinstance(value, bytes) else (json.dumps(value, indent=2) + "\n").encode())
        return self.record(path, role)

    def preserved(self, label):
        self.check(label, all(path.is_file() and not path.is_symlink() and sha(path) == digest
                             for path, digest in self.protected.items()))

    def execute(self, name, arguments, tool=None, reject=False, parse=True, parser_refusal=False):
        command = [str(tool or self.cli), *map(str, arguments)]
        remaining = self.started + self.seconds - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("dimensioned ellipse corpus exceeded total deadline")
        entry = {"name": name, "argv": command, "expected_refusal": reject,
                 "parser_refusal": parser_refusal, "started_utc": datetime.now(timezone.utc).isoformat(), "exit_code": None}
        self.report["commands"].append(entry)
        out, err = (self.directory / f"{name}.{suffix}" for suffix in ("stdout", "stderr"))
        begin, process = time.monotonic(), None
        try:
            with out.open("xb") as stdout, err.open("xb") as stderr:
                process = subprocess.Popen(command, cwd=self.directory, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
                try:
                    while process.poll() is None:
                        if time.monotonic() - begin > min(PROCESS_SECONDS, remaining):
                            raise TimeoutError(name + ": process deadline exceeded")
                        if max(out.stat().st_size, err.stat().st_size) > MAX_TRANSCRIPT_BYTES:
                            raise ValueError(name + ": transcript bound exceeded")
                        time.sleep(.05)
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait(timeout=5)
                    entry["exit_code"] = process.returncode
        finally:
            entry["elapsed_seconds"] = round(time.monotonic() - begin, 3)
            entry["transcripts"] = []
            for path in (out, err):
                if path.exists():
                    self.record(path, "command_transcript")
                    entry["transcripts"].append({"name": path.name, "bytes": path.stat().st_size, "sha256": sha(path)})
        self.check(name + ": bounded expected process outcome", max(out.stat().st_size, err.stat().st_size) <= MAX_TRANSCRIPT_BYTES
                   and (process.returncode != 0 if reject else process.returncode == 0))
        if reject:
            stderr = err.read_bytes()
            # Clap rejects argument syntax before the engine can return structured JSON.
            diagnostic = None if parser_refusal else strict_json(stderr)
            valid = bool(stderr) and (b"error:" in stderr if parser_refusal else
                                      isinstance(diagnostic, dict) and isinstance(diagnostic.get("error"), str)
                                      and bool(diagnostic["error"]))
            self.check(name + ": explicit refusal diagnostic", not out.read_bytes() and valid)
            return None
        return strict_json(out.read_bytes()) if parse else out.read_bytes()

    def inspect(self, name, path):
        self.execute(name + "-check", ["--check", path], tool=self.qpdf, parse=False)
        return qpdf_pages(self.execute(name + "-json", ["--json", "--json-stream-data=inline", path], tool=self.qpdf))

    def case(self, name, kind, action):
        start = len(self.report["commands"])
        entry = {"name": name, "kind": kind, "passed": False}
        self.report["cases"].append(entry)
        try:
            value = action()
            entry["passed"] = True
            return value
        finally:
            entry["commands"] = [item["name"] for item in self.report["commands"][start:]]

    def finish(self):
        self.report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        self.report["counts"] = {"checks": len(self.report["checks"]), "commands": len(self.report["commands"]),
            "refusals": sum(item["expected_refusal"] for item in self.report["commands"]),
            "cases": len(self.report["cases"]), "outputs": len(self.report["outputs"])}
        self.report["protected_integrity"] = [{"name": path.relative_to(self.directory).as_posix(), "sha256": digest,
            "unchanged": path.is_file() and sha(path) == digest} for path, digest in self.protected.items()]
        self.report["inventory"] = [{"name": name, "role": role, "sha256": sha(self.directory / name),
            "bytes": (self.directory / name).stat().st_size} for name, role in sorted(self.roles.items())]
        self.report["counts"]["inventory_files"] = len(self.report["inventory"])
        with (self.directory / "report.json").open("x", encoding="utf-8") as stream:
            json.dump(self.report, stream, indent=2, allow_nan=False)
            stream.write("\n")


def shape_arguments(source, out, case):
    result = ["dimensioned-" + case["shape"], source, "--center", ",".join(map(str, CENTER)),
              "--page", case["page"], "--unit", case["unit"], "--precision", "4", "--rotation-degrees", case["angle"],
              "--author", "Ellipse qualification", "--out", out]
    result += ["--diameter", case["dx"]] if case["shape"] == "circle" else ["--diameter-x", case["dx"], "--diameter-y", case["dy"]]
    result += {"local": ["--calibration", "50,50 150,50", "--known-distance", "1000", "--calibration-unit", "cm"],
               "page": ["--use-page-calibration"], "region": ["--region", "detail"],
               "automatic": ["--automatic-scale"]}[case["scale"]]
    return result


def run_corpus(run):
    version = run.execute("version", ["--version"], parse=False).decode().strip()
    run.report["cli_version"] = version
    run.check("exact CLI version", version == "sunipdf-cli " + run.report["expected_cli_version"])
    run.report["qpdf_version"] = run.execute("qpdf-version", ["--version"], tool=run.qpdf, parse=False).decode().strip()
    source = run.write("source.pdf", fixture_pdf(), "synthetic_source")
    original = run.inspect("source", source)
    for index, page in enumerate(original):
        attrs = page["attributes"]
        run.check(f"fixture page {index + 1}: independent content crop rotation and resources",
                  attrs["/CropBox"] == list(CROP) and attrs["/MediaBox"] == [0, 0, 1000, 800]
                  and attrs["/Rotate"] == ROTATIONS[index] and not page["annotations"]
                  and b"".join(bytes.fromhex(c["decoded"]) for c in page["contents"]) == page_content(index)
                  and attrs["/Resources"] == {"/Font": {"/F1": {"/Type": "/Font", "/Subtype": "/Type1", "/BaseFont": "/Helvetica"}}})
    calibrated = run.directory / "page-scale.pdf"
    run.execute("save-page-scale", ["set-page-calibration", source, "--page", "1", "--calibration", "50,50 150,50",
                "--known-distance", "10", "--unit", "m", "--out", calibrated])
    run.record(calibrated, "calibrated_source")
    expected = run.write("expected-regions.json", [], "authored_region_expectation")
    regions = run.write("regions.json", [{"id": "detail", "name": "Detail", "bounds": [200, 200, 400, 400],
                "calibration": {"points": [[220, 220], [270, 220]], "known_distance": 10, "unit": "m"}}], "authored_regions")
    qualified = run.directory / "qualified-source.pdf"
    run.execute("save-region", ["set-measurement-viewports", calibrated, "--expected", expected,
                              "--regions", regions, "--out", qualified])
    run.record(qualified, "calibrated_source")
    baseline = run.inspect("qualified", qualified)
    for index, (old, new) in enumerate(zip(original, baseline)):
        attrs = {k: v for k, v in new["attributes"].items() if k not in ("/SuniPDFPageCalibration", "/SuniPDFMeasurementViewports")}
        run.check(f"calibration preserves original page {index + 1}", attrs == old["attributes"]
                  and new["contents"] == old["contents"] and not new["annotations"])
    run.check("both persisted scales are present on page1", all(key in baseline[0]["attributes"]
              for key in ("/SuniPDFPageCalibration", "/SuniPDFMeasurementViewports")))
    for path in (source, calibrated, qualified):
        run.report["sources"].append({"name": path.name, "sha256": sha(path)})
    for case in cases():
        def qualify(case=case):
            name = case["name"]
            output = run.directory / (name + ".pdf")
            result = run.execute(name, shape_arguments(qualified, output, case))
            run.record(output, "saved_dimensioned_polygon")
            run.check(name + ": receipt binds actual new output", Path(result["out"]).resolve(strict=True) == output)
            saved = run.inspect(name, output)
            table = run.execute(name + "-quantity", ["quantity-table", output])
            proof = verify_saved(run.check, case, baseline, saved, result, table)
            run.preserved(name + ": every retained source and earlier output unchanged")
            run.report["outputs"].append({**case, **proof, "name": output.name, "sha256": sha(output)})
        run.case(case["name"], "saved_polygon", qualify)
    run.check("all eight annotation identities are fresh", len({o["annotation_identity"] for o in run.report["outputs"]}) == 8)

    qdf = run.directory / "qualified-uncompressed.pdf"
    run.execute("uncompress-source", ["--qdf", "--normalize-content=n", "--object-streams=disable",
                                     "--stream-data=uncompress", qualified, qdf],
                tool=run.qpdf, parse=False)
    run.record(qdf, "stale_fixture_basis")
    raw, needle = qdf.read_bytes(), b"Original ellipse fixture page 1"
    run.check("stale fixture changes one actual content marker with equal byte length", raw.count(needle) == 1)
    stale = run.write("stale-source.pdf", raw.replace(needle, b"Modified ellipse fixture page 1"), "stale_calibrated_source")
    stale_pages = run.inspect("stale", stale)
    run.check("stale source preserves scales resources and other pages", all(a["attributes"] == b["attributes"]
              and (a["contents"] != b["contents"] if i == 0 else a["contents"] == b["contents"])
              for i, (a, b) in enumerate(zip(baseline, stale_pages))))
    run.report["sources"].append({"name": stale.name, "sha256": sha(stale)})
    base = {**cases()[0], "scale": "local"}
    failures = [
        ("zero-diameter", {"dx": "0"}, qualified, [], False),
        ("negative-diameter", {"dx": "-1"}, qualified, [], False),
        ("nonfinite-diameter", {"dx": "NaN"}, qualified, [], False),
        ("tiny-polygon", {"dx": ".001", "dy": ".001"}, qualified, [], False),
        ("outside-page", {"dx": "1000"}, qualified, [], False),
        ("nonfinite-angle", {"angle": "NaN"}, qualified, [], True),
        ("infinite-angle", {"angle": "inf"}, qualified, [], True),
        ("negative-infinite-angle", {"angle": "-inf"}, qualified, [], True),
        ("missing-page-scale", {"scale": "page"}, source, [], False),
        ("missing-region", {"scale": "region"}, source, [], False),
        ("missing-automatic-scale", {"scale": "automatic"}, source, [], False),
        ("crossing-region", {"scale": "region", "dx": "60"}, qualified, [], False),
        ("crossing-automatic-region", {"scale": "automatic", "dx": "60"}, qualified, [], False),
        ("stale-page-scale", {"scale": "page"}, stale, [], False),
        ("stale-region", {"scale": "region"}, stale, [], False),
        ("stale-automatic-scale", {"scale": "automatic"}, stale, [], False),
    ]
    sentinel = run.write("existing.pdf", b"Existing output must remain unchanged.\n", "no_overwrite_sentinel")
    failures.extend([
        ("existing-output", {}, qualified, ["--overwrite"], False),
        ("source-as-output", {}, qualified, ["--overwrite"], False),
        ("degenerate-reference", {}, qualified, [], False),
    ])
    for name, changes, input_pdf, extra, parser_refusal in failures:
        def refuse(name=name, changes=changes, input_pdf=input_pdf, extra=extra, parser_refusal=parser_refusal):
            out = {"existing-output": sentinel, "source-as-output": qualified}.get(name, run.directory / (name + ".pdf"))
            existing = out.exists()
            command = shape_arguments(input_pdf, out, {**base, **changes})
            if name == "degenerate-reference":
                command[command.index("--calibration") + 1] = "50,50 50,50"
            run.execute("refuse-" + name, [*command, *extra], reject=True, parser_refusal=parser_refusal)
            run.check(name + ": no new output", out.exists() == existing)
            run.preserved(name + ": all retained originals and outputs unchanged")
        run.case(name, "refusal", refuse)
    run.preserved("final original and output integrity")
    run.check("CLI and QPDF binaries unchanged", sha(run.cli) == run.report["cli_sha256"]
              and sha(run.qpdf) == run.report["qpdf_sha256"])
    run.check("complete planned coverage", len(run.report["commands"]) == PLANNED["commands"]
              and sum(c["expected_refusal"] for c in run.report["commands"]) == PLANNED["refusals"]
              and len(run.report["cases"]) == PLANNED["cases"] and len(run.report["outputs"]) == PLANNED["outputs"]
              and len(run.report["checks"]) + 1 == PLANNED["checks"] and len(run.roles) == PLANNED["inventory_files"]
              and {path.name for path in run.directory.iterdir()} == set(run.roles))
    run.report["status"] = "passed"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--qpdf", type=Path, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--expected-version", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--deadline-seconds", type=int, default=600)
    args = parser.parse_args()
    if not re.fullmatch(r"\d+\.\d+\.\d+", args.expected_version) or not re.fullmatch(r"[a-fA-F0-9]{40}", args.source_sha):
        parser.error("an explicit semantic version and full source commit are required")
    if not 60 <= args.deadline_seconds <= 1200:
        parser.error("deadline must be between 60 and 1200 seconds")
    cli, qpdf = args.cli.resolve(strict=True), args.qpdf.resolve(strict=True)
    if not cli.is_file() or not qpdf.is_file() or cli == qpdf:
        parser.error("distinct real CLI and QPDF files are required")
    directory = args.directory.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    run = Corpus(cli, qpdf, directory, args.expected_version, args.source_sha, args.deadline_seconds)
    try:
        run_corpus(run)
    except Exception as error:
        run.report["status"], run.report["error"] = "failed", str(error)
        raise
    finally:
        run.finish()
    print(json.dumps({"status": run.report["status"], **run.report["counts"], "report": str(directory / "report.json")}))


if __name__ == "__main__":
    main()
