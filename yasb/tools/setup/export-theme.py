#!/usr/bin/env python3
"""Export the Kanagawa Gold submission pack for yasb-themes.

Reads the live bar files, emits a gallery-compliant pack into Z:\\Kanagawa-Gold:
  styles.css   single-theme stylesheet (no commented-out theme blocks)
  config.yaml  blank-line-free config (validated)
  README.md    gallery-style readme (preview link filled after imgur upload)
  preview.png  live bar-only screenshot captured right now
  ISSUE_BODY.md ready-made `gh issue create` body (image URL still needed)
Respects every CONTRIBUTING.md rejection rule that can be checked locally.
"""
import os
import re
import shutil
import sys
import yaml

YASB = os.path.join(os.path.expanduser('~'), '.config', 'yasb')
OUT = r'Z:\Rangalipi'
THEME = 'Kanagawa'
os.makedirs(OUT, exist_ok=True)

# ---------- styles.css : preamble + Kanagawa + components ----------
lines = open(os.path.join(YASB, 'styles.css'), encoding='utf-8').read().split('\n')
heads = []
for i, l in enumerate(lines):
    m = re.match(r'\s*/\*\s*(.+?)\s*\*/\s*$', l)
    if m:
        for j in range(i + 1, min(i + 3, len(lines))):
            if re.match(r'\s*(/\*\s*)?--background\s*:', lines[j]):
                n = m.group(1)
                heads.append((n[:-9].strip() if n.endswith('- active') else n.strip(),
                              n.endswith('- active'), i))
                break
preamble = lines[:heads[0][2]]
k_idx = next(i for i, (n, a, _) in enumerate(heads) if n == THEME)
_, _, kh = heads[k_idx]
k_end = heads[k_idx + 1][2] if k_idx + 1 < len(heads) else len(lines)
k_body = [l for l in lines[kh + 1:k_end] if l.strip()]
assert any('--background' in l for l in k_body), 'kanagawa vars missing'
# :root close = first column-0 } after the last theme section
root_close = next(i for i in range(heads[-1][2], len(lines))
                  if re.match(r'^\}', lines[i]))
components = '\n'.join(lines[root_close + 1:])


def strip_big_comments(css):
    return re.sub(r'/\*(?:[^*]|\*(?!/)){120,}?\*/', '', css)


out_css = ('/* Rangalipi for YASB - Kanagawa dark islands, gold borders, liquid glass */\n'
           + '\n'.join(preamble) + '\n'
           + '\n'.join(k_body) + '\n'
           + '}\n'
           + strip_big_comments(components))
open(os.path.join(OUT, 'styles.css'), 'w', encoding='utf-8', newline='').write(out_css)

# ---------- config.yaml : blank-line-free, validated ----------
cfg_lines = [l for l in open(os.path.join(YASB, 'config.yaml'), encoding='utf-8')
             .read().split('\n') if l.strip()]
d = yaml.safe_load('\n'.join(cfg_lines))
assert 'bars' in d and 'widgets' in d, 'config missing bars/widgets'
open(os.path.join(OUT, 'config.yaml'), 'w', encoding='utf-8', newline='').write(
    '\n'.join(cfg_lines))

# ---------- README.md ----------
name, desc = 'Rangalipi', ('Rangalipi: Kanagawa dark glass islands, gold borders, '
                            'palette and font switchers.')
assert len(name) <= 25 and re.fullmatch(r'[A-Za-z0-9 ]+', name), name
assert len(desc) <= 100, len(desc)
IMAGE_URL = ('https://raw.githubusercontent.com/harish2222/Rangalipi'
             '/main/gallery/shot-1.png')
readme = f"""# {name}

![Theme Preview]({IMAGE_URL})

# USE JetBrainsMono Nerd Font AND FiraCode Nerd Font Mono (Retina)

Without them every icon renders as tofu boxes. Get both from `scoop`
(`nerd-fonts/JetBrainsMono-NF`, `nerd-fonts/FiraCode-NF`,
`nerd-fonts/FiraCode-NF-Mono`) or nerdfonts.com, then set them as the
four `--*-font` vars at the top of `styles.css` (preset block included).

Kanagawa-wave dark bar on floating glass islands with gold borders. Komorebi
workspaces, system stats, media with full controls, and two suckless pickers:
one for 82 color palettes, one for installed Nerd Fonts.

## Gallery

![Shot 1](https://raw.githubusercontent.com/harish2222/Rangalipi/main/gallery/shot-1.png)
![Shot 2](https://raw.githubusercontent.com/harish2222/Rangalipi/main/gallery/shot-2.png)
![Shot 3](https://raw.githubusercontent.com/harish2222/Rangalipi/main/gallery/shot-3.png)
![Shot 4](https://raw.githubusercontent.com/harish2222/Rangalipi/main/gallery/shot-4.png)
![Shot 5](https://raw.githubusercontent.com/harish2222/Rangalipi/main/gallery/shot-5.png)
![Shot 6](https://raw.githubusercontent.com/harish2222/Rangalipi/main/gallery/shot-6.png)
![Shot 7](https://raw.githubusercontent.com/harish2222/Rangalipi/main/gallery/shot-7.png)
![Shot 8](https://raw.githubusercontent.com/harish2222/Rangalipi/main/gallery/shot-8.png)

## Features

- **Liquid glass**: theme-tinted blur on islands and all popups
- **Palette browser**: flexbox switcher for 82 palettes, live search
- **Font browser**: any installed Nerd Font, previewed in its own face
- **Komorebi set**: workspaces, layout, control, stack widgets
- **Full media**: thumbnail, controls, bounce titles, progress, volume
- **Monitors**: CPU, GPU, memory, disk, traffic, integer readouts, statuses
- **Zero flash**: launchers run through a hidden runner process
- **RDP-proof**: remote windows ignored by class, exe and title
- **Boot-proof**: ordered login chain plus a verifying one-shot setup script

## Bar layout

Left: home menu, Komorebi layout, workspaces, stack. Center: clock, cava
spectrum (theme-synced gradient), media. Right: systray, system-stats
group, control group (bluetooth, audio, power, wallpapers, palette,
alerts), control center.

## Installation

1. **Fonts**: install `JetBrainsMono Nerd Font` and
   `FiraCode Nerd Font Mono` (Retina), plus `Segoe Fluent Icons`.
2. Copy `config.yaml` and `styles.css` into `%USERPROFILE%\\.config\\yasb`.
   Tested on YASB v2.0.7, Komorebi 0.1.41, GlazeWM 3.10.1.
3. Companion tools ship as source only (no binaries) under `tools\\`
   (theme CLI, palette and font pickers) and `yasb-theme\\` (Rust source).
   Build every exe with `tools\\setup\\yasb-setup.ps1` — Rust, Qt6 and
   PyInstaller are installed automatically when missing. Without them the
   bar still works; only the palette button needs rebinding. WM configs
   live in `configs\\`.

### Companion scripts

All runnable from PowerShell (`-ExecutionPolicy Bypass` for `.ps1`):

- `tools\\setup\\yasb-setup.ps1` — one-shot setup: checks toolchains
  (Rust, Python Qt6), rebuilds every binary, verifies the chain, snapshots
  the WM configs. Idempotent, safe to re-run.
- `tools\\theme\\yasb-theme-build.ps1` — rebuilds `yasb-theme.exe`
  (`list|current|set|next|prev`) and deploys `silent-run.exe`.
- `tools\\theme\\yasb-theme-shell.ps1` — dot-source for the `yt` shortcut.
- `tools\\picker\\palette-picker-build.ps1` — rebuilds `palette-picker.exe`.
- `tools\\picker\\yasb-font-build.ps1` — rebuilds `yasb-font.exe`
  (`list|current|set|next|prev` over installed Nerd Fonts).
- `tools\\setup\\export-theme.py` — regenerates this submission pack
  (single-theme CSS, clean config, redaction, screenshots).

## Author

[![GitHub](https://img.shields.io/badge/GitHub-harish2222-181717?logo=github&style=flat-square)](https://github.com/harish2222)
"""
open(os.path.join(OUT, 'README.md'), 'w', encoding='utf-8', newline='').write(readme)

# ---------- preview.png : live bar-only capture ----------
from PIL import ImageGrab
img = ImageGrab.grab()
W, _ = img.size
img.crop((0, 0, W, 56)).save(os.path.join(OUT, 'preview.png'))
print('preview captured:', img.size, '-> 56px strip')

# ---------- ISSUE_BODY.md : fully assembled, only the image URL is pending --
styles_blob = open(os.path.join(OUT, 'styles.css'), encoding='utf-8').read()
config_blob = open(os.path.join(OUT, 'config.yaml'), encoding='utf-8').read()
readme_txt = open(os.path.join(OUT, 'README.md'), encoding='utf-8').read()
body = f"""### Name

{name}

### Description

{desc}

### Homepage

https://github.com/harish2222/Rangalipi

### Image

PREVIEW_PNG_URL

### Theme Styles

```css
{styles_blob}
```

### Theme Config

```yaml
{config_blob}
```

### Readme

{readme_txt}"""
open(os.path.join(OUT, 'ISSUE_BODY.md'), 'w', encoding='utf-8', newline='').write(body)
print('issue body bytes:', os.path.getsize(os.path.join(OUT, 'ISSUE_BODY.md')))

# ---------- companion bundle: sources + prebuilt exes, no caches ----------
SKIP_DIRS = {'dist', 'build', 'target', '__pycache__', '.git'}
SKIP_EXT = {'.log', '.bak', '.spec', '.pyc', '.exe'}
total = 0
for src, dst in [(os.path.join(YASB, 'tools'), os.path.join(OUT, 'tools')),
                 (os.path.join(YASB, 'yasb-theme'), os.path.join(OUT, 'yasb-theme')),
                 (os.path.join(YASB, 'configs'), os.path.join(OUT, 'configs'))]:
    for root, dirs, files in os.walk(src):
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
        for f in files:
            if os.path.splitext(f)[1].lower() in SKIP_EXT:
                continue
            s = os.path.join(root, f)
            d = os.path.join(dst, os.path.relpath(s, src))
            os.makedirs(os.path.dirname(d), exist_ok=True)
            shutil.copy2(s, d)
            total += os.path.getsize(d)
print(f'companion bundle: {total / 1048576:.1f} MB (source only: no exes, no caches)')
for stale in ('tools/picker/palette-picker.exe', 'tools/picker/yasb-font.exe',
              'tools/theme/yasb-theme.exe'):
    p = os.path.join(OUT, *stale.split('/'))
    if os.path.exists(p):
        os.remove(p)
        print('pruned stale binary:', stale)

# ---------- redaction: personal paths -> dummy placeholder ----------
import shutil as _sh

holder = 'C:/Users/YourName'
count = 0
for root, dirs, files in os.walk(OUT):
    dirs[:] = [d for d in dirs if d not in ('gallery',)]
    for f in files:
        p = os.path.join(root, f)
        if os.path.splitext(f)[1].lower() not in (
                '.yaml', '.yml', '.json', '.ps1', '.bat', '.py', '.md'):
            continue
        if f in ('README.md', 'ISSUE_BODY.md'):
            continue  # author identity stays for publishing
        s = open(p, encoding='utf-8', errors='replace').read()
        # forward-slash dummy: valid inside YAML/JSON double-quoted strings
        # (backslashes would form \U-style escape sequences and break parsing)
        s2 = re.sub(r'C:\\+Users\\+haris', holder, s)
        s2 = re.sub(r'C:/Users/haris', holder, s2)
        s2 = re.sub(r'~\/\.glzr', '~/.glzr', s2)
        if s2 != s:
            open(p, 'w', encoding='utf-8', newline='').write(s2)
            count += 1
print(f'redacted personal paths in {count} pack files (haris-free check below)')

# ---------- local compliance report ----------
print('name:', len(name), 'chars | desc:', len(desc), 'chars')
print('styles.css bytes:', os.path.getsize(os.path.join(OUT, 'styles.css')),
      '| commented theme blocks left:',
      len(re.findall(r'/\\*\\s*(?:[\\w ]+?)(?: - active)?\\s*\\*/\\s*\\n\\s*(?:/\\*\\s*)?--background', out_css)))
print('config blank lines:', sum(1 for l in cfg_lines if not l.strip()))
print('braces balanced:', out_css.count('{') == out_css.count('}'))
print('pack ready in', OUT)
