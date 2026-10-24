#!/usr/bin/env python3
"""Check the Apple Silicon bundle's architecture, deployment floor and library closure."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import re
import subprocess
import sys
import unittest


def command(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True, timeout=30).stdout


def version(value):
    if not re.fullmatch(r"\d+\.\d+(?:\.\d+)?", value):
        raise ValueError("Invalid macOS deployment version: " + value)
    parts = tuple(map(int, value.split('.')))
    return parts + (0,) * (3 - len(parts))


def minimum_system(load_commands):
    versions = []
    for block in re.split(r"Load command \d+", load_commands):
        if re.search(r"\bcmd LC_BUILD_VERSION\b", block):
            platform = re.search(r"\bplatform (\S+)", block)
            minimum = re.search(r"\bminos (\S+)", block)
            if not platform or platform[1].lower() not in ('1', 'macos') or not minimum:
                raise ValueError("Binary is not a macOS image")
            versions.append(minimum[1])
        elif re.search(r"\bcmd LC_VERSION_MIN_MACOSX\b", block):
            minimum = re.search(r"\bversion (\S+)", block)
            if not minimum:
                raise ValueError("Missing macOS deployment version")
            versions.append(minimum[1])
    if len(versions) != 1:
        raise ValueError("Expected one macOS deployment command")
    version(versions[0])
    return versions[0]


def dependencies(output, dylib):
    lines = output.splitlines()[1:]
    values = []
    for line in lines:
        match = re.fullmatch(r"\s+(.+?) \(compatibility version .+\)", line)
        if not match:
            raise ValueError("Unexpected dependency record: " + line)
        values.append(match[1])
    # otool -L starts a dynamic library's list with its own install ID.
    return values[1:] if dylib else values


def resolve_dependency(value, binary, app):
    if value.startswith(('/System/Library/', '/usr/lib/')) and '..' not in Path(value).parts:
        return None
    if value.startswith('@loader_path/'):
        path = binary.parent / value[len('@loader_path/'):]
    elif value.startswith('@executable_path/'):
        path = app / 'Contents/MacOS' / value[len('@executable_path/'):]
    else:
        raise ValueError("Non-system dependency is not bundle-relative: " + value)
    resolved = path.resolve(strict=True)
    if not resolved.is_relative_to(app) or path.is_symlink() or not resolved.is_file():
        raise ValueError("Dependency escapes the application: " + value)
    return resolved


def validate(app):
    app = app.resolve(strict=True)
    info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    floor = info['LSMinimumSystemVersion']
    macos = app / 'Contents/MacOS'
    binaries = [macos / name for name in ('sunipdf', 'sunipdf-cli', 'libpdfium.dylib')]
    records = []
    for binary in binaries:
        if binary.is_symlink() or not binary.is_file():
            raise ValueError("Required bundle binary missing: " + binary.name)
        architectures = command('lipo', '-archs', str(binary)).split()
        if architectures != ['arm64']:
            raise ValueError("Expected Apple Silicon only: " + binary.name)
        minimum = minimum_system(command('otool', '-l', str(binary)))
        if version(minimum) > version(floor):
            raise ValueError(f"{binary.name} requires macOS {minimum}, above bundle minimum {floor}")
        libraries = dependencies(command('otool', '-L', str(binary)), binary.suffix == '.dylib')
        for library in libraries:
            linked = resolve_dependency(library, binary, app)
            if linked is not None and linked not in binaries:
                raise ValueError("Unverified additional bundle library: " + library)
        records.append({'path': binary.relative_to(app).as_posix(), 'architectures': architectures,
                        'minimumSystemVersion': minimum, 'dependencies': libraries,
                        'sha256': hashlib.sha256(binary.read_bytes()).hexdigest()})
    return {'schema': 1, 'version': info['CFBundleShortVersionString'],
            'minimumSystemVersion': floor, 'binaries': records}


class BoundaryTests(unittest.TestCase):
    def test_version_and_platform(self):
        self.assertEqual(minimum_system('Load command 8\n cmd LC_BUILD_VERSION\n platform macos\n minos 12.0\n'), '12.0')
        self.assertEqual(minimum_system('Load command 8\n cmd LC_VERSION_MIN_MACOSX\n version 11.0\n'), '11.0')
        for text in ('', 'Load command 1\n cmd LC_BUILD_VERSION\n platform ios\n minos 12.0\n'):
            with self.assertRaises(ValueError):
                minimum_system(text)
        self.assertGreater(version('13.0'), version('12.9.1'))

    def test_library_records(self):
        text = 'libpdfium.dylib:\n\t@rpath/libpdfium.dylib (compatibility version 1.0.0, current version 1.0.0)\n\t/usr/lib/libc++.1.dylib (compatibility version 1.0.0, current version 1.0.0)\n'
        self.assertEqual(dependencies(text, True), ['/usr/lib/libc++.1.dylib'])
        with self.assertRaises(ValueError):
            dependencies('binary:\nunknown dependency', False)

    def test_external_dependencies_rejected(self):
        app = Path('/app/SuniPDF.app')
        for value in ('/opt/homebrew/lib/libfoo.dylib', '@rpath/libfoo.dylib', '/usr/lib/../../tmp/foo'):
            with self.assertRaises(ValueError):
                resolve_dependency(value, app / 'Contents/MacOS/sunipdf', app)
        self.assertIsNone(resolve_dependency('/usr/lib/libSystem.B.dylib', app / 'Contents/MacOS/sunipdf', app))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path)
    parser.add_argument('--report', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(BoundaryTests))
        sys.exit(0 if result.wasSuccessful() else 1)
    if args.app is None or args.report is None:
        parser.error('--app and --report are required')
    proof = validate(args.app)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(proof, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(proof, indent=2))
