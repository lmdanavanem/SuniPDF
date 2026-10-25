"""Qualify packaged overlay-stack CLI exports with original PDFs and decoded pixels."""
import argparse
import copy
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import time
from datetime import datetime, timezone

from PIL import Image, __version__ as pillow_version


SCOPE = "Synthetic packaged CLI overlay exports and independent interior-pixel checks; no native desktop interaction or private-plan claim."


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def original_pdf(crop, rotation, displayed_rectangle, page_count=1):
    """One black rectangle in known displayed coordinates; preceding pages are blank."""
    left, bottom, right, top = crop

    def user_point(x, y):
        # Invert the PDF page rotation and top-left display origin, independently
        # of any application report or image. All source coordinates are integers.
        return {0: (left + x, top - y), 90: (left + y, bottom + x),
                180: (right - x, bottom + y), 270: (right - y, top - x)}[rotation]

    x0, y0, x1, y1 = displayed_rectangle
    corners = [user_point(x, y) for x in (x0, x1) for y in (y0, y1)]
    x, y = min(p[0] for p in corners), min(p[1] for p in corners)
    width, height = max(p[0] for p in corners) - x, max(p[1] for p in corners) - y
    drawing = f"0 g\n{x} {y} {width} {height} re f\n".encode("ascii")
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>", b""]
    kids = []
    for page in range(page_count):
        page_id = len(objects) + 1
        kids.append(f"{page_id} 0 R")
        objects.append((f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 160 160] "
                        f"/CropBox [{' '.join(map(str, crop))}] /Rotate {rotation} "
                        f"/Resources << >> /Contents {page_id + 1} 0 R >>").encode("ascii"))
        content = drawing if page == page_count - 1 else b"q Q\n"
        objects.append(b"<< /Length " + str(len(content)).encode("ascii")
                       + b" >>\nstream\n" + content + b"endstream")
    objects[1] = f"<< /Type /Pages /Count {page_count} /Kids [{' '.join(kids)}] >>".encode("ascii")
    data = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for index, body in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f"{index} 0 obj\n".encode("ascii") + body + b"\nendobj\n")
    xref = len(data)
    data.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode("ascii"))
    for offset in offsets[1:]:
        data.extend(f"{offset:010d} 00000 n \n".encode("ascii"))
    data.extend(f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode("ascii"))
    return bytes(data)


def manual(translation=(0, 0), rotation=0, scale=1):
    return {"mode": "manual", "transform": {"translation": list(translation),
            "rotation_degrees": rotation, "scale": scale}}


def interior_color(layers):
    """Analytic ink-on-white expectation for solid black rectangle interiors."""
    return tuple(math.floor(255 * math.prod(
        (1 - layer["opacity"]) + layer["opacity"] * layer["color"][channel] / 255
        for layer in layers) + 0.5) for channel in range(3)) + (255,)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--expected-version", required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.expected_version):
        raise ValueError("Supply the explicit packaged release version")
    cli = args.cli.resolve(strict=True)
    if not cli.is_file():
        raise ValueError("CLI must be a file")
    directory = args.directory.absolute()
    # Never resolve an existing link into another directory or overwrite earlier evidence.
    if directory.exists() or directory.is_symlink():
        raise FileExistsError(directory)
    directory.mkdir(parents=True, exist_ok=False)
    directory = directory.resolve()
    started = time.monotonic()
    deadline = started + 120
    report = {"schema": 1, "status": "running", "version": args.expected_version,
              "source_revision": os.environ.get("GITHUB_SHA"), "cli_sha256": sha(cli),
              "pillow_version": pillow_version, "scope": SCOPE, "native_desktop_tested": False,
              "fixture_license": "CC0-1.0", "started_utc": datetime.now(timezone.utc).isoformat(),
              "checks": [], "commands": [], "sources": [], "outputs": []}
    sources, outputs = {}, []

    def check(name, condition):
        report["checks"].append({"name": name, "passed": bool(condition)})
        if not condition:
            raise AssertionError(name)

    def execute(name, arguments, reject=False, parse=True):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("Overlay corpus exceeded its 120-second total budget")
        process = subprocess.run([str(cli), *map(str, arguments)], cwd=directory,
                                 stdin=subprocess.DEVNULL, capture_output=True,
                                 timeout=min(30, remaining), env=os.environ.copy())
        for stream in ("stdout", "stderr"):
            (directory / f"{name}.{stream}").write_bytes(getattr(process, stream))
        report["commands"].append({"name": name, "arguments": list(map(str, arguments)),
                                   "exit_code": process.returncode, "expected_refusal": reject})
        check(name + ": process outcome", process.returncode != 0 if reject else process.returncode == 0)
        check(name + ": bounded transcripts", len(process.stdout) <= 2_000_000 and len(process.stderr) <= 2_000_000)
        return json.loads(process.stdout) if parse and not reject else process.stdout

    def manifest(name, layers):
        path = directory / (name + ".json")
        with path.open("x", encoding="utf-8") as stream:
            json.dump({"version": 1, "long_side": 128, "layers": layers}, stream, indent=2)
            stream.write("\n")
        return path

    def stack(name, layers, format="png"):
        path = manifest(name + "-manifest", layers)
        output = directory / (name + "." + format)
        value = execute(name, ["revision-overlay-stack", "--manifest", path, "--format", format, "--out", output])
        check(name + ": new output and source preservation declared", output.is_file() and value["originals_unchanged"] is True)
        outputs.append(output)
        return output, value["report"]

    def image(path):
        with Image.open(path) as decoded:
            check(path.stem + ": fixed base canvas", decoded.format == "PNG" and decoded.size == (128, 128))
            result = decoded.convert("RGBA")
        check(path.stem + ": opaque export", result.getchannel("A").getextrema() == (255, 255))
        return result

    def patch(name, pixels, center, expected):
        x, y = center
        check(name, all(abs(actual - wanted) <= 1
                        for dx in range(-2, 3) for dy in range(-2, 3)
                        for actual, wanted in zip(pixels.getpixel((x + dx, y + dy)), expected)))

    def refuse(name, layers, existing=None, format="png"):
        path = manifest(name + "-manifest", layers)
        output = existing or directory / (name + "." + format)
        protected = {path: sha(path) for path in [*sources, *outputs]}
        arguments = ["--overwrite", "revision-overlay-stack", "--manifest", path,
                     "--format", format, "--out", output]
        execute(name, arguments, reject=True, parse=False)
        check(name + ": no output created", existing is not None or not output.exists())
        check(name + ": sources and previous exports unchanged", all(sha(path) == digest for path, digest in protected.items()))

    try:
        version = execute("version", ["--version"], parse=False).decode("utf-8").strip()
        report["cli_version"] = version
        check("explicit packaged CLI version", version == "sunipdf-cli " + args.expected_version)
        geometry = [([0, 0, 128, 128], 0, [16, 16, 48, 48], 1, [128, 128]),
                    ([10, 20, 138, 116], 90, [16, 16, 48, 48], 1, [96, 128]),
                    ([20, 30, 84, 126], 180, [8, 8, 24, 24], 1, [64, 96]),
                    ([5, 15, 85, 143], 270, [16, 16, 48, 48], 2, [128, 80])]
        for index, (crop, rotation, rectangle, pages, _) in enumerate(geometry):
            path = directory / f"layer-{index + 1}.pdf"
            path.write_bytes(original_pdf(crop, rotation, rectangle, pages))
            sources[path] = sha(path)
        defaults = [{"path": path.name, "expected_sha256": digest} for path, digest in list(sources.items())[:2]]
        for name, alignment, legacy_options in (
                ("defaults", None, []),
                ("transformed", manual((20, 10), 15, 0.8),
                 ["--translate", "20,10", "--rotation", "15", "--scale", "0.8"])):
            layers = copy.deepcopy(defaults)
            if alignment:
                layers[1]["alignment"] = alignment
            actual, _ = stack("stack-" + name, layers)
            legacy = directory / ("legacy-" + name + ".png")
            execute("legacy-" + name, ["revision-overlay", *list(sources)[:2], "--max-side", "128",
                                       *legacy_options, "--out", legacy])
            outputs.append(legacy)
            check(name + ": decoded pixels exactly match legacy", image(actual).tobytes() == image(legacy).tobytes())

        colors = [[220, 20, 40], [15, 160, 210], [40, 180, 60], [250, 130, 20]]
        opacities = [1, 1, 0.5, 0.75]
        layers = [{"path": path.name, "page": geometry[index][3], "color": colors[index],
                   "opacity": opacities[index], "visible": True, "expected_sha256": digest}
                  for index, (path, digest) in enumerate(sources.items())]
        layers[1]["alignment"] = manual((48, 0))
        layers[2]["alignment"] = {"mode": "two_points", "base": [[16, 64], [48, 64]], "revision": [[8, 8], [24, 8]]}
        layers[3]["alignment"] = manual((112, 48), 90)
        separated, details = stack("four-separated", layers)
        separated_pixels = image(separated)
        centers = [(32, 32), (80, 32), (32, 80), (80, 80)]
        for index, center in enumerate(centers):
            patch(f"layer {index + 1}: independently aligned color and opacity", separated_pixels, center, interior_color([layers[index]]))
        patch("separated: empty base area remains white", separated_pixels, (112, 112), (255, 255, 255, 255))
        check("four-layer report schema and independent composition", details["schema"] == "sunipdf-revision-overlay-stack"
              and details["version"] == 1 and details["composition"] == "multiply_luminance_tints_on_white"
              and len(details["layers"]) == 4 and details["canvas"] == {"width": 128, "height": 128, "base_layer": 0, "clip_to_base": True})
        for index, (path, digest) in enumerate(sources.items()):
            entry, expected = details["layers"][index], geometry[index]
            source = entry["source"]
            check(f"layer {index + 1}: exact source page crop and rotation", source["source_sha256"] == digest
                  and Path(source["path"]).samefile(path) and source["page"] == expected[3]
                  and source["page_count"] == expected[3] and source["crop_box_user_pt"] == expected[0]
                  and source["rotation_degrees"] == expected[1]
                  and [source["display_width_pt"], source["display_height_pt"]] == expected[4])
            check(f"layer {index + 1}: exact independent style", entry["color"] == colors[index]
                  and entry["opacity"] == opacities[index] and entry["visible"] is True)
        check("two anchors resolve independently to scale and translation", details["layers"][2]["transform"]
              == {"translation": [0.0, 48.0], "rotation_degrees": 0.0, "scale": 2.0})
        saved_json, json_stdout = stack("four-report", layers, "json")
        check("saved JSON equals the source-bound stdout report", json.loads(saved_json.read_text(encoding="utf-8")) == json_stdout == details)
        for name, index, field, value in (("hidden-third", 2, "visible", False), ("zero-fourth", 3, "opacity", 0)):
            changed = copy.deepcopy(layers)
            changed[index][field] = value
            exported, _ = stack(name, changed)
            decoded = image(exported)
            for layer, center in enumerate(centers):
                expected = (255, 255, 255, 255) if layer == index else interior_color([layers[layer]])
                patch(f"{name}: layer {layer + 1} visibility and style", decoded, center, expected)
        overlap = copy.deepcopy(layers)
        overlap[1]["alignment"] = manual()
        overlap[2]["alignment"]["base"] = [[16, 16], [48, 16]]
        overlap[3]["alignment"] = manual((64, 0), 90)
        combined, _ = stack("four-overlap", overlap)
        combined_pixels = image(combined)
        patch("four overlapping tints match independent solid-ink calculation", combined_pixels, (32, 32), interior_color(overlap))
        patch("overlap: independent transforms vacate the other regions", combined_pixels, (80, 80), (255, 255, 255, 255))

        changed = copy.deepcopy(layers)
        changed[2].update(path="missing-hidden.pdf", visible=False)
        refuse("refuse-missing-hidden", changed)
        changed = copy.deepcopy(layers)
        changed[2].update(expected_sha256="0" * 64, visible=False)
        refuse("refuse-stale-hidden", changed)
        refuse("refuse-fifth-layer", [*layers, copy.deepcopy(layers[0])])
        refuse("refuse-all-hidden", [dict(layer, visible=False) for layer in layers])
        changed = copy.deepcopy(layers)
        changed[0]["alignment"] = manual((1, 0))
        refuse("refuse-base-alignment", changed)
        for name, index, field, value in (("zero-page", 0, "page", 0), ("missing-page", 3, "page", 3),
                                         ("opacity", 1, "opacity", 2), ("unknown-field", 1, "unknown", True)):
            changed = copy.deepcopy(layers)
            changed[index][field] = value
            refuse("refuse-" + name, changed)
        changed = copy.deepcopy(layers)
        changed[2]["alignment"]["revision"][1] = [200, 8]
        refuse("refuse-outside-anchor", changed)
        refuse("refuse-overwrite-png", layers, existing=separated)
        refuse("refuse-overwrite-json", layers, existing=saved_json, format="json")
        check("all original PDFs remain byte-for-byte unchanged", all(sha(path) == digest for path, digest in sources.items()))
        check("bounded total runtime", time.monotonic() - started <= 120)
        report["sources"] = [{"name": path.name, "sha256": digest} for path, digest in sources.items()]
        report["outputs"] = [{"name": path.name, "sha256": sha(path), "bytes": path.stat().st_size} for path in outputs]
        report["status"] = "passed"
    except Exception as error:
        report["status"], report["error"] = "failed", str(error)
        raise
    finally:
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        report["elapsed_seconds"] = round(time.monotonic() - started, 3)
        with (directory / "report.json").open("x", encoding="utf-8") as stream:
            stream.write(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"status": report["status"], "checks": len(report["checks"]),
                      "commands": len(report["commands"]), "report": str(directory / "report.json")}))


if __name__ == "__main__":
    main()
