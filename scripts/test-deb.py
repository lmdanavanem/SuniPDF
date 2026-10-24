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
    assert sha(source) == original
    settings = HOME / ".config/sunipdf/settings.json"
    assert settings.is_file(), "Native application did not save user settings on close"
    proof = {"schema": 1, "version": version, "cases": cases, "sourceSha256": original,
             "settingsSha256": sha(settings), "source": str(source), "settings": str(settings),
             "tesseract": command(["tesseract", "--version"]), "qpdf": command(["qpdf", "--version"])}
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
            themes = theme_settings(app, window, evidence)
            controls = window_controls(app, window, evidence)
            app = subprocess.Popen(["/usr/bin/sunipdf", str(source)], stdout=log, stderr=subprocess.STDOUT)
            restart_theme(app, evidence)
            return themes + controls + ["native-theme-persists-after-restart"]
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


def theme_label_center(tsv, label, image_size):
    """Find one visible label in real OCR output; reject ambiguous/outside targets."""
    groups = {}
    for row in csv.DictReader(io.StringIO(tsv), delimiter="\t"):
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


THEME_TITLEBAR_PATCH = (40, 16, 48, 24)
GLASS_DARK_REDUCED_RGB = (24, 41, 61)
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

    def click_label(label, normalize_dark=False):
        nonlocal target_index
        target_index += 1
        path = capture("native-theme-target")
        ocr_path = path
        scale = 2 if normalize_dark else 1
        with Image.open(path) as image:
            image_size = image.size
            if normalize_dark:
                ocr_path = evidence / f"native-theme-target-{target_index:02d}.pgm"
                dark_theme_ocr_input(image).save(ocr_path)
        ocr_size = (image_size[0] * scale, image_size[1] * scale)
        tsv = command(["tesseract", ocr_path, "stdout", "-l", "eng", "--psm", 11, "tsv"], timeout=30)
        (evidence / f"native-theme-target-{target_index:02d}.tsv").write_text(tsv + "\n", encoding="utf-8")
        x, y = theme_label_center(tsv, label, ocr_size)
        x, y = x // scale, y // scale
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
    # reduced-Glass checks below still require the real checkbox to change.
    click_label("transparency", normalize_dark=True)
    require_choice("liquid_glass", "dark", True)
    record("native-theme-glass-dark-reduced", True)
    click_label("System")
    wait_for("persist system mode", lambda: stored().get("theme_mode") == "system")
    click_label("Light")
    require_choice("liquid_glass", "light", True)
    click_label("Classic")
    require_choice("classic", "light", True)
    record("native-theme-classic-light", False)
    click_label("Liquid Glass")
    click_label("Dark")
    require_choice("liquid_glass", "dark", True)
    command(["xdotool", "key", "--clearmodifiers", "Escape"])
    time.sleep(0.3)
    assert "*" not in command(["xdotool", "getwindowname", window]), "Appearance changes dirtied the PDF"
    baseline = capture("native-theme-before-close")
    with Image.open(baseline) as image:
        baseline_rgb = theme_titlebar_rgb(image)
    # theme.rs gives reduced Glass dark an opaque (24, 41, 61) chrome fill.
    # Classic uses (36, 36, 36). Unreduced Glass overlays white alpha 12,
    # fading across 38 px: even the maximized patch averages alpha 6/255,
    # lifting red by about 5.4 levels, outside the 3-level rounding tolerance.
    assert theme_rgb_matches(baseline_rgb, GLASS_DARK_REDUCED_RGB), ("reduced Glass baseline", baseline_rgb)
    (evidence / "native-theme-settings.json").write_text(json.dumps({
        "schema": 1, "status": "passed", "session": "X11 / Openbox / Xvfb",
        "entry": "Ctrl+K, Theme settings", "savedChoicesVerified": True,
        "systemSelectionVerified": True, "systemAppearanceChangeSimulated": False,
        "pdfUnchanged": True, "scenes": records,
        "restartBaseline": {"file": baseline.name, "pngSha256": sha(baseline),
                            "titleBarSampleRect": THEME_TITLEBAR_PATCH, "titleBarRgb": baseline_rgb,
                            "expectedRgb": GLASS_DARK_REDUCED_RGB, "rgbTolerance": THEME_RGB_TOLERANCE},
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
    assert theme_rgb_matches(baseline_rgb, GLASS_DARK_REDUCED_RGB), ("reduced Glass baseline", baseline_rgb)
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
                or not theme_rgb_matches(last_rgb, GLASS_DARK_REDUCED_RGB)
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
            "renderedReducedGlassVerified": True, "titleBarSampleRect": THEME_TITLEBAR_PATCH,
            "beforeCloseTitleBarRgb": baseline_rgb, "restartedTitleBarRgb": last_rgb,
            "expectedTitleBarRgb": GLASS_DARK_REDUCED_RGB, "rgbTolerance": THEME_RGB_TOLERANCE,
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
        text = command(["tesseract", prompt, "stdout", "-l", "eng", "--psm", 11], timeout=30)
        return text if re.search(r"unsaved\s+changes", text, re.I) else None

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
                   "WINIT_X11_SCALE_FACTOR=1"]
    command(["runuser", "-u", ACCOUNT, "--", *environment, "dbus-run-session", "--", "xvfb-run", "-a", "-s", "-screen 0 1400x1000x24 -dpi 96", "python3", __file__, "--runtime", "--version", version, "--evidence", evidence], timeout=600)
    runtime_proof = json.loads((evidence / "runtime.json").read_text())
    protected = {Path(runtime_proof["source"]): runtime_proof["sourceSha256"], Path(runtime_proof["settings"]): runtime_proof["settingsSha256"]}
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
                         "verified_public_deb_upgrade": True, "update_refusal_guards": True,
                         "reinstall": True, "uninstall": True, "user_data_preserved": True},
              "lifecycle": "Remove and reinstall the same verified candidate; existing documents and settings remain unchanged",
              "installedPayloadFiles": payload_count, "elfDependencies": elf,
              "normalUserExecution": True, "runtimeCases": runtime_proof["cases"]}
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
