#!/usr/bin/env python3
"""Validate a built macOS DMG: checksum, signature, notices, packaged CLI and a desktop launch."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "crates/sunipdf-core/tests/fixtures/two-pages.pdf"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def command(arguments, timeout=120, **kwargs):
    result = subprocess.run([str(value) for value in arguments], text=True, capture_output=True, timeout=timeout, **kwargs)
    if result.returncode:
        raise RuntimeError(f"{Path(str(arguments[0])).name} failed ({result.returncode}): {result.stderr.strip()}")
    return result.stdout


def png_size(path):
    header = Path(path).read_bytes()[:24]
    if header[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("Rendered page is not a PNG image")
    return struct.unpack(">II", header[16:24])


def mounted_copy(dmg, directory):
    """Copy SuniPDF.app out of the read-only image, as a user installing it would."""
    mount = directory / "mount"
    mount.mkdir()
    command(["hdiutil", "attach", "-nobrowse", "-readonly", "-mountpoint", mount, dmg])
    try:
        app = mount / "SuniPDF.app"
        command(["codesign", "--verify", "--deep", "--strict", app])
        if os.readlink(mount / "Applications") != "/Applications":
            raise ValueError("The image does not offer the Applications drop target")
        installed = directory / "installed"
        installed.mkdir()
        command(["ditto", app, installed / "SuniPDF.app"])
    finally:
        command(["hdiutil", "detach", mount])
    return installed


def desktop_launch(executable, source, home, evidence):
    """The packaged desktop app must start with a PDF and stay running."""
    with (evidence / "desktop.log").open("w") as log:
        app = subprocess.Popen([executable, source], stdout=log, stderr=subprocess.STDOUT, env=dict(os.environ, HOME=str(home)))
        try:
            time.sleep(15)
            if app.poll() is not None:
                raise RuntimeError(f"Desktop application exited early with status {app.returncode}")
            # Evidence only: a headless runner session may refuse screen capture.
            subprocess.run(["screencapture", "-x", evidence / "desktop.png"], timeout=30, check=False)
        finally:
            app.terminate()
            try:
                app.wait(timeout=15)
            except subprocess.TimeoutExpired:
                app.kill()


def validate(dmg, evidence):
    match = re.fullmatch(r"SuniPDF-([0-9]+\.[0-9]+\.[0-9]+)-macos-arm64\.dmg", dmg.name)
    if not match:
        raise ValueError("Unexpected DMG name: " + dmg.name)
    version = match[1]
    evidence.mkdir(parents=True, exist_ok=True)
    if dmg.with_name(dmg.name + ".sha256").read_text().split() != [sha(dmg), dmg.name]:
        raise ValueError("DMG checksum file does not match the image")
    cases = ["published-checksum-matches-image"]
    with tempfile.TemporaryDirectory(prefix="sunipdf-dmg-") as directory:
        # Canonical path: the notice verifier refuses symbolic links such as /var -> /private/var.
        directory = Path(directory).resolve()
        installed = mounted_copy(dmg, directory)
        cases.append("ad-hoc-signature-seals-the-bundle-and-image-offers-applications")
        command([sys.executable, "-I", ROOT / "scripts/license-inventory.py", "--macos", "--verify", "--stage", installed])
        cases.append("license-inventory-matches-installed-resources")
        macos = installed / "SuniPDF.app/Contents/MacOS"
        cli = macos / "sunipdf-cli"
        source = directory / "macOS PDF test.pdf"
        shutil.copyfile(FIXTURE, source)
        original = sha(source)
        if version not in command([cli, "--version"]):
            raise ValueError("Packaged CLI reports another version")
        if json.loads(command([cli, "info", source]))["page_count"] != 2:
            raise ValueError("Packaged CLI did not read the fixture")
        if "SuniPDF page 1" not in command([cli, "text", source, "--page", 1]):
            raise ValueError("Packaged CLI did not extract the fixture text")
        rendered = evidence / "page-1.png"
        command([cli, "render", source, "--page", 1, "--scale", 1, "--out", rendered])
        if png_size(rendered) != (612, 792):
            raise ValueError("Packaged PDFium rendered an unexpected page size")
        cases.append("packaged-cli-loads-bundled-pdfium-and-renders-fixture")
        home = directory / "home"
        home.mkdir()
        desktop_launch(macos / "sunipdf", source, home, evidence)
        cases.append("desktop-app-starts-with-a-pdf")
        if sha(source) != original:
            raise ValueError("Validation modified the source PDF")
    proof = {"schema": 1, "version": version, "dmg": dmg.name, "dmgSha256": sha(dmg), "cases": cases,
             "macos": command(["sw_vers"]).strip()}
    (evidence / "runtime.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(proof, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dmg", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    validate(args.dmg.resolve(), args.evidence.resolve())
