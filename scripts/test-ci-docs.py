#!/usr/bin/env python3
"""Regression tests for the conservative documentation-only CI boundary."""

from __future__ import annotations

import csv
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location('ci_docs', Path(__file__).with_name('ci-docs.py'))
assert SPEC and SPEC.loader
DOCS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DOCS)


def record(path='docs/USER_GUIDE.en.md', status='M', old_mode='100644', new_mode='100644'):
    return {'path': path, 'status': status, 'old_mode': old_mode, 'new_mode': new_mode}


class ClassificationTests(unittest.TestCase):
    def test_only_explicit_document_paths_are_eligible(self):
        for path in ['README.md', 'README.fr.md', 'CHANGELOG.en.md', 'SECURITY.md',
                     'docs/USER_GUIDE.en.md', 'docs/nested/guide.md',
                     'docs/qa/RELEASE_0.47.0.json', 'docs/AEC_BACKLOG.csv']:
            with self.subTest(path=path):
                self.assertTrue(DOCS.classify_changes([record(path)]))
                self.assertTrue(DOCS.classify_changes([record(path, 'A', '000000')]))
        for path in ['unknown.md', 'Cargo.toml', 'README.md.exe', '.github/PULL_REQUEST_TEMPLATE.md',
                     '.github/workflows/ci.yml', 'scripts/ci-docs.py', 'apps/app/README.md',
                     'docs/screenshots/image.png', 'docs/qa/nested/report.json', 'docs/other.csv',
                     'docs/tests/guide.md', 'docs/fixtures/guide.md', 'docs/corpus/README.md',
                     'docs/Scripts/README.md', 'docs/examples/README.md', 'docs/testdata/README.md',
                     'docs/.hidden/guide.md', 'docs/../README.md',
                     'docs//guide.md', 'docs\\guide.md', '/docs/guide.md', 'docs/new\nline.md']:
            with self.subTest(path=path):
                self.assertFalse(DOCS.classify_changes([record(path)]))

    def test_uncertain_changes_retain_full_suite(self):
        for change in [record(status='D'), record(status='R100'), record(status='C100'),
                       record(status='T'), record(status='U'), record(new_mode='120000'),
                       record(old_mode='120000'), record(new_mode='100755'),
                       record(old_mode='100755'), record(new_mode='160000')]:
            with self.subTest(change=change):
                self.assertFalse(DOCS.classify_changes([change]))
        self.assertFalse(DOCS.classify_changes([]))
        self.assertFalse(DOCS.classify_changes([record(), record('apps/main.rs')]))

    def test_raw_diff_is_exact_and_nul_delimited(self):
        raw = b':100644 100644 ' + b'a' * 40 + b' ' + b'b' * 40 + b' M\0docs/a b.md\0'
        self.assertEqual(DOCS.parse_changes(raw), [record('docs/a b.md')])
        self.assertEqual(DOCS.parse_changes(b''), [])
        for malformed in [raw[:-1], raw.replace(b'100644', b'100xx4'), raw + b'extra\0', raw.replace(b'M\0', b'M\xff\0')]:
            with self.subTest(malformed=malformed), self.assertRaises((ValueError, UnicodeError)):
                DOCS.parse_changes(malformed)

    def test_errors_never_enable_documentation_only(self):
        with tempfile.TemporaryDirectory() as temporary:
            event = Path(temporary) / 'event.json'
            for payload in ['{invalid', '{}', '[]', '{"pull_request": null}']:
                event.write_text(payload, encoding='utf-8')
                report = DOCS.classify_event(Path(temporary), 'pull_request', event)
                self.assertFalse(report['docs_only'])
            self.assertFalse(DOCS.classify_event(Path(temporary), 'push', event.with_name('absent'))['docs_only'])
            event.write_text('{}', encoding='utf-8')
            with patch.object(DOCS, 'exact_diff', side_effect=subprocess.TimeoutExpired('git', 60)):
                self.assertFalse(DOCS.classify_event(Path(temporary), 'push', event)['docs_only'])
            with patch.object(DOCS, 'exact_diff', side_effect=AttributeError('Unexpected event shape')):
                self.assertFalse(DOCS.classify_event(Path(temporary), 'push', event)['docs_only'])


class GitBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.run_git('init', '-b', 'main')
        self.run_git('config', 'user.name', 'Test')
        self.run_git('config', 'user.email', 'test@example.invalid')
        self.run_git('config', 'commit.gpgsign', 'false')
        (self.root / 'README.md').write_text('# Original\n', encoding='utf-8')
        self.commit('Initial')
        self.base = self.sha()

    def run_git(self, *args):
        return DOCS.git(self.root, *args).decode('utf-8').strip()

    def sha(self):
        return self.run_git('rev-parse', 'HEAD')

    def commit(self, message):
        self.run_git('add', '-A')
        self.run_git('commit', '-m', message)

    def push_event(self, **overrides):
        return {'before': self.base, 'after': self.sha(), 'ref': 'refs/heads/main',
                'forced': False, 'deleted': False, **overrides}

    def test_exact_main_diff_and_refusal_of_unknown_history(self):
        (self.root / 'README.md').write_text('# Changed\n', encoding='utf-8')
        self.commit('Docs')
        self.assertTrue(DOCS.exact_diff(self.root, 'push', self.push_event())['docs_only'])
        for override in [{'before': '0' * 40}, {'before': 'a' * 40}, {'after': self.base},
                         {'forced': True}, {'deleted': True}, {'ref': 'refs/heads/other'}]:
            with self.subTest(override=override), self.assertRaises((ValueError, subprocess.CalledProcessError)):
                DOCS.exact_diff(self.root, 'push', self.push_event(**override))
        with self.assertRaises(ValueError):
            DOCS.exact_diff(self.root, 'workflow_dispatch', {})

    def test_pull_request_uses_merge_base_and_requires_exact_parents(self):
        self.run_git('checkout', '-b', 'feature')
        (self.root / 'README.md').write_text('# Feature docs\n', encoding='utf-8')
        self.commit('Docs')
        head = self.sha()
        self.run_git('checkout', 'main')
        (self.root / 'main.rs').write_text('fn main() {}\n', encoding='utf-8')
        self.commit('Unrelated base update')
        base = self.sha()
        self.run_git('merge', '--no-ff', 'feature', '-m', 'PR merge')
        event = {'pull_request': {'base': {'sha': base}, 'head': {'sha': head}}}
        result = DOCS.exact_diff(self.root, 'pull_request', event)
        self.assertTrue(result['docs_only'])
        self.assertEqual(result['comparison_base'], self.base)
        self.assertEqual([change['path'] for change in result['changes']], ['README.md'])
        self.run_git('checkout', 'feature')
        with self.assertRaises(ValueError):
            DOCS.exact_diff(self.root, 'pull_request', event)

    def test_source_rename_delete_and_executable_docs_use_full_suite(self):
        for change in ('source', 'rename', 'delete', 'executable'):
            with self.subTest(change=change):
                self.run_git('reset', '--hard', self.base)
                if change == 'source':
                    (self.root / 'code.rs').write_text('fn main() {}', encoding='utf-8')
                elif change == 'rename':
                    self.run_git('mv', 'README.md', 'CHANGELOG.md')
                elif change == 'delete':
                    (self.root / 'README.md').unlink()
                else:
                    self.run_git('update-index', '--chmod=+x', 'README.md')
                    self.run_git('commit', '-m', change)
                if change != 'executable':
                    self.commit(change)
                self.assertFalse(DOCS.exact_diff(self.root, 'push', self.push_event())['docs_only'])


class DocumentationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / 'docs' / 'qa').mkdir(parents=True)
        (self.root / 'scripts').mkdir()
        shutil.copyfile(Path(__file__).with_name('package-guides.py'), self.root / 'scripts' / 'package-guides.py')
        (self.root / 'Cargo.toml').write_text('[workspace.package]\nversion = "0.48.0"\n', encoding='utf-8')
        for name in ('USER_GUIDE.en.md', 'USER_GUIDE.fr.md'):
            (self.root / 'docs' / name).write_text('# Guide\n\n[Info](DETAILS.md)\n', encoding='utf-8')
        (self.root / 'docs' / 'DETAILS.md').write_text('# Details\n', encoding='utf-8')
        self.backlog = self.root / 'docs' / 'AEC_BACKLOG.csv'
        with self.backlog.open('w', encoding='utf-8', newline='') as output:
            writer = csv.writer(output)
            writer.writerow(DOCS.BACKLOG_COLUMNS)
            writer.writerow(['AEC-001', 'P1', 'Measure', 'Known dimensions', '', 'In progress', 'Partial'])
        self.evidence = self.root / 'docs' / 'qa' / 'RELEASE_0.47.0.json'
        self.evidence.write_text(json.dumps({'source': 'a' * 40, 'version': '0.47.0', 'tag': 'v0.47.0'}), encoding='utf-8')

    def test_actual_packaged_guides_and_repository_data_are_checked(self):
        before = (self.root / 'docs' / 'USER_GUIDE.en.md').read_bytes()
        report = DOCS.validate(self.root, all_markdown=True)
        self.assertEqual(report['packaged_guides'], {'USER_GUIDE.en.md': 1, 'USER_GUIDE.fr.md': 1})
        self.assertEqual(report['backlog_rows'], 1)
        self.assertEqual(report['evidence_json_files'], 1)
        self.assertEqual((self.root / 'docs' / 'USER_GUIDE.en.md').read_bytes(), before)

    def test_broken_guide_link_and_duplicate_major_heading_fail(self):
        path = self.root / 'docs' / 'USER_GUIDE.en.md'
        for text in ['# Guide\n[Bad](absent.md)', '# Guide\n## Reuse templates\n## Reuse templates\n']:
            path.write_text(text, encoding='utf-8')
            with self.subTest(text=text), self.assertRaises((ValueError, FileNotFoundError)):
                DOCS.validate(self.root)

    def test_legacy_duplicate_headings_are_allowed_but_not_increased(self):
        legacy = '# Release\n## Changes\n## Changes\n'
        DOCS.validate_headings(legacy, legacy)
        with self.assertRaises(ValueError):
            DOCS.validate_headings(legacy + '## Changes\n', legacy)
        DOCS.validate_headings('# Guide\n```md\n## Example\n## Example\n```\n## Example\n')

    def test_bad_json_duplicate_keys_nonfinite_and_version_drift_fail(self):
        for value in ['[]', '{}', '{"source":"wrong"}', '{"a":1,"a":2}', '{"value":NaN}', '{"value":1e400}',
                      '{"version":"0.48.0"}', '{"version":"0.47.0","tag":"v0.48.0"}']:
            self.evidence.write_text(value, encoding='utf-8')
            with self.subTest(value=value), self.assertRaises(ValueError):
                DOCS.validate_evidence(self.evidence)
        self.evidence.write_text('{"deep":' + '[' * 65 + '0' + ']' * 65 + '}', encoding='utf-8')
        with self.assertRaises(ValueError):
            DOCS.validate_evidence(self.evidence)

    def test_backlog_missing_cells_duplicate_ids_and_invalid_status_fail(self):
        valid = self.backlog.read_text(encoding='utf-8')
        for value in [valid + valid.splitlines()[1] + '\n', valid.replace('In progress', 'Complete'),
                      valid.replace(',Partial', ''), valid.replace('capability', 'wrong_header')]:
            self.backlog.write_text(value, encoding='utf-8')
            with self.subTest(value=value), self.assertRaises(ValueError):
                DOCS.validate_backlog(self.backlog)


class WorkflowBoundaryTests(unittest.TestCase):
    def test_required_job_and_every_application_step_have_explicit_boundary(self):
        path = Path(__file__).resolve().parents[1] / '.github' / 'workflows' / 'ci.yml'
        workflow = path.read_text(encoding='utf-8')
        self.assertIn('  windows:\n    runs-on: windows-latest\n', workflow)
        self.assertNotIn('paths-ignore:', workflow)
        self.assertIn('fetch-depth: 0', workflow)
        steps = [block for block in re.split(r'(?=^      - )', workflow.split('    steps:\n', 1)[1], flags=re.MULTILINE) if block]
        self.assertEqual(sum('id: changes\n' in block for block in steps), 1)
        exempt = (
            '      - uses: actions/checkout@',
            '      - name: Classify exact documentation change\n',
            '      - name: Documentation CI boundary regression\n',
            '      - name: Validate actual packaged guides and documentation\n',
            '      - name: Documentation validation evidence\n',
        )
        for step in steps:
            if not step.startswith(exempt):
                with self.subTest(step=step.splitlines()[0]):
                    self.assertRegex(step, r"(?m)^        if: steps\.changes\.outputs\.docs_only != 'true'(?: && .+)?$")
        self.assertIn('run: cargo clippy --workspace --all-targets --locked --keep-going -j 2 -- -D warnings', workflow)
        self.assertIn('cargo test --workspace --locked -j 2 --no-fail-fast', workflow)
        self.assertIn('if-no-files-found: error', workflow)


if __name__ == '__main__':
    unittest.main(verbosity=2)
