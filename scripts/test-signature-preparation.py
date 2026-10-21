"""Qualify the packaged visual-signature CLI with generated images and an independent decoder."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from PIL import Image, ImageOps, __version__ as pillow_version


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    qpdf = Path(os.environ["SUNIPDF_QPDF"]).resolve(strict=True)
    assert sha(qpdf) == "1bdaf878f37eaffea1ea3c30591214c77636a0f2fc7432427516b6e22e36c0ff"
    assert pillow_version == "13.0.0", pillow_version
    started = time.monotonic()
    commands = 0
    cases = []
    sources = {}
    with tempfile.TemporaryDirectory(prefix="sunipdf-visual-signature-") as directory:
        root = Path(directory)

        def remember(path):
            sources[path] = sha(path)
            return path

        def run(arguments, code=0):
            nonlocal commands
            output = subprocess.run([str(cli), *map(str, arguments)], cwd=root,
                                    capture_output=True, timeout=25)
            commands += 1
            assert output.returncode == code, (arguments[0], code, output.returncode, output.stderr[:1000])
            assert len(output.stdout) < 2 * 1024 * 1024 and len(output.stderr) < 2 * 1024 * 1024
            if code:
                assert not output.stdout
                error = json.loads(output.stderr)
                assert error["error"] and error["message"]
                return error
            return json.loads(output.stdout)

        def prepare(source, name, mode="keep", code=0, extra=()):
            out = root / (name + ".png")
            result = run(["prepare-signature-image", source, "--out", out,
                          "--background", mode, *extra], code)
            if code:
                assert not out.exists()
            else:
                assert result["input_written"] is False and result["cryptographic_signature"] is False
                assert result["background"] == mode
                with Image.open(out) as image:
                    assert image.format == "PNG" and image.mode == "RGBA"
                    assert image.size == (result["width"], result["height"])
            return out

        rgba = Image.new("RGBA", (32, 20), (20, 50, 100, 120))
        rgba.putpixel((10, 10), (100, 20, 80, 220))
        input_rgba = root / "source été 中文.png"
        rgba.save(input_rgba)
        remember(input_rgba)
        kept = prepare(input_rgba, "kept")
        with Image.open(kept) as image:
            assert image.size == rgba.size and image.tobytes() == rgba.tobytes()
        cases.append("keep-preserves-visible-rgba")

        gradient = Image.new("RGBA", (240, 120))
        for y in range(120):
            for x in range(240):
                gray = 120 + x // 2
                gradient.putpixel((x, y), (gray, gray, gray, 255))
        for y in range(59, 62):
            for x in range(20, 220):
                gradient.putpixel((x, y), (20, 40, 100, 110 if x % 2 else 255))
        gradient_path = root / "generated uneven paper.png"
        gradient.save(gradient_path)
        remember(gradient_path)
        adaptive = prepare(gradient_path, "adaptive", "adaptive")
        with Image.open(adaptive) as image:
            assert image.size == (212, 15), image.size
            assert image.getpixel((0, 0)) == (0, 0, 0, 0)
            assert image.getpixel((6, 6)) == (20, 40, 100, 255)
            assert image.getpixel((7, 6)) == (20, 40, 100, 110)
        cases.append("adaptive-removes-gradient-preserves-ink-and-alpha")
        threshold = prepare(gradient_path, "threshold", "threshold")
        with Image.open(threshold) as image:
            assert image.getpixel((10, 10))[3] == 255
        cases.append("global-threshold-remains-distinct")

        white = root / "white.png"
        Image.new("RGB", (20, 20), "white").save(white)
        remember(white)
        prepare(white, "blank", "adaptive", code=1)
        cases.append("blank-cleanup-refuses-empty-signature")
        run(["prepare-signature-image", input_rgba, "--out", input_rgba, "--overwrite"], 1)
        assert sha(input_rgba) == sources[input_rgba]
        cases.append("same-input-output-refused-even-with-overwrite")
        before = sha(kept)
        run(["prepare-signature-image", gradient_path, "--out", kept], 6)
        assert sha(kept) == before
        run(["prepare-signature-image", input_rgba, "--out", kept, "--overwrite"])
        assert sha(kept) == before
        cases.append("existing-output-requires-explicit-overwrite")
        bad_extension = root / "wrong.jpg"
        run(["prepare-signature-image", input_rgba, "--out", bad_extension], 1)
        assert not bad_extension.exists()
        cases.append("png-output-required")
        wide = root / "oversized-side.png"
        Image.new("RGB", (20000, 1), "black").save(wide)
        remember(wide)
        prepare(wide, "wide", code=1)
        cases.append("decoder-side-bound")
        large = root / "oversized-file.png"
        with large.open("wb") as file:
            file.truncate(64 * 1024 * 1024 + 1)
        remember(large)
        prepare(large, "large", code=1)
        cases.append("encoded-file-size-bound")
        malformed = root / "malformed.png"
        malformed.write_bytes(b"This is a generated non-image fixture.")
        remember(malformed)
        prepare(malformed, "malformed-result", code=1)
        cases.append("malformed-image-typed-failure")

        photo = Image.new("RGB", (48, 32))
        for y in range(32):
            for x in range(48):
                photo.putpixel((x, y), (20 + x * 4, 30 + y * 5, 60 + x + y))
        for orientation in range(1, 9):
            source = root / f"orientation-{orientation}.jpg"
            exif = Image.Exif()
            exif[274] = orientation
            photo.save(source, quality=100, subsampling=0, exif=exif)
            remember(source)
            with Image.open(source) as original:
                reference = ImageOps.exif_transpose(original).convert("RGBA")
            out = prepare(source, f"oriented-{orientation}")
            with Image.open(out) as actual:
                assert actual.size == reference.size, (orientation, actual.size, reference.size)
                maximum = max(abs(a - b) for a, b in zip(actual.tobytes(), reference.tobytes()))
                assert maximum <= 3, (orientation, maximum)
            cases.append(f"exif-orientation-{orientation}")

        fixture = Path(__file__).resolve().parents[1] / "crates/sunipdf-core/tests/fixtures/two-pages.pdf"
        pdf = root / "original document.pdf"
        pdf.write_bytes(fixture.read_bytes())
        remember(pdf)
        placed = root / "visual signature placed.pdf"
        run(["add-image", pdf, "--image", adaptive, "--x", 100, "--y", 300,
             "--width", 212, "--out", placed])
        image_path = root / "rendered.png"
        run(["render", placed, "--page", 1, "--scale", 1, "--out", image_path])
        with Image.open(image_path) as rendered:
            pixels = rendered.convert("RGB").get_flattened_data()
            assert sum(1 for red, green, blue in pixels if blue > red + 20 and blue > green + 20) >= 100
        checked = subprocess.run([str(qpdf), "--check", str(placed)], capture_output=True, timeout=25)
        assert checked.returncode == 0, checked.stderr[:1000]
        cases.append("prepared-alpha-embeds-renders-and-passes-qpdf")
        prepare(root / "missing.png", "missing-result", code=5)
        cases.append("missing-input-typed-failure")
        assert len(cases) == len(set(cases)) == 20
        assert all(sha(path) == digest for path, digest in sources.items())
    report = dict(gate="visual-signature-0.35.0", status="passed", completed_cases=len(cases),
                  cases=cases, commands=commands, qpdf_checks=1, qpdf_sha256=sha(qpdf),
                  pillow_version=pillow_version, cli_sha256=sha(cli),
                  source_revision=os.environ.get("GITHUB_SHA"), platform=os.name,
                  input_files_unchanged=True, generated_fixtures_only=True,
                  production_identity_used=False, host_installation_performed=False,
                  native_gui_qualified=False, elapsed_seconds=round(time.monotonic() - started, 3))
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: report[key] for key in ["gate", "status", "completed_cases", "commands", "qpdf_checks"]}))


if __name__ == "__main__":
    main()
