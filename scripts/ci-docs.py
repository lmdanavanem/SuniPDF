#!/usr/bin/env python3
"""Conservatively classify CI documentation changes and validate repository guides."""

from __future__ import annotations

import argparse
from collections import Counter
import csv
import importlib.util
import io
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tempfile


ROOT_DOCS = frozenset({
    'README.md', 'CHANGELOG.md', 'CONTRIBUTING.md',
    'CODE_OF_CONDUCT.md', 'SECURITY.md',
})
EXCLUDED_PARTS = frozenset({
    'test', 'tests', 'fixture', 'fixtures', 'corpus', 'corpora',
    'sample', 'samples', 'example', 'examples', 'testdata', 'test-data',
    'test_fixtures', 'script', 'scripts', 'assets', 'vendor',
})
BACKLOG_COLUMNS = [
    'id', 'priority', 'capability', 'acceptance_criteria', 'dependencies',
    'status', 'implementation_progress',
]
SHA = re.compile(r'[0-9a-f]{40}')
VERSION = re.compile(r'(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)')
MAX_BYTES = 8 * 1024 * 1024


def documentation_path(path: str) -> bool:
    """Only ordinary product documentation can bypass application checks."""
    if any(ord(char) < 32 for char in path) or '\\' in path:
        return False
    parts = path.split('/')
    if any(part in ('', '.', '..') or part.startswith('.') for part in parts):
        return False
    if path in ROOT_DOCS:
        return True
    if parts[0] != 'docs' or any(part.lower() in EXCLUDED_PARTS for part in parts[1:-1]):
        return False
    if path == 'docs/AEC_BACKLOG.csv':
        return True
    if len(parts) == 3 and parts[1] == 'qa' and parts[2].endswith('.json'):
        return True
    return path.endswith('.md')


def git(root: Path, *args: str) -> bytes:
    return subprocess.run(
        ['git', '-C', str(root), *args], check=True, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, timeout=60,
    ).stdout


def parse_changes(raw: bytes) -> list[dict[str, str]]:
    """Read NUL-separated raw Git records without rename heuristics or quoting."""
    fields = raw.split(b'\0')
    if fields[-1] != b'' or (len(fields) - 1) % 2:
        raise ValueError('Malformed Git diff records.')
    changes = []
    for index in range(0, len(fields) - 1, 2):
        metadata = fields[index].decode('ascii').split()
        if len(metadata) != 5 or not re.fullmatch(r':[0-7]{6}', metadata[0]):
            raise ValueError('Malformed Git diff metadata.')
        old_mode, new_mode, old_object, new_object, status = metadata
        if not re.fullmatch(r'[0-7]{6}', new_mode) or not SHA.fullmatch(old_object) or not SHA.fullmatch(new_object):
            raise ValueError('Malformed Git modes or object IDs.')
        path = fields[index + 1].decode('utf-8', errors='strict')
        changes.append({'path': path, 'status': status, 'old_mode': old_mode[1:], 'new_mode': new_mode})
    return changes


def classify_changes(changes: list[dict[str, str]]) -> bool:
    return bool(changes) and all(
        documentation_path(change['path'])
        and change['new_mode'] == '100644'
        and (
            (change['status'] == 'A' and change['old_mode'] == '000000')
            or (change['status'] == 'M' and change['old_mode'] == '100644')
        )
        for change in changes
    )


def exact_diff(root: Path, event_name: str, event: dict) -> dict:
    """Bind the classification to the exact checked-out event; never guess refs."""
    current = git(root, 'rev-parse', 'HEAD').decode('ascii').strip()
    if event_name == 'pull_request':
        base = event['pull_request']['base']['sha']
        head = event['pull_request']['head']['sha']
    elif event_name == 'push' and event.get('ref') == 'refs/heads/main':
        base, head = event['before'], event['after']
    else:
        raise ValueError('Only pull requests and main pushes have an eligible exact diff.')
    if not all(isinstance(value, str) and SHA.fullmatch(value) and value != '0' * 40 for value in (base, head)):
        raise ValueError('Missing or invalid event revisions.')
    for revision in (base, head):
        if git(root, 'rev-parse', '--verify', revision + '^{commit}').decode('ascii').strip() != revision:
            raise ValueError('The event revision is not an available commit.')
    if event_name == 'pull_request':
        parents = git(root, 'show', '-s', '--format=%P', 'HEAD').decode('ascii').strip().split()
        if parents != [base, head]:
            raise ValueError('Checkout does not match the exact pull request merge parents.')
        comparison_base = git(root, 'merge-base', base, head).decode('ascii').strip()
        if not SHA.fullmatch(comparison_base):
            raise ValueError('An exact merge base is unavailable.')
    else:
        if current != head or event.get('forced') is not False or event.get('deleted') is not False:
            raise ValueError('Checkout or push ancestry is not an ordinary main update.')
        git(root, 'merge-base', '--is-ancestor', base, head)
        comparison_base = base
    changes = parse_changes(git(root, 'diff', '--raw', '--no-abbrev', '-z', '--no-renames', comparison_base, head, '--'))
    return {
        'docs_only': classify_changes(changes), 'event': event_name,
        'base': base, 'head': head, 'comparison_base': comparison_base,
        'checkout': current, 'changes': changes,
        'reason': 'Exact event diff inspected; only regular added or modified allowlisted documentation is eligible.',
    }


def classify_event(root: Path, event_name: str, event_path: Path) -> dict:
    try:
        event = json.loads(event_path.read_text(encoding='utf-8'))
        return exact_diff(root, event_name, event)
    except Exception as error:
        # A failed classification must retain application checks, not silently skip them.
        return {'docs_only': False, 'event': event_name, 'changes': [],
                'reason': f'Full checks required: {type(error).__name__}.'}


def load_guides(root: Path):
    specification = importlib.util.spec_from_file_location('packaged_guides', root / 'scripts' / 'package-guides.py')
    if specification is None or specification.loader is None:
        raise ValueError('Packaged-guide validator is unavailable.')
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def bounded_text(path: Path) -> str:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_BYTES:
        raise ValueError(f'Document must be a bounded regular file: {path.name}')
    return path.read_text(encoding='utf-8-sig')


def headings(text: str) -> Counter:
    """Count major ATX headings outside fenced/indented code, including BOM input."""
    result = Counter()
    fence = None
    for line in text.lstrip('\ufeff').splitlines():
        match = re.match(r'^ {0,3}(`{3,}|~{3,})', line)
        if fence:
            if match and match[1][0] == fence[0] and len(match[1]) >= len(fence) and not line[match.end():].strip():
                fence = None
        elif match:
            fence = match[1]
        else:
            heading = re.match(r'^ {0,3}(#{1,3})\s+(.+?)\s*#*\s*$', line)
            if heading:
                result[(len(heading[1]), ' '.join(heading[2].casefold().split()))] += 1
    return result


def validate_headings(text: str, previous: str = '') -> None:
    old = headings(previous)
    for heading, count in headings(text).items():
        if count > max(1, old[heading]):
            raise ValueError(f'New duplicate major heading: {heading[1]}')


def validate_backlog(path: Path) -> int:
    reader = csv.DictReader(io.StringIO(bounded_text(path)), strict=True)
    if reader.fieldnames != BACKLOG_COLUMNS:
        raise ValueError('AEC backlog columns changed unexpectedly.')
    rows = list(reader)
    seen = set()
    for row in rows:
        if None in row or any(value is None for value in row.values()):
            raise ValueError('AEC backlog row has missing or extra columns.')
        key = row['id']
        if not re.fullmatch(r'AEC-[0-9]{3}', key) or key in seen:
            raise ValueError('AEC backlog IDs must be unique and well formed.')
        if row['priority'] not in ('P1', 'P2', 'P3') or row['status'] not in ('Planned', 'In progress', 'Done'):
            raise ValueError('AEC backlog priority or status is invalid.')
        if not row['capability'].strip() or not row['acceptance_criteria'].strip():
            raise ValueError('AEC backlog capability and acceptance criteria are required.')
        seen.add(key)
    if not rows:
        raise ValueError('AEC backlog is empty.')
    return len(rows)


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'Duplicate evidence JSON key: {key}')
        result[key] = value
    return result


def reject_constant(value: str):
    raise ValueError(f'Non-finite JSON number: {value}')


def validate_evidence(path: Path) -> None:
    # Structural validation is not a substitute for the release publication verifier.
    value = json.loads(bounded_text(path), object_pairs_hook=unique_object, parse_constant=reject_constant)
    if not isinstance(value, dict) or not value:
        raise ValueError('Evidence JSON must be a nonempty object.')
    pending = [(value, 0)]
    while pending:
        item, depth = pending.pop()
        if depth > 64:
            raise ValueError('Evidence JSON nesting exceeds the supported limit.')
        if isinstance(item, float) and not math.isfinite(item):
            raise ValueError('Evidence JSON numbers must be finite.')
        if isinstance(item, dict):
            pending.extend((child, depth + 1) for child in item.values())
        elif isinstance(item, list):
            pending.extend((child, depth + 1) for child in item)
    if 'source' in value and (not isinstance(value['source'], str) or not SHA.fullmatch(value['source'])):
        raise ValueError('Evidence source must be an exact commit ID.')
    if 'version' in value and (not isinstance(value['version'], str) or not VERSION.fullmatch(value['version'])):
        raise ValueError('Evidence version must be an exact stable version.')
    version = re.search(r'_([0-9]+\.[0-9]+\.[0-9]+)\.json$', path.name)
    if version and 'version' in value and value['version'] != version[1]:
        raise ValueError('Evidence version does not match its filename.')
    if 'tag' in value and 'version' in value and value['tag'] != 'v' + value['version']:
        raise ValueError('Evidence tag does not match its version.')


def validate(root: Path, report: dict | None = None, all_markdown: bool = False) -> dict:
    root = root.resolve(strict=True)
    guide_module = load_guides(root)
    manifest = bounded_text(root / 'Cargo.toml')
    version = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"$', manifest, re.MULTILINE)
    if version is None:
        raise ValueError('The workspace version is missing.')
    tag = 'v' + version[1]
    with tempfile.TemporaryDirectory(prefix='sunipdf-guide-check-') as temporary:
        packaged = guide_module.prepare_guides(root, Path(temporary), tag)
        for name in guide_module.GUIDES:
            validate_headings(bounded_text(root / 'docs' / name))
            if not (Path(temporary) / name).is_file():
                raise ValueError('A packaged guide was not produced.')
    markdown = set()
    if all_markdown:
        markdown.update(path.relative_to(root).as_posix() for path in (root / 'docs').rglob('*.md'))
        markdown.update(name for name in ROOT_DOCS if (root / name).exists())
    changes = (report or {}).get('changes', [])
    markdown.update(change['path'] for change in changes if documentation_path(change['path']) and change['path'].endswith('.md') and change['status'] in ('A', 'M'))
    links = sum(packaged.values())
    for relative in sorted(markdown):
        source = root / PurePosixPath(relative)
        text = bounded_text(source)
        _, count = guide_module.rewrite_guide(text, source, root, tag)
        links += count
        if not all_markdown:
            previous = ''
            if (report or {}).get('comparison_base'):
                try:
                    previous = git(root, 'show', report['comparison_base'] + ':' + relative).decode('utf-8-sig')
                except subprocess.CalledProcessError:
                    # A newly added file has no prior headings to preserve.
                    if not any(change['path'] == relative and change['status'] == 'A' for change in changes):
                        raise
            validate_headings(text, previous)
    evidence = sorted((root / 'docs' / 'qa').glob('*.json'))
    for path in evidence:
        validate_evidence(path)
    return {'packaged_guides': packaged, 'tag': tag, 'markdown_files': len(markdown),
            'local_links': links, 'backlog_rows': validate_backlog(root / 'docs' / 'AEC_BACKLOG.csv'),
            'evidence_json_files': len(evidence)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('classify', 'validate'))
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--report', type=Path, default=Path('target/ci-docs-changes.json'))
    parser.add_argument('--all-markdown', action='store_true')
    args = parser.parse_args()
    if args.mode == 'classify':
        report = classify_event(args.root, os.environ.get('GITHUB_EVENT_NAME', ''), Path(os.environ.get('GITHUB_EVENT_PATH', '')))
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
        if os.environ.get('GITHUB_OUTPUT'):
            with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:
                output.write('docs_only=' + str(report['docs_only']).lower() + '\n')
        print(json.dumps(report))
    else:
        report = json.loads(args.report.read_text(encoding='utf-8')) if args.report.is_file() else None
        print(json.dumps(validate(args.root, report, args.all_markdown), indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
