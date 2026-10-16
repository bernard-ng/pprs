"""Regression checks for the baseline verifier; never mutate the reference checkout."""

import copy
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest

import verify_p00


ROOT = Path(__file__).resolve().parents[2]


class BaselineTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        shutil.copytree(ROOT / "reference", self.root / "reference")
        shutil.copytree(ROOT / "docs/compatibility", self.root / "docs/compatibility")

    def change_inventory(self, name, transform):
        path = self.root / "docs/compatibility" / name
        values = verify_p00.rows(path)
        transform(values)
        path.write_text("".join(json.dumps(row) + "\n" for row in values))
        # Update the checksum to test structural checks, not only byte integrity.
        lock_path = self.root / "reference/lock.json"
        lock = json.loads(lock_path.read_text())
        lock["inventory_sha256"][str(path.relative_to(self.root))] = hashlib.sha256(path.read_bytes()).hexdigest()
        lock_path.write_text(json.dumps(lock))

    def test_frozen_baseline_is_consistent(self):
        verify_p00.verify_metadata(self.root)

    def test_inventory_byte_drift_fails(self):
        with (self.root / "docs/compatibility/fixtures.jsonl").open("a") as file:
            file.write("\n")
        with self.assertRaisesRegex(verify_p00.InvalidBaseline, "inventory changed"):
            verify_p00.verify_metadata(self.root)

    def test_duplicate_case_fails_even_with_updated_checksum(self):
        self.change_inventory("fixtures.jsonl", lambda rows: rows.append(copy.deepcopy(rows[0])))
        with self.assertRaisesRegex(verify_p00.InvalidBaseline, "duplicate fixture ID"):
            verify_p00.verify_metadata(self.root)

    def test_unknown_feature_fails(self):
        self.change_inventory("grammar.jsonl", lambda rows: rows[0].update(features=["unknown"]))
        with self.assertRaisesRegex(verify_p00.InvalidBaseline, "unknown or missing feature"):
            verify_p00.verify_metadata(self.root)

    def test_missing_node_fails(self):
        self.change_inventory("nodes.jsonl", lambda rows: rows.pop())
        with self.assertRaisesRegex(verify_p00.InvalidBaseline, "node inventory is incomplete"):
            verify_p00.verify_metadata(self.root)

    def test_license_drift_fails(self):
        (self.root / "reference/LICENSE.nikic-PHP-Parser").write_text("not the original license")
        with self.assertRaisesRegex(verify_p00.InvalidBaseline, "license copy differs"):
            verify_p00.verify_metadata(self.root)


if __name__ == "__main__":
    unittest.main()
