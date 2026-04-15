#!/usr/bin/env python3

from __future__ import annotations

import argparse
from pathlib import Path
import sys


def load_maincpu(repo_root: Path) -> bytes:
    names = ["pacman.6e", "pacman.6f", "pacman.6h", "pacman.6j"]
    return b"".join(
        (repo_root / "not-required-anymore" / "rom" / name).read_bytes()
        for name in names
    )


def format_bytes(data: bytes) -> str:
    return " ".join(f"{byte:02x}" for byte in data)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Linear Z80 disassembly of the Midway Pac-Man main CPU ROM"
    )
    parser.add_argument(
        "--start",
        type=lambda value: int(value, 0),
        default=0,
        help="start address within the 0x0000-0x3fff main CPU region",
    )
    parser.add_argument(
        "--end",
        type=lambda value: int(value, 0),
        default=0x4000,
        help="end address within the 0x0000-0x3fff main CPU region",
    )
    parser.add_argument(
        "--strings",
        action="store_true",
        help="emit printable ASCII runs instead of instructions",
    )
    args = parser.parse_args()

    try:
        from z80dis import z80
    except Exception as exc:  # pragma: no cover - tooling path
        print(
            "error: z80dis is not installed. Activate .venv-z80dis or install it first.",
            file=sys.stderr,
        )
        print(f"detail: {exc}", file=sys.stderr)
        return 1

    repo_root = Path(__file__).resolve().parents[1]
    rom = load_maincpu(repo_root)
    start = max(0, min(args.start, len(rom)))
    end = max(start, min(args.end, len(rom)))

    if args.strings:
        index = start
        while index < end:
            run_start = index
            while index < end and 32 <= rom[index] <= 126:
                index += 1
            if index - run_start >= 4:
                text = rom[run_start:index].decode("ascii", errors="replace")
                print(f"{run_start:04x}: {text}")
            index = max(index + 1, run_start + 1)
        return 0

    pc = start
    while pc < end:
        decoded = z80.decode(rom[pc:end], pc)
        if decoded.status == z80.DECODE_STATUS.OK and decoded.len > 0:
            bytes_ = rom[pc : pc + decoded.len]
            print(f"{pc:04x}: {format_bytes(bytes_):<14} {z80.disasm(decoded)}")
            pc += decoded.len
        else:
            print(f"{pc:04x}: {rom[pc]:02x}             db 0x{rom[pc]:02x}")
            pc += 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
