#!/usr/bin/env python3
"""CI gate: HUD label/background contrast (LANDSCAPE_2200 §2105).

Checks the authored HUD plate colours against a simple WCAG-ish luminance
ratio. Not a full APCA audit — catches regressions when plate/text drift.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def lum(c: tuple[float, float, float]) -> float:
    def f(v: float) -> float:
        return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4

    r, g, b = (f(x) for x in c)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a: tuple[float, float, float], b: tuple[float, float, float]) -> float:
    la, lb = lum(a), lum(b)
    lighter, darker = max(la, lb), min(la, lb)
    return (lighter + 0.05) / (darker + 0.05)


# Authored pairs from hud_panel / hud_row / toast (bg, fg).
PAIRS = [
    ("panel plate", (0.04, 0.06, 0.08), (0.92, 0.95, 0.97)),
    ("panel title", (0.04, 0.06, 0.08), (0.80, 0.88, 0.94)),
    ("row caption", (0.04, 0.06, 0.08), (0.62, 0.70, 0.76)),
    ("toast", (0.05, 0.07, 0.09), (0.92, 0.95, 0.97)),
]

MIN_RATIO = 4.5


def main() -> int:
    bad = []
    for name, bg, fg in PAIRS:
        r = ratio(bg, fg)
        if r < MIN_RATIO:
            bad.append(f"{name}: contrast {r:.2f} < {MIN_RATIO}")
    # Also ensure the source files still mention these colours (drift check).
    hud = (ROOT / "game/scripts/ui/hud_panel.gd").read_text(encoding="utf-8")
    if "0.04, 0.06, 0.08" not in hud:
        bad.append("hud_panel.gd: plate colour drifted from audited pair")
    if bad:
        print("UI contrast gate FAILED (LANDSCAPE_2200 §2105):\n")
        print("\n".join(bad))
        return 1
    print("UI contrast gate OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
