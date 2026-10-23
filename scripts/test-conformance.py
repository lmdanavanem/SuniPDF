"""Validate original and rewritten PDFs with independent conformance and reader tools."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import xml.etree.ElementTree as ET

spec = importlib.util.spec_from_file_location('conformance_fetch', Path(__file__).with_name('fetch-conformance.py'))
fetcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fetcher)
PROFILES = {'1b': 'PDF/A-1b validation profile', '2b': 'PDF/A-2b validation profile', 'ua1': 'PDF/UA-1 validation profile'}
MAX_OUTPUT = 8 * 1024 * 1024


def run(command, stdout_path, timeout=120, allowed=(0,)):
    """Bound runtime and both output streams without retaining unbounded pipes."""
    error_path = stdout_path.with_suffix(stdout_path.suffix + '.stderr')
    with stdout_path.open('xb') as output, error_path.open('xb') as errors:
        process = subprocess.Popen([str(value) for value in command], stdout=output, stderr=errors,
                                   start_new_session=os.name == 'posix')
        started = time.monotonic()
        try:
            while process.poll() is None:
                if time.monotonic() - started > timeout or max(stdout_path.stat().st_size, error_path.stat().st_size) > MAX_OUTPUT:
                    raise ValueError('External tool exceeded runtime or output bound')
                time.sleep(.05)
            if max(stdout_path.stat().st_size, error_path.stat().st_size) > MAX_OUTPUT:
                raise ValueError('External tool exceeded output bound')
            if process.returncode not in allowed:
                raise ValueError('External tool failed (' + str(process.returncode) + '): ' + error_path.read_text(encoding='utf-8', errors='replace')[-2000:])
        finally:
            if process.poll() is None:
                if os.name == 'posix':
                    os.killpg(process.pid, signal.SIGKILL)
                else:
                    process.kill()
                process.wait(timeout=10)
    return process.returncode


def parse_report(data, pdf, profile, expected, rule=None):
    if not data or len(data) > MAX_OUTPUT or b'<!DOCTYPE' in data.upper() or b'<!ENTITY' in data.upper():
        raise ValueError('Invalid or unsafe validator report')
    root = ET.fromstring(data)
    if root.tag != 'report':
        raise ValueError('Unexpected validator report schema')
    versions = root.findall('./buildInformation/releaseDetails')
    if not versions or not any(x.get('version') == '1.30.3' for x in versions):
        raise ValueError('Missing pinned validator build information')
    jobs = root.findall('./jobs/job')
    summaries = root.findall('./batchSummary')
    if len(jobs) != 1 or len(summaries) != 1:
        raise ValueError('Expected exactly one complete validation job')
    job, summary = jobs[0], summaries[0]
    item = job.find('item')
    if item is None or item.findtext('name') is None or Path(item.findtext('name')).resolve() != pdf.resolve() or item.get('size') != str(pdf.stat().st_size):
        raise ValueError('Validator report does not identify the tested file')
    if summary.get('totalJobs') != '1' or any(summary.get(k) != '0' for k in ('failedToParse', 'encrypted', 'outOfMemory', 'veraExceptions')):
        raise ValueError('Validator processing did not complete cleanly')
    reports = job.findall('validationReport')
    totals = summary.find('validationReports')
    if len(reports) != 1 or totals is None or totals.get('failedJobs') != '0' or (totals.text or '').strip() != '1':
        raise ValueError('Missing or failed validation report')
    report = reports[0]
    if report.get('profileName') != PROFILES[profile] or report.get('isCompliant') not in ('true', 'false'):
        raise ValueError('Unexpected validator profile or missing conformance outcome')
    compliant = report.get('isCompliant') == 'true'
    if totals.get('compliant') != str(int(compliant)) or totals.get('nonCompliant') != str(int(not compliant)):
        raise ValueError('Inconsistent validator totals')
    details = report.find('details')
    if details is None or any(not details.get(k, '').isdigit() for k in ('passedRules', 'failedRules', 'passedChecks', 'failedChecks')):
        raise ValueError('Incomplete validator rule counts')
    if int(details.get('passedRules')) < 1 or (int(details.get('failedRules')) == 0) != compliant or (int(details.get('failedChecks')) == 0) != compliant:
        raise ValueError('Inconsistent validator rule outcomes')
    failures = [{'specification': x.get('specification'), 'clause': x.get('clause'), 'testNumber': x.get('testNumber')}
                for x in details.findall('rule') if x.get('status') == 'failed']
    result = {'profile': profile, 'compliant': compliant, 'passedRules': int(details.get('passedRules')),
              'failedRules': int(details.get('failedRules')), 'failedChecks': int(details.get('failedChecks')), 'failures': failures}
    if compliant != expected:
        raise ValueError('Unexpected conformance result: ' + json.dumps(result, sort_keys=True))
    if rule and not compliant and not any(all(f.get(key) == value for key, value in rule.items()) for f in failures):
        raise ValueError('Negative fixture did not fail its declared rule: ' + json.dumps(result, sort_keys=True))
    return result


def image(path):
    from PIL import Image
    if not path.is_file() or path.stat().st_size > 32 * 1024 * 1024:
        raise ValueError('Missing or oversized reader raster')
    with Image.open(path) as source:
        if source.width * source.height > 4_000_000 or min(source.size) < 1:
            raise ValueError('Reader raster exceeds geometry bounds')
        return source.convert('RGB')


def appearance(first, second, allow_rounding=False, threshold=160):
    from PIL import Image, ImageChops, ImageFilter, ImageStat
    original_sizes = [list(first.size), list(second.size)]
    if first.size != second.size:
        if not allow_rounding or any(abs(a - b) > 1 for a, b in zip(first.size, second.size)):
            raise ValueError('Independent page geometry differs')
        size = tuple(max(a, b) for a, b in zip(first.size, second.size))
        def padded(source):
            result = Image.new('RGB', size, 'white')
            result.paste(source, (0, 0))
            return result
        first, second = padded(first), padded(second)
    left = first.convert('L').point(lambda x: 255 if x < threshold else 0)
    right = second.convert('L').point(lambda x: 255 if x < threshold else 0)
    left_count = left.histogram()[255]
    right_count = right.histogram()[255]
    if min(left_count, right_count) < 8:
        raise ValueError('Reader raster has no meaningful ink')
    left_covered = ImageChops.multiply(left, right.filter(ImageFilter.MaxFilter(5))).histogram()[255] / left_count
    right_covered = ImageChops.multiply(right, left.filter(ImageFilter.MaxFilter(5))).histogram()[255] / right_count
    mae = sum(ImageStat.Stat(ImageChops.difference(first, second)).mean) / 3
    ratio = right_count / left_count
    if min(left_covered, right_covered) < .70 or not .50 <= ratio <= 2 or mae > 12:
        raise ValueError(f'Independent appearance differs: coverage {left_covered:.4f}/{right_covered:.4f}, ink ratio {ratio:.4f}, RGB MAE {mae:.4f}')
    return {'coverageWithinTwoPixels': round(left_covered, 6), 'reverseCoverageWithinTwoPixels': round(right_covered, 6), 'inkRatio': round(ratio, 6), 'rgbMeanAbsoluteError': round(mae, 6), 'rasterSizes': original_sizes, 'canvasRoundingPadded': original_sizes[0] != original_sizes[1], 'inkThreshold': threshold}


def qualify(args):
    value = fetcher.manifest()
    output = args.out.absolute()
    fetcher.safe_path(output.parent, output.name)
    output.mkdir(parents=True, exist_ok=False)
    report = {'schema': 1, 'status': 'running', 'corpusRevision': value['revision'], 'attribution': value['attribution'],
              'license': value['license'], 'validator': value['validator'], 'tools': {}, 'cases': [], 'errors': [],
              'scope': 'Machine validation of the specified profiles and first-page appearance; human PDF/UA semantics remain unqualified.'}
    report_path = output / 'report.json'

    def save():
        report_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')

    try:
        import PIL
        report['tools']['python'] = {'version': sys.version}
        report['tools']['pillow'] = {'version': PIL.__version__}
        for name, executable, arguments in [('verapdf', args.verapdf, ['--version']), ('qpdf', args.qpdf, ['--version']), ('poppler', args.pdftoppm, ['-v']), ('sunipdf', args.cli, ['--version']), ('java', 'java', ['-version'])]:
            executable = Path(shutil.which(str(executable)) or executable).resolve()
            if not executable.is_file():
                raise ValueError('Required executable is missing: ' + name)
            log = output / (name + '-version.txt')
            run([executable, *arguments], log)
            version = log.read_text(encoding='utf-8', errors='replace') + log.with_suffix('.txt.stderr').read_text(encoding='utf-8', errors='replace')
            if name == 'verapdf' and '1.30.3' not in version:
                raise ValueError('Unexpected veraPDF version')
            report['tools'][name] = {'version': version.strip(), 'sha256': fetcher.sha256(executable)}
        save()
        for fixture in value['files']:
            case = {'id': fixture['id'], 'profile': fixture['profile'], 'expectedCompliant': fixture['expectedCompliant'], 'stages': [], 'errors': []}
            report['cases'].append(case)
            folder = output / fixture['id']
            folder.mkdir()
            source = fetcher.safe_path(args.corpus, fixture['id'] + '.pdf')
            try:
                if source.stat().st_size != fixture['bytes'] or fetcher.sha256(source) != fixture['sha256']:
                    raise ValueError('Corpus source differs from pinned content')
                original_hash = fetcher.sha256(source)
                case['sourceSha256'] = original_hash
                stages = [('source', source, None, fixture['expectedCompliant'])]
                if fixture['expectedCompliant']:
                    stages += [('optimized', folder / 'optimized.pdf', ['optimize', str(source)], True),
                               ('metadata', folder / 'metadata.pdf', ['set-metadata', str(source), '--title', 'SuniPDF conformance preservation'], True),
                               ('rotated', folder / 'rotated.pdf', ['rotate', str(source), '--degrees', '90'], True),
                               ('rotation-roundtrip', folder / 'rotation-roundtrip.pdf', ['rotate', str(folder / 'rotated.pdf'), '--degrees', '270'], True),
                               ('stripped', folder / 'stripped.pdf', ['strip-metadata', str(source)], False)]
                baseline = None
                for name, pdf, command, expected in stages:
                    stage = {'name': name, 'expectedCompliant': expected, 'errors': []}
                    case['stages'].append(stage)
                    try:
                        if command:
                            run([args.cli, *command, '--out', pdf], folder / (name + '-command.txt'))
                        if not pdf.is_file() or pdf.stat().st_size > 16 * 1024 * 1024:
                            raise ValueError('Missing or oversized rewritten PDF')
                        stage['sha256'] = fetcher.sha256(pdf)
                        stage['bytesChanged'] = stage['sha256'] != original_hash
                        if command and name != 'optimized' and not stage['bytesChanged']:
                            raise ValueError('Expected a real document rewrite')
                        checks = [
                            ('conformance', lambda: validate(pdf, fixture, expected, folder / (name + '-verapdf.xml'), args.verapdf, fixture['rule'] if name == 'source' and not expected else None)),
                            ('qpdf', lambda: syntax(pdf, folder / (name + '-qpdf.txt'), args.qpdf, fixture.get('expectedQpdfWarnings', []) if name == 'source' and not expected else [])),
                        ]
                        for check, action in checks:
                            try:
                                stage[check] = action()
                            except (ValueError, OSError, ET.ParseError) as error:
                                stage['errors'].append(check + ': ' + str(error))
                        try:
                            # Rotation has its own geometry; roundtrip compares the restored page.
                            raster = folder / (name + '-pdfium.png')
                            independent = folder / (name + '-poppler')
                            run([args.cli, 'render', pdf, '--page', '1', '--scale', '1', '--out', raster], folder / (name + '-pdfium.txt'))
                            run([args.pdftoppm, '-f', '1', '-singlefile', '-cropbox', '-r', '72', '-png', pdf, independent], folder / (name + '-poppler.txt'))
                            native, reader = image(raster), image(independent.with_suffix('.png'))
                            stage['appearance'] = appearance(native, reader, allow_rounding=True, threshold=fixture.get('rasterThreshold', 160))
                            if name == 'source':
                                baseline = reader
                            elif name != 'rotated' and baseline is not None:
                                stage['preservedAppearance'] = appearance(baseline, reader)
                            stage['pdfiumRasterSha256'] = fetcher.sha256(raster)
                            stage['popplerRasterSha256'] = fetcher.sha256(independent.with_suffix('.png'))
                        except (ValueError, OSError) as error:
                            stage['errors'].append('appearance: ' + str(error))
                    except (ValueError, OSError) as error:
                        stage['errors'].append(str(error))
                    stage['status'] = 'failed' if stage['errors'] else 'passed'
                    save()
                case['originalUnchanged'] = fetcher.sha256(source) == original_hash
                if not case['originalUnchanged']:
                    case['errors'].append('Original corpus file changed')
            except (ValueError, OSError) as error:
                case['errors'].append(str(error))
            case['status'] = 'failed' if case['errors'] or any(s['errors'] for s in case['stages']) else 'passed'
            save()
    except (ValueError, OSError, ImportError) as error:
        report['errors'].append(str(error))
    report['status'] = 'passed' if not report['errors'] and len(report['cases']) == len(value['files']) and all(c['status'] == 'passed' for c in report['cases']) else 'failed'
    save()
    return report


def validate(pdf, fixture, expected, path, executable, rule):
    run([executable, '--format', 'xml', '--flavour', fixture['profile'], '--maxfailuresdisplayed', '1', pdf], path, allowed=(0, 1))
    return parse_report(path.read_bytes(), pdf, fixture['profile'], expected, rule)


def syntax(pdf, path, executable, expected_warnings=None):
    code = run([executable, '--check', pdf], path, allowed=(0, 3))
    diagnostic = path.with_suffix(path.suffix + '.stderr').read_text(encoding='utf-8', errors='replace')
    return qpdf_result(code, diagnostic, pdf, expected_warnings or [])


def qpdf_result(code, diagnostic, pdf, expected_warnings):
    if code == 0:
        if diagnostic.strip():
            raise ValueError('Unexpected QPDF diagnostic: ' + diagnostic)
        return {'passed': True}
    warnings = []
    for line in diagnostic.splitlines():
        if line == 'qpdf: operation succeeded with warnings':
            continue
        if not line.startswith('WARNING: '):
            raise ValueError('Unexpected QPDF diagnostic: ' + line)
        message = line[len('WARNING: '):]
        if message.startswith(str(pdf) + ': '):
            message = message[len(str(pdf)) + 2:]
        warnings.append(message)
    if code != 3 or not expected_warnings or warnings != expected_warnings:
        raise ValueError('Unexpected QPDF warnings: ' + json.dumps(warnings))
    return {'passed': False, 'acceptedNegativeFixtureWarnings': True, 'warnings': warnings}


class ReportTests(unittest.TestCase):
    def test_blank_displaced_and_wrong_geometry_rasters_fail(self):
        from PIL import Image, ImageDraw
        first = Image.new('RGB', (80, 80), 'white')
        ImageDraw.Draw(first).rectangle((10, 10, 25, 30), fill='black')
        self.assertEqual(appearance(first, first)['coverageWithinTwoPixels'], 1)
        shifted = Image.new('RGB', first.size, 'white')
        ImageDraw.Draw(shifted).rectangle((50, 50, 65, 70), fill='black')
        for changed in (Image.new('RGB', first.size, 'white'), shifted, Image.new('RGB', (90, 80), 'black')):
            with self.assertRaises(ValueError):
                appearance(first, changed)
        rounded = Image.new('RGB', (81, 80), 'white')
        rounded.paste(first, (0, 0))
        self.assertTrue(appearance(first, rounded, allow_rounding=True)['canvasRoundingPadded'])
        with self.assertRaises(ValueError):
            appearance(first, rounded)

    def test_only_exact_declared_negative_qpdf_warnings_are_accepted(self):
        pdf = Path('fixture.pdf')
        diagnostic = 'WARNING: ' + str(pdf) + ': known negative fixture defect\nqpdf: operation succeeded with warnings\n'
        result = qpdf_result(3, diagnostic, pdf, ['known negative fixture defect'])
        self.assertFalse(result['passed'])
        self.assertTrue(result['acceptedNegativeFixtureWarnings'])
        for expected in ([], ['another warning']):
            with self.assertRaises(ValueError):
                qpdf_result(3, diagnostic, pdf, expected)
        with self.assertRaises(ValueError):
            qpdf_result(3, diagnostic + 'WARNING: new defect\n', pdf, ['known negative fixture defect'])

    def test_process_timeout_and_output_bounds(self):
        with tempfile.TemporaryDirectory() as folder:
            directory = Path(folder)
            with self.assertRaisesRegex(ValueError, 'runtime or output'):
                run([sys.executable, '-c', 'import time; time.sleep(10)'], directory / 'timeout.txt', timeout=.10)
            with self.assertRaisesRegex(ValueError, 'output bound'):
                run([sys.executable, '-c', 'import sys; sys.stdout.buffer.write(b"x" * (9 * 1024 * 1024))'], directory / 'oversized.txt')

    def test_failed_exit_cannot_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaisesRegex(ValueError, 'External tool failed'):
                run([sys.executable, '-c', 'raise SystemExit(7)'], Path(folder) / 'failed.txt')

    def report(self, pdf, compliant=True):
        from xml.sax.saxutils import escape
        flag = str(compliant).lower()
        failed = str(int(not compliant))
        rule = '' if compliant else '<rule status="failed" clause="7.5" testNumber="1"/>'
        return ('<report><buildInformation><releaseDetails version="1.30.3"/></buildInformation><jobs><job><item size="3"><name>' + escape(str(pdf)) + '</name></item><validationReport profileName="PDF/UA-1 validation profile" isCompliant="' + flag + '"><details passedRules="10" passedChecks="20" failedRules="' + failed + '" failedChecks="' + failed + '">' + rule + '</details></validationReport></job></jobs><batchSummary totalJobs="1" failedToParse="0" encrypted="0" outOfMemory="0" veraExceptions="0"><validationReports compliant="' + str(int(compliant)) + '" nonCompliant="' + failed + '" failedJobs="0">1</validationReports></batchSummary></report>').encode()

    def test_complete_positive_and_expected_negative(self):
        with tempfile.TemporaryDirectory() as folder:
            pdf = Path(folder) / 'test.pdf'
            pdf.write_bytes(b'pdf')
            self.assertTrue(parse_report(self.report(pdf), pdf, 'ua1', True)['compliant'])
            self.assertFalse(parse_report(self.report(pdf, False), pdf, 'ua1', False, {'clause': '7.5', 'testNumber': '1'})['compliant'])

    def test_wrong_profile_file_version_incomplete_or_failed_processing_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            pdf = Path(folder) / 'test.pdf'
            pdf.write_bytes(b'pdf')
            data = self.report(pdf)
            for old, new in [(b'PDF/UA-1', b'PDF/A-1b'), (b'test.pdf', b'other.pdf'), (b'1.30.3', b'1.20.0'), (b'totalJobs="1"', b'totalJobs="2"'), (b'failedToParse="0"', b'failedToParse="1"'), (b'failedJobs="0"', b'failedJobs="1"'), (b'isCompliant="true"', b'isCompliant="false"'), (b'passedRules="10"', b'passedRules="0"')]:
                with self.subTest(old=old), self.assertRaises(ValueError):
                    parse_report(data.replace(old, new), pdf, 'ua1', True)
            with self.assertRaises(ValueError):
                parse_report(b'<!DOCTYPE report>' + data, pdf, 'ua1', True)
            with self.assertRaises(ValueError):
                parse_report(self.report(pdf, False), pdf, 'ua1', False, {'clause': '7.2'})

    def test_negative_cannot_silently_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            pdf = Path(folder) / 'test.pdf'
            pdf.write_bytes(b'pdf')
            with self.assertRaises(ValueError):
                parse_report(self.report(pdf), pdf, 'ua1', False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('cli', 'corpus', 'verapdf', 'qpdf', 'pdftoppm', 'out'):
        parser.add_argument('--' + name, type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        unittest.main(argv=['test-conformance'], exit=True)
    if any(getattr(args, name) is None for name in ('cli', 'corpus', 'verapdf', 'qpdf', 'pdftoppm', 'out')):
        parser.error('All tool, corpus and output paths are required')
    result = qualify(args)
    print(json.dumps({'status': result['status'], 'cases': len(result['cases']), 'report': str(args.out / 'report.json')}))
    raise SystemExit(0 if result['status'] == 'passed' else 1)


if __name__ == '__main__':
    main()
