#!/usr/bin/env python3
"""Exercise installer trust boundaries without authorizing or installing a package."""

import hashlib
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
loader = importlib.machinery.SourceFileLoader("linux_update_installer", str(ROOT / "packaging/linux/install-update"))
spec = importlib.util.spec_from_loader(loader.name, loader)
helper = importlib.util.module_from_spec(spec)
loader.exec_module(helper)
package_spec = importlib.util.spec_from_file_location("linux_update_packaging", ROOT / "scripts/package-deb.py")
packaging = importlib.util.module_from_spec(package_spec)
package_spec.loader.exec_module(packaging)

VERSION = "0.74.0"
CONTENT = b"fixture package bytes; never passed to a package manager"
DIGEST = hashlib.sha256(CONTENT).hexdigest()
SIZE = len(CONTENT)
NAME = f"SuniPDF-{VERSION}-linux-amd64.deb"


def manifest():
    tag = "v" + VERSION
    return {"draft": False, "prerelease": False, "tag_name": tag,
            "html_url": helper.RELEASES + "/tag/" + tag,
            "assets": [{"name": NAME, "state": "uploaded", "size": SIZE,
                        "digest": "sha256:" + DIGEST,
                        "browser_download_url": helper.RELEASES + "/download/" + tag + "/" + NAME}]}


class InstallerBoundaries(unittest.TestCase):
    def test_request_accepts_canonical_identity_only(self):
        helper.validate_request("/private/" + NAME, DIGEST, VERSION, SIZE)
        for version in ("v0.74.0", "00.74.0", "0.74", "0.74.0-beta", "0.74.0+build", "-1.0.0", "1.2.3000000000"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                helper.validate_request("/private/" + NAME, DIGEST, version, SIZE)
        for path in (NAME, "/tmp/../" + NAME, "/tmp/./" + NAME, "/tmp/" + NAME + ".other", "/tmp/\n/" + NAME):
            with self.subTest(path=path), self.assertRaises(ValueError):
                helper.validate_request(path, DIGEST, VERSION, SIZE)
        for size in (0, -1, True, helper.MAX_PACKAGE + 1):
            with self.subTest(size=size), self.assertRaises(ValueError):
                helper.validate_request("/private/" + NAME, DIGEST, VERSION, size)
        for digest in (DIGEST[:-1], DIGEST.upper(), "g" * 64):
            with self.subTest(digest=digest), self.assertRaises(ValueError):
                helper.validate_request("/private/" + NAME, digest, VERSION, SIZE)

    def test_authorization_requires_root_and_positive_authenticated_uid(self):
        with patch.object(helper.sys, "platform", "linux"), patch.object(helper.os, "geteuid", return_value=0, create=True):
            with patch.dict(helper.os.environ, {"PKEXEC_UID": "1000"}, clear=True):
                self.assertEqual(helper.require_authorized_caller(), 1000)
            for uid in ("", "0", "-1", "01", "1000;root", "4294967295"):
                with patch.dict(helper.os.environ, {"PKEXEC_UID": uid}, clear=True), self.assertRaises(ValueError):
                    helper.require_authorized_caller()
        with patch.object(helper.sys, "platform", "linux"), patch.object(helper.os, "geteuid", return_value=1000, create=True):
            with self.assertRaises(ValueError):
                helper.require_authorized_caller()

    def test_release_identity_rejects_every_untrusted_boundary(self):
        helper.validate_release(json.dumps(manifest()).encode(), DIGEST, VERSION, SIZE)
        mutations = (
            lambda value: value.update(draft=True),
            lambda value: value.update(prerelease=True),
            lambda value: value.update(tag_name="v0.75.0"),
            lambda value: value.update(html_url="https://example.invalid/release"),
            lambda value: value.update(assets=[]),
            lambda value: value["assets"].append(dict(value["assets"][0])),
            lambda value: value["assets"][0].update(state="open"),
            lambda value: value["assets"][0].update(size=SIZE + 1),
            lambda value: value["assets"][0].update(digest="sha256:" + hashlib.sha256(b"other").hexdigest()),
            lambda value: value["assets"][0].update(browser_download_url="https://example.invalid/" + NAME),
            lambda value: value["assets"][0].update(name="different.deb"),
        )
        for mutation in mutations:
            value = manifest()
            mutation(value)
            with self.subTest(value=value), self.assertRaises(ValueError):
                helper.validate_release(json.dumps(value).encode(), DIGEST, VERSION, SIZE)
        with self.assertRaises(ValueError):
            helper.validate_release(b" " * (helper.MAX_METADATA + 1), DIGEST, VERSION, SIZE)
        for body in (b'{"draft":false,"draft":false}', b'{"draft":NaN}'):
            with self.assertRaises(ValueError):
                helper.validate_release(body, DIGEST, VERSION, SIZE)

    def test_transport_has_no_proxy_redirect_or_user_ca_override(self):
        request_url = helper.API + "v" + VERSION
        body = json.dumps(manifest()).encode()
        response = Mock(status=200, headers={"Content-Length": str(len(body))})
        response.geturl.return_value = request_url
        response.read.return_value = body
        response.__enter__ = Mock(return_value=response)
        response.__exit__ = Mock(return_value=False)
        opener = Mock()
        opener.open.return_value = response
        with patch.object(helper.ssl, "SSLContext") as context, patch.object(helper.urllib.request, "build_opener", return_value=opener) as build:
            helper.verify_public_release(DIGEST, VERSION, SIZE)
        context.assert_called_once_with(helper.ssl.PROTOCOL_TLS_CLIENT)
        context.return_value.load_verify_locations.assert_called_once_with(cafile="/etc/ssl/certs/ca-certificates.crt")
        self.assertEqual(build.call_args.args[0].proxies, {})
        self.assertIsInstance(build.call_args.args[1], helper.NoRedirect)
        self.assertEqual(opener.open.call_args.args[0].full_url, request_url)
        self.assertEqual(opener.open.call_args.kwargs, {"timeout": 30})
        response.read.assert_called_once_with(helper.MAX_METADATA + 1)
        with self.assertRaises(ValueError):
            helper.NoRedirect().redirect_request(None, None, 302, "redirect", {}, "https://example.invalid")

    def test_package_metadata_must_match_all_three_fields(self):
        expected = {"Package": "sunipdf", "Version": VERSION, "Architecture": "amd64"}
        calls = []

        def runner(arguments):
            calls.append(arguments)
            return expected[arguments[-1]]

        helper.validate_package(Path("/stage/") / NAME, VERSION, runner)
        self.assertEqual(len(calls), 3)
        self.assertTrue(all(arguments[:2] == ["/usr/bin/dpkg-deb", "--field"] for arguments in calls))
        for field in expected:
            old = expected[field]
            expected[field] = "untrusted"
            with self.subTest(field=field), self.assertRaises(ValueError):
                helper.validate_package(Path("/stage/") / NAME, VERSION, runner)
            expected[field] = old

    def test_installed_package_requires_official_version_and_architecture(self):
        self.assertEqual(helper.installed_version(lambda _: "installed\n0.73.0\namd64"), "0.73.0")
        for text in ("", "installed\n0.73.0\narm64", "unpacked\n0.73.0\namd64", "installed\n0.73.0-foreign\namd64"):
            with self.subTest(text=text), self.assertRaises(ValueError):
                helper.installed_version(lambda _: text)

    def test_install_refuses_downgrade_before_network_or_file_access(self):
        for current in (VERSION, "0.75.0"):
            with patch.object(helper, "require_authorized_caller", return_value=1000), patch.object(helper, "installed_version", return_value=current), patch.object(helper, "verify_public_release") as network, patch.object(helper, "copy_verified") as copy:
                with self.assertRaisesRegex(ValueError, "newer"):
                    helper.install("/private/" + NAME, DIGEST, VERSION, SIZE)
                network.assert_not_called()
                copy.assert_not_called()

    def test_install_stages_before_apt_and_checks_final_version(self):
        calls = []
        directory = tempfile.TemporaryDirectory()
        stage_directory = Path(directory.name)

        def copy(source, stage, digest, size, uid):
            self.assertEqual((digest, size, uid), (DIGEST, SIZE, 1000))
            self.assertTrue(str(source).startswith("/private/"))
            stage.write_bytes(CONTENT)
            calls.append("copy")

        def validate(stage, version):
            self.assertEqual(stage.read_bytes(), CONTENT)
            self.assertEqual(version, VERSION)
            calls.append("validate")

        def verify(digest, version, size):
            self.assertEqual((digest, version, size), (DIGEST, VERSION, SIZE))
            self.assertEqual((stage_directory / NAME).read_bytes(), CONTENT)
            calls.append("network")

        def run(arguments, timeout):
            self.assertEqual(arguments[:3], ["/usr/bin/apt-get", "--assume-yes", "--no-remove"])
            self.assertEqual(arguments[-3:-1], ["install", "--"])
            self.assertEqual(Path(arguments[-1]).read_bytes(), CONTENT)
            self.assertIsNone(timeout, "Never kill an active dpkg transaction")
            calls.append("apt")

        with patch.object(helper, "require_authorized_caller", return_value=1000), patch.object(helper, "installed_version", side_effect=["0.73.0", "0.73.0", VERSION]) as installed, patch.object(helper, "verify_public_release", side_effect=verify) as network, patch.object(helper.tempfile, "TemporaryDirectory", return_value=directory), patch.object(helper, "copy_verified", side_effect=copy), patch.object(helper, "validate_package", side_effect=validate), patch.object(helper, "run_bounded", side_effect=run):
            helper.install("/private/" + NAME, DIGEST, VERSION, SIZE)
        self.assertEqual(calls, ["copy", "network", "validate", "apt"])
        self.assertEqual(installed.call_count, 3)
        network.assert_called_once_with(DIGEST, VERSION, SIZE)
        self.assertFalse(stage_directory.exists())

    def test_invalid_local_bytes_do_not_consume_release_requests(self):
        directory = tempfile.TemporaryDirectory()
        stage_directory = Path(directory.name)
        with patch.object(helper, "require_authorized_caller", return_value=1000), patch.object(helper, "installed_version", return_value="0.73.0"), patch.object(helper.tempfile, "TemporaryDirectory", return_value=directory), patch.object(helper, "copy_verified", side_effect=ValueError("SHA256 verification failed")), patch.object(helper, "verify_public_release") as network, patch.object(helper, "validate_package") as metadata, patch.object(helper, "run_bounded") as apt:
            with self.assertRaisesRegex(ValueError, "SHA256"):
                helper.install("/private/" + NAME, DIGEST, VERSION, SIZE)
            network.assert_not_called()
            metadata.assert_not_called()
            apt.assert_not_called()
        self.assertFalse(stage_directory.exists())

    def test_public_verification_failure_never_parses_or_installs_staged_bytes(self):
        directory = tempfile.TemporaryDirectory()
        stage_directory = Path(directory.name)

        def copy(source, stage, digest, size, uid):
            stage.write_bytes(CONTENT)

        with patch.object(helper, "require_authorized_caller", return_value=1000), patch.object(helper, "installed_version", return_value="0.73.0"), patch.object(helper.tempfile, "TemporaryDirectory", return_value=directory), patch.object(helper, "copy_verified", side_effect=copy), patch.object(helper, "verify_public_release", side_effect=ValueError("public verification unavailable")), patch.object(helper, "validate_package") as metadata, patch.object(helper, "run_bounded") as apt:
            with self.assertRaisesRegex(ValueError, "public verification unavailable"):
                helper.install("/private/" + NAME, DIGEST, VERSION, SIZE)
            metadata.assert_not_called()
            apt.assert_not_called()
        self.assertFalse(stage_directory.exists())

    def test_staging_failure_or_concurrent_upgrade_never_runs_apt(self):
        for failure in ("copy", "validation", "newer-installed", "apt-failure", "post-version"):
            directory = tempfile.TemporaryDirectory()
            stage_directory = Path(directory.name)
            versions = ["0.73.0", "0.75.0" if failure == "newer-installed" else "0.73.0", "0.73.0"]
            with patch.object(helper, "require_authorized_caller", return_value=1000), patch.object(helper, "installed_version", side_effect=versions), patch.object(helper, "verify_public_release"), patch.object(helper.tempfile, "TemporaryDirectory", return_value=directory), patch.object(helper, "copy_verified", side_effect=ValueError("copy refused") if failure == "copy" else None), patch.object(helper, "validate_package", side_effect=ValueError("invalid metadata") if failure == "validation" else None), patch.object(helper, "run_bounded", side_effect=ValueError("apt failed") if failure == "apt-failure" else None) as apt:
                with self.subTest(failure=failure), self.assertRaises(ValueError):
                    helper.install("/private/" + NAME, DIGEST, VERSION, SIZE)
                self.assertEqual(apt.call_count, int(failure in ("apt-failure", "post-version")))
            self.assertFalse(stage_directory.exists())

    def test_package_installs_fixed_helper_and_authentication_policy(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            packaging.package_update_helper(stage)
            executable = stage / "usr/lib/sunipdf/install-update"
            self.assertTrue(executable.read_bytes().startswith(b"#!/usr/bin/python3 -I\n"))
            self.assertNotIn(b"\r", executable.read_bytes())
            policy = ET.parse(stage / "usr/share/polkit-1/actions/org.sunipdf.update.policy").getroot()
            action = policy.find("action")
            self.assertEqual(action.attrib["id"], "org.sunipdf.update")
            self.assertEqual(action.find("annotate").text, "/usr/lib/sunipdf/install-update")
            self.assertEqual(action.find("defaults/allow_active").text, "auth_admin")
            self.assertEqual(action.find("defaults/allow_any").text, "no")
            if sys.platform == "linux":
                self.assertEqual(executable.stat().st_mode & 0o777, 0o755)
        self.assertTrue({"pkexec", "python3", "ca-certificates"}.issubset(packaging.RUNTIME_DEPENDENCIES))

    @unittest.skipUnless(sys.platform == "linux", "POSIX ownership, nofollow and permission boundary")
    def test_copy_rejects_links_permissions_size_and_hash_and_preserves_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / NAME
            source.write_bytes(CONTENT)
            source.chmod(0o600)
            output = root / "stage.deb"
            helper.copy_verified(source, output, DIGEST, SIZE, os.getuid())
            self.assertEqual(output.read_bytes(), CONTENT)
            self.assertEqual(source.read_bytes(), CONTENT)
            self.assertEqual(output.stat().st_mode & 0o777, 0o600)
            output.unlink()
            for digest, size, uid in ((hashlib.sha256(b"wrong").hexdigest(), SIZE, os.getuid()),
                                       (DIGEST, SIZE + 1, os.getuid()), (DIGEST, SIZE, os.getuid() + 1)):
                with self.assertRaises((ValueError, OSError)):
                    helper.copy_verified(source, output, digest, size, uid)
                output.unlink(missing_ok=True)
            source.chmod(0o644)
            with self.assertRaises(ValueError):
                helper.copy_verified(source, output, DIGEST, SIZE, os.getuid())
            source.chmod(0o600)
            link = root / "linked.deb"
            link.symlink_to(source)
            with self.assertRaises(OSError):
                helper.copy_verified(link, output, DIGEST, SIZE, os.getuid())
            link.unlink()
            os.link(source, link)
            with self.assertRaises(ValueError):
                helper.copy_verified(source, output, DIGEST, SIZE, os.getuid())
            self.assertEqual(source.read_bytes(), CONTENT)

    @unittest.skipUnless(sys.platform == "linux", "POSIX subprocess output and process-group handling")
    def test_subprocess_output_is_bounded_and_environment_is_clean(self):
        output = helper.run_bounded([sys.executable, "-c", "import sys;sys.stdout.write('x'*200000)"])
        self.assertEqual(len(output), helper.MAX_OUTPUT)
        output = helper.run_bounded([sys.executable, "-c", "import os,json;print(json.dumps(dict(os.environ)))"])
        environment = json.loads(output)
        self.assertEqual(environment["HOME"], "/root")
        self.assertNotIn("PYTHONPATH", environment)
        self.assertNotIn("APT_CONFIG", environment)
        with self.assertRaisesRegex(ValueError, "failed"):
            helper.run_bounded([sys.executable, "-c", "raise SystemExit(7)"])
        with self.assertRaisesRegex(ValueError, "timed out"):
            helper.run_bounded([sys.executable, "-c", "import time;time.sleep(10)"], timeout=0.05)


if __name__ == "__main__":
    unittest.main(verbosity=2)
