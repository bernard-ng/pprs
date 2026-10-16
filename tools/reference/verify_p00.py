#!/usr/bin/env python3
"""Verify the immutable P00 baseline. Read-only; no PHP, downloads or regeneration.

Default: verify metadata and the local Git checkout. --metadata-only supports
checking the committed inventory when the ignored .references checkout is absent.
This inventories raw fixture sections; PHP macro expansion belongs to P02.
"""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


class InvalidBaseline(Exception):
    pass


def require(condition, message):
    if not condition:
        raise InvalidBaseline(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rows(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line]


def unique(records, key, label):
    values = [record[key] for record in records]
    require(len(values) == len(set(values)), f"duplicate {label}")
    return set(values)


def verify_metadata(root):
    lock = json.loads((root / "reference/lock.json").read_text())
    require(lock["schema_version"] == 1, "unsupported lock schema")
    require(re.fullmatch(r"[0-9a-f]{40}", lock["commit"]), "invalid commit")
    manifest = root / lock["file_manifest"]
    require(digest(manifest) == lock["file_manifest_sha256"], "file manifest changed")
    files = {}
    for line in manifest.read_text().splitlines():
        checksum, path = line.split("  ", 1)
        require(re.fullmatch(r"[0-9a-f]{64}", checksum), f"invalid hash: {path}")
        require(not Path(path).is_absolute() and ".." not in Path(path).parts,
                f"unsafe reference path: {path}")
        require(path not in files, f"duplicate manifest path: {path}")
        files[path] = checksum
    require(len(files) == lock["tracked_file_count"], "tracked file count mismatch")
    require(digest(root / lock["license_path"]) == files["LICENSE"], "license copy differs")
    for path, checksum in lock["inventory_sha256"].items():
        require(digest(root / path) == checksum, f"P00 inventory changed: {path}")

    directory = root / "docs/compatibility"
    catalog = json.loads((directory / "features.json").read_text())
    features = catalog["features"]
    feature_ids = unique(features, "id", "feature ID")
    feature_scopes = {feature["id"]: feature["scope"] for feature in features}
    require(catalog["target_versions"] == lock["supported_target_versions"], "target mismatch")
    require(catalog["default_target"] == lock["default_target"], "default mismatch")
    grammar = rows(directory / "grammar.jsonl")
    productions = unique(grammar, "name", "grammar production")
    fixtures = rows(directory / "fixtures.jsonl")
    unique(fixtures, "id", "fixture ID")
    nodes = rows(directory / "nodes.jsonl")
    node_paths = unique(nodes, "path", "node path")
    units = rows(directory / "unit-tests.jsonl")
    unit_paths = unique(units, "path", "unit-test path")
    fixture_paths = {row["path"] for row in fixtures}
    expected_fixtures = {p for p in files if p.startswith("test/code/") and
                         (p.endswith(".test") or p.endswith(".file-test"))}
    require(fixture_paths == expected_fixtures, "fixture file inventory is incomplete")
    require(node_paths == {p for p in files if p.startswith("lib/PhpParser/Node/") and p.endswith(".php")},
            "node inventory is incomplete")
    require(unit_paths == {p for p in files if p.startswith("test/PhpParser/") and p.endswith("Test.php")},
            "unit-test inventory is incomplete")

    for feature in features:
        require(feature["scope"] in ("core", "deferred"), "invalid feature scope")
        require(feature["status"] == "not-implemented", "P00 does not certify implementation")
        require(feature["planned_owner"] and feature["lexical_requirements"], "missing feature owner/context")
        require(set(feature["productions"]) <= productions, f"unknown production in {feature['id']}")
        require(set(feature["fixture_files"]) <= fixture_paths, f"unknown fixture in {feature['id']}")
        for path in feature["reference_paths"]:
            require(path in files or any(p.startswith(path.rstrip('/') + '/') for p in files),
                    f"unknown source reference: {path}")
    for entry in grammar + fixtures + nodes:
        require(entry["features"] and set(entry["features"]) <= feature_ids, "unknown or missing feature mapping")
        require(entry["scope"] in ("core", "mixed", "deferred"), "unclassified inventory entry")
    for production in grammar:
        owners = {f["id"] for f in features if production["name"] in f["productions"]}
        require(set(production["features"]) == owners, f"production mapping drift: {production['name']}")
    for entry in fixtures:
        require(entry["reason"] and entry["phase"], f"missing disposition: {entry['id']}")
        expected = {"core": "pending", "mixed": "needs-derived-case", "deferred": "deferred"}
        require(entry["status"] == expected[entry["scope"]], f"bad scope/status: {entry['id']}")
        if entry["scope"] == "mixed":
            require({feature_scopes[key] for key in entry["features"]} == {"core", "deferred"},
                    f"mixed case lacks both feature scopes: {entry['id']}")
        require(entry["id"] == entry["path"].removeprefix("test/code/") + "#" + str(entry["case_index"]),
                f"unstable fixture ID: {entry['id']}")
    for unit in units:
        require(unit["scope"] in ("core", "deferred") and unit["reason"] and unit["phase"],
                f"unclassified unit-test family: {unit['path']}")
    parser_cases = [r for r in fixtures if r["path"].startswith("test/code/parser/")]
    counts = {
        "grammar_productions": len(grammar), "node_source_files": len(nodes),
        "unit_test_source_files": len(units), "fixture_files": len(fixture_paths),
        "fixture_cases": len(fixtures), "parser_cases": len(parser_cases),
    }
    require(counts == lock["counts"], "inventory counts differ from baseline")
    require(dict(Counter(r["scope"] for r in parser_cases)) == lock["parser_case_scopes"],
            "parser scope totals differ from baseline")
    return lock, files, grammar, fixtures


def verify_checkout(checkout, lock, files, grammar, fixtures):
    def git(*args):
        return subprocess.check_output(["git", "-C", str(checkout), *args], stderr=subprocess.PIPE).decode().strip()

    require(checkout.is_dir(), f"reference checkout missing: {checkout}")
    require(git("rev-parse", "HEAD") == lock["commit"], "reference HEAD differs from pin")
    require(git("rev-parse", "HEAD^{tree}") == lock["tree"], "reference tree differs from pin")
    require(not git("status", "--porcelain", "--untracked-files=all"), "reference checkout is dirty")
    tracked = set(git("ls-files", "-z").rstrip("\0").split("\0"))
    require(tracked == set(files), "reference tracked-file set differs")
    for path, checksum in files.items():
        require(digest(checkout / path) == checksum, f"reference bytes changed: {path}")
    source = (checkout / "grammar/php.y").read_text()
    actual_productions = {m[1]: source.count("\n", 0, m.start()) + 1
                          for m in re.finditer(r"^([a-z_][a-z_0-9]*):", source, re.M)}
    require(actual_productions == {r["name"]: r["line"] for r in grammar}, "grammar inventory/lines differ")
    indexed = {entry["id"]: entry for entry in fixtures}
    actual_ids = set()
    for path in sorted({row["path"] for row in fixtures}):
        chunks = re.split(rb"\n-----(?:\n|$)", (checkout / path).read_bytes())[1:]
        arity = 3 if "/formatPreservation/" in path else 2
        require(len(chunks) % arity == 0, f"invalid fixture section count: {path}")
        for offset in range(0, len(chunks), arity):
            parts = chunks[offset:offset + arity]
            case_id = path.removeprefix("test/code/") + "#" + str(offset // arity)
            actual_ids.add(case_id)
            require(case_id in indexed, f"unclassified case: {case_id}")
            row = indexed[case_id]
            mode = parts[-1].split(b"\n", 1)[0][2:].decode() if parts[-1].startswith(b"!!") else None
            require(row["arity"] == arity and row["mode"] == mode, f"fixture mode/arity differs: {case_id}")
            require(row["raw_section_sha256"] == [hashlib.sha256(part).hexdigest() for part in parts],
                    f"raw fixture sections differ: {case_id}")
    require(actual_ids == set(indexed), "fixture case inventory differs")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metadata-only", action="store_true", help="do not require the reference checkout")
    parser.add_argument("--reference", type=Path, help="reference checkout path; defaults to .references")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    try:
        lock, files, grammar, fixtures = verify_metadata(root)
        if not args.metadata_only:
            verify_checkout(args.reference or root / lock["local_checkout"], lock, files, grammar, fixtures)
    except (InvalidBaseline, OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"P00 verification failed: {error}", file=sys.stderr)
        return 1
    level = "metadata only" if args.metadata_only else "metadata and reference checkout"
    print(f"P00 verified ({level}): {len(files)} pinned files, {len(grammar)} productions, "
          f"{len(fixtures)} fixture cases; parser scopes {lock['parser_case_scopes']}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
