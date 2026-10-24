"""Build src-tauri/icons/icon.icns from the same vector-ish drawing as the PNGs.

macOS wants a .iconset directory of exactly-named sizes, then `iconutil` packs
it. Run: python3 scripts/gen_icns.py [workdir]
"""
import importlib.util, os, shutil, subprocess, sys

spec = importlib.util.spec_from_file_location("gen_icons", os.path.join(os.path.dirname(__file__), "gen_icons.py"))
gen = importlib.util.module_from_spec(spec)
sys.argv = [sys.argv[0], os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")]
spec.loader.exec_module(gen)  # also refreshes the PNG set

workdir = sys.argv[1] if len(sys.argv) > 1 else "build"
iconset = os.path.join(workdir, "leetlife.iconset")
shutil.rmtree(iconset, ignore_errors=True)
os.makedirs(iconset, exist_ok=True)

SIZES = {
    16: ["icon_16x16.png"],
    32: ["icon_16x16@2x.png", "icon_32x32.png"],
    64: ["icon_32x32@2x.png"],
    128: ["icon_128x128.png"],
    256: ["icon_128x128@2x.png", "icon_256x256.png"],
    512: ["icon_256x256@2x.png", "icon_512x512.png"],
    1024: ["icon_512x512@2x.png"],
}

for size, names in SIZES.items():
    first = os.path.join(iconset, names[0])
    gen.png(first, size, size, gen.app_icon(size))
    for extra in names[1:]:
        shutil.copyfile(first, os.path.join(iconset, extra))

out = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons", "icon.icns")
subprocess.run(["iconutil", "-c", "icns", iconset, "-o", out], check=True)
print("wrote", os.path.normpath(out))
