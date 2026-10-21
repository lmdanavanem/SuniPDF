"""Exercise page decorations in the actual CLI with independent fixtures and pixels."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import secrets
import subprocess
import tempfile
import time

from PIL import Image, __version__ as pillow_version


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def blank_pdf(rotation=0):
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>",
               b"<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>"]
    for _ in range(2):
        objects.append(("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 600] "
                        f"/CropBox [10 20 390 580] /Rotate {rotation} /Resources << >> >>").encode())
    data = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for index, obj in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f"{index} 0 obj\n".encode() + obj + b"\nendobj\n")
    start = len(data)
    data.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    for offset in offsets[1:]:
        data.extend(f"{offset:010} 00000 n \n".encode())
    data.extend(f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n".encode())
    return bytes(data)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--mutool", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", "target")) / "test-mupdf/1.28.5/mutool.exe")
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    mutool = args.mutool.resolve(strict=True)
    assert sha(mutool) == "1ff045eeab80a108a39760578001a547a31e8b663905f064088c4b2f82cbcf7f"
    qpdf = Path(os.environ["SUNIPDF_QPDF"]).resolve(strict=True)
    assert sha(qpdf) == "1bdaf878f37eaffea1ea3c30591214c77636a0f2fc7432427516b6e22e36c0ff"
    assert pillow_version == "12.3.0", pillow_version
    started = time.monotonic()
    commands = 0
    cases = []
    qpdf_checks = 2
    independent_reader_checks = 0
    sources = {}
    with tempfile.TemporaryDirectory(prefix="sunipdf-page-decorations-") as directory:
        root = Path(directory)

        def remember(path):
            sources[path] = sha(path)
            return path

        def run(arguments, success=True, env=None):
            nonlocal commands
            output = subprocess.run([str(cli), *map(str, arguments)], cwd=root, capture_output=True,
                                    timeout=30, env={**os.environ, **(env or {})})
            commands += 1
            assert (output.returncode == 0) == success, (arguments[0], output.returncode, output.stderr[:1000])
            assert len(output.stdout) < 2 * 1024 * 1024 and len(output.stderr) < 2 * 1024 * 1024
            if success:
                return json.loads(output.stdout)
            assert not output.stdout
            return json.loads(output.stderr)

        source = root / "original été 中文.pdf"
        source.write_bytes(blank_pdf())
        remember(source)
        logo = root / "transparent logo.png"
        image = Image.new("RGBA", (80, 40), (255, 0, 0, 128))
        for x in range(40, 80):
            for y in range(40):
                image.putpixel((x, y), (0, 0, 255, 255))
        image.save(logo)
        remember(logo)

        def watermark(document, name, extra=(), image_path=logo, success=True, env=None):
            out = root / (name + ".pdf")
            report = run(["image-watermark", document, "--image", image_path, "--out", out, *extra], success, env)
            if success:
                assert report["original_unchanged"] and report["foreground"] and out.exists()
            else:
                assert not out.exists()
            return out, report

        positions = ["center", "top-left", "top-center", "top-right", "bottom-left", "bottom-center", "bottom-right"]
        for rotation in [0, 90, 180, 270]:
            document = root / f"rotated-{rotation}.pdf"
            document.write_bytes(blank_pdf(rotation))
            remember(document)
            for position in positions:
                out, report = watermark(document, f"placed-{rotation}-{position}",
                                        ["--pages", "1,1", "--width-percent", "90", "--angle=-45", "--position", position])
                assert report["watermarked"] == 1
                objects = run(["objects", out, "--page", 1])
                assert len(objects["objects"]) == 1
                bounds = objects["objects"][0]["bounds"]
                size = objects["size"]
                assert bounds[0] >= 27.8 and bounds[1] >= 27.8, bounds
                assert bounds[2] <= size["width_pt"] - 27.8 and bounds[3] <= size["height_pt"] - 27.8, bounds
                assert not run(["objects", out, "--page", 2])["objects"]
            cases.append(f"rotation-{rotation}-seven-positions-crop-fit-and-selected-page")

        placed, report = watermark(source, "alpha", ["--opacity", "0.5", "--width-percent", "40"])
        assert report["watermarked"] == 2
        rendered = root / "rendered.png"
        run(["render", placed, "--page", 1, "--scale", 1, "--out", rendered])
        with Image.open(rendered) as pixels:
            colors = list(pixels.convert("RGB").get_flattened_data())
            assert sum(r >= 250 and abs(g - 191) <= 4 and abs(b - 191) <= 4 for r, g, b in colors) > 100
            assert sum(abs(r - 127) <= 4 and abs(g - 127) <= 4 and b >= 250 for r, g, b in colors) > 100
        cases.append("existing-alpha-multiplies-opacity-in-independent-pixels")
        checked = subprocess.run([str(qpdf), "--check", str(placed)], capture_output=True, timeout=25)
        assert checked.returncode == 0, checked.stderr[:1000]
        cases.append("all-pages-copy-passes-independent-qpdf")

        photo = root / "orientation-6.jpg"
        exif = Image.Exif()
        exif[274] = 6
        Image.new("RGB", (80, 40), "red").save(photo, exif=exif)
        remember(photo)
        oriented, _ = watermark(source, "oriented", image_path=photo)
        bounds = run(["objects", oriented])["objects"][0]["bounds"]
        assert abs((bounds[3] - bounds[1]) / (bounds[2] - bounds[0]) - 2) < 0.01
        cases.append("jpeg-exif-orientation-applied-before-fit")

        for name, extra in [
            ("invalid-page", ["--pages", "1,3"]),
            ("invalid-width", ["--width-percent", "0"]),
            ("invalid-opacity", ["--opacity", "NaN"]),
            ("invalid-angle", ["--angle", "NaN"]),
            ("invalid-margin", ["--margin", "1000"]),
            ("invalid-position", ["--position", "outside"]),
        ]:
            watermark(source, name, extra, success=False)
            cases.append(name + "-refuses-copy")
        for name, size in [("transparent", (20, 20)), ("oversized-side", (4097, 1)),
                           ("oversized-pixels", (2100, 2000))]:
            path = root / (name + ".png")
            Image.new("RGBA", size, (0, 0, 0, 0) if name == "transparent" else (0, 0, 0, 255)).save(path)
            remember(path)
            watermark(source, name, image_path=path, success=False)
            cases.append(name + "-refuses-copy")
        malformed = root / "malformed.png"
        malformed.write_bytes(b"generated non-image fixture")
        remember(malformed)
        watermark(source, "malformed", image_path=malformed, success=False)
        cases.append("malformed-image-refuses-copy")
        watermark(source, "missing", image_path=root / "absent.png", success=False)
        cases.append("missing-image-refuses-copy")
        before = sha(placed)
        run(["image-watermark", source, "--image", logo, "--out", placed, "--overwrite"], False)
        assert sha(placed) == before
        cases.append("existing-copy-preserved-even-with-overwrite")
        run(["image-watermark", source, "--image", logo, "--out", source, "--overwrite"], False)
        cases.append("original-cannot-be-output-even-with-overwrite")

        credentials = {"SUNIPDF_DECORATION_USER": secrets.token_hex(24), "SUNIPDF_DECORATION_OWNER": secrets.token_hex(24)}
        for aes in [128, 256]:
            for restricted in [False, True]:
                encrypted = root / f"protected-{aes}-{restricted}.pdf"
                run(["protect", source, "--user-password-env", "SUNIPDF_DECORATION_USER",
                     "--owner-password-env", "SUNIPDF_DECORATION_OWNER", "--out", encrypted]
                    + (["--aes128"] if aes == 128 else []) + (["--no-modify"] if restricted else []), env=credentials)
                remember(encrypted)
                out, report = watermark(encrypted, f"protected-result-{aes}-{restricted}",
                                        ["--password-env", "SUNIPDF_DECORATION_USER"], success=not restricted, env=credentials)
                if not restricted:
                    assert report["encrypted"]
                    for role in ["USER", "OWNER"]:
                        result = run(["objects", out, "--password-env", f"SUNIPDF_DECORATION_{role}"], env=credentials)
                        assert len(result["objects"]) == 1
                        assert not any(value in json.dumps(result) for value in credentials.values())
                cases.append(f"aes{aes}-" + ("modification-restriction-preserved" if restricted else "both-credentials-preserved"))
        zones = ["--header-left", "Left", "--header-center", "Center", "--header-right", "Right",
                 "--footer-left", "Author", "--footer-center", "{n} / {total}", "--footer-right", "Copy"]

        def headers(document, name, extra=(), content=zones, success=True, env=None):
            out = root / (name + ".pdf")
            report = run(["headers-footers", document, *content, "--out", out, *extra], success, env)
            if success:
                assert report["original_unchanged"] and out.exists()
            else:
                assert not out.exists()
            return out, report

        for rotation in [0, 90, 180, 270]:
            document = root / f"rotated-{rotation}.pdf"
            out, report = headers(document, f"headers-{rotation}", ["--pages", "1,1"])
            assert report["decorated"] == 1
            result = run(["objects", out])
            objects = result["objects"]
            assert len(objects) == 6
            assert [obj["text"] for obj in objects] == ["Left", "Center", "Right", "Author", "1 / 2", "Copy"]
            for index, obj in enumerate(objects):
                left, bottom, right, top = obj["bounds"]
                assert left >= 27.8 and bottom >= 27.8
                assert right <= result["size"]["width_pt"] - 27.8 and top <= result["size"]["height_pt"] - 27.8
                if index % 3 != 2:
                    assert right < objects[index + 1]["bounds"][0]
            assert not run(["objects", out, "--page", 2])["objects"]
            cases.append(f"headers-rotation-{rotation}-six-zones-numbering-and-non-overlap")
        header_copy, report = headers(source, "headers-all")
        assert report["decorated"] == 2
        assert run(["objects", header_copy, "--page", 2])["objects"][4]["text"] == "2 / 2"
        checked = subprocess.run([str(qpdf), "--check", str(header_copy)], capture_output=True, timeout=25)
        assert checked.returncode == 0, checked.stderr[:1000]
        cases.append("headers-all-pages-numbering-and-independent-qpdf")
        for name, extra, content in [
            ("empty", [], []), ("control", [], ["--header-left", "a\tb"]),
            ("long", [], ["--header-left", "x" * 257]),
            ("overflow", [], ["--header-left", "Fits", "--footer-right", "W" * 200]),
            ("size", ["--size", "NaN"], zones), ("margin", ["--margin", "1000"], zones),
            ("page", ["--pages", "1,3"], zones),
        ]:
            headers(source, "headers-invalid-" + name, extra, content, False)
            cases.append("headers-invalid-" + name + "-refuses-copy")
        before = sha(header_copy)
        run(["headers-footers", source, *zones, "--out", header_copy, "--overwrite"], False)
        assert sha(header_copy) == before
        cases.append("headers-existing-copy-preserved-even-with-overwrite")
        run(["headers-footers", source, *zones, "--out", source, "--overwrite"], False)
        cases.append("headers-original-cannot-be-output")
        for aes in [128, 256]:
            for restricted in [False, True]:
                encrypted = root / f"headers-protected-{aes}-{restricted}.pdf"
                run(["protect", source, "--user-password-env", "SUNIPDF_DECORATION_USER",
                     "--owner-password-env", "SUNIPDF_DECORATION_OWNER", "--out", encrypted]
                    + (["--aes128"] if aes == 128 else []) + (["--no-modify"] if restricted else []), env=credentials)
                remember(encrypted)
                out, report = headers(encrypted, f"headers-protected-result-{aes}-{restricted}",
                                      ["--password-env", "SUNIPDF_DECORATION_USER"], success=not restricted, env=credentials)
                if not restricted:
                    assert report["encrypted"]
                    for role in ["USER", "OWNER"]:
                        result = run(["objects", out, "--password-env", f"SUNIPDF_DECORATION_{role}"], env=credentials)
                        assert len(result["objects"]) == 6
                        assert not any(value in json.dumps(result) for value in credentials.values())
                cases.append(f"headers-aes{aes}-" + ("restriction-preserved" if restricted else "both-credentials-preserved"))
        def image_stamp(document, name, extra=(), image_path=logo, success=True, env=None):
            out = root / (name + ".pdf")
            result = run(["image-stamp", document, "--image", image_path, "--rect", "40,40,240,240",
                          "--out", out, *extra], success, env)
            assert out.exists() == success
            if success:
                assert result["original_unchanged"] and result["subtype"] == "Stamp"
            return out, result

        def independent_render(document, name):
            nonlocal independent_reader_checks
            path = root / (name + ".ppm")
            output = subprocess.run([str(mutool), "draw", "-F", "ppm", "-c", "rgb", "-r", "72",
                                     "-o", str(path), str(document), "1"], capture_output=True, timeout=30)
            assert output.returncode == 0 and path.exists(), output.stderr[:1000]
            independent_reader_checks += 1
            with Image.open(path) as image:
                return image.convert("RGB")

        for rotation in [0, 90, 180, 270]:
            document = root / f"rotated-{rotation}.pdf"
            out, result = image_stamp(document, f"image-stamp-{rotation}", ["--opacity", "0.5", "--author", "Plan reviewer"])
            annotations = run(["annotations", out])["pages"]
            assert len(annotations[0]["annotations"]) == 1 and not annotations[1]["annotations"]
            annotation = annotations[0]["annotations"][0]
            assert annotation["subtype"] == "Stamp"
            assert not run(["objects", out])["objects"], "image stamp must not become page content"
            rendered = root / f"image-stamp-{rotation}.png"
            run(["render", out, "--scale", 1, "--out", rendered])
            with Image.open(rendered) as image:
                pdfium = image.convert("RGB")
            mupdf = independent_render(out, f"stamp-independent-{rotation}")
            assert pdfium.size == mupdf.size
            for image in [pdfium, mupdf]:
                height = image.height
                for x, expected in [(80, (255, 191, 191)), (180, (127, 127, 255))]:
                    actual = image.getpixel((x, height - 1 - 140))
                    assert all(abs(a - b) <= 4 for a, b in zip(actual, expected)), (rotation, actual)
                assert image.getpixel((80, height - 1 - 60)) == (255, 255, 255), "aspect margin stays empty"
            checked = subprocess.run([str(qpdf), "--check", str(out)], capture_output=True, timeout=25)
            assert checked.returncode == 0, checked.stderr[:1000]
            qpdf_checks += 1
            cases.append(f"image-stamp-rotation-{rotation}-real-annotation-alpha-aspect-qpdf-and-mupdf")
        oriented, _ = image_stamp(source, "image-stamp-exif", image_path=photo)
        pixels = independent_render(oriented, "stamp-exif-independent")
        red = [(x, y) for y in range(pixels.height) for x in range(pixels.width)
               if pixels.getpixel((x, y))[0] > 240 and pixels.getpixel((x, y))[1] < 20]
        assert red
        width = max(x for x, _ in red) - min(x for x, _ in red) + 1
        height = max(y for _, y in red) - min(y for _, y in red) + 1
        assert abs(height / width - 2) < 0.05
        cases.append("image-stamp-exif-orientation-visible-in-independent-reader")
        for name, extra in [("opacity", ["--opacity", "NaN"]), ("invisible", ["--opacity", "0"]),
                            ("page", ["--page", "3"]), ("rect", ["--rect", "0,0,1,1"]),
                            ("outside", ["--rect=-10,0,100,100"]), ("author", ["--author", "bad\nauthor"])]:
            # Avoid duplicate flags so clap cannot mask the geometry refusal.
            if name in ["rect", "outside"]:
                out = root / f"stamp-invalid-{name}.pdf"
                run(["image-stamp", source, "--image", logo, *extra, "--out", out], False)
                assert not out.exists()
            else:
                image_stamp(source, "stamp-invalid-" + name, extra, success=False)
            cases.append("image-stamp-invalid-" + name + "-preserves-input")
        for name in ["transparent", "oversized-side", "oversized-pixels", "malformed"]:
            image_stamp(source, "stamp-invalid-image-" + name, image_path=root / (name + ".png"), success=False)
            cases.append("image-stamp-" + name + "-refuses-copy")
        saved_stamp = root / "image-stamp-0.pdf"
        digest = sha(saved_stamp)
        for destination in [saved_stamp, source]:
            run(["image-stamp", source, "--image", logo, "--rect", "40,40,240,240", "--out", destination, "--overwrite"], False)
        assert sha(saved_stamp) == digest
        cases.append("image-stamp-source-and-existing-destination-preserved")

        geometries = {"length": ("40,70 140,70", 10.0), "polyline": ("40,70 140,70 140,120", 15.0),
                      "perimeter": ("40,70 140,70 140,120 40,120", 30.0),
                      "area": ("40,70 140,70 140,120 40,120", 50.0), "angle": ("40,120 40,70 140,70", 90.0)}

        def measure(document, name, kind="length", points="40,70 140,70", calibration="40,40 140,40", known="10",
                    unit="m", extra=(), success=True, env=None):
            out = root / (name + ".pdf")
            result = run(["measure", document, "--kind", kind, "--points", points, "--calibration", calibration,
                          "--known-distance", known, "--unit", unit, "--out", out, *extra], success, env)
            assert out.exists() == success
            return out, result

        for rotation in [0, 90, 180, 270]:
            document = root / f"rotated-{rotation}.pdf"
            for kind, (points, expected) in geometries.items():
                out, result = measure(document, f"measure-{rotation}-{kind}", kind, points)
                assert result["original_unchanged"] and math.isclose(result["measurement"]["value"], expected, abs_tol=1e-8)
                exported = run(["measurements", out])["pages"]
                records = exported[0]["measurements"]
                assert len(records) == 1 and not exported[1]["measurements"]
                assert records[0]["kind"] == kind and math.isclose(records[0]["value"], expected, abs_tol=1e-8)
                assert records[0]["unit"] == ("deg" if kind == "angle" else "m2" if kind == "area" else "m")
            pixels = independent_render(out, f"measure-independent-{rotation}")
            assert sum(b > r + 70 and b > g + 50 for r, g, b in pixels.get_flattened_data()) > 60
            checked = subprocess.run([str(qpdf), "--check", str(out)], capture_output=True, timeout=25)
            assert checked.returncode == 0, checked.stderr[:1000]
            qpdf_checks += 1
            cases.append(f"measurements-rotation-{rotation}-five-known-geometries-reopen-qpdf-mupdf")
        for unit, metres in [("mm", 0.001), ("cm", 0.01), ("m", 1), ("in", 0.0254), ("ft", 0.3048)]:
            out, result = measure(source, "measure-unit-" + unit, known=str(10 / metres), unit=unit)
            assert math.isclose(result["measurement"]["value"] * metres, 10, abs_tol=1e-8)
        cases.append("measurement-metric-decimal-feet-and-inches-known-equivalence")
        for name, kwargs in [("zero-scale", {"known":"0"}), ("nan-scale", {"known":"NaN"}),
                             ("zero-calibration", {"calibration":"40,40 40,40"}),
                             ("outside", {"points":"40,70 9000,70"}), ("nonfinite", {"points":"NaN,70 140,70"}),
                             ("degenerate", {"points":"40,70 40,70"}),
                             ("crossed-area", {"kind":"area", "points":"40,70 140,120 40,120 140,70"}),
                             ("precision", {"extra":["--precision", "9"]})]:
            measure(source, "measure-invalid-" + name, success=False, **kwargs)
            cases.append("measurement-invalid-" + name + "-refuses-copy")
        saved_measure = root / "measure-unit-m.pdf"
        digest = sha(saved_measure)
        for destination in [saved_measure, source]:
            run(["measure", source, "--kind", "length", "--points", "40,70 140,70", "--calibration", "40,40 140,40",
                 "--known-distance", "10", "--out", destination, "--overwrite"], False)
        assert sha(saved_measure) == digest
        cases.append("measurement-source-and-existing-destination-preserved")
        for name, extra in [("overflow", ["--text", "W" * 256]), ("nan-margin", ["--text", "Text", "--margin", "NaN"]),
                            ("invalid-later-page", ["--text", "Text", "--pages", "1,3"])]:
            out = root / ("text-atomic-" + name + ".pdf")
            run(["stamp", source, *extra, "--out", out], False)
            assert not out.exists()
            cases.append("text-stamp-atomic-" + name)
        for aes in [128, 256]:
            for restricted in [False, True]:
                encrypted = root / f"annotation-protected-{aes}-{restricted}.pdf"
                run(["protect", source, "--user-password-env", "SUNIPDF_DECORATION_USER", "--owner-password-env", "SUNIPDF_DECORATION_OWNER",
                     "--out", encrypted, "--no-modify"] + (["--aes128"] if aes == 128 else [])
                    + (["--no-annotate"] if restricted else []), env=credentials)
                remember(encrypted)
                flags = ["--password-env", "SUNIPDF_DECORATION_USER"]
                out, result = image_stamp(encrypted, f"stamp-aes-{aes}-{restricted}", flags, success=not restricted, env=credentials)
                measured, report = measure(encrypted, f"measure-aes-{aes}-{restricted}", extra=flags, success=not restricted, env=credentials)
                if not restricted:
                    assert result["encrypted"] and report["encrypted"]
                    for role in ["USER", "OWNER"]:
                        assert len(run(["annotations", out, "--password-env", f"SUNIPDF_DECORATION_{role}"], env=credentials)["pages"][0]["annotations"]) == 1
                        assert run(["measurements", measured, "--password-env", f"SUNIPDF_DECORATION_{role}"], env=credentials)["pages"][0]["measurements"][0]["value"] == 10
                cases.append(f"image-stamp-and-measurement-aes{aes}-" + ("annotation-restriction-preserved" if restricted else "annotation-only-permission-both-credentials"))
        assert all(sha(path) == digest for path, digest in sources.items())
        assert len(cases) == len(set(cases))
    report = dict(gate="page-decorations-v1", status="passed", completed_cases=len(cases), cases=cases,
                  commands=commands, qpdf_checks=qpdf_checks, qpdf_sha256=sha(qpdf), pillow_version=pillow_version,
                  independent_reader_checks=independent_reader_checks, mutool_sha256=sha(mutool),
                  cli_sha256=sha(cli), source_revision=os.environ.get("GITHUB_SHA"), platform=os.name,
                  input_files_unchanged=True, generated_fixtures_only=True, secret_argv_used=False,
                  production_identity_used=False, host_installation_performed=False, native_gui_qualified=False,
                  elapsed_seconds=round(time.monotonic() - started, 3))
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: report[key] for key in ["gate", "status", "completed_cases", "commands", "qpdf_checks"]}))


if __name__ == "__main__":
    main()
