# 1000 More: Landform Diversity on the Cylinder

**Status:** v1.0 · **Date:** 2026-09-07 · **Items 3201–4200**
**Continues:** `LANDSCAPE_200/800/1400/2000/2200/3200.md`
**Subject:** zoned landforms — massifs, cliffs, plateaus, forest kinds, swamps vs meadows, dune deserts, inland seas with islands — so the Kepler Drum stops being the same clay hills everywhere.
**Companion:** `RENDER_CONTRACT.md` · `NEXT.md` · `CALIBRATION.md` · `MODEL_LIMITS.md` · `FIELDS.md`
**Cross-refs (do not duplicate):** mountains §N 201–270 and forests §Q 411–490 in `LANDSCAPE_800.md`; biome palette §BA 2441–2540 and tree archetypes §BB 2541–2620 in `LANDSCAPE_3200.md`. This register supplies the *macro-structure* those items need underneath them.

---

## Why this register exists

The picture and the sim both know drainage, biomes, plants and weather, but every
hectare of the drum is still made by **one relief recipe**. Walk a kilometre and
the hills look like the last kilometre. There are no deserts because moisture is
drainage alone. There are no seas because nothing is carved below the waterline.
There are no forest *kinds* because temperature has no lapse rate and aridity
does not exist. Biomes classify correctly; the input fields cannot produce
difference.

This register adds one engineered mechanism — a **province (zoning) layer** —
and then the landform recipes, climate fields, seas, swamps, forest types and
dune seas that sit on top of it. Every item is commit-sized, names a mechanism,
and ends in a target file.

### Verified state at time of writing

```
relief recipe   : base*0.40 + ridge^1.3*0.74 + ribs*0.20  (terrain.rs L91–99)
max_elevation   : 235 m  (habitat.rs)
water_level     : 22 m constant-radius cylinder
moisture init   : 0.15 + 0.70*flux  (soil.rs) — no aridity
temperature     : 8 + 18*axial °C  (weather.rs) — no lapse rate
wind            : uniform +z bands — no orographic rain / rain shadow
biomes          : 9 query-time ids; fields collapse to mostly grassland
chunk radial    : lo_e-96 .. hi_e+40 fixed margins (lib.rs) — OK for 235 m
SDF / digs      : volumetric density + stroke-only persist (non-negotiable)
already queued  : mountains 201–270, forests 411–490, palette 2441–2540,
                  tree archetypes 2541–2620 — this register cross-refs them
```

### Holistic read

- **One formula everywhere.** Ridged FBM + ribs + droplets produce clay hills
  of similar amplitude around the whole drum. Zoning does not exist.
- **No aridity.** Soil moisture tracks flux, so dry scrub is rare and true
  desert is impossible. Condensers rain, but rain never leaves a shadow.
- **No seas.** Waterline cells are outlets; elevation is never intentionally
  below them, so there is no shelf, beach, island or delta landform.
- **Biomes lack kinds.** `FOREST` is one bucket. Swamp vs meadow is not a
  drainage-state distinction the classifier can make.
- **Taller peaks need budget.** Raising `max_elevation` to ~440 m (chosen:
  gravity at summits ≈ 50% of hull, embraced as traversal) forces radial-band,
  fog and camera review — not just a constant change.
- **Honesty of mechanism.** Provinces are habitat-designer zoning (fiction
  already says engineered). Relief recipes and climate fields must produce the
  look; paint and props only read them.

### Design rules this register enforces

1. Province is `f(θ, z, seed)`, never a stored painted map — same discipline as materials/biomes.
2. Worley/lattice cell count around θ is an integer (wrap like `value2_wrapped`).
3. SDF remains source of truth; cliffs/overhangs are `density()` terms.
4. Stroke-only persistence; `elev0` stays the strata parent.
5. Distances from `hab.radius` / `hab.length` / `drum_diagonal()`.
6. `max_elevation` ~440 m; document low-g summits in `CALIBRATION.md` / `MODEL_LIMITS.md`.
7. Cross-ref §N / §Q / §BA / §BB; do not re-list those items.

### Province → landform data flow

```
seed → province.rs (wrapped Worley → archetype weights)
     → terrain.rs (per-archetype relief, blended)
     → erosion (hardness-aware) → flow (seas/lakes/rivers)
     → climate (aridity, lapse, orographic rain)
     → biome.rs classify (+ new ids) → paint / grass / plants / shaders
     → density() cliffs → chunk_mesh_at radial band
```

### Implementation ledger (v1.0)

| Range | Section | Wave 1 target |
|---|---|---|
| 3201–3280 | BM Provinces | `province.rs` + `province_at` |
| 3281–3370 | BN Relief recipes | blended recipes + max_elev ~440 |
| 3371–3450 | BO Cliffs / SDF | strata cliffs + talus |
| 3451–3520 | BP Erosion | province-aware droplets |
| 3521–3600 | BQ Climate | aridity + lapse + rain shadow |
| 3601–3680 | BR Seas / islands | sea basins + shores |
| 3681–3750 | BS Swamp vs meadow | drainage-state biomes |
| 3751–3840 | BT Forest types | kind table + stand structure |
| 3841–3910 | BU Deserts / dunes | SAND + dune relief |
| 3911–3960 | BV Ecotones | blend widths + golden transitions |
| 3961–4040 | BW Meshing / LOD | radial band + skyline LOD |
| 4041–4100 | BX Gameplay | spawn, cliffs, low-g, digs |
| 4101–4160 | BY Tests / docs | determinism + census gates |
| 4161–4200 | BZ Sequence | ten waves; Wave 1 = zoned land |

---

## BM. Provinces: the zoning layer (3201–3280)

*The drum was engineered. Habitat designers zone land. One low-frequency*
*field of archetype weights is the keystone; everything else reads it.*

3201. Create `sim/src/province.rs` as the zoning module; no stored map, query-time only → `sim/src/province.rs`
3202. Define archetype enum: Massif, Plateau, Badlands, Meadow, SwampBasin, DuneSea, SeaBasin, Karst, EndcapWall → `sim/src/province.rs`
3203. Give each archetype a stable u8 id for census, HUD and FIELDS registration → `sim/src/province.rs`
3204. Add `name(id)` and `color(id)` for map legend parity with biome.rs → `sim/src/province.rs`
3205. Choose Worley/jittered lattice with integer NT_PROV cells around θ so wrap is exact → `sim/src/province.rs`
3206. Default lattice: 16 cells around × 10 along axis (~350 m × 600 m cells on Kepler Drum) → `sim/src/province.rs`
3207. Jitter cell centres with hash of (cell, seed); clamp jitter so cells stay simply connected → `sim/src/province.rs`
3208. Wrap θ distance with rem_euclid; clamp z at endcaps → `sim/src/province.rs`
3209. Assign each cell a primary archetype from a weighted table biased by axial position → `sim/src/province.rs`
3210. Bias EndcapWall to |z| near habitat ends; forbid SeaBasin within endcap belt → `sim/src/province.rs`
3211. Bias DuneSea and Badlands toward mid-axial dry intent bands → `sim/src/province.rs`
3212. Bias SwampBasin and SeaBasin toward mid-habitat where condensers already cluster → `sim/src/province.rs`
3213. Bias Massif toward structural rib phases so engineered ribs seed mountain spines → `sim/src/province.rs`
3214. Guarantee at least one SeaBasin, one DuneSea, one Massif, one SwampBasin, one Meadow per seed → `sim/src/province.rs`
3215. Store per-cell climate intent: arid, mesic, humid, marine as a second tag → `sim/src/province.rs`
3216. Derive climate intent defaults from archetype; allow rare overrides via hash → `sim/src/province.rs`
3217. Expose `province_at(hab, theta, z) -> ProvinceSample` with top-two weights → `sim/src/province.rs`
3218. Blend weights by inverse-distance to nearest two cell centres; blend width 150–400 m tunable → `sim/src/province.rs`
3219. Reject all-neighbour average; top-two only (same greying rule as paint.rs) → `sim/src/province.rs`
3220. Add `province_weights_at` returning fixed-size array indexed by archetype id → `sim/src/province.rs`
3221. Add `dominant_at` for HUD and spawn preference → `sim/src/province.rs`
3222. Register province as derived field in FIELDS.md (not persisted) → `docs/FIELDS.md`
3223. Document province cell size and blend width provenance in CALIBRATION.md → `docs/CALIBRATION.md`
3224. State MODEL_LIMITS: provinces are designer zoning, not plate tectonics → `docs/MODEL_LIMITS.md`
3225. Wire `mod province` and re-exports from lib.rs / biosphere as needed → `sim/src/lib.rs`
3226. Add `province_census(samples)` returning counts per archetype → `sim/src/province.rs`
3227. Expose province census through RamaTerrain FFI dictionary → `sim/src/lib.rs`
3228. Draw province overlay mode on habitat map HUD (toggle next to biome) → `game/scripts/overlay.gd`
3229. Match overlay colours to province::color → `game/scripts/overlay.gd`
3230. Print province under reticle in debug mode only → `game/scripts/world.gd`
3231. Add unit test: province_at wraps θ continuously (seam Δ weight < epsilon) → `sim/src/province.rs`
3232. Add unit test: same seed → identical dominant map hash → `sim/src/province.rs`
3233. Add unit test: guaranteed archetype set present for kepler_drum seed → `sim/src/province.rs`
3234. Add selftest line printing province census after generate → `game/scripts/world.gd`
3235. Forbid painting province into elev or soil grids; keep query-time → `docs/SIM_ARCH_BRIEF.md`
3236. Add blend_width_m to Habitat or ProvinceParams; default 280 m → `sim/src/province.rs`
3237. Scale blend_width with hab.radius so other habitats keep proportion → `sim/src/province.rs`
3238. Add soft EndcapWall fade: within 8% of length from each end, weight EndcapWall upward → `sim/src/province.rs`
3239. Prevent Massif and SeaBasin from sharing a cell; pick by hash if both candidate → `sim/src/province.rs`
3240. Allow Karst only where humid intent and mid elevation bias → `sim/src/province.rs`
3241. Publish province archetype table as const data, not scattered matches → `sim/src/province.rs`
3242. Each archetype row: default relief amp, slope class, moisture bias, plant density bias → `sim/src/province.rs`
3243. Relief amp for Massif targets 0.85–1.0 of max_elevation → `sim/src/province.rs`
3244. Relief amp for Meadow targets ≤0.25 of max_elevation → `sim/src/province.rs`
3245. Relief amp for SeaBasin targets floor below water_level → `sim/src/province.rs`
3246. Relief amp for DuneSea targets 15–45 m dune height, not mountain amp → `sim/src/province.rs`
3247. Plateau targets mid amp with terrace quantisation flag → `sim/src/province.rs`
3248. Badlands targets mid amp with high-frequency rill flag → `sim/src/province.rs`
3249. SwampBasin targets near-waterline flats with hummock flag → `sim/src/province.rs`
3250. Karst targets mid amp with doline depression flag → `sim/src/province.rs`
3251. EndcapWall targets high amp + steep toward end seal → `sim/src/province.rs`
3252. Add debug PNG or PackedByteArray export of dominant province for tools → `sim/src/lib.rs`
3253. Document that async MP later merges strokes only; provinces stay seed-derived → `docs/REQUIREMENTS.md`
3254. Add province to biome_at path as soft prior, not hard override → `sim/src/biosphere.rs`
3255. Keep classify() pure: pass province moisture/temp bias in, do not import province.rs into biome hot if avoidable → `sim/src/biome.rs`
3256. Add NEXT note when province lands: displace only render queue items, keep register → `docs/NEXT.md`
3257. Measure province_at cost; target < 50 ns/sample amortized → `sim/src/bin/bench.rs`
3258. Cache nothing global; pure function of θ,z,seed,params → `sim/src/province.rs`
3259. Use same hash family as noise.rs for cross-platform determinism → `sim/src/noise.rs`
3260. Add ProvinceParams struct: nt, nz, blend_m, seed_xor, weights table → `sim/src/province.rs`
3261. Default weights table sums to 1.0 after axial remapping → `sim/src/province.rs`
3262. Log warning if census missing a guaranteed archetype (debug builds) → `sim/src/province.rs`
3263. Expose province params in RamaTerrain::params() dictionary → `sim/src/lib.rs`
3264. Add plan-view soil overlay sibling for province (V-cycle or separate key) → `game/scripts/minimap_overlay.gd`
3265. Do not let province overlay fight soil mode; exclusive modes → `game/scripts/world.gd`
3266. Write inspiration note: O'Neill zoning belts as fiction source → `docs/inspiration/README.md`
3267. Cross-link LANDSCAPE_800 §N: province Massif is the macro input those microforms need → `docs/LANDSCAPE_800.md`
3268. Cross-link LANDSCAPE_3200 §BA: palette reads biome which will read climate from province → `docs/LANDSCAPE_3200.md`
3269. Add photo metadata province name under reticle → `game/scripts/world.gd`
3270. Ship province without changing elev yet; land as API + overlay first if needed → `docs/NEXT.md`
3271. Add integration smoke: generate() calls province_census once → `sim/src/terrain.rs`
3272. Forbid float cell counts; NT_PROV and NZ_PROV are usize consts → `sim/src/province.rs`
3273. Document anti-pattern: painting Voronoi into a texture and sampling GPU-side → `docs/SIM_ARCH_BRIEF.md`
3274. Ensure EndcapWall cannot be blended away at z=±length/2 → `sim/src/province.rs`
3275. Add soft adjacency preference: SeaBasin prefers neighbour Meadow or SwampBasin → `sim/src/province.rs`
3276. Add soft adjacency preference: DuneSea prefers neighbour Badlands or Plateau → `sim/src/province.rs`
3277. Adjacency preferences are reassignment passes, not runtime costs → `sim/src/province.rs`
3278. Done means a player can name three provinces from the map overlay alone → `docs/NEXT.md`
3279. Done means province_at is deterministic on macOS/Linux CI → `sim/src/province.rs`
3280. Done means FIELDS.md lists province as derived with archetype ids → `docs/FIELDS.md`

---

## BN. Relief recipes per archetype (3281–3370)

*One recipe function per archetype, blended by province weights. The drum*
*stops being one FBM call with ribs.*

3281. Split Terrain::generate engineered relief into per-archetype recipe functions → `sim/src/terrain.rs`
3282. Recipe signature: (tf, zf, hab, seed) -> elevation metres before erosion → `sim/src/terrain.rs`
3283. Blend recipes by province top-two weights at each elev cell → `sim/src/terrain.rs`
3284. Keep structural ribs as a shared additive term, weighted higher in Massif → `sim/src/terrain.rs`
3285. Massif recipe: ridged2 dominant, powf ≥ 1.45, amp ≈ max_elevation → `sim/src/terrain.rs`
3286. Massif recipe: add second ridged octave bank for subsidiary ridges → `sim/src/terrain.rs`
3287. Massif recipe: suppress base FBM so valleys are carved by erosion not noise mush → `sim/src/terrain.rs`
3288. Plateau recipe: quantise elevation into 3–5 terrace steps of 12–25 m → `sim/src/terrain.rs`
3289. Plateau recipe: soft edge noise so terraces are not perfect cylinders → `sim/src/terrain.rs`
3290. Plateau recipe: canyon slots cut by low-frequency trenches toward waterline → `sim/src/terrain.rs`
3291. Meadow recipe: fbm2 only, 3 octaves, amp ≤ 60 m absolute → `sim/src/terrain.rs`
3292. Meadow recipe: gentle corrugation; no ridged2 → `sim/src/terrain.rs`
3293. SwampBasin recipe: target elev = water_level + 2..6 m + hummock noise ≤ 1.5 m → `sim/src/terrain.rs`
3294. SwampBasin recipe: broad flat floors; forbid steep local gradients at gen → `sim/src/terrain.rs`
3295. DuneSea recipe: anisotropic ridges aligned to wind_z (+z), wavelength 40–120 m → `sim/src/terrain.rs`
3296. DuneSea recipe: asymmetric slip face (steep lee, gentle stoss) → `sim/src/terrain.rs`
3297. DuneSea recipe: amp 15–45 m; substrate pedestal near water_level + 8 m → `sim/src/terrain.rs`
3298. SeaBasin recipe: floor elev in 9–15 m (below water_level 22, above bedrock 8.5) → `sim/src/terrain.rs`
3299. SeaBasin recipe: residual ridged knobs become islands (elev 25–80 m) → `sim/src/terrain.rs`
3300. SeaBasin recipe: shelf slope 1–3° from shore to floor over 80–200 m → `sim/src/terrain.rs`
3301. Badlands recipe: high-frequency ridged + channelled gullies → `sim/src/terrain.rs`
3302. Badlands recipe: amp mid (80–160 m) with dense drainage seed → `sim/src/terrain.rs`
3303. Karst recipe: base plateau + circular doline depressions from Worley pits → `sim/src/terrain.rs`
3304. Karst recipe: some dolines punch near water_level for disappearing streams → `sim/src/terrain.rs`
3305. EndcapWall recipe: ramp elev toward max_elevation * 1.2 near seals → `sim/src/terrain.rs`
3306. EndcapWall recipe: keep existing cap boost but province-localise it → `sim/src/terrain.rs`
3307. Raise Habitat::max_elevation from 235 to ~440 for Kepler Drum → `sim/src/habitat.rs`
3308. Record gravity at summit: g_hull * (R - e)/R ≈ 0.5 at 440 m; document as feature → `docs/CALIBRATION.md`
3309. Update MODEL_LIMITS with chosen peak height and low-g note → `docs/MODEL_LIMITS.md`
3310. Clamp elev after recipes to [8.5, max_elevation] → `sim/src/terrain.rs`
3311. Preserve elev0 clone after recipes + erosion as today → `sim/src/terrain.rs`
3312. Do not change droplet count until BP; keep 520k as baseline first land → `sim/src/terrain.rs`
3313. Add hypsometric histogram helper over elev grid → `sim/src/terrain.rs`
3314. Target hypsometry: ≥8% cells below water_level in SeaBasin-rich seeds → `sim/src/terrain.rs`
3315. Target hypsometry: ≥5% cells above 0.7 * max_elevation → `sim/src/terrain.rs`
3316. Target hypsometry: meadow provinces mean elev < 0.3 * max_elevation → `sim/src/terrain.rs`
3317. Expose hypsometric buckets in census dictionary → `sim/src/lib.rs`
3318. Generation budget: province blend + recipes ≤ +30% of current generate CPU → `sim/src/bin/bench.rs`
3319. Keep tunnels generation after elev; do not daylight tunnels on dune/sea floors → `sim/src/terrain.rs`
3320. Bias tunnel depth deeper under Massif; shallower under Meadow → `sim/src/terrain.rs`
3321. SeaBasin islands: require local prominence > 12 m to keep as island seed → `sim/src/terrain.rs`
3322. Limit island count per SeaBasin cell via hash thinning → `sim/src/terrain.rs`
3323. Shore band: where elev crosses water_level, force soft 4–8 m transition → `sim/src/terrain.rs`
3324. Avoid vertical walls at waterline from recipe discontinuities → `sim/src/terrain.rs`
3325. Domain-warp Massif recipe mildly (2–8 m) using fbm2; not on Meadow → `sim/src/terrain.rs`
3326. Plateau terrace heights derive from material band thicknesses (link material.rs) → `sim/src/material.rs`
3327. Badlands: multiply high-freq by (1 - soft_cap) so endcaps stay monumental → `sim/src/terrain.rs`
3328. DuneSea: rotate anisotropy if wind_theta ever becomes strong; default +z → `sim/src/terrain.rs`
3329. SwampBasin: punch occasional deeper pools to water_level - 1 for sph seed → `sim/src/terrain.rs`
3330. Karst: connect some dolines with shallow slots (polje hints) → `sim/src/terrain.rs`
3331. Massif: sharpen ridges with arête bias where two valleys approach → `sim/src/terrain.rs`
3332. Cols: lower saddles between massif peaks by 10–25% of local amp → `sim/src/terrain.rs`
3333. Keep engineered spoil-heap hooks as optional additive in Meadow/Farm-adjacent → `sim/src/terrain.rs`
3334. Alloy rib outcrops: where ribs > 0.85, lift elev slightly and mark for paint → `sim/src/terrain.rs`
3335. Do not bake biome into elev; elev remains physical → `sim/src/terrain.rs`
3336. Add before/after elev hash log in selftest for recipe landings → `game/scripts/world.gd`
3337. Visual gate: far field must show distinct dark sea masses and pale dune belts → `docs/NEXT.md`
3338. Visual gate: skyline must show peaked massifs not uniform rumple → `docs/NEXT.md`
3339. Keep noise seeds XOR'd per recipe so layers do not phase-lock → `sim/src/terrain.rs`
3340. Document recipe coefficients with invented tags until screenshot-calibrated → `docs/CALIBRATION.md`
3341. Meadow: slight tilt toward nearest SeaBasin/Swamp for gentle drainage → `sim/src/terrain.rs`
3342. SeaBasin floor: very low freq undulation ≤ 2 m to avoid flat pool look under water → `sim/src/terrain.rs`
3343. Island knobs use ridged2 with high powf for steep little peaks → `sim/src/terrain.rs`
3344. Plateau canyons prefer θ-aligned or z-aligned by hash for variety → `sim/src/terrain.rs`
3345. EndcapWall blends into Massif when both present; never into SeaBasin → `sim/src/terrain.rs`
3346. Apply recipes in elev.chunks_mut parallel-friendly loop as today → `sim/src/terrain.rs`
3347. Avoid per-cell province_at if possible: evaluate on coarse grid then bilinear weights → `sim/src/terrain.rs`
3348. Coarse weight grid: NT/4 × NZ/4 then upsample — measure seam quality → `sim/src/terrain.rs`
3349. If coarse upsample seams, fall back to full-res province_at for Wave 1 → `sim/src/terrain.rs`
3350. Update REQUIREMENTS A1 note: engineered zoning is an allowed relief source → `docs/REQUIREMENTS.md`
3351. Far mesh must sample same elev; no separate far recipe → `sim/src/lib.rs`
3352. Mid mesh same elev source → `sim/src/lib.rs`
3353. Dig elev clamp max stays max_elevation after raise → `sim/src/terrain.rs`
3354. find_spawn scoring: prefer Meadow elev band 20–80 m → `sim/src/lib.rs`
3355. Reject spawn in SeaBasin floor or dune slip faces → `sim/src/lib.rs`
3356. Add elev variance metric per province for selftest → `sim/src/lib.rs`
3357. Massif variance high; Meadow variance low — assert ranges → `sim/src/terrain.rs`
3358. Keep artifact tunnels out of SeaBasin floors (density already depth-gated) → `sim/src/terrain.rs`
3359. DuneSea pedestal must sit above water_level so dunes are not flooded by default → `sim/src/terrain.rs`
3360. Occasional flooded sabkha: allow 5% of DuneSea cells to dip to water_level + 1 → `sim/src/terrain.rs`
3361. SwampBasin mean elev within 4 m of water_level ± hummocks → `sim/src/terrain.rs`
3362. Badlands: ensure enough relief for rills after erosion pass → `sim/src/terrain.rs`
3363. Publish recipe pseudocode in SIM_ARCH_BRIEF § terrain-from-process → `docs/SIM_ARCH_BRIEF.md`
3364. Golden still: massif skyline from colonist view → `shots/`
3365. Golden still: sea with two islands from drum view → `shots/`
3366. Golden still: dune sea looking +z → `shots/`
3367. Golden still: meadow into swamp ecotone → `shots/`
3368. Done means generate() uses blended recipes, not single FBM → `sim/src/terrain.rs`
3369. Done means max_elevation ~440 and radial band still covers relief → `sim/src/lib.rs`
3370. Done means hypsometry targets pass on default seed → `sim/src/terrain.rs`

---

## BO. Mountains and cliffs in the SDF (3371–3450)

*Cross-ref LANDSCAPE_800 §N (201–270) for microforms. This section wires*
*strata hardness into density(), talus, and low-g summit play so Massif*
*provinces read as mountains rather than tall clay.*

3371. Drive cliff terraces in density() from material hardness ladder, not slope alone → `sim/src/terrain.rs`
3372. Where sandstone-over-clay soft undercut: push density airward under hard cap within 2 m → `sim/src/terrain.rs`
3373. Bound undercut so ground_below still finds a surface within 40 m start band → `sim/src/terrain.rs`
3374. Add joint-set planar bias: prefer faces near two hash-chosen strike angles per Massif cell → `sim/src/terrain.rs`
3375. Joint bias is a density warp ≤ 1.5 m, not a second mesh → `sim/src/terrain.rs`
3376. Expose cliffiness scalar: slope * hardness_factor for paint and grass kill → `sim/src/paint.rs`
3377. Talus: call talus_relax with angle-of-repose per surface material after gen erosion → `sim/src/erosion.rs`
3378. Repose: regolith 35°, scree/basalt 40°, sand 32°, clay 28° (document invented tags) → `docs/CALIBRATION.md`
3379. Talus deposits thicken elev at cliff feet; keep sediment budget local → `sim/src/erosion.rs`
3380. Scree stripes: pale paint where talus_relax recently moved mass → `sim/src/paint.rs`
3381. Arêtes: sharpen ridge lines where both flanking slopes > threshold in Massif → `sim/src/terrain.rs`
3382. Cols already lowered in BN; ensure density does not fill saddles → `sim/src/terrain.rs`
3383. Prominence: compute peak prominence on elev grid for naming hooks → `sim/src/terrain.rs`
3384. Name peaks with prominence > 40 m; store list in generate side channel for HUD → `sim/src/lib.rs`
3385. Peak names are presentation only; not gameplay locks → `docs/MODEL_LIMITS.md`
3386. Alloy rib outcrops: density hard shell near ribs at surface (ties material ALLOY) → `sim/src/material.rs`
3387. Cliff bands with slope > 0.75 set movement block flag for player walk → `game/scripts/player.gd`
3388. Allow scramble if facing and speed low; else slide/fall → `game/scripts/player.gd`
3389. Scree slide: reduce control and add sound when on talus material → `game/scripts/player.gd`
3390. Low-g on summits: use existing omega²r; ensure jump impulse feels lighter as r drops → `game/scripts/player.gd`
3391. UI instrument: show local g when elev > 200 m (debug or survey view) → `game/scripts/world.gd`
3392. Overhang term in density max horizontal intrusion 3 m; document limit → `docs/MODEL_LIMITS.md`
3393. Raycast dig still works on cliff faces; hardness gates dig_scale → `sim/src/terrain.rs`
3394. Fresh dig on cliff: brighter albedo (weathering age 0) via paint rule → `sim/src/paint.rs`
3395. Cross-ref item 245 rock colour by weathering age — implement minimal version here → `sim/src/paint.rs`
3396. Massif skyline LOD: mid/far must not flatten peaks below 70% elev → `sim/src/lib.rs`
3397. Far field sample denser on high elev cells (silhouette-aware subsample) → `sim/src/lib.rs`
3398. Columnar jointing visual: only as shader mottling on basalt cliffs, not geo → `game/shaders/terrain.gdshader`
3399. Do not implement full §N arch/karst cave graph here; Karst province uses dolines + existing caves → `sim/src/terrain.rs`
3400. Spring hook: where permeable-over-impermeable and flux, boost moisture (BQ will rain) → `sim/src/soil.rs`
3401. Hanging valley hint: tributary elev floor above main when Massif + high flux junction → `sim/src/terrain.rs`
3402. Knickpoint: after erosion, lock soft→hard transitions as steps (BP details) → `sim/src/erosion.rs`
3403. Boulder field Multimesh below cliffs using grass_field-like scatter → `game/scripts/world.gd`
3404. Boulder density from cliffiness * (1 - vegetation) → `sim/src/lib.rs`
3405. Erratics: sparse large rocks in Meadow downslope of Massif → `game/scripts/world.gd`
3406. Avalanche chute: pale stripe paint + low plant density on steep Massif faces → `sim/src/plant.rs`
3407. Rockfall event stub: rare tick spawns dust + elev talus bump (optional Wave 2+) → `sim/src/erosion.rs`
3408. Cliff shadow: rely on sun contract from LANDSCAPE_3200; no second system → `game/shaders/rama_light.gdshaderinc`
3409. Contact darkening under overhangs as AO in flat_shade or shader → `sim/src/mesher.rs`
3410. Ensure chunk radial band includes overhangs: expand hi margin when cliffiness high → `sim/src/lib.rs`
3411. Strata bands visible on cliff faces via existing color_at band code — boost contrast → `sim/src/lib.rs`
3412. Cap rock mesas in Plateau: hard layer elev quantisation already in BN → `sim/src/terrain.rs`
3413. Inselberg: residual Massif knob inside Meadow blend zone → `sim/src/terrain.rs`
3414. Pediment: gentle bedrock ramp elev recipe between Massif and Meadow → `sim/src/terrain.rs`
3415. Alluvial fan elev additive at Massif→flat junctions after erosion (BP) → `sim/src/erosion.rs`
3416. Player camera far still drum_diagonal(); peaks must not clip near plane oddly → `game/scripts/player.gd`
3417. Fog: do not hide massif bases; retune after elev raise → `game/scripts/world.gd`
3418. Survey/drum view: peaks readable as silhouettes against opposite wall → `game/scripts/world.gd`
3419. Climb affordance: narrow ledges as density shelves every 8–12 m on hard bands → `sim/src/terrain.rs`
3420. Ledge depth ≤ 1.2 m; enough to stand → `sim/src/terrain.rs`
3421. Block dwellings on slope > 0.25 still; cliffs are not buildable → `sim/src/dwelling.rs`
3422. Geologist readout: strike/dip approximate from joint set + strata → `sim/src/lib.rs`
3423. Ore float: ferrous fragments downslope of ferrous outcrop (economy hook later) → `sim/src/economy.rs`
3424. Publish limit: no true rock mechanics, no fracture propagation sim → `docs/MODEL_LIMITS.md`
3425. Massif weather: colder via lapse (BQ); wetter windward (BQ) → `sim/src/weather.rs`
3426. Snow line by temperature not elev — when frost palette enabled (3200 placeholder) → `docs/MODEL_LIMITS.md`
3427. Cloud cap stub: humidity high + peak elev → cloud opacity boost at summit → `sim/src/weather.rs`
3428. Katabatic: night wind downslope bias weak; document as future → `docs/NEXT.md`
3429. Mountains as water towers: high elev intercept rain (BQ orographic) → `sim/src/weather.rs`
3430. Cross-ref 269: this section + BQ satisfy water-tower role → `docs/LANDSCAPE_800.md`
3431. Cliff grass kill: grass_field rejects cliffiness > 0.6 → `sim/src/lib.rs`
3432. Tree kill on cliffs: plant seed rejects slope > 0.55 except krummholz → `sim/src/plant.rs`
3433. Krummholz allowed on Massif high elev with alpine biome → `sim/src/plant.rs`
3434. Far wall massifs: province colour mass + elev shading, not near meshes → `sim/src/paint.rs`
3435. Bisect group for cliffs-only debug hide → `game/scripts/debug/bisect.gd`
3436. Selftest: max elev after gen within 5% of max_elevation → `game/scripts/world.gd`
3437. Selftest: at least N cells with slope > 0.7 in Massif census → `sim/src/lib.rs`
3438. Bench density() with cliff terms; keep sample budget honest → `sim/src/bin/bench.rs`
3439. Do not store cliff SDF separately; derive in density() → `sim/src/terrain.rs`
3440. Document joint set seeds in CALIBRATION → `docs/CALIBRATION.md`
3441. Spoil heaps remain distinct from natural talus (angle + bedding paint) → `sim/src/paint.rs`
3442. Quarry face stub: rectilinear cut near towns if dwell exists → `game/scripts/world.gd`
3443. Handholds: only if climb mode added; else skip — register as optional → `docs/NEXT.md`
3444. Frost shatter debris texture near endcaps (angular) vs chemical (rounded) as prop tint → `game/scripts/world.gd`
3445. Case hardening: dig_scale slightly harder at surface than 1 m in on sandstone → `sim/src/material.rs`
3446. Mineral staining below seeps: paint iron/manganese rules when moisture high on rock → `sim/src/paint.rs`
3447. Salt crust where arid + pond (BU) → `sim/src/paint.rs`
3448. Done means Massif skylines read as cliffs and peaks in colonist view → `docs/NEXT.md`
3449. Done means player can be blocked by a cliff band and find a col → `game/scripts/player.gd`
3450. Done means density cliffs + talus land without breaking dig/flow → `sim/src/terrain.rs`

---

## BP. Erosion aware of provinces (3451–3520)

*Droplets and thermal erosion should spend effort where the landform needs it,*
*and respect hardness — not smear every province into the same valleys.*

3451. Weight droplet spawn density by province: high in Massif/Badlands, low in DuneSea/SeaBasin → `sim/src/terrain.rs`
3452. DuneSea: replace hydraulic droplets with wind-transport pass along +z → `sim/src/erosion.rs`
3453. SeaBasin floor: minimal erosion; preserve shelf and islands → `sim/src/terrain.rs`
3454. SwampBasin: low erosive capacity; preserve flats → `sim/src/terrain.rs`
3455. Hardness map from surface_hardness / material, not flux alone → `sim/src/erosion.rs`
3456. Soft rock erodes faster; hard bands form knickpoints → `sim/src/erosion.rs`
3457. Thermal/talus pass after hydraulic; province-specific repose → `sim/src/erosion.rs`
3458. Alluvial fans: deposit when slope drops Massif→Meadow below threshold → `sim/src/erosion.rs`
3459. Fan shape: radial elev wedge, paint as sediment → `sim/src/erosion.rs`
3460. Badlands rilling: increase droplet steps and reduce inertia in Badlands → `sim/src/terrain.rs`
3461. Keep total droplet budget fixed (e.g. 520k) but redistribute by province area → `sim/src/terrain.rs`
3462. Live erosion.rs rainfall-weighted: multiply by climate rain when BQ lands → `sim/src/erosion.rs`
3463. Do not erode below 8.5 m bedrock clamp → `sim/src/erosion.rs`
3464. Do not fill SeaBasin islands away; hardness high on island knobs → `sim/src/erosion.rs`
3465. Meadow: light smoothing only; keep farmable variance → `sim/src/erosion.rs`
3466. Karst: preferential deepening of doline centres → `sim/src/erosion.rs`
3467. EndcapWall: reduce lateral erosion; keep monumental ramp → `sim/src/terrain.rs`
3468. Determinism: same seed → same elev hash after erosion on CI → `sim/src/terrain.rs`
3469. Avoid platform float drift: use same hash RNG as today → `sim/src/noise.rs`
3470. Expose erosion_stats: mass moved, mean slope change → `sim/src/lib.rs`
3471. Selftest prints erosion_stats → `game/scripts/world.gd`
3472. Knickpoint detect: mark cells where hard band stalls incision → `sim/src/flow.rs`
3473. Waterfalls at hanging valley junctions: channel mesh drop → `game/scripts/world.gd`
3474. Gorge: deepen high-flux soft corridors through Plateau → `sim/src/erosion.rs`
3475. Slot canyon: narrow deep cut when hard walls + soft floor → `sim/src/erosion.rs`
3476. Sediment delivery to SeaBasin builds tiny deltas (BR) → `sim/src/erosion.rs`
3477. Dune migration stub: slow elev shift along wind over long ticks (optional) → `sim/src/erosion.rs`
3478. Publish limit: toy fluvial, no isostasy, no full sediment budget → `docs/MODEL_LIMITS.md`
3479. CPU budget: gen erosion still within generate() acceptable time → `sim/src/bin/bench.rs`
3480. Parallelise droplet batches with thread::scope if threaded flag on → `sim/src/terrain.rs`
3481. Thermal pass O(NT*NZ) once; do not nest inside droplets → `sim/src/erosion.rs`
3482. Angle-of-repose from CALIBRATION constants → `docs/CALIBRATION.md`
3483. Spoil heaps excluded from natural talus relax or tagged → `sim/src/erosion.rs`
3484. Live dig still marks flow dirty; erosion tick uses rainfall from weather → `sim/src/biosphere.rs`
3485. Province weights cached coarse during gen erosion for speed → `sim/src/terrain.rs`
3486. Assert Massif mean slope > Meadow mean slope after gen → `sim/src/terrain.rs`
3487. Assert Badlands drainage density > Meadow → `sim/src/flow.rs`
3488. Fans must not dam SeaBasin outlets incorrectly → `sim/src/flow.rs`
3489. Channel width still ∝ √Q after new relief → `sim/src/flow.rs`
3490. Update droplet constants in CALIBRATION with provenance → `docs/CALIBRATION.md`
3491. Cross-ref §N differential weathering — hardness path is the implementation → `docs/LANDSCAPE_800.md`
3492. Debris flow path: pale chute if rare high discharge (optional visual) → `sim/src/paint.rs`
3493. Patterned ground near endcaps: low-amp polygons in elev or paint only → `sim/src/paint.rs`
3494. Do not run 520k droplets on SeaBasin cells; skip spawn there → `sim/src/terrain.rs`
3495. Wind transport moves sand from stoss to lee in DuneSea only → `sim/src/erosion.rs`
3496. Sand conservation approximate within DuneSea cell → `sim/src/erosion.rs`
3497. Peat in SwampBasin: resist erosion (high cohesion) → `sim/src/material.rs`
3498. Clay in swamp/meadow flats: cohesion from material palette → `sim/src/material.rs`
3499. After erosion, rebuild flow full once → `sim/src/terrain.rs`
3500. Local rebuild path unchanged for digs → `sim/src/flow.rs`
3501. Document redistribution weights table → `docs/CALIBRATION.md`
3502. Golden still: badlands rills close-up → `shots/`
3503. Golden still: fan at mountain front → `shots/`
3504. Bench before/after province-aware erosion → `sim/src/bin/bench.rs`
3505. If erosion time exceeds budget, cut Badlands multiplier first → `docs/NEXT.md`
3506. Keep inertia/gravity constants unless CALIBRATION says otherwise → `sim/src/terrain.rs`
3507. Evaporate factor may vary slightly by aridity when BQ exists → `sim/src/terrain.rs`
3508. Massif headwaters: higher water spawn amount → `sim/src/terrain.rs`
3509. Rain shadow dry side: fewer droplets post-BQ → `sim/src/terrain.rs`
3510. Integrate with live erosion tick without double-applying gen carve → `sim/src/erosion.rs`
3511. Gen erosion writes elev; live erosion continues from there → `sim/src/erosion.rs`
3512. Test: dig trench still reroutes (regression from waterline outlets) → `game/scripts/world.gd`
3513. Test: SeaBasin still has lake/sea mask cells → `sim/src/flow.rs`
3514. Avoid filling entire drum as lake (already fixed); keep MAX_POND → `sim/src/flow.rs`
3515. Swamp ponding uses MAX_POND honestly with finite depth → `sim/src/flow.rs`
3516. Done means province-weighted droplets change landform character → `sim/src/terrain.rs`
3517. Done means DuneSea not hydraulically rilled into badlands → `sim/src/erosion.rs`
3518. Done means fans appear at range fronts on default seed → `sim/src/erosion.rs`
3519. Done means elev hash stable on CI → `sim/src/terrain.rs`
3520. Done means generate time within agreed budget → `sim/src/bin/bench.rs`

---

## BQ. Climate as real fields (3521–3600)

*Without aridity, lapse rate and orographic rain, deserts and forest kinds*
*cannot exist. Extend weather — do not invent a second parallel climate.*

3521. Add aridity field at weather or climate resolution (WT×WZ or ST×SZ) → `sim/src/weather.rs`
3522. Aridity base from province climate intent (arid/mesic/humid/marine) → `sim/src/weather.rs`
3523. Increase aridity with distance from nearest condenser → `sim/src/weather.rs`
3524. Decrease aridity near SeaBasin and permanent water → `sim/src/weather.rs`
3525. Expose aridity_at(theta,z) → `sim/src/weather.rs`
3526. Add elevation lapse rate: temp_c -= elev * 0.0065 °C/m (document calibration) → `sim/src/weather.rs`
3527. temp_at samples axial gradient then applies lapse from elevation() → `sim/src/weather.rs`
3528. Document lapse as Earth-like fiction inside engineered habitat → `docs/CALIBRATION.md`
3529. Orographic rain: integrate elev gradient along wind_z upstream → `sim/src/weather.rs`
3530. Windward flank of Massif gets rain multiplier; leeward rain shadow → `sim/src/weather.rs`
3531. Rain shadow strength scales with elev barrier height → `sim/src/weather.rs`
3532. Combine condenser rain with orographic term into rainfall grid → `sim/src/weather.rs`
3533. Humidity base per province intent before condenser tick → `sim/src/weather.rs`
3534. Soil moisture init: mix rain climatology × flux, not flux alone → `sim/src/soil.rs`
3535. soil.moisture = clamp(a*rain_clim + b*flux + c*(1-aridity)) → `sim/src/soil.rs`
3536. Retune coefficients so Meadow stays mesic and DuneSea stays dry → `docs/CALIBRATION.md`
3537. Live soil tick: evaporation higher where aridity high → `sim/src/soil.rs`
3538. Live soil tick: rain from weather.rainfall as today plus orographic → `sim/src/soil.rs`
3539. biome::classify gains desert path: aridity high + moisture low → DESERT → `sim/src/biome.rs`
3540. Pass aridity into classify or pre-bake moisture so desert emerges → `sim/src/biome.rs`
3541. Alpine uses lapse-cooled high elev more reliably → `sim/src/biome.rs`
3542. Forest moisture thresholds distinguish swamp forest vs dry woodland via fields → `sim/src/biome.rs`
3543. Expose climate debug overlay: aridity / rain / temp modes → `game/scripts/world.gd`
3544. HUD instruments can show local rain and aridity in survey → `game/scripts/world.gd`
3545. Cloud opacity boost on windward massifs → `game/shaders/clouds.gdshader`
3546. Keep Coriolis rain drift; apply after orographic deposit → `sim/src/weather.rs`
3547. Banded wind still primary; orographic uses wind_z direction → `sim/src/weather.rs`
3548. Aspect: slopes facing axis strip vs hull — optional moisture bias small → `sim/src/weather.rs`
3549. Frost hollow stub: valley night cold — document only if not in Wave 1 → `docs/MODEL_LIMITS.md`
3550. Register climate fields in FIELDS.md → `docs/FIELDS.md`
3551. MODEL_LIMITS: not a GCM; banded + orographic toy → `docs/MODEL_LIMITS.md`
3552. Unit test: leeward of synthetic ridge drier than windward → `sim/src/weather.rs`
3553. Unit test: summit colder than same θ at waterline by lapse*elev → `sim/src/weather.rs`
3554. Unit test: DuneSea province mean aridity > Meadow → `sim/src/weather.rs`
3555. Selftest prints climate census: mean rain, aridity by province → `game/scripts/world.gd`
3556. biosphere::biome_at uses updated temp_at with lapse → `sim/src/biosphere.rs`
3557. Fallback biome path in lib.rs without biosphere also uses lapse if elev available → `sim/src/lib.rs`
3558. Condenser placement AI later can prefer arid cells — out of scope note → `docs/NEXT.md`
3559. Rain curtains (3200 item) read rainfall intensity — ensure field non-zero in wet bands → `game/scripts/world.gd`
3560. Do not store climate in save; derive from seed + condensers + elev → `docs/FIELDS.md`
3561. Marine intent near SeaBasin: lower aridity, higher humidity → `sim/src/weather.rs`
3562. Humid SwampBasin: high humidity floor even without condenser → `sim/src/weather.rs`
3563. Arid DuneSea: humidity ceiling → `sim/src/weather.rs`
3564. Massif humid windward can support forest on slopes → `sim/src/biome.rs`
3565. Plateau rain shadow tablelands can be scrub → `sim/src/biome.rs`
3566. Endcap cold + elev → alpine/tundra-like via existing alpine id → `sim/src/biome.rs`
3567. Climate tick order: update humidity/temp/rain before soil tick → `sim/src/biosphere.rs`
3568. Performance: orographic integrate on WT×WZ not NT×NZ → `sim/src/weather.rs`
3569. Upsample rain to ST×SZ as condensers already do → `sim/src/weather.rs`
3570. Document orographic kernel size in CALIBRATION → `docs/CALIBRATION.md`
3571. Optional climate.rs module if weather.rs grows too large — prefer extend first → `sim/src/weather.rs`
3572. If split: weather owns condensers/day; climate owns aridity/orographic → `sim/src/weather.rs`
3573. Wind_theta weak Ekman can slightly skew rain drift already present → `sim/src/weather.rs`
3574. Drought stress visual hooks read aridity (3200 items) — fields must exist → `sim/src/plant.rs`
3575. Evaporite / salt flats need arid + pond — BQ supplies arid → `sim/src/paint.rs`
3576. Oasis: high flux cell inside arid province → local moisture spike → `sim/src/soil.rs`
3577. Oasis from condenser overlap in desert also valid → `sim/src/weather.rs`
3578. Map overlay colours for aridity legend → `game/scripts/overlay.gd`
3579. Photo metadata: rain, temp, aridity under reticle → `game/scripts/world.gd`
3580. Cross-ref 264–265 orographic/rain shadow — implement here → `docs/LANDSCAPE_800.md`
3581. Cross-ref 75 snow-by-temp — lapse enables it later → `docs/LANDSCAPE_200.md`
3582. Keep reactor power budget gating condensers → `sim/src/weather.rs`
3583. Aridity does not spend reactor power → `sim/src/weather.rs`
3584. Golden still: green windward vs dry leeward of massif → `shots/`
3585. Golden still: desert far from condensers → `shots/`
3586. CI test climate determinism hash → `sim/src/weather.rs`
3587. Ensure desert biome appears in census ≥1% on default seed after BQ+BU → `sim/src/biome.rs`
3588. Ensure wetland/swamp possible in humid flats → `sim/src/biome.rs`
3589. Temperature inversions: out of scope; state in MODEL_LIMITS → `docs/MODEL_LIMITS.md`
3590. Cloud caps on peaks when humidity and cold align → `sim/src/weather.rs`
3591. Dust potential field = aridity * wind_speed for BU dust → `sim/src/weather.rs`
3592. Heat shimmer hook: high temp + arid near camera (shader later) → `game/shaders/terrain.gdshader`
3593. Done means soil moisture not equal to flux everywhere → `sim/src/soil.rs`
3594. Done means summits colder than valleys at same θ → `sim/src/weather.rs`
3595. Done means rain shadow measurable in test → `sim/src/weather.rs`
3596. Done means aridity_at exposed to biome path → `sim/src/biosphere.rs`
3597. Done means FIELDS lists aridity and climatological rain → `docs/FIELDS.md`
3598. Done means DuneSea can stay dry without killing global wetlands → `sim/src/weather.rs`
3599. Done means no second conflicting weather system → `sim/src/weather.rs`
3600. Done means CALIBRATION documents lapse and orographic constants → `docs/CALIBRATION.md`

---

## BR. Seas, islands, shores (3601–3680)

*SeaBasin provinces carve below the waterline. The constant-radius water*
*sheet becomes a real inland sea with shelves, beaches, islands and deltas.*

3601. Tag flow cells in SeaBasin floors as sea (distinct from ponded lake) → `sim/src/flow.rs`
3602. Add sea_mask parallel to lake or encode in lake byte as enum → `sim/src/flow.rs`
3603. Waterline outlets remain; sea is intentional bathymetry below water_level → `sim/src/flow.rs`
3604. lake entities vs sea: seas are large connected components of sea_mask → `sim/src/lakes.rs`
3605. Name large seas for HUD map when area > threshold → `sim/src/lakes.rs`
3606. Island detect: elev islands completely surrounded by sea_mask → `sim/src/terrain.rs`
3607. Export island list: centroid θ,z, peak elev, area → `sim/src/lib.rs`
3608. Shoreline cells: elev within 0–2 m above water_level adjacent to sea → `sim/src/flow.rs`
3609. Beach band uses new SAND material near shore → `sim/src/material.rs`
3610. paint rules: pale beach albedo on SAND + shore → `sim/src/paint.rs`
3611. Shelf depth tint in water.gdshader from bathymetry (water_level - elev) → `game/shaders/water.gdshader`
3612. Deep sea darker; shallow cyan-green; respect RENDER_CONTRACT → `game/shaders/water.gdshader`
3613. Pass bathymetry via vertex colour or shader uniform texture from lake mask → `game/scripts/world.gd`
3614. Shoreline foam band where depth shallow and wind fetch → `game/shaders/water.gdshader`
3615. Channel ribbons mute inside sea_mask (already mute lakes) → `sim/src/flow.rs`
3616. River→sea: delta elev deposit when high discharge meets sea → `sim/src/erosion.rs`
3617. Delta paint: branching wet sediment fans → `sim/src/paint.rs`
3618. Far field: sea colour masses readable as dark continents of water → `sim/src/paint.rs`
3619. Far field islands as elev spikes above water plane → `sim/src/lib.rs`
3620. Water mesh still constant radius; bathymetry is visual + biome, not second mesh → `game/scripts/world.gd`
3621. Document: no wave heightfield sim; foam/tint are presentation → `docs/MODEL_LIMITS.md`
3622. sph pools still for dig pits; seas are elev-based → `sim/src/sph.rs`
3623. Standing water biome WATER on sea and deep lakes → `sim/src/biome.rs`
3624. New biome SHORE for beach band → `sim/src/biome.rs`
3625. SHORE id appended after existing ids; update COUNT → `sim/src/biome.rs`
3626. census colours for SHORE sandy → `sim/src/biome.rs`
3627. FIELDS.md register SHORE and sea_mask → `docs/FIELDS.md`
3628. Grass kill on SHORE; sparse dune grass optional later → `sim/src/lib.rs`
3629. Reeds at swampy shores not sandy beaches → `sim/src/plant.rs`
3630. Willow on wet non-sand shores → `sim/src/plant.rs`
3631. Island spawn: rare homestead only if area large and Meadow-like elev → `sim/src/dwelling.rs`
3632. find_spawn never on island unless debug flag → `sim/src/lib.rs`
3633. Boat/module fantasy out of scope; islands are destinations on foot via shallows if any → `docs/MODEL_LIMITS.md`
3634. Shallow fords: elev within 0.5 m of water_level as walkable wet (player rule) → `game/scripts/player.gd`
3635. Deep water: player swims or blocked — pick blocked for Wave 1 honesty → `game/scripts/player.gd`
3636. Document swimming as non-goal → `docs/MODEL_LIMITS.md`
3637. Sea audio: low wind fetch loop when near sea_mask → `game/scripts/audio.gd`
3638. Map HUD draws seas as water colour with island dots → `game/scripts/overlay.gd`
3639. Minimap shows local shoreline → `game/scripts/minimap_overlay.gd`
3640. Selftest: sea cell count > 0 on default seed → `game/scripts/world.gd`
3641. Selftest: island count ≥ 1 if SeaBasin present → `game/scripts/world.gd`
3642. Unit test: sea floor elev < water_level - 5 → `sim/src/terrain.rs`
3643. Unit test: island elev > water_level → `sim/src/terrain.rs`
3644. Prevent priority-flood from filling sea into mountains (outlets at waterline) → `sim/src/flow.rs`
3645. MAX_POND still caps absurd basins inland → `sim/src/flow.rs`
3646. Sea basins connect through θ wrap if adjacent → `sim/src/flow.rs`
3647. Coastal cliffs: Massif meeting SeaBasin → steep shore, rocky not sandy → `sim/src/paint.rs`
3648. Rocky shore material: no SAND; basalt/pebble paint → `sim/src/paint.rs`
3649. Pocket beaches between headlands → `sim/src/terrain.rs`
3650. Tide fiction: none — engineered waterline fixed → `docs/MODEL_LIMITS.md`
3651. Salinity field stub optional; skip for Wave 1 → `docs/NEXT.md`
3652. Fishery trophic later; sea is landform first → `docs/NEXT.md`
3653. Cloud reflections on water: use sky/strip colour in water shader → `game/shaders/water.gdshader`
3654. Cross-ref LANDSCAPE_3200 water sparkle items — seas need them more → `docs/LANDSCAPE_3200.md`
3655. Lake vs sea legend in overlay → `game/scripts/overlay.gd`
3656. River mouths widen visually near sea → `game/shaders/channel.gdshader`
3657. Wet sand darkening on SHORE when rain → `game/shaders/terrain.gdshader`
3658. Footprints on beach optional decal later → `docs/NEXT.md`
3659. Island peak alpine if tall enough via lapse → `sim/src/biome.rs`
3660. Small islands scrub/rock only → `sim/src/biome.rs`
3661. Archipelago: multiple islands in one SeaBasin from knob recipe → `sim/src/terrain.rs`
3662. Atoll-like ring not required; forbid unless elev naturally forms → `docs/MODEL_LIMITS.md`
3663. Ferry fantasy: no → `docs/MODEL_LIMITS.md`
3664. Dig on beach: sand collapses (BU dig rule) → `sim/src/terrain.rs`
3665. Fill into sea: elev raise can reclaim land and dirty flow → `sim/src/terrain.rs`
3666. Engineering gameplay: dig canal to sea already possible; celebrate in toast → `game/scripts/world.gd`
3667. Golden still: sea + islands drum view → `shots/`
3668. Golden still: beach colonist view → `shots/`
3669. Golden still: delta aerial/survey → `shots/`
3670. Photo metadata: sea depth under reticle if over water → `game/scripts/world.gd`
3671. Bathymetry in geologist tool if over sea → `sim/src/lib.rs`
3672. Ensure water.gdshader lake mask includes sea → `game/scripts/world.gd`
3673. Pool shader not used for open sea → `game/shaders/pool.gdshader`
3674. Performance: sea tint is shader; no extra sea mesh → `game/scripts/world.gd`
3675. Done means seas visible as dark water masses with islands → `docs/NEXT.md`
3676. Done means SHORE biome in census → `sim/src/biome.rs`
3677. Done means deltas form at major river mouths → `sim/src/erosion.rs`
3678. Done means player blocked from deep sea → `game/scripts/player.gd`
3679. Done means FIELDS documents sea vs lake → `docs/FIELDS.md`
3680. Done means default seed has ≥1 sea and ≥1 island → `sim/src/terrain.rs`

---

## BS. Swamps vs meadows (3681–3750)

*Distinguished by drainage state and relief recipe — not by sticking reeds*
*on grassland. New biome ids SWAMP and MEADOW make the split legible.*

3681. Append biome id SWAMP and MEADOW with stable u8 values → `sim/src/biome.rs`
3682. Update name/color/census COUNT for new ids → `sim/src/biome.rs`
3683. classify: SWAMP when moisture high, slope low, ponding or SwampBasin prior, not open sea → `sim/src/biome.rs`
3684. classify: MEADOW when mesic, gentle, Meadow province or drained fertile flats → `sim/src/biome.rs`
3685. Keep GRASSLAND for drier mixed herb; MEADOW is lush drained → `sim/src/biome.rs`
3686. Keep WETLAND as cool wet; SWAMP warmer/peatier if temp allows → `sim/src/biome.rs`
3687. FIELDS.md update biome id table → `docs/FIELDS.md`
3688. HUD census colours match biome::color → `game/scripts/world.gd`
3689. Swamp relief already SwampBasin; ensure ponding depth flags lake lightly → `sim/src/flow.rs`
3690. Hummocks: micro elev noise preserved through erosion → `sim/src/terrain.rs`
3691. Standing pools: sph or pool mesh in swamp depressions → `sim/src/sph.rs`
3692. Dark peat paint for SWAMP ground → `sim/src/paint.rs`
3693. Meadow warm soil between dense grass → `sim/src/paint.rs`
3694. Reeds MultiMesh density high in SWAMP → `game/scripts/world.gd`
3695. Meadow flowers flecks in grass shader (cross-ref 2455) → `game/shaders/grass.gdshader`
3696. Swamp mist pocket: local fog density when humidity high → `game/scripts/world.gd`
3697. Meadow: no mist; clear sightlines → `game/scripts/world.gd`
3698. Plant species: willow/reed in swamp; dense herb + scattered broadleaf edge in meadow → `sim/src/plant.rs`
3699. Tree density low in meadow interior; higher at meadow–forest ecotone → `sim/src/plant.rs`
3700. Swamp: canopy sparse; understorey reeds dominate → `sim/src/plant.rs`
3701. Agents avoid SWAMP for pathing bias (higher cost) → `sim/src/agent.rs`
3702. Dwellings: swamp not arable; meadow preferred with farm → `sim/src/dwelling.rs`
3703. find_spawn prefers MEADOW province + biome → `sim/src/lib.rs`
3704. Soil organic high in swamp; moisture capped wet → `sim/src/soil.rs`
3705. Meadow soil deep, mesic, farm-eligible → `sim/src/soil.rs`
3706. Audio: swamp insects dense; meadow birds/light wind → `game/scripts/audio.gd`
3707. Grass density: meadow max; swamp reduced for pools → `sim/src/lib.rs`
3708. Grass colour: meadow bright; swamp dark olive → `game/shaders/grass.gdshader`
3709. BIOME_GRASS_COL table extend for new ids → `game/scripts/world.gd`
3710. BIOME_PLANT_COL table extend → `game/scripts/world.gd`
3711. Overlay map shows swamp vs meadow contrast → `game/scripts/overlay.gd`
3712. Selftest census: both SWAMP and MEADOW counts > 0 → `game/scripts/world.gd`
3713. Unit test classify priority: sea WATER beats SWAMP → `sim/src/biome.rs`
3714. Unit test: swamp not classified on steep ground → `sim/src/biome.rs`
3715. Drainage state: filled-elev depth > 0.2 in swamp flats → `sim/src/flow.rs`
3716. Meadow: filled ≈ elev (drained) → `sim/src/flow.rs`
3717. Dig ditch in swamp: should drain locally and may flip biome over time → `sim/src/biosphere.rs`
3718. Toast when ditch drains swamp patch (delight) → `game/scripts/world.gd`
3719. Fill/block drainage to create swamp — engineered wetland gameplay → `sim/src/terrain.rs`
3720. Peat dig: soft dig_scale, spoil looks dark → `sim/src/material.rs`
3721. Cross-ref 3200 wetland reeds — swamp owns primary reed biome → `docs/LANDSCAPE_3200.md`
3722. Do not grey ecotone: top-two biome blend only → `sim/src/paint.rs`
3723. Golden still: swamp interior → `shots/`
3724. Golden still: meadow with flowers → `shots/`
3725. Golden still: swamp–meadow edge → `shots/`
3726. Photo caption can name drainage evidence → `game/scripts/world.gd`
3727. Far wall: swamp dark green mass; meadow lighter → `sim/src/paint.rs`
3728. Riparian still corridors; swamp is areal → `sim/src/biome.rs`
3729. WETLAND vs SWAMP documentation in MODEL_LIMITS → `docs/MODEL_LIMITS.md`
3730. Farm biome still deep soil gentle; meadow may transition to farm near dwellings → `sim/src/biome.rs`
3731. Grazers prefer meadow NPP → `sim/src/trophic.rs`
3732. Fear fields still work; swamp slows grazers optional → `sim/src/trophic.rs`
3733. Plant seed_stands rejection uses new biome gates → `sim/src/plant.rs`
3734. Litter: swamp leaves/peat clumps; meadow dry thatch → `game/scripts/world.gd`
3735. Rocks rare in swamp; occasional in meadow → `game/scripts/world.gd`
3736. Boardwalk fantasy out of scope → `docs/MODEL_LIMITS.md`
3737. Disease/mosquito sim out of scope; audio only → `docs/MODEL_LIMITS.md`
3738. Done means player names swamp vs meadow from still → `docs/NEXT.md`
3739. Done means ditching changes drainage readout → `sim/src/flow.rs`
3740. Done means spawn in meadow not swamp → `sim/src/lib.rs`
3741. Done means census includes both ids → `sim/src/biome.rs`
3742. Done means paint/grass differ → `sim/src/paint.rs`
3743. Moss on wet boulder sides in swamp → `game/shaders/prop.gdshader`
3744. Meadow anthills/molehills micro detail optional paint → `sim/src/paint.rs`
3745. Season: meadow flowers follow day cycle param if present → `sim/src/plant.rs`
3746. Drought: meadow browns via aridity; swamp persists wetter → `sim/src/paint.rs`
3747. Ice near endcap swamp → alpine/wet rock instead → `sim/src/biome.rs`
3748. Karst polje wet floor can classify swamp → `sim/src/biome.rs`
3749. Agents graze meadow plots first → `sim/src/agent.rs`
3750. Done means FIELDS and overlay updated → `docs/FIELDS.md`

---

## BT. Forest types (3751–3840)

*Cross-ref §Q 411–490 and §BB 2541–2620 for meshes/lifecycle. Here: derived*
*forest kinds, stand structure, and species tables so FOREST is not one cone.*

3751. Add ForestKind enum: Conifer, Broadleaf, SwampForest, OpenWoodland, Krummholz, RiparianGallery → `sim/src/biome.rs`
3752. Derive ForestKind from temp, moisture, aridity, elev, province, flux → `sim/src/biome.rs`
3753. Conifer: cold (endcap/lapse) + moist enough → `sim/src/biome.rs`
3754. Broadleaf: warm mesic forest → `sim/src/biome.rs`
3755. SwampForest: SWAMP/WETLAND + trees viable → `sim/src/biome.rs`
3756. OpenWoodland: dry margin forest / scrub transition → `sim/src/biome.rs`
3757. Krummholz: alpine + Massif high → `sim/src/biome.rs`
3758. RiparianGallery: high flux corridor trees → `sim/src/biome.rs`
3759. Expose forest_kind in plant LOD stride or parallel field → `sim/src/lib.rs`
3760. Species distribution table: ForestKind → archetype weights (ties §BB) → `sim/src/plant.rs`
3761. Conifer → cone archetype; Broadleaf → broad; Swamp → willow; Open → thorn/scrub; Krummholz → alpine; Gallery → willow/broad mix → `sim/src/plant.rs`
3762. Do not duplicate §BB mesh work; consume archetypes when present, else tint/scale cones → `game/scripts/world.gd`
3763. Stand structure: age classes via genome/carbon already — bias seed ages by kind → `sim/src/plant.rs`
3764. Canopy gaps: Poisson-like rejection holes in dense Broadleaf/Conifer → `sim/src/plant.rs`
3765. Edge densification: higher stems at forest–meadow boundary → `sim/src/plant.rs`
3766. Wind-thrown edge facing prevailing +z: snags and lean → `sim/src/plant.rs`
3767. Canopy density field 0..1 from local stem count → `sim/src/plant.rs`
3768. Grass suppression under canopy_density → `sim/src/lib.rs`
3769. Understorey light reduction stub for plants → `sim/src/plant.rs`
3770. Forest floor litter density from canopy → `game/scripts/world.gd`
3771. Deadwood/snags where kind allows (cross-ref 2551) → `sim/src/plant.rs`
3772. OpenWoodland: wide spacing, grass between → `sim/src/plant.rs`
3773. RiparianGallery: linear spawn along flux > thresh → `sim/src/plant.rs`
3774. Krummholz: short stature cap even if carbon high → `sim/src/plant.rs`
3775. SwampForest: buttress/willow lean toward pools → `game/scripts/world.gd`
3776. Conifer dark olive; Broadleaf yellow-green crowns (shader) → `game/shaders/tree.gdshader`
3777. Kind-specific wind sway amplitudes → `game/shaders/tree.gdshader`
3778. seed_stands uses ForestKind gates not only biome id FOREST → `sim/src/plant.rs`
3779. FOREST biome may host multiple kinds; kind is finer readout → `sim/src/biome.rs`
3780. HUD debug shows forest kind under reticle → `game/scripts/world.gd`
3781. Census: counts per ForestKind → `sim/src/plant.rs`
3782. Map overlay optional forest-kind mode → `game/scripts/overlay.gd`
3783. Far wall: dark conifer masses vs lighter broadleaf → `sim/src/paint.rs`
3784. Montane band: elev + cold → conifer ring on Massif flanks → `sim/src/biome.rs`
3785. Timberline: hard stop trees above temp/elev threshold except krummholz → `sim/src/plant.rs`
3786. Rain shadow slope: OpenWoodland or scrub instead of closed forest → `sim/src/biome.rs`
3787. Windward Massif: denser Conifer/Broadleaf → `sim/src/plant.rs`
3788. Cross-ref §Q stand structure items — implement gap/edge/kind here → `docs/LANDSCAPE_800.md`
3789. Avoid encyclopedia of species; ≤6 kinds → `docs/MODEL_LIMITS.md`
3790. FIELDS: forest_kind derived → `docs/FIELDS.md`
3791. Unit test: cold elev → Conifer or Krummholz → `sim/src/biome.rs`
3792. Unit test: arid → not closed Broadleaf → `sim/src/biome.rs`
3793. Unit test: high flux corridor → RiparianGallery preference → `sim/src/biome.rs`
3794. Selftest prints forest kind census → `game/scripts/world.gd`
3795. Plant fill budget still respected → `game/scripts/world.gd`
3796. MultiMesh pools by archetype not by every kind if colours encode kind → `game/scripts/world.gd`
3797. Genome_id maps into species table row → `sim/src/plant.rs`
3798. Shade-matched LOD when §BB lands; until then consistent tint → `game/shaders/tree.gdshader`
3799. Bird density follows canopy health (3200) — kind modulates → `sim/src/trophic.rs`
3800. Audio: conifer wind vs broadleaf rustle → `game/scripts/audio.gd`
3801. Fire/disturbance overlay later; kinds do not need fire yet → `docs/NEXT.md`
3802. Orchard is cultivated not ForestKind — stays §BB cultivated → `sim/src/plant.rs`
3803. Farm biome excludes closed forest seed → `sim/src/plant.rs`
3804. Town clearing: reduce canopy near dwellings → `sim/src/plant.rs`
3805. Path wear: thinner understorey near agent routes optional → `sim/src/plant.rs`
3806. Golden still: conifer flank → `shots/`
3807. Golden still: broadleaf interior gap → `shots/`
3808. Golden still: riparian gallery along river → `shots/`
3809. Golden still: timberline krummholz → `shots/`
3810. Golden still: open woodland savanna-like → `shots/`
3811. Photo metadata forest kind → `game/scripts/world.gd`
3812. Plant NPP/trophic uses kind leaf area proxies → `sim/src/trophic.rs`
3813. Carbon allocation already exists; kind changes allometry curves → `sim/src/plant.rs`
3814. Conifer taller thinner; broadleaf wider crown for same carbon → `sim/src/plant.rs`
3815. SwampForest shorter max height → `sim/src/plant.rs`
3816. Document allometry in CALIBRATION → `docs/CALIBRATION.md`
3817. Edge ecotone width 40–80 m for density ramp → `sim/src/plant.rs`
3818. Do not hard-cut forest at province boundary → `sim/src/plant.rs`
3819. Seed failure in SeaBasin floors and active dune slip faces → `sim/src/plant.rs`
3820. Beach: no forest; SHORE scrub only if any → `sim/src/plant.rs`
3821. Alpine rock: lichen only (3200) — no ForestKind → `sim/src/biome.rs`
3822. Performance: kind classify once per stand at seed, store on plant → `sim/src/plant.rs`
3823. Dirty regenerates kinds only if climate/biome fields change majorly → `sim/src/plant.rs`
3824. Done means six kinds appear in census on default seed → `sim/src/plant.rs`
3825. Done means grass suppressed under closed canopy → `sim/src/lib.rs`
3826. Done means riparian galleries follow rivers → `sim/src/plant.rs`
3827. Done means timberline visible on massifs → `sim/src/plant.rs`
3828. Done means species table drives archetype choice → `sim/src/plant.rs`
3829. Nurse logs / gap regeneration stub optional → `docs/NEXT.md`
3830. Browse line from grazers: lower foliage missing optional visual → `docs/NEXT.md`
3831. Epiphyte/moss on swamp forest trunks as tint → `game/shaders/tree.gdshader`
3832. Snow on conifer later with frost palette → `docs/MODEL_LIMITS.md`
3833. Keep honesty: kinds are ecological categories not Latin binomials → `docs/MODEL_LIMITS.md`
3834. Update LANDSCAPE_800 §Q pointer to this section for kind macro → `docs/LANDSCAPE_800.md`
3835. Agent woodcutting prefers OpenWoodland edge not sacred grove fantasy — practical → `sim/src/agent.rs`
3836. Chronicle: first sight of timberline event optional → `sim/src/chronicle.rs`
3837. Bisect: hide trees by kind for debug → `game/scripts/debug/bisect.gd`
3838. Quality preset: drop swamp forest and krummholz first on low → `game/scripts/world.gd`
3839. Done means player distinguishes ≥3 forest kinds from stills → `docs/NEXT.md`
3840. Done means FIELDS + tests green for forest_kind → `docs/FIELDS.md`

---

## BU. Deserts and dune seas (3841–3910)

*Aridity (BQ) + DuneSea relief (BN) + SAND material make deserts real.*
*Wind-aligned dunes, salt flats, oases — not yellow grassland.*

3841. Add material id SAND to palette (or map sediment subtype); document id stability → `sim/src/material.rs`
3842. SAND hardness low, cohesion low, albedo pale ochre → `sim/src/material.rs`
3843. dig_scale high for sand; walls collapse toward repose after dig → `sim/src/terrain.rs`
3844. Post-dig talus_relax on SAND strokes → `sim/src/erosion.rs`
3845. Append biome DESERT and DUNE ids → `sim/src/biome.rs`
3846. DESERT: arid + low moisture + not active dune form → `sim/src/biome.rs`
3847. DUNE: DuneSea province dominant + sand cover → `sim/src/biome.rs`
3848. name/color/census for DESERT and DUNE → `sim/src/biome.rs`
3849. FIELDS update → `docs/FIELDS.md`
3850. paint: dune ochre, desert varnish darker on stable rock desert → `sim/src/paint.rs`
3851. Ripple detail in terrain.gdshader for DUNE using world xz hash → `game/shaders/terrain.gdshader`
3852. Ripples fade with distance; near band only → `game/shaders/terrain.gdshader`
3853. Barchan vs transverse already in BN recipe; paint slip face darker → `sim/src/paint.rs`
3854. Salt flats: arid + ponding → white evaporite paint rule → `sim/src/paint.rs`
3855. Sabkha wet salt darker when moist → `sim/src/paint.rs`
3856. Oasis: moisture spike cells get palm/reed cluster and green paint → `sim/src/plant.rs`
3857. Oasis plants use reed/palm archetype if available else willow scaled → `game/scripts/world.gd`
3858. Rock desert / pediment: thin sand over sandstone where amp low in arid → `sim/src/material.rs`
3859. Hamada: bare rock arid — BARE_ROCK with arid paint tint → `sim/src/paint.rs`
3860. Grass kill in DESERT/DUNE except oasis → `sim/src/lib.rs`
3861. Sparse desert scrub: thorn archetype very low density → `sim/src/plant.rs`
3862. No forest kinds in desert interior → `sim/src/plant.rs`
3863. Dust potential from weather drives optional dust motes near camera → `game/scripts/world.gd`
3864. Heat shimmer: refraction-ish in post or terrain when arid+hot → `game/shaders/tiltshift.gdshader`
3865. Audio: dry wind hiss; no insects → `game/scripts/audio.gd`
3866. Far wall: pale dune belts readable → `sim/src/paint.rs`
3867. Agents avoid dunes for dwellings; oasis may settle → `sim/src/dwelling.rs`
3868. find_spawn rejects DUNE slip faces → `sim/src/lib.rs`
3869. Vehicle fantasy none; walking soft sand slows player slightly → `game/scripts/player.gd`
3870. Foot sink visual optional → `docs/NEXT.md`
3871. Sandstorm stub: visibility reduction when dust high — optional Wave+ → `docs/NEXT.md`
3872. Unit test: DESERT appears when aridity high in classify → `sim/src/biome.rs`
3873. Unit test: DuneSea elev anisotropic correlation with z → `sim/src/terrain.rs`
3874. Selftest: desert+dune census > 0 → `game/scripts/world.gd`
3875. Wind transport BP keeps dunes alive vs hydraulic smoothing → `sim/src/erosion.rs`
3876. Condenser in desert creates local green ring (gameplay) → `sim/src/weather.rs`
3877. Toast when first oasis found → `game/scripts/world.gd`
3878. Map overlay desert colours dusty ochre not orange (3200 rule) → `sim/src/biome.rs`
3879. Golden still: transverse dunes +z → `shots/`
3880. Golden still: oasis → `shots/`
3881. Golden still: salt flat → `shots/`
3882. Golden still: rock desert pediment → `shots/`
3883. Photo metadata biome desert/dune → `game/scripts/world.gd`
3884. SAND in strata_column near dunes → `sim/src/material.rs`
3885. Economy: sand as dig yield bulk optional → `sim/src/economy.rs`
3886. Glass craft later — out of scope note → `docs/NEXT.md`
3887. Night cold deserts: lapse + clear sky — temp already → `sim/src/weather.rs`
3888. Desert varnish on long-stable arid rock (cross-ref 246) → `sim/src/paint.rs`
3889. MODEL_LIMITS: no full aeolian grain sim → `docs/MODEL_LIMITS.md`
3890. CALIBRATION: dune wavelength and slip angle → `docs/CALIBRATION.md`
3891. Quality low: disable ripple shader term → `game/scripts/world.gd`
3892. Bisect: desert props group → `game/scripts/debug/bisect.gd`
3893. Ensure DuneSea pedestal above waterline except sabkha % → `sim/src/terrain.rs`
3894. Flooded dune edge → SHORE/SWAMP classify carefully → `sim/src/biome.rs`
3895. Player dig canal from sea into desert: allowed; interesting → `sim/src/terrain.rs`
3896. Done means dunes read as dunes not hills → `docs/NEXT.md`
3897. Done means SAND material digs and collapses → `sim/src/terrain.rs`
3898. Done means desert in census and map → `sim/src/biome.rs`
3899. Done means oasis possible near condensers → `sim/src/plant.rs`
3900. Done means salt flats where arid ponds → `sim/src/paint.rs`
3901. Prop rocks iron-stained in desert → `game/shaders/prop.gdshader`
3902. No meadow flowers in desert → `game/shaders/grass.gdshader`
3903. Scrub biome vs DESERT: scrub less arid; keep both → `sim/src/biome.rs`
3904. Priority: DUNE before DESERT before SCRUB in classify order → `sim/src/biome.rs`
3905. Farprobe: dune belt coverage on far wall → `game/scripts/debug/farprobe.gd`
3906. Bounce tint from desert far side warms shadowed near terrain → `game/shaders/rama_light.gdshaderinc`
3907. Document anti-emerald / anti-orange desert palette → `docs/RENDER_CONTRACT.md`
3908. CI contrast check includes desert vs meadow Delta E → `tools/`
3909. Done means FIELDS lists SAND DESERT DUNE → `docs/FIELDS.md`
3910. Done means default seed shows a dune sea from drum view → `shots/`

---

## BV. Ecotones and transitions (3911–3960)

*Hard seams kill the cylinder. Province and biome edges must blend;*
*golden transition shots gate the work.*

3911. Province blend width default 280 m; expose in params → `sim/src/province.rs`
3912. Clamp blend width 150–400 m → `sim/src/province.rs`
3913. Top-two province weights only everywhere elev/climate sample → `sim/src/terrain.rs`
3914. Top-two biome blend for paint (existing paint rule) → `sim/src/paint.rs`
3915. Grass density ramps across biome edges over 30–60 m → `sim/src/lib.rs`
3916. Plant density ramps across forest–meadow edges → `sim/src/plant.rs`
3917. Forbid visible θ-seam from province lattice; wrap test → `sim/src/province.rs`
3918. Dither biome edges in far paint to avoid polygon borders → `sim/src/paint.rs`
3919. Hash variation inside biome lower than contrast between biomes → `docs/CALIBRATION.md`
3920. Transition golden: wetland/swamp to meadow → `shots/`
3921. Transition golden: meadow to forest → `shots/`
3922. Transition golden: forest to alpine/massif → `shots/`
3923. Transition golden: scrub to desert → `shots/`
3924. Transition golden: desert to oasis → `shots/`
3925. Transition golden: shore to coastal scrub → `shots/`
3926. Transition golden: massif to sea coastal cliff → `shots/`
3927. tools/: script to compare edge Delta E → `tools/`
3928. Plan overlay debug: draw province boundaries faintly → `game/scripts/overlay.gd`
3929. Plan overlay debug: biome edges → `game/scripts/overlay.gd`
3930. Selftest fails if province seam Δ elev > 8 m across 1 cell without blend → `sim/src/terrain.rs`
3931. Ecotone object order: ground, understorey, signature (3200 rule) → `sim/src/paint.rs`
3932. No unique meshes that only exist as 1-cell thick walls → `game/scripts/world.gd`
3933. River always overrides dry grass in narrow riparian band → `sim/src/paint.rs`
3934. Sea always overrides adjacent land paint within wet pixels → `sim/src/paint.rs`
3935. Document ecotone widths in CALIBRATION → `docs/CALIBRATION.md`
3936. Rain shadow ecotone: forest to scrub across ridge — must be gradual → `sim/src/biome.rs`
3937. EndcapWall to Massif blend soft → `sim/src/province.rs`
3938. Meadow to SwampBasin: drainage gradient visible → `sim/src/flow.rs`
3939. Player should not teleport biomes in 2 m walk → `docs/RENDER_CONTRACT.md`
3940. Photo orbit path across one ecotone for QA → `game/scripts/world.gd`
3941. Quality presets must not sharpen edges oddly → `game/scripts/world.gd`
3942. Far wall continents soft-edged → `sim/src/paint.rs`
3943. Near band can be sharper but still top-two blend → `sim/src/paint.rs`
3944. Unit test bilinear province weights sum ≈ 1 → `sim/src/province.rs`
3945. Reject weighted average of all 9+ biomes → `docs/RENDER_CONTRACT.md`
3946. Disturbance overlay does not create hard biome id change → `sim/src/paint.rs`
3947. Farm patchwork edges soft at field scale → `sim/src/paint.rs`
3948. Done means transition stills exist in shots/ → `shots/`
3949. Done means seam test passes → `sim/src/province.rs`
3950. Done means blend width documented → `docs/CALIBRATION.md`
3951. Signature object cap prevents ecotone clutter → `game/scripts/world.gd`
3952. Audio crossfade between biome loops over 20 m → `game/scripts/audio.gd`
3953. Mist only on swamp side of swamp–meadow edge → `game/scripts/world.gd`
3954. Dust only on desert side of desert–scrub edge → `game/scripts/world.gd`
3955. Tree line timberline is elev/temp not province wall → `sim/src/plant.rs`
3956. Island shores: SHORE→interior kind ramp on small islands → `sim/src/biome.rs`
3957. Delta ecotone: river sediment into sea colour → `sim/src/paint.rs`
3958. Fan ecotone: Massif rock to meadow sediment → `sim/src/paint.rs`
3959. Done means no hard province wallpaper look → `docs/NEXT.md`
3960. Done means RENDER_CONTRACT mentions landform ecotone rule → `docs/RENDER_CONTRACT.md`

---

## BW. Meshing, LOD and rendering for tall steep land (3961–4040)

*440 m peaks and cliffs stress radial bands, skylines, fog and budgets.*
*Fix the mesh contract before piling on props.*

3961. Replace fixed lo_e-96/hi_e+40 with margins from local province max relief → `sim/src/lib.rs`
3962. rk_lo margin ≥ max(96, 0.25*local_hi_e); rk_hi margin ≥ 40 + overhang budget → `sim/src/lib.rs`
3963. Cap n_rad so chunk mesh memory stays bounded; document cap → `docs/MODEL_LIMITS.md`
3964. If relief exceeds cap, split radial or reduce cell locally — prefer wider margin first → `sim/src/lib.rs`
3965. Selftest: no black rectangular holes on massif valleys after elev raise → `game/scripts/world.gd`
3966. Far field elev sampling preserves peaks (BN skyline rule) → `sim/src/lib.rs`
3967. Mid field span still 1500 m; ensure massifs inside window resolve → `game/scripts/world.gd`
3968. NEAR_FADE distances may need retune after taller relief; derive not bake → `game/scripts/world.gd`
3969. Fog depth end remains drum_diagonal()*1.15 → `game/scripts/world.gd`
3970. cam_far remains drum_diagonal()*1.15 → `game/scripts/player.gd`
3971. Shadow split distance stays focus-band ~240 m not habitat-sized → `game/scripts/world.gd`
3972. Taller peaks outside shadow distance use bounce/haze only → `game/shaders/rama_light.gdshaderinc`
3973. Slope-split cliff shading in terrain.gdshader (3200 Wave 3 overlap OK) → `game/shaders/terrain.gdshader`
3974. Overhang normals from SDF; flat_shade AO under lips → `sim/src/mesher.rs`
3975. Threaded meshing --threaded retest after heavier radial slabs → `sim/src/lib.rs`
3976. Budget overlay: chunk tri count and radial depth → `game/scripts/world.gd`
3977. Quality deck: reduce CHUNK_RADIUS before breaking skylines → `game/scripts/world.gd`
3978. Far sectors: anti-theta streak paint still required → `game/shaders/terrain.gdshader`
3979. Sea depth tint must work on far water sheet → `game/shaders/water.gdshader`
3980. Dune ripple only near; far uses macro dune colour → `game/shaders/terrain.gdshader`
3981. Measure meshing ms/chunk on massif vs meadow; record CALIBRATION → `docs/CALIBRATION.md`
3982. plant_lod_radius may grow for taller tree lines visibility — derive from diagonal fraction → `game/scripts/world.gd`
3983. Grass radius stays near; do not grass whole massif → `game/scripts/world.gd`
3984. Instancing caps unchanged unless budget proves room → `docs/MODEL_LIMITS.md`
3985. Endcap meshes still seal; EndcapWall province meets them → `game/scripts/world.gd`
3986. No metre literals in shaders; use uniforms from world.gd → `tools/check_shader_literals.py`
3987. Update check if new cut-offs added → `tools/check_shader_literals.py`
3988. Farprobe after elev raise: no voids → `game/scripts/debug/farprobe.gd`
3989. Bisect groups: near/mid/far/water/props → `game/scripts/debug/bisect.gd`
3990. LOD handoff: near faceted vs mid smooth — document known gap; do not worsen → `docs/HANDOFF_FAR_FIELD.md`
3991. Heightfield mid must use same elev as near → `sim/src/lib.rs`
3992. Caves under tall massifs: radial band must include cave depth term → `sim/src/lib.rs`
3993. Density cave fade still >7 m below surface → `sim/src/terrain.rs`
3994. Remesh queue: digs on cliffs remesh neighbour chunks → `game/scripts/world.gd`
3995. Unload radius still 6; memory OK with deeper radial? → `game/scripts/world.gd`
3996. If memory high, shrink cave detail before peak margin → `docs/NEXT.md`
3997. Surface nets emit disjoint ownership unchanged → `sim/src/mesher.rs`
3998. Lattice LATTICE_CELL 1.4 / CHUNK_N 32 unchanged unless proven need → `sim/src/lib.rs`
3999. NT_LAT wrap still exact → `sim/src/lib.rs`
4000. Sagitta justification still holds at 32 m chord → `docs/EM_BRIEF.md`
4001. Player ground_below start -40 may need -max(40, 0.15*local_relief) → `game/scripts/world.gd`
4002. Raycast dig max distance OK on tall faces → `sim/src/terrain.rs`
4003. Water sheet z-order vs tall islands: islands poke through correctly → `game/scripts/world.gd`
4004. Island near mesh when player visits; far when not → `game/scripts/world.gd`
4005. Streaming crosses sea without holes → `game/scripts/world.gd`
4006. GPU: MSAA 2x remains default → `game/scripts/world.gd`
4007. No SDFGI required for landforms → `docs/RENDER_CONTRACT.md`
4008. Tilt-shift focus band still ~240 m → `game/shaders/tiltshift.gdshader`
4009. Survey view strips haze to read provinces → `game/scripts/world.gd`
4010. Drum view emphasises sea/dune/massif continents → `game/scripts/world.gd`
4011. Benchmark generate+first mesh after changes → `sim/src/bin/bench.rs`
4012. Fix bench.rs compile so it can measure (known broken) → `sim/src/bin/bench.rs`
4013. CI does not require GPU shots; CPU tests enough for elev → `.github/workflows/ci.yml`
4014. Document frame budget still 60 fps target with new relief → `docs/REQUIREMENTS.md`
4015. Amortise plant upload still PLANT_FILL_BUDGET → `game/scripts/world.gd`
4016. Dirty region updates for flow after sea dig reclaim → `sim/src/flow.rs`
4017. Vertex colour precision enough for new biome ids → `sim/src/lib.rs`
4018. Normal blur on far for tall ridges reduce sparkle → `sim/src/lib.rs`
4019. Cliff sparkle: reduce specular on rock → `game/shaders/terrain.gdshader`
4020. Wet rock on sea cliffs darker → `game/shaders/terrain.gdshader`
4021. Cloud shadows still low-freq multiplier → `game/shaders/terrain.gdshader`
4022. Axis strip fill not a second sun → `game/shaders/rama_light.gdshaderinc`
4023. Bounce from opposite biome includes desert/sea colours → `game/shaders/rama_light.gdshaderinc`
4024. Done means radial band covers 440 m peaks without holes → `sim/src/lib.rs`
4025. Done means farprobe clean → `game/scripts/debug/farprobe.gd`
4026. Done means fog/camera derived → `game/scripts/world.gd`
4027. Done means meshing budget measured → `docs/CALIBRATION.md`
4028. Done means --threaded safe or stays off with note → `docs/NEXT.md`
4029. Impostor trees on far massif flanks optional colour only → `sim/src/paint.rs`
4030. No new LOD system — extend existing three tiers → `docs/SIM_ARCH_BRIEF.md`
4031. Water mask texture resolution enough for islands → `game/scripts/world.gd`
4032. River rebuild after tall watersheds still < frame spike via deferred → `game/scripts/world.gd`
4033. Catchment HUD works on massif slopes → `sim/src/flow.rs`
4034. Minimap SubViewport far plane scaled → `game/scripts/minimap_overlay.gd`
4035. Photo mode orbit clears tall peaks → `game/scripts/world.gd`
4036. Selftest 360 visibility still passes → `game/scripts/world.gd`
4037. Codesign/build.sh unchanged process → `build.sh`
4038. Document chunk memory formula with n_rad → `docs/SIM_ARCH_BRIEF.md`
4039. Done means HANDOFF_FAR_FIELD updated for tall relief → `docs/HANDOFF_FAR_FIELD.md`
4040. Done means no baked 235-era margins left in lib.rs → `sim/src/lib.rs`

---

## BX. Gameplay consequences (4041–4100)

*Landforms must change how you walk, dig, spawn and settle — or they are*
*only wallpaper.*

4041. find_spawn prefers Meadow province + MEADOW/FARM biome + gentle slope → `sim/src/lib.rs`
4042. find_spawn rejects SeaBasin floor, DUNE slip, cliffiness high, deep swamp → `sim/src/lib.rs`
4043. Spawn score adds province meadow weight → `sim/src/lib.rs`
4044. Dwelling arable slope 0.25 unchanged; biome must be farm-eligible → `sim/src/dwelling.rs`
4045. No dwellings in DESERT/DUNE except oasis exception flag → `sim/src/dwelling.rs`
4046. No dwellings in WATER/SHORE → `sim/src/dwelling.rs`
4047. Cliff band movement block + col routes (BO) → `game/scripts/player.gd`
4048. Scree slide control loss → `game/scripts/player.gd`
4049. Soft sand slow in DUNE → `game/scripts/player.gd`
4050. Deep sea blocked; shallows ford OK → `game/scripts/player.gd`
4051. Low-g summit jumps: verify jump height scales with local g → `game/scripts/player.gd`
4052. Coriolis still applies airborne; more airtime on summits → `game/scripts/player.gd`
4053. Instrument: local g and elev on survey HUD → `game/scripts/world.gd`
4054. Island destinations: waypoint can target island centroids → `game/scripts/world.gd`
4055. Map shows province names on hover/debug → `game/scripts/overlay.gd`
4056. Map shows sea/island markers → `game/scripts/overlay.gd`
4057. Dig sand collapse feedback toast → `game/scripts/world.gd`
4058. Dig peat soft + dark spoil → `game/scripts/world.gd`
4059. Dig canal to sea toast when sea_mask connects → `game/scripts/world.gd`
4060. Drain swamp ditch toast → `game/scripts/world.gd`
4061. Agents avoid swamp tiles in wander when possible → `sim/src/agent.rs`
4062. Agents prefer meadow for plots → `sim/src/agent.rs`
4063. Agents do not path through deep sea → `sim/src/agent.rs`
4064. Place_module snaps ground_at; reject if underwater → `game/scripts/world.gd`
4065. Levelling brush on sand needs wider repose fix → `sim/src/terrain.rs`
4066. Rockfall risk overlay optional for build sites near cliffs → `game/scripts/world.gd`
4067. Slope-stability warning when digging undercut cliff → `game/scripts/world.gd`
4068. Homestead prefers meadow near flux for water → `game/scripts/world.gd`
4069. Towns avoid dune seas in _build_towns → `game/scripts/world.gd`
4070. Chronicle: first mountain summit visit → `sim/src/chronicle.rs`
4071. Chronicle: first sea sighting → `sim/src/chronicle.rs`
4072. Chronicle: first desert crossing → `sim/src/chronicle.rs`
4073. Help text / intent strings for new biomes → `docs/INTENT_STRINGS.md`
4074. Menu credits/docs link landform register → `game/scripts/menu.gd`
4075. Playtest marks: time to first sea, first summit → `game/scripts/world.gd`
4076. Photo mode composition includes landform variety checklist → `game/scripts/world.gd`
4077. Reticle debug: province, biome, forest kind, aridity → `game/scripts/world.gd`
4078. Default HUD does not spam biome names (infer first) → `docs/RENDER_CONTRACT.md`
4079. Waypoint across drum still works with seas (go around) → `game/scripts/player.gd`
4080. No navmesh still; agents plot-local → `docs/REQUIREMENTS.md`
4081. Economy dig yield sand vs ore unchanged rules → `sim/src/economy.rs`
4082. Condenser placement in desert is high-value gameplay → `sim/src/weather.rs`
4083. Reactor budget still gates condensers → `sim/src/weather.rs`
4084. Farm water: meadow mesic enough without; desert needs condenser → `sim/src/soil.rs`
4085. Grazer density follows meadow NPP not desert → `sim/src/trophic.rs`
4086. Carcasses rare in dunes → `sim/src/trophic.rs`
4087. Fear fields unchanged mechanically → `sim/src/trophic.rs`
4088. Save/load: provinces not saved; strokes still work on new elev → `sim/src/persist.rs`
4089. Warn if save from old max_elevation? optional migration note → `docs/FIELDS.md`
4090. Seed override generate(seed) still deterministic landforms → `sim/src/lib.rs`
4091. Multiplayer later: provinces from seed agree; strokes merge → `docs/REQUIREMENTS.md`
4092. Tutorial beat: walk from meadow to shore within first session if seed allows → `docs/PD_BRIEF.md`
4093. Delight: opposite wall shows different province colour mass → `docs/RENDER_CONTRACT.md`
4094. Climb fatigue optional — skip Wave 1 → `docs/NEXT.md`
4095. Fall damage from cliffs optional — document → `docs/MODEL_LIMITS.md`
4096. Swim stamina none — blocked deep water → `docs/MODEL_LIMITS.md`
4097. Done means spawn in meadow → `sim/src/lib.rs`
4098. Done means cliffs alter routes → `game/scripts/player.gd`
4099. Done means digs in sand/peat feel different → `sim/src/terrain.rs`
4100. Done means map shows provinces/seas → `game/scripts/overlay.gd`

---

## BY. Tests, gates, tooling, docs (4101–4160)

*Without gates the drum will silently flatten again.*

4101. Unit test elev hash stable for kepler_drum seed after full generate → `sim/src/terrain.rs`
4102. Unit test hypsometric buckets meet min sea / peak fractions → `sim/src/terrain.rs`
4103. Unit test province census guarantees → `sim/src/province.rs`
4104. Unit test biome census includes SWAMP MEADOW DESERT DUNE SHORE WATER → `sim/src/biome.rs`
4105. Unit test forest kind census ≥4 kinds present → `sim/src/plant.rs`
4106. Unit test rain shadow differential → `sim/src/weather.rs`
4107. Unit test lapse rate magnitude → `sim/src/weather.rs`
4108. Unit test θ wrap province continuity → `sim/src/province.rs`
4109. Unit test sea floor below water_level → `sim/src/terrain.rs`
4110. Unit test island above water_level → `sim/src/terrain.rs`
4111. Selftest: print all censuses → `game/scripts/world.gd`
4112. Selftest: fail if sea_count==0 → `game/scripts/world.gd`
4113. Selftest: fail if max_elev < 0.85*max_elevation → `game/scripts/world.gd`
4114. Selftest: far coverage after tall relief → `game/scripts/world.gd`
4115. Selftest: flow reroute dig still works → `game/scripts/world.gd`
4116. Fix sim/src/bin/bench.rs compile (economy path) → `sim/src/bin/bench.rs`
4117. bench: generate timing, density ns, mesh massif chunk → `sim/src/bin/bench.rs`
4118. tools/: landform golden still checklist script → `tools/`
4119. tools/: biome contrast Delta E including new ids → `tools/`
4120. tools/: province map export PNG → `tools/`
4121. CI: cargo test includes new tests → `.github/workflows/ci.yml`
4122. CI: shader literal check still passes → `tools/check_shader_literals.py`
4123. Update CALIBRATION.md: max_elev 440, lapse, dune λ, blend width, repose angles → `docs/CALIBRATION.md`
4124. Update MODEL_LIMITS.md: provinces, climate toy, no swim, low-g summits, aeolian limits → `docs/MODEL_LIMITS.md`
4125. Update FIELDS.md: province, aridity, sea_mask, new biome ids, forest_kind, SAND → `docs/FIELDS.md`
4126. Update REQUIREMENTS.md: zoning allowed; 60fps with tall relief → `docs/REQUIREMENTS.md`
4127. Update SIM_ARCH_BRIEF.md: province in terrain-from-process diagram → `docs/SIM_ARCH_BRIEF.md`
4128. Update RENDER_CONTRACT.md: ecotone, desert palette, sea tint → `docs/RENDER_CONTRACT.md`
4129. Update HANDOFF_FAR_FIELD.md: tall peaks sampling → `docs/HANDOFF_FAR_FIELD.md`
4130. Update INTENT_STRINGS.md for new place names → `docs/INTENT_STRINGS.md`
4131. Cross-link LANDSCAPE_800 §N/§Q to 4200 sections → `docs/LANDSCAPE_800.md`
4132. Cross-link LANDSCAPE_3200 §BA/§BB to 4200 kinds/palette → `docs/LANDSCAPE_3200.md`
4133. README gotcha: max elev / low-g note short → `README.md`
4134. Golden shots directory landform set listed in docs → `shots/`
4135. farprobe exit non-zero on void after raise → `game/scripts/debug/farprobe.gd`
4136. bisect documents new groups → `game/scripts/debug/bisect.gd`
4137. Playtest JSON marks for landform milestones → `game/scripts/world.gd`
4138. Determinism across macOS/Linux in CI matrix if available → `.github/workflows/ci.yml`
4139. Float hash: use integer elev quantise for hash test → `sim/src/terrain.rs`
4140. Document known non-goals list in MODEL_LIMITS one paragraph → `docs/MODEL_LIMITS.md`
4141. Failing test if biome id added without color/name → `sim/src/biome.rs`
4142. Failing test if province archetype missing table row → `sim/src/province.rs`
4143. Failing test if SAND missing from PALETTE when feature flag on → `sim/src/material.rs`
4144. Screenshot caption QA checklist in RENDER_CONTRACT → `docs/RENDER_CONTRACT.md`
4145. Perf regression gate: generate < 2× baseline or documented → `docs/CALIBRATION.md`
4146. Mesh tris/chunk gate on massif sample → `docs/CALIBRATION.md`
4147. Memory note for deeper radial bands → `docs/SIM_ARCH_BRIEF.md`
4148. Save format version bump if biome id bytes widen — prefer append-only ids → `sim/src/persist.rs`
4149. Overlay legend unit test colours match Rust → `tools/`
4150. Docs date stamp 2026-09-07 on 4200 header already → `docs/LANDSCAPE_4200.md`
4151. NEXT.md points at Wave 1 of this register → `docs/NEXT.md`
4152. Orphan item check: every item has → backticks path → `tools/`
4153. Number continuity check script 3201–4200 → `tools/`
4154. Inspiration README note on zoning fiction → `docs/inspiration/README.md`
4155. PD_BRIEF delight: first 90s show landform variety if seed kind → `docs/PD_BRIEF.md`
4156. EM_BRIEF risk: meshing cost of tall relief → `docs/EM_BRIEF.md`
4157. Done means cargo test green with new modules → `sim/Cargo.toml`
4158. Done means selftest fails loudly on missing sea/peak → `game/scripts/world.gd`
4159. Done means docs suite updated → `docs/FIELDS.md`
4160. Done means bench compiles → `sim/src/bin/bench.rs`

---

## BZ. Sequence (4161–4200)

*Ten waves. Broad structure first; props last. Do not start assets before*
*province + recipes + climate fields exist.*

4161. Wave 1 = zoned land: province.rs, blended recipes, max_elev ~440, sea basins, aridity+lapse, new biome ids, SAND, cliff terraces+talus, map overlay, determinism tests → `docs/NEXT.md`
4162. Wave 1 done means default seed shows sea, dunes, massif, meadow as distinct masses → `docs/NEXT.md`
4163. Wave 2 = climate polish + rain shadow vegetation response + soil moisture retune → `sim/src/weather.rs`
4164. Wave 2 done means leeward scrub vs windward forest readable → `docs/NEXT.md`
4165. Wave 3 = cliffs/SDF microforms from §N subset + movement blocking → `sim/src/terrain.rs`
4166. Wave 3 done means player routes around a cliff to a col → `docs/NEXT.md`
4167. Wave 4 = swamp vs meadow presentation (peat, reeds, flowers, mist) → `sim/src/paint.rs`
4168. Wave 4 done means stills name swamp vs meadow without HUD → `docs/NEXT.md`
4169. Wave 5 = forest kinds + species table consuming §BB archetypes → `sim/src/plant.rs`
4170. Wave 5 done means ≥3 forest kinds in stills → `docs/NEXT.md`
4171. Wave 6 = desert presentation: ripples, salt, oasis, dust → `game/shaders/terrain.gdshader`
4172. Wave 6 done means dunes read as dunes from colonist view → `docs/NEXT.md`
4173. Wave 7 = seas presentation: bathymetry tint, foam, deltas, map islands → `game/shaders/water.gdshader`
4174. Wave 7 done means sea+islands from drum view → `docs/NEXT.md`
4175. Wave 8 = ecotone QA + golden transition grid + contrast gates → `tools/`
4176. Wave 8 done means transition shots and Delta E pass → `docs/NEXT.md`
4177. Wave 9 = meshing/LOD/perf recovery for tall relief + bench → `sim/src/lib.rs`
4178. Wave 9 done means farprobe clean and budget measured → `docs/CALIBRATION.md`
4179. Wave 10 = docs freeze: FIELDS CALIBRATION MODEL_LIMITS REQUIREMENTS RENDER HANDOFF → `docs/FIELDS.md`
4180. Wave 10 done means future changes cannot silently flatten provinces → `docs/NEXT.md`
4181. Do not start tree archetype art before ForestKind fields exist → `docs/NEXT.md`
4182. Do not start dune ripples before DuneSea recipe + DESERT biome → `docs/NEXT.md`
4183. Do not start sea foam before SeaBasin bathymetry → `docs/NEXT.md`
4184. Do not raise max_elev without radial band update same PR → `sim/src/lib.rs`
4185. Do not add biome ids without FIELDS + census colours same PR → `sim/src/biome.rs`
4186. Do not paint deserts without aridity field → `sim/src/paint.rs`
4187. Displaced LANDSCAPE_3200 Wave 1 render items remain registered; re-queue after landform Wave 1 → `docs/LANDSCAPE_3200.md`
4188. Sun/light contract still required for screenshots; may interleave after province elev lands → `docs/NEXT.md`
4189. Sequence rule: structure → climate → biome ids → presentation → perf → docs → `docs/NEXT.md`
4190. First week of landform work is Wave 1 only → `docs/NEXT.md`
4191. Cap NEXT at ten; move finished items off → `docs/NEXT.md`
4192. Catalogue is register not queue (item 799 rule) → `docs/LANDSCAPE_800.md`
4193. Each wave updates CALIBRATION tags from invented to measured where possible → `docs/CALIBRATION.md`
4194. Each wave adds ≥1 golden still → `shots/`
4195. Each wave keeps ./build.sh + selftest green → `build.sh`
4196. Refuse scope: tectonics, GCM, full grain aeolian, swimming, boats, navmesh → `docs/MODEL_LIMITS.md`
4197. Refuse scope: painted province textures as source of truth → `docs/SIM_ARCH_BRIEF.md`
4198. Refuse scope: heightfield-only world without SDF → `docs/REQUIREMENTS.md`
4199. Success metric: player describes five landforms after ten minutes without opening map names → `docs/PD_BRIEF.md`
4200. And the rule, unchanged: honesty of mechanism beats cosmetic spectacle; legibility beats completeness; delight in the first ninety seconds beats feature count; stated limits beat implied precision → `docs/NEXT.md`

---

## What to build first, out of all 1000

| Wave | Build first | Done when |
|---|---|---|
| 1 | Province field + blended relief recipes + max_elev ~440 + sea basins + aridity/lapse + new biome ids + SAND + cliff/talus + map/tests | Default seed shows sea, dunes, massif, meadow as distinct masses; elev hash stable |
| 2 | Climate polish: rain shadow vegetation + soil moisture retune | Windward forest vs leeward scrub readable |
| 3 | Cliff SDF microforms + movement blocking + cols | Player routes around a cliff |
| 4 | Swamp vs meadow presentation | Stills name swamp vs meadow without HUD |
| 5 | Forest kinds + species table (§BB meshes) | ≥3 forest kinds in stills |
| 6 | Desert presentation: ripples, salt, oasis, dust | Dunes read as dunes |
| 7 | Sea presentation: bathymetry, foam, deltas, islands on map | Sea+islands from drum view |
| 8 | Ecotone QA + transition goldens + contrast gates | Transitions pass Delta E |
| 9 | Meshing/LOD/perf for tall relief + bench | Farprobe clean; budget measured |
| 10 | Docs freeze (FIELDS, CALIBRATION, MODEL_LIMITS, REQUIREMENTS, RENDER, HANDOFF) | Flattening cannot land silently |

## The shortest useful summary

Add a **province zoning layer**, give each archetype a **real relief recipe**, raise
peaks to **~440 m**, carve **sea basins with islands**, invent **aridity and lapse
rate** so deserts and forest kinds can exist, split **swamp vs meadow** by
drainage, then spend presentation and mesh budget making those structures
legible. Cross-ref mountains §N, forests §Q, palette §BA and tree archetypes §BB —
do not redo them. **Structure before props; fields before paint.**
