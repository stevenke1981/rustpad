"""Run after cargo metadata --filter-platform <target> into metadata-windows.json.
Copies license texts from official Cargo package sources; no home paths in output.
"""
import json
import pathlib
import shutil

root = pathlib.Path(__file__).resolve().parent.parent
metadata = json.loads((root / "metadata-windows.json").read_text(encoding="utf-8-sig"))
resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
packages = sorted((p for p in metadata["packages"] if p["id"] in resolved and p["source"]), key=lambda p: p["name"])
rows = ["# Third-party licenses (Windows dependency graph)", "", "Generated from locked Cargo metadata. Original InkPage source: MIT.", "Embedded syntax acknowledgements: [SYNTAXES.md](licenses/SYNTAXES.md). Original color palettes; no upstream theme data embedded.", "", "| Package | Version | SPDX license |", "|---|---|---|"]
for package in packages:
    rows.append(f'| {package["name"]} | {package["version"]} | {package["license"] or "SEE PACKAGE"} |')
    source = pathlib.Path(package["manifest_path"]).parent
    dest = root / "licenses" / (package["name"] + "-" + package["version"])
    dest.mkdir(exist_ok=True)
    for path in source.iterdir():
        if path.is_file() and path.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE")):
            shutil.copyfile(path, dest / path.name)
rows += ["", "Pinned package license/notice files are retained under licenses/. Linux-specific packages must be audited before a Linux binary is distributed.", ""]
(root / "THIRD_PARTY_LICENSES.md").write_text("\n".join(rows), encoding="utf-8")
