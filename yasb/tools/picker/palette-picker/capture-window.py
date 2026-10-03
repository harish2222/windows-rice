#!/usr/bin/env python3
"""Screenshot one Win32 window by title.

`PIL.ImageGrab` bit-blts the screen and misses windows painted through
`UpdateLayeredWindow` (which is how both palette-picker and saka-popup
draw, for per-pixel alpha). `PrintWindow` with `PW_RENDERFULLCONTENT`
asks the window to render itself into a DC we own, which does capture
them.

Usage: capture-window.py <title-substring> <out.png>
"""
import ctypes
from ctypes import wintypes

import sys
from PIL import Image

u = ctypes.windll.user32
g = ctypes.windll.gdi32

PW_RENDERFULLCONTENT = 0x00000002
SRCCOPY = 0x00CC0020


def find(title):
    hit = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)
    def cb(h, _):
        n = u.GetWindowTextLengthW(h)
        if n:
            b = ctypes.create_unicode_buffer(n + 1)
            u.GetWindowTextW(h, b, n + 1)
            if title in b.value and u.IsWindowVisible(h):
                hit.append(h)
        return True

    u.EnumWindows(cb, 0)
    return hit[0] if hit else None


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    title, out = sys.argv[1], sys.argv[2]
    hwnd = find(title)
    if not hwnd:
        print(f"no visible window matching {title!r}")
        return 1

    r = wintypes.RECT()
    u.GetWindowRect(hwnd, ctypes.byref(r))
    w, h = r.right - r.left, r.bottom - r.top
    if w <= 0 or h <= 0:
        print(f"window has no area: {w}x{h}")
        return 1

    dc = u.GetWindowDC(hwnd)
    mem = g.CreateCompatibleDC(dc)
    bmp = g.CreateCompatibleBitmap(dc, w, h)
    old = g.SelectObject(mem, bmp)
    ok = u.PrintWindow(hwnd, mem, PW_RENDERFULLCONTENT)

    class BITMAPINFOHEADER(ctypes.Structure):
        _fields_ = [
            ("biSize", wintypes.DWORD),
            ("biWidth", wintypes.LONG),
            ("biHeight", wintypes.LONG),
            ("biPlanes", wintypes.WORD),
            ("biBitCount", wintypes.WORD),
            ("biCompression", wintypes.DWORD),
            ("biSizeImage", wintypes.DWORD),
            ("biXPelsPerMeter", wintypes.LONG),
            ("biYPelsPerMeter", wintypes.LONG),
            ("biClrUsed", wintypes.DWORD),
            ("biClrImportant", wintypes.DWORD),
        ]

    bi = BITMAPINFOHEADER()
    bi.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.biWidth = w
    bi.biHeight = -h
    bi.biPlanes = 1
    bi.biBitCount = 32
    bi.biCompression = 0
    buf = ctypes.create_string_buffer(w * h * 4)
    g.GetDIBits(mem, bmp, 0, h, buf, ctypes.byref(bi), 0)
    img = Image.frombuffer("RGBA", (w, h), buf, "raw", "BGRA", 0, 1).convert("RGB")
    img.save(out)

    g.SelectObject(mem, old)
    g.DeleteObject(bmp)
    g.DeleteDC(mem)
    u.ReleaseDC(hwnd, dc)
    print(f"captured {w}x{h} PrintWindow={ok} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())