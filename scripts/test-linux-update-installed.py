#!/usr/bin/env python3
"""Exercise the candidate installer with two public releases in disposable Ubuntu only."""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import signal
import ssl
import stat
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
RELEASES = "https://github.com/lmdanavanem/SuniPDF/releases"
MAX_PACKAGE = 256 * 1024 * 1024
MAX_LOG = 64 * 1024
ACCOUNT = "sunipdf-test"
HELPER = Path("/usr/lib/sunipdf/install-update")
SAFE_ENV = {"PATH": "/usr/sbin:/usr/bin:/sbin:/bin", "HOME": "/root",
            "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8", "DEBIAN_FRONTEND": "noninteractive"}


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(128 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def public_pin(value, version):
    require(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version), "Invalid historical baseline version")
    name = f"SuniPDF-{version}-linux-amd64.deb"
    url = f"{RELEASES}/download/v{version}/{name}"
    require(value.get("version") == version and value.get("tag") == "v" + version,
            "Historical evidence version does not match the requested baseline")
    matches = [item for item in value.get("assets", []) if item.get("name") == name]
    require(len(matches) == 1, "Historical Ubuntu package is missing or ambiguous")
    pin = matches[0]
    require(pin.get("url") == url and pin.get("public_unauthenticated") is True,
            "Historical package was not verified as the exact public asset")
    require(type(pin.get("size")) is int and 0 < pin["size"] <= MAX_PACKAGE,
            "Historical package size is invalid")
    require(isinstance(pin.get("sha256"), str) and re.fullmatch(r"[0-9a-f]{64}", pin["sha256"]),
            "Historical package digest is invalid")
    return {key: pin[key] for key in ("name", "url", "size", "sha256")}


def trusted_download_url(url):
    parsed = urllib.parse.urlsplit(url)
    return (parsed.scheme == "https" and parsed.hostname in
            {"github.com", "release-assets.githubusercontent.com", "objects.githubusercontent.com"}
            and parsed.port in (None, 443) and parsed.username is None and parsed.password is None)


class AssetRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, newurl):
        require(trusted_download_url(newurl), "Historical download left the official HTTPS asset hosts")
        return super().redirect_request(request, response, code, message, headers, newurl)


def download(pin, destination):
    require(trusted_download_url(pin["url"]), "Invalid historical download URL")
    context = ssl.create_default_context(cafile="/etc/ssl/certs/ca-certificates.crt")
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), AssetRedirect(),
                                         urllib.request.HTTPSHandler(context=context))
    request = urllib.request.Request(pin["url"], headers={"User-Agent": "SuniPDF-installed-update-test"})
    started = time.monotonic()
    with opener.open(request, timeout=60) as response, destination.open("xb") as output:
        require(response.status == 200 and trusted_download_url(response.geturl()), "Unexpected asset response")
        length = response.headers.get("Content-Length")
        require(length is None or length == str(pin["size"]), "Historical asset length differs")
        count = 0
        while chunk := response.read(128 * 1024):
            count += len(chunk)
            require(count <= pin["size"] and time.monotonic() - started < 180,
                    "Historical download exceeded its size or time bound")
            output.write(chunk)
    require(destination.stat().st_size == pin["size"] and sha(destination) == pin["sha256"],
            "Historical package failed its pinned SHA256/size check")


def run_record(arguments, name, evidence, records, expected=0, timeout=360, environment=None):
    """Keep a bounded command transcript; a fixture timeout fails this disposable container."""
    process = subprocess.Popen([str(item) for item in arguments], stdin=subprocess.DEVNULL,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                               cwd="/", env=environment or SAFE_ENV, start_new_session=True)
    tail = bytearray()
    output_bytes = 0

    def drain():
        nonlocal output_bytes
        with process.stdout:
            while chunk := process.stdout.read(4096):
                output_bytes += len(chunk)
                tail.extend(chunk)
                if len(tail) > MAX_LOG:
                    del tail[:-MAX_LOG]

    reader = threading.Thread(target=drain, daemon=True)
    reader.start()
    timed_out = False
    try:
        code = process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(process.pid, signal.SIGTERM)
        try:
            code = process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            code = process.wait(timeout=10)
    reader.join(timeout=10)
    text = bytes(tail).decode("utf-8", errors="replace")
    log = evidence / (name + ".log")
    log.write_bytes(bytes(tail))
    records.append({"name": name, "argv": [str(item) for item in arguments], "exitCode": code,
                    "timedOut": timed_out, "log": log.name, "logSha256": sha(log),
                    "outputBytes": output_bytes, "retainedBytes": len(tail)})
    require(not reader.is_alive() and not timed_out, "Installed update fixture command timed out: " + name)
    require(code == expected, f"Installed update fixture command {name} returned {code}: {text[-4096:]}")
    return text.strip()


def inventory():
    value = json.loads(Path("/usr/share/doc/sunipdf/licenses/dependency-inventory.json").read_text())
    require(value.get("platform") == "linux" and value.get("architecture") == "amd64", "Wrong installed inventory")
    for relative, record in value["payloadFiles"].items():
        file = Path("/") / relative
        require(relative.startswith("usr/") and ".." not in Path(relative).parts and not file.is_symlink(),
                "Unsafe installed inventory path")
        require(file.is_file() and file.stat().st_size == record["bytes"] and sha(file) == record["sha256"],
                "Installed candidate differs from its payload inventory: " + relative)
    require("usr/lib/sunipdf/install-update" in value["payloadFiles"], "Candidate inventory omits its installer")
    return value


def helper_arguments(helper, package, pin, version):
    return ["/usr/bin/python3", "-I", str(helper), "--package", str(package),
            "--sha256", pin["sha256"], "--version", version, "--size", str(pin["size"])]


def verify_updates(candidate, evidence, source_sha, protected):
    require(sys.platform == "linux" and hasattr(os, "geteuid") and os.geteuid() == 0
            and Path("/.dockerenv").is_file(), "Installed upgrade qualification requires a disposable root Docker container")
    import pwd
    user = pwd.getpwnam(ACCOUNT)
    require(user.pw_uid > 0 and Path(user.pw_dir) == Path("/home") / ACCOUNT, "Missing isolated test account")
    require(re.fullmatch(r"[0-9a-f]{40}", source_sha or "") and os.environ.get("GITHUB_SHA") == source_sha,
            "Exact candidate workflow source is required")
    candidate = Path(candidate)
    require(candidate.is_file() and not candidate.is_symlink(), "Missing candidate package")
    candidate = candidate.resolve(strict=True)
    evidence = Path(evidence)
    require(evidence.is_dir() and not evidence.is_symlink(), "Missing evidence directory")
    require(protected and all(path.is_file() and not path.is_symlink()
                             and path.resolve().is_relative_to(Path(user.pw_dir))
                             and sha(path) == digest for path, digest in protected.items()),
            "Runtime fixture or preferences are not preserved")
    provenance_path = Path("/usr/share/doc/sunipdf/build-provenance.json")
    require(json.loads(provenance_path.read_text())["sourceRevision"] == source_sha,
            "Installed candidate source does not match the workflow")
    before_inventory = inventory()
    helper_stat = HELPER.stat()
    require(not HELPER.is_symlink() and stat.S_ISREG(helper_stat.st_mode) and helper_stat.st_uid == 0
            and helper_stat.st_mode & 0o022 == 0, "The candidate helper is not a protected root-owned regular file")
    helper_sha = sha(HELPER)
    pins = {}
    for version in ("0.72.0", "0.73.0"):
        file = ROOT / "docs/qa" / f"RELEASE_{version}.json"
        pins[version] = public_pin(json.loads(file.read_text(encoding="utf-8")), version)
        pins[version]["evidenceSha256"] = sha(file)
    records = []
    report = {"schema": 1, "status": "failed", "sourceSha": source_sha,
              "candidate": {"name": candidate.name, "sha256": sha(candidate)},
              "helperSha256": helper_sha, "publicBaselines": pins,
              "polkitAuthenticationTested": False, "desktopUpdaterInteractionTested": False,
              "scope": "Exact candidate helper invoked as root with the isolated desktop user's PKEXEC_UID. "
                       "Real APT upgrade between two pinned public DEB releases. No polkit dialog or desktop updater interaction.",
              "commands": records, "candidateRestored": False, "userDataPreserved": False}
    query = ["/usr/bin/dpkg-query", "--show", "--showformat=${Version}", "sunipdf"]
    installed = lambda name: run_record(query, name, evidence, records, timeout=30)
    initial_version = installed("linux-update-initial-version")
    require(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", initial_version)
            and tuple(map(int, initial_version.split("."))) > (0, 73, 0), "Candidate must be newer than both public baselines")
    report["candidate"]["version"] = initial_version
    changed = False
    failure = None
    try:
        with tempfile.TemporaryDirectory(prefix="sunipdf-update-ci-", dir="/var/tmp") as directory:
            private = Path(directory)
            require(private.stat().st_uid == 0 and private.stat().st_mode & 0o077 == 0,
                    "Installer fixture staging directory is not private")
            copied_helper = private / "install-update"
            shutil.copyfile(HELPER, copied_helper)
            copied_helper.chmod(0o755)
            require(sha(copied_helper) == helper_sha, "Copied installer differs from the exact candidate")
            packages = {}
            for version, pin in pins.items():
                path = private / pin["name"]
                download(pin, path)
                packages[version] = path
            with tempfile.TemporaryDirectory(prefix="sunipdf-update-fixture-", dir=user.pw_dir) as user_directory:
                user_root = Path(user_directory)
                os.chown(user_root, user.pw_uid, user.pw_gid)
                user_package = user_root / pins["0.73.0"]["name"]
                shutil.copyfile(packages["0.73.0"], user_package)
                user_package.chmod(0o600)
                os.chown(user_package, user.pw_uid, user.pw_gid)
                changed = True
                run_record(["/usr/bin/apt-get", "--assume-yes", "--no-remove", "--allow-downgrades",
                            "install", "--", packages["0.72.0"]], "linux-update-baseline-install", evidence, records)
                require(installed("linux-update-before") == "0.72.0", "Public baseline was not installed")
                report["versionBefore"] = "0.72.0"
                helper_env = dict(SAFE_ENV, PKEXEC_UID=str(user.pw_uid))
                args = helper_arguments(copied_helper, user_package, pins["0.73.0"], "0.73.0")
                with user_package.open("r+b") as stream:
                    value = stream.read(1)
                    stream.seek(0)
                    stream.write(bytes([value[0] ^ 1]))
                corrupt = json.loads(run_record(args, "linux-update-corrupt-refused", evidence, records,
                                                expected=1, environment=helper_env))
                require(corrupt.get("status") == "failed" and "SHA256" in corrupt.get("error", ""),
                        "Corrupt package did not reach the SHA256 refusal")
                require(installed("linux-update-after-corrupt") == "0.72.0", "Corrupt package changed the installation")
                report["corruptPackageRefused"] = True
                shutil.copyfile(packages["0.73.0"], user_package)
                require(sha(user_package) == pins["0.73.0"]["sha256"], "Public fixture was not restored")
                success = json.loads(run_record(args, "linux-update-helper-upgrade", evidence, records,
                                                environment=helper_env))
                require(success == {"status": "installed", "version": "0.73.0"}, "Helper did not confirm the actual upgrade")
                require(installed("linux-update-after") == "0.73.0", "APT did not install the requested public release")
                report["versionAfter"] = "0.73.0"
                cli = run_record(["/usr/bin/sunipdf-cli", "--version"], "linux-update-cli-version", evidence, records, timeout=30)
                require(cli == "sunipdf-cli 0.73.0", "Updated CLI reports another version")
                report["updatedCliVersion"] = cli
                repeated = json.loads(run_record(args, "linux-update-same-version-refused", evidence, records,
                                                 expected=1, environment=helper_env))
                require(repeated.get("status") == "failed" and "newer" in repeated.get("error", ""),
                        "Same-version refusal was not reported")
                require(installed("linux-update-after-repeat") == "0.73.0", "Rejected reinstall changed the installation")
                report["sameVersionRefused"] = True
                require(all(sha(path) == value for path, value in protected.items()), "Upgrade changed a document or preferences")
                require(sha(copied_helper) == helper_sha and sha(user_package) == pins["0.73.0"]["sha256"],
                        "Qualification changed a verified helper or public package")
    except Exception as error:
        failure = error
        report["error"] = str(error)[:4096]
    finally:
        if changed:
            try:
                run_record(["/usr/bin/apt-get", "--assume-yes", "--no-remove", "install", "--", candidate],
                           "linux-update-candidate-restore", evidence, records)
                require(installed("linux-update-restored-version") == initial_version, "Candidate version was not restored")
                require(inventory() == before_inventory and sha(HELPER) == helper_sha,
                        "Restored candidate differs from the qualified inventory")
                require(json.loads(provenance_path.read_text())["sourceRevision"] == source_sha,
                        "Restored candidate source differs")
                report["candidateRestored"] = True
            except Exception as error:
                report["restoreError"] = str(error)[:4096]
                failure = failure or error
        report["userDataPreserved"] = all(path.is_file() and sha(path) == value for path, value in protected.items())
        if failure is None and report["candidateRestored"] and report["userDataPreserved"]:
            report["status"] = "passed"
        (evidence / "linux-update-installed.json").write_text(json.dumps(report, indent=2) + "\n")
    if failure is not None:
        raise failure
    require(report["status"] == "passed", "Installed update qualification was incomplete")
    return report


class BoundaryTests(unittest.TestCase):
    def fixture(self):
        return {"version": "0.73.0", "tag": "v0.73.0", "assets": [{
            "name": "SuniPDF-0.73.0-linux-amd64.deb", "size": 24,
            "url": RELEASES + "/download/v0.73.0/SuniPDF-0.73.0-linux-amd64.deb",
            "sha256": "a" * 64, "public_unauthenticated": True}]}

    def test_public_pin_requires_one_exact_verified_asset(self):
        self.assertEqual(public_pin(self.fixture(), "0.73.0")["size"], 24)
        for field, value in (("size", 0), ("size", True), ("size", MAX_PACKAGE + 1),
                             ("sha256", "a" * 63), ("url", "https://example.com/update.deb"),
                             ("public_unauthenticated", False)):
            evidence = self.fixture()
            evidence["assets"][0][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(AssertionError):
                public_pin(evidence, "0.73.0")
        duplicate = self.fixture()
        duplicate["assets"] *= 2
        with self.assertRaises(AssertionError):
            public_pin(duplicate, "0.73.0")

    def test_redirects_never_accept_other_hosts_credentials_or_http(self):
        self.assertTrue(trusted_download_url("https://release-assets.githubusercontent.com/asset?token=fixture"))
        for url in ("http://github.com/file", "https://github.com.evil.test/file", "https://user@github.com/file",
                    "https://github.com:444/file", "file:///tmp/update.deb"):
            self.assertFalse(trusted_download_url(url), url)

    def test_non_linux_refuses_before_starting_any_process(self):
        with patch.object(sys, "platform", "win32"), patch.object(subprocess, "Popen") as launch:
            with self.assertRaisesRegex(AssertionError, "disposable root Docker"):
                verify_updates(None, None, "", {})
            launch.assert_not_called()

    def test_helper_argv_preserves_paths_as_one_literal_argument(self):
        pin = public_pin(self.fixture(), "0.73.0")
        arguments = helper_arguments(PurePosixPath("/root/helper"), PurePosixPath("/home/test/a b;$(x)/SuniPDF-0.73.0-linux-amd64.deb"), pin, "0.73.0")
        self.assertEqual(arguments[:4], ["/usr/bin/python3", "-I", "/root/helper", "--package"])
        self.assertEqual(arguments[4], "/home/test/a b;$(x)/SuniPDF-0.73.0-linux-amd64.deb")
        self.assertEqual(arguments[-2:], ["--size", "24"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    options = parser.parse_args()
    if options.self_test:
        unittest.main(argv=[sys.argv[0]], verbosity=2)
    else:
        parser.error("Use through the disposable test-deb lifecycle or pass --self-test")
