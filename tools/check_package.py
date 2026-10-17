#!/usr/bin/env python3
"""Build and test an independent consumer against the Cargo archive, outside Git."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--toolchain', default='stable')
    args = parser.parse_args()
    cargo = ['cargo', '+' + args.toolchain]
    package = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']
    name = f"{package['name']}-{package['version']}"
    target = ROOT / 'target/package-check'
    subprocess.run(cargo + ['package', '--locked', '--offline', '--allow-dirty',
                           '--no-verify', '--target-dir', str(target)], cwd=ROOT, check=True)
    archive = target / 'package' / (name + '.crate')
    with tempfile.TemporaryDirectory(prefix='pprs-package-') as scratch:
        directory = Path(scratch)
        with tarfile.open(archive) as source:
            members = source.getnames()
            relative = [Path(member).relative_to(name).as_posix() for member in members]
            for member in relative:
                assert (member.startswith('src/') or member in {
                    'docs/guide.md', 'Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock',
                    'README.md', 'THIRD_PARTY_NOTICES.md',
                    'reference/LICENSE.nikic-PHP-Parser', '.cargo_vcs_info.json',
                }), f'unexpected package file: {member}'
            source.extractall(directory, filter='data')
        library = directory / name
        consumer = directory / 'consumer'
        shutil.copytree(ROOT / 'tests/consumer', consumer, ignore=shutil.ignore_patterns('target'))
        manifest = consumer / 'Cargo.toml'
        text = manifest.read_text()
        assert 'path = "../.."' in text
        manifest.write_text(text.replace('path = "../.."', f'path = "../{name}"'))
        subprocess.run(cargo + ['test', '--locked', '--offline', '--manifest-path', str(manifest)], cwd=directory, check=True)
        subprocess.run(cargo + ['test', '--locked', '--offline', '--doc', '--manifest-path', str(library / 'Cargo.toml')], cwd=directory, check=True)
        # Both library compilation and its embedded documentation must be self-contained.
        assert not (directory / '.references').exists()
        assert not (directory / '.git').exists()
    print(json.dumps({'archive': str(archive.relative_to(ROOT)),
                      'sha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
                      'files': len(members), 'toolchain': args.toolchain,
                      'consumer_tests': 'passed', 'packaged_doctests': 'passed'}, indent=2))


if __name__ == '__main__':
    main()
