"""Qualify reusable overlay setups with a packaged CLI and independently decoded pixels."""
import argparse
import copy
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import time

from PIL import Image, __version__ as pillow_version


MAX_TRANSCRIPT_BYTES = 2_000_000
PROCESS_SECONDS = 30
MAX_MANIFEST_BYTES = 256 * 1024
SCOPE = (
    "Original synthetic local PDFs, packaged CLI setup save/reopen/relocation, "
    "source bindings, refusal behavior and independent RGBA checks. "
    "No native desktop interaction or private-plan qualification is claimed."
)
GEOMETRY = [
    ([0, 0, 128, 128], 0, [16, 16, 48, 48], 1, [128, 128]),
    ([10, 20, 138, 116], 90, [16, 16, 48, 48], 1, [96, 128]),
    ([20, 30, 84, 126], 180, [8, 8, 24, 24], 1, [64, 96]),
    ([5, 15, 85, 143], 270, [16, 16, 48, 48], 2, [128, 80]),
]


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def load_producer():
    path = Path(__file__).with_name("test-overlay-stack.py")
    spec = importlib.util.spec_from_file_location("overlay_setup_fixture", path)
    producer = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(producer)
    return producer, path


def validate_fixture_xref(data):
    """Check the generated PDF's offsets and stream lengths without invoking a PDF editor."""
    start = re.search(rb"startxref\n(\d+)\n%%EOF\n$", data)
    if start is None:
        raise AssertionError("fixture has no final startxref")
    xref = int(start.group(1))
    if data[xref:xref + 5] != b"xref\n":
        raise AssertionError("fixture startxref is incorrect")
    lines = data[xref:].splitlines()
    first, count = map(int, lines[1].split())
    if first != 0 or lines[2] != b"0000000000 65535 f ":
        raise AssertionError("fixture has an invalid free xref entry")
    for number, entry in enumerate(lines[3:count + 2], 1):
        offset = int(entry.split()[0])
        if not data[offset:].startswith(f"{number} 0 obj\n".encode("ascii")):
            raise AssertionError("fixture xref does not point at its object")
    streams = list(re.finditer(rb"/Length (\d+) >>\nstream\n", data))
    for stream in streams:
        end = stream.end() + int(stream.group(1))
        if data[end:end + 9] != b"endstream":
            raise AssertionError("fixture stream length is incorrect")
    return {"indirect_objects": count - 1, "streams": len(streams), "xref_verified": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--expected-version", required=True)
    parser.add_argument("--source-sha", default=os.environ.get("GITHUB_SHA"))
    parser.add_argument("--deadline-seconds", type=int, default=180)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.expected_version):
        parser.error("expected-version must be an explicit packaged release version")
    if not args.source_sha or not re.fullmatch(r"(?:[0-9a-fA-F]{40}|[0-9a-fA-F]{64})", args.source_sha):
        parser.error("supply the full source commit with --source-sha or GITHUB_SHA")
    if not 60 <= args.deadline_seconds <= 600:
        parser.error("deadline-seconds must be between 60 and 600")
    cli = args.cli.resolve(strict=True)
    if not cli.is_file():
        parser.error("cli must be a regular executable file")
    directory = args.directory.absolute()
    if directory.exists() or directory.is_symlink():
        raise FileExistsError(directory)
    directory.mkdir(parents=True, exist_ok=False)
    directory = directory.resolve()
    started = time.monotonic()
    deadline = started + args.deadline_seconds
    producer, producer_path = load_producer()
    report = {
        "schema": "sunipdf-overlay-setups-qualification", "version": 1, "status": "running",
        "expected_cli_version": args.expected_version, "source_revision": args.source_sha.lower(),
        "cli_sha256": sha(cli), "script_sha256": sha(Path(__file__)),
        "fixture_producer": {"name": producer_path.name, "sha256": sha(producer_path)},
        "pillow_version": pillow_version, "scope": SCOPE, "native_desktop_tested": False,
        "fixture_license": "CC0-1.0", "started_utc": datetime.now(timezone.utc).isoformat(),
        "limits": {"total_seconds": args.deadline_seconds, "process_seconds": PROCESS_SECONDS,
                   "transcript_bytes_per_stream": MAX_TRANSCRIPT_BYTES},
        "checks": [], "commands": [], "cases": [], "sources": [], "fault_injections": [],
        "inventory_excludes": ["report.json"],
    }
    original_hashes, protected_hashes, roles = {}, {}, {}

    def relative(path):
        return str(Path(path).relative_to(directory)).replace("\\", "/")

    def check(name, passed):
        report["checks"].append({"name": name, "passed": bool(passed)})
        if not passed:
            raise AssertionError(name)

    def remaining():
        seconds = deadline - time.monotonic()
        if seconds <= 0:
            raise TimeoutError("Overlay setup corpus exceeded its total deadline")
        return seconds

    def record(path, role, protect=True):
        roles[relative(path)] = role
        if protect:
            protected_hashes[path] = sha(path)

    def write_bytes(path, data, role, protect=True):
        with path.open("xb") as stream:
            stream.write(data)
        record(path, role, protect)
        return path

    def write_json(name, value, role="input_manifest"):
        return write_bytes(directory / name, (json.dumps(value, indent=2) + "\n").encode("utf-8"), role)

    def preserved(name):
        check(name, all(path.is_file() and sha(path) == digest for path, digest in protected_hashes.items()))

    def execute(name, arguments, reject=False, parse=True):
        budget = min(PROCESS_SECONDS, remaining())
        command = [str(cli), *map(str, arguments)]
        entry = {"name": name, "argv": command, "expected_refusal": reject,
                 "started_utc": datetime.now(timezone.utc).isoformat(), "exit_code": None}
        report["commands"].append(entry)
        stdout, stderr = (directory / f"{name}.{suffix}" for suffix in ("stdout", "stderr"))
        command_started = time.monotonic()
        process = None
        try:
            with stdout.open("xb") as out, stderr.open("xb") as err:
                process = subprocess.Popen(command, cwd=directory, stdin=subprocess.DEVNULL,
                                           stdout=out, stderr=err, env=os.environ.copy())
                try:
                    while process.poll() is None:
                        if time.monotonic() - command_started >= budget:
                            raise TimeoutError(name + ": process deadline exceeded")
                        if max(stdout.stat().st_size, stderr.stat().st_size) > MAX_TRANSCRIPT_BYTES:
                            raise RuntimeError(name + ": transcript limit exceeded")
                        time.sleep(min(0.05, max(0.001, remaining())))
                    entry["exit_code"] = process.returncode
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait(timeout=5)
                    entry["exit_code"] = process.returncode
        except Exception as error:
            entry["error"] = str(error)
            raise
        finally:
            entry["elapsed_seconds"] = round(time.monotonic() - command_started, 3)
            entry["transcripts"] = []
            for path in (stdout, stderr):
                if path.is_file():
                    record(path, "command_transcript")
                    entry["transcripts"].append({"name": relative(path), "bytes": path.stat().st_size,
                                                 "sha256": sha(path)})
        check(name + ": bounded transcripts", max(stdout.stat().st_size, stderr.stat().st_size) <= MAX_TRANSCRIPT_BYTES)
        check(name + ": process outcome", process.returncode != 0 if reject else process.returncode == 0)
        remaining()
        data = stdout.read_bytes()
        return json.loads(data) if parse and not reject else data

    def run_case(name, kind, operation, **metadata):
        before_commands, before_checks = len(report["commands"]), len(report["checks"])
        passed = False
        try:
            result = operation()
            passed = True
            return result
        finally:
            report["cases"].append({"name": name, "kind": kind, "passed": passed, **metadata,
                                    "commands": [item["name"] for item in report["commands"][before_commands:]],
                                    "checks_observed": len(report["checks"]) - before_checks})

    def stack(name, manifest, output, format):
        value = execute(name, ["revision-overlay-stack", "--manifest", manifest, "--format", format, "--out", output])
        check(name + ": new output and source preservation receipt", output.is_file()
              and value["originals_unchanged"] is True and value["format"] == format)
        record(output, "saved_setup" if format == "setup" else "rendered_png")
        return value["report"]

    def decoded(path):
        check(relative(path) + ": bounded image file", path.stat().st_size <= 4 * 1024 * 1024)
        with Image.open(path) as image:
            check(relative(path) + ": PNG canvas", image.format == "PNG" and image.size == (128, 128))
            rgba = image.convert("RGBA")
        check(relative(path) + ": opaque export", rgba.getchannel("A").getextrema() == (255, 255))
        return rgba

    def patch(name, image, center, expected):
        x, y = center
        check(name, all(abs(actual - wanted) <= 1
                        for dx in range(-2, 3) for dy in range(-2, 3)
                        for actual, wanted in zip(image.getpixel((x + dx, y + dy)), expected)))

    def copy_bundle(name, setup, count, skip=None, fault=False):
        bundle = directory / name
        bundle.mkdir(exist_ok=False)
        copied = write_bytes(bundle / "setup.json", setup.read_bytes(), "relocated_setup")
        for index, source in enumerate(original_hashes):
            if index < count and index != skip:
                write_bytes(bundle / source.name, source.read_bytes(),
                            "fault_source_copy" if fault else "relocated_source", protect=not fault)
        return bundle, copied

    def refuse(name, manifest, output=None, format="setup"):
        output = output or directory / f"{name}.output.json"
        existed = output.exists() or output.is_symlink()
        previous = sha(output) if output.is_file() else None
        execute(name, ["--overwrite", "revision-overlay-stack", "--manifest", manifest,
                       "--format", format, "--out", output], reject=True, parse=False)
        error = json.loads((directory / f"{name}.stderr").read_text(encoding="utf-8"))
        expected_code = "already_exists" if existed else "io" if name == "refuse-missing-hidden" else "other"
        check(name + ": typed refusal rather than crash or argument failure", error.get("error") == expected_code
              and isinstance(error.get("message"), str) and bool(error["message"])
              and not (directory / f"{name}.stdout").read_bytes().strip())
        report["commands"][-1]["error_code"] = error["error"]
        check(name + ": no new output or replacement", (output.is_file() and sha(output) == previous)
              if existed else not output.exists() and not output.is_symlink())
        preserved(name + ": all protected originals and evidence unchanged")

    try:
        version = execute("version", ["--version"], parse=False).decode("utf-8").strip()
        report["cli_version"] = version
        check("exact packaged CLI version", version == "sunipdf-cli " + args.expected_version)
        for index, (crop, rotation, rectangle, pages, displayed) in enumerate(GEOMETRY):
            data = producer.original_pdf(crop, rotation, rectangle, pages)
            structure = validate_fixture_xref(data)
            path = write_bytes(directory / f"layer-{index + 1}.pdf", data, "original_source")
            original_hashes[path] = sha(path)
            report["sources"].append({"name": path.name, "sha256": sha(path), "crop_box_user_pt": crop,
                                      "rotation_degrees": rotation, "page": pages, "page_count": pages,
                                      "display_size_pt": displayed, "rectangle_display_top_left": rectangle,
                                      **structure})
        colors, opacities = [[220, 20, 40], [15, 160, 210], [40, 180, 60], [250, 130, 20]], [1, 1, 0.5, 0.75]
        alignments = [producer.manual(), producer.manual((48, 0)),
                      {"mode": "two_points", "base": [[16, 64], [48, 64]], "revision": [[8, 8], [24, 8]]},
                      producer.manual((112, 48), 90)]
        all_layers = [{"path": path.name, "page": GEOMETRY[index][3], "color": colors[index],
                       "opacity": opacities[index], "visible": index != 2, "alignment": alignments[index]}
                      for index, path in enumerate(original_hashes)]
        # Optional CLI hashes are preserved as an input contract; saving binds every source.
        all_layers[0]["expected_sha256"] = list(original_hashes.values())[0]
        all_layers[1]["expected_sha256"] = list(original_hashes.values())[1].upper()
        saved_setups, baselines = {}, {}

        def round_trip(count):
            layers = copy.deepcopy(all_layers[:count])
            incoming = write_json(f"layers-{count}.input.json", {"version": 1, "long_side": 128, "layers": layers})
            baseline = directory / f"layers-{count}.original.png"
            before = stack(f"layers-{count}-original", incoming, baseline, "png")
            pixels = decoded(baseline)
            setup = directory / f"layers-{count}.setup.json"
            saved_report = stack(f"layers-{count}-save", incoming, setup, "setup")
            expected_layers = copy.deepcopy(layers)
            for index, layer in enumerate(expected_layers):
                layer["expected_sha256"] = list(original_hashes.values())[index]
            expected = {"version": 1, "long_side": 128, "layers": expected_layers}
            value = json.loads(setup.read_text(encoding="utf-8"))
            check(f"layers-{count}: complete exact source-bound setup", value == expected)
            check(f"layers-{count}: no environment or credential fields", all(
                "password_env" not in layer and "password" not in layer for layer in value["layers"]))
            check(f"layers-{count}: contained relative paths", all(not Path(layer["path"]).is_absolute()
                  and ".." not in Path(layer["path"]).parts for layer in value["layers"]))
            check(f"layers-{count}: save uses the same reviewed sources", saved_report == before)
            for index, source in enumerate(original_hashes):
                if index >= count:
                    break
                actual, spec = before["layers"][index]["source"], GEOMETRY[index]
                check(f"layers-{count}: layer-{index + 1} source geometry", actual["source_sha256"] == original_hashes[source]
                      and Path(actual["path"]).samefile(source) and actual["page"] == spec[3]
                      and actual["page_count"] == spec[3] and actual["crop_box_user_pt"] == spec[0]
                      and actual["rotation_degrees"] == spec[1]
                      and [actual["display_width_pt"], actual["display_height_pt"]] == spec[4])
            for index, center in enumerate([(32, 32), (80, 32), (32, 80), (80, 80)]):
                wanted = producer.interior_color([layers[index]]) if index < count and layers[index]["visible"] else (255, 255, 255, 255)
                patch(f"layers-{count}: independent interior at region {index + 1}", pixels, center, wanted)
            patch(f"layers-{count}: known white canvas", pixels, (112, 112), (255, 255, 255, 255))
            reopened = directory / f"layers-{count}.reopened.png"
            stack(f"layers-{count}-reopen", setup, reopened, "png")
            check(f"layers-{count}: all reopened RGBA pixels identical", decoded(reopened).tobytes() == pixels.tobytes())
            resaved = directory / f"layers-{count}.resaved.json"
            stack(f"layers-{count}-resave", setup, resaved, "setup")
            check(f"layers-{count}: resaving retains every setup field", json.loads(resaved.read_text(encoding="utf-8")) == expected)
            bundle, relocated = copy_bundle(f"relocated-{count}", setup, count)
            moved_png = bundle / "reopened.png"
            moved_report = stack(f"layers-{count}-relocate", relocated, moved_png, "png")
            check(f"layers-{count}: all relocated RGBA pixels identical", decoded(moved_png).tobytes() == pixels.tobytes())
            check(f"layers-{count}: relocated sources resolve inside moved bundle", all(
                Path(layer["source"]["path"]).samefile(bundle / expected_layers[index]["path"])
                for index, layer in enumerate(moved_report["layers"])))
            preserved(f"layers-{count}: originals and earlier evidence remain unchanged")
            saved_setups[count], baselines[count] = setup, baseline

        for count in (2, 3, 4):
            run_case(f"round-trip-{count}-layers", "round_trip", lambda count=count: round_trip(count), layers=count,
                     hidden_layers=0 if count == 2 else 1)

        saved = saved_setups[4]
        baseline_manifest = json.loads(saved.read_text(encoding="utf-8"))
        stale_bundle, stale_manifest = copy_bundle("changed-hidden", saved, 4, fault=True)
        hidden = stale_bundle / "layer-3.pdf"
        before_mutation = sha(hidden)
        changed_pdf = producer.original_pdf(GEOMETRY[2][0], GEOMETRY[2][1], [30, 8, 46, 24], GEOMETRY[2][3])
        validate_fixture_xref(changed_pdf)
        hidden.write_bytes(changed_pdf)
        report["fault_injections"].append({"name": relative(hidden), "mutation": "replace copied hidden PDF with distinct original drawing",
                                           "before_sha256": before_mutation, "after_sha256": sha(hidden)})
        for path in stale_bundle.glob("*.pdf"):
            record(path, "fault_source_copy")
        run_case("refuse-changed-hidden", "refusal", lambda: refuse("refuse-changed-hidden", stale_manifest))
        missing_bundle, missing_manifest = copy_bundle("missing-hidden", saved, 4, skip=2)
        report["fault_injections"].append({"name": relative(missing_bundle / "layer-3.pdf"), "mutation": "omit copied hidden source"})
        run_case("refuse-missing-hidden", "refusal", lambda: refuse("refuse-missing-hidden", missing_manifest))

        for name, mutate in [
            ("invalid-version", lambda value: value.update(version=2)),
            ("unknown-root-field", lambda value: value.update(unknown=True)),
            ("unknown-layer-field", lambda value: value["layers"][1].update(unknown=True)),
            ("bad-sha", lambda value: value["layers"][2].update(expected_sha256="not-a-sha256")),
            ("remote-path", lambda value: value["layers"][2].update(path="https://example.invalid/source.pdf")),
            ("unc-path", lambda value: value["layers"][2].update(path="//server/source.pdf")),
        ]:
            changed = copy.deepcopy(baseline_manifest)
            mutate(changed)
            manifest = write_json(f"refuse-{name}.input.json", changed, "invalid_manifest")
            run_case("refuse-" + name, "refusal", lambda name=name, manifest=manifest: refuse("refuse-" + name, manifest))
        oversized = write_bytes(directory / "oversized.input.json", b" " * (MAX_MANIFEST_BYTES + 1), "invalid_manifest")
        run_case("refuse-oversized", "refusal", lambda: refuse("refuse-oversized", oversized))
        run_case("refuse-existing-setup", "refusal", lambda: refuse("refuse-existing-setup", saved, saved))
        run_case("refuse-source-output", "refusal", lambda: refuse("refuse-source-output", saved, next(iter(original_hashes))))
        run_case("refuse-existing-png", "refusal", lambda: refuse("refuse-existing-png", saved, baselines[4]))
        preserved("all original PDFs and protected outputs are byte-for-byte unchanged")
        check("all four source rotations are exercised", sorted(item["rotation_degrees"] for item in report["sources"]) == [0, 90, 180, 270])
        remaining()
        report["status"] = "passed"
    except Exception as error:
        report["status"], report["error"] = "failed", str(error)
        raise
    finally:
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        report["elapsed_seconds"] = round(time.monotonic() - started, 3)
        report["source_integrity"] = [{"name": relative(path), "expected_sha256": digest,
                                       "observed_sha256": sha(path) if path.is_file() else None}
                                      for path, digest in original_hashes.items()]
        report["originals_unchanged"] = all(item["expected_sha256"] == item["observed_sha256"]
                                            for item in report["source_integrity"])
        if not report["originals_unchanged"]:
            report["status"], report["error"] = "failed", "An original source PDF changed or disappeared"
        report["inventory"] = [{"name": relative(path), "role": roles.get(relative(path), "unclassified_artifact"),
                                "bytes": path.stat().st_size, "sha256": sha(path)}
                               for path in sorted(directory.rglob("*")) if path.is_file() and path.name != "report.json"]
        report["counts"] = {
            "checks_observed": len(report["checks"]),
            "checks_passed": sum(item["passed"] for item in report["checks"]),
            "commands_started": len(report["commands"]),
            "commands_completed": sum(item["exit_code"] is not None for item in report["commands"]),
            "refusal_commands_completed": sum(item["expected_refusal"] and item["exit_code"] is not None for item in report["commands"]),
            "cases_observed": len(report["cases"]), "cases_passed": sum(item["passed"] for item in report["cases"]),
            "original_source_pdfs": len(report["sources"]), "artifact_files": len(report["inventory"]),
        }
        with (directory / "report.json").open("x", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2)
            stream.write("\n")
    if report["status"] != "passed":
        raise AssertionError(report.get("error", "Overlay setup corpus did not pass"))
    print(json.dumps({"status": report["status"], "counts": report["counts"], "report": str(directory / "report.json")}))


if __name__ == "__main__":
    main()
