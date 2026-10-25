"""Check saved stamp/image library capture with the packaged CLI and independent PDF inspection."""
import argparse
import base64
import copy
import hashlib
import importlib.util
from io import BytesIO
import json
import os
from pathlib import Path
import re
import subprocess
from datetime import datetime, timezone

from PIL import Image


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--qpdf', type=Path, required=True)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--expected-version', required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'0\.\d+\.0', args.expected_version):
        parser.error('expected version must be an explicit 0.x.0 release')
    cli, qpdf = args.cli.resolve(strict=True), args.qpdf.resolve(strict=True)
    directory = args.directory.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    report = {'version': 1, 'status': 'running', 'source_revision': os.environ.get('GITHUB_SHA'),
              'started_utc': datetime.now(timezone.utc).isoformat(), 'cli_sha256': sha(cli),
              'qpdf_sha256': sha(qpdf), 'checks': [], 'commands': [], 'outputs': [],
              'scope': 'Synthetic packaged CLI capture/reuse and independently decoded PDF appearance resources; no native desktop interaction claim.'}

    def check(name, passed):
        report['checks'].append({'name': name, 'passed': bool(passed)})
        if not passed:
            raise AssertionError(name)

    def execute(name, command, reject=False, parse=True):
        result = subprocess.run([str(value) for value in command], cwd=directory, stdin=subprocess.DEVNULL,
                                capture_output=True, timeout=120)
        for stream in ['stdout', 'stderr']:
            (directory / f'{name}.{stream}').write_bytes(getattr(result, stream))
        report['commands'].append({'name': name, 'exit_code': result.returncode})
        check(name + ': process outcome', result.returncode != 0 if reject else result.returncode == 0)
        return json.loads(result.stdout) if parse and not reject else result.stdout

    def call(name, *arguments, **kwargs):
        return execute(name, [cli, *arguments], **kwargs)

    def write(name, value):
        path = directory / name
        with path.open('x', encoding='utf-8') as stream:
            json.dump(value, stream, indent=2)
            stream.write('\n')
        return path

    def inspect(name, path):
        execute(name + '-check', [qpdf, '--check', path], parse=False)
        value = execute(name + '-json', [qpdf, '--json', '--json-stream-data=inline', path])
        objects = value['qpdf'][1]

        def canonical(value, depth=0):
            if depth > 32:
                raise AssertionError('appearance graph exceeded inspection depth')
            if isinstance(value, str) and re.fullmatch(r'\d+ \d+ R', value):
                entry = objects['obj:' + value]
                if 'stream' in entry:
                    stream = entry['stream']
                    return {'dictionary': canonical({key: item for key, item in stream['dict'].items()
                                                     if key not in ['/Length', '/Filter', '/DecodeParms']}, depth + 1),
                            'decoded': base64.b64decode(stream['data'], validate=True).hex()}
                return canonical(entry['value'], depth + 1)
            if isinstance(value, dict):
                return {key: canonical(item, depth + 1) for key, item in value.items()}
            if isinstance(value, list):
                return [canonical(item, depth + 1) for item in value]
            return value

        page = objects['obj:' + value['pages'][0]['object']]['value']
        annotations = [objects['obj:' + ref]['value'] for ref in page.get('/Annots', [])]
        content = b''.join(base64.b64decode(objects['obj:' + ref]['stream']['data'], validate=True)
                           for ref in value['pages'][0]['contents'])
        return page, annotations, [canonical(annotation['/AP']) for annotation in annotations], content

    try:
        version = call('version', '--version', parse=False).decode().strip()
        report['cli_version'] = version
        check('exact CLI version', version == 'sunipdf-cli ' + args.expected_version)
        spec = importlib.util.spec_from_file_location('fixture', Path(__file__).with_name('native-plan-link-fixture.py'))
        fixture = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(fixture)
        blank = directory / 'blank.pdf'
        blank.write_bytes(fixture.pdf([b'BT /F1 18 Tf 72 700 Td (Original saved-stamp fixture) Tj ET\n']))
        blank_sha = sha(blank)
        pixels = Image.new('RGBA', (12, 8), (240, 30, 40, 180))
        pixels.putpixel((0, 0), (0, 0, 0, 0))
        pixels.putpixel((1, 0), (20, 180, 40, 255))
        buffer = BytesIO()
        pixels.save(buffer, format='PNG')
        encoded = buffer.getvalue()
        embedded = {'png_base64': base64.b64encode(encoded).decode(), 'width': 12, 'height': 8,
                    'sha256': hashlib.sha256(encoded).hexdigest()}
        style = {'rgb': [200, 30, 40], 'fill_rgb': None, 'width_pt': 1.5, 'opacity': 0.8, 'dash_pattern': []}
        library = write('original-library.json', {'format': 'sunipdf-annotation-library', 'version': 1, 'entries': [{
            'id': 'review', 'name': 'Synthetic mixed symbol',
            'provenance': {'author': 'Test fixture', 'source': 'Original synthetic artwork', 'license': 'CC0-1.0'},
            'primitives': [
                {'kind': 'shape', 'shape': 'square', 'rect': [0, 0, 80, 40], 'style': style},
                {'kind': 'stamp', 'stamp': 'approved', 'rect': [90, 0, 170, 40]},
                {'kind': 'image', 'rect': [180, 0, 240, 60], 'png': embedded, 'opacity': 0.5}
            ]}]})
        source = directory / 'saved-source.pdf'
        call('place-original', 'annotation-library', 'place', library, '--id', 'review', '--file', blank,
             '--page', 1, '--origin', '40,60', '--scale', 1, '--out', source)
        source_sha = sha(source)
        inventory = call('inventory', 'annotation-library', 'capture-candidates', source, '--page', 1)
        rows = inventory['annotations']
        check('all three native members selectable', len(rows) == 3 and all(isinstance(row.get('fingerprint'), str)
              and re.fullmatch(r'[a-f0-9]{64}', row['fingerprint']) and row.get('refusal') is None for row in rows))
        selections = [part for index, row in enumerate(rows) for part in ['--annotation', f'{index}:{row["fingerprint"]}']]
        captured = directory / 'captured-library.json'
        capture_args = ['annotation-library', 'capture', source, '--page', 1, '--id', 'review', '--name', 'Captured mixed symbol',
                        '--source', 'Original synthetic artwork', '--credit', 'Test fixture', '--license', 'CC0-1.0']
        result = call('capture', *capture_args, *selections, '--out', captured)
        check('capture receipt', result['count'] == 3 and result['original_unchanged'] is True)
        value = json.loads(captured.read_text(encoding='utf-8'))
        entry = value['entries'][0]
        check('portable library version', value['format'] == 'sunipdf-annotation-library' and value['version'] == 1)
        check('mixed member types', [item['kind'] for item in entry['primitives']] == ['shape', 'stamp', 'image'])
        check('explicit provenance retained', entry['provenance'] == {'source': 'Original synthetic artwork', 'author': 'Test fixture', 'license': 'CC0-1.0'})
        check('no private PDF path copied', str(source) not in captured.read_text(encoding='utf-8'))
        saved_image = entry['primitives'][2]
        check('already-applied opacity is not repeated', saved_image['opacity'] == 1)
        png_bytes = base64.b64decode(saved_image['png']['png_base64'], validate=True)
        check('embedded PNG hash', hashlib.sha256(png_bytes).hexdigest() == saved_image['png']['sha256'])
        with Image.open(BytesIO(png_bytes)) as image:
            image.load()
            rgba = image.convert('RGBA')
            check('embedded dimensions', rgba.size == (12, 8))
            expected = [(0, 0, 0, 0) if alpha == 0 else (red, green, blue, int(alpha * .5 + .5))
                        for red, green, blue, alpha in pixels.getdata()]
            check('every saved RGB and alpha pixel', list(rgba.getdata()) == expected)
        reused = directory / 'reused.pdf'
        call('reuse', 'annotation-library', 'place', captured, '--id', 'review', '--file', blank,
             '--page', 1, '--origin', '40,60', '--scale', 1, '--out', reused)
        old_page, old_annots, old_appearances, old_content = inspect('source', source)
        new_page, new_annots, new_appearances, new_content = inspect('reused', reused)
        check('source content retained', old_content == new_content)
        check('source geometry retained', all(old_page.get(key) == new_page.get(key) for key in ['/MediaBox', '/CropBox', '/Rotate']))
        check('three independent annotations reused', len(old_annots) == len(new_annots) == 3)
        for index, (old, new, old_ap, new_ap) in enumerate(zip(old_annots, new_annots, old_appearances, new_appearances)):
            check(f'member {index}: fresh identity', old['/NM'] != new['/NM'])
            check(f'member {index}: same type and geometry', all(old.get(key) == new.get(key) for key in ['/Subtype', '/Name', '/Rect']))
            check(f'member {index}: identical independently decoded appearance', old_ap == new_ap)
        refused = directory / 'refused.json'
        call('refuse-stale', *capture_args, '--annotation', '0:' + '0' * 64, '--out', refused, reject=True, parse=False)
        check('stale capture creates no library', not refused.exists())
        captured_sha = sha(captured)
        call('refuse-capture-overwrite', *capture_args, *selections, '--out', captured, '--overwrite', reject=True, parse=False)
        check('existing library retained', sha(captured) == captured_sha)
        reused_sha = sha(reused)
        call('refuse-placement-overwrite', 'annotation-library', 'place', captured, '--id', 'review', '--file', blank,
             '--page', 1, '--origin', '40,60', '--scale', 1, '--out', reused, '--overwrite', reject=True, parse=False)
        check('existing reused PDF retained', sha(reused) == reused_sha)
        changed = copy.deepcopy(value)
        changed['entries'][0]['primitives'][2]['png']['sha256'] = '0' * 64
        invalid_library = write('invalid-image-library.json', changed)
        call('refuse-corrupt-image', 'annotation-library', 'inspect', invalid_library, reject=True, parse=False)
        check('original blank PDF unchanged', sha(blank) == blank_sha)
        check('captured source PDF unchanged', sha(source) == source_sha)
        report['sources'] = [{'name': path.name, 'sha256': digest} for path, digest in [(blank, blank_sha), (source, source_sha)]]
        report['outputs'] = [{'name': path.name, 'sha256': sha(path), 'bytes': path.stat().st_size} for path in [captured, reused]]
        report['status'] = 'passed'
    except Exception as error:
        report['status'], report['error'] = 'failed', str(error)
        raise
    finally:
        report['finished_utc'] = datetime.now(timezone.utc).isoformat()
        with (directory / 'report.json').open('x', encoding='utf-8') as stream:
            json.dump(report, stream, indent=2)
            stream.write('\n')
    print(json.dumps({'status': report['status'], 'checks': len(report['checks']), 'commands': len(report['commands'])}))


if __name__ == '__main__':
    main()
