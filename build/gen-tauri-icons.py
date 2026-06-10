#!/usr/bin/env python3
"""Generate minimal Tauri app icons (solid Aegis blue) without external deps."""

from __future__ import annotations

import struct
import zlib
from pathlib import Path

# AegisCenter accent blue (#1f6feb), opaque
RGBA = (0x1F, 0x6F, 0xEB, 0xFF)


def write_png(path: Path, size: int) -> None:
    r, g, b, a = RGBA
    row = b"\x00" + bytes([r, g, b, a] * size)
    raw = row * size

    def chunk(tag: bytes, data: bytes) -> bytes:
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

    # Tauri requires RGBA (color type 6), not RGB.
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(png)


def write_ico(path: Path, sizes: list[int]) -> None:
    images: list[tuple[int, bytes]] = []
    for size in sizes:
        tmp = path.with_suffix(f".{size}.png")
        write_png(tmp, size)
        images.append((size, tmp.read_bytes()))
        tmp.unlink()

    offset = 6 + 16 * len(images)
    parts = [struct.pack("<HHH", 0, 1, len(images))]
    blobs: list[bytes] = []
    for size, data in images:
        parts.append(
            struct.pack(
                "<BBBBHHII",
                size if size < 256 else 0,
                size if size < 256 else 0,
                0,
                0,
                1,
                32,
                len(data),
                offset,
            )
        )
        blobs.append(data)
        offset += len(data)
    path.write_bytes(b"".join(parts) + b"".join(blobs))


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description="Generate Tauri RGBA icons")
    parser.add_argument(
        "--root",
        type=Path,
        default=None,
        help="Project root (default: parent of build/). Use /root/aegisrescue on WSL.",
    )
    args = parser.parse_args()
    root = args.root.resolve() if args.root else Path(__file__).resolve().parent.parent
    targets = [
        root / "gui" / "src-tauri" / "icons",
        root / "installer" / "src-tauri" / "icons",
    ]
    files = [
        ("32x32.png", 32),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512),
    ]
    for icons_dir in targets:
        for name, size in files:
            write_png(icons_dir / name, size)
        write_ico(icons_dir / "icon.ico", [16, 32, 48, 64, 128, 256])
        print(f"generated icons in {icons_dir}")


if __name__ == "__main__":
    main()
