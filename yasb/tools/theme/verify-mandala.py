"""Verify every theme block activates its own --motif-mandala.

Two halves:

* **art** -- delegated to `mandala-gen check`, which owns the real assertions
  (22 regions, url matches the block's stem, PNG exists, all byte-unique, and
  every pair far enough apart as a *pattern* via dHash). Re-implementing that
  in Python is exactly the drift this repo keeps hitting.
* **switcher round-trip** -- still checked here: activate each of the 22
  themes in turn, assert the active block's `--motif-mandala` line is bare
  (so `var(--motif-mandala)` resolves and CSS var() resolves), and that the
  file comes back byte-identical afterwards.

The block contract is subtle: an inactive theme is ONE comment chunk whose
opener sits on its `--acrylic` line and whose closer sits on its `--runner`
line, so interior declarations are bare text and must NOT be counted when
looking for "the active block".

Exits non-zero on the first violation.
"""

import re
import subprocess
import sys
from pathlib import Path

THEME = Path(__file__).resolve().parent
YASB = THEME.parents[1]
STYLES = YASB / "styles.css"
SWITCH = THEME / "yasb-theme.exe"
MANDALA_GEN = THEME / "mandala-gen" / "target" / "release" / "mandala-gen.exe"

HEADER = re.compile(r'^(\s*)/\*\s*(?P<name>[^*]*?)\s*(?P<active>- active)?\s*\*/\s*$')
BARE_MANDALA = re.compile(r"^\s*--motif-mandala:\s*url")


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


def check_art() -> int:
    if not MANDALA_GEN.exists():
        print(
            f"{MANDALA_GEN} missing; build it with\n"
            "  cargo build --release --manifest-path tools/theme/mandala-gen/Cargo.toml",
            file=sys.stderr,
        )
        return 1
    return subprocess.run([str(MANDALA_GEN), "check", "--styles", str(STYLES)]).returncode


def check_roundtrip() -> int:
    gold = STYLES.read_text(encoding="utf-8", newline="")

    listing = subprocess.run(
        [str(SWITCH), "list"], capture_output=True, text=True, check=True
    ).stdout
    themes = []
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
        lines = STYLES.read_text(encoding="utf-8", newline="").split("\n")
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
        bare = [l for l in lines[hdr:end] if BARE_MANDALA.match(l)]
        if len(bare) != 1:
            print(f"FAIL [{name}] {len(bare)} bare --motif-mandala in active block", file=sys.stderr)
            failures += 1
            continue
        png = re.search(r"motif-[a-z0-9-]+-mandala\.png", bare[0]).group(0)
        if not (YASB / png).exists():
            print(f"FAIL [{name}] missing {png}", file=sys.stderr)
            failures += 1
            continue
        print(f"ok  {aname:<28} -> {png}")

    # Restore and require byte-identical round-trip.
    subprocess.run([str(SWITCH), "set", "Rangalipi Wine"], capture_output=True, check=True)
    now = STYLES.read_text(encoding="utf-8", newline="")
    if now != gold:
        print("FAIL round-trip is not byte-identical", file=sys.stderr)
        failures += 1
    else:
        print("ok  round-trip byte-identical after 22 switches")

    print(f"failures={failures}")
    return 1 if failures else 0


def main() -> int:
    return check_art() or check_roundtrip()


if __name__ == "__main__":
    raise SystemExit(main())