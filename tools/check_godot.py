#!/usr/bin/env python3
"""Godot load gate: scripts compile, shaders compile, scenes resolve.

Runs `game/scripts/debug/ci_check.gd` headless and fails on any error Godot
reports. Sibling of the other `tools/check_*.py` craft gates.

    python3 tools/check_godot.py            # gate the tree
    python3 tools/check_godot.py --verify   # gate the gate

Finding Godot, in order: $GODOT, $GODOT_BIN, `godot` / `godot4` on PATH, then
the usual macOS app bundle. CI uses `chickensoft-games/setup-godot`, which puts
`godot` on PATH.

## Why this wrapper exists rather than asserting inside the engine

Godot hands back a usable-looking object for a file it failed to compile: both a
broken script and a broken shader `load()` non-null. Scripts have a real
predicate (`GDScript.reload()` returns an error code) and `ci_check.gd` uses it.
Shaders have none — a valid shader may legitimately declare zero uniforms, so
uniform count proves nothing, and there is no exposed compile status. What a
broken shader *does* do is make Godot print `SHADER ERROR`.

So for shaders this output scan is the only thing standing between a syntax
error and a release. That is why the gate lives out here: not decoration around
the harness, but the half of it that shaders depend on.

Exit 0 clean, 1 on a reported problem, 2 if Godot could not be run at all (so a
missing toolchain is never mistaken for a passing gate).
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PROJECT = ROOT / "game"
# Native extension libraries, by platform. Checked before Godot is launched.
NATIVE_LIBS = ("bin/librama_sim.dylib", "bin/librama_sim.so", "bin/rama_sim.dll")
HARNESS = "res://scripts/debug/ci_check.gd"
TIMEOUT_S = 300

# Substrings that mean Godot reported a real failure. Kept as plain substrings
# so they survive Godot's formatting changes better than anchored patterns.
ERROR_SIGNATURES = (
    "SCRIPT ERROR",
    "SHADER ERROR",
    "Parse Error",
    "Failed to load script",
    "Shader compilation failed",
    "Failed loading resource",
    "ci_check FAIL",  # the harness's own printerr lines
)

# Benign lines that contain a signature but are not failures. Anything added
# here must be justified — this list is how a real gate quietly becomes a
# rubber stamp.
ALLOWLIST: tuple[re.Pattern[str], ...] = ()


def find_godot() -> str | None:
    for var in ("GODOT", "GODOT_BIN"):
        cand = os.environ.get(var)
        if cand and (shutil.which(cand) or Path(cand).is_file()):
            return cand
    for name in ("godot", "godot4"):
        found = shutil.which(name)
        if found:
            return found
    mac = Path("/Applications/Godot.app/Contents/MacOS/Godot")
    if mac.is_file():
        return str(mac)
    return None


def signature_hint() -> str | None:
    """Why a macOS run might die with no output at all.

    Overwriting the dylib in place invalidates its ad-hoc signature and the
    kernel then SIGKILLs Godot at load, printing nothing — so the gate looks
    broken rather than the library. `build.sh` documents the remove-copy-re-sign
    dance; this catches the case where someone (me) copied it by hand instead.
    """
    if sys.platform != "darwin":
        return None
    lib = PROJECT / "bin/librama_sim.dylib"
    if not lib.is_file():
        return None
    try:
        r = subprocess.run(
            ["codesign", "--verify", "--strict", str(lib)],
            capture_output=True, text=True, timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if r.returncode != 0:
        return (
            f"{lib.relative_to(ROOT)} has no valid signature "
            f"({(r.stderr or '').strip().splitlines()[-1] if r.stderr.strip() else 'codesign failed'}).\n"
            "  On Apple Silicon the kernel SIGKILLs Godot at load with no output.\n"
            "  Fix: ./build.sh   (or: codesign --force --sign - "
            f"{lib.relative_to(ROOT)})"
        )
    return None


def is_error(line: str) -> bool:
    if not any(sig in line for sig in ERROR_SIGNATURES):
        return False
    return not any(p.search(line) for p in ALLOWLIST)


def run_harness(godot: str, project: Path) -> tuple[int, str]:
    """Run the harness against `project`. Returns (exit code, combined output).

    Exit code -1 means the run itself did not complete (timeout or OS error).
    """
    cmd = [godot, "--headless", "--path", str(project), "--script", HARNESS]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=TIMEOUT_S)
    except subprocess.TimeoutExpired:
        return -1, "TIMEOUT"
    except OSError as exc:
        return -1, f"OSERROR {exc}"
    return proc.returncode, (proc.stdout or "") + (proc.stderr or "")


# Breakage injected by --verify. If the gate cannot see these, it cannot see
# anything, and its green tick is worthless.
PROBE_SHADER = "shader_type spatial;\nvoid fragment() { ALBEDO = vec3(1.0) \n"
PROBE_SCRIPT = "extends Node\nfunc broken(:\n\treturn 1\n"


def verify(godot: str) -> int:
    """Prove the gate still detects breakage on *this* platform and engine.

    A gate whose error signatures have gone stale reports success on a broken
    tree, which is worse than no gate. Both failure modes this guards against
    were observed for real while writing it: `load()` returns a non-null object
    for a file that failed to compile, and a Shader is not compiled until
    something queries it, so an unlucky implementation catches nothing while
    looking thorough.
    """
    with tempfile.TemporaryDirectory() as td:
        proj = Path(td) / "game"
        shutil.copytree(PROJECT, proj, ignore=shutil.ignore_patterns(".godot"))
        (proj / "shaders" / "zz_gate_probe.gdshader").write_text(PROBE_SHADER)
        (proj / "scripts" / "zz_gate_probe.gd").write_text(PROBE_SCRIPT)
        code, output = run_harness(godot, proj)

    if code == -1:
        print(f"Gate self-check COULD NOT RUN: {output}", file=sys.stderr)
        return 2

    missed = []
    if not any("SHADER ERROR" in ln or "Shader compilation failed" in ln
               for ln in output.splitlines()):
        missed.append(
            "broken SHADER not detected — a syntax error would reach a release"
        )
    if not any("zz_gate_probe.gd" in ln and is_error(ln) for ln in output.splitlines()):
        missed.append("broken SCRIPT not detected")
    if code == 0:
        missed.append("harness exited 0 on a broken project")

    if missed:
        print("Gate self-check FAILED — this gate is not catching what it claims:",
              file=sys.stderr)
        for m in missed:
            print(f"  - {m}", file=sys.stderr)
        print("\n  Godot's error wording may have changed; update ERROR_SIGNATURES.",
              file=sys.stderr)
        return 1
    print("Gate self-check OK — injected shader and script errors were both caught")
    return 0


def main(argv: list[str]) -> int:
    godot = find_godot()
    if godot is None:
        print(
            "Godot load gate COULD NOT RUN: no Godot binary found.\n"
            "  Set $GODOT to the binary, or put `godot` on PATH.",
            file=sys.stderr,
        )
        return 2
    if not (PROJECT / "project.godot").is_file():
        print(f"Godot load gate COULD NOT RUN: no project at {PROJECT}", file=sys.stderr)
        return 2

    bad_sig = signature_hint()
    if bad_sig is not None:
        print(f"Godot load gate FAILED (native library)\n\n  {bad_sig}", file=sys.stderr)
        return 1

    if "--verify" in argv:
        return verify(godot)

    returncode, output = run_harness(godot, PROJECT)
    if returncode == -1:
        if output == "TIMEOUT":
            print(
                f"Godot load gate FAILED: harness exceeded {TIMEOUT_S}s.\n"
                "  It must not boot main.tscn — world generation does not finish.",
                file=sys.stderr,
            )
            return 1
        print(f"Godot load gate COULD NOT RUN: {output}", file=sys.stderr)
        return 2

    problems = [ln.rstrip() for ln in output.splitlines() if is_error(ln)]

    # Trust neither signal alone: the harness reports what it can name, the
    # output scan catches what it cannot (shaders), and a crash shows up only
    # in the exit code.
    if returncode != 0 or problems:
        print("Godot load gate FAILED (§2002 family — headless load)\n", file=sys.stderr)
        if problems:
            seen: set[str] = set()
            for ln in problems:
                if ln not in seen:
                    seen.add(ln)
                    print(f"  {ln}", file=sys.stderr)
        if returncode != 0 and not problems:
            print(
                f"  harness exited {returncode} with no recognised error line",
                file=sys.stderr,
            )
            if returncode in (-9, 137):
                print(
                    "  Killed with no output — on macOS that is almost always the\n"
                    "  native library's signature. Try ./build.sh to re-sign it.",
                    file=sys.stderr,
                )
            tail = [ln for ln in output.splitlines() if ln.strip()][-15:]
            for ln in tail:
                print(f"  | {ln}", file=sys.stderr)
        return 1

    summary = next(
        (ln for ln in output.splitlines() if ln.startswith("ci_check OK")), ""
    )
    print(f"Godot load gate OK{' — ' + summary.removeprefix('ci_check OK — ') if summary else ''}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
