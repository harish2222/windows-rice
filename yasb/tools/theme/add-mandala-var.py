"""One-off: give every Rangalipi theme block its own --motif-mandala.

The control-center / home-menu / pomodoro panels paint `var(--motif-mandala)`,
but only the active block declared it, so the art disappeared on every theme
switch. Each block already names its own motif SVG (`--motif: url(...motif-X.svg)`),
and the generator wrote a matching `motif-X-mandala.png`, so the variable is
derived from the motif name instead of being hand-maintained per palette.

Idempotent: blocks that already declare --motif-mandala are left alone.
"""

import re
import sys
from pathlib import Path

STYLES = Path(__file__).resolve().parents[2] / "styles.css"
MOTIF_RE = re.compile(r'^(\s*)--motif:\s*url\("(?P<path>[^"]*?motif-(?P<stem>[a-z0-9-]+)\.svg)"\);\s*\r?$')
MANDALA_RE = re.compile(r'^\s*--motif-mandala:')


def main() -> int:
    # newline="" keeps the file's own line endings; the default would rewrite
    # every LF as CRLF on Windows and show up as a whole-file diff.
    with STYLES.open("r", encoding="utf-8", newline="") as fh:
        lines = fh.readlines()
    out: list[str] = []
    added = 0
    skipped = 0

    ending = "\r\n" if lines and lines[0].endswith("\r\n") else "\n"
    for i, line in enumerate(lines):
        out.append(line)
        m = MOTIF_RE.match(line.rstrip("\r\n"))
        if not m:
            continue
        indent = m.group(1)
        stem = m.group("stem")
        png = Path(m.group("path")).with_name(f"motif-{stem}-mandala.png")
        # Already declared (active block) -> leave as authored.
        if i + 1 < len(lines) and MANDALA_RE.match(lines[i + 1].rstrip("\r\n")):
            skipped += 1
            continue
        if not png.exists():
            print(f"missing {png.name} for motif {stem}; skipping", file=sys.stderr)
            return 1
        out.append(f'{indent}--motif-mandala: url("{png.as_posix()}");{ending}')
        added += 1

    with STYLES.open("w", encoding="utf-8", newline="") as fh:
        fh.write("".join(out).replace("\r\n", "\n").replace("\n", ending))
    print(f"added={added} already-present={skipped}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())