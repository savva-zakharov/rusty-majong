# Make the Android launcher icons (android/res/mipmap-*/ic_launcher.png)
# from assets/icon.png. Run again after changing the icon:
#   python android/icons.py
# Needs Pillow (pip install pillow).
from pathlib import Path

from PIL import Image

root = Path(__file__).resolve().parent.parent
icon = Image.open(root / "assets" / "icon.png").convert("RGBA")
# Launcher icon sizes per screen density.
for density, size in {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}.items():
    out = root / "android" / "res" / f"mipmap-{density}" / "ic_launcher.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    icon.resize((size, size), Image.LANCZOS if size < icon.width else Image.NEAREST).save(out)
    print(out.relative_to(root))
