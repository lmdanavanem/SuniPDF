#!/usr/bin/env python3
"""Prepare bundled guides with links to their exact release documentation."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import tempfile
import unittest
from urllib.parse import quote, unquote, urlsplit, urlunsplit


GUIDES = ('USER_GUIDE.en.md', 'USER_GUIDE.fr.md')
REPOSITORY = 'https://github.com/lmdanavanem/SuniPDF'
MAX_GUIDE_BYTES = 4 * 1024 * 1024
INLINE_LINK = re.compile(
    r'(?P<before>!?\[[^\]\n]*\]\(\s*)'
    r'(?P<url><[^>\n]*>|[^\s()]+)'
    r'(?P<after>(?:\s+(?:"[^"\n]*"|\x27[^\x27\n]*\x27|\([^\n)]*\)))?\s*\))'
)
REFERENCE_LINK = re.compile(
    r'^(?P<before> {0,3}\[[^\]\n]+\]:\s*)'
    r'(?P<url><[^>\n]*>|\S+)(?P<after>.*)$'
)
INLINE_CODE = re.compile(r'(`+).*?\1')
FENCE = re.compile(r'^ {0,3}(`{3,}|~{3,})')


def validate_tag(tag: str) -> None:
    if not re.fullmatch(r'v(?:0|[1-9][0-9]{0,8})\.(?:0|[1-9][0-9]{0,8})\.(?:0|[1-9][0-9]{0,8})', tag):
        raise ValueError('An exact stable release tag such as v0.43.0 is required.')


def rewrite_guide(text: str, source: Path, root: Path, tag: str) -> tuple[str, int]:
    """Rewrite file links outside code; reject absent or out-of-repository targets."""
    validate_tag(tag)
    root = root.resolve(strict=True)
    source = source.resolve(strict=True)
    if not source.is_relative_to(root):
        raise ValueError('The guide source must be inside the repository.')
    count = 0

    def replace_link(match: re.Match[str]) -> str:
        nonlocal count
        raw = match['url']
        angled = raw.startswith('<') and raw.endswith('>')
        url = raw[1:-1] if angled else raw
        if url.startswith('#') or url.startswith('//'):
            return match[0]
        if '\\' in url or re.match(r'^[A-Za-z]:', url):
            raise ValueError(f'Local filesystem link is not portable: {url}')
        parsed = urlsplit(url)
        if parsed.scheme:
            if parsed.scheme.lower() == 'file':
                raise ValueError(f'Local filesystem link is not portable: {url}')
            return match[0]
        if not parsed.path:
            return match[0]
        decoded = unquote(parsed.path)
        if decoded.startswith('/') or '\\' in decoded or '\x00' in decoded:
            raise ValueError(f'Absolute or invalid local documentation link: {url}')
        target = (source.parent / decoded).resolve(strict=True)
        if not target.is_relative_to(root):
            raise ValueError(f'Documentation link leaves the repository: {url}')
        relative = quote(target.relative_to(root).as_posix(), safe='/')
        destination = urlunsplit(('', '', f'{REPOSITORY}/blob/{tag}/{relative}', parsed.query, parsed.fragment))
        if angled:
            destination = '<' + destination + '>'
        count += 1
        return match['before'] + destination + match['after']

    def replace_prose(line: str) -> str:
        if REFERENCE_LINK.match(line):
            return REFERENCE_LINK.sub(replace_link, line)
        parts = []
        position = 0
        for code in INLINE_CODE.finditer(line):
            parts.append(INLINE_LINK.sub(replace_link, line[position:code.start()]))
            parts.append(code[0])
            position = code.end()
        parts.append(INLINE_LINK.sub(replace_link, line[position:]))
        return ''.join(parts)

    output = []
    active_fence = None
    for line in text.splitlines(keepends=True):
        fence = FENCE.match(line)
        if active_fence:
            output.append(line)
            if fence and fence[1][0] == active_fence[0] and len(fence[1]) >= len(active_fence) and not line[fence.end():].strip():
                active_fence = None
        elif fence:
            active_fence = fence[1]
            output.append(line)
        elif line.startswith('    ') or line.startswith('\t'):
            output.append(line)
        else:
            output.append(replace_prose(line))
    return ''.join(output), count


def prepare_guides(root: Path, stage: Path, tag: str) -> dict[str, int]:
    validate_tag(tag)
    root = root.resolve(strict=True)
    if stage.is_symlink():
        raise ValueError('The package stage must not be a symbolic link.')
    stage = stage.resolve(strict=True)
    if not stage.is_dir():
        raise ValueError('The package stage must be an existing directory.')
    prepared = []
    for name in GUIDES:
        source = root / 'docs' / name
        destination = stage / name
        if destination.is_symlink() or destination.resolve() == source.resolve():
            raise ValueError('Packaged guides must be separate copies of the repository sources.')
        if source.stat().st_size > MAX_GUIDE_BYTES:
            raise ValueError(f'The guide exceeds {MAX_GUIDE_BYTES} bytes: {name}')
        rendered, count = rewrite_guide(source.read_bytes().decode('utf-8'), source, root, tag)
        prepared.append((destination, rendered.encode('utf-8'), count))
    # Validate both documents before replacing either staged copy.
    for destination, payload, _ in prepared:
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(dir=stage, prefix='.guide-', suffix='.tmp', delete=False) as output:
                temporary = Path(output.name)
                output.write(payload)
            os.replace(temporary, destination)
        finally:
            if temporary is not None and temporary.exists():
                temporary.unlink()
    return {destination.name: count for destination, _, count in prepared}


class GuideTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.root = self.base / 'repo'
        self.docs = self.root / 'docs'
        self.docs.mkdir(parents=True)
        self.stage = self.base / 'package'
        self.stage.mkdir()
        self.source = self.docs / GUIDES[0]
        self.source.write_text('# Guide\n', encoding='utf-8')
        (self.docs / GUIDES[1]).write_text('# Guide FR\n', encoding='utf-8')
        (self.docs / 'MEASUREMENTS.md').write_text('# Measurements\n', encoding='utf-8')
        (self.root / 'CHANGELOG.md').write_text('# Changes\n', encoding='utf-8')

    def render(self, text: str, tag: str = 'v0.43.0') -> tuple[str, int]:
        return rewrite_guide(text, self.source, self.root, tag)

    def test_relative_links_titles_fragments_and_unicode(self) -> None:
        (self.docs / 'Mesures été.md').write_text('# Mesures\n', encoding='utf-8')
        text = '[M](MEASUREMENTS.md#cli "Read")\r\n[C](../CHANGELOG.md)\r\n[F](<Mesures%20%C3%A9t%C3%A9.md#données>)\r\n'
        result, count = self.render(text)
        self.assertEqual(count, 3)
        self.assertEqual(result.count('\r\n'), 3)
        self.assertIn(f'{REPOSITORY}/blob/v0.43.0/docs/MEASUREMENTS.md#cli "Read"', result)
        self.assertIn(f'{REPOSITORY}/blob/v0.43.0/CHANGELOG.md', result)
        self.assertIn('/docs/Mesures%20%C3%A9t%C3%A9.md#données>', result)

    def test_external_anchors_and_code_are_unchanged(self) -> None:
        text = '[W](https://example.com/a) [E](mailto:test@example.com) [A](#local)\n`[code](missing.md)`\n```md\n[code](missing.md)\n```\n~~~\n[code](missing.md)\n~~~\n    [code](missing.md)\n'
        self.assertEqual(self.render(text), (text, 0))

    def test_reference_links(self) -> None:
        result, count = self.render('[scale]: MEASUREMENTS.md#cli "Details"\n')
        self.assertEqual(count, 1)
        self.assertEqual(result, f'[scale]: {REPOSITORY}/blob/v0.43.0/docs/MEASUREMENTS.md#cli "Details"\n')

    def test_invalid_or_escaping_links_fail(self) -> None:
        (self.base / 'outside.md').write_text('# Outside\n', encoding='utf-8')
        for target in ('missing.md', '../../outside.md', 'file:///C:/data.md', 'C:/data.md', '/absolute.md', '%2Fabsolute.md'):
            with self.subTest(target=target), self.assertRaises((ValueError, FileNotFoundError)):
                self.render(f'[bad]({target})')

    def test_invalid_tags_fail(self) -> None:
        for tag in ('main', '../v0.43.0', 'v0.43', 'v0.43.0/bad', 'v0.43.0-beta', 'v00.43.0'):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                self.render('[M](MEASUREMENTS.md)', tag)

    def test_prepare_leaves_sources_and_payload_names_unchanged(self) -> None:
        for name in GUIDES:
            (self.docs / name).write_bytes(b'\xef\xbb\xbf[M](MEASUREMENTS.md)\r\n')
        before = {name: (self.docs / name).read_bytes() for name in GUIDES}
        counts = prepare_guides(self.root, self.stage, 'v0.43.0')
        self.assertEqual(counts, {name: 1 for name in GUIDES})
        self.assertEqual({p.name for p in self.stage.iterdir()}, set(GUIDES))
        for name in GUIDES:
            self.assertEqual((self.docs / name).read_bytes(), before[name])
            self.assertIn(b'/blob/v0.43.0/docs/MEASUREMENTS.md', (self.stage / name).read_bytes())
            self.assertTrue((self.stage / name).read_bytes().startswith(b'\xef\xbb\xbf'))

    def test_validation_failure_preserves_both_staged_files(self) -> None:
        for name in GUIDES:
            (self.stage / name).write_bytes(b'original copy')
        (self.docs / GUIDES[1]).write_text('[bad](missing.md)', encoding='utf-8')
        with self.assertRaises(FileNotFoundError):
            prepare_guides(self.root, self.stage, 'v0.43.0')
        for name in GUIDES:
            self.assertEqual((self.stage / name).read_bytes(), b'original copy')

    def test_source_directory_cannot_be_stage(self) -> None:
        before = self.source.read_bytes()
        with self.assertRaises(ValueError):
            prepare_guides(self.root, self.docs, 'v0.43.0')
        self.assertEqual(self.source.read_bytes(), before)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--stage', type=Path)
    parser.add_argument('--tag')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(GuideTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    if args.source_root is None or args.stage is None or args.tag is None:
        parser.error('--source-root, --stage and --tag are required.')
    try:
        counts = prepare_guides(args.source_root, args.stage, args.tag)
    except (OSError, UnicodeError, ValueError) as error:
        parser.exit(1, f'Guide preparation failed: {error}\n')
    for name, count in counts.items():
        print(f'{name}: {count} links pinned to {args.tag}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
