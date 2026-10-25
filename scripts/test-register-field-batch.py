"""Qualify explicit register-field batches with original PDFs and independent QPDF objects."""
import argparse
import base64
import copy
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import time


PROCESS_SECONDS = 30
MAX_TRANSCRIPT_BYTES = 2_000_000
MAX_ARTIFACT_BYTES = 16 * 1024 * 1024
SCOPE = ("Original synthetic two-page PDF; explicit cross-page field selection, saved PDF "
         "content and appearance preservation, refusal guards and AES-256 permissions. "
         "No native desktop interaction, Undo/Redo, signature validity or external-reader rendering is claimed.")
SELECTED = ((1, 0), (2, 0), (2, 1))
CHANGED = ((1, 0), (2, 0))
PATCH = {"business_status": {"operation": "set", "value": "Reviewed"},
         "discipline": {"operation": "clear"}, "custom": {
             "Room": {"operation": "clear"}, "Note": {"operation": "set", "value": "Checked"},
             "Keep": {"operation": "keep"}}}
PAGE_CONTENTS = [b"0 0 0 rg BT /F1 20 Tf 60 740 Td (Original register page one) Tj ET\n",
                 b"0 0 0 rg BT /F1 20 Tf 60 740 Td (Original register page two) Tj ET\n"]
AP_CONTENT = b"q /GS gs 0 0 1 RG 1 1 1 rg 1.5 w 2 2 20 20 re B 6 12 m 18 12 l 12 6 m 12 18 l S Q\n"


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def fields(status, room):
    return {"business_status": status, "discipline": "Electrical",
            "custom": {"Keep": room, "Room": room}}


def changed_fields(before):
    """Literal field oracle, independent of application patch evaluation."""
    return {"business_status": "Reviewed", "discipline": None,
            "custom": {"Keep": before["custom"]["Keep"], "Note": "Checked"}}


def fixture_rows():
    result = []
    for page, index, group, status, room in [(1, 0, "Lights", "Open", "A"),
            (1, 1, "Unselected", "Pending", "B"), (2, 0, "Sockets", "Pending", "C"),
            (2, 1, "Unchanged", "Reviewed", "D"), (2, 2, "Locked", "Reviewed", "E")]:
        before = fields(status, room)
        if page == 2 and index in (1, 2):
            before = changed_fields(before)
        result.append({"page": page, "index": index, "id": f"SuniPDF-original-{page}-{index}",
                       "group": group, "fields": before, "locked": page == 2 and index == 2})
    return result


def stream_bytes(content, extra=b""):
    return b"<< /Length " + str(len(content)).encode() + b" " + extra + b" >>\nstream\n" + content + b"endstream"


def pdf_bytes(objects):
    data = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for number, body in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f"{number} 0 obj\n".encode() + body + b"\nendobj\n")
    xref = len(data)
    data.extend(f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode())
    for offset in offsets:
        data.extend(f"{offset:010} 00000 n \n".encode())
    data.extend(f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    return bytes(data)


def fixture_pdf(malformed=False):
    def encoded(value):
        return b"<" + json.dumps(value, sort_keys=True, separators=(",", ":")).encode().hex().encode() + b">"
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>",
               b"<< /Type /Pages /Count 2 /Kids [4 0 R 6 0 R] >>",
               b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
               b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /CropBox [0 0 600 800] /Rotate 0 /Resources << /Font << /F1 3 0 R >> >> /Contents 5 0 R /Annots [9 0 R 10 0 R] >>",
               stream_bytes(PAGE_CONTENTS[0]),
               b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /CropBox [0 0 600 800] /Rotate 0 /Resources << /Font << /F1 3 0 R >> >> /Contents 7 0 R /Annots [11 0 R 12 0 R 13 0 R] >>",
               stream_bytes(PAGE_CONTENTS[1]),
               stream_bytes(AP_CONTENT, b"/Type /XObject /Subtype /Form /BBox [0 0 24 24] /Resources << /ExtGState << /GS << /Type /ExtGState /CA 1 /ca 1 >> >> >>")]
    for row in fixture_rows():
        x, parent = 70 + row["index"] * 55, 4 if row["page"] == 1 else 6
        register = {"version": 1, "fields": row["fields"]}
        if malformed and row["page"] == 2 and row["index"] == 0:
            register["version"] = 99
        quantity = {"version": 2, "fields": {"unit_price": None, "custom": {
            "Length": {"type": "number", "value": "2.5", "unit": "m"},
            "Label": {"type": "text", "value": row["group"]}}}}
        objects.append((f"<< /Type /Annot /Subtype /Stamp /Name /SuniPDFCount /Rect [{x} 650 {x + 24} 674] "
                        f"/P {parent} 0 R /F {132 if row['locked'] else 4} /NM ({row['id']}) "
                        f"/T (Original reviewer) /M (D:20200102030405Z) /Contents (Count: {row['group']}) "
                        "/Subj (Original synthetic count) /StateModel (Review) /State (Accepted) "
                        "/AP << /N 8 0 R >> /SuniPDFRegister ").encode() + encoded(register)
                       + b" /SuniPDFCount " + encoded({"version": 1, "group": row["group"]})
                       + b" /SuniPDFQuantity " + encoded(quantity) + b" >>")
    return pdf_bytes(objects)


def qpdf_pages(value):
    """Resolve QPDF object references, comparing decoded streams rather than object numbers."""
    objects = value["qpdf"][1]

    def resolve(item, active=()):
        if isinstance(item, str) and re.fullmatch(r"\d+ \d+ R", item):
            if item in active or len(active) >= 32:
                raise ValueError("unexpected recursive PDF object")
            entry = objects["obj:" + item]
            if "stream" in entry:
                stream = entry["stream"]
                attrs = {key: val for key, val in stream["dict"].items()
                         if key not in ("/Length", "/Filter", "/DecodeParms")}
                return {"stream_dictionary": resolve(attrs, active + (item,)),
                        "decoded_hex": base64.b64decode(stream["data"], validate=True).hex()}
            return resolve(entry["value"], active + (item,))
        if isinstance(item, dict):
            return {key: resolve(val, active) for key, val in item.items()}
        if isinstance(item, list):
            return [resolve(val, active) for val in item]
        if isinstance(item, str) and item.startswith("b:"):
            raw = bytes.fromhex(item[2:])
            try:
                return "u:" + raw.decode("utf-8")
            except UnicodeDecodeError:
                return item
        return item

    result = []
    for number, page in enumerate(value["pages"], 1):
        details = objects["obj:" + page["object"]]["value"]
        entries = details.get("/Annots", [])
        if isinstance(entries, str):
            entries = objects["obj:" + entries]["value"]
        annotations = []
        for entry in entries:
            raw = copy.deepcopy(objects["obj:" + entry]["value"] if isinstance(entry, str) else entry)
            if raw.pop("/P", None) != page["object"]:
                raise ValueError("annotation must reference its actual owning page")
            annotations.append(resolve(raw))
        result.append({"page": number, "attributes": resolve({key: val for key, val in details.items()
                       if key not in ("/Parent", "/Annots", "/Contents")}),
                       "contents": [resolve(ref) for ref in page["contents"]], "annotations": annotations})
    return result


def pdf_json(value):
    if not isinstance(value, str) or not value.startswith("u:"):
        raise ValueError("expected decoded UTF-8 PDF metadata")
    return json.loads(value[2:])


def make_plan(inventory, selected=SELECTED, patch=PATCH):
    rows = {(row["page"], row["index"]): row for row in inventory["rows"]}
    return {"version": 1, "expected_source_sha256": inventory["source_file_sha256"],
            "targets": [{"page": page, "index": index,
                         "expected_row_sha256": rows[(page, index)]["expected_row_sha256"]}
                        for page, index in selected], "patch": copy.deepcopy(patch)}


class Corpus:
    def __init__(self, cli, qpdf, directory, version, source, seconds):
        self.cli, self.qpdf, self.directory = cli, qpdf, directory
        self.started, self.seconds = time.monotonic(), seconds
        self.roles, self.protected = {}, {}
        self.credentials = {"SUNIPDF_BATCH_USER": secrets.token_hex(24), "SUNIPDF_BATCH_OWNER": secrets.token_hex(24)}
        self.report = {"schema": "sunipdf-register-field-batch-qualification", "version": 1,
            "status": "running", "source_revision": source.lower(), "expected_cli_version": version,
            "cli_sha256": sha(cli), "qpdf_sha256": sha(qpdf), "script_sha256": sha(Path(__file__)),
            "started_utc": datetime.now(timezone.utc).isoformat(), "scope": SCOPE,
            "fixture_license": "CC0-1.0", "native_desktop_tested": False,
            "planned_coverage": {"commands": 35, "refusals": 16, "cases": 18,
                                 "source_pages": 2, "source_annotations": 5, "selected_rows": 3, "changed_rows": 2},
            "limits": {"total_seconds": seconds, "process_seconds": PROCESS_SECONDS,
                       "transcript_bytes_per_stream": MAX_TRANSCRIPT_BYTES, "artifact_bytes": MAX_ARTIFACT_BYTES},
            "checks": [], "commands": [], "cases": [], "sources": [], "inventory_excludes": ["report.json"]}

    def relative(self, path):
        return Path(path).relative_to(self.directory).as_posix()

    def remaining(self):
        seconds = self.started + self.seconds - time.monotonic()
        if seconds <= 0:
            raise TimeoutError("Register-field corpus exceeded its total deadline")
        return seconds

    def check(self, name, passed):
        self.report["checks"].append({"name": name, "passed": bool(passed)})
        if not passed:
            raise AssertionError(name)

    def record(self, path, role):
        if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_ARTIFACT_BYTES:
            raise ValueError("artifact is missing, linked or excessive: " + str(path))
        self.roles[self.relative(path)] = role
        self.protected[path] = sha(path)
        return path

    def write(self, name, value, role="plan"):
        data = value if isinstance(value, bytes) else (json.dumps(value, indent=2) + "\n").encode()
        path = self.directory / name
        with path.open("xb") as stream:
            stream.write(data)
        return self.record(path, role)

    def preserved(self, name):
        self.check(name, all(path.is_file() and sha(path) == digest for path, digest in self.protected.items()))

    def execute(self, name, arguments, tool=None, reject=False, parse=True):
        executable = tool or self.cli
        command = [str(executable), *map(str, arguments)]
        safe_command = [next(("--password=<ephemeral>" for secret in self.credentials.values()
                              if value == "--password=" + secret), value) for value in command]
        entry = {"name": name, "argv": safe_command, "expected_refusal": reject,
                 "started_utc": datetime.now(timezone.utc).isoformat(), "exit_code": None}
        self.report["commands"].append(entry)
        out, err = (self.directory / f"{name}.{suffix}" for suffix in ("stdout", "stderr"))
        begin, budget, process = time.monotonic(), min(PROCESS_SECONDS, self.remaining()), None
        try:
            with out.open("xb") as stdout, err.open("xb") as stderr:
                process = subprocess.Popen(command, cwd=self.directory, stdin=subprocess.DEVNULL,
                    stdout=stdout, stderr=stderr, env={**os.environ, **self.credentials})
                try:
                    while process.poll() is None:
                        if time.monotonic() - begin > budget:
                            raise TimeoutError(name + ": process deadline exceeded")
                        if max(out.stat().st_size, err.stat().st_size) > MAX_TRANSCRIPT_BYTES:
                            raise ValueError(name + ": transcript bound exceeded")
                        time.sleep(min(0.05, max(0.001, self.remaining())))
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait(timeout=5)
                    entry["exit_code"] = process.returncode
        finally:
            entry["elapsed_seconds"] = round(time.monotonic() - begin, 3)
            entry["transcripts"] = []
            for path in (out, err):
                if path.exists():
                    raw = path.read_bytes()
                    for secret in self.credentials.values():
                        raw = raw.replace(secret.encode(), b"<ephemeral>")
                    if raw != path.read_bytes():
                        path.write_bytes(raw)
                    self.record(path, "command_transcript")
                    entry["transcripts"].append({"name": path.name, "bytes": path.stat().st_size, "sha256": sha(path)})
        self.check(name + ": bounded expected process outcome",
                   max(out.stat().st_size, err.stat().st_size) <= MAX_TRANSCRIPT_BYTES
                   and (process.returncode != 0 if reject else process.returncode == 0))
        if reject:
            self.check(name + ": structured refusal without success output", not out.read_bytes()
                       and isinstance(json.loads(err.read_bytes()), dict))
            return None
        return json.loads(out.read_bytes()) if parse else out.read_bytes()

    def case(self, name, kind, action):
        start = len(self.report["commands"])
        entry = {"name": name, "kind": kind, "passed": False}
        self.report["cases"].append(entry)
        try:
            result = action()
            entry["passed"] = True
            return result
        finally:
            entry["commands"] = [item["name"] for item in self.report["commands"][start:]]

    def inspect(self, name, path, encrypted=False):
        flags = ["--password-env", "SUNIPDF_BATCH_USER"] if encrypted else []
        return self.execute(name, ["annotation-fields-batch", "inspect", path, *flags])

    def qpdf_inspect(self, name, path, encrypted=False):
        flags = ["--password=" + self.credentials["SUNIPDF_BATCH_USER"]] if encrypted else []
        self.execute(name + "-check", [*flags, "--check", path], tool=self.qpdf, parse=False)
        value = self.execute(name + "-json", [*flags, "--json", "--json-stream-data=inline", path], tool=self.qpdf)
        if encrypted:
            self.check(name + ": actual encrypted PDF", value["encrypt"]["encrypted"] is True)
        return qpdf_pages(value)

    def finish(self):
        self.report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        self.report["elapsed_seconds"] = round(time.monotonic() - self.started, 3)
        self.report["protected_integrity"] = [{"name": self.relative(path), "expected_sha256": digest,
            "observed_sha256": sha(path) if path.is_file() else None} for path, digest in self.protected.items()]
        self.report["source_integrity"] = [{"name": item["name"], "expected_sha256": item["sha256"],
            "observed_sha256": sha(self.directory / item["name"]) if (self.directory / item["name"]).is_file() else None}
            for item in self.report["sources"]]
        self.report["originals_unchanged"] = bool(self.report["source_integrity"]) and all(
            item["expected_sha256"] == item["observed_sha256"] for item in self.report["source_integrity"])
        if any(item["expected_sha256"] != item["observed_sha256"] for item in self.report["protected_integrity"]):
            self.report["status"], self.report["error"] = "failed", "Protected input or output changed"
        self.report["inventory"] = [{"name": self.relative(path), "role": self.roles.get(self.relative(path), "unclassified"),
            "bytes": path.stat().st_size, "sha256": sha(path)} for path in sorted(self.directory.rglob("*")) if path.is_file()]
        self.report["counts"] = {"checks_observed": len(self.report["checks"]),
            "checks_passed": sum(item["passed"] for item in self.report["checks"]),
            "commands_started": len(self.report["commands"]),
            "commands_completed": sum(item["exit_code"] is not None for item in self.report["commands"]),
            "refusal_commands_completed": sum(item["expected_refusal"] and item["exit_code"] is not None for item in self.report["commands"]),
            "cases_observed": len(self.report["cases"]), "cases_passed": sum(item["passed"] for item in self.report["cases"]),
            "artifact_files": len(self.report["inventory"]), "original_source_pdfs": len(self.report["sources"]),
            "source_pages": 2, "source_annotations": 5,
            "selected_rows": 3, "changed_rows": 2}
        with (self.directory / "report.json").open("x", encoding="utf-8") as stream:
            json.dump(self.report, stream, indent=2)
            stream.write("\n")


def assert_inventory(run, inventory, path, after=False):
    actual = {(row["page"], row["index"]): row for row in inventory["rows"]}
    expected = fixture_rows()
    run.check(path.name + ": exact inventory and source binding", inventory["source_file_sha256"] == sha(path)
              and inventory["selected_rows"] == 0 and inventory["original_unchanged"] is True
              and set(actual) == {(row["page"], row["index"]) for row in expected})
    run.check(path.name + ": known fields, identities and bounded fingerprints", all(
        actual[(row["page"], row["index"])]["fields"] ==
            (changed_fields(row["fields"]) if after and (row["page"], row["index"]) in CHANGED else row["fields"])
        and actual[(row["page"], row["index"])]["annotation_id"] == row["id"]
        and actual[(row["page"], row["index"])]["kind"] == "Stamp"
        and actual[(row["page"], row["index"])]["locked"] is row["locked"]
        and re.fullmatch(r"[a-f0-9]{64}", actual[(row["page"], row["index"])]["expected_row_sha256"])
        for row in expected))


def verify_pdf(run, label, original, saved):
    run.check(label + ": exact page content, geometry and resources", len(saved) == len(original) == 2 and all(
        old["attributes"] == new["attributes"] and old["contents"] == new["contents"]
        for old, new in zip(original, saved)))
    run.check(label + ": exact annotation counts", [len(page["annotations"]) for page in saved] == [2, 3])
    for row in fixture_rows():
        pair = row["page"], row["index"]
        before = original[pair[0] - 1]["annotations"][pair[1]]
        after = saved[pair[0] - 1]["annotations"][pair[1]]
        excluded = ("/M", "/SuniPDFRegister") if pair in CHANGED else ()
        run.check(f"{label}: page {pair[0]} annotation {pair[1]} preserves all unrelated objects",
                  {key: val for key, val in before.items() if key not in excluded}
                  == {key: val for key, val in after.items() if key not in excluded})
        run.check(f"{label}: page {pair[0]} annotation {pair[1]} exact persisted fields",
                  pdf_json(after["/SuniPDFRegister"]) == {"version": 1, "fields":
                  changed_fields(row["fields"]) if pair in CHANGED else row["fields"]})


def run_corpus(run):
    version = run.execute("version", ["--version"], parse=False).decode().strip()
    run.report["cli_version"] = version
    run.check("exact requested CLI version", version == "sunipdf-cli " + run.report["expected_cli_version"])
    run.report["qpdf_version"] = run.execute("qpdf-version", ["--version"], tool=run.qpdf, parse=False).decode().strip()
    source = run.write("original.pdf", fixture_pdf(), "original_source")
    run.report["sources"].append({"name": source.name, "sha256": sha(source), "pages": 2, "annotations": 5})
    original = run.qpdf_inspect("original", source)
    run.check("original independently authored page and appearance streams", all(
        bytes.fromhex(page["contents"][0]["decoded_hex"]) == PAGE_CONTENTS[index]
        and all(bytes.fromhex(annot["/AP"]["/N"]["decoded_hex"]) == AP_CONTENT for annot in page["annotations"])
        for index, page in enumerate(original)))
    run.check("original count, review and typed quantity metadata", all(
        pdf_json(original[row["page"] - 1]["annotations"][row["index"]]["/SuniPDFCount"])
            == {"version": 1, "group": row["group"]}
        and original[row["page"] - 1]["annotations"][row["index"]]["/State"] == "u:Accepted"
        and pdf_json(original[row["page"] - 1]["annotations"][row["index"]]["/SuniPDFQuantity"])["fields"]["custom"]["Length"]
            == {"type": "number", "value": "2.5", "unit": "m"} for row in fixture_rows()))
    inventory = run.inspect("inspect-original", source)
    assert_inventory(run, inventory, source)
    one_page = run.execute("inspect-page-two", ["annotation-fields-batch", "inspect", source, "--page", 2])
    run.check("page filter remains read-only and explicit", one_page["rows"] == [row for row in inventory["rows"] if row["page"] == 2]
              and one_page["selected_rows"] == 0 and one_page["source_file_sha256"] == sha(source))
    plan = make_plan(inventory)
    plan_path = run.write("reviewed-plan.json", plan)
    output = run.directory / "reviewed.pdf"

    def preview_apply():
        preview = run.execute("preview", ["annotation-fields-batch", "preview", source, "--plan", plan_path])
        rows = {(row["page"], row["index"]): row for row in fixture_rows()}
        expected = [{"page": page, "index": index, "annotation_id": rows[(page, index)]["id"],
            "expected_row_sha256": target["expected_row_sha256"], "before": rows[(page, index)]["fields"],
            "after": changed_fields(rows[(page, index)]["fields"])}
            for (page, index), target in zip(SELECTED, plan["targets"])]
        run.check("reviewed before/after includes selected unchanged row", preview["committed"] is False
                  and preview["source_file_sha256"] == sha(source) and preview["changes"]["selected_count"] == 3
                  and preview["changes"]["changed_count"] == 2 and preview["changes"]["changes"] == expected
                  and not output.exists())
        result = run.execute("apply", ["annotation-fields-batch", "apply", source, "--plan", plan_path, "--out", output])
        run.record(output, "saved_copy")
        run.check("saved receipt matches complete reviewed changes", result["committed"] is True
                  and result["original_unchanged"] is True and result["source_file_sha256"] == sha(source)
                  and result["changes"]["changes"] == expected and result["encrypted"] is False)
        reopened = run.inspect("inspect-reopened", output)
        assert_inventory(run, reopened, output, after=True)
        verify_pdf(run, "saved", original, run.qpdf_inspect("saved", output))
        run.preserved("preview, save and reopen preserve original sources")
    run.case("explicit-cross-page-preview-save-reopen", "success", preview_apply)

    def refusal(name, value=plan, file=source, destination=None, encrypted=False, raw=None):
        manifest = run.write(name + ".json", raw if raw is not None else value, "refusal_plan")
        destination = destination or run.directory / (name + ".pdf")
        existed = destination.exists()
        arguments = ["annotation-fields-batch", "apply", file, "--plan", manifest, "--out", destination, "--overwrite"]
        if encrypted:
            arguments += ["--password-env", "SUNIPDF_BATCH_USER"]
        run.execute(name, arguments, reject=True)
        run.check(name + ": no new output or changed protected artifact", (existed or not destination.exists())
                  and all(path.is_file() and sha(path) == digest for path, digest in run.protected.items()))

    for name, destination in [("existing-copy", output), ("source-output", source)]:
        run.case(name, "refusal", lambda name=name, destination=destination: refusal(name, destination=destination))
    mutations = [
        ("late-stale-row", lambda v: v["targets"][-1].update(expected_row_sha256="0" * 64)),
        ("late-missing-row", lambda v: v["targets"][-1].update(index=99)),
        ("duplicate-target", lambda v: v["targets"].append(copy.deepcopy(v["targets"][0]))),
        ("wrong-source-hash", lambda v: v.update(expected_source_sha256="0" * 64)),
        ("unknown-plan-field", lambda v: v.update(unknown=True)),
        ("unknown-patch-field", lambda v: v["patch"].update(review_status={"operation": "clear"})),
        ("invalid-field-value", lambda v: v["patch"].update(business_status={"operation": "set", "value": " bad "})),
        ("too-many-targets", lambda v: v.update(targets=[dict(v["targets"][0], index=i) for i in range(257)])),
    ]
    for name, mutation in mutations:
        value = copy.deepcopy(plan)
        mutation(value)
        run.case(name, "refusal", lambda name=name, value=value: refusal(name, value))
    raw = json.dumps(plan).replace('"custom": {', '"custom": {"Room":{"operation":"keep"},', 1).encode()
    run.case("duplicate-custom-operation", "refusal", lambda: refusal("duplicate-custom-operation", raw=raw))
    late_locked = make_plan(inventory, SELECTED + ((2, 2),))
    run.case("late-locked-unchanged-row", "refusal", lambda: refusal("late-locked-unchanged-row", late_locked))
    modified = run.write("externally-modified.pdf", source.read_bytes() + b"\n% changed saved bytes\n", "stale_source")
    run.case("stale-saved-source", "refusal", lambda: refusal("stale-saved-source", file=modified))
    no_op = make_plan(inventory, patch={})
    no_op_path = run.write("no-op-preview.json", no_op)
    preview = run.execute("no-op-preview", ["annotation-fields-batch", "preview", source, "--plan", no_op_path])
    run.check("no-op preview exposes no changes without committing", preview["committed"] is False
              and preview["changes"]["selected_count"] == 3 and preview["changes"]["changed_count"] == 0
              and all(item["before"] == item["after"] for item in preview["changes"]["changes"]))
    run.case("no-op-apply", "refusal", lambda: refusal("no-op-apply", no_op))
    malformed = run.write("malformed-metadata.pdf", fixture_pdf(malformed=True), "malformed_source")
    invalid_plan = copy.deepcopy(plan)
    invalid_plan["expected_source_sha256"] = sha(malformed)
    run.case("malformed-stored-metadata", "refusal", lambda: refusal("malformed-stored-metadata", invalid_plan, file=malformed))

    for restricted in (True, False):
        name = "restricted" if restricted else "permitted"
        encrypted = run.directory / (name + ".pdf")
        arguments = ["protect", source, "--user-password-env", "SUNIPDF_BATCH_USER",
                     "--owner-password-env", "SUNIPDF_BATCH_OWNER", "--out", encrypted]
        if restricted:
            arguments += ["--no-annotate"]
        run.execute(name + "-protect", arguments)
        run.record(encrypted, "encrypted_source")
        encrypted_inventory = run.inspect(name + "-inspect", encrypted, encrypted=True)
        assert_inventory(run, encrypted_inventory, encrypted)
        encrypted_plan = make_plan(encrypted_inventory)
        if restricted:
            run.case("encrypted-permissions", "refusal", lambda: refusal("encrypted-permissions", encrypted_plan,
                                                                         file=encrypted, encrypted=True))
        else:
            def encrypted_save():
                manifest = run.write("encrypted-plan.json", encrypted_plan)
                saved = run.directory / "encrypted-reviewed.pdf"
                result = run.execute("encrypted-apply", ["annotation-fields-batch", "apply", encrypted,
                    "--plan", manifest, "--out", saved, "--password-env", "SUNIPDF_BATCH_USER"])
                run.record(saved, "encrypted_saved_copy")
                run.check("encrypted receipt retains source and encryption", result["encrypted"] is True
                          and result["committed"] is True and result["source_file_sha256"] == sha(encrypted)
                          and result["changes"]["selected_count"] == 3 and result["changes"]["changed_count"] == 2)
                verify_pdf(run, "encrypted saved", original, run.qpdf_inspect("encrypted-saved", saved, encrypted=True))
            run.case("permitted-encrypted-save", "success", encrypted_save)
    run.preserved("every retained source, plan and successful output unchanged")
    run.check("executables unchanged throughout corpus", sha(run.cli) == run.report["cli_sha256"]
              and sha(run.qpdf) == run.report["qpdf_sha256"])
    run.check("every retained file has a role", all(run.relative(path) in run.roles
              for path in run.directory.rglob("*") if path.is_file()))
    run.check("all planned commands, refusals and cases observed",
              len(run.report["commands"]) == run.report["planned_coverage"]["commands"]
              and sum(command["expected_refusal"] for command in run.report["commands"]) == run.report["planned_coverage"]["refusals"]
              and len(run.report["cases"]) == run.report["planned_coverage"]["cases"])
    run.remaining()


def self_test():
    """Pure fixture/oracle checks only; this never launches a product or QPDF process."""
    data = fixture_pdf()
    xref = int(data.rsplit(b"startxref\n", 1)[1].splitlines()[0])
    assert data[xref:].startswith(b"xref\n0 14\n")
    for number, line in enumerate(data[xref:].splitlines()[3:16], 1):
        offset = int(line[:10])
        assert data[offset:].startswith(f"{number} 0 obj\n".encode())
    assert fixture_pdf(malformed=True) != data
    assert len(fixture_rows()) == 5 and len(SELECTED) == 3 and len(CHANGED) == 2
    assert changed_fields(fixture_rows()[3]["fields"]) == fixture_rows()[3]["fields"]
    inventory = {"source_file_sha256": "a" * 64, "rows": [{"page": row["page"], "index": row["index"],
        "expected_row_sha256": "b" * 64} for row in fixture_rows()]}
    plan = make_plan(inventory)
    assert len(plan["targets"]) == 3 and plan["patch"] == PATCH
    raw = json.dumps(plan).replace('"custom": {', '"custom": {"Room":{"operation":"keep"},', 1)
    duplicate_pairs = []
    json.loads(raw, object_pairs_hook=lambda pairs: duplicate_pairs.append(pairs) or dict(pairs))
    assert any([key for key, _ in pairs].count("Room") == 2 for pairs in duplicate_pairs)
    doc = {"pages": [{"object": "1 0 R", "contents": ["2 0 R"]}], "qpdf": [{}, {
        "obj:1 0 R": {"value": {"/Type": "/Page", "/Annots": ["3 0 R"]}},
        "obj:2 0 R": {"stream": {"dict": {"/Length": 2}, "data": base64.b64encode(b"qQ").decode()}},
        "obj:3 0 R": {"value": {"/P": "1 0 R", "/AP": {"/N": "2 0 R"}, "/Text": "b:4142"}}}]}
    before = qpdf_pages(doc)
    assert before[0]["contents"][0]["decoded_hex"] == b"qQ".hex()
    assert before[0]["annotations"][0]["/Text"] == "u:AB"
    doc["qpdf"][1]["obj:2 0 R"]["stream"]["data"] = base64.b64encode(b"QQ").decode()
    assert qpdf_pages(doc) != before
    doc["qpdf"][1]["obj:3 0 R"]["value"]["/P"] = "9 0 R"
    try:
        qpdf_pages(doc)
    except ValueError:
        pass
    else:
        raise AssertionError("wrong annotation ownership was accepted")
    assert pdf_json("u:" + json.dumps({"version": 1})) == {"version": 1}

    class OracleCheck:
        @staticmethod
        def check(name, passed):
            if not passed:
                raise AssertionError(name)

    original = [{"attributes": {"/Resources": {"/Font": "original-font"}},
                 "contents": [{"decoded_hex": contents.hex()}], "annotations": []} for contents in PAGE_CONTENTS]
    for row in fixture_rows():
        original[row["page"] - 1]["annotations"].append({
            "/SuniPDFRegister": "u:" + json.dumps({"version": 1, "fields": row["fields"]}),
            "/SuniPDFQuantity": "u:original-typed-quantity", "/SuniPDFCount": "u:" + row["group"],
            "/State": "u:Accepted", "/AP": {"/N": {"decoded_hex": AP_CONTENT.hex()}}, "/M": "u:original-date"})
    saved = copy.deepcopy(original)
    for page, index in CHANGED:
        annotation = saved[page - 1]["annotations"][index]
        initial = pdf_json(annotation["/SuniPDFRegister"])["fields"]
        annotation["/SuniPDFRegister"] = "u:" + json.dumps({"version": 1, "fields": changed_fields(initial)})
        annotation["/M"] = "u:new-date"
    verify_pdf(OracleCheck(), "offline synthetic positive", original, saved)
    for mutation in [
        lambda value: value[0]["contents"][0].update(decoded_hex="00"),
        lambda value: value[1]["attributes"].update(unknown="changed"),
        lambda value: value[0]["annotations"][0]["/AP"]["/N"].update(decoded_hex="00"),
        lambda value: value[0]["annotations"][0].update({"/SuniPDFQuantity": "u:changed"}),
        lambda value: value[1]["annotations"][0].update({"/SuniPDFCount": "u:changed"}),
        lambda value: value[1]["annotations"][0].update({"/State": "u:Rejected"}),
        lambda value: value[0]["annotations"][1].update({"/M": "u:changed-unselected"}),
        lambda value: value[1]["annotations"][1].update({"/M": "u:changed-selected-no-op"}),
        lambda value: value[1]["annotations"][0].update({"/SuniPDFRegister": original[1]["annotations"][0]["/SuniPDFRegister"]}),
    ]:
        tampered = copy.deepcopy(saved)
        mutation(tampered)
        try:
            verify_pdf(OracleCheck(), "offline synthetic tamper", original, tampered)
        except AssertionError:
            pass
        else:
            raise AssertionError("independent oracle accepted altered saved PDF objects")
    print("Register batch fixture/oracle self-test passed; no product process executed.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--qpdf", type=Path)
    parser.add_argument("--directory", type=Path)
    parser.add_argument("--expected-version")
    parser.add_argument("--source-sha", default=os.environ.get("GITHUB_SHA"))
    parser.add_argument("--deadline-seconds", type=int, default=240)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        if args.cli or args.qpdf or args.directory:
            parser.error("self-test does not accept executables or an output directory")
        self_test()
        return
    if not args.cli or not args.qpdf or not args.directory or not args.expected_version or not re.fullmatch(r"\d+\.\d+\.\d+", args.expected_version):
        parser.error("supply --cli, --qpdf, a new --directory and --expected-version")
    if not args.source_sha or not re.fullmatch(r"[a-fA-F0-9]{40}", args.source_sha):
        parser.error("supply the full --source-sha or GITHUB_SHA")
    if not 60 <= args.deadline_seconds <= 600:
        parser.error("deadline-seconds must be between 60 and 600")
    cli, qpdf = args.cli.resolve(strict=True), args.qpdf.resolve(strict=True)
    if not cli.is_file() or not qpdf.is_file():
        parser.error("CLI and QPDF must be regular executable files")
    directory = args.directory.absolute()
    if directory.exists() or directory.is_symlink():
        raise FileExistsError(directory)
    directory.mkdir(parents=True, exist_ok=False)
    run = Corpus(cli, qpdf, directory.resolve(), args.expected_version, args.source_sha, args.deadline_seconds)
    try:
        run_corpus(run)
        run.report["status"] = "passed"
    except Exception as error:
        run.report["status"], run.report["error"] = "failed", str(error)
        raise
    finally:
        run.finish()
    if run.report["status"] != "passed":
        raise AssertionError("Register field batch corpus did not pass")
    print(json.dumps({"status": run.report["status"], "counts": run.report["counts"], "report": str(directory / "report.json")}))


if __name__ == "__main__":
    main()
