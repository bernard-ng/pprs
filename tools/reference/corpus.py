#!/usr/bin/env python3
"""Offline corpus integrity and phase coverage; never invokes PHP."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re

from verify_p00 import verify_metadata

ROOT = Path(__file__).resolve().parents[2]
CHANNELS = ('tokens', 'structure', 'positions', 'diagnostics')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def rows(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def unbytes(value):
    require(isinstance(value, dict) and set(value) == {'bytes'}, 'Expected tagged bytes')
    require(re.fullmatch(r'(?:[0-9a-f]{2})*', value['bytes']) is not None, 'Noncanonical byte encoding')
    return bytes.fromhex(value['bytes'])


def canonicalize(raw):
    return b'\n'.join(line.rstrip(b' \t') for line in raw.replace(b'\r\n', b'\n').rstrip(b'\n').split(b'\n'))


def validate(directory, root=ROOT):
    lock, files, _, inventory = verify_metadata(root)
    manifest = json.loads((directory / 'manifest.json').read_text())
    require(manifest['schema_version'] == 1, 'Unknown corpus schema')
    require(manifest['reference_commit'] == lock['commit'], 'Reference pin differs')
    require(manifest['reference_manifest_sha256'] == lock['file_manifest_sha256'], 'Reference hashes differ')
    for name, expected in manifest['files'].items():
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'Unsafe artifact path')
        require(sha((directory / name).read_bytes()) == expected, f'Corpus hash differs: {name}')
    actual_files = {str(p.relative_to(directory)).replace('\\', '/') for p in directory.rglob('*') if p.is_file()}
    require(actual_files == set(manifest['files']) | {'manifest.json'}, 'Unexpected/missing generated files')
    for name, expected in manifest['exporter_sha256'].items():
        require(sha((root / 'tools/reference' / name).read_bytes()) == expected, f'Exporter changed: {name}; regenerate')
    records = rows(directory / 'cases.jsonl')
    indexed = {r['id']: r for r in records}
    inputs = {r['id']: r for r in json.loads((root / 'tools/reference/inputs.json').read_text())}
    require(len(indexed) == len(records), 'Duplicate corpus ID')
    require(list(indexed) == sorted(indexed), 'Nondeterministic record order')
    originals = {r['id']: r for r in inventory if r['id'].startswith('parser/')}
    require({r['id'] for r in records if r['origin']} == set(originals), 'Original case set differs')
    require(set(indexed) == set(originals) | set(inputs), 'Supplemental case set differs')
    feature_ids = {f['id'] for f in json.loads((root / 'docs/compatibility/features.json').read_text())['features']}
    expected_raw = {'raw/' + r['path'].removeprefix('test/code/') for r in originals.values()}
    require({name for name in manifest['files'] if name.startswith('raw/')} == expected_raw, 'Raw file set differs')
    require(rows(directory / 'deferred-printer.jsonl') == [r for r in inventory if not r['id'].startswith('parser/')],
            'Printer inventory differs')
    for record in records:
        identifier = record['id']
        source = unbytes(record['source'])
        require(record['schema_version'] == 1 and record['profile_id'] == manifest['profile_id'], 'Case schema/profile differs')
        require(sha(source) == record['source_sha256'], f'Source hash differs: {identifier}')
        if record['origin']:
            original = originals[identifier]
            origin = record['origin']
            require(origin['commit'] == lock['commit'] and origin['file_sha256'] == files[origin['path']], 'Bad provenance')
            require(origin['path'] == original['path'] and origin['case_index'] == original['case_index'], 'Origin differs')
            raw = (directory / ('raw/' + origin['path'].removeprefix('test/code/'))).read_bytes()
            require(sha(raw) == origin['file_sha256'], 'Raw source file differs')
            parts = re.split(rb'\n-----(?:\n|$)', raw)[1:]
            sections = parts[2 * origin['case_index']:2 * origin['case_index'] + 2]
            require([sha(part) for part in sections] == original['raw_section_sha256'] == origin['raw_section_sha256'], 'Raw section hashes differ')
            require(all(record[key] == original[key] for key in ('scope', 'phase', 'features', 'mode', 'reason')), 'Classification differs')
            require(record['original_dump_verified'] is True, 'Unverified original dump')
            if b'@@{' not in raw:
                # Independent byte-level importer check where no PHP evaluation is necessary.
                sections = re.split(rb'\n-----(?:\n|$)', canonicalize(raw))[1:]
                expected_source, expected = sections[2 * origin['case_index']:2 * origin['case_index'] + 2]
                if expected.startswith(b'!!'):
                    expected = expected.split(b'\n', 1)[1]
                require(source == expected_source and unbytes(record['original_expected']) == expected, 'Importer fidelity differs')
        require(record['features'] and set(record['features']) <= feature_ids, 'Unknown feature mapping')
        if not record['origin']:
            definition = inputs[identifier]
            require(all(record[key] == definition[key] for key in ('title', 'features', 'phase', 'reason')), 'Supplemental classification differs')
            require(record['derivation'] == definition.get('derivation'), 'Derivation definition differs')
            require(record['comparison_target'] == definition['target'], 'Supplemental target differs')
            if not record['derivation']:
                require(record['source'] == definition['source'], 'Supplemental source differs')
        if record['derivation']:
            derivation = record['derivation']
            parent = unbytes(indexed[derivation['parent']]['source'])
            require(sha(parent) == derivation['source_sha256'], 'Derived parent changed')
            require(all(0 <= a <= b <= len(parent) for a, b in derivation['segments']), 'Invalid derivation range')
            require(source == b''.join(parent[a:b] for a, b in derivation['segments']), 'Derived source differs')
        if record['scope'] == 'core':
            require(record['comparison_target'] in lock['supported_target_versions'], 'Out-of-scope comparison target')
            require(record['oracle']['target'] == record['comparison_target'], 'Oracle target differs')
        else:
            require(record['comparison_target'] is None, 'Out-of-scope original became comparable')
    counts = {'original_parser': len(originals), 'derived': sum(r['derivation'] is not None for r in records),
              'project': sum(r['id'].startswith('project/') for r in records), 'printer_deferred': 240, 'total': len(records)}
    require(counts == manifest['counts'], 'Manifest counts differ')
    profile = (json.dumps(manifest['profile'], ensure_ascii=True, separators=(',', ':')) + '\n').encode()
    require(sha(profile) == manifest['profile_id'], 'Profile identity differs')
    return records


def coverage(records, state):
    require(state['schema_version'] == 1, 'Unknown coverage schema')
    indexed = {r['id']: r for r in records}
    require(set(state['cases']) == set(indexed), 'Coverage case set differs')
    phases = defaultdict(Counter)
    case_states = Counter({s: 0 for s in ('pending', 'passing', 'intentional-difference', 'deferred')})
    for identifier, channels in state['cases'].items():
        require(set(channels) == set(CHANNELS), 'Coverage channels differ')
        observed = set()
        for channel, entry in channels.items():
            status = entry['state']
            require(status in ('pending', 'passing', 'intentional-difference', 'deferred'), 'Unknown coverage state')
            require(re.fullmatch(r'P(?:0[5-9]|1[0-4])|E\d\d', entry['phase']), 'Missing/invalid owner phase')
            require((status == 'deferred') == (indexed[identifier]['scope'] != 'core'), 'Invalid scope deferral')
            if status in ('deferred', 'intentional-difference'):
                require(bool(entry.get('reason')), 'Missing disposition reason')
            if status == 'intentional-difference':
                require(bool(entry.get('expectation')), 'Difference needs a checked-in exact expectation')
            phases[entry['phase']][status] += 1
            observed.add(status)
        case_states['deferred' if observed == {'deferred'} else 'passing' if observed == {'passing'}
                    else 'intentional-difference' if observed <= {'passing', 'intentional-difference'} else 'pending'] += 1
    return {'total': len(records), 'comparable': sum(r['scope'] == 'core' for r in records),
            'original_scopes': dict(sorted(Counter(r['scope'] for r in records if r['origin']).items())),
            'cases': dict(sorted(case_states.items())),
            'channels_by_phase': {p: dict(sorted(c.items())) for p, c in sorted(phases.items())}}


def prevent_demotion(previous, current):
    for identifier, channels in previous['cases'].items():
        for channel, entry in channels.items():
            if entry['state'] in ('passing', 'intentional-difference'):
                now = current['cases'].get(identifier, {}).get(channel, {})
                require(now.get('state') in ('passing', 'intentional-difference'), f'Coverage demotion: {identifier}/{channel}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write-report', action='store_true')
    parser.add_argument('--previous-coverage', type=Path)
    args = parser.parse_args()
    records = validate(ROOT / 'tests/fixtures/upstream')
    state = json.loads((ROOT / 'tests/fixtures/coverage.json').read_text())
    report = coverage(records, state)
    if args.previous_coverage:
        prevent_demotion(json.loads(args.previous_coverage.read_text()), state)
    path = ROOT / 'tests/fixtures/coverage-report.json'
    if args.write_report:
        path.write_text(json.dumps(report, indent=2) + '\n')
    else:
        require(json.loads(path.read_text()) == report, 'Coverage report stale; run --write-report')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
