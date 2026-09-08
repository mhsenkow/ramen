#!/usr/bin/env python3
"""CI gate: lighting shaders must include the shared colour/light contract (§2002)."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = {
    "terrain.gdshader",
    "grass.gdshader",
    "tree.gdshader",
    "prop.gdshader",
    "spoil.gdshader",
    "pool.gdshader",
    "channel.gdshader",
    "water.gdshader",
    "endcap.gdshader",
}
MARKER = '#include "rama_light.gdshaderinc"'


def main() -> int:
    shader_dir = ROOT / "game" / "shaders"
    bad = []
    for name in sorted(REQUIRED):
        text = (shader_dir / name).read_text(encoding="utf-8")
        if MARKER not in text:
            bad.append(f"{name}: missing {MARKER}")
        if "srgb_to_linear" not in text and name != "endcap.gdshader":
            # endcap uses it; others must too for palette paths
            pass
        if "srgb_to_linear" not in text:
            bad.append(f"{name}: must call srgb_to_linear (or document opt-out)")
    if bad:
        print("shader include gate FAILED (LANDSCAPE_2200 §2002):\n")
        print("\n".join(bad))
        return 1
    print("shader include gate OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
