#!/usr/bin/env python3
"""Run bounded measurements from the desktop test binary already built by CI."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
CASES = {
    "view::tests::performance_cached_layout_5000_pages": (
        "cached_layout", {"pages": 5000, "samples": 1000}
    ),
    "annotation_register::tests::performance_register_filter_10000_annotations": (
        "register_filter", {"rows": 10000, "samples": 100}
    ),
}


def executable_from_log(text: str, windows: bool) -> str:
    plain = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", text)
    suffix = r"\.exe" if windows else ""
    pattern = (r"^\s*Running\s+unittests\s+src[\\/]main\.rs\s+\("
               r"(target[\\/]debug[\\/]deps[\\/]sunipdf-[a-f0-9]{16}" + suffix + r")\)\s*$")
    matches = re.findall(pattern, plain, re.MULTILINE)
    if len(matches) != 1:
        raise ValueError("Workspace log must identify exactly one desktop test executable")
    return matches[0].replace("\\", "/")


def metric_from_output(text: str, metric: str, expected: dict[str, int]) -> dict[str, int]:
    lines = re.findall(r"(?:^|\.\.\. )" + re.escape(metric) + r" (.+)$", text, re.MULTILINE)
    if len(lines) != 1:
        raise ValueError("Missing or duplicated performance measurement")
    parts = lines[0].strip().split()
    if any(not re.fullmatch(r"[a-z0-9_]+=[0-9]+", part) for part in parts):
        raise ValueError("Invalid performance measurement")
    data = {key: int(value) for key, value in (part.split("=") for part in parts)}
    if len(parts) != len(data) or set(data) != set(expected) | {"p95_us"}:
        raise ValueError("Unexpected performance measurement fields")
    if any(data[key] != value for key, value in expected.items()):
        raise ValueError("Performance workload differs from the qualified fixture")
    return data


class BoundaryTests(unittest.TestCase):
    def test_binary_selection(self):
        win = r"Running unittests src\main.rs (target\debug\deps\sunipdf-0123456789abcdef.exe)"
        unix = "Running unittests src/main.rs (target/debug/deps/sunipdf-0123456789abcdef)"
        self.assertEqual(executable_from_log(win, True), "target/debug/deps/sunipdf-0123456789abcdef.exe")
        self.assertEqual(executable_from_log(unix, False), "target/debug/deps/sunipdf-0123456789abcdef")
        for invalid in ["", win + "\n" + win, win.replace("target\\debug", "..\\outside"),
                        win.replace("sunipdf-", "sunipdf_cli-"), win.replace("src\\main", "tests\\main")]:
            with self.assertRaises(ValueError):
                executable_from_log(invalid, True)
        with self.assertRaises(ValueError):
            executable_from_log(win, False)

    def test_measurement_contract(self):
        expected = {"pages": 5000, "samples": 1000}
        valid = "cached_layout pages=5000 samples=1000 p95_us=4"
        self.assertEqual(metric_from_output(valid, "cached_layout", expected)["p95_us"], 4)
        self.assertEqual(metric_from_output("test view::tests::sample ... " + valid, "cached_layout", expected)["p95_us"], 4)
        for invalid in ["", valid + "\n" + valid, valid.replace("5000", "500"),
                        valid.replace("p95_us=4", "p95_us=-4"), valid + " pages=5000",
                        valid + " unknown=1", valid.replace("p95_us=4", "p95_us=NaN")]:
            with self.assertRaises(ValueError):
                metric_from_output(invalid, "cached_layout", expected)


def run() -> None:
    if (os.environ.get("GITHUB_ACTIONS") != "true"
            or os.environ.get("GITHUB_REPOSITORY") != "lmdanavanem/SuniPDF"
            or os.environ.get("RUNNER_OS") not in {"Windows", "Linux"}
            or os.environ.get("RUNNER_ARCH") != "X64"):
        raise ValueError("Performance evidence runs only in repository x64 CI")
    source = os.environ.get("GITHUB_SHA", "")
    if not re.fullmatch(r"[a-f0-9]{40}", source):
        raise ValueError("Missing exact source revision")
    directory = ROOT / "target/performance-evidence"
    directory.mkdir(parents=True, exist_ok=True)
    report = {"schema": 1, "status": "failed", "sourceSha": source,
              "runnerOS": os.environ["RUNNER_OS"], "runnerArchitecture": "X64",
              "profile": "debug-unit-tests", "measurements": [],
              "qualification": "Bounded model operations measured on the CI runner; excludes complete UI frame time, GPU rendering and physical hardware qualification."}
    try:
        log = ROOT / "target/workspace-tests.log"
        if log.stat().st_size > 8 * 1024 * 1024:
            raise ValueError("Workspace log exceeds its size bound")
        workspace_log = log.read_bytes()
        if len(workspace_log) > 8 * 1024 * 1024:
            raise ValueError("Workspace log exceeds its size bound")
        (directory / "workspace-tests.log").write_bytes(workspace_log)
        report["workspaceLogSha256"] = hashlib.sha256(workspace_log).hexdigest()
        relative = executable_from_log(workspace_log.decode("utf-8", errors="replace"), os.name == "nt")
        executable = ROOT / relative
        if executable.is_symlink() or executable.resolve().parent != (ROOT / "target/debug/deps").resolve():
            raise ValueError("Desktop test executable escapes its qualified directory")
        digest = hashlib.sha256()
        with executable.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
        report["executableSha256"] = digest.hexdigest()
        manifest = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
        package = re.search(r"(?ms)^\[workspace\.package\]\s*\n([^\[]+)", manifest)
        version = re.search(r'^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"\s*$', package[1], re.MULTILINE) if package else None
        if not version:
            raise ValueError("Missing workspace release version")
        report["version"] = version[1]
        listing = subprocess.run([str(executable), "--list", "--ignored"], cwd=ROOT / "apps/sunipdf-desktop",
                                 capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=30, check=True)
        for test in CASES:
            if listing.stdout.splitlines().count(test + ": test") != 1:
                raise ValueError("Required measurement is absent from the qualified test binary")
        for test, (metric, expected) in CASES.items():
            started = time.monotonic()
            result = subprocess.run([str(executable), "--ignored", "--exact", test, "--nocapture", "--test-threads=1"],
                                    cwd=ROOT / "apps/sunipdf-desktop", capture_output=True,
                                    text=True, encoding="utf-8", errors="replace", timeout=120, check=False)
            output = result.stdout + result.stderr
            if len(output.encode("utf-8")) > 1024 * 1024:
                raise ValueError("Measurement output exceeds its size bound")
            (directory / f"{metric}.log").write_text(output, encoding="utf-8")
            print(output, end="")
            if result.returncode or not re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;", output):
                raise ValueError("Required performance test did not complete successfully")
            values = metric_from_output(output, metric, expected)
            report["measurements"].append({"test": test, "metric": metric, **values,
                                           "elapsedSeconds": round(time.monotonic() - started, 3)})
        report["status"] = "passed"
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        (directory / "performance-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        unittest.main(argv=[sys.argv[0]])
    else:
        run()
