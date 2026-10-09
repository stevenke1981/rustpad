"""Package only the executable, public docs, and license notices. No cache/config."""
import argparse
import pathlib
import zipfile

parser = argparse.ArgumentParser()
parser.add_argument("--exe", required=True)
parser.add_argument("--output", required=True)
args = parser.parse_args()
root = pathlib.Path(__file__).resolve().parent.parent
output = pathlib.Path(args.output)
if output.exists():
    raise SystemExit("Refusing to replace an existing package")
with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
    archive.write(args.exe, "InkPage/InkPage.exe")
    for name in ("README.md", "SPEC.md", "V02.md", "ALIGNMENT.md", "DIFF-0.3.md", "DIFF-0.4.md", "DIFF-0.5.md", "DIFF-0.6.md", "DIFF-0.7.md", "DIFF-0.8.md", "DIFF-0.9.md", "DIFF-0.10.md", "DIFF-0.10.1.md", "QA-0.10.md", "QA-0.10.1.md", "QA-NATIVE.md", "QA.md", "QA-0.1.md", "QA-0.2.md", "QA-0.3.md", "QA-0.4.md", "PERFORMANCE.md", "PERFORMANCE-0.3.md", "PERFORMANCE-0.4.md", "PERFORMANCE-0.5.md", "PERFORMANCE-0.8.md", "PERFORMANCE-0.9.md", "LICENSE", "THIRD_PARTY_LICENSES.md"):
        archive.write(root / name, "InkPage/" + name)
    for name in ("DESIGN-0.11.md", "DIFF-0.11.md", "QA-0.11.md"):
        archive.write(root / name, "InkPage/" + name)
    for path in sorted((root / "licenses").rglob("*")):
        if path.is_file():
            archive.write(path, "InkPage/" + path.relative_to(root).as_posix())
    for path in sorted((root / "assets").iterdir()):
        if path.is_file() and path.suffix in (".png", ".ico", ".md"):
            archive.write(path, "InkPage/assets/" + path.name)
print(output.stat().st_size)
