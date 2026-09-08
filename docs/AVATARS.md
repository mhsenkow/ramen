# Avatars — builds and the way they move

**Code:** `game/scripts/avatar/body.gd`, `game/scripts/avatar/gait.gd`
**Preview:** `Godot --path game -- --parade` · in play, **F2 / F3**
**Creator:** Esc → Build

## The one idea

A body is a **dictionary of numbers**. A gait is a **dictionary of numbers**. An
archetype is a **named point** in those spaces, and nothing downstream knows the
names — `gait.pose()` has never heard of a bear.

That is what makes it a system rather than eight statues:

- any two archetypes blend, so the eight are a **spectrum**, not a menu;
- a hand-built body still moves plausibly, because `gait.for_body()` derives a
  walk from the shape (mass widens the track, muscle locks the thorax and holds
  the arms out, limb length lengthens the stride);
- adding an axis is one line in `AXES` and one line in `measure()`. Blending,
  saving, randomising and the creator UI all pick it up for free.

Everything is a **ratio of stature**. A 1.70 m twink and a 1.99 m lanky are the
same construction at different scales, and the same code poses both. The only
metres in the whole system come out of `measure()`.

## Body axes

| Axis | 0 | 1 |
|---|---|---|
| `stature` | 1.52 m | 2.10 m |
| `mass` | slight | heavy |
| `muscle` | untrained | competition |
| `soft` | cut | plush |
| `taper` | straight | extreme V |
| `limb` | stocky | leggy |
| `neck` | slender | traps to the ears |
| `head` | small (reads older) | large (reads younger) |
| `hair` `beard` `fur` | none | full |

`muscle` and `soft` are **independent on purpose**. A muscle daddy is high on
both. A competition build is high on one. That distinction is the whole point,
and a single "weight" slider cannot express it.

Where the mass goes is decided in `measure()`: muscle goes up and out —
shoulders, arms, chest, quads — and soft goes forward and down — belly, hips,
face, upper arm. The two shapes that fall out of that are the whole silhouette:

```
             shoulders   waist    ratio
jock            0.55      0.33     1.67   ← the V
bear            0.49      0.55     0.89   ← wider at the waist than the shoulders
```

The selftest asserts both of those, because archetypes drifting toward each
other is the failure mode a body system dies of.

## Gait axes

Two rules the whole file is built on.

**Phase advances with DISTANCE, not time.** Stride length is a real measurement
in metres, so the foot cannot skate — walking, running, uphill, encumbered, it is
the same relationship. Speed changes the gait by changing stride and cadence,
which is what it does to a person. Above a comfortable pace the stride stops
growing and cadence takes over, which is why a run is not a fast walk.

**Every event has a place in the cycle.** `_bump(phase, at, width)` puts a
movement at a phase — heel strike at 0, toe-off just before π, knee peak in
mid-swing, the dip of arriving weight just after each contact — instead of
layering sines and hoping. That is why these read as different walks rather than
one walk at different amplitudes.

Phase 0 is initial contact for the right foot; 0..π is right stance, π..2π right
swing, and the left is the same half a cycle later.

## What makes each one recognisable

Not amplitude. Each of these is a specific mechanism.

**Twink** — short quick stride, and the feet land **inside** the hip line
(`cross`), which is what makes the walk read from behind. Big pelvic drop and a
figure-eight second beat in the sway (`swish`), loose arms swinging wide and
crossing the body, high bounce.

**Jock** — the lats do two things, and both are in the numbers. They hold the
arms off the ribs, so he *cannot* swing them inward: tiny `arm_swing`, huge
`arm_carry`, forearms pronated. And they lock the thorax to the pelvis, so
`chest_counter` is near zero. What is left is a wide, braced, shoulder-driven
roll — `shoulder_roll` is the largest in the set. Everyone recognises it.

**Muscle daddy** — the same locked frame, twenty years of not hurrying. Longer
stride, slower cadence, more weight in every plant (`settle`), leaning back
rather than forward.

**Bear** — a waddle is not a wobble. The mass is too wide to swing a leg through
the midline, so instead of passing the leg under the body he **rolls the body
over the leg**: high `waddle`, wide `track`, short stride, real `hip_drop`, and
a heavy `settle` onto each foot. The shoulders rock because the chest does not
fight it.

**Cub** — that waddle with the brakes off.

**Otter / daddy** — the middle of the range. Daddy's swagger is `shoulder_roll`
*without* the bracing, which is the one thing the jock's lats will not let him do.

**Lanky** — long shanks, no mass to carry, so the cycle is slow and enormous: the
longest stride in the set, the knee coming up to the chest, near-straight arms
in full arcs, real hang time, and `head_lead` so the head arrives first and the
rest catches up. (This is the long-limbed cartoon-alien run; if you had a
specific show in mind, the numbers to move are `stride`, `knee_lift`, `bounce`
and `head_lead`.)

## Who gets one

| | |
|---|---|
| **The player** | Full rig. Build from `RamaControls.avatar` — archetype, optional blend toward a second, then any hand-set axes, in that order. |
| **Colonists within 62 m** | Full rig, from a pool of six. Sticky slots, so two men swapping rank does not rebuild both. |
| **Everyone else** | One MultiMesh **per archetype**, drawn from `RamaBody.bake()` — the same construction, merged. A bear reads as a bear across a field. |

A colonist's archetype comes from **his id and nothing else**, so it is stable
for the life of the save with no table to persist, and `RamaBody.vary()` then
moves every axis a little off it — one man, not one of eight statues.

Near rigs **chase** their sim position at their own comfortable speed rather than
snapping to it, and the gait is driven by the distance they actually covered. So
arrival, hesitation and idle fall out for free, and the feet always mean
something.

## Colour

Source-mesh vertex colours are authored as **display values in the 0.45–0.85
band**, because a MultiMesh instance colour multiplies them and `prop.gdshader`
then raises the product to 1.95. See `RENDER_CONTRACT.md` §2009 — this is the
same rule that turned litter into black pebbles.

Far-tier instance colours are therefore a near-white **tint** (mood, hunger,
fatigue), not a replacement.
