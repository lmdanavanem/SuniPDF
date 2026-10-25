"""Qualify explicit formula unit conversion with the packaged CLI and an independent decimal oracle."""
import argparse
from collections import Counter
import csv
from datetime import datetime, timezone
from decimal import Decimal, localcontext, ROUND_HALF_UP
from fractions import Fraction
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import time
import xml.etree.ElementTree as ET
import zipfile


SCOPE = ("Original synthetic PDF, packaged CLI quantity formula conversion, JSON/CSV values, "
         "issue report PDF text and formula-free workbook cells. No native desktop interaction, "
         "external spreadsheet recalculation or private-plan accuracy is claimed.")
PROCESS_SECONDS = 30
MAX_TRANSCRIPT_BYTES = 2_000_000
MAX_ARTIFACT_BYTES = 32 * 1024 * 1024
METRES = {"mm": Fraction(1, 1000), "cm": Fraction(1, 100), "m": Fraction(1),
          "in": Fraction(127, 5000), "ft": Fraction(381, 1250)}
INPUTS = {"mm": "0.0005", "cm": "-2.345678", "m": "1.234567", "in": "0", "ft": "2.5"}
QUANTITIES = [(Fraction(1), "each"), (Fraction(10), "m"), (Fraction(90), "deg")]


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def quantized(value, places):
    """Decimal's decimal rounding, independent of the application's scaled-integer implementation."""
    value = Fraction(value)
    with localcontext() as context:
        context.prec = 90
        decimal = Decimal(value.numerator) / Decimal(value.denominator)
        result = decimal.quantize(Decimal(1).scaleb(-places), rounding=ROUND_HALF_UP)
    return Fraction(result)


def fixed(value, places):
    rounded = quantized(value, places)
    with localcontext() as context:
        context.prec = 90
        decimal = Decimal(rounded.numerator) / Decimal(rounded.denominator)
        return format(abs(decimal) if not rounded else decimal, f".{places}f")


def unit_scale(unit):
    dimension = int(unit[-1]) if unit[-1:].isdigit() else 1
    base = unit[:-1] if dimension != 1 else unit
    return dimension, METRES[base] ** dimension


def conversion(value, source, target):
    source_dimension, source_scale = unit_scale(source)
    target_dimension, target_scale = unit_scale(target)
    if source_dimension != target_dimension:
        raise ValueError("independent oracle requires equal geometric dimensions")
    result = quantized(Fraction(value) * source_scale / target_scale, 6)
    if abs(result) > 10 ** 12:
        raise OverflowError("independent decimal limit exceeded")
    return result


def good(value, unit, precision=6):
    return {"value": fixed(value, precision), "unit": unit, "error": None}


def bad(code):
    return {"value": None, "unit": None, "error": code}


def vector(name, expression, expected, precision=6, quantity_unit=None):
    return {"column": {"name": name, "expression": expression, "quantity_unit": quantity_unit,
                       "rounding_decimals": precision}, "expected_rows": expected}


def field_vector(name, expression, value, unit, precision=6):
    return vector(name, expression, [good(value, unit, precision), bad("missing_field"), bad("missing_field")], precision)


def vectors():
    pairs = []
    for dimension in (1, 2, 3):
        suffix = "" if dimension == 1 else str(dimension)
        for source in METRES:
            for target in METRES:
                source_unit, target_unit = source + suffix, target + suffix
                pairs.append(field_vector(f"{source_unit} to {target_unit}", f"convert([{source_unit}], {target_unit})",
                                          conversion(INPUTS[source], source_unit, target_unit), target_unit))
    rounding = [vector(f"Length precision {precision}", "convert(quantity, ft)",
                       [bad("unit_mismatch"), good(conversion(10, "m", "ft"), "ft", precision), bad("unit_mismatch")],
                       precision, "m") for precision in range(7)]
    boundaries = [
        field_vector("Tiny positive", "convert([mm], m)", conversion(INPUTS["mm"], "mm", "m"), "m"),
        field_vector("Tiny negative", "convert(-[mm], m)", conversion(-Fraction(INPUTS["mm"]), "mm", "m"), "m"),
        field_vector("Tiny zero", "convert([mm3], m3)", conversion(INPUTS["mm"], "mm3", "m3"), "m3"),
        field_vector("Nested rounding", "convert(convert([mm], m), mm)",
                     conversion(conversion(INPUTS["mm"], "mm", "m"), "m", "mm"), "mm"),
        field_vector("Positive half", "convert([m] / 1.234567 * 1.005, m)", Fraction("1.005"), "m", 2),
        field_vector("Negative half", "convert([m] / 1.234567 * -1.005, m)", Fraction("-1.005"), "m", 2),
        field_vector("Positive limit", "convert([ft] * 400000000000, ft)", 10 ** 12, "ft"),
        field_vector("Negative limit", "convert(-[ft] * 400000000000, ft)", -(10 ** 12), "ft"),
    ]
    errors = [
        vector("Scalar refused", "convert(1, m)", [bad("unit_mismatch")] * 3),
        vector("Count refused", "convert(quantity, m)", [bad("unit_mismatch")] * 3, quantity_unit="each"),
        vector("Angle refused", "convert(quantity, m)", [bad("unit_mismatch")] * 3, quantity_unit="deg"),
        vector("Dimension refused", "convert([m2], m)", [bad("unit_mismatch"), bad("missing_field"), bad("missing_field")]),
        vector("Missing field", "convert([Absent], m)", [bad("missing_field")] * 3),
        vector("Text field", "convert([convert], m)", [bad("non_numeric_field"), bad("missing_field"), bad("missing_field")]),
        vector("Zero divisor", "convert([m] / 0, ft)", [bad("division_by_zero"), bad("missing_field"), bad("missing_field")]),
        vector("Conversion overflow", "convert([ft] * 400000000000, mm)", [bad("overflow"), bad("missing_field"), bad("missing_field")]),
        vector("Exact source guard", "convert(quantity, m)", [bad("unit_mismatch")] * 3, quantity_unit="cm"),
    ]
    legacy = [
        vector("Legacy negative", "-1.005", [good(Fraction("-1.005"), None, 2)] * 3, 2),
        vector("Legacy decimal", "1 / 6", [good(quantized(Fraction(1, 6), 6), None)] * 3),
        vector("Legacy count", "quantity * 2", [good(2, "each", 2), bad("unit_mismatch"), bad("unit_mismatch")], 2, "each"),
        vector("Legacy length", "quantity * 2", [bad("unit_mismatch"), good(20, "m", 2), bad("unit_mismatch")], 2, "m"),
        vector("Legacy angle", "quantity / 2", [bad("unit_mismatch"), bad("unit_mismatch"), good(45, "deg", 2)], 2, "deg"),
        vector("No implicit conversion", "[m] + [cm]", [bad("unit_mismatch"), bad("missing_field"), bad("missing_field")]),
        vector("Field named convert", "[convert]", [bad("non_numeric_field"), bad("missing_field"), bad("missing_field")]),
    ]
    groups = [("pairs", pairs), ("rounding", rounding), ("boundaries", boundaries), ("errors", errors), ("legacy", legacy)]
    return [(f"{name}-{offset // 8 + 1}", values[offset:offset + 8], name != "legacy")
            for name, values in groups for offset in range(0, len(values), 8)]


def invalid_configs():
    expressions = {
        "missing-target": "convert([m])", "empty-target": "convert([m],)",
        "empty-expression": "convert(,m)", "quoted-target": 'convert([m], "m")',
        "field-target": "convert([m], [m])", "extra-argument": "convert([m], m, cm)",
        "wrong-case-function": "Convert([m],m)", "wrong-case-unit": "convert([m],M)",
        "count-target": "convert([m],each)", "angle-target": "convert([m],deg)",
        "unknown-target": "convert([m],km)", "fourth-dimension": "convert([m],m4)",
        "trailing-code": "convert([m],m);1", "missing-close": "convert([m],m",
        "expression-bytes": "1" * 513,
        "expression-depth": "convert(" * 20 + "[m]" + ",m)" * 20,
        "expression-tokens": "+".join(["convert([m],m)"] * 19),
    }
    result = [(name, [{"name": "Refused", "expression": expression, "quantity_unit": None, "rounding_decimals": 6}])
              for name, expression in expressions.items()]
    definition = {"name": "Refused", "expression": "convert([m],m)", "quantity_unit": None, "rounding_decimals": 6}
    result.extend([
        ("rounding-seven", [dict(definition, rounding_decimals=7)]),
        ("missing-source-unit", [dict(definition, expression="convert(quantity,m)")]),
        ("unknown-field", [dict(definition, unexpected=True)]),
        ("duplicate-name", [definition, definition]),
        ("ninth-column", [dict(definition, name=f"Column {index}") for index in range(9)]),
        ("oversized-config", b" " * 65537),
    ])
    return result


def coverage_plan():
    batches = vectors()
    invalid = invalid_configs()
    return {"formula_vectors": sum(len(batch) for _, batch, _ in batches),
            "same_dimension_pairs": sum(len(batch) for name, batch, _ in batches if name.startswith("pairs-")),
            "formula_batches": len(batches), "invalid_config_cases": len(invalid),
            "existing_output_cases": 6, "stale_source_cases": 3,
            "expected_commands": 1 + 6 + 2 * len(batches) + 8 + len(invalid) + 6 + 3,
            "expected_cases": 1 + len(batches) + 1 + len(invalid) + 6 + 3}


def load_fixture():
    path = Path(__file__).with_name("test-overlay-stack.py")
    spec = importlib.util.spec_from_file_location("formula_original_fixture", path)
    producer = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(producer)
    data = producer.original_pdf([0, 0, 160, 160], 0, [10, 10, 15, 15])
    start = re.search(rb"startxref\n(\d+)\n%%EOF\n$", data)
    if start is None or data[int(start.group(1)):].splitlines()[0] != b"xref":
        raise AssertionError("Original fixture xref is invalid")
    entries = data[int(start.group(1)):].splitlines()
    count = int(entries[1].split()[1])
    for number, entry in enumerate(entries[3:count + 2], 1):
        if not data[int(entry.split()[0]):].startswith(f"{number} 0 obj\n".encode()):
            raise AssertionError("Original fixture object offset is invalid")
    for stream in re.finditer(rb"/Length (\d+) >>\nstream\n", data):
        end = stream.end() + int(stream.group(1))
        if data[end:end + 9] != b"endstream":
            raise AssertionError("Original fixture stream length is invalid")
    return data, path


def self_test():
    references = [("1", "in", "mm", "25.400000"), ("1", "ft2", "m2", "0.092903"),
                  ("1", "ft3", "m3", "0.028317"), ("1", "m3", "ft3", "35.314667"),
                  ("0.0005", "mm", "m", "0.000001"), ("-0.0005", "mm", "m", "-0.000001"),
                  ("0.0005", "mm3", "m3", "0.000000"), ("0", "in3", "ft3", "0.000000")]
    for value, source, target, expected in references:
        assert fixed(conversion(value, source, target), 6) == expected
    assert fixed(Fraction("1.005"), 2) == "1.01" and fixed(Fraction("-1.005"), 2) == "-1.01"
    assert fixed(conversion(conversion("0.0005", "mm", "m"), "m", "mm"), 6) == "0.001000"
    assert [fixed(conversion(10, "m", "ft"), p) for p in range(7)] == ["33", "32.8", "32.81", "32.808", "32.8084", "32.80840", "32.808399"]
    try:
        conversion(10 ** 12, "ft", "mm")
        raise AssertionError("Expected oracle overflow")
    except OverflowError:
        pass
    data, producer = load_fixture()
    batches = vectors()
    assert len(batches) == 15 and sum(len(values) for _, values, _ in batches) == 106
    assert sum(len(values) for name, values, _ in batches if name.startswith("pairs")) == 75
    assert len(invalid_configs()) == 23
    first = {"source_sha256": "a" * 64, "rows": [{"quantity": "1.25"}]}
    second = {"source_sha256": "b" * 64, "rows": [{"quantity": "1.25"}]}
    assert same_quantities(first, second)
    assert not same_quantities(first, dict(second, rows=[{"quantity": "1.26"}]))
    assert not same_quantities(first, dict(second, source_sha256="invalid"))
    left = {"quantities": first, "provenance": {"snapshot_sha256": "a" * 64, "saved_file_sha256": "c" * 64}}
    right = {"quantities": second, "provenance": {"snapshot_sha256": "b" * 64, "saved_file_sha256": "c" * 64}}
    assert same_issue(left, right)
    assert not same_issue(left, dict(right, provenance=dict(right["provenance"], snapshot_sha256="a" * 64)))
    assert not same_issue(left, dict(right, provenance=dict(right["provenance"], saved_file_sha256="d" * 64)))
    print(json.dumps({"status": "passed", "scope": "offline decimal oracle, inventory and original fixture checks only",
                      "product_executed": False, "reference_conversions": len(references), "formula_vectors": 106,
                      "same_dimension_pairs": 75, "formula_batches": len(batches), "invalid_configs": 23,
                      "planned_runtime_coverage": coverage_plan(),
                      "fixture_sha256": hashlib.sha256(data).hexdigest(), "fixture_producer_sha256": sha(producer)}))


class Corpus:
    def __init__(self, cli, directory, version, source, seconds):
        self.cli, self.directory = cli, directory
        self.started = time.monotonic()
        self.deadline = self.started + seconds
        self.protected, self.roles = {}, {}
        self.report = {"schema": "sunipdf-formula-unit-conversion-qualification", "version": 1, "status": "running",
                       "expected_cli_version": version, "source_revision": source.lower(), "cli_sha256": sha(cli),
                       "script_sha256": sha(Path(__file__)), "scope": SCOPE, "native_desktop_tested": False,
                       "fixture_license": "CC0-1.0", "started_utc": datetime.now(timezone.utc).isoformat(),
                       "limits": {"total_seconds": seconds, "process_seconds": PROCESS_SECONDS,
                                  "transcript_bytes_per_stream": MAX_TRANSCRIPT_BYTES, "artifact_file_bytes": MAX_ARTIFACT_BYTES},
                       "oracle": {"method": "Fraction SI ratios and Decimal ROUND_HALF_UP", "precision": 90,
                                  "arithmetic_places": 6, "source": "independent decimal constants; no application calculation code"},
                       "checks": [], "commands": [], "cases": [], "inventory_excludes": ["report.json"]}

    def check(self, name, condition):
        self.report["checks"].append({"name": name, "passed": bool(condition)})
        if not condition: raise AssertionError(name)

    def remaining(self):
        value = self.deadline - time.monotonic()
        if value <= 0: raise TimeoutError("Formula conversion corpus total deadline exceeded")
        return value

    def record(self, path, role):
        path = Path(path)
        if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_ARTIFACT_BYTES:
            raise RuntimeError("Missing, linked or oversized corpus artifact: " + path.name)
        self.roles[path.relative_to(self.directory).as_posix()] = role
        self.protected[path] = sha(path)

    def write(self, name, value, role):
        path = self.directory / name
        data = value if isinstance(value, bytes) else (json.dumps(value, indent=2) + "\n").encode()
        with path.open("xb") as out: out.write(data)
        self.record(path, role)
        return path

    def preserved(self, name):
        self.check(name, all(path.is_file() and sha(path) == expected for path, expected in self.protected.items()))

    def case(self, name, kind, operation, **metadata):
        command_start, check_start = len(self.report["commands"]), len(self.report["checks"])
        passed = False
        try:
            value = operation()
            passed = True
            return value
        finally:
            self.report["cases"].append({"name": name, "kind": kind, "passed": passed, **metadata,
                                         "commands": [row["name"] for row in self.report["commands"][command_start:]],
                                         "checks_observed": len(self.report["checks"]) - check_start})

    def execute(self, name, arguments, code=0, parse=True):
        budget = min(PROCESS_SECONDS, self.remaining())
        argv = [str(self.cli), *map(str, arguments)]
        entry = {"name": name, "argv": argv, "expected_exit_code": code, "expected_refusal": code in (1, 6), "exit_code": None,
                 "started_utc": datetime.now(timezone.utc).isoformat()}
        self.report["commands"].append(entry)
        stdout, stderr = [self.directory / (name + "." + extension) for extension in ("stdout", "stderr")]
        started = time.monotonic()
        process = None
        try:
            with stdout.open("xb") as out, stderr.open("xb") as err:
                process = subprocess.Popen(argv, cwd=self.directory, stdin=subprocess.DEVNULL,
                                           stdout=out, stderr=err, env=os.environ.copy())
                try:
                    while process.poll() is None:
                        if time.monotonic() - started >= budget: raise TimeoutError(name + ": process deadline exceeded")
                        if max(stdout.stat().st_size, stderr.stat().st_size) > MAX_TRANSCRIPT_BYTES:
                            raise RuntimeError(name + ": transcript limit exceeded")
                        time.sleep(min(.05, self.remaining()))
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait(timeout=5)
                    entry["exit_code"] = process.returncode
        finally:
            entry["elapsed_seconds"] = round(time.monotonic() - started, 3)
            entry["transcripts"] = []
            for path in (stdout, stderr):
                if path.is_file():
                    self.record(path, "command_transcript")
                    entry["transcripts"].append({"name": path.name, "bytes": path.stat().st_size, "sha256": sha(path)})
        self.check(name + ": bounded transcripts", max(stdout.stat().st_size, stderr.stat().st_size) <= MAX_TRANSCRIPT_BYTES)
        self.check(name + ": exact process outcome", entry["exit_code"] == code)
        self.remaining()
        if code in (1, 6):
            error = json.loads(stderr.read_bytes())
            expected = "already_exists" if code == 6 else "other"
            self.check(name + ": typed refusal", error.get("error") == expected and isinstance(error.get("message"), str)
                       and bool(error["message"]) and not stdout.read_bytes().strip())
            entry["error_code"] = error["error"]
            return error
        self.check(name + ": no error transcript", not stderr.read_bytes().strip())
        return json.loads(stdout.read_bytes()) if parse else stdout.read_bytes()

    def finish(self):
        self.report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        self.report["elapsed_seconds"] = round(time.monotonic() - self.started, 3)
        self.report["originals_unchanged"] = all(path.is_file() and sha(path) == value for path, value in self.protected.items())
        if not self.report["originals_unchanged"]:
            self.report["status"] = "failed"
        self.report["inventory"] = [{"name": path.relative_to(self.directory).as_posix(),
                                     "role": self.roles.get(path.relative_to(self.directory).as_posix(), "unclassified"),
                                     "bytes": path.stat().st_size, "sha256": sha(path)}
                                    for path in sorted(self.directory.rglob("*")) if path.is_file() and path.name != "report.json"]
        commands, checks, cases = self.report["commands"], self.report["checks"], self.report["cases"]
        self.report["counts"] = {"commands_started": len(commands), "commands_completed": sum(row["exit_code"] is not None for row in commands),
                                 "refusal_commands": sum(row["expected_refusal"] for row in commands),
                                 "checks_observed": len(checks), "checks_passed": sum(row["passed"] for row in checks),
                                 "cases_observed": len(cases), "cases_passed": sum(row["passed"] for row in cases),
                                 "artifact_files": len(self.report["inventory"])}
        with (self.directory / "report.json").open("x", encoding="utf-8") as out:
            json.dump(self.report, out, indent=2)
            out.write("\n")


def expected_group(item):
    column, values = item["column"], item["expected_rows"]
    errors = Counter(value["error"] for value in values if value["error"] is not None)
    successful = [value for value in values if value["error"] is None]
    totals = []
    for unit in sorted({value["unit"] for value in successful}, key=lambda value: value or ""):
        contributing = [value for value in successful if value["unit"] == unit]
        totals.append({"unit": unit, "rounding_decimals": column["rounding_decimals"],
                       "amount": fixed(sum((Fraction(value["value"]) for value in contributing), Fraction()), column["rounding_decimals"]),
                       "contributing_rows": len(contributing)})
    return {"formula_name": column["name"], "group": None, "matched_rows": 3,
            "contributing_rows": len(successful), "error_rows": 3 - len(successful),
            "errors": dict(errors), "complete": not errors, "totals": totals}


def valid_snapshot(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def same_quantities(left, right):
    # PDFium may assign a fresh snapshot identifier to separate saves of the same source.
    # Compare every report value while retaining each invocation's own receipt binding.
    return (valid_snapshot(left.get("source_sha256")) and valid_snapshot(right.get("source_sha256"))
            and {key: value for key, value in left.items() if key != "source_sha256"}
            == {key: value for key, value in right.items() if key != "source_sha256"})


def same_provenance(left, right):
    return (valid_snapshot(left.get("snapshot_sha256")) and valid_snapshot(right.get("snapshot_sha256"))
            and {key: value for key, value in left.items() if key != "snapshot_sha256"}
            == {key: value for key, value in right.items() if key != "snapshot_sha256"})


def same_issue(left, right):
    return (same_quantities(left["quantities"], right["quantities"])
            and same_provenance(left["provenance"], right["provenance"])
            and all(model["quantities"]["source_sha256"] == model["provenance"]["snapshot_sha256"]
                    for model in (left, right))
            and {key: value for key, value in left.items() if key not in ("quantities", "provenance")}
            == {key: value for key, value in right.items() if key not in ("quantities", "provenance")})


def validate_table(run, name, value, batch, explicit, baseline):
    definitions = [item["column"] for item in batch]
    run.check(name + ": exact formula definitions and policy", value["schema"] == "sunipdf-quantity-table"
              and value["version"] == 2 and value["formulas"] == definitions
              and value["unit_conversion"] == ("price_basis_and_explicit_formulas" if explicit else "price_basis_only")
              and value["formula_arithmetic"] == "decimal_6_places_half_away_from_zero_each_operation"
              and value["formula_subtotal_policy"] == "sum_rounded_row_results_by_formula_group_unit_and_precision")
    run.check(name + ": source snapshot and original rows retained", valid_snapshot(value["source_sha256"])
              and value["rows"] == baseline["rows"])
    expected_keys = {(1, index) for index in range(3)}
    actual_rows = {(row["page"], row["index"]): row["values"] for row in value["formula_rows"]}
    run.check(name + ": complete formula row inventory", len(value["formula_rows"]) == 3 and set(actual_rows) == expected_keys
              and all(set(row) == {column["name"] for column in definitions} for row in actual_rows.values()))
    groups = {group["formula_name"]: group for group in value["summary"]["formula_groups"]}
    run.check(name + ": complete formula group inventory", len(groups) == len(batch)
              and len(value["summary"]["formula_groups"]) == len(batch))
    for item in batch:
        column = item["column"]
        for index, expected in enumerate(item["expected_rows"]):
            run.check(name + f": {column['name']} row {index} exact decimal/unit/error", actual_rows[(1, index)][column["name"]] == expected)
        run.check(name + f": {column['name']} independently summed coverage", groups[column["name"]] == expected_group(item))
    run.check(name + ": formula completeness is honest", value["formula_evaluation_complete"] is all(
        cell["error"] is None for item in batch for cell in item["expected_rows"]))


def validate_csv(run, name, path, batch, explicit, snapshot):
    with path.open(encoding="utf-8", newline="") as stream:
        rows = list(csv.DictReader(stream))
    actual_rows = [row for row in rows if row["record_type"] == "row"]
    run.check(name + ": three source-bound CSV rows", len(actual_rows) == 3
              and {(row["page"], row["index"]) for row in actual_rows} == {("1", str(index)) for index in range(3)}
              and valid_snapshot(snapshot) and all(row["source_sha256"] == snapshot for row in actual_rows))
    by_index = {int(row["index"]): row for row in actual_rows}
    for item in batch:
        key = item["column"]["name"]
        for index, expected in enumerate(item["expected_rows"]):
            run.check(name + f": {key} row {index} exact CSV cells", all(by_index[index][f"formula:{key}:{part}"] == (expected[part] or "")
                       for part in ("value", "unit", "error")))
        coverage = [row for row in rows if row["record_type"] == "formula_coverage" and row["formula_name"] == key]
        totals = [row for row in rows if row["record_type"] == "formula_subtotal" and row["formula_name"] == key]
        group = expected_group(item)
        run.check(name + f": {key} CSV coverage and exact subtotal", len(coverage) == 1
                  and coverage[0]["formula_contributing_rows"] == str(group["contributing_rows"])
                  and coverage[0]["formula_error_rows"] == str(group["error_rows"])
                  and json.loads(coverage[0]["formula_errors"]) == group["errors"]
                  and len(totals) == len(group["totals"])
                  and all(row["formula_subtotal"] == expected["amount"] and row["unit"] == (expected["unit"] or "")
                          for row, expected in zip(totals, group["totals"])))
    metadata = [row for row in rows if row["record_type"] == "metadata"]
    run.check(name + ": CSV calculation policy", len(metadata) == 1 and json.loads(metadata[0]["export_options"])["unit_conversion"]
              == ("price_basis_and_explicit_formulas" if explicit else "price_basis_only"))
    count = by_index[0]
    run.check(name + ": signed numeric inputs and escaped free text", count["custom:cm:value"] == INPUTS["cm"]
              and count["custom:cm:unit"] == "cm" and count["custom:convert:value"] == "'=SUM(A1)")


def validate_workbook(run, path, batch):
    namespace = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
    with zipfile.ZipFile(path) as archive:
        entries = archive.infolist()
        run.check("workbook: bounded unique contained ZIP members", len(entries) <= 32 and len({entry.filename for entry in entries}) == len(entries)
                  and sum(entry.file_size for entry in entries) <= 32 * 1024 * 1024
                  and all(not entry.filename.startswith(("/", "\\")) and ".." not in Path(entry.filename).parts for entry in entries))
        xml = {entry.filename: ET.fromstring(archive.read(entry)) for entry in entries if entry.filename.endswith((".xml", ".rels"))}
        run.check("workbook: no spreadsheet formulas or external relationships", all(
            element.tag != namespace + "f" and element.get("TargetMode") != "External"
            for root in xml.values() for element in root.iter()))
        root = xml["xl/worksheets/sheet1.xml"]
        rows = []
        for row in root.findall(".//" + namespace + "row"):
            rows.append(["".join(cell.itertext()) for cell in row.findall(namespace + "c")])
        run.check("workbook: exact source row inventory", len(rows) == 4)
        headers = rows[0]
        dictionaries = [dict(zip(headers, row)) for row in rows[1:]]
        by_index = {int(row["Index"]): row for row in dictionaries}
        run.check("workbook: all annotation indices", set(by_index) == {0, 1, 2})
        for item in batch:
            name = item["column"]["name"]
            for index, expected in enumerate(item["expected_rows"]):
                row = by_index[index]
                run.check(f"workbook: {name} row {index} exact value/unit/error", row[name + ": exact"] == (expected["value"] or "")
                          and row[name + ": unit"] == (expected["unit"] or "") and row[name + ": error"] == (expected["error"] or ""))


def run_corpus(run):
    run.report["planned_coverage"] = coverage_plan()
    version = run.execute("version", ["--version"], parse=False).decode("utf-8").strip()
    run.report["cli_version"] = version
    run.check("exact packaged CLI version", version == "sunipdf-cli " + run.report["expected_cli_version"])
    data, producer = load_fixture()
    original = run.write("original.pdf", data, "original_source")
    run.report["fixture_producer"] = {"name": producer.name, "sha256": sha(producer)}
    run.report["fixture"] = {"name": original.name, "sha256": sha(original), "page_count": 1,
                             "media_box": [0, 0, 160, 160], "crop_box": [0, 0, 160, 160], "rotation": 0,
                             "indirect_objects": 4, "streams": 1, "xref_verified": True}
    source_pdfs = [original]

    def seed():
        counted, measured, angled = [run.directory / name for name in ("counted.pdf", "measured.pdf", "angled.pdf")]
        run.execute("seed-count", ["count", original, "--group", "Conversion fixtures", "--point", "30,30",
                                   "--author", "Fixture reviewer", "--out", counted])
        run.record(counted, "cli_seed_pdf")
        run.execute("seed-length", ["measure", counted, "--kind", "length", "--points", "20,100 120,100",
                                    "--calibration", "20,20 120,20", "--known-distance", "10", "--unit", "m", "--out", measured])
        run.record(measured, "cli_seed_pdf")
        run.execute("seed-angle", ["measure", measured, "--kind", "angle", "--points", "50,60 70,60 70,80", "--out", angled])
        run.record(angled, "cli_seed_pdf")
        before = run.execute("seed-table", ["quantity-table", angled])
        run.check("seed: count length angle exact quantities", len(before["rows"]) == 3
                  and [(Fraction(str(row["quantity"]["value"])), row["quantity"]["unit"]) for row in before["rows"]] == QUANTITIES)
        expected = run.write("expected-count.json", before["rows"][0], "expected_row")
        custom = {base + suffix: {"type": "number", "value": value, "unit": base + suffix}
                  for suffix in ("", "2", "3") for base, value in INPUTS.items()}
        custom["convert"] = {"type": "text", "value": "=SUM(A1)"}
        fields = run.write("numeric-fields.json", {"unit_price": None, "custom": custom}, "typed_fields")
        source = run.directory / "quantity-source.pdf"
        receipt = run.execute("seed-fields", ["set-quantity-fields", angled, "--page", "1", "--expected-record", expected,
                                               "--fields", fields, "--out", source])
        run.record(source, "cli_seed_pdf")
        baseline = run.execute("seed-reopen", ["quantity-table", source])
        run.check("seed: fields are preserved on reopen", baseline["rows"][0]["fields"] == {"unit_price": None, "custom": custom}
                  and baseline["version"] == 2 and receipt["original_unchanged"] is True and receipt["source_content_exported"] is False)
        run.check("seed: unrelated measurements remain current", baseline["rows"][1:] == before["rows"][1:]
                  and all(row["measurement_validity"] == "current" for row in baseline["rows"][1:]))
        source_pdfs.extend([counted, measured, angled, source])
        run.preserved("seed: all original and intermediate files unchanged")
        return source, baseline, expected, fields

    source, baseline, stale_expected, fields = run.case("seed-real-count-and-measurements", "seed", seed)
    run.report["source"] = {"name": source.name, "saved_file_sha256": sha(source), "snapshot_sha256": baseline["source_sha256"],
                            "annotation_count": 3, "numeric_columns": 15, "text_columns": 1}
    batches = vectors()
    run.report["formula_vectors"] = [{"batch": name, "explicit_conversion": explicit, **item}
                                     for name, batch, explicit in batches for item in batch]
    retained = {}
    for name, batch, explicit in batches:
        def exercise(name=name, batch=batch, explicit=explicit):
            configuration = run.write(name + ".formulas.json", [item["column"] for item in batch], "formula_config")
            output, csv_path = run.directory / (name + ".json"), run.directory / (name + ".csv")
            receipt = run.execute(name + "-json", ["quantity-table", source, "--formulas", configuration, "--out", output])
            run.record(output, "quantity_json")
            value = json.loads(output.read_bytes())
            run.check(name + ": JSON output receipt", receipt["original_unchanged"] is True and receipt["format"] == "json"
                      and receipt["source_sha256"] == value["source_sha256"] and valid_snapshot(receipt["source_sha256"])
                      and Path(receipt["out"]).resolve(strict=True) == output.resolve(strict=True))
            validate_table(run, name, value, batch, explicit, baseline)
            receipt = run.execute(name + "-csv", ["quantity-table", source, "--formulas", configuration, "--format", "csv", "--out", csv_path])
            run.record(csv_path, "quantity_csv")
            run.check(name + ": CSV output receipt", receipt["original_unchanged"] is True and receipt["format"] == "csv"
                      and valid_snapshot(receipt["source_sha256"])
                      and Path(receipt["out"]).resolve(strict=True) == csv_path.resolve(strict=True))
            validate_csv(run, name, csv_path, batch, explicit, receipt["source_sha256"])
            run.preserved(name + ": sources and preceding evidence unchanged")
            retained[name] = (configuration, output, csv_path, value)
        run.case(name, "formula_json_csv", exercise, formulas=len(batch), explicit_conversion=explicit)

    selected = next(batch for name, batch, _ in batches if name == "boundaries-1")
    configuration, _, _, selected_value = retained["boundaries-1"]
    report_pdf, xlsx, workbook_manifest = [run.directory / name for name in ("issue-report.pdf", "quantity-workbook.xlsx", "workbook-manifest.json")]

    def reports():
        options = ["--formulas", configuration, "--expected-source-sha256", sha(source), "--no-legend"]
        issue = run.execute("issue-inspect", ["issue-report", "inspect", source, *options], code=2)
        run.check("issue report: current saved source and exact shared quantities", issue["provenance"]["saved_file_sha256"] == sha(source)
                  and issue["provenance"]["source_unchanged"] is True and same_quantities(issue["quantities"], selected_value)
                  and issue["provenance"]["snapshot_sha256"] == issue["quantities"]["source_sha256"])
        issue_json = run.directory / "issue-report.json"
        json_receipt = run.execute("issue-json", ["issue-report", "export", source, *options, "--format", "json", "--out", issue_json], code=2)
        run.record(issue_json, "issue_json")
        saved_issue = json.loads(issue_json.read_bytes())
        run.check("issue report: JSON model and byte receipt", same_issue(saved_issue, issue)
                  and json_receipt["provenance"] == saved_issue["provenance"]
                  and Path(json_receipt["out"]).resolve(strict=True) == issue_json.resolve(strict=True)
                  and json_receipt["output_sha256"] == sha(issue_json) and json_receipt["bytes"] == issue_json.stat().st_size)
        pdf_receipt = run.execute("issue-pdf", ["issue-report", "export", source, *options, "--out", report_pdf], code=2)
        run.record(report_pdf, "issue_pdf")
        run.check("issue report: actual PDF and source-bound receipt", report_pdf.read_bytes().startswith(b"%PDF-")
                  and pdf_receipt["output_sha256"] == sha(report_pdf) and pdf_receipt["bytes"] == report_pdf.stat().st_size
                  and pdf_receipt["matched_annotations"] == 3 and pdf_receipt["page_count"] == issue["page_count"]
                  and same_provenance(pdf_receipt["provenance"], issue["provenance"])
                  and Path(pdf_receipt["out"]).resolve(strict=True) == report_pdf.resolve(strict=True))
        text = run.execute("issue-text", ["text", report_pdf], parse=False).decode("utf-8")
        normalized = re.sub(r"\s+", " ", text)
        for item in selected:
            name, expected = item["column"]["name"], item["expected_rows"][0]
            run.check("issue PDF text: " + name, name + ": " + expected["value"] in normalized)
        defaults = run.execute("workbook-defaults", ["quantity-workbook", "defaults"])
        defaults["quantities"]["formulas"] = [item["column"] for item in selected]
        settings = run.write("workbook-settings.json", defaults, "workbook_settings")
        arguments = [source, "--settings", settings, "--expected-source-sha256", sha(source)]
        model = run.execute("workbook-inspect", ["quantity-workbook", "inspect", *arguments], code=2)
        run.check("workbook: current saved source and exact shared quantities", model["provenance"]["saved_file_sha256"] == sha(source)
                  and same_quantities(model["quantities"], selected_value)
                  and model["provenance"]["snapshot_sha256"] == model["quantities"]["source_sha256"])
        for format, output, role in (("xlsx", xlsx, "quantity_xlsx"), ("manifest", workbook_manifest, "workbook_manifest")):
            receipt = run.execute("workbook-" + format, ["quantity-workbook", "export", *arguments, "--format", format, "--out", output], code=2)
            run.record(output, role)
            run.check("workbook: " + format + " output receipt", receipt["source_unchanged"] is True and receipt["output_encrypted"] is False
                      and receipt["output_sha256"] == sha(output) and receipt["bytes"] == output.stat().st_size and receipt["matched_rows"] == 3
                      and same_provenance(receipt["provenance"], model["provenance"])
                      and Path(receipt["out"]).resolve(strict=True) == output.resolve(strict=True))
        value = json.loads(workbook_manifest.read_bytes())
        run.check("workbook manifest: exact retained settings and calculated values", value["options"] == defaults
                  and value["provenance"] == receipt["provenance"]
                  and [row["row"] for row in value["rows"]] == baseline["rows"]
                  and [row["formulas"] for row in value["rows"]] == [row["values"] for row in selected_value["formula_rows"]])
        validate_workbook(run, xlsx, selected)
        run.preserved("reports: all sources and preceding evidence unchanged")
    run.case("issue-and-workbook-exports", "report_exports", reports)

    def refusal(name, arguments, output, code=1):
        existed = output.exists()
        previous = sha(output) if existed else None
        run.execute(name, ["--overwrite", *arguments], code=code)
        run.check(name + ": no output or replacement", sha(output) == previous if existed else not output.exists() and not output.is_symlink())
        run.preserved(name + ": all source and evidence bytes unchanged")

    for suffix, value in invalid_configs():
        name = "refuse-" + suffix
        invalid = run.write(name + ".formulas.json", value, "invalid_formula_config")
        output = run.directory / (name + ".csv")
        arguments = ["quantity-table", source, "--formulas", invalid, "--format", "csv", "--out", output]
        run.case(name, "invalid_config_refusal", lambda name=name, arguments=arguments, output=output: refusal(name, arguments, output))
    for suffix, output in (("json", retained["boundaries-1"][1]), ("csv", retained["boundaries-1"][2]), ("source", source)):
        name = "refuse-existing-" + suffix
        arguments = ["quantity-table", source, "--formulas", configuration, "--out", output]
        run.case(name, "existing_output_refusal", lambda name=name, arguments=arguments, output=output: refusal(name, arguments, output, 6))
    for suffix, command, output in (("issue", ["issue-report", "export", source, "--formulas", configuration], report_pdf),
                                    ("xlsx", ["quantity-workbook", "export", source], xlsx),
                                    ("manifest", ["quantity-workbook", "export", source, "--format", "manifest"], workbook_manifest)):
        name = "refuse-existing-" + suffix
        arguments = command + ["--out", output]
        run.case(name, "existing_output_refusal", lambda name=name, arguments=arguments, output=output: refusal(name, arguments, output, 6))
    for suffix, command in (("issue-source-hash", ["issue-report", "export", source, "--expected-source-sha256", "0" * 64]),
                           ("workbook-source-hash", ["quantity-workbook", "export", source, "--expected-source-sha256", "0" * 64]),
                           ("stale-expected-row", ["set-quantity-fields", source, "--page", "1", "--expected-record", stale_expected, "--fields", fields])):
        name = "refuse-" + suffix
        output = run.directory / (name + ".output")
        arguments = command + ["--out", output]
        run.case(name, "stale_source_refusal", lambda name=name, arguments=arguments, output=output: refusal(name, arguments, output))
    run.preserved("all originals and outputs remain byte-for-byte unchanged")
    run.report["source_integrity"] = [{"name": path.name, "expected_sha256": run.protected[path], "observed_sha256": sha(path)} for path in source_pdfs]
    run.check("all 75 same-dimension pairs exercised", sum(len(batch) for name, batch, _ in batches if name.startswith("pairs-")) == 75)
    run.check("every rounding precision exercised", {item["column"]["rounding_decimals"] for name, batch, _ in batches
              if name.startswith("rounding-") for item in batch} == set(range(7)))
    run.check("all planned commands and cases observed", len(run.report["commands"]) == run.report["planned_coverage"]["expected_commands"]
              and len(run.report["cases"]) == run.report["planned_coverage"]["expected_cases"])
    run.check("exact packaged CLI bytes unchanged", sha(run.cli) == run.report["cli_sha256"])
    run.check("every retained file has an explicit role", all(path.relative_to(run.directory).as_posix() in run.roles
              for path in run.directory.rglob("*") if path.is_file()))
    run.remaining()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--directory", type=Path)
    parser.add_argument("--expected-version")
    parser.add_argument("--source-sha", default=os.environ.get("GITHUB_SHA"))
    parser.add_argument("--deadline-seconds", type=int, default=240)
    parser.add_argument("--self-test", action="store_true", help="Only test the independent oracle, inventory and fixture; never launch the CLI")
    args = parser.parse_args()
    if args.self_test:
        if args.cli or args.directory: parser.error("self-test does not accept CLI or output directories")
        self_test()
        return
    if not args.cli or not args.directory or not args.expected_version or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.expected_version):
        parser.error("supply --cli, a new --directory and the explicit --expected-version")
    if not args.source_sha or not re.fullmatch(r"[0-9a-fA-F]{40}", args.source_sha):
        parser.error("supply the full source commit with --source-sha or GITHUB_SHA")
    if not 60 <= args.deadline_seconds <= 600: parser.error("deadline must be between 60 and 600 seconds")
    cli = args.cli.resolve(strict=True)
    if not cli.is_file(): parser.error("CLI must be a regular file")
    directory = args.directory.absolute()
    if directory.exists() or directory.is_symlink(): raise FileExistsError(directory)
    directory.mkdir(parents=True, exist_ok=False)
    run = Corpus(cli, directory.resolve(), args.expected_version, args.source_sha, args.deadline_seconds)
    try:
        run_corpus(run)
        run.report["status"] = "passed"
    except Exception as error:
        run.report["status"], run.report["error"] = "failed", str(error)
        raise
    finally:
        run.finish()
    if run.report["status"] != "passed": raise AssertionError("Formula conversion corpus did not pass")
    print(json.dumps({"status": run.report["status"], "counts": run.report["counts"], "report": str(run.directory / "report.json")}))


if __name__ == "__main__":
    main()
