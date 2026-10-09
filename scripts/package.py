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
    archive.write(args.exe, "RustPad/rustpad.exe")
    for name in ("README.md", "SPEC.md", "V02.md", "ALIGNMENT.md", "DIFF-0.3.md", "DIFF-0.4.md", "QA.md", "QA-0.1.md", "QA-0.2.md", "QA-0.3.md", "PERFORMANCE.md", "PERFORMANCE-0.3.md", "PERFORMANCE-0.4.md", "LICENSE", "THIRD_PARTY_LICENSES.md"):
        archive.write(root / name, "RustPad/" + name)
    for path in sorted((root / "licenses").rglob("*")):
        if path.is_file():
            archive.write(path, "RustPad/" + path.relative_to(root).as_posix())
print(output.stat().st_size)
