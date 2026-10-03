"""Verify every theme block activates its own --motif-mandala.

The block contract is subtle: an inactive theme is ONE comment chunk whose
opener sits on its `--acrylic` line and whose closer sits on its `--runner`
line, so interior declarations are bare text and must NOT be counted when
looking for "the active block". This walks the same region structure
yasb-theme's parser uses and checks, for each of the 22 themes:

  * exactly one block is active (bare opener + `- active` header),
  * that block's `--motif-mandala` is bare (so CSS var() resolves),
  * the referenced PNG actually exists,
  * no other block leaks a bare mandala.

Exits non-zero on the first violation.
"""

import re
import sys
from pathlib import Path

YASB = Path(__file__).resolve().parents[2]
STYLES = YASB / "styles.css"
SWITCH = YASB / "tools" / "theme" / "yasb-theme.exe"

HEADER = re.compile(r'^(\s*)/\*\s*(?P<name>[^*]*?)\s*(?P<active>- active)?\s*\*/\s*$')


def regions(lines):
    """(name, active_flag, indices) per theme block, mirroring the Rust parser."""
    out = []
    for i, line in enumerate(lines):
        m = HEADER.match(line)
        if not m:
            continue
        name = m.group("name").strip()
        # Skip prose comments: only name-like inners are theme headers.
        if not name or not any(c.isalpha() for c in name):
            continue
        if not all(c.isalnum() or c in " -" for c in name):
            continue
        if out:
            out[-1][2].append(i)  # previous block ended here
        out.append([name, bool(m.group("active")), []])
    return out


def main() -> int:
    gold = STYLES.read_text(encoding="utf-8", newline="").replace("\r\n", "\n")

    import subprocess

    listing = subprocess.run(
        [str(SWITCH), "list"], capture_output=True, text=True, check=True
    ).stdout
    themes: list[str] = []
    # `list` prints one theme per line, each prefixed by "* " when active.
    for raw in listing.replace("\r", "").splitlines():
        name = raw.strip().lstrip("*").strip()
        if name:
            themes.append(name)

    if len(themes) != 22:
        print(f"expected 22 themes, got {len(themes)}", file=sys.stderr)
        return 1

    failures = 0
    for name in themes:
        subprocess.run([str(SWITCH), "set", name], capture_output=True, check=True)
        text = STYLES.read_text(encoding="utf-8", newline="").replace("\r\n", "\n")
        lines = text.split("\n")
        blocks = regions(lines)
        active = [b for b in blocks if b[1]]
        if len(active) != 1:
            print(f"FAIL [{name}] {len(active)} starred headers", file=sys.stderr)
            failures += 1
            continue

        aname = active[0][0]
        # The active block runs from its header comment to the next header.
        hdr = next(
            i
            for i, l in enumerate(lines)
            if (m := HEADER.match(l)) and m.group("name").strip() == aname
        )
        end = next((i for i in range(hdr + 1, len(lines)) if HEADER.match(lines[i])), len(lines))
        chunk = lines[hdr:end]

        bare = [l for l in chunk if re.match(r"^\s*--motif-mandala:\s*url", l)]
        if len(bare) != 1:
            print(f"FAIL [{name}] {len(bare)} bare --motif-mandala in active block", file=sys.stderr)
            failures += 1
            continue
        png = re.search(r'motif-([a-z0-9-]+)-mandala\.png', bare[0]).group(0)
        if not (YASB / png).exists():
            print(f"FAIL [{name}] missing {png}", file=sys.stderr)
            failures += 1
            continue
        print(f"ok  {aname:<28} -> {png}")

    # Restore and require byte-identical round-trip.
    subprocess.run([str(SWITCH), "set", "Rangalipi Wine"], capture_output=True, check=True)
    now = STYLES.read_text(encoding="utf-8", newline="").replace("\r\n", "\n")
    if now != gold:
        print("FAIL round-trip is not byte-identical", file=sys.stderr)
        failures += 1
    else:
        print("ok  round-trip byte-identical after 22 switches")

    print(f"failures={failures}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())