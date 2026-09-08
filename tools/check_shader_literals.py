#!/usr/bin/env python3
"""CI gate: bare metre literals in shaders (LANDSCAPE_2200 §2008).

Flags float literals >= 100.0 that look like habitat distances baked into
expressions — the class of bug that crushed terrain lighting when the drum
grew from 600 m to 900 m.

Allows:
  - uniform default values
  - expressions that already scale by hab_radius / hab_len / hab_length
  - lines tagged // metre-ok or // feature: (intentional falloffs)
  - canvas_item post shaders (not habitat metres)
  - hash / noise magic numbers (very large, or on hash/fract(sin lines)
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHADER_DIR = ROOT / "game" / "shaders"

FLOAT_RE = re.compile(r"(?<![\w.])(\d{3,}(?:\.\d+)?|\d{2,}\.\d+)(?![\w.])")
UNIFORM_RE = re.compile(r"\buniform\b")
HAB_RE = re.compile(r"\bhab_(?:radius|len|length)\b")
OK_RE = re.compile(r"//\s*(metre-ok|feature:)", re.I)
HASH_RE = re.compile(r"\b(hash|fract\s*\(\s*sin|43758|289\.1)\b")
CANVAS_RE = re.compile(r"shader_type\s+canvas_item")


def check_file(path: Path) -> list[str]:
    hits: list[str] = []
    text = path.read_text(encoding="utf-8")
    if CANVAS_RE.search(text):
        return hits
    if path.suffix == ".gdshaderinc":
        return hits
    lines = text.splitlines()
    for i, line in enumerate(lines, 1):
        code = line.split("//", 1)[0]
        if UNIFORM_RE.search(code):
            continue
        if HAB_RE.search(code):
            continue
        if OK_RE.search(line):
            continue
        # Look back through recent comment lines for a feature-size note.
        lookback = "\n".join(lines[max(0, i - 5) : i])
        if OK_RE.search(lookback) or re.search(r"~?\d+(\.\d+)?\s*m\b", lookback):
            continue
        if HASH_RE.search(line):
            continue
        for m in FLOAT_RE.finditer(code):
            try:
                v = float(m.group(1))
            except ValueError:
                continue
            if v >= 10000.0:
                continue  # noise magic
            if v >= 100.0:
                hits.append(
                    f"{path.relative_to(ROOT)}:{i}: literal {m.group(1)} — "
                    f"scale by hab_* or tag // metre-ok / // feature:"
                )
    return hits


def main() -> int:
    bad: list[str] = []
    for p in sorted(SHADER_DIR.glob("*.gdshader")):
        bad.extend(check_file(p))
    if bad:
        print("shader metre-literal gate FAILED (LANDSCAPE_2200 §2008):\n")
        print("\n".join(bad))
        return 1
    print("shader metre-literal gate OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
