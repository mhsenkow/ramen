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
    # Field book (hud.gd). The inactive tab is the one worth watching: it sits
    # on a lighter plate than the panels do, so it has the least headroom.
    ("book page", (0.04, 0.06, 0.08), (0.92, 0.95, 0.97)),
    ("tab active", (0.16, 0.20, 0.24), (0.96, 0.97, 0.98)),
    ("tab inactive", (0.07, 0.09, 0.11), (0.74, 0.81, 0.87)),
    ("book hint", (0.04, 0.06, 0.08), (0.72, 0.79, 0.85)),
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
    # The tabbed panes share one palette (ui/tabs.gd), so check the constants
    # themselves rather than formatted literals buried in StyleBoxes.
    book = (ROOT / "game/scripts/ui/tabs.gd").read_text(encoding="utf-8")
    for const, want in (
        ("PANE_BG", "0.04, 0.06, 0.08"),
        ("TAB_ON_BG", "0.16, 0.20, 0.24"),
        ("TAB_OFF_BG", "0.07, 0.09, 0.11"),
        ("TAB_ON_FG", "0.96, 0.97, 0.98"),
        ("TAB_OFF_FG", "0.74, 0.81, 0.87"),
        ("HINT_FG", "0.72, 0.79, 0.85"),
    ):
        if f"{const} := Color({want}" not in book:
            bad.append(f"ui/tabs.gd: {const} drifted from its audited pair")
    if bad:
        print("UI contrast gate FAILED (LANDSCAPE_2200 §2105):\n")
        print("\n".join(bad))
        return 1
    print("UI contrast gate OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
