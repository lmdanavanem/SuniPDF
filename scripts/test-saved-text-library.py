"""Qualify saved native text library capture with packaged CLI, decoded PDF graphs and pixels."""
import argparse
import base64
import copy
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import time


PROCESS_SECONDS = 60
MAX_TRANSCRIPT_BYTES = 4 * 1024 * 1024
MAX_ARTIFACT_BYTES = 16 * 1024 * 1024
ROTATIONS = (0, 90, 180, 270)
CROP = (100.25, 50.5, 900.25, 850.5)
ORIGIN = (40, 50)
CONTENT = b"0 0 0 rg BT /F1 16 Tf 140 800 Td (Original saved text fixture) Tj ET\n"
SCOPE = ("Original synthetic PDFs and 12 standard font variants on four rotated cropped pages; "
         "packaged CLI capture and reuse, independently decoded PDF objects and exact rendered RGBA. "
         "No native desktop interaction, external-reader rendering, arbitrary fonts or PDF/A claim.")


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def pdf_bytes(objects):
    data = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for index, body in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f"{index} 0 obj\n".encode() + body + b"\nendobj\n")
    xref = len(data)
    data.extend(f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode())
    for offset in offsets:
        data.extend(f"{offset:010} 00000 n \n".encode())
    data.extend(f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    return bytes(data)


def fixture_pdf(rotation):
    if rotation not in ROTATIONS:
        raise ValueError("unsupported fixture rotation")
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Count 1 /Kids [4 0 R] >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        ("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1000 1000] "
         f"/CropBox [{' '.join(map(str, CROP))}] /Rotate {rotation} "
         "/Resources << /Font << /F1 3 0 R >> >> /Contents 5 0 R >>").encode(),
        b"<< /Length " + str(len(CONTENT)).encode() + b" >>\nstream\n" + CONTENT + b"endstream"]
    return pdf_bytes(objects)


def primitives():
    result = []
    for family in ("helvetica", "times", "courier"):
        for bold, italic in ((False, False), (True, False), (False, True), (True, True)):
            index = len(result)
            x, y = (index % 2) * 360, (index // 2) * 100
            result.append({"kind": "free_text", "rect": [x, y, x + 340, y + 80],
                "text": f"Résumé café à vérifier — {family}\nSecond line with several words that wrap inside the text box.",
                "style": {"family": family, "size_pt": 12, "bold": bold, "italic": italic,
                          "rgb": [30, 70, 150], "opacity": 0.65,
                          "fill_rgb": [240, 230, 210] if index % 2 == 0 else None,
                          "border_pt": 1.25 if index % 2 == 0 else 0}})
    result.extend([
        {"kind": "shape", "shape": "square", "rect": [0, 610, 80, 650],
         "style": {"rgb": [200, 30, 40], "fill_rgb": None, "width_pt": 1.5,
                   "opacity": 0.8, "dash_pattern": []}},
        {"kind": "stamp", "stamp": "approved", "rect": [100, 610, 260, 650]}])
    return result


def library():
    return {"format": "sunipdf-annotation-library", "version": 1, "entries": [{
        "id": "native-text", "name": "Standard text styles",
        "provenance": {"author": "Fixture creator", "source": "Original synthetic artwork", "license": "CC0-1.0"},
        "primitives": primitives()}]}


def user_rect(rect, rotation):
    left, bottom, right, top = CROP
    def point(x, y):
        x, y = x + ORIGIN[0], y + ORIGIN[1]
        return {0: (left + x, bottom + y), 90: (right - y, bottom + x),
                180: (right - x, top - y), 270: (left + y, top - x)}[rotation]
    a, b = point(*rect[:2]), point(*rect[2:])
    return [min(a[0], b[0]), min(a[1], b[1]), max(a[0], b[0]), max(a[1], b[1])]


def base_font(style):
    return {"helvetica": ("Helvetica", "Helvetica-Bold", "Helvetica-Oblique", "Helvetica-BoldOblique"),
            "times": ("Times-Roman", "Times-Bold", "Times-Italic", "Times-BoldItalic"),
            "courier": ("Courier", "Courier-Bold", "Courier-Oblique", "Courier-BoldOblique")
            }[style["family"]][int(style["bold"]) + 2 * int(style["italic"])]


def qpdf_page(value):
    if len(value["pages"]) != 1:
        raise ValueError("expected exactly one PDF page")
    objects = value["qpdf"][1]
    def resolve(item, active=()):
        if isinstance(item, str) and re.fullmatch(r"\d+ \d+ R", item):
            if item in active or len(active) >= 32:
                raise ValueError("recursive PDF graph")
            entry = objects["obj:" + item]
            if "stream" in entry:
                stream = entry["stream"]
                return {"dictionary": resolve({key: val for key, val in stream["dict"].items()
                        if key not in ("/Length", "/Filter", "/DecodeParms")}, active + (item,)),
                        "decoded": base64.b64decode(stream["data"], validate=True).hex()}
            return resolve(entry["value"], active + (item,))
        if isinstance(item, dict):
            return {key: resolve(val, active) for key, val in item.items()}
        if isinstance(item, list):
            return [resolve(val, active) for val in item]
        if isinstance(item, str) and item.startswith("b:"):
            raw = bytes.fromhex(item[2:])
            try:
                return "u:" + (raw[2:].decode("utf-16-be") if raw.startswith(b"\xfe\xff") else raw.decode("utf-8"))
            except UnicodeDecodeError:
                return item
        return item
    page = value["pages"][0]
    attrs = objects["obj:" + page["object"]]["value"]
    entries = attrs.get("/Annots", [])
    if isinstance(entries, str):
        entries = objects["obj:" + entries]["value"]
    annotations = []
    for entry in entries:
        annotation = copy.deepcopy(objects["obj:" + entry]["value"] if isinstance(entry, str) else entry)
        if annotation.pop("/P", None) != page["object"]:
            raise ValueError("annotation must reference its actual owning page")
        annotations.append(resolve(annotation))
    return {"attributes": resolve({key: val for key, val in attrs.items() if key not in ("/Parent", "/Annots", "/Contents")}),
            "contents": [resolve(ref) for ref in page["contents"]], "annotations": annotations}


def native_da_matches(value, font_name, style):
    if not isinstance(value, str) or not value.startswith("u:"):
        return False
    parts = value[2:].split(" ")
    if len(parts) != 7 or parts[0] != font_name or parts[2] != "Tf" or parts[6] != "rg":
        return False
    try:
        return (f32(float(parts[1])) == f32(style["size_pt"])
                and [f32(float(value)) for value in parts[3:6]] == [f32(value / 255) for value in style["rgb"]])
    except (ValueError, OverflowError):
        return False


def has_embedded_font(value):
    if isinstance(value, dict):
        return any(key in ("/FontFile", "/FontFile2", "/FontFile3") or has_embedded_font(item)
                   for key, item in value.items())
    if isinstance(value, list):
        return any(has_embedded_font(item) for item in value)
    return False


def verify_pdf(check, name, source, reused, rotation):
    expected = primitives()
    check(name + ": page content and resources preserved", source["attributes"] == reused["attributes"]
          and source["contents"] == reused["contents"])
    check(name + ": original content stream preserved", b"".join(bytes.fromhex(item["decoded"]) for item in source["contents"]) == CONTENT)
    attrs = source["attributes"]
    check(name + ": independent crop and rotation", attrs["/CropBox"] == list(CROP)
          and attrs["/MediaBox"] == [0, 0, 1000, 1000] and attrs.get("/Rotate", 0) == rotation)
    check(name + ": original page font preserved", attrs["/Resources"] == {"/Font": {"/F1": {
          "/Type": "/Font", "/Subtype": "/Type1", "/BaseFont": "/Helvetica"}}})
    before, after = source["annotations"], reused["annotations"]
    check(name + ": complete mixed annotation group", len(before) == len(after) == 14)
    old_names = [item["/NM"] for item in before]
    new_names = [item["/NM"] for item in after]
    check(name + ": independent fresh identities", len(set(old_names)) == len(set(new_names)) == 14
          and set(old_names).isdisjoint(new_names)
          and all(value.startswith("u:SuniPDF-") for value in old_names + new_names))
    identity = {"/NM", "/T", "/M", "/CreationDate"}
    matrix = {0: [1, 0, 0, 1, 0, 0], 90: [0, 1, -1, 0, 0, 0],
              180: [-1, 0, 0, -1, 0, 0], 270: [0, -1, 1, 0, 0, 0]}[rotation]
    for index, (old, new, primitive) in enumerate(zip(before, after, expected)):
        label = f"{name}: annotation {index}"
        check(label + " exact appearance and metadata", {key: value for key, value in old.items() if key not in identity}
              == {key: value for key, value in new.items() if key not in identity})
        check(label + " selected author and geometry", old.get("/T") == "u:Source author"
              and new.get("/T") == "u:Reuse author" and old["/Rect"] == user_rect(primitive["rect"], rotation))
        if primitive["kind"] != "free_text":
            check(label + " preserved non-text primitive", old["/Subtype"] == {"shape": "/Square", "stamp": "/Stamp"}[primitive["kind"]])
            continue
        style = primitive["style"]
        ap = old["/AP"]["/N"]
        fonts = ap["dictionary"]["/Resources"]["/Font"]
        check(label + " actual standard font and text", old["/Subtype"] == "/FreeText"
              and old["/SuniPDFRich"] == "/FreeText" and old["/Contents"] == "u:" + primitive["text"]
              and len(fonts) == 1 and next(iter(fonts.values()))["/BaseFont"] == "/" + base_font(style)
              and next(iter(fonts.values()))["/Subtype"] == "/Type1" and not has_embedded_font(fonts))
        check(label + " actual appearance bounds and transform", ap["dictionary"]["/BBox"] == [0, 0, 340, 80]
              and ap["dictionary"]["/Matrix"] == matrix
              and all(re.search(rb"\b" + operator + rb"\b", bytes.fromhex(ap["decoded"]))
                      for operator in (b"BT", b"ET", b"Tf"))
              and re.search(rb"\b(?:Tj|TJ)\b", bytes.fromhex(ap["decoded"])))
        check(label + " actual style and native DA", f32(old["/CA"]) == f32(style["opacity"])
              and old["/BS"]["/W"] == style["border_pt"]
              and (old.get("/C") is None if style["fill_rgb"] is None else
                   [f32(value) for value in old["/C"]] == [f32(value / 255) for value in style["fill_rgb"]])
              and native_da_matches(old["/DA"], next(iter(fonts)), style))


def image_pixels(path):
    from PIL import Image
    with Image.open(path) as image:
        image.load()
        if image.format != "PNG" or image.size != (800, 800) or image.mode != "RGBA":
            raise ValueError("expected actual 800 by 800 RGBA PNG")
        return image.tobytes()


class Corpus:
    def __init__(self, cli, qpdf, directory, version, source, seconds):
        self.cli, self.qpdf, self.directory = cli, qpdf, directory
        self.started, self.seconds = time.monotonic(), seconds
        self.roles, self.protected = {}, {}
        self.report = {"schema": "sunipdf-saved-text-library-qualification", "version": 1, "status": "running",
            "source_revision": source.lower(), "expected_cli_version": version, "cli_sha256": sha(cli),
            "qpdf_sha256": sha(qpdf), "script_sha256": sha(Path(__file__)), "scope": SCOPE,
            "started_utc": datetime.now(timezone.utc).isoformat(), "fixture_license": "CC0-1.0",
            "native_desktop_tested": False, "planned_coverage": {"commands": 45, "refusals": 4, "cases": 8,
                "rotations": 4, "font_variants": 12, "text_annotations": 48, "png_files": 8,
                "checks": 388, "inventory_files": 115},
            "limits": {"total_seconds": seconds, "process_seconds": PROCESS_SECONDS,
                       "transcript_bytes_per_stream": MAX_TRANSCRIPT_BYTES, "artifact_bytes": MAX_ARTIFACT_BYTES},
            "checks": [], "commands": [], "cases": [], "sources": [], "outputs": [], "images": [],
            "inventory_excludes": ["report.json"]}

    def check(self, name, passed):
        self.report["checks"].append({"name": name, "passed": bool(passed)})
        if not passed:
            raise AssertionError(name)

    def record(self, path, role):
        if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_ARTIFACT_BYTES:
            raise ValueError("missing, linked or excessive artifact: " + str(path))
        self.roles[path.relative_to(self.directory).as_posix()] = role
        self.protected[path] = sha(path)
        return path

    def write(self, name, value, role):
        path = self.directory / name
        with path.open("xb") as stream:
            stream.write(value if isinstance(value, bytes) else (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode())
        return self.record(path, role)

    def preserved(self, label):
        self.check(label, all(path.is_file() and sha(path) == digest for path, digest in self.protected.items()))

    def execute(self, name, arguments, tool=None, reject=False, parse=True):
        command = [str(tool or self.cli), *map(str, arguments)]
        remaining = self.started + self.seconds - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("saved text corpus exceeded total deadline")
        entry = {"name": name, "argv": command, "expected_refusal": reject,
                 "started_utc": datetime.now(timezone.utc).isoformat(), "exit_code": None}
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
                        time.sleep(0.05)
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
            self.check(name + ": structured refusal", not out.read_bytes() and isinstance(json.loads(err.read_bytes()), dict))
            return None
        return json.loads(out.read_bytes()) if parse else out.read_bytes()

    def inspect(self, name, path):
        self.execute(name + "-check", ["--check", path], tool=self.qpdf, parse=False)
        return qpdf_page(self.execute(name + "-json", ["--json", "--json-stream-data=inline", path], tool=self.qpdf))

    def case(self, name, kind, action):
        start = len(self.report["commands"])
        record = {"name": name, "kind": kind, "passed": False}
        self.report["cases"].append(record)
        try:
            value = action()
            record["passed"] = True
            return value
        finally:
            record["commands"] = [item["name"] for item in self.report["commands"][start:]]

    def finish(self):
        self.report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        self.report["counts"] = {"checks": len(self.report["checks"]), "commands": len(self.report["commands"]),
            "refusals": sum(item["expected_refusal"] for item in self.report["commands"]),
            "cases": len(self.report["cases"]), "images": len(self.report["images"])}
        self.report["protected_integrity"] = [{"name": path.relative_to(self.directory).as_posix(), "sha256": digest,
            "unchanged": path.is_file() and sha(path) == digest} for path, digest in self.protected.items()]
        self.report["inventory"] = [{"name": name, "role": role, "sha256": sha(self.directory / name),
            "bytes": (self.directory / name).stat().st_size} for name, role in sorted(self.roles.items())]
        self.report["counts"]["inventory_files"] = len(self.report["inventory"])
        with (self.directory / "report.json").open("x", encoding="utf-8") as stream:
            json.dump(self.report, stream, indent=2, ensure_ascii=False)
            stream.write("\n")


def run_corpus(run):
    version = run.execute("version", ["--version"], parse=False).decode().strip()
    run.report["cli_version"] = version
    run.check("exact CLI version", version == "sunipdf-cli " + run.report["expected_cli_version"])
    authored = run.write("authored-library.json", library(), "authored_library")
    results = []
    for rotation in ROTATIONS:
        def qualify(rotation=rotation):
            label = f"rotation-{rotation}"
            blank = run.write(label + "-blank.pdf", fixture_pdf(rotation), "original_pdf")
            source, reused = (run.directory / f"{label}-{kind}.pdf" for kind in ("source", "reused"))
            placement = ["--id", "native-text", "--file", blank, "--page", 1, "--origin", "40,50", "--scale", 1]
            placed = run.execute(label + "-place", ["annotation-library", "place", authored, *placement, "--author", "Source author", "--out", source])
            run.record(source, "saved_source_pdf")
            run.check(label + ": original placement receipt", Path(placed["out"]).resolve() == source.resolve()
                      and placed["count"] == 14 and placed["indices"] == list(range(14)) and placed["original_unchanged"] is True)
            candidates = run.execute(label + "-candidates", ["annotation-library", "capture-candidates", source, "--page", 1])
            rows = candidates["annotations"]
            run.check(label + ": explicit complete selectable inventory", candidates["page"] == 1 and candidates["index_base"] == 0
                      and len(rows) == 14 and [row["index"] for row in rows] == list(range(14))
                      and all(re.fullmatch(r"[a-f0-9]{64}", row.get("fingerprint") or "") and row.get("refusal") is None for row in rows)
                      and [row["subtype"] for row in rows] == ["FreeText"] * 12 + ["Square", "Stamp"])
            selections = [part for row in rows for part in ["--annotation", f"{row['index']}:{row['fingerprint']}"]]
            captured = run.directory / (label + "-captured.json")
            capture = ["annotation-library", "capture", source, "--page", 1, "--id", "native-text", "--name", "Captured standard text",
                       "--source", "Declared synthetic source", "--credit", "Declared creator", "--license", "CC0-1.0"]
            receipt = run.execute(label + "-capture", [*capture, *selections, "--out", captured])
            run.record(captured, "captured_library")
            run.check(label + ": capture receipt", Path(receipt["out"]).resolve() == captured.resolve() and receipt["count"] == 14
                      and receipt["entry_id"] == "native-text" and receipt["page"] == 1 and receipt["original_unchanged"] is True)
            value = json.loads(captured.read_bytes())
            entry = value["entries"][0]
            run.check(label + ": exact portable text styles and geometry", value["format"] == "sunipdf-annotation-library"
                      and value["version"] == 1 and len(value["entries"]) == 1 and entry["primitives"] == primitives())
            run.check(label + ": explicit provenance only", entry["provenance"] == {"author": "Declared creator", "source": "Declared synthetic source", "license": "CC0-1.0"}
                      and str(source) not in captured.read_text(encoding="utf-8") and "Source author" not in captured.read_text(encoding="utf-8"))
            reuse = run.execute(label + "-reuse", ["annotation-library", "place", captured, *placement, "--author", "Reuse author", "--out", reused])
            run.record(reused, "reused_pdf")
            run.check(label + ": reuse receipt", Path(reuse["out"]).resolve() == reused.resolve() and reuse["count"] == 14
                      and reuse["indices"] == list(range(14)) and reuse["original_unchanged"] is True)
            verify_pdf(run.check, label, run.inspect(label + "-source", source), run.inspect(label + "-reused", reused), rotation)
            images = []
            for kind, pdf in (("source", source), ("reused", reused)):
                path = run.directory / f"{label}-{kind}.png"
                result = run.execute(label + "-render-" + kind, ["render", pdf, "--page", 1, "--scale", 1, "--out", path])
                run.record(path, "rendered_png")
                pixels = image_pixels(path)
                run.check(label + ": " + kind + " actual rendered image", Path(result["out"]).resolve() == path.resolve()
                          and result["width"] == result["height"] == 800 and result["page_pixels"] == [800, 800]
                          and result["page"] == 1 and result["region"] is None
                          and sum(pixels[offset:offset+3] != b"\xff\xff\xff" for offset in range(0, len(pixels), 4)) > 5000)
                run.report["images"].append({"name": path.name, "sha256": sha(path), "rgba_sha256": hashlib.sha256(pixels).hexdigest(),
                    "width": 800, "height": 800, "source_pdf": pdf.name, "source_pdf_sha256": sha(pdf), "rotation": rotation})
                images.append(pixels)
            run.check(label + ": every rendered RGBA pixel preserved", images[0] == images[1])
            run.preserved(label + ": originals and retained evidence unchanged")
            run.report["sources"].extend({"name": path.name, "sha256": sha(path), "rotation": rotation, "kind": kind}
                                        for kind, path in (("blank", blank), ("saved", source)))
            run.report["outputs"].extend({"name": path.name, "sha256": sha(path), "bytes": path.stat().st_size} for path in (captured, reused))
            return captured, source, reused, placement, capture, selections
        results.append(run.case(f"rotation-{rotation}", "capture_reuse", qualify))
    captured, source, reused, placement, capture, selections = results[0]
    absent = run.directory / "stale-refused.json"
    stale = selections.copy()
    stale[-1] = stale[-1].split(":")[0] + ":" + "0" * 64
    refusals = [
        ("refuse-stale-selection", [*capture, *stale, "--out", absent], absent),
        ("refuse-capture-overwrite", [*capture, *selections, "--out", captured, "--overwrite"], None),
        ("refuse-reuse-overwrite", ["annotation-library", "place", captured, *placement, "--out", reused, "--overwrite"], None),
        ("refuse-source-overwrite", ["annotation-library", "place", captured, *placement, "--out", source, "--overwrite"], None)]
    for name, args, missing in refusals:
        def refuse(name=name, args=args, missing=missing):
            run.execute(name, args, reject=True)
            run.check(name + ": no partial output", missing is None or not missing.exists())
            run.preserved(name + ": all existing files retained")
        run.case(name, "refusal", refuse)
    run.report["originals_unchanged"] = all(path.is_file() and sha(path) == digest for path, digest in run.protected.items())
    run.check("all sources and artifacts unchanged", run.report["originals_unchanged"])
    run.check("complete planned command and case coverage", len(run.report["commands"]) == 45
              and sum(item["expected_refusal"] for item in run.report["commands"]) == 4
              and len(run.report["cases"]) == 8 and len(run.report["images"]) == 8)


def offline_page(rotation, author, identity):
    """Explicitly synthetic decoded graph for offline oracle tests; never runtime evidence."""
    annotations = []
    matrix = {0: [1, 0, 0, 1, 0, 0], 90: [0, 1, -1, 0, 0, 0],
              180: [-1, 0, 0, -1, 0, 0], 270: [0, -1, 1, 0, 0, 0]}[rotation]
    for index, value in enumerate(primitives()):
        annotation = {"/NM": f"u:SuniPDF-{identity}-{index}", "/T": "u:" + author,
                      "/Rect": user_rect(value["rect"], rotation),
                      "/Subtype": {"free_text": "/FreeText", "shape": "/Square", "stamp": "/Stamp"}[value["kind"]]}
        if value["kind"] == "free_text":
            style = value["style"]
            annotation.update({"/SuniPDFRich": "/FreeText", "/Contents": "u:" + value["text"],
                "/CA": style["opacity"], "/BS": {"/W": style["border_pt"]},
                "/DA": "u:/F1 12 Tf " + " ".join(str(f32(channel / 255)) for channel in style["rgb"]) + " rg",
                "/AP": {"/N": {"dictionary": {"/BBox": [0, 0, 340, 80], "/Matrix": matrix,
                    "/Resources": {"/Font": {"/F1": {"/BaseFont": "/" + base_font(style), "/Subtype": "/Type1"}}}},
                    "decoded": b"BT /F1 12 Tf (offline synthetic glyphs) Tj ET".hex()}}})
            if style["fill_rgb"] is not None:
                annotation["/C"] = [f32(channel / 255) for channel in style["fill_rgb"]]
        annotations.append(annotation)
    return {"attributes": {"/CropBox": list(CROP), "/MediaBox": [0, 0, 1000, 1000], "/Rotate": rotation,
                "/Resources": {"/Font": {"/F1": {"/Type": "/Font", "/Subtype": "/Type1", "/BaseFont": "/Helvetica"}}}},
            "contents": [{"decoded": CONTENT.hex()}], "annotations": annotations}


def self_test():
    for rotation in ROTATIONS:
        data = fixture_pdf(rotation)
        offset = int(data.split(b"startxref\n")[1].splitlines()[0])
        assert data[offset:].startswith(b"xref\n0 6\n")
        entries = data[offset:].splitlines()[3:8]
        for number, line in enumerate(entries, 1):
            assert data[int(line[:10]):].startswith(f"{number} 0 obj\n".encode())
        assert CONTENT in data
    values = primitives()
    assert len(values) == 14 and len({base_font(value["style"]) for value in values[:12]}) == 12
    assert user_rect([0, 0, 340, 80], 0) == [140.25, 100.5, 480.25, 180.5]
    assert user_rect([0, 0, 340, 80], 90) == [770.25, 90.5, 850.25, 430.5]
    value = {"pages": [{"object": "1 0 R", "contents": ["2 0 R"]}], "qpdf": [{}, {
        "obj:1 0 R": {"value": {"/Type": "/Page", "/Annots": ["3 0 R"]}},
        "obj:2 0 R": {"stream": {"dict": {"/Length": 2}, "data": base64.b64encode(b"qQ").decode()}},
        "obj:3 0 R": {"value": {"/P": "1 0 R", "/AP": {"/N": "2 0 R"}, "/Text": "b:4142"}}}]}
    original = qpdf_page(value)
    assert original["annotations"][0]["/Text"] == "u:AB"
    assert original["annotations"][0]["/AP"]["/N"]["decoded"] == b"qQ".hex()
    value["qpdf"][1]["obj:3 0 R"]["value"]["/P"] = "9 0 R"
    try:
        qpdf_page(value)
    except ValueError:
        pass
    else:
        raise AssertionError("incorrect owning page was accepted")
    def assert_check(name, passed):
        if not passed:
            raise AssertionError(name)
    for rotation in ROTATIONS:
        source = offline_page(rotation, "Source author", "original")
        reused = offline_page(rotation, "Reuse author", "reused")
        verify_pdf(assert_check, "offline positive", source, reused, rotation)
        for target, mutation in [
            ("reused", lambda page: page["contents"][0].update(decoded=b"changed".hex())),
            ("both", lambda page: page["attributes"].update({"/Rotate": -1})),
            ("both", lambda page: page["annotations"][0].update({"/Contents": "u:Changed text"})),
            ("both", lambda page: page["annotations"][0].update({"/DA": "u:/F1 12 Tf 1 0 0 rg"})),
            ("both", lambda page: page["annotations"][0].update({"/CA": 0.2})),
            ("both", lambda page: page["annotations"][0]["/AP"]["/N"]["dictionary"]["/Resources"]["/Font"]["/F1"].update({"/BaseFont": "/Courier"})),
            ("both", lambda page: page["annotations"][0]["/AP"]["/N"]["dictionary"]["/Resources"]["/Font"]["/F1"].update({"/FontDescriptor": {"/FontFile2": "embedded"}})),
            ("both", lambda page: page["annotations"][0]["/AP"]["/N"].update(decoded=b"q Q".hex())),
            ("reused", lambda page: page["annotations"][0]["/AP"]["/N"].update(decoded=b"BT /F1 12 Tf (different) Tj ET".hex())),
            ("reused", lambda page: page["annotations"][0].update({"/NM": "u:SuniPDF-original-0"})),
            ("reused", lambda page: page["annotations"][0].update({"/T": "u:Source author"})),
        ]:
            old, new = copy.deepcopy(source), copy.deepcopy(reused)
            mutation(new)
            if target == "both":
                mutation(old)
            try:
                verify_pdf(assert_check, "offline tamper", old, new, rotation)
            except AssertionError:
                pass
            else:
                raise AssertionError("independent oracle accepted altered text PDF")
    print("Saved text fixture, decoding and 44 tamper self-tests passed; no product process executed.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--qpdf", type=Path)
    parser.add_argument("--directory", type=Path)
    parser.add_argument("--expected-version")
    parser.add_argument("--source-sha", default=os.environ.get("GITHUB_SHA"))
    parser.add_argument("--deadline-seconds", type=int, default=600)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        if args.cli or args.qpdf or args.directory:
            parser.error("self-test does not accept executable paths or an output directory")
        self_test()
        return
    if not args.cli or not args.qpdf or not args.directory or not args.expected_version or not re.fullmatch(r"\d+\.\d+\.\d+", args.expected_version):
        parser.error("supply --cli, --qpdf, a new --directory and --expected-version")
    if not args.source_sha or not re.fullmatch(r"[a-fA-F0-9]{40}", args.source_sha):
        parser.error("supply full --source-sha or GITHUB_SHA")
    if not 60 <= args.deadline_seconds <= 1200:
        parser.error("deadline-seconds must be between 60 and 1200")
    cli, qpdf = args.cli.resolve(strict=True), args.qpdf.resolve(strict=True)
    if not cli.is_file() or not qpdf.is_file():
        parser.error("CLI and QPDF must be regular files")
    directory = args.directory.absolute()
    if directory.exists() or directory.is_symlink():
        raise FileExistsError(directory)
    directory.mkdir(parents=True, exist_ok=False)
    run = Corpus(cli, qpdf, directory.resolve(), args.expected_version, args.source_sha, args.deadline_seconds)
    try:
        run_corpus(run)
        run.report["status"] = "passed"
    except Exception as error:
        run.report["status"], run.report["error"] = "failed", str(error)
        raise
    finally:
        run.finish()
    print(json.dumps({"status": run.report["status"], "counts": run.report["counts"], "report": str(directory / "report.json")}))


if __name__ == "__main__":
    main()
