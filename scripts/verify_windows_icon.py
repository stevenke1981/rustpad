"""Read embedded icon/version resources; does not start the executable."""
import argparse
import ctypes
import json
import pathlib
import struct

parser = argparse.ArgumentParser()
parser.add_argument("exe", type=pathlib.Path)
parser.add_argument("--output", type=pathlib.Path)
parser.add_argument("--version", default="0.10.1")
args = parser.parse_args()
path = str(args.exe.resolve())
kernel = ctypes.WinDLL("kernel32", use_last_error=True)
kernel.LoadLibraryExW.argtypes = [ctypes.c_wchar_p, ctypes.c_void_p, ctypes.c_uint]
kernel.LoadLibraryExW.restype = ctypes.c_void_p
kernel.FindResourceW.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
kernel.FindResourceW.restype = ctypes.c_void_p
kernel.SizeofResource.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
kernel.SizeofResource.restype = ctypes.c_uint
kernel.LoadResource.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
kernel.LoadResource.restype = ctypes.c_void_p
kernel.LockResource.argtypes = [ctypes.c_void_p]
kernel.LockResource.restype = ctypes.c_void_p
kernel.FreeLibrary.argtypes = [ctypes.c_void_p]
module = kernel.LoadLibraryExW(path, None, 0x22)  # data / image resource only
assert module, ctypes.WinError(ctypes.get_last_error())
def resource(name, kind):
    item = kernel.FindResourceW(module, name, kind)
    assert item, f"Missing resource {name}/{kind}"
    size = kernel.SizeofResource(module, item)
    data = kernel.LockResource(kernel.LoadResource(module, item))
    assert data
    return ctypes.string_at(data, size)
try:
    group = resource(1, 14)
    reserved, kind, count = struct.unpack_from("<HHH", group)
    assert (reserved, kind, count) == (0, 1, 9)
    sizes = []
    for index in range(count):
        width, height, _, _, _, _, length, icon_id = struct.unpack_from("<BBBBHHIH", group, 6 + 14 * index)
        sizes.append([width or 256, height or 256])
        assert len(resource(icon_id, 3)) == length
    assert sorted(sizes) == [[n, n] for n in (16, 20, 24, 32, 40, 48, 64, 128, 256)]
    assert resource(1, 16), "Missing version info"
finally:
    kernel.FreeLibrary(module)
version = ctypes.WinDLL("version", use_last_error=True)
version.GetFileVersionInfoSizeW.argtypes = [ctypes.c_wchar_p, ctypes.c_void_p]
version.GetFileVersionInfoSizeW.restype = ctypes.c_uint
version.GetFileVersionInfoW.argtypes = [ctypes.c_wchar_p, ctypes.c_uint, ctypes.c_uint, ctypes.c_void_p]
version.VerQueryValueW.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_uint)]
length = version.GetFileVersionInfoSizeW(path, None)
buffer = ctypes.create_string_buffer(length)
assert version.GetFileVersionInfoW(path, 0, length, buffer)
values = {}
for key in ("ProductName", "FileDescription", "FileVersion", "ProductVersion"):
    pointer = ctypes.c_void_p()
    size = ctypes.c_uint()
    assert version.VerQueryValueW(buffer, "\\StringFileInfo\\040904B0\\" + key, ctypes.byref(pointer), ctypes.byref(size))
    values[key] = ctypes.wstring_at(pointer)
assert values["ProductName"] == "InkPage"
assert values["ProductVersion"] == args.version
report = {"executable": path, "icon_sizes": sizes, "version": values}
if args.output:
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps(report, ensure_ascii=False))
