#!/usr/bin/env python3
"""Deterministic seeds from the frozen core corpus; no reference/PHP dependency."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

def seeds():
    for line in (ROOT / 'tests/fixtures/upstream/cases.jsonl').read_text().splitlines():
        row = json.loads(line)
        if row['scope'] != 'core':
            continue
        source = bytes.fromhex(row['source']['bytes'])
        yield source
        # Sample truncations and deterministic single-byte deletion/replacement.
        for at in sorted({0, len(source)//4, len(source)//2, 3*len(source)//4, max(0,len(source)-1)}):
            yield source[:at]
            if at < len(source):
                yield source[:at] + source[at+1:]
                yield source[:at] + b'\x00' + source[at+1:]
    for value in range(256):
        yield bytes([value])

def main():
    unique = {hashlib.sha256(s).hexdigest():s for s in seeds()}
    for target in ['lex_bytes', 'parse_bytes']:
        directory = ROOT / 'fuzz/corpus' / target
        directory.mkdir(parents=True,exist_ok=True)
        for name,source in unique.items():
            (directory/name).write_bytes(source)
    directory = ROOT / 'fuzz/corpus/parse_generated'
    directory.mkdir(parents=True,exist_ok=True)
    for value in range(256):
        (directory/f'{value:03}').write_bytes(bytes([value, value//2, value%7, 255-value])*4)
    print(json.dumps({'byte_seeds':len(unique),'byte_seed_sha256':hashlib.sha256(''.join(sorted(unique)).encode()).hexdigest(),'generated_seeds':256},sort_keys=True))

if __name__ == '__main__':
    main()
