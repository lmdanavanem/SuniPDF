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
    if report.get('jobEndStatus') != 'normal':
        raise ValueError('Validator job did not finish normally')
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


# --- Feature re-reads (issue #106): SuniPDF writes, independent readers assert concrete facts. ---
TITLE, AUTHOR, SUBJECT, KEYWORDS = 'Relecture indépendante 106', 'SuniPDF QA', 'Lecteurs tiers', 'qpdf; poppler; openssl'
USER_PASSWORD, OWNER_PASSWORD = 'lecture-106', 'proprietaire-106'
FIELD_VALUE = 'Durand QA 106'
FREE_TEXT, REPLY = 'Texte libre 106', 'Reponse 106'
URL = 'https://example.org/sunipdf-106'
# kind, CLI arguments, expected subtype, displayed rectangle, rectangle tolerance (stroke and arrow heads).
ANNOTATIONS = [
    ('highlight', ['--rect', '72,695,260,722'], '/Highlight', (72, 695, 260, 722), 1),
    ('underline', ['--rect', '72,660,200,680'], '/Underline', (72, 660, 200, 680), 1),
    ('strikeout', ['--rect', '300,660,420,680'], '/StrikeOut', (300, 660, 420, 680), 1),
    ('ink', ['--stroke', '320,500 360,560 400,520'], '/Ink', (320, 500, 400, 560), 12),
    ('freetext', ['--rect', '320,600,560,640', '--text', FREE_TEXT, '--size', '14'], '/FreeText', (320, 600, 560, 640), 1),
    ('square', ['--rect', '80,380,200,460'], '/Square', (80, 380, 200, 460), 3),
    ('circle', ['--rect', '220,380,340,460'], '/Circle', (220, 380, 340, 460), 3),
    ('line', ['--rect', '360,380,480,460'], '/Line', (360, 380, 480, 460), 12),
    ('arrow', ['--rect', '380,300,540,360', '--dashed'], '/Line', (380, 300, 540, 360), 16),
    ('link', ['--rect', '72,300,200,330', '--url', URL], '/Link', (72, 300, 200, 330), 1),
    ('link', ['--rect', '220,300,340,330', '--goto-page', '2'], '/Link', (220, 300, 340, 330), 1),
]
STAMP_RECT = (400, 80, 560, 160)


def expect(condition, message):
    if not condition:
        raise ValueError(message)


def base_pdf(path):
    """Two plain pages with a catalog XMP packet, built with the repository's dependency-free fixture writer."""
    spec = importlib.util.spec_from_file_location('make_fixtures', Path(__file__).with_name('make_fixtures.py'))
    fixtures = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixtures)
    xmp = ('<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?><x:xmpmeta xmlns:x="adobe:ns:meta/">'
           '<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" '
           'xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:xmp="http://ns.adobe.com/xap/1.0/">'
           '<dc:title><rdf:Alt><rdf:li xml:lang="x-default">Source</rdf:li></rdf:Alt></dc:title>'
           '<xmp:CreatorTool>SuniPDF reader fixture</xmp:CreatorTool></rdf:Description></rdf:RDF></x:xmpmeta>'
           '<?xpacket end="w"?>').encode('utf-8')
    objects = [
        b'<< /Type /Catalog /Pages 2 0 R /Metadata 8 0 R >>',
        b'<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>',
        b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 7 0 R >> >> >>',
        fixtures.stream(b'0 0 1 rg 72 500 200 150 re f\nBT /F1 24 Tf 72 700 Td (SuniPDF page 1) Tj ET\n'),
        b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 6 0 R /Resources << /Font << /F1 7 0 R >> >> >>',
        fixtures.stream(b'BT /F1 24 Tf 72 400 Td (SuniPDF page 2) Tj ET\n'),
        b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
        b'<< /Type /Metadata /Subtype /XML /Length ' + str(len(xmp)).encode() + b' >>\nstream\n' + xmp + b'\nendstream',
    ]
    path.write_bytes(fixtures.build(objects))


def png(path):
    """A small dark-ink RGB PNG for the visual signature image (no imaging dependency)."""
    import struct
    import zlib
    width, height = 120, 40
    rows = b''.join(b'\0' + b''.join(b'\x10\x10\x60' if 10 < x < 110 and abs(y - 20 - (x % 20 - 10)) < 3 else b'\xff\xff\xff'
                                     for x in range(width)) for y in range(height))
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
                     + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))


def plain(value, v2):
    """Normalise QPDF JSON v1 (qpdf 10) and v2 (qpdf 11+) object values to one shape."""
    if isinstance(value, dict):
        return {key: plain(item, v2) for key, item in value.items()}
    if isinstance(value, list):
        return [plain(item, v2) for item in value]
    if v2 and isinstance(value, str) and value[:2] in ('u:', 'b:'):
        return value[2:] if value[0] == 'u' else bytes.fromhex(value[2:]).decode('latin-1')
    return value


def qpdf_objects(data):
    if 'qpdf' in data:
        objects = {}
        for key, entry in data['qpdf'][1].items():
            name = key[4:] if key.startswith('obj:') else key
            objects[name] = plain(entry['value'], True) if 'value' in entry else {'stream': plain(entry['stream'], True)}
        return objects
    return plain(data['objects'], False)


def stream_dict(value):
    return value['stream']['dict'] if isinstance(value, dict) and 'stream' in value else value


class Reader:
    """Bounded independent-tool calls that keep every command output beside the evidence."""

    def __init__(self, args, output):
        self.args, self.output, self.count, self.json = args, output, 0, ['--json=2', '--json-key=qpdf']

    def call(self, command, allowed=(0,)):
        self.count += 1
        path = self.output / f'{self.count:03d}-{Path(str(command[0])).stem}.txt'
        code = run(command, path, allowed=allowed)
        return code, path.read_bytes(), path.with_suffix('.txt.stderr').read_text(encoding='utf-8', errors='replace')

    def cli(self, *arguments):
        return json.loads(self.call([self.args.cli, *arguments])[1])

    @staticmethod
    def password(password):
        return [] if password is None else ['--password=' + password]

    def objects(self, pdf, password=None):
        # QPDF 10 (Ubuntu 22.04) only writes JSON v1 (`--json`, no version argument); QPDF 11+ writes v2.
        data = self.call([self.args.qpdf, *self.password(password), *self.json, pdf])[1]
        objects = qpdf_objects(json.loads(data))
        def resolve(value):
            return objects.get(value, value) if isinstance(value, str) and value.endswith(' R') else value
        return objects, resolve

    def stream(self, pdf, reference, password=None):
        return self.call([self.args.qpdf, *self.password(password), '--show-object=' + reference.split()[0],
                          '--filtered-stream-data', pdf])[1]

    def poppler(self, name):
        return None if self.args.poppler is None else self.args.poppler / name


def check_metadata(reader, pdf, password=None):
    objects, resolve = reader.objects(pdf, password)
    info = resolve(objects['trailer']['/Info'])
    for key, value in (('/Title', TITLE), ('/Author', AUTHOR), ('/Subject', SUBJECT), ('/Keywords', KEYWORDS)):
        expect(info.get(key) == value, f'QPDF Info {key} is {info.get(key)!r}')
    xmp = reader.stream(pdf, resolve(objects['trailer']['/Root'])['/Metadata'], password)
    expect(b'<!DOCTYPE' not in xmp.upper() and b'<!ENTITY' not in xmp.upper(), 'Unsafe XMP packet')
    root = ET.fromstring(xmp[xmp.index(b'<x:xmpmeta'):xmp.rindex(b'</x:xmpmeta>') + len(b'</x:xmpmeta>')])
    ns = {'rdf': 'http://www.w3.org/1999/02/22-rdf-syntax-ns#', 'dc': 'http://purl.org/dc/elements/1.1/',
          'pdf': 'http://ns.adobe.com/pdf/1.3/', 'xmp': 'http://ns.adobe.com/xap/1.0/'}
    def text(path):
        found = root.find('.//' + path, ns)
        return None if found is None else found.text
    keywords = text('pdf:Keywords')
    if keywords is None:
        keywords = next((d.get('{%s}Keywords' % ns['pdf']) for d in root.iter('{%s}Description' % ns['rdf'])
                         if d.get('{%s}Keywords' % ns['pdf']) is not None), None)
    facts = {'title': text('dc:title/rdf:Alt/rdf:li'), 'creator': text('dc:creator/rdf:Seq/rdf:li'),
             'description': text('dc:description/rdf:Alt/rdf:li'), 'keywords': keywords,
             'preservedCreatorTool': text('xmp:CreatorTool')}
    expected = {'title': TITLE, 'creator': AUTHOR, 'description': SUBJECT, 'keywords': KEYWORDS,
                'preservedCreatorTool': 'SuniPDF reader fixture'}
    expect(facts == expected, 'XMP read by QPDF differs: ' + json.dumps(facts, ensure_ascii=False))
    return {'info': 'Title, Author, Subject and Keywords match', 'xmp': facts}


def check_annotations(reader, pdf, password=None):
    objects, resolve = reader.objects(pdf, password)
    pages = resolve(resolve(objects['trailer']['/Root'])['/Pages'])['/Kids']
    annots = [(reference, resolve(reference)) for reference in resolve(resolve(pages[0]).get('/Annots', []))]
    subtypes = sorted(a['/Subtype'] for _, a in annots)
    expected = sorted([subtype for _, _, subtype, _, _ in ANNOTATIONS] + ['/Stamp', '/Text'])
    expect(subtypes == expected, 'Annotation subtypes differ: ' + json.dumps(subtypes))
    facts, pending = [], list(annots)
    for kind, arguments, subtype, rect, tolerance in ANNOTATIONS + [('stamp', [], '/Stamp', STAMP_RECT, 1)]:
        match = next((item for item in pending if item[1]['/Subtype'] == subtype and all(
            abs(float(a) - b) <= tolerance for a, b in zip(item[1]['/Rect'], rect))), None)
        expect(match is not None, f'No {kind} annotation within {tolerance} pt of {rect}')
        pending.remove(match)
        reference, annotation = match
        fact = {'kind': kind, 'subtype': subtype, 'rect': [round(float(v), 3) for v in annotation['/Rect']]}
        if kind != 'link':
            normal = resolve(resolve(annotation.get('/AP', {})).get('/N'))
            # JSON v1 shows a stream as its dictionary only; /BBox is required on appearance forms.
            expect(isinstance(normal, dict) and '/BBox' in stream_dict(normal), f'{kind} has no normal appearance stream')
            fact['normalAppearance'] = True
        if kind in ('highlight', 'underline', 'strikeout'):
            quads = [float(v) for v in annotation['/QuadPoints']]
            expect(len(quads) == 8 and (min(quads[0::2]), min(quads[1::2]), max(quads[0::2]), max(quads[1::2])) == rect,
                   f'{kind} QuadPoints differ: {quads}')
            fact['quadPoints'] = quads
        elif kind == 'ink':
            ink = [[float(v) for v in stroke] for stroke in annotation['/InkList']]
            expect(ink == [[320, 500, 360, 560, 400, 520]], f'InkList differs: {ink}')
            fact['inkList'] = ink
        elif kind == 'freetext':
            expect(annotation.get('/Contents') == FREE_TEXT and '/DA' in annotation, 'FreeText contents or /DA differ')
            reply = next(a for _, a in annots if a['/Subtype'] == '/Text')
            expect(reply.get('/IRT') == reference and reply.get('/Contents') == REPLY, 'Reply is not threaded to the FreeText')
            fact.update({'contents': annotation['/Contents'], 'reply': {'inReplyTo': True, 'contents': reply['/Contents']}})
        elif kind in ('line', 'arrow'):
            line = [float(v) for v in annotation['/L']]
            expect(line == [float(v) for v in rect], f'{kind} /L differs: {line}')
            ends = annotation.get('/LE', ['/None', '/None'])
            expect((ends != ['/None', '/None']) == (kind == 'arrow'), f'{kind} line endings differ: {ends}')
            if kind == 'arrow':
                dash = resolve(annotation.get('/BS', {})).get('/D')
                expect([float(v) for v in dash or []] == [6, 3], f'Arrow dash differs: {dash}')
            fact.update({'line': line, 'endings': ends})
        elif kind == 'link':
            action = resolve(annotation.get('/A', {}))
            if '--url' in arguments:
                expect(action.get('/S') == '/URI' and action.get('/URI') == URL, f'URI action differs: {action}')
                fact['uri'] = URL
            else:
                destination = resolve(annotation.get('/Dest') or action.get('/D'))
                expect(isinstance(destination, list) and destination[0] == pages[1], f'GoTo destination differs: {destination}')
                fact['destinationPage'] = 2
        elif kind == 'stamp':
            resources = resolve(stream_dict(resolve(annotation['/AP']['/N'])).get('/Resources', {}))
            xobjects = [stream_dict(resolve(x)) for x in resolve(resources.get('/XObject', {})).values()]
            expect(any(x.get('/Subtype') == '/Image' for x in xobjects), 'Visual signature stamp has no image XObject')
            fact['imageXObject'] = True
        facts.append(fact)
    return {'count': len(annots), 'subtypes': subtypes, 'annotations': facts}


def check_encryption(reader, pdf, revision, method):
    code = reader.call([reader.args.qpdf, '--requires-password', pdf], allowed=(0, 2, 3))[0]
    expect(code == 0, f'QPDF --requires-password returned {code}; an open password was expected')
    code = reader.call([reader.args.qpdf, '--password=wrong-106', '--check', pdf], allowed=(0, 2, 3))[0]
    expect(code == 2, 'QPDF opened the file with a wrong password')
    output = reader.call([reader.args.qpdf, '--password=' + USER_PASSWORD, '--show-encryption', pdf])[1].decode('utf-8', 'replace')
    lines = {}
    for line in output.splitlines():
        key, separator, value = line.partition(': ') if ': ' in line else line.partition(' = ')
        if separator:
            lines[key.strip()] = value.strip()
    expected = {'R': str(revision), 'stream encryption method': method, 'string encryption method': method,
                'print low resolution': 'not allowed', 'print high resolution': 'not allowed',
                'extract for any purpose': 'not allowed', 'modify annotations': 'allowed', 'modify other': 'allowed'}
    found = {key: lines.get(key) for key in expected}
    expect(found == expected, 'QPDF encryption facts differ: ' + json.dumps(found))
    reader.call([reader.args.qpdf, '--password=' + OWNER_PASSWORD, '--check', pdf])
    return {'qpdf': found, 'passwordRequired': True, 'wrongPasswordRejected': True, 'ownerPasswordAccepted': True}


def check_form(reader, filled, flattened):
    objects, resolve = reader.objects(filled)
    fields = {}
    def walk(references, prefix=''):
        for reference in references:
            field = resolve(reference)
            name = prefix + field.get('/T', '')
            fields[name] = field
            walk(field.get('/Kids', []), name + '.')
    walk(resolve(resolve(objects['trailer']['/Root'])['/AcroForm'])['/Fields'])
    nom, accepte, pays = fields['nom'], fields['accepte'], fields['pays']
    expect(nom.get('/V') == FIELD_VALUE, f"QPDF reads nom={nom.get('/V')!r}")
    expect(accepte.get('/V') == '/Yes' and accepte.get('/AS') == '/Yes', 'Checkbox value or appearance state differs')
    expect(pays.get('/V') == 'CA', f"QPDF reads pays={pays.get('/V')!r}")
    expect(FIELD_VALUE.encode() in reader.stream(filled, resolve(nom['/AP'])['/N']), 'Text field appearance does not draw the new value')
    flat, flat_resolve = reader.objects(flattened)
    catalog = flat_resolve(flat['trailer']['/Root'])
    remaining = flat_resolve(flat_resolve(catalog.get('/AcroForm', {})).get('/Fields', []))
    page = flat_resolve(flat_resolve(flat_resolve(catalog['/Pages'])['/Kids'])[0])
    widgets = [a for a in (flat_resolve(r) for r in flat_resolve(page.get('/Annots', []))) if a.get('/Subtype') == '/Widget']
    expect(not remaining and not widgets, 'Flattened copy still has form fields or widgets')
    return {'filled': {'nom': FIELD_VALUE, 'accepte': '/Yes', 'pays': 'CA', 'appearanceDrawsValue': True},
            'flattened': {'fields': 0, 'widgets': 0}}


def der_length(data):
    expect(len(data) > 4 and data[0] == 0x30, 'Signature contents are not a DER SEQUENCE')
    if data[1] < 0x80:
        return 2 + data[1]
    size = data[1] & 0x7F
    return 2 + size + int.from_bytes(data[2:2 + size], 'big')


def check_signature(reader, pdf):
    objects, resolve = reader.objects(pdf)
    form = resolve(resolve(objects['trailer']['/Root'])['/AcroForm'])
    expect(int(form.get('/SigFlags', 0)) == 3, 'AcroForm /SigFlags is not 3')
    signatures = [resolve(f) for f in resolve(form['/Fields']) if resolve(f).get('/FT') == '/Sig']
    expect(len(signatures) == 1, 'Expected exactly one signature field')
    value = resolve(signatures[0]['/V'])
    data = pdf.read_bytes()
    start, first, second, length = (int(v) for v in value['/ByteRange'])
    expect(start == 0 and 0 < first < second and second + length == len(data), 'ByteRange does not cover the whole file')
    expect(data[first:first + 1] == b'<' and data[second - 1:second] == b'>', 'ByteRange gap is not exactly /Contents')
    contents = bytes.fromhex(data[first + 1:second - 1].decode('ascii'))
    der = contents[:der_length(contents)]
    expect(not contents[len(der):].strip(b'\0'), 'Unexpected bytes after the CMS object')
    signature, ranges = reader.output / 'signature.der', reader.output / 'signed-ranges.bin'
    signature.write_bytes(der)
    signed = data[:first] + data[second:]
    ranges.write_bytes(signed)
    verify = [reader.args.openssl, 'cms', '-verify', '-binary', '-inform', 'DER', '-in', signature,
              '-content', ranges, '-noverify', '-out', reader.output / 'verified-content.bin']
    code, _, diagnostic = reader.call(verify, allowed=(0, 4))
    expect(code == 0 and 'Verification successful' in diagnostic, 'OpenSSL rejected the CMS signature: ' + diagnostic)
    tampered = bytearray(signed)
    tampered[len(tampered) // 2] ^= 1
    ranges.write_bytes(bytes(tampered))
    expect(reader.call(verify, allowed=(0, 4))[0] == 4, 'OpenSSL accepted a signature over tampered bytes')
    return {'subFilter': value.get('/SubFilter'), 'byteRange': [start, first, second, length],
            'coversWholeFileExceptContents': True, 'cmsBytes': len(der), 'opensslVerified': True,
            'tamperedRejected': True, 'certificateTrust': 'not evaluated (-noverify, self-signed test identity)'}


def check_poppler(reader, files):
    tool, facts = reader.poppler, {}
    info = reader.call([tool('pdfinfo'), files['metadata']])[1].decode('utf-8', 'replace')
    fields = {k.strip(): v.strip() for k, _, v in (line.partition(':') for line in info.splitlines())}
    found = {key: fields.get(key) for key in ('Title', 'Author', 'Subject', 'Keywords')}
    expect(found == {'Title': TITLE, 'Author': AUTHOR, 'Subject': SUBJECT, 'Keywords': KEYWORDS},
           'pdfinfo metadata differs: ' + json.dumps(found, ensure_ascii=False))
    xmp = reader.call([tool('pdfinfo'), '-meta', files['metadata']])[1].decode('utf-8', 'replace')
    expect(TITLE in xmp and KEYWORDS in xmp, 'pdfinfo -meta does not show the updated XMP')
    facts['metadata'] = {'info': found, 'xmpShowsUpdatedValues': True}
    for name, algorithm in (('aes256', 'AES-256'), ('aes128', 'AES')):
        expect(reader.call([tool('pdfinfo'), files[name]], allowed=(0, 1))[0] == 1, f'pdfinfo opened {name} without a password')
        info = reader.call([tool('pdfinfo'), '-upw', USER_PASSWORD, files[name]])[1].decode('utf-8', 'replace')
        line = next((value for value in info.splitlines() if value.startswith('Encrypted:')), '')
        wanted = f'yes (print:no copy:no change:yes addNotes:yes algorithm:{algorithm})'
        expect(line.partition(':')[2].strip() == wanted, f'pdfinfo encryption differs: {line}')
        expect(any(value.startswith('Title:') and value.partition(':')[2].strip() == TITLE for value in info.splitlines()),
               f'pdfinfo cannot read the encrypted title of {name}')
        text = reader.call([tool('pdftotext'), '-upw', USER_PASSWORD, files[name], '-'])[1].decode('utf-8', 'replace')
        expect('SuniPDF page 1' in text and 'SuniPDF page 2' in text, f'pdftotext cannot read {name}')
        facts[name] = {'encrypted': wanted, 'passwordRequired': True, 'title': TITLE, 'text': True}
    text = reader.call([tool('pdftotext'), files['flattened'], '-'])[1].decode('utf-8', 'replace')
    expect(FIELD_VALUE in text, 'pdftotext does not find the flattened field value in page content')
    facts['flattened'] = {'valueInPageText': True}
    output = reader.call([tool('pdfsig'), files['annotated']], allowed=(0, 1, 2))[1]
    expect(b'does not contain any signatures' in output, 'pdfsig reports a signature in the visual-only copy')
    facts['visualSignature'] = {'pdfsig': 'no digital signature (visual stamp only)'}
    raster, independent = reader.output / 'annotated-pdfium.png', reader.output / 'annotated-poppler'
    reader.call([reader.args.cli, 'render', files['annotated'], '--page', '1', '--scale', '1', '--out', raster])
    reader.call([tool('pdftoppm'), '-f', '1', '-singlefile', '-cropbox', '-r', '72', '-png', files['annotated'], independent])
    facts['annotationAppearance'] = appearance(image(raster), image(independent.with_suffix('.png')), allow_rounding=True)
    return facts


def readers(args):
    """Write each feature with the CLI, then assert facts with independent readers."""
    output = args.out.absolute()
    output.mkdir(parents=True, exist_ok=False)
    report = {'schema': 1, 'status': 'running', 'issue': 106, 'tools': {}, 'cases': {}, 'skipped': [], 'errors': [],
              'scope': 'SuniPDF CLI outputs re-read by independent implementations: QPDF objects and encryption, '
                       'Poppler info/text/signatures/raster, OpenSSL CMS. Adobe Acrobat checks remain manual.'}
    reader, cases = Reader(args, output), report['cases']
    try:
        report['tools']['python'] = sys.version
        for name, executable, arguments in [('sunipdf', args.cli, ['--version']), ('qpdf', args.qpdf, ['--version']),
                                            ('poppler', reader.poppler('pdfinfo'), ['-v']), ('openssl', args.openssl, ['version'])]:
            if executable is None:
                continue
            resolved = Path(shutil.which(str(executable)) or executable).resolve()
            expect(resolved.is_file(), 'Required executable is missing: ' + name)
            _, out, err = reader.call([resolved, *arguments])
            report['tools'][name] = {'version': (out.decode('utf-8', 'replace') + err).strip(), 'sha256': fetcher.sha256(resolved)}
            if name == 'qpdf':
                if out.split()[2].startswith(b'10.'):
                    reader.json = ['--json', '--json-key=objects']
        files = {name: output / (name + '.pdf') for name in ('base', 'metadata', 'annotated', 'aes256', 'aes128', 'filled', 'flattened', 'signed')}
        base_pdf(files['base'])
        reader.cli('set-metadata', files['base'], '--title', TITLE, '--author', AUTHOR, '--subject', SUBJECT,
                   '--keywords', KEYWORDS, '--out', files['metadata'])
        current = files['metadata']
        for step, (kind, arguments, _, _, _) in enumerate(ANNOTATIONS):
            target = output / f'annotation-{step:02d}-{kind}.pdf'
            reader.cli('annotate', current, '--page', '1', '--kind', kind, *arguments, '--out', target)
            current = target
        listed = reader.cli('annotations', current, '--page', '1')
        free_text = next(a['index'] for a in listed['pages'][0]['annotations'] if a['subtype'] == 'FreeText')
        reader.cli('reply', current, '--page', '1', '--index', str(free_text), '--text', REPLY, '--author', AUTHOR,
                   '--out', output / 'replied.pdf')
        png(output / 'signature-source.png')
        reader.cli('prepare-signature-image', output / 'signature-source.png', '--out', output / 'signature.png', '--background', 'threshold')
        reader.cli('image-stamp', output / 'replied.pdf', '--image', output / 'signature.png', '--rect', ','.join(map(str, STAMP_RECT)),
                   '--out', files['annotated'])
        for name, extra in (('aes256', []), ('aes128', ['--aes128'])):
            reader.cli('protect', files['annotated'], '--user-password', USER_PASSWORD, '--owner-password', OWNER_PASSWORD,
                       '--no-print', '--no-copy', *extra, '--out', files[name])
        form = Path(__file__).resolve().parents[1] / 'crates/sunipdf-core/tests/fixtures/acroform.pdf'
        values = ['--set', 'nom=' + FIELD_VALUE, '--set', 'accepte=true', '--set', 'pays=CA']
        reader.cli('fill-form', form, *values, '--out', files['filled'])
        reader.cli('fill-form', form, *values, '--flatten', '--out', files['flattened'])

        def case(name, action, *pdfs, password=None):
            entry = cases[name] = {'pdfSha256': {pdf.name: fetcher.sha256(pdf) for pdf in pdfs}}
            try:
                for pdf in pdfs:
                    code = reader.call([args.qpdf, *reader.password(password), '--check', pdf], allowed=(0, 2, 3))[0]
                    expect(code == 0, f'QPDF --check failed for {pdf.name} ({code})')
                entry['facts'] = action()
                entry['status'] = 'passed'
            except (ValueError, KeyError, TypeError, IndexError, StopIteration, ET.ParseError) as error:
                entry.update({'status': 'failed', 'error': f'{type(error).__name__}: {error}'})
        case('xmp-metadata', lambda: check_metadata(reader, files['metadata']), files['metadata'])
        case('annotations', lambda: check_annotations(reader, files['annotated']), files['annotated'])
        for name, revision, method in (('aes256', 6, 'AESv3'), ('aes128', 4, 'AESv2')):
            case(name, lambda name=name, revision=revision, method=method: {
                'encryption': check_encryption(reader, files[name], revision, method),
                'metadataThroughEncryption': check_metadata(reader, files[name], USER_PASSWORD),
                'annotationsThroughEncryption': check_annotations(reader, files[name], USER_PASSWORD)['subtypes']},
                files[name], password=USER_PASSWORD)
        case('forms', lambda: check_form(reader, files['filled'], files['flattened']), files['filled'], files['flattened'])
        if args.openssl is None:
            report['skipped'].append('cryptographic signature: OpenSSL not provided')
        else:
            key, cert, pfx = output / 'identity.key', output / 'identity.crt', output / 'identity.pfx'
            reader.call([args.openssl, 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-subj', '/CN=SuniPDF Reader Check',
                         '-days', '2', '-keyout', key, '-out', cert])
            reader.call([args.openssl, 'pkcs12', '-export', '-inkey', key, '-in', cert, '-out', pfx, '-passout', 'pass:reader-106'])
            os.environ['SUNIPDF_READER_PFX_PASSWORD'] = 'reader-106'
            code, _, diagnostic = reader.call([args.cli, 'sign', files['metadata'], '--certificate', pfx, '--certificate-password-env',
                                               'SUNIPDF_READER_PFX_PASSWORD', '--reason', 'Independent reader check',
                                               '--out', files['signed']], allowed=(0, 1))
            if code and 'unavailable on this platform' in diagnostic:
                report['skipped'].append('cryptographic signature: the CLI cannot sign on this platform (Windows-only backend)')
            else:
                expect(code == 0, 'Signing failed: ' + diagnostic[-1000:])
                case('cryptographic-signature', lambda: check_signature(reader, files['signed']), files['signed'])
        if args.poppler is None:
            report['skipped'].append('poppler: pdfinfo, pdftotext, pdfsig and pdftoppm not provided')
        else:
            try:
                cases['poppler'] = {'facts': check_poppler(reader, files), 'status': 'passed'}
            except (ValueError, ImportError, StopIteration) as error:
                cases['poppler'] = {'status': 'failed', 'error': f'{type(error).__name__}: {error}'}
    except (ValueError, OSError, KeyError, StopIteration, json.JSONDecodeError) as error:
        report['errors'].append(f'{type(error).__name__}: {error}')
    passed = bool(cases) and all(case['status'] == 'passed' for case in cases.values())
    report['status'] = 'passed' if passed and not report['errors'] else 'failed'
    (output / 'report.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    return report


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
        return ('<report><buildInformation><releaseDetails version="1.30.3"/></buildInformation><jobs><job><item size="3"><name>' + escape(str(pdf)) + '</name></item><validationReport jobEndStatus="normal" profileName="PDF/UA-1 validation profile" isCompliant="' + flag + '"><details passedRules="10" passedChecks="20" failedRules="' + failed + '" failedChecks="' + failed + '">' + rule + '</details></validationReport></job></jobs><batchSummary totalJobs="1" failedToParse="0" encrypted="0" outOfMemory="0" veraExceptions="0"><validationReports compliant="' + str(int(compliant)) + '" nonCompliant="' + failed + '" failedJobs="0">1</validationReports></batchSummary></report>').encode()

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
            for old, new in [(b'PDF/UA-1', b'PDF/A-1b'), (b'test.pdf', b'other.pdf'), (b'1.30.3', b'1.20.0'), (b'totalJobs="1"', b'totalJobs="2"'), (b'failedToParse="0"', b'failedToParse="1"'), (b'failedJobs="0"', b'failedJobs="1"'), (b'jobEndStatus="normal"', b'jobEndStatus="error"'), (b'isCompliant="true"', b'isCompliant="false"'), (b'passedRules="10"', b'passedRules="0"')]:
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


class ReaderTests(unittest.TestCase):
    def test_qpdf_json_v1_and_v2_normalise_alike(self):
        v1 = {'objects': {'1 0 R': {'/Title': 'Été', '/Kids': ['2 0 R']}, '2 0 R': {'/BBox': [0, 0, 1, 1]},
                          'trailer': {'/Root': '1 0 R'}}}
        v2 = {'qpdf': [{'jsonversion': 2}, {'obj:1 0 R': {'value': {'/Title': 'u:Été', '/Kids': ['2 0 R']}},
                                            'obj:2 0 R': {'stream': {'dict': {'/BBox': [0, 0, 1, 1]}}},
                                            'trailer': {'value': {'/Root': '1 0 R'}}}]}
        first, second = qpdf_objects(v1), qpdf_objects(v2)
        self.assertEqual(first['1 0 R'], second['1 0 R'])
        self.assertEqual(first['trailer'], second['trailer'])
        self.assertEqual(stream_dict(first['2 0 R']), stream_dict(second['2 0 R']))
        self.assertEqual(plain('b:c9', True), 'É')

    def test_der_length_bounds_the_cms_object(self):
        self.assertEqual(der_length(bytes([0x30, 0x03, 1, 2, 3, 0, 0])), 5)
        self.assertEqual(der_length(bytes([0x30, 0x82, 0x01, 0x00]) + bytes(256)), 260)
        with self.assertRaises(ValueError):
            der_length(b'\x31\x01\x00\x00\x00')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('cli', 'corpus', 'verapdf', 'qpdf', 'pdftoppm', 'out', 'poppler', 'openssl'):
        parser.add_argument('--' + name, type=Path)
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--readers', action='store_true', help='Re-read SuniPDF feature outputs (issue #106); '
                        '--poppler (directory) and --openssl are optional and recorded as skipped when absent')
    args = parser.parse_args()
    if args.self_test:
        unittest.main(argv=['test-conformance'], exit=True)
    if args.readers:
        if any(getattr(args, name) is None for name in ('cli', 'qpdf', 'out')):
            parser.error('--readers requires --cli, --qpdf and --out')
        result = readers(args)
        print(json.dumps({'status': result['status'], 'cases': {k: v['status'] for k, v in result['cases'].items()},
                          'skipped': result['skipped'], 'errors': result['errors'], 'report': str(args.out / 'report.json')}))
        raise SystemExit(0 if result['status'] == 'passed' else 1)
    if any(getattr(args, name) is None for name in ('cli', 'corpus', 'verapdf', 'qpdf', 'pdftoppm', 'out')):
        parser.error('All tool, corpus and output paths are required')
    result = qualify(args)
    print(json.dumps({'status': result['status'], 'cases': len(result['cases']), 'report': str(args.out / 'report.json')}))
    raise SystemExit(0 if result['status'] == 'passed' else 1)


if __name__ == '__main__':
    main()
