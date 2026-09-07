# Product Brief — *RAMA CYCLE* (working title)

**Status:** Draft v0.1 · **Owner:** Product · **Date:** 2026-09-06

---

## 1. One-liner

A generation-ship colony sim where you farm, mine and hunt across the curved
inner surface of a rotating space habitat — and where the only way the colony
survives past your lifetime is the men you choose to bond with.

## 2. The premise

A multi-century interstellar transit. The crew complement is entirely gay men —
in-fiction, because of a selected germline trait that confers unusual resistance
to cosmic-ray genetic damage, making them the only viable long-haul lineage
stock. The consequence the fiction has to pay for: **this crew cannot reproduce
sexually.** Continuation runs through the Crèche — in-vitro gametogenesis,
ectogenesis, and a lineage committee that decides which pairings get to become
the next generation.

That premise is not decoration. It is the load-bearing beam that ties the three
genres together (see §4).

## 3. Pillars

| # | Pillar | Reference points | Confidence |
|---|--------|------------------|-----------|
| P1 | **Habitat as world** — third-person survival/build sim on the interior of rotating habitats | Minecraft, Astroneer, Halo's ring | High |
| P2 | **Livelihood loop** — farming, mining, hunting, crafting, **cooking** (ramen as the signature bowl: biome gardens → kitchen grade) | Stardew Valley, Harvest Moon | High |
| P3 | **Relationship sim** — deep, branching, stat- and memory-driven romance with a cast of men | Stardew + *Coming Out on Top*, *Boyfriend Dungeon* | High |
| P4 | **Shared habitat** — asynchronous multiplayer social layer | Stardew co-op, Animal Crossing island visits | Medium |
| P5 | **The Archive** — live-action photoreal video, diegetically framed. Kisses, not sex. | *Immortality*, *Her Story*, FMV-era C&C | Medium-high |

## 4. The thing that makes it one game: the Crèche

Every pillar feeds one resource: **Continuity.**

```
Farm/mine/hunt output ──► habitat carrying capacity ──┐
                                                      ├──► Crèche slots
Bond depth with a partner ──► genome pairing quality ─┘        │
                                                               ▼
                                                     Next-generation colonists
                                                               │
                              new NPCs, new labor, new habitat ring unlocked
```

- You cannot romance your way to a good ending; the Crèche needs the farm.
- You cannot grind your way to a good ending; the Crèche needs a real bond.
- Ending state = what the colony looks like three generations after you die.

This is the pitch. Lead with it. "Gay Stardew in space" is the hook; **"the
farm is a fertility system and the fertility system is a relationship system"**
is the design.

## 5. World & topology

Habitats are the "planets." Each is a distinct biome *and* a distinct set of
physics rules — topology is gameplay, not skin.

| Habitat | Form | Gravity behaviour | Gameplay consequence |
|---|---|---|---|
| **Kepler Drum** | O'Neill cylinder | Uniform 1g at hull, 0g at axis | Tutorial world. Axis = flight/zero-g zones. Horizon curves *up*. |
| **Verge** | Stanford torus | 1g in the rim tube only | Claustrophobic ring-corridor world; everything is a loop. |
| **The Bell** | Bernal sphere | Gravity falls off toward the poles | Farming viable only at the equator; poles are low-g mining. |
| **Hollow** | Spun asteroid, irregular | Uneven, patchy | Late-game. Gravity itself is a hazard to survey. |
| **Ossuary** | Derelict, despun | Zero-g | Horror/exploration set-piece. Rama proper. |

**Signature feel:** Coriolis. Thrown, dropped and fired objects visibly curve.
Cheap to implement, impossible to mistake for another game, and it teaches the
player they're inside a spinning can better than any dialogue could.

**Camera:** near third-person with a tilt-shift post-process. Sells the world as
a diorama-in-a-can, matches the "you are a small thing inside a large made
object" theme, and conveniently lets distant geometry drop LOD aggressively.

## 6. Cast

12 romanceable characters at full scope, 6 at MVP.

**Design rule:** subculture archetypes are the *entry read*, never the
character. Each cast member is legible in one glance (the bear-ish hydroponics
chief; the twinky comms officer; the leather-clad reactor engineer) and then
each one's arc is specifically about the archetype failing them. The ship's
sociologist NPC exists partly to say this out loud. A parade of types is both
worse writing and a reputational liability; a parade of types that knows it's a
parade and dismantles itself is the actual subject matter.

Each character carries: an affinity track, a **memory ledger** (specific things
you did, referenced later), an independent opinion of *every other NPC*, and a
Crèche stance (do they want a lineage? with whom?).

## 7. Multiplayer (P4)

Dating sims and multiplayer are architecturally opposed — the genre's fantasy is
that you are the protagonist and the cast is authored for you. Ship the version
that doesn't break that:

- **v1 — Shared Habitat (async).** Visit other players' habitats. Gift economy,
  trade, co-op builds, crop/genome strain sharing. Your NPCs gossip about
  visitors. Low risk, high retention.
- **v2 — The Committee (seasonal, opt-in).** 4–8 players on one habitat compete
  for a limited number of Crèche slots and a shared NPC cast. Jealousy, alliance,
  sabotage. This is genuinely novel and thematically perfect. It is also a
  live-ops and toxicity problem — gate it behind a season structure.
- **Never — player↔player romance.** The moderation, age-verification and
  consent surface is not worth it, and it must never touch P5 content.

## 8. The Archive (P5)

Explicit content is **cut**. What remains: real, photoreal, live-action video of
men kissing, used as the emotional peak of a romance arc.

This is a completely different product than the one in the previous draft. No age
verification, no adult payment processor, no separate SKU, no off-platform
distribution, no CDN gating. Rated T or M. Consoles and mobile are back on the
table. The entire compliance apparatus goes away.

What's left is one genuine design problem and one production problem.

### 8.1 The design problem: medium shift

Cutting from a tilt-shift stylised 3D world to photoreal live-action footage is
a hard tonal seam. Done carelessly it reads as a bug. Done deliberately it is
the most memorable thing in the game.

**Make it diegetic.** The ship records everything — it has to. A colony that
reproduces by committee keeps an archive of who bonded with whom, because that
archive *is* the lineage record. So the video is not a cutscene. It is a
**memory pulled from the Archive**, and the player is watching it the way the
Crèche committee will one day watch it.

That framing buys you, for free:
- A reason for the aspect ratio to change, for grain, for compression artifacts,
  for a timecode, for the footage to degrade with age.
- A reason the camera is fixed and slightly wrong — it's a ship camera.
- A late-game gut-punch: the player can browse the Archive. Including scenes
  from lineages that came before them. Including, eventually, their own.
- An answer to "why does this look different" that is *thematic* rather than
  technical.

The medium shift stops being a compromise and becomes the point: the sim is how
the colony *works*, the Archive is what it's actually *for*.

### 8.2 The production problem: where the footage comes from

Two routes. I recommend the second.

**Route A — AI-generated.** Cheaper on paper. Two real problems: identity
consistency (the same face, every scene, across a two-year production) is still
the unsolved problem in video generation, and mainstream models' safety filters
are notoriously over-broad on same-sex intimacy — you will get refusals on
content that is, by any reasonable standard, a kiss. Fighting that is a
recurring tax with no end date.

**Route B — shoot it. With actors.** For roughly ten short scenes this is one
or two shoot days, an intimacy coordinator, a DP, and likeness releases signed
cleanly up front. And it consolidates: **the actor you cast is the actor you
photoscan for the 3D character and the actor you record VO from.** One casting
process, one contract, one person — and the 3D character and the Archive
footage are unmistakably the same man, which is exactly the thing Route A cannot
guarantee.

Budget it as a short-film production line item, not an engineering one.

### 8.3 Asset weight

~10 scenes × ~20s = about 3½ minutes of video. Even at generous bitrate that is
well under 1GB and ships in the build. This was a CDN architecture problem in
the previous draft; it is now a folder.

### 8.4 Residual risks

- **Tonal seam** — test it before committing (see `EM_BRIEF.md` spike S4).
- **Casting is a hard dependency on the art pipeline.** If actors are also the
  3D likenesses, casting must happen *before* character art, not after. This is
  the one place the Archive can hold up the schedule.
- **Some markets will not take a game with men kissing on camera**, regardless
  of rating. Plan for non-distribution in those regions rather than a censored
  variant — a cut version of this game is a different game.

## 9. Audience & positioning

Primary: adults who play cozy sims and are underserved by their romance content.
Secondary: hard-SF sim players (the habitat physics is a real draw on its own).
Tertiary: narrative/VN audience.

Rated T/M, not adult — so the addressable market is the whole cozy-sim
audience, and consoles are viable. The comp that matters commercially is
*Stardew Valley*'s long tail, not any adult title. Position as **a real sim game whose romance happens to be
uncompromising**, not as an adult title with farming attached. The former has a
ceiling of millions; the latter has a ceiling of a Patreon.

## 10. Scope options

| | **A — Slice** | **B — Target** | **C — Full pitch** |
|---|---|---|---|
| Habitats | 1 (Kepler Drum) | 2 + Ossuary set-piece | 5 |
| Cast | 6 | 9 | 12 |
| MP | none | async Shared Habitat | + The Committee |
| Archive | ~10 scenes, 1 shoot day | ~24 scenes, 2 shoots | 40+, branching variants |
| Team | 6–8 | 14–18 | 30+ |
| Time | 18–24 mo | 3 yr | 4–5 yr |

**Recommendation: build A as a shippable Early Access product**, not as a demo.
Prove the Crèche loop and the curved world with paying players before funding B.

## 11. Open questions for the next pass

- Does the player character have an authored identity or are they a blank? (The
  Crèche system argues for authored — a lineage needs a genome with a history.)
- Is death/generational succession a mechanic? Playing your own descendant is
  thematically enormous and structurally expensive.
- Real-time seasons or transit-year abstraction?
- Is the ship's destination ever reached, or is arrival the thing the game
  refuses you?
