"""Package a built Pacman executable and its README for GitHub Releases."""

import argparse
from pathlib import Path
import re
import tarfile
import zipfile

WINDOWS_TARGET = "x86_64-pc-windows-msvc"
TARGETS = ("aarch64-apple-darwin", WINDOWS_TARGET,
           "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu")


def package_release(root: Path, target: str, label: str) -> Path:
    """Create a platform archive from an existing release build."""
    if target not in TARGETS or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", label):
        raise ValueError("unsupported target or invalid archive label")
    binary_name = "pacman.exe" if target == WINDOWS_TARGET else "pacman"
    binary = root / "target" / target / "release" / binary_name
    readme = root / "README.md"
    for source in (binary, readme):
        if not source.is_file():
            raise FileNotFoundError(source)
    dist = root / "dist"
    dist.mkdir(exist_ok=True)
    extension = "zip" if target == WINDOWS_TARGET else "tar.gz"
    archive = dist / f"pacman-{label}-{target}.{extension}"
    entries = ((binary, binary_name), (readme, "README.md"))
    if target == WINDOWS_TARGET:
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as output:
            for source, name in entries:
                output.write(source, name)
    else:
        with tarfile.open(archive, "w:gz") as output:
            for source, name in entries:
                output.add(source, arcname=name)
    return archive


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=TARGETS)
    parser.add_argument("--label", required=True, help="release tag or commit SHA")
    args = parser.parse_args()
    print(package_release(Path.cwd(), args.target, args.label))


if __name__ == "__main__":
    main()
