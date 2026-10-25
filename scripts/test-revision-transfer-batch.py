"""Exercise real multi-file transfer commands and independently inspect saved PDFs with QPDF."""
import argparse
import base64
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import subprocess
from datetime import datetime, timezone


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def same_pdf_rgb(left, right):
    """Require identical native binary32 RGB values, despite decimal spelling."""
    return (isinstance(left, list) and isinstance(right, list)
            and len(left) == len(right) == 3
            and all(type(a) in (int, float) and type(b) in (int, float)
                    and 0 <= a <= 1 and 0 <= b <= 1
                    and struct.pack('!f', a) == struct.pack('!f', b)
                    for a, b in zip(left, right)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--qpdf', type=Path, required=True)
    parser.add_argument('--directory', type=Path, required=True)
    args = parser.parse_args()
    cli, qpdf = args.cli.resolve(strict=True), args.qpdf.resolve(strict=True)
    directory = args.directory.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    report = {'version': 1, 'status': 'running', 'started_utc': datetime.now(timezone.utc).isoformat(),
              'cli_sha256': sha(cli), 'qpdf_sha256': sha(qpdf), 'checks': [], 'commands': [],
              'scope': 'Synthetic CLI batch creation and independent saved-output checks; no desktop interaction or private-plan claim.'}
    spec = importlib.util.spec_from_file_location('fixture', Path(__file__).with_name('native-plan-link-fixture.py'))
    fixture = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixture)

    def check(name, condition):
        report['checks'].append({'name': name, 'passed': bool(condition)})
        if not condition:
            raise AssertionError(name)

    def execute(name, command, reject=False, parse=True):
        result = subprocess.run([str(value) for value in command], capture_output=True,
                                stdin=subprocess.DEVNULL, timeout=120, cwd=directory, env=os.environ.copy())
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
            json.dump(value, stream, ensure_ascii=False, indent=2)
            stream.write('\n')
        return path

    def inspect_pdf(name, path):
        execute(name + '-check', [qpdf, '--check', path], parse=False)
        value = execute(name + '-json', [qpdf, '--json', '--json-stream-data=inline', path])
        objects = value['qpdf'][1]
        pages = []
        for page in value['pages']:
            details = objects['obj:' + page['object']]['value']
            annotations = [objects['obj:' + ref]['value'] if isinstance(ref, str) else ref
                           for ref in details.get('/Annots', [])]
            streams = [base64.b64decode(objects['obj:' + ref]['stream']['data'], validate=True)
                       for ref in page['contents']]
            pages.append({'details': details, 'annotations': annotations, 'content': b''.join(streams)})
        return pages

    try:
        report['cli_version'] = call('version', '--version', parse=False).decode().strip()
        report['qpdf_version'] = execute('qpdf-version', [qpdf, '--version'], parse=False).decode().strip()
        style = {'rgb': [190, 40, 40], 'fill_rgb': None, 'width_pt': 1.5, 'opacity': 1.0, 'dash_pattern': []}
        library = write('library.json', {'format': 'sunipdf-annotation-library', 'version': 1, 'entries': [
            {'id': identity, 'name': identity,
             'provenance': {'author': 'SuniPDF', 'source': 'Original synthetic test drawing', 'license': 'CC0-1.0'},
             'primitives': [{'kind': 'shape', 'shape': shape, 'rect': [0, 0, 80, 40], 'style': style}]}
            for identity, shape in [('transfer', 'square'), ('existing', 'circle')]
        ]})
        items, protected = [], {}
        for number in [1, 2]:
            old_blank, new_blank = directory / f'old-blank-{number}.pdf', directory / f'new-blank-{number}.pdf'
            old_blank.write_bytes(fixture.pdf([f'BT /F1 20 Tf 72 700 Td (Old plan {number}) Tj ET\n'.encode()]))
            new_blank.write_bytes(fixture.pdf([f'BT /F1 20 Tf 72 700 Td (New plan {number}) Tj ET\n'.encode()]))
            old, new = directory / f'old-{number}.pdf', directory / f'new-{number}.pdf'
            for source, output, entry, origin in [(old_blank, old, 'transfer', '60,80'),
                                                   (new_blank, new, 'existing', '300,400')]:
                call(f'prepare-{entry}-{number}', 'annotation-library', 'place', library, '--id', entry,
                     '--file', source, '--page', '1', '--origin', origin, '--scale', '1',
                     '--author', 'Synthetic reviewer', '--out', output)
            protected.update({old: sha(old), new: sha(new)})
            inspected = call(f'inspect-{number}', 'revision-transfer', 'inspect', old, new, '--pages', '1:1')
            candidates = inspected['pages'][0]['candidates']
            check(f'pair {number}: supported native source shape', len(candidates) == 1
                  and candidates[0]['fingerprint'] and candidates[0]['refusal'] is None)
            recipe = write(f'review-{number}.json', {'version': 1, 'old_sha256': sha(old), 'new_sha256': sha(new),
                'acknowledge_repeated_transfer': False, 'pairs': [
                    {'old_page': 1, 'new_page': 1, 'old_sheet_id': None, 'new_sheet_id': None,
                     'confirmed': True, 'alignment': {'mode': 'identity'},
                     'selections': [{'index': candidates[0]['index'], 'fingerprint': candidates[0]['fingerprint']}]}]})
            items.append({'id': f'pair-{number}', 'old': old.name, 'new': new.name,
                          'recipe': recipe.name, 'out': f'reviewed-{number}.pdf'})
        manifest = write('batch.json', {'version': 1, 'items': items})
        preview = call('batch-preview', 'revision-transfer', 'batch-preview', '--manifest', manifest)
        check('preflight prepares both pairs without creating outputs', preview['complete'] and not preview['published']
              and len(preview['items']) == 2 and all(not (directory / item['out']).exists() for item in items))
        applied = call('batch-apply', 'revision-transfer', 'batch-apply', '--manifest', manifest)
        outcome = applied['outcome']
        check('both outputs have committed receipts', applied['complete'] and applied['published']
              and outcome['status'] == 'complete' and len(outcome['completed']) == 2
              and not outcome['remaining'] and outcome['failure'] is None)
        identities = set()
        for number, item in enumerate(items, 1):
            output = directory / item['out']
            receipt = next(row['report'] for row in outcome['completed'] if row['id'] == item['id'])
            check(f'pair {number}: receipt output hash', receipt['output_sha256'] == sha(output))
            old = inspect_pdf(f'old-{number}', directory / item['old'])[0]
            new = inspect_pdf(f'new-{number}', directory / item['new'])[0]
            saved_pages = inspect_pdf(f'saved-{number}', output)
            check(f'pair {number}: page count preserved', len(saved_pages) == 1)
            saved = saved_pages[0]
            check(f'pair {number}: incoming content retained', saved['content'] == new['content'])
            check(f'pair {number}: incoming geometry retained', all(saved['details'].get(key) == new['details'].get(key)
                  for key in ['/MediaBox', '/CropBox', '/Rotate']))
            check(f'pair {number}: existing plus transferred annotations', len(saved['annotations']) == 2)
            for key in ['/Subtype', '/Rect', '/Contents', '/T', '/NM', '/C']:
                actual, expected = saved['annotations'][0].get(key), new['annotations'][0].get(key)
                check(f'pair {number}: original annotation {key}',
                      same_pdf_rgb(actual, expected) if key == '/C' else actual == expected)
            copied = saved['annotations'][1]
            for key in ['/Subtype', '/Rect', '/T', '/C']:
                actual, expected = copied.get(key), old['annotations'][0].get(key)
                check(f'pair {number}: transferred annotation {key}',
                      same_pdf_rgb(actual, expected) if key == '/C' else actual == expected)
            check(f'pair {number}: fresh annotation identity', copied['/NM'] != old['annotations'][0]['/NM']
                  and copied['/NM'] not in identities)
            identities.add(copied['/NM'])

        def refusal(name, modified):
            path = write(name + '.json', {'version': 1, 'items': modified})
            before = {file: sha(file) for file in directory.glob('*.pdf')}
            call(name, 'revision-transfer', 'batch-apply', '--manifest', path, reject=True, parse=False)
            check(name + ': no PDF created or changed', before == {file: sha(file) for file in directory.glob('*.pdf')})

        existing = copy.deepcopy(items)
        existing[0]['out'] = 'must-not-publish-before-existing-refusal.pdf'
        refusal('refuse-existing-later-output', existing)
        duplicate = copy.deepcopy(items)
        duplicate[0]['out'] = duplicate[1]['out'] = 'duplicate-output.pdf'
        refusal('refuse-duplicate-output', duplicate)
        alias = copy.deepcopy(items)
        alias[0]['out'] = items[1]['new']
        alias[1]['out'] = 'source-alias-other.pdf'
        refusal('refuse-cross-source-output', alias)
        stale = copy.deepcopy(items)
        stale[0]['out'], stale[1]['out'] = 'stale-first.pdf', 'stale-second.pdf'
        stale_recipe = json.loads((directory / stale[1]['recipe']).read_text(encoding='utf-8'))
        stale_recipe['new_sha256'] = '0' * 64
        stale[1]['recipe'] = write('stale-review.json', stale_recipe).name
        refusal('refuse-stale-later-recipe', stale)
        refusal('refuse-empty-batch', [])
        check('all original source PDFs unchanged', all(sha(path) == digest for path, digest in protected.items()))
        report['sources'] = [{'name': path.name, 'sha256': digest} for path, digest in protected.items()]
        report['outputs'] = [{'name': item['out'], 'sha256': sha(directory / item['out'])} for item in items]
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
