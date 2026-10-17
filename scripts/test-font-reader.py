"""Compare subset PDFs in pinned MuPDF with original-font PDFium raster baselines."""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

MUTOOL_SHA256 = '1ff045eeab80a108a39760578001a547a31e8b663905f064088c4b2f82cbcf7f'
CASES = ('truetype-unicode-subset', 'cff-otf-subset', 'ttc-second-cff-face', 'cff-cjk-subset')


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ppm(path):
    data = path.read_bytes()
    if len(data) > 128 * 1024 * 1024:
        raise ValueError('Raster evidence exceeds the size bound')
    tokens = []
    position = 0
    while len(tokens) < 4:
        while position < len(data) and data[position] in b' \r\n\t':
            position += 1
        if data[position:position + 1] == b'#':
            position = data.index(b'\n', position) + 1
            continue
        end = position
        while end < len(data) and data[end] not in b' \r\n\t':
            end += 1
        tokens.append(data[position:end])
        position = end
    if tokens[0] != b'P6' or tokens[3] != b'255':
        raise ValueError('Expected an 8-bit RGB PPM raster')
    width, height = int(tokens[1]), int(tokens[2])
    if width < 1 or height < 1 or width * height > 32_000_000:
        raise ValueError('Invalid raster geometry')
    if data[position:position + 2] == b'\r\n':
        position += 2
    elif data[position:position + 1] in (b' ', b'\n', b'\r', b'\t'):
        position += 1
    else:
        raise ValueError('Missing PPM data delimiter')
    pixels = data[position:]
    if len(pixels) != width * height * 3:
        raise ValueError('Truncated or oversized raster payload')
    return width, height, pixels


def mask(image, region):
    width, height, pixels = image
    if len(region) != 4 or not all(math.isfinite(value) for value in region):
        raise ValueError('Invalid glyph evidence region')
    left, top = max(0, math.floor(region[0]) - 3), max(0, math.floor(region[1]) - 3)
    right, bottom = min(width, math.ceil(region[2]) + 3), min(height, math.ceil(region[3]) + 3)
    if not (0 <= left < right <= width and 0 <= top < bottom <= height):
        raise ValueError('Invalid glyph evidence region')
    return {(x, y) for y in range(top, bottom) for x in range(left, right)
            if min(pixels[(y * width + x) * 3:(y * width + x) * 3 + 3]) < 128}


def compare_masks(expected, actual):
    if min(len(expected), len(actual)) < 8:
        raise ValueError('Glyph raster is empty or too small for comparison')
    box = lambda points: [min(x for x, _ in points), min(y for _, y in points), max(x for x, _ in points), max(y for _, y in points)]
    dilate = lambda points: {(x + dx, y + dy) for x, y in points for dx in (-1, 0, 1) for dy in (-1, 0, 1)}
    baseline_box, actual_box = box(expected), box(actual)
    delta = max(abs(a - b) for a, b in zip(baseline_box, actual_box))
    coverage = len(expected & dilate(actual)) / len(expected)
    reverse = len(actual & dilate(expected)) / len(actual)
    ratio = len(actual) / len(expected)
    iou = len(expected & actual) / len(expected | actual)
    if delta > 3 or min(coverage, reverse) < .85 or not .65 <= ratio <= 1.55:
        raise ValueError(f'Glyph geometry/baseline differs: delta={delta}, coverage={coverage:.3f}/{reverse:.3f}, ink ratio={ratio:.3f}')
    return {'baselineBox': baseline_box, 'readerBox': actual_box, 'maxBoxDeltaPixels': delta,
            'baselineInkPixels': len(expected), 'readerInkPixels': len(actual),
            'coverageWithinOnePixel': round(coverage, 4), 'reverseCoverageWithinOnePixel': round(reverse, 4), 'rawMaskIoU': round(iou, 4)}


def check_text(actual, expected):
    required = Counter(''.join(expected))
    present = Counter(actual)
    missing = {char: count - present[char] for char, count in required.items() if not char.isspace() and present[char] < count}
    if missing:
        raise ValueError('Independent reader lost requested Unicode characters: ' + repr(missing))


def run(mutool, arguments):
    result = subprocess.run([str(mutool), *arguments], capture_output=True, timeout=60)
    diagnostic = result.stderr.decode('utf-8', errors='replace')
    if result.returncode:
        raise ValueError('Independent reader failed: ' + diagnostic[-2000:])
    return result.stdout.decode('utf-8', errors='replace'), diagnostic


def validate(mutool, directory):
    if sha256(mutool) != MUTOOL_SHA256:
        raise ValueError('Independent reader does not match the pinned official binary')
    stdout, stderr = run(mutool, ['-v'])
    version = (stdout + stderr).strip()
    if '1.28.5' not in version:
        raise ValueError('Independent reader version differs')
    report = {'schema': 1, 'status': 'running', 'reader': version, 'readerSha256': MUTOOL_SHA256,
              'scope': 'Independent extraction and glyph raster comparison against original full-font PDFium baselines; four bounded fixture cases', 'cases': []}
    output = directory / 'mupdf'
    output.mkdir(exist_ok=True)
    for name in CASES:
        evidence = json.loads((directory / (name + '.json')).read_text(encoding='utf-8'))
        if evidence.get('schema') != 1 or evidence.get('dpi') != 144 or evidence.get('page') != 1:
            raise ValueError('Unexpected font baseline evidence format')
        pdf = directory / evidence['pdf']
        baseline = directory / evidence['baseline']
        if pdf.parent.resolve() != directory.resolve() or baseline.parent.resolve() != directory.resolve():
            raise ValueError('Font baseline resource escapes its evidence directory')
        text_path, raster_path = output / (name + '.txt'), output / (name + '.ppm')
        _, text_diagnostic = run(mutool, ['draw', '-F', 'txt', '-o', str(text_path), str(pdf), '1'])
        check_text(text_path.read_text(encoding='utf-8-sig'), evidence['expectedText'])
        _, raster_diagnostic = run(mutool, ['draw', '-F', 'ppm', '-c', 'rgb', '-r', '144', '-o', str(raster_path), str(pdf), '1'])
        expected, actual = ppm(baseline), ppm(raster_path)
        if expected[:2] != actual[:2] or expected[:2] != (evidence['width'], evidence['height']):
            raise ValueError('Independent reader page geometry differs')
        if len(evidence['regions']) != 2:
            raise ValueError('Expected both newly added text regions')
        regions = []
        for index, region in enumerate(evidence['regions']):
            expected_mask, actual_mask = mask(expected, region), mask(actual, region)
            try:
                regions.append(compare_masks(expected_mask, actual_mask))
            except ValueError as error:
                raise ValueError(f'{name}, region {index}: {error}; '
                                 f'baseline ink={len(expected_mask)}, reader ink={len(actual_mask)}') from error
        report['cases'].append({'name': name, 'pdfSha256': sha256(pdf), 'baselineSha256': sha256(baseline),
                                'readerRasterSha256': sha256(raster_path), 'pagePixels': list(actual[:2]),
                                'unicodeTextPassed': True, 'regions': regions,
                                'readerDiagnostics': (text_diagnostic + raster_diagnostic).strip()})
    report['status'] = 'passed'
    return report


class ReaderTests(unittest.TestCase):
    def test_unicode_counts_require_both_added_objects(self):
        check_text('\u65e5\u672c\u65e5\u672c', ['\u65e5\u672c', '\u65e5\u672c'])
        with self.assertRaises(ValueError):
            check_text('\u65e5\u672c', ['\u65e5\u672c', '\u65e5\u672c'])

    def test_raster_comparison_rejects_missing_or_shifted_glyphs(self):
        original = {(x, y) for x in range(10, 20) for y in range(10, 20)}
        compare_masks(original, original)
        compare_masks(original, {(x + 1, y) for x, y in original})
        for changed in (set(), {(x + 10, y) for x, y in original}, {(10, y) for y in range(10, 20)}):
            with self.assertRaises(ValueError):
                compare_masks(original, changed)

    def test_ppm_geometry_and_truncation(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'raster.ppm'
            path.write_bytes(b'P6\n2 1\n255\n' + b'\xff' * 6)
            self.assertEqual(ppm(path)[:2], (2, 1))
            path.write_bytes(b'P6\n2 1\n255\n' + b'\xff' * 5)
            with self.assertRaises(ValueError):
                ppm(path)

    def test_non_finite_or_empty_regions_are_refused(self):
        image = (4, 4, b'\xff' * 48)
        for region in ([0, 0, float('inf'), 2], [0, float('nan'), 2, 2], [], [20, 20, 30, 30]):
            with self.assertRaises(ValueError):
                mask(image, region)


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mutool', type=Path, default=root / 'target/test-mupdf/1.28.5/mutool.exe')
    parser.add_argument('--directory', type=Path, default=root / 'target/font-embedding-evidence')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        return 0 if unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(ReaderTests)).wasSuccessful() else 1
    destination = root / 'target/font-independent-reader-report.json'
    try:
        report = validate(args.mutool, args.directory)
    except (ValueError, OSError, KeyError, subprocess.TimeoutExpired) as error:
        destination.write_text(json.dumps({'schema': 1, 'status': 'failed', 'error': str(error)}, indent=2) + '\n', encoding='utf-8')
        raise
    destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    print('Independent font reader PASS: ' + str(len(report['cases'])) + ' cases; ' + str(destination))
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, subprocess.TimeoutExpired) as error:
        print('Independent font reader failed: ' + str(error), file=sys.stderr)
        sys.exit(1)
