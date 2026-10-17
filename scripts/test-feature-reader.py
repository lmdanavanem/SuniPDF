"""Check synthetic 0.30 PDF structures and appearances with pinned independent readers."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('font_reader', Path(__file__).with_name('test-font-reader.py'))
font_reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(font_reader)


def contained(root, value):
    path = (root / value).resolve()
    if root.resolve() not in path.parents or not path.is_file():
        raise ValueError('Evidence resource is missing or escapes its directory')
    if path.stat().st_size > 128 * 1024 * 1024:
        raise ValueError('Evidence resource exceeds its size bound')
    return path


def ink(image):
    width, height, pixels = image
    points = {(index % width, index // width) for index in range(width * height)
              if min(pixels[index * 3:index * 3 + 3]) < 128}
    if not 8 <= len(points) <= 500_000:
        raise ValueError('Raster ink is absent or exceeds the comparison bound')
    return points


def appearance(expected, actual):
    if expected[:2] != actual[:2]:
        raise ValueError('Independent page geometry differs')
    baseline, reader = ink(expected), ink(actual)
    def coverage(source, target):
        return sum(any((x + dx, y + dy) in target
                       for dx in range(-2, 3) for dy in range(-2, 3))
                   for x, y in source) / len(source)
    forward, reverse = coverage(baseline, reader), coverage(reader, baseline)
    mae = sum(abs(a - b) for a, b in zip(expected[2], actual[2])) / len(expected[2])
    ratio = len(reader) / len(baseline)
    if min(forward, reverse) < .70 or not .50 <= ratio <= 2.0 or mae > 12:
        raise ValueError(f'Independent appearance differs: ink coverage={forward:.4f}/{reverse:.4f}, ratio={ratio:.4f}, RGB MAE={mae:.4f}')
    return {'baselineInkPixels': len(baseline), 'readerInkPixels': len(reader),
            'coverageWithinTwoPixels': round(forward, 4),
            'reverseCoverageWithinTwoPixels': round(reverse, 4), 'rgbMeanAbsoluteError': round(mae, 4)}


def run(executable, arguments):
    result = subprocess.run([str(executable), *arguments], capture_output=True, timeout=60)
    diagnostic = result.stderr.decode('utf-8', errors='replace')
    if result.returncode != 0:
        raise ValueError('Independent PDF check failed: ' + diagnostic[-2000:] + result.stdout.decode('utf-8', errors='replace')[-2000:])
    return result.stdout.decode('utf-8', errors='replace'), diagnostic


def validate(root, directory, mutool, qpdf):
    if font_reader.sha256(mutool) != font_reader.MUTOOL_SHA256:
        raise ValueError('MuPDF binary differs from the pinned official reader')
    version, diagnostic = run(mutool, ['-v'])
    if '1.28.5' not in version + diagnostic:
        raise ValueError('Unexpected MuPDF version')
    qversion, _ = run(qpdf, ['--version'])
    report = {'schema': 1, 'status': 'running', 'reader': (version + diagnostic).strip(),
              'readerSha256': font_reader.MUTOOL_SHA256, 'qpdf': qversion.strip(),
              'scope': 'Synthetic rich-annotation and document-structure output; strict QPDF syntax and independent MuPDF page appearance, with documented raster tolerances', 'cases': []}
    seen = set()
    for group in ('rich', 'structure'):
        manifest = json.loads(contained(directory, group + '/manifest.json').read_text(encoding='utf-8-sig'))
        cases = manifest.get('cases', [])
        if manifest.get('schema') != 1 or not 1 <= len(cases) <= 12:
            raise ValueError('Invalid feature evidence manifest')
        for case in cases:
            name = case['name']
            if not isinstance(name, str) or not name or (group, name) in seen:
                raise ValueError('Invalid or duplicate feature case')
            seen.add((group, name))
            if case.get('page') != 1 or case.get('scale') != 1 or case.get('readable') is not True:
                raise ValueError('Unexpected feature raster contract')
            pdf = contained(directory, os.path.relpath(root / case['pdf'], directory))
            baseline = contained(directory, os.path.relpath(root / case['baseline_ppm'], directory))
            password = case.get('password')
            qargs = ([] if password is None else ['--password=' + password]) + ['--check', str(pdf)]
            _, qdiagnostic = run(qpdf, qargs)
            raster = pdf.with_suffix('.mupdf.ppm')
            arguments = ['draw', '-F', 'ppm', '-c', 'rgb', '-r', '72', '-o', str(raster)]
            if password is not None:
                arguments += ['-p', password]
            _, reader_diagnostic = run(mutool, [*arguments, str(pdf), '1'])
            metrics = appearance(font_reader.ppm(baseline), font_reader.ppm(raster))
            report['cases'].append({'group': group, 'name': name, 'pdfSha256': font_reader.sha256(pdf),
                                    'baselineSha256': font_reader.sha256(baseline),
                                    'readerRasterSha256': font_reader.sha256(raster), 'qpdfPassed': True,
                                    'appearance': metrics, 'diagnostics': (qdiagnostic + reader_diagnostic).strip()})
    report['status'] = 'passed'
    return report


class EvidenceTests(unittest.TestCase):
    def test_white_missing_and_displaced_appearances_fail(self):
        def raster(points):
            data = bytearray(b'\xff' * (30 * 30 * 3))
            for x, y in points:
                data[(y * 30 + x) * 3:(y * 30 + x) * 3 + 3] = b'\0\0\0'
            return 30, 30, bytes(data)
        points = {(x, y) for x in range(3, 8) for y in range(3, 8)}
        appearance(raster(points), raster(points))
        for changed in (set(), {(x + 15, y) for x, y in points}, {(3, 3)}):
            with self.assertRaises(ValueError):
                appearance(raster(points), raster(changed))

    def test_evidence_paths_cannot_escape(self):
        with tempfile.TemporaryDirectory() as value:
            root = Path(value)
            (root / 'group').mkdir()
            (root / 'outside.pdf').write_bytes(b'fixture')
            with self.assertRaises(ValueError):
                contained(root / 'group', '../outside.pdf')
            with self.assertRaises(ValueError):
                contained(root, 'missing.pdf')


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mutool', type=Path, default=root / 'target/test-mupdf/1.28.5/mutool.exe')
    parser.add_argument('--qpdf', type=Path, default=os.environ.get('SUNIPDF_QPDF'))
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        return 0 if unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(EvidenceTests)).wasSuccessful() else 1
    directory = root / 'target/feature-030-evidence'
    destination = root / 'target/feature-independent-reader-report.json'
    try:
        if args.qpdf is None:
            raise ValueError('The pinned QPDF checker is unavailable')
        report = validate(root, directory, args.mutool, args.qpdf)
    except (ValueError, OSError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        destination.write_text(json.dumps({'schema': 1, 'status': 'failed', 'error': str(error)}, indent=2) + '\n', encoding='utf-8')
        raise
    destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    print('Independent feature reader PASS: ' + str(len(report['cases'])) + ' cases')
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        print('Independent feature check failed: ' + str(error), file=sys.stderr)
        sys.exit(1)
