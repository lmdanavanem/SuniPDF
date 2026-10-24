#!/usr/bin/env python3
"""Prepare an explicitly synthetic linked-PDF fixture with the installed CLI.

This is fixture preparation, not evidence of desktop link creation or saving.
"""
import hashlib
import json
from pathlib import Path


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def pdf(pages):
    """Small searchable pages with distinct coloured text; no external PDF dependency."""
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>", b"",
               b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"]
    kids = []
    for content in pages:
        page_id = len(objects) + 1
        kids.append(f"{page_id} 0 R")
        objects.append((f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] "
                        f"/Resources << /Font << /F1 3 0 R >> >> /Contents {page_id + 1} 0 R >>").encode())
        objects.append(b"<< /Length " + str(len(content)).encode() + b" >>\nstream\n" + content + b"\nendstream")
    objects[1] = (f"<< /Type /Pages /Kids [{' '.join(kids)}] /Count {len(kids)} >>").encode()
    data = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for index, body in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f"{index} 0 obj\n".encode() + body + b"\nendobj\n")
    xref = len(data)
    data.extend(f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode())
    for offset in offsets:
        data.extend(f"{offset:010d} 00000 n \n".encode())
    data.extend(f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    return bytes(data)


def prepare(directory, evidence, command, cli):
    directory.mkdir(parents=True, exist_ok=False)
    source = directory / "Native source.pdf"
    target = directory / "Native destination.pdf"
    source.write_bytes(pdf([b"0 0 1 rg\nBT /F1 28 Tf 72 700 Td (Native source document) Tj ET\n"
                            b"BT /F1 36 Tf 72 590 Td (A-101) Tj ET\n72 480 280 60 re f\n"]))
    target.write_bytes(pdf([
        b"0 0.6 0 rg\nBT /F1 28 Tf 72 700 Td (Native wrong first page) Tj ET\n72 480 280 60 re f\n",
        b"1 0 0 rg\nBT /F1 28 Tf 72 700 Td (Native destination two) Tj ET\n72 480 280 60 re f\n",
    ]))
    original = {str(path): sha(path) for path in (source, target)}
    commands = []

    def invoke(arguments):
        result = json.loads(command([cli, *arguments]))
        commands.append({"arguments": [str(value) for value in arguments], "result": result})
        return result

    initial = directory / "indexed.json"
    manifest = directory / "plan-set.json"
    invoke(["plan-set", "create", source, target, "--name", "Native navigation fixture", "--out", initial])
    indexed = json.loads(initial.read_text(encoding="utf-8"))
    source_id = next(file["id"] for file in indexed["files"] if file["path"] == source.name)
    target_id = next(file["id"] for file in indexed["files"] if file["path"] == target.name)
    sheet = next(sheet for sheet in indexed["sheets"] if sheet["file_id"] == target_id and sheet["page"] == 2)
    invoke(["plan-set", "edit-sheet", initial, "--id", sheet["id"], "--number", "A-101",
            "--title", "Native destination two", "--revision", "R1", "--out", manifest])
    inspected = invoke(["plan-links", "inspect", manifest, "--source-file", source_id,
                        "--pages", "1", "--target-sheet", sheet["id"]])
    inspection = inspected["inspection"]
    assert inspected["complete"] and len(inspection["occurrences"]) == 1
    occurrence = inspection["occurrences"][0]
    assert occurrence["status"] == "ready" and occurrence["target_sheet_ids"] == [sheet["id"]]
    review = directory / "review.json"
    review.write_text(json.dumps({
        "version": 1, "manifest_sha256": inspection["manifest_sha256"],
        "source_sha256": inspection["source_sha256"],
        "scan": {"source_file_id": source_id, "pages": [1], "target_sheet_ids": [sheet["id"]]},
        "selections": [{"occurrence_id": occurrence["id"], "fingerprint": occurrence["fingerprint"],
                        "target_sheet_id": sheet["id"], "confirmed": True}],
    }, indent=2) + "\n", encoding="utf-8")
    linked = directory / "Linked source.pdf"
    applied = invoke(["plan-links", "apply", manifest, "--review", review, "--out", linked])
    assert applied["published"] and len(applied["report"]["created"]) == 1
    qpdf = command(["qpdf", "--check", linked])
    annotations = invoke(["annotations", linked, "--page", "1"])
    links = [item for item in annotations["pages"][0]["annotations"] if item["subtype"] == "Link"]
    assert len(links) == 1
    link = links[0]["link"]
    assert link["remote_page"] == 2 and link["relative_path"] == target.name
    assert link["sheet_id"] == sheet["id"] and link["sha256"] == sha(target)
    assert link["requires_verified_plan_set"] and link["byte_len"] == target.stat().st_size
    assert all(sha(Path(path)) == digest for path, digest in original.items())
    protected = {str(path): sha(path) for path in (source, target, manifest, linked)}
    report = {
        "schema": 1, "status": "prepared", "method": "Installed CLI fixture preparation",
        "desktopLinkCreationTested": False, "desktopLinkSavingTested": False,
        "syntheticOnly": True, "cliSha256": sha(cli), "commands": commands,
        "qpdf": {"scope": "Structural validation of the prepared fixture, not external-reader interoperability",
                 "output": qpdf}, "protectedFiles": protected,
        "linkRectPdfPoints": links[0]["rect"], "expectedPhysicalDestinationPage": 2,
    }
    (evidence / "native-plan-link-fixture.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return {"source": source, "target": target, "manifest": manifest, "linked": linked,
            "protected": protected, "fixtureReport": "native-plan-link-fixture.json"}
