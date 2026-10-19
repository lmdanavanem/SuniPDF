"""Exercise the actual CLI with independently written PDF bytes; no application installation."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import subprocess
import tempfile
import time


def pdf(objects):
    data = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for number, obj in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f"{number} 0 obj\n".encode() + obj + b"\nendobj\n")
    xref = len(data)
    data.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    for offset in offsets[1:]:
        data.extend(f"{offset:010} 00000 n \n".encode())
    data.extend(f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    return bytes(data)


def stream(data, attributes=b""):
    return b"<< /Length " + str(len(data)).encode() + b" " + attributes + b" >>\nstream\n" + data + b"\nendstream"


def base(content=b""):
    return [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources << >> >>",
        stream(content),
    ]


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    started = time.monotonic()
    cases = []
    kinds = ["javascript", "automatic_actions", "external_actions", "embedded_files", "xfa", "multimedia", "signatures"]
    commands = 0
    with tempfile.TemporaryDirectory(prefix="sunipdf-inspection-") as directory:
        root = Path(directory)

        def run(arguments, code, env=None):
            nonlocal commands
            output = subprocess.run([str(cli), *map(str, arguments)], capture_output=True,
                                    timeout=20, env={**os.environ, **(env or {})}, cwd=root)
            commands += 1
            assert output.returncode == code, (arguments[0], output.returncode, output.stderr[:1000])
            assert len(output.stdout) < 2 * 1024 * 1024 and len(output.stderr) < 2 * 1024 * 1024
            if code in (0, 2):
                return json.loads(output.stdout)
            assert not output.stdout
            return json.loads(output.stderr)

        def check(name, data, counts, complete=True):
            path = root / f"{name} été 中文.pdf"
            path.write_bytes(data)
            before = sha(path)
            report = run(["inspect", path], 0 if complete else 2)
            assert report["complete"] is complete and report["source_sha256"] == before
            assert report["source_bytes"] == len(data)
            assert [finding["kind"] for finding in report["findings"]] == kinds
            assert [finding["dictionaries"] for finding in report["findings"]] == counts
            assert all(report[key] is False for key in ["stream_contents_examined", "malware_scan",
                       "signature_integrity_verified", "conformance_validated", "network_accessed", "file_modified"])
            assert sha(path) == before
            cases.append(name)
            return path

        plain = check("ordinary-dictionaries", pdf(base()), [0] * 7)
        objects = base()
        objects[0] = b"<< /Type /Catalog /Pages 2 0 R /OpenAction 5 0 R /Names << /EmbeddedFiles << /Names [(example.txt) 6 0 R] >> >> /AcroForm 8 0 R >>"
        objects += [
            b"<< /S /JavaScript /JS (void 0) >>",
            b"<< /Type /Filespec /F (example.txt) /EF << /F 7 0 R >> >>",
            stream(b"hello", b"/Type /EmbeddedFile"),
            b"<< /XFA () /Fields [9 0 R] >>",
            b"<< /FT /Sig /V 10 0 R >>",
            b"<< /Type /Sig /Contents <> >>",
            b"<< /Subtype /Screen /S /URI /URI (https://example.invalid/never-opened) >>",
        ]
        check("all-seven-features-and-unreachable-objects", pdf(objects), [1, 1, 1, 3, 1, 1, 2])
        check("stream-body-is-not-interpreted-as-dictionaries", pdf(base(b"/S /JavaScript /JS (decoy)")), [0] * 7)
        check("dangling-reference-is-partial", pdf(base() + [b"<< /JS 9999 0 R >>"]), [1, 0, 0, 0, 0, 0, 0], False)
        check("cyclic-feature-name-is-partial", pdf(base() + [b"5 0 R", b"<< /S 5 0 R >>"]), [0] * 7, False)

        malformed = root / "malformed.pdf"
        malformed.write_bytes(b"not a PDF")
        assert run(["inspect", malformed], 4)["error"] == "invalid_format"
        assert malformed.read_bytes() == b"not a PDF"
        cases.append("malformed-input-has-typed-error")
        assert run(["inspect", root / "absent.pdf"], 5)["error"] == "io"
        cases.append("missing-input-has-typed-error")
        large = root / "large.pdf"
        with large.open("wb") as file:
            file.truncate(128 * 1024 * 1024 + 1)
        assert "128 MiB" in run(["inspect", large], 1)["message"]
        assert large.stat().st_size == 128 * 1024 * 1024 + 1
        cases.append("oversized-file-is-refused")

        credentials = {"SUNIPDF_INSPECTION_USER": secrets.token_hex(24),
                       "SUNIPDF_INSPECTION_OWNER": secrets.token_hex(24)}
        for aes in [128, 256]:
            protected = root / f"protected-{aes}.pdf"
            run(["protect", plain, "--user-password-env", "SUNIPDF_INSPECTION_USER",
                 "--owner-password-env", "SUNIPDF_INSPECTION_OWNER", "--out", protected]
                + (["--aes128"] if aes == 128 else []), 0, credentials)
            original = sha(protected)
            assert run(["inspect", protected], 3)["error"] == "password_required"
            cases.append(f"aes{aes}-missing-credential-is-explicit")
            for role in ["USER", "OWNER"]:
                report = run(["inspect", protected, "--password-env", f"SUNIPDF_INSPECTION_{role}"], 0, credentials)
                assert report["complete"] and report["source_sha256"] == original
                assert all(finding["dictionaries"] == 0 for finding in report["findings"])
                assert not any(secret in json.dumps(report) for secret in credentials.values())
                assert sha(protected) == original
                cases.append(f"aes{aes}-{role.lower()}-environment-credential")
        assert sha(plain) == hashlib.sha256(pdf(base())).hexdigest()
    assert len(cases) == 14
    report = dict(status="passed", gate="pdf-inspection-0.34.0", cli_sha256=sha(cli), cases=cases,
                  completed_cases=len(cases), subprocess_calls=commands, elapsed_seconds=round(time.monotonic()-started, 3),
                  source_revision=os.environ.get("GITHUB_SHA"), os=os.name, production_credentials_used=False,
                  secret_argv_used=False, application_installed=False, native_gui_qualified=False,
                  input_sources="independently written PDF/xref bytes; encrypted copies made by packaged CLI")
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
