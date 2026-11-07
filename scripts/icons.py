#!/usr/bin/env python3
"""Regenerate committed app/favicon assets: uv run --with pillow scripts/icons.py."""
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

root = Path(__file__).resolve().parent.parent
assets = root / "assets"
source = Image.open(assets / "shelf-icon.png").convert("RGBA")
source = source.resize((1024, 1024), Image.Resampling.LANCZOS)
mask = Image.new("L", source.size)
ImageDraw.Draw(mask).rounded_rectangle((8, 8, 1016, 1016), radius=210, fill=255)
source.putalpha(mask)
source.save(assets / "shelf-icon.png", optimize=True)
with tempfile.TemporaryDirectory(prefix="shelf-icon-") as temporary:
    iconset = Path(temporary) / "Shelf.iconset"
    iconset.mkdir()
    for size in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            name = f"icon_{size}x{size}{'@2x' if scale == 2 else ''}.png"
            source.resize((size * scale, size * scale), Image.Resampling.LANCZOS).save(iconset / name)
    subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(assets / "Shelf.icns")], check=True)

with tempfile.TemporaryDirectory(prefix="shelf-favicon-") as temporary:
    subprocess.run(["qlmanage", "-t", "-s", "1024", "-o", temporary, str(assets / "favicon.svg")], check=True, stdout=subprocess.DEVNULL)
    small = Image.open(Path(temporary) / "favicon.svg.png").convert("RGBA")
    small.load()

def favicon(size):
    return small.resize((size, size), Image.Resampling.LANCZOS)

favicon(64).save(assets / "favicon.ico", sizes=[(16, 16), (32, 32), (48, 48), (64, 64)])
favicon(32).save(assets / "favicon.png")
for size in (192, 512):
    favicon(size).save(assets / f"icon-{size}.png")
touch = Image.new("RGB", (180, 180), "#064c40")
touch.paste(favicon(180), mask=favicon(180).getchannel("A"))
touch.save(assets / "apple-touch-icon.png")
print("Generated Shelf.icns and favicon package")
