"""Filesystem/fault-injection checks; EXDEV injection is not a second-volume run."""
import errno
import importlib.util
import os
from pathlib import Path
import sys
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
spec = importlib.util.spec_from_file_location("cache_policy_fixture", HERE / "runtime_policy.py")
policy = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = policy
spec.loader.exec_module(policy)


class Publication(unittest.TestCase):
    def setUp(self):
        self.root = ROOT / f"target/cache-location-validation/publication-{os.getpid()}/{self._testMethodName}"
        self.root.mkdir(parents=True, exist_ok=True)
        self.source = self.root / "cache/media.bin"
        self.source.parent.mkdir(exist_ok=True)
        self.source.write_bytes(b"synthetic publication bytes" * 200)
        self.destination = self.root / "final/media.bin"

    def test_same_volume_publication(self):
        expected = self.source.read_bytes()
        policy.publish(self.source, self.destination)
        self.assertEqual(self.destination.read_bytes(), expected)
        self.assertFalse(self.source.exists())

    def test_established_output_is_preserved(self):
        self.destination.parent.mkdir()
        self.destination.write_bytes(b"established output")
        with self.assertRaises(FileExistsError):
            policy.publish(self.source, self.destination)
        self.assertEqual(self.destination.read_bytes(), b"established output")
        self.assertTrue(self.source.exists())

    def test_cross_volume_copy_injection(self):
        if os.name != "nt":
            self.skipTest("This fixture injects the Windows publication primitive")
        rename = os.rename
        calls = 0
        def cross_device_once(source, destination):
            nonlocal calls
            calls += 1
            if calls == 1:
                raise OSError(errno.EXDEV, "injected cross-device move")
            return rename(source, destination)
        expected = self.source.read_bytes()
        with mock.patch.object(policy.os, "rename", side_effect=cross_device_once):
            policy.publish(self.source, self.destination)
        self.assertEqual(self.destination.read_bytes(), expected)
        self.assertFalse(self.source.exists())
        self.assertFalse(list(self.destination.parent.glob("*.tmp")))

    def test_copy_failure_keeps_source_and_cleans_own_publication_file(self):
        with mock.patch.object(policy.os, "rename", side_effect=OSError(errno.EXDEV, "injected")), \
             mock.patch.object(policy.shutil, "copyfileobj", side_effect=OSError("injected copy failure")):
            with self.assertRaises(OSError):
                policy.publish(self.source, self.destination)
        self.assertTrue(self.source.exists())
        self.assertFalse(self.destination.exists())
        self.assertFalse(list(self.destination.parent.glob("*.tmp")))


if __name__ == "__main__":
    unittest.main(verbosity=2)
