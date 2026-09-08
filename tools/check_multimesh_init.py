#!/usr/bin/env python3
"""CI gate: MultiMesh use_colors + instance_count without colour init (§2009–2010).

Any block that sets instance_count to a literal > 0 while use_colors is true
must also call set_instance_color (or set colours in a loop) before the
function ends — otherwise instances render opaque white until a later refresh.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT_DIRS = [ROOT / "game" / "scripts"]

COUNT_RE = re.compile(r"\.instance_count\s*=\s*(\d+)")
USE_COLORS_RE = re.compile(r"\.use_colors\s*=\s*true")
SET_COLOR_RE = re.compile(r"set_instance_color\s*\(")
FUNC_RE = re.compile(r"^func\s+")


def check_file(path: Path) -> list[str]:
    hits: list[str] = []
    lines = path.read_text(encoding="utf-8").splitlines()
    i = 0
    while i < len(lines):
        if not FUNC_RE.match(lines[i]):
            i += 1
            continue
        # Collect function body until next top-level func or EOF.
        start = i
        i += 1
        body_start = i
        while i < len(lines) and not FUNC_RE.match(lines[i]):
            i += 1
        body = "\n".join(lines[body_start:i])
        if not USE_COLORS_RE.search(body):
            continue
        for m in COUNT_RE.finditer(body):
            n = int(m.group(1))
            if n <= 0:
                continue
            # Find line number of this assignment within the file.
            abs_line = body_start + body[: m.start()].count("\n") + 1
            # Look ahead from this assignment to end of function for color init.
            after = body[m.end() :]
            if SET_COLOR_RE.search(after):
                continue
            hits.append(
                f"{path.relative_to(ROOT)}:{abs_line}: instance_count={n} with "
                f"use_colors but no set_instance_color in the same function "
                f"(LANDSCAPE_2200 §2009–2010)"
            )
    return hits


def main() -> int:
    bad: list[str] = []
    for d in SCRIPT_DIRS:
        for p in sorted(d.rglob("*.gd")):
            bad.extend(check_file(p))
    if bad:
        print("MultiMesh colour-init gate FAILED:\n")
        print("\n".join(bad))
        return 1
    print("MultiMesh colour-init gate OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
