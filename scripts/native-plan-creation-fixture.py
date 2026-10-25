#!/usr/bin/env python3
"""Synthetic unlinked plan inputs and independent checks for native creation QA.

The CLI only indexes the original PDFs and edits sheet metadata. It never creates
the linked copies qualified by this workflow.
"""
import base64
import csv
import hashlib
import importlib.util
import io
import json
import math
from pathlib import Path
import re


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def prepare(directory, evidence, command, cli):
    spec = importlib.util.spec_from_file_location(
        "native_plan_original_pdf", Path(__file__).with_name("native-plan-link-fixture.py"))
    original = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(original)
    directory.mkdir(parents=True, exist_ok=False)
    sources = [directory / "Alpha.pdf", directory / "Beta.pdf"]
    for path, label in zip(sources, ("alpha", "beta")):
        path.write_bytes(original.pdf([
            ("0 0 1 rg\nBT /F1 28 Tf 72 700 Td (Native source " + label + ") Tj ET\n"
             "BT /F1 36 Tf 72 590 Td (A-101) Tj ET\n72 480 280 60 re f\n").encode("ascii")]))
    target = directory / "Target.pdf"
    target.write_bytes(original.pdf([
        b"0 0.6 0 rg\nBT /F1 28 Tf 72 700 Td (Native wrong first page) Tj ET\n72 480 280 60 re f\n",
        b"1 0 0 rg\nBT /F1 28 Tf 72 700 Td (Native destination two) Tj ET\n72 480 280 60 re f\n",
    ]))
    protected = {str(path): sha(path) for path in [*sources, target]}
    commands = []

    def invoke(arguments):
        # This allowlist also guards against accidentally turning preparation into
        # a CLI link-creation test when extending the native workflow.
        assert arguments[:2] in (["plan-set", "create"], ["plan-set", "edit-sheet"])
        result = json.loads(command([cli, *arguments]))
        commands.append({"arguments": [str(value) for value in arguments], "result": result})
        return result

    initial, manifest = directory / "indexed.json", directory / "plan-set.json"
    invoke(["plan-set", "create", *sources, target, "--name", "Native creation fixture", "--out", initial])
    indexed = json.loads(initial.read_text(encoding="utf-8"))
    identities = {file["path"]: file["id"] for file in indexed["files"]}
    sheet = next(sheet for sheet in indexed["sheets"]
                 if sheet["file_id"] == identities[target.name] and sheet["page"] == 2)
    invoke(["plan-set", "edit-sheet", initial, "--id", sheet["id"], "--number", "A-101",
            "--title", "Native destination two", "--revision", "R1", "--out", manifest])
    assert all(sha(path) == digest for path, digest in protected.items())
    protected[str(manifest)] = sha(manifest)
    outputs = [source.with_name(source.stem + "-linked.pdf") for source in sources]
    assert not any(output.exists() for output in outputs)
    assert all(b"/Annots" not in source.read_bytes() and b"/GoToR" not in source.read_bytes()
               for source in sources)
    report = {"schema": 1, "status": "prepared", "syntheticOnly": True,
              "method": "Original PDF generation and installed CLI manifest indexing only",
              "desktopLinkCreationTested": False, "desktopLinkSavingTested": False,
              "sourcesHaveNoAnnotations": True, "commands": commands,
              "protectedFiles": protected, **original.cli_provenance(cli),
              "sources": [{"path": str(path), "fileId": identities[path.name], "output": str(output)}
                          for path, output in zip(sources, outputs)],
              "target": {"path": str(target), "sheetId": sheet["id"], "physicalPage": 2,
                         "sha256": sha(target), "bytes": target.stat().st_size}}
    name = "native-plan-creation-fixture.json"
    (evidence / name).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return {"sources": sources, "outputs": outputs, "target": target, "manifest": manifest,
            "sourceIds": [identities[path.name] for path in sources], "sheetId": sheet["id"],
            "protected": protected, "fixtureReport": name}


class LabelMissing(AssertionError):
    pass


def label_box(tsv, label, size, *, line_contains=None, exact_line=False, below=None, within=None):
    """Resolve a unique actual OCR label, optionally scoped to its visible row.

    Punctuation-only glyphs are ignored. Words and numbers are never fuzzy matched,
    and an ambiguous row is refused instead of choosing the first occurrence.
    """
    def tokens(value):
        return re.findall(r"[a-z0-9]+(?:[.-][a-z0-9]+)*", value.casefold())

    groups = {}
    for row in csv.DictReader(io.StringIO(tsv), delimiter="\t", quoting=csv.QUOTE_NONE):
        if row.get("level") == "5" and tokens(row.get("text", "")):
            key = tuple(row.get(name) for name in ("page_num", "block_num", "par_num", "line_num"))
            groups.setdefault(key, []).append(row)
    wanted, qualifier = tokens(label), tokens(line_contains or "")
    assert wanted
    matches = []
    all_rows = [row for rows in groups.values() for row in rows]
    for rows in groups.values():
        expanded = [(token, row) for row in rows for token in tokens(row["text"])]
        words = [token for token, _ in expanded]
        if qualifier:
            # Sparse-text OCR can split adjacent controls into different blocks.
            # Scope to the same measured baseline, never another visual row.
            middle = sum(int(row["top"]) + int(row["height"]) / 2 for row in rows) / len(rows)
            height = max(int(row["height"]) for row in rows)
            same_line = sorted((row for row in all_rows
                                if abs(int(row["top"]) + int(row["height"]) / 2 - middle)
                                <= max(2, height * 0.55)), key=lambda row: int(row["left"]))
            qualified_words = [token for row in same_line for token in tokens(row["text"])]
            if not any(qualified_words[i:i + len(qualifier)] == qualifier
                       for i in range(len(qualified_words) - len(qualifier) + 1)):
                continue
        if exact_line and words != wanted:
            continue
        for i in range(len(words) - len(wanted) + 1):
            if words[i:i + len(wanted)] != wanted:
                continue
            matched = [row for _, row in expanded[i:i + len(wanted)]]
            box = (min(int(row["left"]) for row in matched), min(int(row["top"]) for row in matched),
                   max(int(row["left"]) + int(row["width"]) for row in matched),
                   max(int(row["top"]) + int(row["height"]) for row in matched))
            if not (0 <= box[0] < box[2] <= size[0] and 0 <= box[1] < box[3] <= size[1]):
                raise AssertionError("Native plan control escaped the captured window")
            if below is not None and box[1] <= below:
                continue
            if within is not None and not (within[0] <= box[0] < box[2] <= within[2]
                                           and within[1] <= box[1] < box[3] <= within[3]):
                continue
            matches.append(box)
    if not matches:
        raise LabelMissing(f"No visible native plan control: {label!r} ({line_contains!r})")
    assert len(matches) == 1, f"Ambiguous native plan control: {label!r} ({len(matches)} matches)"
    return matches[0]


def fixture_visible(kind, pixels, text):
    titles = {"alpha": "Native source alpha", "beta": "Native source beta",
              "destination": "Native destination two"}
    wanted = titles[kind]
    return (pixels > 1000 and len(re.findall(r"\b" + r"\s+".join(wanted.split()) + r"\b", text, re.I)) == 1
            and not re.search(r"\bNative\s+wrong\s+first\s+page\b", text, re.I)
            and all(other == wanted or not re.search(r"\b" + r"\s+".join(other.split()) + r"\b", text, re.I)
                    for other in titles.values()))


def pdf_snapshot(document):
    """Decode QPDF objects independently of application link reports."""
    objects = document["qpdf"][1]

    def resolve(value, seen=()):
        if isinstance(value, str) and re.fullmatch(r"\d+ \d+ R", value):
            assert value not in seen, "Cyclic object in qualified page content"
            obj = objects["obj:" + value]
            if "stream" in obj:
                stream = obj["stream"]
                return {"stream": base64.b64decode(stream["data"], validate=True).hex(),
                        "attributes": resolve({k: v for k, v in stream["dict"].items()
                                               if k not in ("/Length", "/Filter", "/DecodeParms")}, (*seen, value))}
            return resolve(obj["value"], (*seen, value))
        if isinstance(value, list):
            return [resolve(item, seen) for item in value]
        if isinstance(value, dict):
            return {key: resolve(item, seen) for key, item in value.items()}
        return value

    pages = []
    for page in document["pages"]:
        details = objects["obj:" + page["object"]]["value"]
        contents = b"".join(base64.b64decode(objects["obj:" + ref]["stream"]["data"], validate=True)
                            for ref in page["contents"])
        annotations = details.get("/Annots", [])
        if isinstance(annotations, str):
            annotations = objects["obj:" + annotations]["value"]
        decoded_annotations = []
        for annotation in annotations:
            if isinstance(annotation, str):
                annotation = objects["obj:" + annotation]["value"]
            # /P is an expected back-reference, not a resource to recursively
            # resolve through the page tree. Verify it before normalizing it.
            assert annotation["/P"] == page["object"], "Link annotation points at another page"
            decoded = resolve({key: value for key, value in annotation.items() if key != "/P"})
            decoded["/P"] = "verified-current-page"
            decoded_annotations.append(decoded)
        pages.append({"page": resolve({key: value for key, value in details.items()
                                       if key not in ("/Parent", "/Annots", "/Contents")}),
                      "contentSha256": hashlib.sha256(contents).hexdigest(),
                      "annotations": decoded_annotations})
    return pages


def pdf_string(value):
    assert isinstance(value, str)
    if value.startswith("u:"):
        return value[2:]
    assert value.startswith("b:"), "Expected a QPDF string"
    return bytes.fromhex(value[2:]).decode("utf-8")


def expected_target(fixture):
    target = fixture["target"]
    return {"relative_path": target.name, "page": 1, "sheet_id": fixture["sheetId"],
            "sha256": sha(target), "byte_len": target.stat().st_size}


def verify_link_page(before, after, target):
    assert len(before) == len(after) == 1, "Expected one original and one saved source page"
    assert before[0]["annotations"] == [], "Native source already contains annotations"
    assert before[0]["page"] == after[0]["page"], "Native save changed page geometry or resources"
    assert before[0]["contentSha256"] == after[0]["contentSha256"], "Native save changed page content"
    links = after[0]["annotations"]
    assert len(links) == 1 and links[0]["/Subtype"] == "/Link", "Expected exactly one authored link"
    link = links[0]
    assert not set(link).intersection({"/AA", "/Dest", "/JS"}), "Unexpected alternate link action"
    action = link["/A"]
    assert set(action) == {"/S", "/F", "/D", "/NewWindow", "/SuniPDFPlanTarget"}
    assert action["/S"] == "/GoToR" and action["/D"] == [1, "/Fit"] and action["/NewWindow"] is True
    file_spec = action["/F"]
    assert set(file_spec) == {"/Type", "/F", "/UF"} and file_spec["/Type"] == "/Filespec"
    assert pdf_string(file_spec["/F"]) == pdf_string(file_spec["/UF"]) == target["relative_path"]
    assert json.loads(pdf_string(action["/SuniPDFPlanTarget"])) == target
    rect = link["/Rect"]
    assert len(rect) == 4 and all(isinstance(v, (int, float)) and math.isfinite(v) for v in rect)
    # The synthetic A-101 reference is 36pt Helvetica at (72,590), with no rotation.
    assert 70 <= rect[0] < rect[2] <= 180 and 580 <= rect[1] < rect[3] <= 630
    assert rect[2] - rect[0] > 50 and rect[3] - rect[1] > 15
    return {"rect": rect, "annotationName": pdf_string(link["/NM"]), "target": target}


def verify_outputs(fixture, evidence, command):
    assert all(sha(path) == digest for path, digest in fixture["protected"].items()), "Original inputs changed"
    results = []
    for source, output, file_id in zip(fixture["sources"], fixture["outputs"], fixture["sourceIds"]):
        assert output.is_file() and not output.is_symlink() and output.parent == source.parent
        assert output != source and sha(output) != sha(source)
        snapshots, transcripts = [], []
        for path in (source, output):
            check = command(["qpdf", "--check", path])
            raw = command(["qpdf", "--json", "--json-stream-data=inline", path])
            stored = evidence / ("native-created-" + path.stem + "-qpdf.json")
            stored.write_text(raw + "\n", encoding="utf-8")
            snapshots.append(pdf_snapshot(json.loads(raw)))
            transcripts.append({"file": stored.name, "sha256": sha(stored), "check": check})
        link = verify_link_page(*snapshots, expected_target(fixture))
        results.append({"source": str(source), "sourceFileId": file_id, "sourceSha256": sha(source),
                        "output": str(output), "outputSha256": sha(output), "bytes": output.stat().st_size,
                        "qpdf": transcripts, **link})
    assert len({item["annotationName"] for item in results}) == 2, "Saved link IDs are not distinct"
    return results


def verify_receipt(fixture, receipt_path, outputs):
    exported = json.loads(receipt_path.read_text(encoding="utf-8"))
    assert set(exported) == {"kind", "receipt"} and exported["kind"] == "batch"
    receipt = exported["receipt"]
    assert receipt["version"] == 1 and receipt["status"] == "complete"
    assert receipt["failure"] is None and receipt["remaining"] == []
    assert receipt["last_input_validation"] == {"valid": True, "reason": None}
    assert len(receipt["completed"]) == len(outputs) == 2
    assert {item["output"] for item in receipt["completed"]} == {item["output"] for item in outputs}
    for saved in receipt["completed"]:
        verified = next(item for item in outputs if item["output"] == saved["output"])
        assert saved["bytes"] == verified["bytes"] and sha(saved["output"]) == verified["outputSha256"]
        report = saved["report"]
        assert report["version"] == 1 and report["output_sha256"] == verified["outputSha256"]
        inspection = report["inspection"]
        assert inspection["manifest_sha256"] == sha(fixture["manifest"])
        assert inspection["source_sha256"] == verified["sourceSha256"]
        assert inspection["source_file_id"] == verified["sourceFileId"]
        assert inspection["requested_pages"] == [1] and inspection["scanned_pages"] == 1
        assert inspection["scan_complete"] and not inspection["truncated"] and inspection["issues"] == []
        assert len(inspection["occurrences"]) == len(report["created"]) == 1
        created = report["created"][0]
        assert created["page"] == 1 and created["annotation_index"] == 0
        assert created["name"] == verified["annotationName"] and created["target"] == verified["target"]
        assert all(abs(a - b) < 0.001 for a, b in zip(created["rect"], verified["rect"]))
        assert len(created["rect"]) == 4 and report["excluded_occurrences"] == []
        occurrence = inspection["occurrences"][0]
        assert occurrence["id"] == created["occurrence_id"] and occurrence["status"] == "ready"
        assert occurrence["text"] == "A-101" and occurrence["target_sheet_ids"] == [fixture["sheetId"]]
    assert all(sha(path) == digest for path, digest in fixture["protected"].items())
    return {"path": str(receipt_path), "sha256": sha(receipt_path), "outputs": len(outputs),
            "sourceAndOutputHashesVerified": True, "physicalDestinationPage": 2}
