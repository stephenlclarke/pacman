"""Check release archive contents and failure cases without a compiler."""

from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile

from package_release import TARGETS, WINDOWS_TARGET, package_release


class PackageReleaseTests(unittest.TestCase):
    def test_platform_archives_preserve_binary_and_readme(self):
        for target in TARGETS:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                name = "pacman.exe" if target == WINDOWS_TARGET else "pacman"
                binary = root / "target" / target / "release" / name
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b"test executable")
                binary.chmod(0o755)
                (root / "README.md").write_text("Release instructions", encoding="utf-8")
                archive = package_release(root, target, "v1.0.1")
                if target == WINDOWS_TARGET:
                    with zipfile.ZipFile(archive) as content:
                        self.assertEqual(set(content.namelist()), {name, "README.md"})
                        self.assertEqual(content.read(name), binary.read_bytes())
                else:
                    with tarfile.open(archive) as content:
                        self.assertEqual(set(content.getnames()), {name, "README.md"})
                        self.assertEqual(content.extractfile(name).read(), binary.read_bytes())
                        self.assertEqual(content.getmember(name).mode & 0o111, 0o111)

    def test_missing_binary_fails_before_creating_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(FileNotFoundError):
                package_release(root, TARGETS[0], "v1.0.1")
            self.assertFalse((root / "dist").exists())

    def test_invalid_labels_and_targets_are_rejected(self):
        for target, label in ((TARGETS[0], "../bad"), ("unknown", "v1.0.1")):
            with self.subTest(target=target, label=label), self.assertRaises(ValueError):
                package_release(Path("."), target, label)


if __name__ == "__main__":
    unittest.main()
