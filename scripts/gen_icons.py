"""Generate LeetLife icons (app icon + monochrome menu-bar template) using only stdlib."""
import struct, zlib, math, os, sys

def png(path, w, h, pixel):
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        for x in range(w):
            raw.extend(pixel(x, y))
    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)

def house_mask(x, y, s):
    """1 if (x,y) inside a tiny house silhouette drawn in an s×s box."""
    u, v = x / s, y / s
    # roof: triangle from (0.1,0.5) apex (0.5,0.12) to (0.9,0.5)
    if 0.12 <= v <= 0.52:
        half = (v - 0.12) / 0.40 * 0.42
        if abs(u - 0.5) <= half:
            return 1
    # body
    if 0.5 <= v <= 0.88 and 0.2 <= u <= 0.8:
        # door cut-out
        if 0.62 <= v <= 0.88 and 0.42 <= u <= 0.58:
            return 0
        return 1
    return 0

def app_icon(size):
    def px(x, y):
        # rounded-square gradient background
        r = size * 0.22
        cx = min(max(x, r), size - r); cy = min(max(y, r), size - r)
        inside = (x - cx) ** 2 + (y - cy) ** 2 <= r * r
        if not inside:
            return (0, 0, 0, 0)
        t = y / size
        bg = (int(46 + 40 * t), int(52 + 30 * t), int(140 + 60 * (1 - t)), 255)
        # house in warm yellow, with slight inset
        inset = size * 0.16
        if inset <= x < size - inset and inset <= y < size - inset:
            if house_mask(x - inset, y - inset, size - 2 * inset):
                return (255, 214, 92, 255)
        return bg
    return px

def tray_icon(size):
    def px(x, y):
        inset = size * 0.08
        if inset <= x < size - inset and inset <= y < size - inset:
            if house_mask(x - inset, y - inset, size - 2 * inset):
                return (0, 0, 0, 255)
        return (0, 0, 0, 0)
    return px

out = sys.argv[1] if len(sys.argv) > 1 else "src-tauri/icons"
for s in (32, 128, 256, 512, 1024):
    name = {32: "32x32.png", 128: "128x128.png", 256: "128x128@2x.png", 512: "icon.png", 1024: "source.png"}[s]
    png(os.path.join(out, name), s, s, app_icon(s))
png(os.path.join(out, "tray.png"), 44, 44, tray_icon(44))
png(os.path.join(out, "tray@1x.png"), 22, 22, tray_icon(22))
print("icons written to", out)
