"""Export the selected icon artwork without changing its design or transparency."""
import argparse
import pathlib
import shutil
from PIL import Image

parser = argparse.ArgumentParser()
parser.add_argument("source", type=pathlib.Path)
args = parser.parse_args()
assets = pathlib.Path(__file__).resolve().parent.parent / "assets"
assets.mkdir(exist_ok=True)
master = assets / "inkpage-source.png"
if args.source.resolve() != master.resolve():
    shutil.copy2(args.source, master)
with Image.open(master) as original:
    image = original.convert("RGBA")
    assert image.width == image.height, "Icon artwork must be square"
    assert image.getextrema()[3][0] == 0, "Icon must preserve transparent pixels"
    for size in (16, 20, 24, 32, 40, 48, 64, 128, 256):
        image.resize((size, size), Image.Resampling.LANCZOS).save(
            assets / f"inkpage-{size}.png", optimize=True
        )
    image.save(assets / "InkPage.ico", sizes=[(size, size) for size in (16, 20, 24, 32, 40, 48, 64, 128, 256)])
print("Exported PNG icons and multi-resolution Windows ICO")
