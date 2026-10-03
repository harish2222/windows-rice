#!/usr/bin/env python3
"""Screenshot the screen INCLUDING layered windows.

Plain `BitBlt` from the screen DC silently skips layered windows, and
`PrintWindow` returns black for them, so both `PIL.ImageGrab` and the
usual PrintWindow trick fail on windows drawn with `UpdateLayeredWindow`
(what palette-picker and saka-popup use for per-pixel alpha).

`BitBlt` with the `CAPTUREBLT` raster operation includes layered windows,
so this is the way to actually see them.

Usage: capture-screen.py <out.png> [crop x0 y0 x1 y1]
"""
import ctypes
from ctypes import wintypes

import sys
from PIL import Image

u = ctypes.windll.user32
g = ctypes.windll.gdi32

SRCCOPY = 0x00CC0020
CAPTUREBLT = 0x40000000
DIB_RGB_COLORS = 0


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


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    out = sys.argv[1]
    u.SetProcessDPIAware()
    w = u.GetSystemMetrics(0)
    h = u.GetSystemMetrics(1)

    dc = u.GetDC(None)
    mem = g.CreateCompatibleDC(dc)
    bmp = g.CreateCompatibleBitmap(dc, w, h)
    old = g.SelectObject(mem, bmp)
    ok = g.BitBlt(mem, 0, 0, w, h, dc, 0, 0, SRCCOPY | CAPTUREBLT)

    bi = BITMAPINFOHEADER()
    bi.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.biWidth = w
    bi.biHeight = -h
    bi.biPlanes = 1
    bi.biBitCount = 32
    bi.biCompression = 0
    buf = ctypes.create_string_buffer(w * h * 4)
    g.GetDIBits(mem, bmp, 0, h, buf, ctypes.byref(bi), DIB_RGB_COLORS)
    img = Image.frombuffer("RGBA", (w, h), buf, "raw", "BGRA", 0, 1).convert("RGB")

    if len(sys.argv) >= 7:
        img = img.crop(tuple(int(v) for v in sys.argv[2:7]))
    img.save(out)

    g.SelectObject(mem, old)
    g.DeleteObject(bmp)
    g.DeleteDC(mem)
    u.ReleaseDC(None, dc)
    distinct = len(img.getcolors(maxcolors=100000) or [])
    print(f"captured {img.size} BitBlt={ok} distinct-colours={distinct} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())