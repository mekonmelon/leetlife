#!/usr/bin/env python3
"""Native smoke test: launch normally, launch quietly, then reopen the same process.

Uses macOS window metadata and a login-style launch. Does not inspect other applications,
require Accessibility access, or alter game progress. Restarts LeetLife and leaves it open.
Run after building/installing: python3 scripts/test_macos_open.py [LeetLife.app]
"""
import ctypes as c
import json
import pathlib
import plistlib
import re
import subprocess
import os
import signal
import urllib.request
import sys
import time

app_path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "~/Applications/LeetLife.app").expanduser().resolve()
with (app_path / "Contents/Info.plist").open("rb") as source:
    bundle_id = plistlib.load(source)["CFBundleIdentifier"]
executable = app_path / "Contents/MacOS/leetlife"

cg = c.CDLL("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")
cf = c.CDLL("/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")
cg.CGWindowListCopyWindowInfo.argtypes = [c.c_uint32, c.c_uint32]
cg.CGWindowListCopyWindowInfo.restype = c.c_void_p
cf.CFPropertyListCreateData.argtypes = [c.c_void_p, c.c_void_p, c.c_long, c.c_ulong, c.c_void_p]
cf.CFPropertyListCreateData.restype = c.c_void_p
cf.CFDataGetLength.argtypes = [c.c_void_p]
cf.CFDataGetLength.restype = c.c_long
cf.CFDataGetBytePtr.argtypes = [c.c_void_p]
cf.CFDataGetBytePtr.restype = c.c_void_p
cf.CFRelease.argtypes = [c.c_void_p]


def pid():
    process = subprocess.run(["pgrep", "-f", "^" + re.escape(str(executable)) + "($| )"], capture_output=True, text=True, timeout=3)
    values = process.stdout.strip().splitlines()
    return int(values[0]) if values else None


def visible_windows():
    process_id = pid()
    windows = cg.CGWindowListCopyWindowInfo(1, 0)  # on-screen only
    if not windows:
        return []
    data = cf.CFPropertyListCreateData(None, windows, 100, 0, None)
    try:
        items = plistlib.loads(c.string_at(cf.CFDataGetBytePtr(data), cf.CFDataGetLength(data)))
        return [item for item in items if item.get("kCGWindowOwnerPID") == process_id
                and item.get("kCGWindowBounds", {}).get("Height", 0) > 100
                and item.get("kCGWindowBounds", {}).get("Width", 0) >= 300]
    finally:
        cf.CFRelease(data)
        cf.CFRelease(windows)


def expect_visible(expected):
    for _ in range(50):
        if bool(visible_windows()) == expected:
            return
        time.sleep(0.1)
    raise AssertionError("LeetLife window should be " + ("visible" if expected else "hidden"))


subprocess.run(["open", str(app_path)], check=True, timeout=10)
expect_visible(True)
original_pid = pid()
print("PASS: opening LeetLife displays a native window")
# Accessory menu-bar apps cannot reliably be hidden through NSRunningApplication.
# Exercise a real hidden-window state using the supported quiet login launch.
os.kill(original_pid, signal.SIGTERM)
for _ in range(50):
    if pid() is None:
        break
    time.sleep(0.1)
else:
    raise AssertionError("LeetLife did not stop before the login-launch check")
subprocess.run(["open", str(app_path), "--args", "--autostart"], check=True, timeout=10)
for _ in range(100):
    try:
        with urllib.request.urlopen("http://127.0.0.1:47831/health", timeout=0.2) as response:
            ready = response.status == 200
    except OSError:
        ready = False
    if pid() and ready:
        break
    time.sleep(0.1)
else:
    raise AssertionError("Quiet login launch did not become ready")
time.sleep(0.5)
expect_visible(False)
original_pid = pid()
print("PASS: login launch runs with its window hidden")
subprocess.run(["open", str(app_path)], check=True, timeout=10)
expect_visible(True)
assert pid() == original_pid, "Reopening should reuse the existing process"
# Focus changes from launching/returning to a terminal must not immediately dismiss it.
time.sleep(1)
expect_visible(True)
print("PASS: reopening the hidden app restores its window and keeps it visible")
print(json.dumps({"bundle_id": bundle_id, "pid": original_pid,
                  "window_bounds": [w["kCGWindowBounds"] for w in visible_windows()]}))
