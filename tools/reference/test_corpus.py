"""Independent negative checks for corpus import, provenance and coverage gates."""
import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest

import corpus

ROOT = Path(__file__).resolve().parents[2]


class CorpusTests(unittest.TestCase):
    def test_complete_corpus_and_report(self):
        records = corpus.validate(ROOT / 'tests/fixtures/upstream')
        states = json.loads((ROOT / 'tests/fixtures/coverage.json').read_text())
        report = corpus.coverage(records, states)
        self.assertEqual(report, json.loads((ROOT / 'tests/fixtures/coverage-report.json').read_text()))
        self.assertEqual(len(records), 313)

    def mutate_corpus(self, mutate, message):
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / 'corpus'
            shutil.copytree(ROOT / 'tests/fixtures/upstream', destination)
            mutate(destination)
            with self.assertRaisesRegex(ValueError, message):
                corpus.validate(destination)

    def test_source_byte_drift_is_detected(self):
        def mutate(directory):
            path = directory / 'cases.jsonl'
            path.write_bytes(path.read_bytes().replace(b'3c3f706870', b'3c3f706871', 1))
        self.mutate_corpus(mutate, 'Corpus hash differs')

    def test_missing_case_even_after_updating_file_checksum(self):
        def mutate(directory):
            path = directory / 'cases.jsonl'
            lines = path.read_bytes().splitlines(keepends=True)
            index = next(i for i, line in enumerate(lines) if json.loads(line)['origin'])
            del lines[index]
            path.write_bytes(b''.join(lines))
            manifest_path = directory / 'manifest.json'
            manifest = json.loads(manifest_path.read_text())
            manifest['files']['cases.jsonl'] = corpus.sha(path.read_bytes())
            manifest_path.write_text(json.dumps(manifest))
        self.mutate_corpus(mutate, 'Original case set differs')

    def test_raw_macro_file_is_preserved(self):
        def mutate(directory):
            path = next(p for p in (directory / 'raw').rglob('*.test') if b'@@{' in p.read_bytes())
            path.write_bytes(path.read_bytes().replace(b'@@{', b'@@(', 1))
        self.mutate_corpus(mutate, 'Corpus hash differs')

    def test_passing_cannot_be_demoted_or_deleted(self):
        previous = {'cases': {'a': {'tokens': {'state': 'passing'}}}}
        for current in ({'cases': {}}, {'cases': {'a': {'tokens': {'state': 'pending'}}}}):
            with self.assertRaisesRegex(ValueError, 'Coverage demotion'):
                corpus.prevent_demotion(previous, current)
        corpus.prevent_demotion(previous, copy.deepcopy(previous))

    def test_unknown_state_missing_reason_and_core_deferral_are_rejected(self):
        records = corpus.validate(ROOT / 'tests/fixtures/upstream')
        original = json.loads((ROOT / 'tests/fixtures/coverage.json').read_text())
        identifier = next(r['id'] for r in records if r['scope'] == 'core')
        for entry, message in [({'state': 'ignored', 'phase': 'P05'}, 'Unknown coverage'),
                               ({'state': 'deferred', 'phase': 'P05'}, 'Invalid scope'),
                               ({'state': 'intentional-difference', 'phase': 'P05'}, 'Missing disposition')]:
            state = copy.deepcopy(original)
            state['cases'][identifier]['tokens'] = entry
            with self.assertRaisesRegex(ValueError, message):
                corpus.coverage(records, state)

    def test_canonicalization_and_byte_encoding(self):
        self.assertEqual(corpus.canonicalize(b'raw \t\r\n@@{x}@@ \n\n'), b'raw\n@@{x}@@')
        self.assertEqual(corpus.unbytes({'bytes': '00ff0d0a'}), b'\x00\xff\r\n')
        for invalid in ({'bytes': 'f'}, {'bytes': 'FF'}, {'bytes': '00', 'other': 1}):
            with self.assertRaises(ValueError):
                corpus.unbytes(invalid)


if __name__ == '__main__':
    unittest.main()
