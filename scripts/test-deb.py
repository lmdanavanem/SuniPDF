#!/usr/bin/env python3
"""Validate the installed .deb in a clean Ubuntu container, including its native UI."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
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
    gui(evidence, source)
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
            while time.monotonic() < deadline:
                if app.poll() is not None:
                    raise AssertionError("Installed desktop exited before rendering a PDF")
                time.sleep(2)
                maps = Path(f"/proc/{app.pid}/maps").read_text()
                if "/usr/lib/sunipdf/libpdfium.so" not in maps:
                    continue
                command(["import", "-window", "root", screenshot], timeout=15)
                with Image.open(screenshot) as image:
                    blue = sum(1 for r, g, b in image.convert("RGB").getdata() if b > 180 and r < 70 and g < 70)
                displayed = command(["tesseract", screenshot, "stdout", "-l", "eng", "--psm", 11], timeout=30)
                (evidence / "native-reader-text.txt").write_text(displayed + "\n")
                if blue > 2000 and re.search(r"SuniPDF\s+page\s+[1Il]", displayed, re.I):
                    (evidence / "native-reader-proof.json").write_text(json.dumps({"privatePdfiumLoaded": True, "blueFixturePixels": blue, "visiblePdfText": "SuniPDF page 1", "renderer": "glow", "screenshotSha256": sha(screenshot)}, indent=2) + "\n")
                    success = True
                    break
            assert success, "Native screenshot did not contain the fixture's PDF text and blue rectangle"
            window = command(["xdotool", "search", "--onlyvisible", "--pid", app.pid]).splitlines()[-1]
            command(["wmctrl", "-i", "-c", hex(int(window))])
            assert app.wait(timeout=25) == 0, "Native desktop did not exit cleanly"
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


def lifecycle(package, evidence):
    if sys.platform != "linux" or os.geteuid() != 0:
        raise ValueError("Lifecycle validation runs as root in a disposable Ubuntu container")
    package = package.resolve(strict=True)
    expected, filename = package.with_suffix(".deb.sha256").read_text().strip().split("  ", 1)
    assert filename == package.name and sha(package) == expected
    version = command(["dpkg-deb", "--field", package, "Version"])
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
                   "LIBGL_ALWAYS_SOFTWARE=1", "SUNIPDF_RENDERER=glow", "XDG_SESSION_TYPE=x11"]
    command(["runuser", "-u", ACCOUNT, "--", *environment, "dbus-run-session", "--", "xvfb-run", "-a", "-s", "-screen 0 1400x1000x24", "python3", __file__, "--runtime", "--version", version, "--evidence", evidence], timeout=600)
    runtime_proof = json.loads((evidence / "runtime.json").read_text())
    protected = {Path(runtime_proof["source"]): runtime_proof["sourceSha256"], Path(runtime_proof["settings"]): runtime_proof["settingsSha256"]}
    command(["apt-get", "remove", "-y", "sunipdf"], timeout=120)
    assert not Path("/usr/bin/sunipdf").exists()
    assert all(path.is_file() and sha(path) == value for path, value in protected.items())
    command(["apt-get", "install", "-y", str(package)], timeout=180)
    assert verify_installed_inventory() == payload_count
    assert all(path.is_file() and sha(path) == value for path, value in protected.items())
    assert "SuniPDF page 1" in command(["runuser", "-u", ACCOUNT, "--", *environment, CLI, "text", runtime_proof["source"], "--page", 1])
    os_release = Path("/etc/os-release").read_text()
    distro = dict(line.split("=", 1) for line in os_release.splitlines() if "=" in line)
    screenshots = [{"file": path.name, "sha256": sha(path)} for path in sorted(evidence.glob("*.png"))]
    report = {"schema": 1, "status": "passed", "exitCode": 0, "version": version, "sourceSha": source_sha,
              "distro": {"id": distro["ID"].strip('"'), "version_id": distro["VERSION_ID"].strip('"')},
              "deb": {"name": package.name, "sha256": sha(package)}, "screenshots": screenshots,
              "checks": {"install": True, "cli": True, "pdf_render_save_reopen": True,
                         "ocr_en": True, "ocr_fr": True, "native_gui_pdf_visible": True,
                         "reinstall": True, "uninstall": True, "user_data_preserved": True},
              "lifecycle": "Remove and reinstall the same verified candidate; existing documents and settings remain unchanged",
              "installedPayloadFiles": payload_count, "osRelease": os_release, "elfDependencies": elf,
              "normalUserExecution": True, "runtimeCases": runtime_proof["cases"]}
    (evidence / "test-deb.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--deb", type=Path)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--runtime", action="store_true")
    parser.add_argument("--version")
    args = parser.parse_args()
    try:
        if args.runtime:
            if not args.version:
                parser.error("--runtime requires --version")
            runtime(args.evidence.resolve(), args.version)
        else:
            if not args.deb:
                parser.error("--deb is required")
            lifecycle(args.deb, args.evidence.resolve())
    except Exception as error:
        args.evidence.mkdir(parents=True, exist_ok=True)
        (args.evidence / "failure.txt").write_text(str(error) + "\n")
        if not args.runtime:
            failure = {"schema": 1, "status": "failed", "exitCode": 1, "version": args.version,
                       "sourceSha": os.environ.get("GITHUB_SHA"), "error": str(error),
                       "screenshots": [{"file": path.name, "sha256": sha(path)} for path in sorted(args.evidence.glob("*.png"))]}
            (args.evidence / "test-deb.json").write_text(json.dumps(failure, indent=2) + "\n")
        raise
