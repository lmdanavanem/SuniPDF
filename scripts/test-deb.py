#!/usr/bin/env python3
"""Validate the installed .deb in a clean Ubuntu container, including its native UI."""
import argparse
import csv
import hashlib
import io
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
ACCOUNT = "sunipdf-test"
HOME = Path("/home") / ACCOUNT
CLI = "/usr/bin/sunipdf-cli"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(arguments, timeout=120, **kwargs):
    result = subprocess.run([str(value) for value in arguments], text=True, capture_output=True, timeout=timeout, **kwargs)
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}): {arguments}\n{result.stdout[-4000:]}\n{result.stderr[-4000:]}")
    return result.stdout.strip()


def blue_fixture_mask(pixels):
    """Keep only the fixture's saturated blue ink, as black on white for OCR."""
    mask = bytes(0 if b > 180 and r < 70 and g < 70 else 255 for r, g, b in pixels)
    return mask, mask.count(0)


def fixture_text_is_visible(blue_pixels, displayed):
    return blue_pixels > 2000 and re.search(r"\bSuniPDF\s+page\s+[1Il]\b", displayed, re.I) is not None


def distribution_identity():
    os_release = Path("/etc/os-release").read_text()
    distro = dict(line.split("=", 1) for line in os_release.splitlines() if "=" in line)
    return {"distro": {"id": distro["ID"].strip('"'), "version_id": distro["VERSION_ID"].strip('"')},
            "osRelease": os_release}


def verify_installed_inventory():
    path = Path("/usr/share/doc/sunipdf/licenses/dependency-inventory.json")
    inventory = json.loads(path.read_text())
    if inventory.get("platform") != "linux" or inventory.get("architecture") != "amd64":
        raise AssertionError("Installed license inventory does not describe Linux amd64")
    for relative, expected in inventory["payloadFiles"].items():
        installed = Path("/") / relative
        if not relative.startswith("usr/") or ".." in Path(relative).parts or installed.is_symlink():
            raise AssertionError("Unexpected installed inventory path")
        if not installed.is_file() or sha(installed) != expected["sha256"] or installed.stat().st_size != expected["bytes"]:
            raise AssertionError("Installed package differs from its inventory: " + relative)
    return len(inventory["payloadFiles"])


def runtime(evidence, version):
    from PIL import Image, ImageDraw, ImageFont
    evidence.mkdir(parents=True, exist_ok=True)
    source = HOME / "Documents/Ubuntu PDF test.pdf"
    source.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / "crates/sunipdf-core/tests/fixtures/two-pages.pdf", source)
    original = sha(source)
    cases = []
    assert version in command([CLI, "--version"])
    assert json.loads(command([CLI, "info", source]))["page_count"] == 2
    assert "SuniPDF page 1" in command([CLI, "text", source, "--page", 1])
    cases.append("installed-cli-opens-and-extracts-text")
    rendered = evidence / "page-1.png"
    command([CLI, "render", source, "--page", 1, "--scale", 1, "--out", rendered])
    with Image.open(rendered) as image:
        assert image.size == (612, 792), image.size
        assert sum(1 for r, g, b in image.convert("RGB").getdata() if b > 180 and r < 70 and g < 70) > 20000
    cases.append("installed-cli-loads-private-pdfium-and-renders-fixture")
    edited = evidence / "reordered.pdf"
    command([CLI, "extract", source, "--pages", "2,1", "--out", edited])
    command(["qpdf", "--check", edited])
    assert json.loads(command([CLI, "info", edited]))["page_count"] == 2
    assert "SuniPDF page 2" in command([CLI, "text", edited, "--page", 1])
    cases.append("save-reopen-reordered-pages-and-independent-qpdf")
    font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 68)
    for language, words in (("eng", "English document local"), ("fra", "Bonjour document français")):
        raster = evidence / (language + "-scan.png")
        scan = Image.new("RGB", (1600, 400), "white")
        ImageDraw.Draw(scan).text((60, 130), words, font=font, fill="black")
        scan.save(raster)
        pdf = evidence / (language + "-scan.pdf")
        output = evidence / (language + "-searchable.pdf")
        command([CLI, "images2pdf", raster, "--out", pdf])
        assert not command([CLI, "text", pdf]).strip()
        report = json.loads(command([CLI, "ocr", pdf, "--language", language, "--out", output], timeout=180))
        assert report["recognized_pages"] == 1, report
        text = command([CLI, "text", output]).lower()
        assert "document" in text and ("english" if language == "eng" else "bonjour") in text, text
        command(["qpdf", "--check", output])
        cases.append("offline-ocr-" + language + "-saved-searchable-layer")
    assert sha(source) == original
    cases.extend(gui(evidence, source))
    cases.append("native-ui-open-pdf-visible-text-blue-fixture-and-private-library")
    cases.extend(plan_link_navigation(evidence, version))
    cases.extend(plan_link_creation(evidence, version))
    assert sha(source) == original
    settings = HOME / ".config/sunipdf/settings.json"
    assert settings.is_file(), "Native application did not save user settings on close"
    proof = {"schema": 1, "version": version, "cases": cases, "sourceSha256": original,
             "settingsSha256": sha(settings), "source": str(source), "settings": str(settings),
             "tesseract": command(["tesseract", "--version"]), "qpdf": command(["qpdf", "--version"])}
    proof["nativePlanLinks"] = {"file": "native-plan-links.json", "sha256": sha(evidence / "native-plan-links.json")}
    proof["nativePlanCreation"] = {"file": "native-plan-creation.json", "sha256": sha(evidence / "native-plan-creation.json")}
    (evidence / "runtime.json").write_text(json.dumps(proof, indent=2) + "\n")


def gui(evidence, source):
    from PIL import Image
    manager = subprocess.Popen(["openbox"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    with (evidence / "desktop.log").open("w") as log:
        app = subprocess.Popen(["/usr/bin/sunipdf", str(source)], stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 75
            success = False
            screenshot = evidence / "native-reader.png"
            masked_screenshot = evidence / "native-reader-blue-mask.png"
            while time.monotonic() < deadline:
                if app.poll() is not None:
                    raise AssertionError("Installed desktop exited before rendering a PDF")
                time.sleep(2)
                maps = Path(f"/proc/{app.pid}/maps").read_text()
                if "/usr/lib/sunipdf/libpdfium.so" not in maps:
                    continue
                command(["import", "-window", "root", screenshot], timeout=15)
                with Image.open(screenshot) as image:
                    pixels, blue = blue_fixture_mask(image.convert("RGB").getdata())
                    Image.frombytes("L", image.size, pixels).save(masked_screenshot)
                displayed = command(["tesseract", masked_screenshot, "stdout", "-l", "eng", "--psm", 11], timeout=30)
                (evidence / "native-reader-text.txt").write_text(displayed + "\n")
                if fixture_text_is_visible(blue, displayed):
                    (evidence / "native-reader-proof.json").write_text(json.dumps({"privatePdfiumLoaded": True, "blueFixturePixels": blue, "visiblePdfText": "SuniPDF page 1", "renderer": "glow", "screenshotSha256": sha(screenshot), "ocrMaskSha256": sha(masked_screenshot), "ocrMask": "saturated-blue-pixels-only"}, indent=2) + "\n")
                    success = True
                    break
            assert success, "Native screenshot did not contain the fixture's PDF text and blue rectangle"
            window = command(["xdotool", "search", "--onlyvisible", "--pid", app.pid]).splitlines()[-1]
            views = reader_views(app, window, evidence)
            themes = theme_settings(app, window, evidence)
            controls = window_controls(app, window, evidence)
            app = subprocess.Popen(["/usr/bin/sunipdf", str(source)], stdout=log, stderr=subprocess.STDOUT)
            restart_theme(app, evidence)
            return views + themes + controls + ["native-theme-persists-after-restart"]
        finally:
            if app.poll() is None:
                app.terminate()
                try:
                    app.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    app.kill()
                    app.wait()
            manager.terminate()
            manager.wait(timeout=10)


def colored_fixture_mask(pixels, color):
    """Isolate actual fixture ink; ordinary interface text cannot satisfy PDF evidence."""
    assert color in ("blue", "red")
    def matches(pixel):
        r, g, b = pixel
        # Keep antialiased fixture letters at fit-page zoom while excluding neutral
        # interface text. Saturated-core-only thresholds remove most small glyphs.
        return (b > 180 and b - r > 70 and b - g > 70) if color == "blue" else (r > 180 and r - g > 70 and r - b > 70)
    mask = bytes(0 if matches(pixel) else 255 for pixel in pixels)
    return mask, mask.count(0)


def native_error_mask(pixels):
    """Keep red UI error ink, including the darker accessible palette colour."""
    mask = bytes(0 if r >= 128 and r - g > 45 and r - b > 35 else 255 for r, g, b in pixels)
    return mask, mask.count(0)


def reader_fixture_visible(color, count, text):
    page = r"[1Il]" if color == "blue" else "2"
    # A closed split must contain exactly one fixture title, not two synchronized
    # panes that would also satisfy a simple substring check.
    return count > 500 and len(re.findall(r"\bSuniPDF\s+page\s+" + page + r"\b", text, re.I)) == 1


def reader_views(app, window, evidence):
    """Operate the installed two-pane reader through its public command palette."""
    from PIL import Image
    records = []

    def palette(label):
        assert app.poll() is None, "Reader exited during split interaction"
        command(["xdotool", "windowactivate", "--sync", window, "key", "--clearmodifiers", "ctrl+k"])
        time.sleep(0.25)
        command(["xdotool", "type", "--clearmodifiers", "--delay", 35, label])
        command(["xdotool", "key", "--clearmodifiers", "Return"])
        time.sleep(0.5)

    def capture(name, expected):
        path = evidence / (name + ".png")
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            assert app.poll() is None, "Reader exited before both panes rendered"
            command(["import", "-window", window, path], timeout=15)
            checks = []
            with Image.open(path) as image:
                rgb = image.convert("RGB")
                for side, color in expected:
                    bounds = (0, 0, rgb.width // 2, rgb.height) if side == "left" else (
                        rgb.width // 2, 0, rgb.width, rgb.height) if side == "right" else (0, 0, rgb.width, rgb.height)
                    crop = rgb.crop(bounds)
                    mask, count = colored_fixture_mask(crop.getdata(), color)
                    mask_path = evidence / f"{name}-{side}-{color}-mask.png"
                    Image.frombytes("L", crop.size, mask).resize((crop.width * 3, crop.height * 3), resample=Image.NEAREST).save(mask_path)
                    displayed = command(["tesseract", mask_path, "stdout", "-l", "eng", "--psm", 11], timeout=20)
                    checks.append({"side": side, "color": color, "inkPixels": count,
                                   "pdfText": displayed, "passed": reader_fixture_visible(color, count, displayed),
                                   "maskSha256": sha(mask_path)})
            if all(check["passed"] for check in checks):
                records.append({"scene": name, "pngSha256": sha(path), "checks": checks})
                return
            time.sleep(0.5)
        raise AssertionError(f"Split PDF ink/text missing in {name}: {checks}")

    # Hide the navigation dock so thumbnail ink cannot satisfy a reader-pane check.
    palette("Thumbnails")
    settings_path = HOME / ".config/sunipdf/settings.json"
    if json.loads(settings_path.read_text()).get("show_thumbnails"):
        palette("Thumbnails")
    settings = json.loads(settings_path.read_text())
    assert not settings.get("show_thumbnails") and not settings.get("show_tools"), "Navigation dock remains visible"
    palette("Single Page")
    palette("Fit Page")
    palette("Split document view")
    palette("Switch active view")
    palette("Next Page")
    palette("Fit Page")
    capture("native-reader-split-independent", [("left", "blue"), ("right", "red")])
    palette("Synchronize views")
    capture("native-reader-split-synchronized", [("left", "red"), ("right", "red")])
    palette("Split document view")
    palette("Previous Page")
    palette("Fit Page")
    capture("native-reader-split-closed", [("whole", "blue")])
    assert "*" not in command(["xdotool", "getwindowname", window]), "Reader operations dirtied the PDF"
    palette("Thumbnails")
    (evidence / "native-reader-views.json").write_text(json.dumps({
        "status": "passed", "scenes": records, "documentStillOpen": True,
        "method": "Installed application command palette, X11 captures and color-isolated PDF text OCR",
        "limits": "Same two-page fixture; no physical GPU, Wayland, pointer editing or human accessibility qualification"
    }, indent=2) + "\n", encoding="utf-8")
    return ["native-reader-independent-pages", "native-reader-synchronize-pages", "native-reader-close-split-keeps-document"]


def plan_fixture_visible(kind, pixels, text):
    """Both PDF colour and a unique full title must agree; UI labels cannot qualify a page."""
    titles = {"source": r"Native\s+source\s+document", "destination": r"Native\s+destination\s+two"}
    return pixels > 500 and len(re.findall(r"\b" + titles[kind] + r"\b", text, re.I)) == 1


def plan_error_visible(kind, text):
    patterns = {
        "changed-target": r"\b(?:fingerprint\s+changed|destination\s+has\s+changed|source\s+has\s+changed)\b",
        "missing-target": r"\b(?:no\s+such\s+file|not\s+found|file\s+missing|file\s+is\s+missing)\b",
        # The captured native error font can make OCR confuse PDF with POF.
        "changed-source": r"\bsource\s+P[DO]F\s+changed\s+since\s+the\s+preview\b",
    }
    return re.search(patterns[kind], text, re.I) is not None


def plan_source_location(tsv, image_size, scale):
    """Observe two separate source-PDF anchors, never sidebar or page-number labels."""
    assert scale > 0
    return {label: [coordinate / scale for coordinate in theme_label_center(tsv, label, image_size)]
            for label in ("Native source document", "A-101")}


def plan_location_restored(before, after):
    """A small raster tolerance permits antialiasing, not a different zoom or scroll."""
    labels = {"Native source document", "A-101"}
    return (set(before) == labels and set(after) == labels
            and all(len(before[label]) == len(after[label]) == 2
                    and all(abs(old - new) <= 3 for old, new in zip(before[label], after[label]))
                    for label in labels))


def plan_error_is_visible(kind, pixels, text):
    return pixels >= 20 and plan_error_visible(kind, text)


def plan_link_navigation(evidence, version):
    """Qualify native navigation only; the linked PDF is prepared by the installed CLI."""
    from PIL import Image
    spec = importlib.util.spec_from_file_location("native_plan_link_fixture", ROOT / "scripts/native-plan-link-fixture.py")
    fixtures = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixtures)
    fixture = fixtures.prepare(HOME / "Documents/Native plan navigation", evidence, command, CLI)
    profile = HOME / "native-plan-navigation-profile"
    settings = profile / ".config/sunipdf/settings.json"
    settings.parent.mkdir(parents=True, exist_ok=False)
    settings.write_text(json.dumps({"language": "en-US", "dark_theme": False, "theme_mode": "light",
                                    "show_tools": False, "show_thumbnails": False, "continuous_view": False,
                                    "view_mode": "single", "autosave": False, "check_updates": False}), encoding="utf-8")
    environment = dict(os.environ, HOME=str(profile), XDG_CONFIG_HOME=str(profile / ".config"),
                       XDG_DATA_HOME=str(profile / ".local/share"))
    controls = evidence / "plan-navigation-controls"
    controls.mkdir(exist_ok=True)
    cases, scenes, actions = [], [], []
    app, manager, window = None, None, None
    report = {"schema": 1, "status": "failed", "version": version,
              "sourceSha": os.environ.get("GITHUB_SHA"), "fixtureReport": fixture["fixtureReport"],
              "fixtureReportSha256": sha(evidence / fixture["fixtureReport"]),
              "syntheticOnly": True, "isolatedProfile": True, "cases": cases, "scenes": scenes,
              "actions": actions, "protectedFiles": fixture["protected"],
              "desktopLinkCreationTested": False, "desktopLinkSavingTested": False,
              "method": "Installed application, actual X11 pointer/keyboard input, native captures and colour-isolated PDF text OCR",
              "limits": "Synthetic fixture, X11/Openbox/Xvfb and software rendering. No batch link creation, physical GPU, Wayland, dirty destination tab or human accessibility qualification."}

    def alive():
        assert app is not None and app.poll() is None, "Native plan-link application exited unexpectedly"

    def key(*keys):
        alive()
        command(["xdotool", "windowactivate", "--sync", window, "key", "--clearmodifiers", *keys])

    def title():
        return command(["xdotool", "getwindowname", window])

    def capture(path):
        alive()
        command(["import", "-window", window, path], timeout=15)
        return path

    def ocr(path, color=None):
        with Image.open(path) as image:
            if color is None:
                ocr_path, ink, scale = path, 0, 1
            else:
                rgb = image.convert("RGB").getdata()
                mask, ink = native_error_mask(rgb) if color == "error-red" else colored_fixture_mask(rgb, color)
                ocr_path, scale = path.with_name(path.stem + "-" + color + "-mask.png"), 3
                Image.frombytes("L", image.size, mask).resize((image.width * scale, image.height * scale),
                                                            resample=Image.NEAREST).save(ocr_path)
            size = (image.width * scale, image.height * scale)
        text = command(["tesseract", ocr_path, "stdout", "-l", "eng", "--psm", 11], timeout=20)
        return text, ink, ocr_path, scale, size

    def click_label(label, color=None):
        path = capture(controls / f"control-{len(actions) + 1:02d}.png")
        deadline = time.monotonic() + 15
        while True:
            _, _, ocr_path, scale, size = ocr(path, color)
            tsv = command(["tesseract", ocr_path, "stdout", "-l", "eng", "--psm", 11, "tsv"], timeout=20)
            try:
                x, y = theme_label_center(tsv, label, size)
                break
            except ThemeLabelMissing:
                if time.monotonic() >= deadline:
                    raise
                time.sleep(0.25)
                capture(path)
        tsv_path = path.with_suffix(".tsv")
        tsv_path.write_text(tsv, encoding="utf-8")
        x, y = x // scale, y // scale
        actions.append({"label": label, "point": [x, y], "color": color,
                        "capture": str(path.relative_to(evidence)), "captureSha256": sha(path),
                        "ocr": str(tsv_path.relative_to(evidence)), "ocrSha256": sha(tsv_path)})
        command(["xdotool", "mousemove", "--window", window, x, y, "click", "1"])

    def wait_scene(name, *, kind=None, text_pattern=None, error_kind=None, expected_title=None,
                   observe_source_location=False, expected_source_location=None, different_source_location=None):
        path = evidence / (name + ".png")
        deadline = time.monotonic() + 30
        last = ""
        while time.monotonic() < deadline:
            capture(path)
            last, ink, mask, scale, size = ocr(path, "blue" if kind == "source" else "red" if kind else "error-red" if error_kind else None)
            matched = (plan_fixture_visible(kind, ink, last) if kind else
                       plan_error_is_visible(error_kind, ink, last) if error_kind else
                       re.search(text_pattern, last, re.I) is not None)
            caption = title()
            location = None
            if matched and (observe_source_location or expected_source_location is not None or different_source_location is not None):
                assert kind == "source", "Reading-position evidence must come from source PDF ink"
                tsv = command(["tesseract", mask, "stdout", "-l", "eng", "--psm", 11, "tsv"], timeout=20)
                try:
                    location = plan_source_location(tsv, size, scale)
                except ThemeLabelMissing:
                    matched = False
                if expected_source_location is not None:
                    matched = matched and plan_location_restored(expected_source_location, location)
                if different_source_location is not None:
                    matched = matched and not plan_location_restored(different_source_location, location)
            if matched and (expected_title is None or expected_title in caption):
                text_path = path.with_suffix(".txt")
                text_path.write_text(last + "\n", encoding="utf-8")
                scene = {"name": name, "png": path.name, "pngSha256": sha(path),
                               "windowTitle": caption, "text": last, "textSha256": sha(text_path),
                               "pdfKind": kind, "pdfInkPixels": ink if kind else 0, "errorKind": error_kind,
                               "errorInkPixels": ink if error_kind else 0,
                               "ocrContent": "pdf-fixture" if kind else "native-error-text" if error_kind else "interface",
                               "ocrMask": mask.name if kind or error_kind else None,
                               "ocrMaskSha256": sha(mask) if kind or error_kind else None}
                if location is not None:
                    tsv_path = path.with_suffix(".tsv")
                    tsv_path.write_text(tsv, encoding="utf-8")
                    scene.update(sourceLocation=location, sourceLocationOcr=tsv_path.name,
                                 sourceLocationOcrSha256=sha(tsv_path), locationTolerancePixels=3)
                scenes.append(scene)
                return scene
            time.sleep(0.25)
        raise AssertionError(f"Native plan-link scene {name} did not qualify: {last}")

    def open_preview(name, load=False):
        click_label("A-101", "blue")
        wait_scene(name + "-confirmation", text_pattern=r"Manifest\s+path", expected_title=fixture["linked"].name)
        if load:
            # The dialog focuses its labelled path field; use the actual keyboard route.
            # Placeholder OCR is unreliable for the deliberately subdued hint text.
            key("ctrl+a")
            command(["xdotool", "type", "--clearmodifiers", "--delay", "1", str(fixture["manifest"])])
            click_label("Load manifest")
            wait_scene(name + "-manifest", text_pattern=r"Native\s+navigation\s+fixture",
                       expected_title=fixture["linked"].name)
        click_label("Preview destination")
        wait_scene(name + "-preview", kind="destination", expected_title=fixture["linked"].name)

    try:
        manager = subprocess.Popen(["openbox"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        with (evidence / "native-plan-links-desktop.log").open("w") as log:
            app = subprocess.Popen(["/usr/bin/sunipdf", str(fixture["linked"])], env=environment,
                                   stdout=log, stderr=subprocess.STDOUT)
            deadline = time.monotonic() + 75
            while time.monotonic() < deadline:
                alive()
                found = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(app.pid)],
                                       text=True, capture_output=True, timeout=10)
                if found.returncode == 0 and found.stdout.strip():
                    window = found.stdout.splitlines()[-1]
                    break
                time.sleep(0.25)
            assert window is not None, "Native plan-link window was not created"
            command(["xdotool", "windowsize", window, "1200", "900", "windowmove", window, "40", "40"])
            wait_scene("native-plan-source-initial", kind="source", expected_title=fixture["linked"].name)
            key("ctrl+k")
            time.sleep(0.25)
            command(["xdotool", "type", "--clearmodifiers", "--delay", "35", "Fit Page"])
            key("Return")
            fitted = wait_scene("native-plan-source-fit-page", kind="source", expected_title=fixture["linked"].name,
                                observe_source_location=True)
            stored = json.loads(settings.read_text(encoding="utf-8"))
            assert not stored["show_thumbnails"] and not stored["show_tools"], "Navigation ink could contaminate plan-link evidence"
            open_preview("native-plan-cancel", load=True)
            key("Escape")
            wait_scene("native-plan-cancelled-source", kind="source", expected_title=fixture["linked"].name)
            assert "*" not in title(), "Cancelling a preview dirtied the source"
            assert all(sha(Path(path)) == expected for path, expected in fixture["protected"].items())
            cases.append("native-plan-link-preview-cancel-preserves-source")

            for failure in ("changed-target", "missing-target", "changed-source"):
                open_preview("native-plan-" + failure)
                changed = fixture["linked"] if failure == "changed-source" else fixture["target"]
                saved = changed.read_bytes()
                held = changed.with_name(changed.name + ".held-for-native-test")
                try:
                    if failure == "missing-target":
                        assert not held.exists()
                        changed.rename(held)
                    else:
                        changed.write_bytes(saved + b"\n% deliberate synthetic native navigation change\n")
                    click_label("Open verified destination")
                    wait_scene("native-plan-" + failure + "-refused", error_kind=failure,
                               expected_title=fixture["linked"].name)
                    key("Escape")
                    wait_scene("native-plan-" + failure + "-source", kind="source", expected_title=fixture["linked"].name)
                    assert "*" not in title(), "Refusing a changed destination dirtied the source"
                finally:
                    if failure == "missing-target":
                        if held.exists():
                            assert not changed.exists(), "Missing fixture unexpectedly reappeared"
                            held.rename(changed)
                    else:
                        changed.write_bytes(saved)
                assert all(sha(Path(path)) == expected for path, expected in fixture["protected"].items())
                cases.append("native-plan-link-" + failure + "-after-preview-refused")

            key("ctrl+k")
            time.sleep(0.25)
            command(["xdotool", "type", "--clearmodifiers", "--delay", "35", "Zoom In"])
            key("Return")
            departure = wait_scene("native-plan-source-before-departure", kind="source",
                                   expected_title=fixture["linked"].name,
                                   different_source_location=fitted["sourceLocation"])
            open_preview("native-plan-open")
            click_label("Open verified destination")
            wait_scene("native-plan-opened-physical-page-two", kind="destination", expected_title=fixture["target"].name)
            assert "*" not in title(), "Native navigation dirtied the destination"
            cases.append("native-plan-link-open-verified-physical-page-two")
            click_label("Return to source")
            returned = wait_scene("native-plan-returned-source-location", kind="source",
                                  expected_title=fixture["linked"].name,
                                  expected_source_location=departure["sourceLocation"])
            assert "*" not in title(), "Returning to the plan source dirtied the document"
            # Move away from the former button so its hover text cannot outlive it.
            x, y = returned["sourceLocation"]["Native source document"]
            command(["xdotool", "mousemove", "--window", window, round(x), round(y)])
            consumed = wait_scene("native-plan-return-control-consumed",
                                  text_pattern=r"(?s)\A(?=.*\bMenu\b)(?=.*\bTools\b)(?!.*\bReturn\s+to\s+source\b)",
                                  expected_title=fixture["linked"].name)
            report["returnToSource"] = {"departureScene": departure["name"], "returnedScene": returned["name"],
                                        "sameSourcePdfAndReadingPosition": True, "twoPaneRestoreTested": False,
                                        "consumedControlScene": consumed["name"], "returnControlHiddenAfterReturn": True,
                                        "repeatedReturnTested": False}
            cases.append("native-plan-link-return-to-source-restores-reading-position")
            assert all(sha(Path(path)) == expected for path, expected in fixture["protected"].items())
            report["status"] = "passed"
            report["fixturesUnchanged"] = True
    finally:
        try:
            for process in (app, manager):
                if process is not None and process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=10)
        finally:
            (evidence / "native-plan-links.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return cases


def plan_link_creation(evidence, version):
    """Create and save two reviewed link copies using the installed native UI."""
    from PIL import Image
    spec = importlib.util.spec_from_file_location("native_plan_creation_fixture", ROOT / "scripts/native-plan-creation-fixture.py")
    fixtures = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixtures)
    fixture = fixtures.prepare(HOME / "Documents/Native plan creation", evidence, command, CLI)
    profile = HOME / "native-plan-creation-profile"
    settings = profile / ".config/sunipdf/settings.json"
    settings.parent.mkdir(parents=True, exist_ok=False)
    settings.write_text(json.dumps({"language": "en-US", "dark_theme": False, "theme_mode": "light",
                                    "show_tools": False, "show_thumbnails": False, "continuous_view": False,
                                    "view_mode": "single", "autosave": False, "check_updates": False}), encoding="utf-8")
    environment = dict(os.environ, HOME=str(profile), XDG_CONFIG_HOME=str(profile / ".config"),
                       XDG_DATA_HOME=str(profile / ".local/share"))
    controls = evidence / "plan-creation-controls"
    controls.mkdir(exist_ok=False)
    scene_directory = evidence / "native-plan-creation-scenes"
    scene_directory.mkdir(exist_ok=False)
    cases, scenes, actions = [], [], []
    app, manager, window = None, None, None
    capture_index = 0
    report = {"schema": 1, "status": "failed", "version": version, "sourceSha": os.environ.get("GITHUB_SHA"),
              "fixtureReport": fixture["fixtureReport"], "fixtureReportSha256": sha(evidence / fixture["fixtureReport"]),
              "syntheticOnly": True, "isolatedProfile": True, "cases": cases, "scenes": scenes, "actions": actions,
              "protectedFiles": dict(fixture["protected"]), "desktopLinkCreationTested": False,
              "desktopLinkSavingTested": False, "sourceCount": 2, "expectedPhysicalDestinationPage": 2,
              "method": "Installed desktop, actual X11/OCR controls, independent QPDF object inspection and exported save receipt",
              "limits": "Synthetic searchable PDFs, X11/Openbox/Xvfb and software rendering. No image-only reference recognition, physical GPU, Wayland, encrypted inputs, dirty tabs or external-reader interoperability qualification."}

    def alive():
        assert app is not None and app.poll() is None, "Native plan creation application exited"

    def key(*keys, target=None):
        alive()
        command(["xdotool", "windowactivate", "--sync", target or window, "key", "--clearmodifiers", *keys])

    def type_text(value, target=None):
        key("ctrl+a", target=target)
        command(["xdotool", "type", "--clearmodifiers", "--delay", "1", str(value)])
        actions.append({"keyboardText": str(value), "window": target or window})

    def capture(target=None):
        nonlocal capture_index
        alive()
        capture_index += 1
        path = controls / f"capture-{capture_index:04d}.png"
        command(["import", "-window", target or window, path], timeout=15)
        return path

    def read_capture(path, color=None, *, preparation="full-rgb-nearest-2x-psm11"):
        methods = {"full-rgb-nearest-2x-psm11": (2, Image.NEAREST, 11),
                   "full-rgb-lanczos-3x-psm11": (3, Image.LANCZOS, 11),
                   "full-rgb-lanczos-4x-psm11": (4, Image.LANCZOS, 11),
                   "full-rgb-lanczos-4x-psm6": (4, Image.LANCZOS, 6),
                   "color-mask-nearest-3x-psm11": (3, Image.NEAREST, 11),
                   "color-mask-nearest-3x-psm6": (3, Image.NEAREST, 6),
                   "color-mask-lanczos-3x-psm11": (3, Image.LANCZOS, 11)}
        assert preparation in methods, "Unknown native OCR preparation"
        assert bool(color) == preparation.startswith("color-mask-"), \
            "Coloured PDF fixtures retain their original OCR mask"
        scale, resample, psm = methods[preparation]
        with Image.open(path) as image:
            size = image.size
            if color:
                mask, ink = colored_fixture_mask(image.convert("RGB").getdata(), color)
                prepared = Image.frombytes("L", size, mask)
            else:
                prepared, ink = image.convert("RGB"), 0
            prepared = prepared.resize((size[0] * scale, size[1] * scale),
                                       resample=resample)
            ocr_path = path.with_name(path.stem + "-" + preparation + ("-" + color if color else "") + ".png")
            with ocr_path.open("xb") as stream:
                prepared.save(stream, format="PNG")
        tsv = command(["tesseract", ocr_path, "stdout", "-l", "eng", "--psm", str(psm), "tsv"], timeout=25)
        tsv_path = ocr_path.with_suffix(".tsv")
        with tsv_path.open("x", encoding="utf-8") as stream:
            stream.write(tsv)
        words = [row["text"] for row in csv.DictReader(io.StringIO(tsv), delimiter="\t", quoting=csv.QUOTE_NONE)
                 if row.get("level") == "5" and row.get("text", "").strip()]
        return {"text": " ".join(words), "tsv": tsv, "scale": scale, "size": size, "ink": ink,
                "ocrPreparation": preparation,
                "png": str(path.relative_to(evidence)), "pngSha256": sha(path),
                "ocrInput": str(ocr_path.relative_to(evidence)), "ocrInputSha256": sha(ocr_path),
                "ocr": str(tsv_path.relative_to(evidence)), "ocrSha256": sha(tsv_path)}

    def observations(path, color=None):
        # All alternatives read one unchanged full frame. Only a missing literal
        # label permits another preparation; ambiguous labels remain fatal.
        # PDF alternatives never read the RGB image: every method starts from
        # the same full-frame color mask and original ink count. Neutral interface
        # text is excluded; the expected complete PDF title must still be unique.
        preparations = (("color-mask-nearest-3x-psm11", "color-mask-nearest-3x-psm6",
                         "color-mask-lanczos-3x-psm11") if color else
                        ("full-rgb-nearest-2x-psm11", "full-rgb-lanczos-3x-psm11",
                         "full-rgb-lanczos-4x-psm11", "full-rgb-lanczos-4x-psm6"))
        for preparation in preparations:
            yield read_capture(path, color, preparation=preparation)

    def evidence_fields(observation):
        return {key: value for key, value in observation.items() if key not in ("tsv", "size", "scale")}

    def scroll(direction, count=3):
        # Scroll the actual dialog body, to the right of its nested destination list.
        # Derive the point from the observed title and window bounds on this frame.
        observed, box = locate("Link multiple plan PDFs", allow_scroll=False)
        scale = observed["scale"]
        x = min(observed["size"][0] - 140, box[0] // scale + 780)
        y = min(observed["size"][1] - 180, box[3] // scale + 110)
        command(["xdotool", "mousemove", "--window", window, x, y, "click", "--repeat", str(count),
                 "--delay", "70", "4" if direction == "up" else "5"])
        actions.append({"scroll": direction, "wheelSteps": count, "point": [x, y], **evidence_fields(observed)})
        time.sleep(0.15)

    def locate(label, *, line_contains=None, exact_line=False, color=None, target=None, allow_scroll=True,
               below_label=None, above_label=None, within=None):
        for attempt in range(12 if allow_scroll and target is None else 6):
            missing_observations = []
            for observed in observations(capture(target), color):
                size = tuple(value * observed["scale"] for value in observed["size"])
                try:
                    def anchor(value, edge):
                        if value is None:
                            return None
                        if isinstance(value, tuple):
                            return fixtures.label_box(observed["tsv"], value[0], size, line_contains=value[1])[edge]
                        return fixtures.label_box(observed["tsv"], value, size)[edge]

                    box = fixtures.label_box(observed["tsv"], label, size,
                                             line_contains=line_contains, exact_line=exact_line,
                                             below=anchor(below_label, 3), above=anchor(above_label, 1),
                                             within=tuple(value * observed["scale"] for value in within) if within else None)
                    return observed, box
                except fixtures.LabelMissing:
                    missing_observations.append(observed)
            if allow_scroll and target is None:
                if attempt == 2:
                    scroll("up", 18)
                elif attempt > 2:
                    scroll("down", 3)
            time.sleep(0.2)
        error = fixtures.LabelMissing(f"Native control never became visible: {label!r}; last OCR: {observed['text']}")
        error.observations = missing_observations
        raise error

    def click(label, **options):
        observed, box = locate(label, **options)
        scale = observed["scale"]
        point = [(box[0] + box[2]) // (2 * scale), (box[1] + box[3]) // (2 * scale)]
        target = options.get("target") or window
        actions.append({"click": label, "lineContains": options.get("line_contains"), "point": point,
                        "belowLabel": options.get("below_label"), "aboveLabel": options.get("above_label"),
                        "within": options.get("within"), "exactLine": options.get("exact_line", False),
                        "window": target, **evidence_fields(observed)})
        # Let the application process the observed pointer location before the
        # button event. Moving and clicking in one command can miss a combo's
        # first interaction while leaving only its hover state visible.
        command(["xdotool", "mousemove", "--window", target, *map(str, point)])
        time.sleep(0.15)
        command(["xdotool", "click", "1"])
        time.sleep(0.2)
        return point

    def choose_popup(combo, option, *, line_contains, margins, exact_line=False):
        # Opening a native popup is an observed transition, not an assumed
        # consequence of an input event. Retry only this same source-bound
        # control; never send Escape, which could close the whole dialog.
        for attempt in range(1, 4):
            x, y = click(combo, line_contains=line_contains)
            left, top, right, bottom = margins
            within = (max(0, x - left), max(0, y - top), x + right, y + bottom)
            try:
                return click(option, allow_scroll=False, within=within, exact_line=exact_line)
            except fixtures.LabelMissing as error:
                if attempt == 3:
                    raise
                # Preserve every preparation from the same last frame so an
                # independent reader can verify the missing option and scope.
                actions.append({"popupRetry": combo, "option": option, "lineContains": line_contains,
                                "attempt": attempt, "nextAttempt": attempt + 1, "within": within,
                                "exactLine": exact_line,
                                "observations": [evidence_fields(item) for item in error.observations]})

    def verify_control(label, **options):
        observed, box = locate(label, **options)
        scale = observed["scale"]
        point = [(box[0] + box[2]) // (2 * scale), (box[1] + box[3]) // (2 * scale)]
        actions.append({"observedControl": label, "belowLabel": options.get("below_label"),
                        "aboveLabel": options.get("above_label"), "lineContains": options.get("line_contains"),
                        "within": options.get("within"), "exactLine": options.get("exact_line", False), "point": point,
                        **evidence_fields(observed)})

    def scene(name, *, pattern=None, kind=None, caption=None, allow_scroll=False):
        deadline = time.monotonic() + 35
        attempts = 0
        while time.monotonic() < deadline:
            color = "red" if kind == "destination" else "blue" if kind else None
            title = command(["xdotool", "getwindowname", window])
            for observed in observations(capture(), color):
                matched = (fixtures.fixture_visible(kind, observed["ink"], observed["text"]) if kind
                           else re.search(pattern, observed["text"], re.I) is not None)
                if matched and name in ("creation-approved-alpha", "creation-approved-beta"):
                    source_name = "Alpha.pdf" if name.endswith("alpha") else "Beta.pdf"
                    try:
                        fixtures.label_box(observed["tsv"], source_name,
                                           tuple(value * observed["scale"] for value in observed["size"]),
                                           line_contains="1 Approved links")
                    except fixtures.LabelMissing:
                        matched = False
                if matched and (caption is None or caption in title):
                    assert not any(item["name"] == name for item in scenes), "Native creation scene name is not unique"
                    stable = evidence_fields(observed)
                    for field, suffix in (("png", ".png"), ("ocrInput", "-ocr.png"), ("ocr", ".tsv")):
                        retained = scene_directory / (name + suffix)
                        with retained.open("xb") as stream:
                            stream.write((evidence / observed[field]).read_bytes())
                        assert sha(retained) == observed[field + "Sha256"]
                        stable[field] = str(retained.relative_to(evidence))
                    value = {"name": name, "windowTitle": title, "pdfKind": kind,
                             "ocrContent": "pdf-fixture" if kind else "interface", **stable}
                    scenes.append(value)
                    return value
            attempts += 1
            if allow_scroll and attempts > 1:
                scroll("down", 2)
            time.sleep(0.2)
        raise AssertionError(f"Native creation scene {name} did not qualify: {observed['text']}")

    def open_command(label, name):
        key("ctrl+k")
        time.sleep(0.2)
        command(["xdotool", "type", "--clearmodifiers", "--delay", "35", label])
        scene(name, pattern=re.escape(label))
        key("Return")

    def preview_sources():
        # Source and target are selected on the same actual control row; other
        # occurrences in headings and preview captions cannot become click targets.
        for index, name in enumerate(("Alpha.pdf", "Beta.pdf")):
            if index:
                # The popup can flip above the combo near the viewport bottom.
                # Its exact option must still be unique beside this observed row.
                choose_popup("Alpha.pdf", name, line_contains="Preview source",
                             exact_line=True, margins=(80, 120, 220, 120))
            click("Preview source", line_contains=name)
            scene("created-preview-" + name[:-4].lower(), kind=("alpha", "beta")[index], allow_scroll=True)
            # The native popup may open above or below its observed combo. Keep
            # its unique literal option near that row, away from other sources
            # and from the rendered PDF title further down the dialog.
            choose_popup("Choose a destination", "Native destination two", line_contains=name,
                         margins=(180, 120, 360, 120))
            click("Preview destination", line_contains=name)
            scene("created-destination-for-" + name[:-4].lower(), kind="destination", allow_scroll=True)

    def export_receipt(path):
        assert not path.exists()
        before = set(command(["xdotool", "search", "--onlyvisible", "--name", ".*"]).splitlines())
        click("Export save receipt")
        chooser = None
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            visible = set(command(["xdotool", "search", "--onlyvisible", "--name", ".*"]).splitlines()) - before
            matches = []
            for candidate in visible:
                geometry = command(["xdotool", "getwindowgeometry", "--shell", candidate])
                values = dict(line.split("=", 1) for line in geometry.splitlines() if "=" in line)
                if int(values.get("WIDTH", "0")) < 300 or int(values.get("HEIGHT", "0")) < 150:
                    continue
                for observed in observations(capture(candidate)):
                    if re.search(r"\bSave\b", observed["text"], re.I) and re.search(r"\b(?:Name|Location)\b", observed["text"], re.I):
                        matches.append((candidate, observed))
                        break
            assert len(matches) <= 1, "More than one native save chooser was visible"
            if matches:
                chooser, observed = matches[0]
                actions.append({"nativeSaveChooser": chooser, **evidence_fields(observed)})
                break
            time.sleep(0.25)
        assert chooser is not None, "Native receipt save chooser was not observed"
        # GTK portal and Zenity file choosers both expose the actual location
        # entry through Ctrl+L. No dialog process or return value is replaced.
        key("ctrl+l", target=chooser)
        type_text(path, target=chooser)
        key("Return", target=chooser)
        time.sleep(1)
        if not path.exists():
            visible = set(command(["xdotool", "search", "--onlyvisible", "--name", ".*"]).splitlines())
            if chooser in visible:
                click("Save", exact_line=True, target=chooser, allow_scroll=False)
        deadline = time.monotonic() + 25
        while time.monotonic() < deadline and not path.exists():
            time.sleep(0.25)
        assert path.is_file(), "Native save chooser did not export the receipt"
        scene("created-receipt-exported", pattern=r"Save\s+receipt\s+exported", allow_scroll=True)

    try:
        manager = subprocess.Popen(["openbox"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        with (evidence / "native-plan-creation-desktop.log").open("w") as log:
            # No source document is open behind the preview. Coloured PDF OCR can
            # therefore only come from the actual selected preview image.
            app = subprocess.Popen(["/usr/bin/sunipdf"], env=environment, stdout=log, stderr=subprocess.STDOUT)
            deadline = time.monotonic() + 75
            while time.monotonic() < deadline:
                alive()
                result = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(app.pid)],
                                        text=True, capture_output=True, timeout=10)
                if result.returncode == 0 and result.stdout.strip():
                    window = result.stdout.splitlines()[-1]
                    break
                time.sleep(0.25)
            assert window is not None, "Native creation window was not created"
            command(["xdotool", "windowsize", window, "1200", "900", "windowmove", window, "40", "40"])
            scene("creation-initial-home", pattern=r"Welcome\s+to\s+SuniPDF")
            open_command("Plan links", "creation-command-open")
            scene("creation-manifest-field", pattern=r"Manifest\s+path")
            type_text(fixture["manifest"])
            scene("creation-manifest-path-entered", pattern=r"plan-set\.json")
            click("Load manifest", allow_scroll=False)
            scene("creation-loaded-manifest", pattern=r"Native\s+creation\s+fixture")
            click("Link multiple PDFs", allow_scroll=False)
            scene("creation-batch-mode", pattern=r"Link\s+multiple\s+plan\s+PDFs")
            for index, source in enumerate(fixture["sources"]):
                following = (fixture["sources"][index + 1] if index + 1 < len(fixture["sources"])
                             else fixture["target"]).name
                click(source.name)
                click("Destination sheets 0", below_label=source.name, above_label=following)
                click("Native destination two", below_label=source.name, above_label=following)
                # Selecting a destination changes the header identity and collapses
                # the list. Observe the new count without reopening the prior source.
                verify_control("Destination sheets 1", below_label=source.name, above_label=following)
            click("Use saved files only")
            click("Find references")
            scene("creation-two-scanned-sources", pattern=r"Pages\s+checked\s*:?\s*2.*References\s+found\s*:?\s*2", allow_scroll=True)
            for index, source in enumerate(fixture["sources"]):
                click(source.name, line_contains="0 Approved links")
                following = ((fixture["sources"][index + 1].name, "0 Approved links")
                             if index + 1 < len(fixture["sources"]) else None)
                click("A-101", line_contains="Page 1", below_label=(source.name, "0 Approved links"),
                      above_label=following)
                scene("creation-approved-" + source.stem.lower(), pattern=re.escape(source.name) + r".*1\s+Approved\s+links")
                # Approval changes the result header identity and collapses it.
                # Do not reopen it before approving the next source's reference.
            click("Validate batch preview")
            scene("creation-two-validated-outputs", pattern=r"Validated\s+output\s+files", allow_scroll=True)
            preview_sources()
            cases.append("native-plan-batch-explicit-two-source-review-and-previews")

            occupied = fixture["outputs"][0]
            marker = b"Synthetic occupied output: native batch must not overwrite this file.\n"
            with occupied.open("xb") as stream:
                stream.write(marker)
            try:
                click("Save reviewed copies")
                scene("creation-existing-output-refused", pattern=r"Batch\s+stopped\s+after\s+an\s+error", allow_scroll=True)
                assert not fixture["outputs"][1].exists() and occupied.read_bytes() == marker
                assert all(sha(path) == value for path, value in fixture["protected"].items())
                report["occupiedOutputRefusal"] = {"output": str(occupied), "sha256": sha(occupied),
                                                   "unoccupiedOutputAbsent": True, "sentinelUnchanged": True,
                                                   "scope": "First reviewed output occupied after preview"}
            finally:
                if occupied.exists():
                    assert occupied.read_bytes() == marker, "Refused output was unexpectedly changed"
                    occupied.unlink()
            cases.append("native-plan-batch-existing-output-refused-before-any-write")
            click("Validate batch preview")
            scene("creation-revalidated-after-refusal", pattern=r"Validated\s+output\s+files", allow_scroll=True)
            click("Save reviewed copies")
            scene("creation-completed-two-copies", pattern=r"Batch\s+completed", allow_scroll=True)
            outputs = fixtures.verify_outputs(fixture, evidence, command)
            report["outputs"] = outputs
            cases.append("native-plan-batch-save-two-copies-qpdf-originals-unchanged")
            receipt_path = fixture["manifest"].parent / "native-save-receipt.json"
            export_receipt(receipt_path)
            report["saveReceipt"] = fixtures.verify_receipt(fixture, receipt_path, outputs)
            cases.append("native-plan-batch-exported-receipt-matches-files")

            for index, output in enumerate(fixture["outputs"]):
                if index:
                    open_command("Plan links", "creation-command-reopen")
                    scene("creation-reopened-batch-receipt", pattern=r"Link\s+multiple\s+plan\s+PDFs")
                click(str(output), line_contains="New linked copy saved")
                click("Verify and open saved copy", below_label=(str(output), "New linked copy saved"))
                scene("created-reopened-" + output.stem, kind=("alpha", "beta")[index], caption=output.name)
                click("A-101", color="blue", allow_scroll=False)
                scene("created-remote-confirmation-" + output.stem, pattern=r"Manifest\s+path", caption=output.name)
                scene("created-bound-manifest-" + output.stem, pattern=r"Native\s+creation\s+fixture", caption=output.name)
                click("Preview destination", allow_scroll=False)
                scene("created-remote-preview-" + output.stem, kind="destination", caption=output.name)
                click("Open verified destination", allow_scroll=False)
                scene("created-physical-page-two-" + output.stem, kind="destination", caption=fixture["target"].name)
                click("Return to source", allow_scroll=False)
                scene("created-returned-" + output.stem, kind=("alpha", "beta")[index], caption=output.name)
                assert "*" not in command(["xdotool", "getwindowname", window])
            cases.append("native-plan-batch-reopen-and-verified-physical-page-two")
            assert all(sha(path) == value for path, value in fixture["protected"].items())
            report["protectedFiles"].update({item["output"]: item["outputSha256"] for item in outputs})
            report["protectedFiles"][str(receipt_path)] = sha(receipt_path)
            retained_directory = evidence / "native-plan-creation-files"
            retained_directory.mkdir(exist_ok=False)
            retained_files = []
            originals = [(path, "source") for path in fixture["sources"]]
            originals += [(fixture["target"], "target"), (fixture["manifest"], "manifest")]
            originals += [(path, "output") for path in fixture["outputs"]] + [(receipt_path, "receipt")]
            for original, role in originals:
                assert original.is_file() and not original.is_symlink()
                contents = original.read_bytes()
                expected = report["protectedFiles"][str(original)]
                assert hashlib.sha256(contents).hexdigest() == expected
                retained = retained_directory / original.name
                with retained.open("xb") as stream:
                    stream.write(contents)
                assert sha(retained) == expected
                retained_files.append({"role": role, "originalPath": str(original),
                                       "file": str(retained.relative_to(evidence)), "sha256": expected,
                                       "bytes": len(contents)})
            report["retainedFiles"] = retained_files
            report.update(status="passed", originalsUnchanged=True, desktopLinkCreationTested=True,
                          desktopLinkSavingTested=True, receiptExportedAndVerified=True,
                          copiesReopenedAndDestinationVerified=2)
    finally:
        try:
            for process in (app, manager):
                if process is not None and process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=10)
        finally:
            (evidence / "native-plan-creation.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return cases


class ThemeLabelMissing(AssertionError):
    pass


def theme_label_center(tsv, label, image_size):
    """Find one visible label in real OCR output; reject ambiguous/outside targets."""
    groups = {}
    for row in csv.DictReader(io.StringIO(tsv), delimiter="\t", quoting=csv.QUOTE_NONE):
        if row.get("level") != "5" or not row.get("text", "").strip():
            continue
        key = tuple(row.get(name) for name in ("page_num", "block_num", "par_num", "line_num"))
        groups.setdefault(key, []).append(row)
    wanted = label.casefold().split()
    found = []
    for rows in groups.values():
        words = [row["text"].strip(".,:;[]()").casefold() for row in rows]
        if wanted in (["light"], ["dark"], ["system"]) and not all(
                word in {"light", "dark", "system"} for word in words):
            # Color choices may share one OCR line. Their explanatory sentence
            # also contains light/dark and must never become a click target.
            continue
        for start in range(len(words) - len(wanted) + 1):
            if words[start:start + len(wanted)] != wanted:
                continue
            match = rows[start:start + len(wanted)]
            left = min(int(row["left"]) for row in match)
            top = min(int(row["top"]) for row in match)
            right = max(int(row["left"]) + int(row["width"]) for row in match)
            bottom = max(int(row["top"]) + int(row["height"]) for row in match)
            if not (0 <= left < right <= image_size[0] and 0 <= top < bottom <= image_size[1]):
                raise AssertionError("Theme label escaped the captured window")
            found.append(((left + right) // 2, (top + bottom) // 2))
    if not found:
        raise ThemeLabelMissing(f"Expected one visible theme label {label!r}, found 0")
    if len(found) != 1:
        raise AssertionError(f"Expected one visible theme label {label!r}, found {len(found)}")
    return found[0]


def dark_theme_ocr_input(image):
    """Normalize light control text for older OCR without changing the native capture."""
    from PIL import Image
    # Dark material and low-contrast checkbox borders become white; bright text
    # becomes black. Doubling pixels gives small interface text enough OCR detail.
    mask = image.convert("L").point(lambda value: 0 if value >= 170 else 255)
    return mask.resize((image.width * 2, image.height * 2), resample=Image.NEAREST)


def unsaved_close_dialog_text(prompt):
    """Read the same native capture with a bounded dark-text fallback for older OCR."""
    from PIL import Image
    original_sha = sha(prompt)
    for normalize_dark in (False, True):
        ocr_path = prompt
        suffix = "-raw-ocr"
        if normalize_dark:
            ocr_path = prompt.with_name(prompt.stem + "-normalized.pgm")
            with Image.open(prompt) as image:
                dark_theme_ocr_input(image).save(ocr_path)
            suffix = "-normalized-ocr"
        text = command(["tesseract", ocr_path, "stdout", "-l", "eng", "--psm", 11], timeout=30)
        transcript = prompt.with_name(prompt.stem + suffix + ".txt")
        transcript.write_text(text + "\n", encoding="utf-8")
        if re.search(r"unsaved\s+changes", text, re.I):
            assert sha(prompt) == original_sha, "OCR normalization changed the native capture"
            prompt.with_name(prompt.stem + "-ocr.json").write_text(json.dumps({
                "nativeCapture": prompt.name, "nativeCaptureSha256": original_sha,
                "method": "normalized-light-text" if normalize_dark else "native-capture",
                "ocrInput": ocr_path.name, "ocrInputSha256": sha(ocr_path),
                "transcript": transcript.name, "transcriptSha256": sha(transcript),
                "unsavedChangesRecognized": True,
            }, indent=2) + "\n", encoding="utf-8")
            return text
    return None


THEME_TITLEBAR_PATCH = (40, 16, 48, 24)
GLASS_DARK_CHROME_RGB = (32, 34, 38)
THEME_RGB_TOLERANCE = 3


def theme_titlebar_rgb(image):
    """Sample flat client chrome, away from the centered fixture title and controls."""
    assert image.width >= 200 and image.height >= 100, "Theme capture is not an application window"
    pixels = list(image.convert("RGB").crop(THEME_TITLEBAR_PATCH).getdata())
    return [round(sum(pixel[channel] for pixel in pixels) / len(pixels), 3) for channel in range(3)]


def theme_rgb_matches(actual, expected):
    return all(abs(value - target) <= THEME_RGB_TOLERANCE for value, target in zip(actual, expected))


def theme_settings(app, window, evidence):
    """Exercise the installed UI, save real choices, then retain Glass dark for restart."""
    from PIL import Image
    settings_path = HOME / ".config/sunipdf/settings.json"
    records = []
    target_index = 0

    def wait_for(description, predicate):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            assert app.poll() is None, "Desktop exited during " + description
            result = predicate()
            if result:
                return result
            time.sleep(0.2)
        raise AssertionError("Theme settings timed out: " + description)

    def stored():
        try:
            return json.loads(settings_path.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def capture(name):
        path = evidence / (name + ".png")
        command(["import", "-window", window, path], timeout=15)
        return path

    def click_label(label):
        nonlocal target_index
        target_index += 1
        path = capture("native-theme-target")
        with Image.open(path) as image:
            image_size = image.size
            for normalize_dark in (False, True):
                scale = 2 if normalize_dark else 1
                ocr_path = evidence / f"native-theme-target-{target_index:02d}.pgm"
                if normalize_dark:
                    dark_theme_ocr_input(image).save(ocr_path)
                else:
                    ocr_path = path
                ocr_size = (image_size[0] * scale, image_size[1] * scale)
                tsv = command(["tesseract", ocr_path, "stdout", "-l", "eng", "--psm", 11, "tsv"], timeout=30)
                suffix = "-normalized" if normalize_dark else ""
                (evidence / f"native-theme-target-{target_index:02d}{suffix}.tsv").write_text(tsv + "\n", encoding="utf-8")
                try:
                    x, y = theme_label_center(tsv, label, ocr_size)
                except ThemeLabelMissing:
                    if normalize_dark:
                        raise
                    continue
                x, y = x // scale, y // scale
                break
        assert 0 <= x < image_size[0] and 0 <= y < image_size[1], "OCR target escaped native capture"
        command(["xdotool", "mousemove", "--window", window, x, y, "click", "1"])

    def require_choice(style, mode, reduced):
        def matches():
            value = stored()
            return (value.get("theme_style") == style and value.get("theme_mode") == mode
                    and value.get("reduce_transparency") is reduced)
        wait_for(f"persist {style}/{mode}/reduced={reduced}", matches)
        time.sleep(0.3)

    def record(name, dark):
        path = capture(name)
        with Image.open(path) as image:
            brightness = sum(theme_titlebar_rgb(image)) / 3
        # The modal scrim also covers the title strip; its lighter appearance
        # remains distinctly brighter than the dark material after compositing.
        assert (brightness < 80 if dark else brightness > 100), (name, brightness)
        records.append({"scene": name, "dark": dark, "titleBarBrightness": brightness,
                        "pngSha256": sha(path), "settingsSha256": sha(settings_path)})

    command(["xdotool", "windowactivate", "--sync", window, "key", "--clearmodifiers", "ctrl+k"])
    time.sleep(0.3)
    command(["xdotool", "type", "--clearmodifiers", "--delay", 45, "Theme settings"])
    command(["xdotool", "key", "--clearmodifiers", "Return"])
    time.sleep(0.5)
    click_label("Liquid Glass")
    require_choice("liquid_glass", "light", False)
    record("native-theme-glass-light", False)
    click_label("Dark")
    require_choice("liquid_glass", "dark", False)
    record("native-theme-glass-dark", True)
    # OCR can attach the adjacent checkbox glyph to "Reduce". Target the unique
    # complete second word of the same clickable label; persistence and rendered
    # persistence checks below still require the real checkbox to change.
    click_label("transparency")
    require_choice("liquid_glass", "dark", True)
    record("native-theme-glass-dark-reduced", True)
    click_label("System")
    require_choice("liquid_glass", "system", True)
    click_label("Light")
    require_choice("liquid_glass", "light", True)
    click_label("Classic")
    require_choice("classic", "light", True)
    record("native-theme-classic-light", False)
    click_label("Liquid Glass")
    # The Glass-only effects row changes the modal height and its centered
    # position. Wait for persistence and repaint before locating the next choice.
    require_choice("liquid_glass", "light", True)
    click_label("Dark")
    require_choice("liquid_glass", "dark", True)
    command(["xdotool", "key", "--clearmodifiers", "Escape"])
    time.sleep(0.3)
    assert "*" not in command(["xdotool", "getwindowname", window]), "Appearance changes dirtied the PDF"
    baseline = capture("native-theme-before-close")
    with Image.open(baseline) as image:
        baseline_rgb = theme_titlebar_rgb(image)
    # Docked Glass chrome is neutral and flat in both effects modes. This sample
    # verifies the rendered Glass palette; the native checkbox and saved state
    # verify Reduce transparency. Floating-effect rendering is checked separately.
    assert theme_rgb_matches(baseline_rgb, GLASS_DARK_CHROME_RGB), ("Glass chrome baseline", baseline_rgb)
    (evidence / "native-theme-settings.json").write_text(json.dumps({
        "schema": 1, "status": "passed", "session": "X11 / Openbox / Xvfb",
        "entry": "Ctrl+K, Theme settings", "savedChoicesVerified": True,
        "systemSelectionVerified": True, "systemAppearanceChangeSimulated": False,
        "pdfUnchanged": True, "scenes": records,
        "restartBaseline": {"file": baseline.name, "pngSha256": sha(baseline),
                            "titleBarSampleRect": THEME_TITLEBAR_PATCH, "titleBarRgb": baseline_rgb,
                            "expectedRgb": GLASS_DARK_CHROME_RGB, "rgbTolerance": THEME_RGB_TOLERANCE,
                            "scope": "Rendered Glass chrome; reduced effects are not inferred from flat chrome pixels"},
    }, indent=2) + "\n")
    return ["native-theme-classic-and-glass-live-switch", "native-theme-light-dark-and-system-selection",
            "native-theme-reduced-transparency-and-modal-escape"]


def restart_theme(app, evidence):
    from PIL import Image
    deadline = time.monotonic() + 45
    path = evidence / "native-theme-restarted.png"
    baseline = evidence / "native-theme-before-close.png"
    with Image.open(baseline) as image:
        baseline_rgb = theme_titlebar_rgb(image)
    assert theme_rgb_matches(baseline_rgb, GLASS_DARK_CHROME_RGB), ("Glass chrome baseline", baseline_rgb)
    last_rgb = None
    while time.monotonic() < deadline:
        assert app.poll() is None, "Desktop exited while checking saved appearance"
        found = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(app.pid)],
                               text=True, capture_output=True, timeout=10)
        if not found.stdout.strip():
            time.sleep(0.3)
            continue
        window = found.stdout.splitlines()[-1]
        if "/usr/lib/sunipdf/libpdfium.so" not in Path(f"/proc/{app.pid}/maps").read_text():
            time.sleep(0.3)
            continue
        command(["import", "-window", window, path], timeout=15)
        with Image.open(path) as image:
            if image.width < 200 or image.height < 100:
                continue
            blue = blue_fixture_mask(image.convert("RGB").getdata())[1]
            last_rgb = theme_titlebar_rgb(image)
            brightness = sum(last_rgb) / 3
            width = image.width
        if (blue <= 20 or brightness >= 100
                or not theme_rgb_matches(last_rgb, GLASS_DARK_CHROME_RGB)
                or not theme_rgb_matches(last_rgb, baseline_rgb)):
            time.sleep(0.3)
            continue
        saved = json.loads((HOME / ".config/sunipdf/settings.json").read_text())
        assert saved["theme_style"] == "liquid_glass" and saved["theme_mode"] == "dark" and saved["reduce_transparency"]
        command(["xdotool", "mousemove", "--window", window, width - 30, 26, "click", "1"])
        assert app.wait(timeout=25) == 0
        (evidence / "native-theme-restart.json").write_text(json.dumps({
            "status": "passed", "style": "liquid_glass", "mode": "dark", "reduceTransparency": True,
            "privatePdfiumLoaded": True, "blueFixturePixels": blue, "titleBarBrightness": brightness,
            "screenshotSha256": sha(path), "cleanExit": True,
            "renderedGlassChromeVerified": True, "reducedEffectsPersistenceVerified": True,
            "effectsRenderingScope": "Flat chrome is identical in both effects modes; floating effects are qualified separately",
            "titleBarSampleRect": THEME_TITLEBAR_PATCH,
            "beforeCloseTitleBarRgb": baseline_rgb, "restartedTitleBarRgb": last_rgb,
            "expectedTitleBarRgb": GLASS_DARK_CHROME_RGB, "rgbTolerance": THEME_RGB_TOLERANCE,
            "beforeCloseScreenshotSha256": sha(baseline),
        }, indent=2) + "\n")
        return
    raise AssertionError(f"Restart did not restore the saved dark reduced Glass appearance with the PDF visible; title RGB={last_rgb}")


def window_controls(app, window, evidence):
    """Exercise real client controls on X11; do not infer Wayland coverage."""
    from PIL import Image

    def wait_for(description, predicate, seconds=15):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            assert app.poll() is None, "Desktop exited while " + description
            result = predicate()
            if result:
                return result
            time.sleep(0.2)
        raise AssertionError("Window control timed out: " + description)

    def state():
        return command(["xprop", "-id", window, "_NET_WM_STATE"])

    def geometry():
        values = dict(line.split("=", 1) for line in command(
            ["xdotool", "getwindowgeometry", "--shell", window]).splitlines() if "=" in line)
        return {key.lower(): int(values[key]) for key in ("X", "Y", "WIDTH", "HEIGHT")}

    def maximized():
        value = state()
        return "_NET_WM_STATE_MAXIMIZED_VERT" in value and "_NET_WM_STATE_MAXIMIZED_HORZ" in value

    def control_point(index):
        # Logical target centers from the title strip, scaled to actual X11 pixels.
        # The container has a fixed 96dpi Xvfb screen and no user UI scaling.
        edge = 0 if maximized() else 6
        return geometry()["width"] - edge - 24 - (2 - index) * 40, edge + 20

    def click_control(index):
        x, y = control_point(index)
        command(["xdotool", "mousemove", "--window", window, x, y, "click", "1"])

    command(["xdotool", "windowactivate", "--sync", window])
    initial = geometry()
    records = []
    screenshot = evidence / "native-window-controls.png"
    command(["import", "-window", window, screenshot], timeout=15)
    with Image.open(screenshot) as image:
        image.crop((image.width - 144, 0, image.width, 52)).save(evidence / "native-window-controls-detail.png")
    x, y = control_point(2)
    command(["xdotool", "mousemove", "--window", window, x, y])
    time.sleep(0.3)
    hover = evidence / "native-window-close-hover.png"
    command(["import", "-window", window, hover], timeout=15)
    with Image.open(hover) as image:
        crop = image.crop((x - 16, y - 16, x + 16, y + 16)).convert("RGB")
        red_pixels = sum(1 for r, g, b in crop.getdata() if r > 130 and g < 90 and b < 100)
        assert red_pixels > 200, "Close control hover did not render its visible red circular target"
    records.append({"check": "close-hover-visible", "redPixels": red_pixels, "screenshot": hover.name})

    click_control(0)
    wait_for("minimize from title button", lambda: "_NET_WM_STATE_HIDDEN" in state())
    records.append({"check": "minimize-button", "state": state()})
    command(["xdotool", "windowmap", window, "windowactivate", "--sync", window])
    wait_for("restore minimized window", lambda: "_NET_WM_STATE_HIDDEN" not in state())

    click_control(1)
    wait_for("maximize from title button", maximized)
    time.sleep(0.3)
    records.append({"check": "maximize-button", "state": state(), "geometry": geometry()})
    command(["import", "-window", window, evidence / "native-window-maximized.png"], timeout=15)
    click_control(1)
    wait_for("restore from title button", lambda: not maximized())
    wait_for("restore window dimensions", lambda: all(abs(geometry()[key] - initial[key]) <= 2 for key in ("width", "height")))
    records.append({"check": "restore-button", "state": state(), "geometry": geometry()})

    command(["xdotool", "mousemove", "--window", window, 220, 26, "click", "--repeat", "2", "--delay", "100", "1"])
    wait_for("maximize by double-clicking title strip", maximized)
    time.sleep(0.3)
    click_control(1)
    wait_for("restore after title double-click", lambda: not maximized())
    wait_for("restore dimensions after title double-click", lambda: all(abs(geometry()[key] - initial[key]) <= 2 for key in ("width", "height")))
    records.append({"check": "title-double-click-maximize-and-restore", "geometry": geometry()})

    before_drag = geometry()
    command(["xdotool", "mousemove", "--window", window, 220, 26, "mousedown", "1"])
    time.sleep(0.2)
    # Cross the UI drag threshold first, then move under the window manager's grab.
    command(["xdotool", "mousemove_relative", "--sync", "--", 12, 0])
    time.sleep(0.2)
    command(["xdotool", "mousemove_relative", "--sync", "--", 30, 24])
    time.sleep(0.2)
    command(["xdotool", "mouseup", "1"])
    wait_for("drag title strip", lambda: geometry()["x"] != before_drag["x"] or geometry()["y"] != before_drag["y"])
    records.append({"check": "title-drag", "before": before_drag, "after": geometry()})

    before_resize = geometry()
    command(["xdotool", "mousemove", "--window", window, before_resize["width"] - 2, before_resize["height"] - 2, "mousedown", "1"])
    time.sleep(0.2)
    command(["xdotool", "mousemove_relative", "--sync", "--", -80, -50])
    time.sleep(0.2)
    command(["xdotool", "mouseup", "1"])
    wait_for("resize lower-right gutter", lambda: geometry()["width"] <= before_resize["width"] - 40 and geometry()["height"] <= before_resize["height"] - 20)
    records.append({"check": "corner-resize", "before": before_resize, "after": geometry()})

    command(["xdotool", "key", "--clearmodifiers", "ctrl+r"])
    wait_for("create an unsaved page rotation", lambda: "*" in command(["xdotool", "getwindowname", window]))
    click_control(2)
    prompt = evidence / "native-window-unsaved-close.png"

    def unsaved_prompt():
        command(["import", "-window", window, prompt], timeout=15)
        return unsaved_close_dialog_text(prompt)

    text = wait_for("guard custom close with unsaved-changes dialog", unsaved_prompt)
    (evidence / "native-window-unsaved-close.txt").write_text(text + "\n")
    records.append({"check": "close-keeps-unsaved-document-open", "screenshot": prompt.name, "processRunning": app.poll() is None})
    command(["xdotool", "key", "--clearmodifiers", "Escape"])
    time.sleep(0.3)
    command(["xdotool", "key", "--clearmodifiers", "ctrl+z"])
    wait_for("undo rotation after cancelling close", lambda: "*" not in command(["xdotool", "getwindowname", window]))
    click_control(2)
    assert app.wait(timeout=25) == 0, "Custom close did not exit cleanly after undoing the edit"
    records.append({"check": "close-button-clean-exit", "exitCode": 0})
    (evidence / "native-window-controls.json").write_text(json.dumps({
        "schema": 1, "status": "passed", "session": "X11 / Openbox / Xvfb",
        "waylandTested": False, "checks": records,
        "screenshots": [{"file": path.name, "sha256": sha(path)} for path in sorted(evidence.glob("native-window-*.png"))],
    }, indent=2) + "\n")
    return ["native-window-minimize-maximize-restore-drag-resize", "native-window-close-preserves-unsaved-work", "native-window-close-clean-exit"]


def lifecycle(package, evidence, identity):
    if sys.platform != "linux" or os.geteuid() != 0:
        raise ValueError("Lifecycle validation runs as root in a disposable Ubuntu container")
    import pwd
    identity.update(distribution_identity())
    package = package.resolve(strict=True)
    identity["deb"] = {"name": package.name, "sha256": sha(package)}
    expected, filename = package.with_suffix(".deb.sha256").read_text().strip().split("  ", 1)
    assert filename == package.name and sha(package) == expected
    version = command(["dpkg-deb", "--field", package, "Version"])
    identity["version"] = version
    assert command(["dpkg-deb", "--field", package, "Package"]) == "sunipdf"
    assert command(["dpkg-deb", "--field", package, "Architecture"]) == "amd64"
    evidence.mkdir(parents=True, exist_ok=True)
    (evidence / "package-control.txt").write_text(command(["dpkg-deb", "--info", package]) + "\n")
    command(["apt-get", "install", "-y", str(package)], timeout=600)
    payload_count = verify_installed_inventory()
    source_sha = os.environ.get("GITHUB_SHA", "")
    assert re.fullmatch(r"[0-9a-f]{40}", source_sha), "Exact workflow source revision is required"
    provenance = json.loads(Path("/usr/share/doc/sunipdf/build-provenance.json").read_text())
    assert provenance["sourceRevision"] == source_sha, "Installed candidate comes from another source revision"
    command(["desktop-file-validate", "/usr/share/applications/sunipdf.desktop"])
    elf = {}
    for name in ("sunipdf", "sunipdf-cli", "libpdfium.so"):
        path = Path("/usr/lib/sunipdf") / name
        elf[name] = command(["ldd", path])
        assert "not found" not in elf[name], elf[name]
        assert not path.stat().st_mode & 0o022
    command(["useradd", "--create-home", "--shell", "/bin/bash", ACCOUNT])
    user = pwd.getpwnam(ACCOUNT)
    os.chown(evidence, user.pw_uid, user.pw_gid)
    runtime_dir = HOME / ".runtime"
    runtime_dir.mkdir(mode=0o700)
    os.chown(runtime_dir, user.pw_uid, user.pw_gid)
    environment = ["env", "HOME=" + str(HOME), "XDG_CONFIG_HOME=" + str(HOME / ".config"),
                   "XDG_DATA_HOME=" + str(HOME / ".local/share"), "XDG_RUNTIME_DIR=" + str(runtime_dir),
                   "LIBGL_ALWAYS_SOFTWARE=1", "SUNIPDF_RENDERER=glow", "XDG_SESSION_TYPE=x11",
                   "WINIT_X11_SCALE_FACTOR=1", "GITHUB_SHA=" + source_sha]
    command(["runuser", "-u", ACCOUNT, "--", *environment, "dbus-run-session", "--", "xvfb-run", "-a", "-s", "-screen 0 1400x1000x24 -dpi 96", "python3", __file__, "--runtime", "--version", version, "--evidence", evidence], timeout=1200)
    runtime_proof = json.loads((evidence / "runtime.json").read_text())
    plan_path = evidence / "native-plan-links.json"
    plan_proof = json.loads(plan_path.read_text(encoding="utf-8"))
    assert runtime_proof["nativePlanLinks"] == {"file": plan_path.name, "sha256": sha(plan_path)}
    assert plan_proof["status"] == "passed" and plan_proof["fixturesUnchanged"]
    assert plan_proof["version"] == version and plan_proof["sourceSha"] == source_sha
    assert plan_proof["syntheticOnly"] and plan_proof["isolatedProfile"]
    assert not plan_proof["desktopLinkCreationTested"] and not plan_proof["desktopLinkSavingTested"]
    assert len(plan_proof["cases"]) == 6 and len(set(plan_proof["cases"])) == 6
    assert plan_proof["returnToSource"]["sameSourcePdfAndReadingPosition"]
    assert plan_proof["returnToSource"]["returnControlHiddenAfterReturn"]
    assert all(case in runtime_proof["cases"] for case in plan_proof["cases"])
    creation_path = evidence / "native-plan-creation.json"
    creation = json.loads(creation_path.read_text(encoding="utf-8"))
    assert runtime_proof["nativePlanCreation"] == {"file": creation_path.name, "sha256": sha(creation_path)}
    assert creation["status"] == "passed" and creation["originalsUnchanged"]
    assert creation["version"] == version and creation["sourceSha"] == source_sha
    assert creation["syntheticOnly"] and creation["isolatedProfile"]
    assert creation["desktopLinkCreationTested"] and creation["desktopLinkSavingTested"]
    assert creation["receiptExportedAndVerified"] and creation["copiesReopenedAndDestinationVerified"] == 2
    assert creation["sourceCount"] == len(creation["outputs"]) == 2
    assert len(creation["cases"]) == len(set(creation["cases"])) == 5
    assert all(case in runtime_proof["cases"] for case in creation["cases"])
    assert sha(evidence / creation["fixtureReport"]) == creation["fixtureReportSha256"]
    assert sha(Path(creation["saveReceipt"]["path"])) == creation["saveReceipt"]["sha256"]
    for scene in creation["scenes"]:
        assert sha(evidence / scene["png"]) == scene["pngSha256"]
        assert sha(evidence / scene["ocrInput"]) == scene["ocrInputSha256"]
        assert sha(evidence / scene["ocr"]) == scene["ocrSha256"]
    protected = {Path(runtime_proof["source"]): runtime_proof["sourceSha256"], Path(runtime_proof["settings"]): runtime_proof["settingsSha256"]}
    protected.update({Path(path): expected for path, expected in plan_proof["protectedFiles"].items()})
    protected.update({Path(path): expected for path, expected in creation["protectedFiles"].items()})
    assert all(path.is_file() and sha(path) == expected for path, expected in protected.items())
    update_spec = importlib.util.spec_from_file_location("installed_update_qualification", ROOT / "scripts/test-linux-update-installed.py")
    update_qualification = importlib.util.module_from_spec(update_spec)
    update_spec.loader.exec_module(update_qualification)
    update_proof = update_qualification.verify_updates(package, evidence, source_sha, protected)
    assert update_proof["status"] == "passed" and update_proof["candidateRestored"]
    assert verify_installed_inventory() == payload_count
    command(["apt-get", "remove", "-y", "sunipdf"], timeout=120)
    assert not Path("/usr/bin/sunipdf").exists()
    assert all(path.is_file() and sha(path) == value for path, value in protected.items())
    command(["apt-get", "install", "-y", str(package)], timeout=180)
    assert verify_installed_inventory() == payload_count
    assert all(path.is_file() and sha(path) == value for path, value in protected.items())
    assert "SuniPDF page 1" in command(["runuser", "-u", ACCOUNT, "--", *environment, CLI, "text", runtime_proof["source"], "--page", 1])
    screenshots = [{"file": path.name, "sha256": sha(path)} for path in sorted(evidence.glob("*.png"))]
    report = {"schema": 1, "status": "passed", "exitCode": 0, **identity, "sourceSha": source_sha,
              "screenshots": screenshots,
              "checks": {"install": True, "cli": True, "pdf_render_save_reopen": True,
                         "ocr_en": True, "ocr_fr": True, "native_gui_pdf_visible": True,
                         "native_x11_window_controls": True, "native_close_unsaved_guard": True,
                         "native_theme_settings": True, "native_theme_restart_persistence": True,
                         "native_reader_independent_and_synchronized_pages": True,
                         "native_reader_close_split_preserves_document": True,
                         "native_plan_link_destination_preview_and_cancel": True,
                         "native_plan_link_verified_physical_page_navigation": True,
                         "native_plan_link_changed_or_missing_destination_refused": True,
                         "native_plan_link_changed_source_refused": True,
                         "native_plan_link_return_to_source_preserves_reading_position": True,
                         "native_plan_link_two_source_review_create_save": True,
                         "native_plan_link_occupied_output_refused_before_writes": True,
                         "native_plan_link_saved_copies_independently_checked": True,
                         "native_plan_link_exported_receipt_verified": True,
                         "verified_public_deb_upgrade": True, "update_refusal_guards": True,
                         "reinstall": True, "uninstall": True, "user_data_preserved": True},
              "lifecycle": "Remove and reinstall the same verified candidate; existing documents and settings remain unchanged",
              "installedPayloadFiles": payload_count, "elfDependencies": elf,
              "normalUserExecution": True, "runtimeCases": runtime_proof["cases"]}
    report["nativePlanLinkEvidence"] = {**runtime_proof["nativePlanLinks"], "cases": plan_proof["cases"],
                                       "desktopLinkCreationTested": False, "desktopLinkSavingTested": False}
    report["nativePlanCreationEvidence"] = {**runtime_proof["nativePlanCreation"], "cases": creation["cases"],
                                           "desktopLinkCreationTested": True, "desktopLinkSavingTested": True,
                                           "copiesReopenedAndDestinationVerified": 2}
    report["installedUpdateEvidence"] = {"file": "linux-update-installed.json",
                                        "sha256": sha(evidence / "linux-update-installed.json"),
                                        "polkitAuthenticationTested": False,
                                        "desktopUpdaterInteractionTested": False}
    (evidence / "test-deb.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--deb", type=Path)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--runtime", action="store_true")
    parser.add_argument("--version")
    args = parser.parse_args()
    identity = {"version": args.version, "sourceSha": os.environ.get("GITHUB_SHA")}
    try:
        if args.runtime:
            if not args.version:
                parser.error("--runtime requires --version")
            runtime(args.evidence.resolve(), args.version)
        else:
            if not args.deb:
                parser.error("--deb is required")
            lifecycle(args.deb, args.evidence.resolve(), identity)
    except Exception as error:
        args.evidence.mkdir(parents=True, exist_ok=True)
        (args.evidence / "failure.txt").write_text(str(error) + "\n")
        if not args.runtime:
            failure = {"schema": 1, "status": "failed", "exitCode": 1, **identity, "error": str(error),
                       "screenshots": [{"file": path.name, "sha256": sha(path)} for path in sorted(args.evidence.glob("*.png"))]}
            (args.evidence / "test-deb.json").write_text(json.dumps(failure, indent=2) + "\n")
        raise
