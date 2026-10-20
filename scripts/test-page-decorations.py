"""Exercise page decorations in the actual CLI with independent fixtures and pixels."""

import argparse
import hashlib
import json
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
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    qpdf = Path(os.environ["SUNIPDF_QPDF"]).resolve(strict=True)
    assert sha(qpdf) == "1bdaf878f37eaffea1ea3c30591214c77636a0f2fc7432427516b6e22e36c0ff"
    assert pillow_version == "12.3.0", pillow_version
    started = time.monotonic()
    commands = 0
    cases = []
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
        assert all(sha(path) == digest for path, digest in sources.items())
        assert len(cases) == len(set(cases)) == 24
    report = dict(gate="page-decorations-v1", status="passed", completed_cases=len(cases), cases=cases,
                  commands=commands, qpdf_checks=1, qpdf_sha256=sha(qpdf), pillow_version=pillow_version,
                  cli_sha256=sha(cli), source_revision=os.environ.get("GITHUB_SHA"), platform=os.name,
                  input_files_unchanged=True, generated_fixtures_only=True, secret_argv_used=False,
                  production_identity_used=False, host_installation_performed=False, native_gui_qualified=False,
                  elapsed_seconds=round(time.monotonic() - started, 3))
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: report[key] for key in ["gate", "status", "completed_cases", "commands", "qpdf_checks"]}))


if __name__ == "__main__":
    main()
