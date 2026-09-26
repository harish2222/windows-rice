#!/usr/bin/env python3
"""Build the submittable issue body (GitHub caps bodies at 65536 chars).

Reads Z:\\Rangalipi pack files, emits Z:\\Rangalipi\\submission\\ with
compact-but-complete styles.css + config.yaml, revalidates with the
bot-equivalent validator, and assembles FINAL_ISSUE_BODY.md.
Asserts total body <= 65000 bytes. Live files are never touched.
"""
import os
import re
import sys

Z = r'Z:\Rangalipi'
SUB = os.path.join(Z, 'submission')
os.makedirs(SUB, exist_ok=True)

# ---------- compact CSS: one rule per line, comments stripped ----------
css = open(os.path.join(Z, 'styles.css'), encoding='utf-8').read()
css_nocom = re.sub(r'/\*.*?\*/', '', css, flags=re.S)
rules = []
for m in re.finditer(r'([^{}]+)\{([^{}]*)\}', css_nocom):
    sel = re.sub(r'\s+', ' ', m.group(1)).strip()
    sel = re.sub(r'\s*,\s*', ', ', sel)
    decls = [d.strip() for d in m.group(2).split(';') if d.strip()]
    decls = [re.sub(r'\s*:\s*', ': ', d, count=1) for d in decls]
    rules.append(f'{sel} {{{"; ".join(decls)}}}')
before_decls = sorted(re.findall(r'([\w-]+)\s*:', css_nocom))
compact_css = '\n'.join(rules) + '\n'
after_decls = sorted(re.findall(r'([\w-]+)\s*:', compact_css))
assert before_decls == after_decls, 'declaration set changed!'
open(os.path.join(SUB, 'styles.css'), 'w', encoding='utf-8', newline='').write(compact_css)

# ---------- config: drop full-line comments, then strip schema defaults --
# (identical behavior: omitted values are schema defaults verbatim) --------
ylines = [l for l in open(os.path.join(Z, 'config.yaml'), encoding='utf-8')
          .read().split('\n') if not re.match(r'\s*#', l)]
import yaml
yaml.safe_load('\n'.join(ylines))
open(os.path.join(SUB, 'config.yaml'), 'w', encoding='utf-8', newline='').write(
    '\n'.join(ylines))
import subprocess as _sp
_val = r'C:\Users\haris\AppData\Local\Temp\kilo\valenv\Scripts\python.exe'
_shr = r'C:\Users\haris\AppData\Local\Temp\kilo\shrink.py'
_r = _sp.run([_val, _shr], capture_output=True, text=True, timeout=300)
print(_r.stdout.strip().split('\n')[-2:])
ylines = open(os.path.join(SUB, 'config.yaml'), encoding='utf-8').read().split('\n')
yaml.safe_load('\n'.join(ylines))

# ---------- bot-equivalent validation on the exact submitted bytes ----------
sys.path.insert(0, r'C:\Users\haris\AppData\Local\Temp\kilo\yasb-main\src')
from core.validation.deprecation import migrate_config
from core.validation.config import YasbConfig
patched, issues = migrate_config('\n'.join(ylines))
assert not issues, issues[:3]
YasbConfig.model_validate(yaml.safe_load(patched))
print('submission config: bot validation PASS')

# ---------- assemble body ----------
name = 'Rangalipi'
desc = ('Rangalipi: Kanagawa dark glass islands, gold borders.')
readme = """# Rangalipi

![Theme Preview](https://raw.githubusercontent.com/harish2222/Rangalipi/main/preview.png)

Kanagawa dark bar on floating glass islands with gold borders, full Komorebi
widget set, complete media controls, and suckless palette + font browsers.

## Features

- Liquid glass islands and popups, theme-tinted blur
- Palette browser: 82 themes, search, arrows + Enter
- Font browser: any installed Nerd Font, live previews
- Komorebi + media, zero-flash launchers

## Installation

1. Fonts: `JetBrainsMono Nerd Font` and `FiraCode Nerd Font Mono`.
2. Copy `config.yaml` + `styles.css` to `%USERPROFILE%\\.config\\yasb`.

## Author: harish2222
"""
body = (f'### Name\n\n{name}\n\n### Description\n\n{desc}\n\n### Homepage\n\n'
        f'https://github.com/harish2222/Rangalipi\n\n### Image\n\n'
        f'https://raw.githubusercontent.com/harish2222/Rangalipi/main/preview.png\n\n'
        f'### Theme Styles\n\n```css\n{compact_css}```\n\n'
        f'### Theme Config\n\n```yaml\n' + '\n'.join(ylines) + '\n```\n\n'
        f'### Readme\n\n' + readme)
open(os.path.join(SUB, 'FINAL_ISSUE_BODY.md'), 'w', encoding='utf-8',
     newline='').write(body)
n = len(body.encode('utf-8'))
print(f'body bytes: {n} (cap 65536, headroom {65536 - n})')
assert n <= 65000, 'STILL OVER CAP'
print('submission build OK')
