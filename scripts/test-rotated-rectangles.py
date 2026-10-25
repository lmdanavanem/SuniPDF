"""Verify dimensioned rectangle rotation through the real CLI and independent QPDF geometry."""
import argparse
import base64
import hashlib
import json
import math
import re
from pathlib import Path
import subprocess
from datetime import datetime, timezone


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def fixture_pdf():
    objects = [b'<< /Type /Catalog /Pages 2 0 R >>', b'',
               b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>']
    kids = []
    for rotation in [0, 90, 180, 270]:
        page = len(objects) + 1
        kids.append(f'{page} 0 R')
        content = f'BT /F1 20 Tf 72 700 Td (Synthetic page rotation {rotation}) Tj ET\n'.encode()
        objects.append((f'<< /Type /Page /Parent 2 0 R /MediaBox [-20 -30 620 830] '
                        f'/CropBox [10 20 610 820] /Rotate {rotation} '
                        f'/Resources << /Font << /F1 3 0 R >> >> /Contents {page + 1} 0 R >>').encode())
        objects.append(b'<< /Length ' + str(len(content)).encode() + b' >>\nstream\n' + content + b'endstream')
    objects[1] = f'<< /Type /Pages /Kids [{" ".join(kids)}] /Count 4 >>'.encode()
    data, offsets = bytearray(b'%PDF-1.7\n%\xe2\xe3\xcf\xd3\n'), []
    for index, body in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f'{index} 0 obj\n'.encode() + body + b'\nendobj\n')
    xref = len(data)
    data.extend(f'xref\n0 {len(objects) + 1}\n0000000000 65535 f \n'.encode())
    for offset in offsets:
        data.extend(f'{offset:010d} 00000 n \n'.encode())
    data.extend(f'trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode())
    return bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--qpdf', type=Path, required=True)
    parser.add_argument('--directory', type=Path, required=True)
    args = parser.parse_args()
    cli, qpdf = args.cli.resolve(strict=True), args.qpdf.resolve(strict=True)
    directory = args.directory.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    source = directory / 'rotated-pages.pdf'
    source.write_bytes(fixture_pdf())
    source_hash = sha(source)
    report = {'version': 1, 'status': 'running', 'started_utc': datetime.now(timezone.utc).isoformat(),
              'cli_sha256': sha(cli), 'qpdf_sha256': sha(qpdf), 'source_sha256': source_hash,
              'checks': [], 'commands': [], 'outputs': [],
              'scope': 'Synthetic packaged CLI and QPDF geometry, not native pointer or physical survey qualification.'}

    def check(name, condition):
        report['checks'].append({'name': name, 'passed': bool(condition)})
        if not condition:
            raise AssertionError(name)

    def invoke(name, command, reject=False, parse=True):
        result = subprocess.run([str(value) for value in command], stdin=subprocess.DEVNULL,
                                capture_output=True, timeout=120, cwd=directory)
        for stream in ['stdout', 'stderr']:
            (directory / f'{name}.{stream}').write_bytes(getattr(result, stream))
        report['commands'].append({'name': name, 'exit_code': result.returncode})
        check(name + ': process outcome', result.returncode != 0 if reject else result.returncode == 0)
        return json.loads(result.stdout) if parse and not reject else result.stdout

    def independent(name, path):
        invoke(name + '-check', [qpdf, '--check', path], parse=False)
        value = invoke(name + '-json', [qpdf, '--json', '--json-stream-data=inline', path])
        objects = value['qpdf'][1]
        pages = []
        for page in value['pages']:
            details = objects['obj:' + page['object']]['value']
            annotations = [objects['obj:' + item]['value'] if isinstance(item, str) else item
                           for item in details.get('/Annots', [])]
            for annotation in annotations:
                appearance = objects['obj:' + annotation['/AP']['/N']]['stream']
                annotation['_appearance'] = appearance['dict']
                annotation['_drawing'] = base64.b64decode(appearance['data'], validate=True).decode('ascii')
            content = b''.join(base64.b64decode(objects['obj:' + item]['stream']['data'], validate=True)
                               for item in page['contents'])
            pages.append((details, annotations, content))
        return pages

    try:
        report['cli_version'] = invoke('version', [cli, '--version'], parse=False).decode().strip()
        original = independent('source', source)
        base_arguments = [cli, 'dimensioned-rectangle', source, '--anchor', '300,300', '--width', '4',
                          '--height', '6', '--unit', 'm', '--precision', '4', '--calibration', '50,50 150,50',
                          '--known-distance', '10']
        for page_index, page_rotation in enumerate([0, 90, 180, 270]):
            for angle_index, angle in enumerate([0, 33.5, 90, -27, 360]):
                name = f'page-{page_index + 1}-angle-{angle_index}'
                output = directory / (name + '.pdf')
                result = invoke(name, [*base_arguments, '--page', page_index + 1,
                                      '--rotation-degrees', angle, '--out', output])
                check(name + ': normalized reported rotation', math.isclose(result['rotation_degrees'], angle % 360, abs_tol=1e-10))
                check(name + ': real area and unit', math.isclose(result['measurement']['value'], 24, rel_tol=2e-5)
                      and result['measurement']['kind'] == 'area')
                saved = independent(name, output)
                check(name + ': all pages retained', len(saved) == 4)
                for index, ((details, annotations, content), (old_details, _, old_content)) in enumerate(zip(saved, original)):
                    check(name + f': page {index + 1} original content', content == old_content)
                    check(name + f': page {index + 1} original geometry', all(details.get(key) == old_details.get(key)
                          for key in ['/MediaBox', '/CropBox', '/Rotate']))
                    check(name + f': page {index + 1} annotation count', len(annotations) == (1 if index == page_index else 0))
                annotation = saved[page_index][1][0]
                check(name + ': measurement stamp retained', annotation['/Subtype'] == '/Stamp'
                      and annotation['/Name'] == '/SuniPDFMeasurement')
                encoded = annotation['/SuniPDFMeasurement']
                check(name + ': readable measurement metadata', isinstance(encoded, str) and encoded.startswith('u:'))
                metadata = json.loads(encoded[2:])
                check(name + ': measured area metadata', metadata['version'] == 2
                      and metadata['coordinate_system'] == 'pdf-user-space'
                      and metadata['kind'] == 'area' and metadata['unit'] == 'm'
                      and len(metadata['points']) == 4)
                points = metadata['points']
                edges = [(points[(index + 1) % 4][0] - point[0], points[(index + 1) % 4][1] - point[1])
                         for index, point in enumerate(points)]
                check(name + ': all real side lengths', all(math.isclose(math.hypot(*edge), wanted, abs_tol=.002)
                      for edge, wanted in zip(edges, [40, 60, 40, 60])))
                check(name + ': perpendicular edges', abs(sum(a * b for a, b in zip(edges[0], edges[1]))) < .2)
                area = abs(sum(point[0] * points[(index + 1) % 4][1] - points[(index + 1) % 4][0] * point[1]
                               for index, point in enumerate(points))) / 2
                check(name + ': independently calculated area', math.isclose(area, 2400, abs_tol=.15))
                actual_angle = math.degrees(math.atan2(edges[0][1], edges[0][0])) % 360
                expected_angle = (page_rotation - angle) % 360
                error = abs((actual_angle - expected_angle + 180) % 360 - 180)
                check(name + ': clockwise angle in PDF user space', error < .005)
                outline = re.search(r'(?m)^([-+0-9.]+) ([-+0-9.]+) m\n'
                                    r'([-+0-9.]+) ([-+0-9.]+) l\n'
                                    r'([-+0-9.]+) ([-+0-9.]+) l\n'
                                    r'([-+0-9.]+) ([-+0-9.]+) l\nh\nS\n', annotation['_drawing'])
                check(name + ': closed four-sided drawn outline', outline is not None)
                values = list(map(float, outline.groups()))
                drawn = list(zip(values[::2], values[1::2]))
                drawn_edges = [(drawn[(i + 1) % 4][0] - point[0], drawn[(i + 1) % 4][1] - point[1])
                               for i, point in enumerate(drawn)]
                check(name + ': drawn sides match dimensions', all(math.isclose(math.hypot(*edge), wanted, abs_tol=.003)
                      for edge, wanted in zip(drawn_edges, [40, 60, 40, 60])))
                drawn_angle = math.degrees(math.atan2(drawn_edges[0][1], drawn_edges[0][0]))
                check(name + ': drawn clockwise angle', abs((drawn_angle + angle + 180) % 360 - 180) < .005)
                report['outputs'].append({'name': output.name, 'sha256': sha(output), 'page': page_index + 1,
                                          'page_rotation': page_rotation, 'rotation_degrees': angle % 360, 'area_points_squared': area})
        for angle in ['NaN', 'inf', '-inf']:
            name = 'refuse-angle-' + angle.replace('-', 'minus')
            output = directory / (name + '.pdf')
            invoke(name, [*base_arguments, '--rotation-degrees', angle, '--out', output], reject=True, parse=False)
            check(name + ': no output', not output.exists())
        outside = directory / 'outside.pdf'
        outside_arguments = list(base_arguments)
        outside_arguments[outside_arguments.index('--anchor') + 1] = '5,5'
        invoke('refuse-outside', [*outside_arguments, '--rotation-degrees', '33.5', '--out', outside], reject=True, parse=False)
        check('outside rotation leaves no output', not outside.exists())
        existing = directory / report['outputs'][0]['name']
        digest = sha(existing)
        invoke('refuse-overwrite', [*base_arguments, '--rotation-degrees', '10', '--overwrite', '--out', existing], reject=True, parse=False)
        check('existing output preserved', sha(existing) == digest)
        check('input PDF unchanged', sha(source) == source_hash)
        report['status'] = 'passed'
    except Exception as error:
        report['status'], report['error'] = 'failed', str(error)
        raise
    finally:
        report['finished_utc'] = datetime.now(timezone.utc).isoformat()
        (directory / 'report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'status': report['status'], 'checks': len(report['checks']), 'commands': len(report['commands']),
                      'report': str(directory / 'report.json')}))


if __name__ == '__main__':
    main()
