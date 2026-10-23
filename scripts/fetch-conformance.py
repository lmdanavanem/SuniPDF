"""Fetch the pinned, attributed conformance corpus and optional CI-only validator."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tempfile
import urllib.parse
import urllib.request
import unittest
import zipfile
from xml.sax.saxutils import escape

MANIFEST = Path(__file__).with_name('conformance-corpus.json')
REVISION = '377728596647e40edf16003d68d620773ce96ec5'
VALIDATOR_SHA256 = 'b512bf24945503a33fc10738ecffc1f2dccaaf8593a448e94ee9ac86a9559a59'


def sha256(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def safe_path(root, relative):
    parts = PurePosixPath(relative)
    if not relative or '\\' in relative or ':' in relative or parts.is_absolute() or '..' in parts.parts:
        raise ValueError('Unsafe corpus path')
    root = root.absolute()
    path = root / relative
    if any(p.is_symlink() for p in (root, *root.parents, path, *path.parents)):
        raise ValueError('Symlinks are not accepted for qualification evidence')
    if root.resolve() not in path.resolve().parents:
        raise ValueError('Corpus path escapes its directory')
    return path


def manifest():
    value = json.loads(MANIFEST.read_text(encoding='utf-8'))
    files = value.get('files', [])
    if value.get('schema') != 1 or value.get('revision') != REVISION or value.get('license') != 'CC-BY-4.0' or not 6 <= len(files) <= 20:
        raise ValueError('Invalid corpus manifest')
    seen = set()
    for item in files:
        if not re.fullmatch(r'[a-z0-9-]{1,64}', item['id']) or item['id'] in seen:
            raise ValueError('Invalid or duplicate corpus identifier')
        seen.add(item['id'])
        safe_path(Path('.'), item['path'])
        if not item['path'].endswith('.pdf') or not 100 <= item['bytes'] <= 8 * 1024 * 1024 or not re.fullmatch(r'[0-9a-f]{64}', item['sha256']):
            raise ValueError('Invalid corpus file bounds or digest')
        if item['profile'] not in ('1b', '2b', 'ua1') or type(item['expectedCompliant']) is not bool:
            raise ValueError('Invalid corpus expectation')
    if sum(item['bytes'] for item in files) > 64 * 1024 * 1024:
        raise ValueError('Corpus exceeds aggregate limit')
    validator = value['validator']
    if validator != {'version': '1.30.3', 'url': 'https://software.verapdf.org/releases/1.30/verapdf-greenfield-1.30.3-installer.zip', 'bytes': 32971125, 'sha256': VALIDATOR_SHA256}:
        raise ValueError('Unexpected validator provenance')
    return value


def allowed_url(url):
    parsed = urllib.parse.urlsplit(url)
    return (parsed.scheme == 'https' and parsed.port in (None, 443) and not parsed.username
            and not parsed.password and not parsed.query and not parsed.fragment
            and ((parsed.hostname == 'raw.githubusercontent.com' and parsed.path.startswith('/veraPDF/veraPDF-corpus/' + REVISION + '/'))
                 or url == 'https://software.verapdf.org/releases/1.30/verapdf-greenfield-1.30.3-installer.zip'))


class RestrictedRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, newurl):
        if not allowed_url(newurl):
            raise ValueError('Download redirect leaves the pinned upstream origin')
        return super().redirect_request(request, fp, code, message, headers, newurl)


def fetch(url, destination, size, digest):
    if not allowed_url(url):
        raise ValueError('Unapproved download origin')
    if destination.exists():
        if destination.is_symlink() or destination.stat().st_size != size or sha256(destination) != digest:
            raise ValueError('Existing downloaded file does not match its pinned digest')
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    opener = urllib.request.build_opener(RestrictedRedirect())
    with opener.open(url, timeout=60) as response:
        if not allowed_url(response.url):
            raise ValueError('Unapproved final origin')
        data = response.read(size + 1)
    if len(data) != size or hashlib.sha256(data).hexdigest() != digest:
        raise ValueError('Download differs from the pinned size or digest')
    with destination.open('xb') as stream:
        stream.write(data)


def install_validator(destination, value):
    if os.environ.get('GITHUB_ACTIONS') != 'true' or os.name != 'posix':
        raise ValueError('Validator installation is restricted to Linux GitHub Actions runners')
    destination = destination.absolute()
    safe_path(destination.parent, destination.name)
    if destination.exists():
        raise ValueError('Validator installation requires a new directory')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=destination.parent) as temporary:
        work = Path(temporary)
        archive = work / 'validator.zip'
        fetch(value['url'], archive, value['bytes'], value['sha256'])
        member = 'verapdf-greenfield-1.30.3/verapdf-izpack-installer-1.30.3.jar'
        with zipfile.ZipFile(archive) as bundle:
            entry = bundle.getinfo(member)
            if len(bundle.infolist()) > 20 or entry.file_size > 64 * 1024 * 1024 or (entry.external_attr >> 16) & 0o170000 == 0o120000:
                raise ValueError('Unsafe validator archive')
            jar = work / 'installer.jar'
            jar.write_bytes(bundle.read(entry))
        config = work / 'install.xml'
        config.write_text('<AutomatedInstallation langpack="eng">'
            '<com.izforge.izpack.panels.htmlhello.HTMLHelloPanel id="welcome"/>'
            '<com.izforge.izpack.panels.target.TargetPanel id="install_dir"><installpath>' + escape(str(destination)) + '</installpath></com.izforge.izpack.panels.target.TargetPanel>'
            '<com.izforge.izpack.panels.packs.PacksPanel id="sdk_pack_select">'
            '<pack index="0" name="veraPDF GUI" selected="false"/>'
            '<pack index="1" name="veraPDF CLI" selected="true"/>'
            '<pack index="2" name="veraPDF Documentation" selected="false"/>'
            '<pack index="3" name="veraPDF Sample Plugins" selected="false"/>'
            '</com.izforge.izpack.panels.packs.PacksPanel>'
            '<com.izforge.izpack.panels.install.InstallPanel id="install"/>'
            '<com.izforge.izpack.panels.finish.FinishPanel id="finish"/></AutomatedInstallation>', encoding='utf-8')
        result = subprocess.run(['java', '-jar', str(jar), str(config)], capture_output=True, timeout=180)
        if result.returncode or not (destination / 'verapdf').is_file():
            raise ValueError('Validator installation failed: ' + result.stderr.decode(errors='replace')[-2000:])
    (destination / 'provenance.json').write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


class FetchTests(unittest.TestCase):
    def test_manifest(self):
        self.assertEqual(len(manifest()['files']), 12)

    def test_path_escape(self):
        for value in ('../escape', '/root/file', 'C:/file', 'folder\\file', ''):
            with self.assertRaises(ValueError):
                safe_path(Path('target'), value)

    def test_download_origin(self):
        self.assertTrue(allowed_url('https://raw.githubusercontent.com/veraPDF/veraPDF-corpus/' + REVISION + '/file.pdf'))
        for value in ('http://raw.githubusercontent.com/file', 'https://raw.githubusercontent.com.attacker.invalid/file', 'https://user@software.verapdf.org/file', 'https://raw.githubusercontent.com/veraPDF/veraPDF-corpus/main/file.pdf'):
            self.assertFalse(allowed_url(value))

    def test_validator_requires_ci(self):
        from unittest.mock import patch
        with patch.dict(os.environ, {'GITHUB_ACTIONS': 'false'}):
            with self.assertRaises(ValueError):
                install_validator(Path('target/never-install'), manifest()['validator'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--install-verapdf', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        unittest.main(argv=['fetch-conformance'], exit=True)
    if args.out is None:
        parser.error('--out is required')
    value = manifest()
    for item in value['files']:
        url = 'https://raw.githubusercontent.com/veraPDF/veraPDF-corpus/' + REVISION + '/' + urllib.parse.quote(item['path'], safe='/')
        fetch(url, safe_path(args.out, item['id'] + '.pdf'), item['bytes'], item['sha256'])
    safe_path(args.out, 'ATTRIBUTION.txt').write_text(value['attribution'] + '\n' + value['licenseUrl'] + '\n' + value['licenseSource'] + '\nUnmodified source fixtures; derived files are separately identified in evidence.\n', encoding='utf-8')
    safe_path(args.out, 'provenance.json').write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')
    if args.install_verapdf:
        install_validator(args.install_verapdf, value['validator'])
    print('Verified ' + str(len(value['files'])) + ' pinned CC-BY-4.0 conformance fixtures.')


if __name__ == '__main__':
    main()
