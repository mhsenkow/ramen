# 600 More: Forests, Rivers, Rain, Mountains, and Graphics That Make Sense Here

**Status:** v1.0 · **Date:** 2026-09-06 · **Items 201–800**
**Continues:** `LANDSCAPE_200.md` (items 1–200)
**Subject:** the living surface and how it is drawn.

---

## Adopted from ORRERY, second pass

The first review took its prioritisation rule. This one takes its *architecture*.

| ORRERY pattern | What we copy |
|---|---|
| `paintEval.js` — "a ramp with conditions, not a lambda per world" | Our `color_at()` is a hardcoded if-else in Rust. Colour must become a **compiled data table** with conditions and ramps (§W). |
| `present.js` — `applyLight`, `windSway`, `wearAt`, `seasonAt`, `waterStage` | A single **presentation layer** deriving look-time values from sim state, so no renderer ever invents its own truth. |
| `chronicle.js` — `whatHappenedHere` | A per-cell **event chronicle** the player can query. This is item 197's causal chain, made into a system. |
| `lifeColour.js` — `legendEntries`, `cellMatchesLegend` | Legends you can **click to highlight** matching ground. The legend becomes a query, not a key. |
| `isoline.js` | Contours and isolines over any field — the cheapest legibility win available. |
| `weather.js` — `precipTypeAt`, `visibilityReduction`, `frostDewAt`, `rainbowAt` | Weather exposed as **granular presentation hooks**, not one global state. |
| `sky-model.md` — two frames, two clocks, published limits, calibration spine with provenance tags | The documentation discipline for every field we add. |
| `briefs/accessibility.md`, `contrast-audit.md` | A contrast audit as a **CI gate**, not an afterthought. |
| Numbered registers (`weather-500`, `quality-400`) | This document's own format. |

---

## Verified state at time of writing

Items from `LANDSCAPE_200.md` **landed while this document was being written**
(`sim/src/flow.rs`, `sim/src/material.rs`, `docs/MODEL_LIMITS.md` — not my work):
live D8 flow routing with priority-flood, an eight-material derived palette with
hardness gating dig cost, `river_segments`, `catchment_mask`, and excavation
writing to the elevation grid. The build is clean and the selftest passes.

**But the headline coupling does not yet work, and I measured it rather than
assuming it.** Adding a proper metric — a hash of the whole downstream-pointer
field, plus a per-cell diff — gives:

```
live flow test   : 36 digs, elev 72.6 -> 55.7 m (Δ 16.9), strokes 36
routing rerouted : 0 cells changed downstream · signature IDENTICAL
depression fill  : lake cells 1529652 -> 1529652 · mean fill depth 179.0 m
```

Zero cells rerouted after cutting a 17 m trench. The cause is in the last line:
**1,529,652 of 1,572,864 cells — 97 % of the drum — are flagged as lake, and the
routing surface sits a mean 179 m above the real terrain.**

Priority-flood assumes an open domain with outlets on its boundary. **A sealed
cylinder has no boundary and no outlet**: theta wraps, and both endcaps rise to
maximum elevation. So the algorithm correctly concludes that the entire habitat
is one closed basin and floods it to the rim. Drainage is then meaningless, and
nothing the player digs can possibly matter.

The algorithm is right; the model is wrong. In a closed habitat there is no sea
level to drain to, so **depression fill must be replaced by a finite-volume lake
fill**: lakes rise only to the water actually available, from the habitat's
closed water budget. That is items 43–44 and §J together, and it is now a
blocker rather than a nicety. It moves to the front of wave 1.

---

## N. Mountains and rock form (201–270)

*We have ridged noise plus droplet erosion. That gives lumps. Mountains are
**structure exposed by removal** — and in a drum the structure is engineered,
which is a gift, not a limitation.*

201. Give bedrock **horizontal strata with real thickness variation**, so
     erosion exposes bands rather than a uniform grey.
202. Alternate hard and soft layers. Every stepped profile, bench and cliff line
     in nature comes from exactly this and nothing else.
203. Cap rock: a hard layer over soft produces mesas, buttes and hoodoos as the
     soft undercuts. Free landmark generation.
204. Differential weathering rate per material (§A item 6), so the shape follows
     the substrate rather than the noise.
205. **Joint sets** — two or three preferred fracture planes per rock type,
     driving cliff faces to be planar and roughly parallel. This is the single
     biggest thing separating "rock" from "blob".
206. Columnar jointing in basalt: hexagonal prisms at outcrops. Distinctive,
     cheap, and instantly reads as igneous.
207. Exfoliation domes in massive rock — onion-skin sheets, rounded summits.
208. Scarp-and-dip asymmetry: one side of a ridge steep, the other a gentle dip
     slope following the bedding. Asymmetric ridges read as geology.
209. Arêtes between adjacent valley heads, sharpened as both sides erode back.
210. Cirque-like headwall hollows where freeze-thaw concentrates near the cold
     endcaps (§F item 109) — the drum's substitute for glacial carving.
211. Hanging valleys where a tributary's floor sits above the main valley,
     producing a waterfall at the junction.
212. Knickpoints that retreat upstream through soft rock and stall on hard,
     giving river profiles their correct concave-with-steps shape.
213. Gorges and slot canyons where a river cuts fast through a resistant unit.
214. Talus cones at cliff feet, at the material's angle of repose, growing over
     play-time from rockfall events.
215. Rockfall as a discrete event with sound, dust and a permanent talus
     addition — mountains should occasionally *do* something.
216. Avalanche and debris chutes: recurring paths kept clear of vegetation,
     visible as pale stripes down slopes.
217. Boulder fields below cliffs, as scattered instanced meshes sized from the
     joint spacing of the parent rock.
218. Erratic boulders carried and dropped by past debris flows, as landmarks.
219. Badlands where soft, poorly-cemented material erodes into dense rills — a
     completely different texture at the same elevation.
220. Patterned ground near the cold ends: polygons and stripes from freeze-thaw
     sorting.
221. Weathering pits and tafoni on exposed faces, as small-scale decals.
222. Natural arches where two joint sets and undercutting meet.
223. Karst in soluble layers: sinkholes, dolines, disappearing streams. Ties
     caves to surface hydrology.
224. Springs at the base of permeable-over-impermeable contacts — a geologically
     *correct* reason for water to appear on a hillside (§C item 50).
225. **Snow line by temperature, not elevation** (§D item 75), so it tilts along
     the drum's axial gradient rather than sitting flat.
226. Aspect matters: slopes facing the axis strip differently to slopes facing
     away, changing snow persistence and vegetation.
227. Rock glacier / debris-covered ice near the cold ends, moving imperceptibly.
228. Alluvial fans at range fronts where confined flow spreads — the classic
     landform we are currently missing entirely.
229. Pediments: gentle bedrock ramps at mountain feet, thinly veneered.
230. Inselbergs — isolated resistant knobs standing above a worn plain.
231. Structural ribs (already in `terrain.rs`) should **outcrop visibly** at the
     surface as alloy ridges, tying the engineered fiction to the landform.
232. Spoil heaps from the original excavation, with the internal angle and
     bedding of dumped material, not of natural rock.
233. Quarry faces and benches left by the builders — rectilinear, weathered,
     unmistakably artificial.
234. Cut-and-fill terraces from construction roads, now overgrown.
235. Subsidence hollows over old workings, which the drainage then exploits.
236. Named peaks and ridges, generated from prominence, so the world has
     landmarks the colony talks about.
237. **Topographic prominence computed properly**, so naming picks real summits
     rather than noise maxima.
238. Ridgeline extraction for silhouette quality checks — at distance a mountain
     is its skyline and nothing else.
239. Skyline-aware LOD: never simplify vertices that lie on a silhouette edge.
240. Summit plateaus and cols as navigable features, so ridges have routes.
241. Passes that the flow router and any future pathfinding both agree on.
242. Scree slopes that the player slides on, with sound and a little loss of
     control.
243. Cliff bands that genuinely block movement, forcing route-finding.
244. Handholds and ledges as traversal affordances on otherwise impassable faces.
245. Rock colour varying with **weathering age**, not just type — freshly
     exposed faces are brighter than long-weathered ones.
246. Desert varnish or its analogue on long-stable surfaces: dark patina that
     records exposure time.
247. Lichen cover as a slow biological clock on stable rock (§S).
248. Moss on the wet side of boulders, keyed to the moisture field.
249. Frost shattering producing angular debris; chemical weathering producing
     rounded. Two visibly different debris textures from two processes.
250. Case hardening: a resistant surface crust over softer interior, so digging
     into a face is easier than expected.
251. Bedding-plane caves and shelters, usable as shelter and storage.
252. Overhangs that genuinely shelter ground beneath from rain — the wetness
     field should show a dry apron.
253. Mineral staining below seeps: iron reds, manganese blacks, calcite whites.
254. Tufa and travertine deposits at calcium-rich springs, building over time.
255. Salt or evaporite crusts where water pools and evaporates in dry bands.
256. Fumarole-analogue: warm vents from habitat machinery, with their own
     microclimate and vegetation.
257. Rockfall risk overlay for the player considering a building site.
258. Slope-stability overlay showing where excavation will trigger collapse.
259. A geologist's cross-section tool: draw a line, get the strata profile.
260. Strike-and-dip readout at any outcrop, for players who want to predict
     what is under the next hill.
261. Ore prospecting by surface indication — stained rock, indicator minerals —
     rather than an X-ray view.
262. Float: ore fragments in a stream bed that point upslope to the source.
263. Mountain weather divergence: the high ground is colder, wetter, windier,
     and the fields already support all three.
264. Orographic lift where banded wind meets high ground, producing a rainfall
     maximum on the windward flank.
265. Rain shadow behind it — the paired consequence (§I item 161).
266. Cloud caps sitting on summits when humidity and temperature align.
267. Katabatic drainage of cold air off high ground into valleys at night,
     producing frost hollows.
268. Temperature inversions in enclosed basins, trapping fog.
269. Mountains as the drum's **water towers**: they intercept, store as snow,
     and release. That is their job in the system, not merely their look.
270. Publish the limits: no tectonics, no glaciation, no true rock mechanics —
     these are process-shaped forms, not simulated geology.

## O. Rivers and running water (271–350)

*The single most legible connective tissue in a landscape, and currently a blue
tint on a vertex.*

271. Extract **channel centrelines** as polylines from the flow-accumulation
     grid above a discharge threshold.
272. Order them (Strahler or Shreve) so tributaries and trunks are distinguished
     and can be styled differently.
273. Build channel **cross-sections**: width from discharge, depth from width,
     bank angle from bank material.
274. Cut the channel into the terrain rather than laying water on top of it.
     Rivers should sit in valleys they made.
275. Meander migration: outer banks erode, inner banks deposit, the channel
     wanders across its floodplain over long time.
276. Point bars on the inside of bends, as visible sediment.
277. Cut banks on the outside — steep, raw, undercut.
278. Oxbow formation when a meander neck is cut through; the abandoned loop
     becomes a lake, then a marsh, then a scar.
279. Braided reaches where sediment load is high and banks are weak — a
     completely different channel pattern from the same rules.
280. Anabranching around stable islands.
281. Riffle–pool sequences at roughly five to seven channel widths, which is
     what makes a river look like a river up close.
282. Step–pool morphology in steep reaches.
283. Waterfalls at knickpoints (§N item 212) with a plunge pool below.
284. Cascades where the gradient is steep but continuous.
285. Rapids classified by gradient and roughness, with matching sound and foam.
286. Confluence geometry: the junction angle follows the discharge ratio.
287. Confluence mixing: two tributaries with different sediment loads should
     visibly run side by side before mixing.
288. Floodplain extraction, so the flood extent in §C item 54 has a map.
289. Natural levees built by overbank deposition, which then constrain the
     channel and set up the next avulsion.
290. Avulsion: the river abandons its channel for a lower path across the
     floodplain. Rare, dramatic, entirely emergent.
291. River terraces recording past base levels, as flat benches above the
     current channel.
292. Deltas prograding into lakes with distributary channels (§C item 56).
293. Ephemeral channels that only run after rain, dry and gravelly otherwise.
294. Rills and gullies on bare slopes, the smallest end of the same system.
295. Seeps and springlines as diffuse wet ground rather than point sources.
296. Groundwater-fed baseflow keeping perennial rivers running between storms.
297. Bank vegetation stabilising channels — clear it and the river widens.
298. Large woody debris from fallen trees creating pools and diverting flow.
299. Beaver-analogue fauna damming small streams, if §S gets that far.
300. **Water surface rendering**: a flow-aligned normal map scrolling along the
     channel direction, speed from local velocity. This alone sells motion.
301. Foam accumulating at obstacles, on the outside of bends, and below drops,
     driven by a foam field advected with the flow.
302. Depth-based colour: shallow water takes the bed colour, deep water goes to
     the water body colour.
303. Turbidity from suspended sediment, so a river in flood is genuinely brown.
304. Refraction of the bed through the surface, cheap screen-space.
305. Caustics on the bed, from a scrolling projected pattern, intensity by depth.
306. Shoreline wetness band that follows the actual water level as it changes.
307. Wet-rock darkening in the splash zone below waterfalls.
308. Spray and mist particles at falls and rapids, lit by the axis strip.
309. Rainbows in that spray when the geometry is right — ORRERY has
     `rainbowAt`; the condition here is a solved geometry problem, since the
     light source is a known line.
310. Ripples where flow passes over a submerged obstacle, generated from the
     bed heightfield rather than a texture.
311. Standing waves in rapids, positioned by bed geometry.
312. Eddies and recirculation behind obstacles, visible in the foam field.
313. Flow vectors visualisable as an overlay for players engineering their water.
314. **River audio by discharge and gradient** — the procedural noise generator
     already exists; filter and layer it per reach.
315. Distinct sound for riffle, pool, cascade and fall, cross-faded by proximity.
316. Water temperature affecting where ice forms and what lives there.
317. Ice shelves at the banks in cold bands; a frozen surface with flow beneath.
318. Break-up in the melt season, with ice floes.
319. Aufeis where a spring freezes progressively in place.
320. Fords: shallow crossings the player can find, use and improve.
321. Bridges as buildable modules whose span is limited by channel width.
322. Weirs and check-dams that raise upstream level and reduce erosion.
323. Dams with a real reservoir volume, spillway, and siltation lifetime.
324. Dam failure as a consequence, not a random event: overtopping when inflow
     exceeds spillway capacity.
325. Canals and leats the player digs, which the flow router honours because it
     only knows about the surface (§C item 61).
326. Aqueducts crossing valleys, as modules with a maintained gradient.
327. Penstocks and micro-hydro generating power from head and discharge, feeding
     the energy budget in §J.
328. Water wheels and mills as an early-game alternative to reactor power.
329. Irrigation offtakes with a share of flow, contested between regions.
330. Water rights as a colony-level political mechanic, since supply is finite
     and shared.
331. Gauging stations the player builds, producing a discharge time series.
332. A hydrograph readout per station: baseflow, storm peaks, recession.
333. Flood recurrence estimated from the record, so the player can site
     buildings on evidence.
334. Water quality: turbidity, nutrient load, contamination from tailings.
335. Eutrophication when nutrient-rich runoff hits a slow lake — algal bloom,
     oxygen crash, fish die-off. A visible consequence of over-fertilising.
336. Self-purification along a reach, so distance from the source matters.
337. Aquatic vegetation zoned by depth and flow.
338. Fish and aquatic fauna with habitat requirements tied to those zones (§S).
339. Migration barriers at weirs and falls, and fish passes as a module.
340. Riparian corridor as its own biome (§G item 127), narrow and distinctive.
341. Wetland formation where a channel loses gradient.
342. Marsh gas and peat accumulation in permanently waterlogged ground.
343. River as a **transport route** — floating logs downstream is free energy.
344. Erosion of the player's own structures by the river they diverted.
345. Sediment budget per reach: supply, transport capacity, deposition. Closing
     it is what stops rivers from silently creating or destroying mass.
346. Bedload versus suspended load, moving at different rates and depositing
     in different places.
347. Armouring of the bed where fines are winnowed out, leaving cobbles.
348. Scour and fill cycles at bends through the flood season.
349. A **catchment inspector** that walks the network upstream from any point,
     listing everything that drains to it.
350. Publish the limits: no Manning's equation, no true shallow-water solver,
     daily timestep, channels as polylines rather than a solved free surface.

## P. Rain, storms and wetness (351–410)

*Rain in a drum falls from a machine, lands sideways, and is the only reason
anything is green. It should be the most consequential weather in any game.*

351. Precipitation type as a function of air temperature at the cloud radius and
     at the ground: drizzle, rain, sleet, snow, hail. ORRERY's `precipTypeAt`
     is the shape to copy.
352. Intensity as a continuous field, not a boolean "raining".
353. Duration and onset ramps, so storms build and fade instead of snapping on.
354. **Rain falls at an angle set by Coriolis and wind**, not straight down
     (§E). Compute the drift with the throw integrator we already have —
     roughly 1.1 m/s of sideways terminal drift over a 250 m fall.
355. Therefore rain **lands offset from the cloud that made it**, which is the
     mechanic that makes condenser siting a skill.
356. Rain particles rendered as camera-facing stretched quads aligned to the
     *actual* drift vector, so what you see matches where it lands.
357. Near-field rain as a screen-space effect; far-field as volumetric-ish
     curtains, so a storm reads across the drum.
358. Rain curtains visible as translucent sheets under distant cloud bands — the
     single most atmospheric thing this setting can show.
359. Rain streaks on the camera lens when the player looks up into it, fading.
360. Splash particles on hard surfaces; none on soft or vegetated ground.
361. Splash rings on water surfaces, density from intensity.
362. Drip from vegetation continuing after the rain stops — a canopy holds water.
363. Stemflow and throughfall: a forest floor gets less rain than open ground,
     and it arrives late. This matters to §B's moisture field.
364. **Wetness as a real field** with an accumulate-and-dry curve, not a global
     "is raining" multiplier on albedo.
365. Wetness darkens albedo, raises specular, and lowers roughness — three
     values, one field, and the world visibly responds within a minute.
366. Different materials wet and dry at different rates: sand goes dark fast and
     dries fast; clay stays dark for hours.
367. Puddles forming in local depressions, computed from the terrain's own
     small-scale concavity rather than from decals.
368. Puddle level rising and falling with the balance of rain and evaporation.
369. Puddles freezing when soil temperature drops, becoming ice.
370. Runoff rivulets on slopes during heavy rain, appearing along the same flow
     directions the router already knows.
371. Sheet flow on saturated ground when rainfall exceeds infiltration capacity —
     the mechanism behind flash flooding.
372. Infiltration capacity from soil texture and compaction (§B item 27), so a
     compacted path floods while the field beside it does not.
373. Mud: saturated bare soil becoming a distinct surface with its own colour,
     sound and movement penalty.
374. Footprints and wheel ruts in mud, persisting until it dries.
375. Erosion spikes during storms (§F item 102), so the landscape visibly changes
     after a big one.
376. Landslides triggered by saturation on steep slopes — rare, dramatic,
     predictable from a slope-stability overlay.
377. Storm structure organised into **axial bands** by Coriolis (§E item 89), so
     a storm is a long ribbon travelling around the drum, not a blob.
378. Storms recur on a schedule because the condensers do — the player can
     forecast them exactly (§D item 80).
379. Convective cells where a heated region drives rising air into the cloud
     band.
380. Lightning is genuinely possible here if convection separates charge — and
     in a sealed habitat it is an *emergency*, not a mood effect.
381. Thunder delay from distance, computed properly, because the geometry of a
     closed cylinder makes it audible all the way round.
382. Echo of thunder around the drum: sound wraps. Nothing else sounds like this.
383. Hail damaging crops, with size from updraft strength.
384. Wind gusts as a modulation over the banded mean flow.
385. Wind visible in vegetation sway (ORRERY's `windSway`), grass bending,
     dust lifting, and water surface roughening — one field, four readouts.
386. Wind-driven rain hitting one side of structures; the leeward side stays dry.
387. Visibility reduction in heavy rain and fog, on both the camera and the
     player's ability to see the far side of the drum.
388. Fog forming in cold hollows at night (§N item 267) and burning off with the
     light schedule.
389. Radiation fog, advection fog and valley fog as distinct behaviours from the
     same humidity and temperature fields.
390. Dew and frost deposition overnight on exposed surfaces — ORRERY's
     `frostDewAt`.
391. Frost visible as a pale rime on vegetation and rock, sublimating at dawn.
392. Snow accumulating as a depth field, drifting with wind against obstacles.
393. Snow compaction and settling over days.
394. Melt driven by temperature and light, feeding the spring discharge pulse
     (§C item 60).
395. Snow albedo feedback: snow-covered ground reflects the axis light and stays
     colder, which is a real feedback with the correct sign.
396. Tracks in snow, persisting and gradually filling.
397. Icicles at overhangs and seeps in cold bands.
398. Rainbow geometry solved analytically: the light source is a known line, so
     the bow is a computable locus rather than a billboard.
399. Halos and light pillars in ice fog around the axis strip.
400. Petrichor as an audio-visual cue — the sound changes before the rain lands.
401. A rain gauge module producing a local record.
402. A weather station aggregating temperature, humidity, wind and rainfall.
403. Regional weather history queryable (§D item 83), so failures are explicable.
404. Crop damage thresholds per species for hail, frost, and waterlogging.
405. Shelter belts and windbreaks as buildable modules with measurable effect on
     the local wind field.
406. Cloches and polytunnels raising local temperature and holding moisture.
407. Drought as a slow, legible emergency: humidity falls, soil dries, crops
     stress, and the player can watch every step of it.
408. Condenser failure producing a regional drought (§D item 82) — a machine
     fault, not bad luck.
409. Water rationing as a colony decision with visible consequences.
410. Publish the limits: no cloud microphysics, no radiative transfer, single
     cloud radius, precipitation as a field rather than tracked droplets.

## Q. Forests and stands (411–490)

*Forests are the most expensive thing on screen and the most valuable thing in
the system. Both facts deserve engineering.*

411. Trees as **individuals at T0** with the same carbon-allocation state as
     crops (§H item 137) — a tree is a plant that lived a long time.
412. Diameter at breast height derived from stem biomass, and everything else
     scaled from it. One number drives the whole allometry.
413. Height from DBH via a species curve that saturates, so old trees get thick
     rather than infinitely tall.
414. Crown radius from DBH, which sets the shading footprint.
415. **Light competition through a canopy height-and-cover raster**: each tree
     reads the light left after everything taller within reach has taken its
     share.
416. Suppressed trees in the understory growing slowly, waiting.
417. **Gap dynamics**: when a canopy tree dies, the light pulse releases the
     suppressed ones beneath. This is the engine of forest structure and it
     costs almost nothing once item 415 exists.
418. Self-thinning as stands close, following the −3/2 power law that falls out
     of competition rather than being imposed.
419. Even-aged stands after disturbance; uneven-aged in old growth. Both emerge.
420. Canopy layers — emergent, canopy, subcanopy, shrub, ground — as a readout
     of the height distribution, not as authored tiers.
421. Species mix by moisture, temperature, soil depth and pH (§G item 119).
422. Shade tolerance as a species trait, which decides who can regenerate under
     whom. The main axis of forest succession.
423. Pioneer species colonising bare ground fast and dying young.
424. Late-successional species growing slowly in shade and living long.
425. Nurse effects: pioneers shelter the seedlings that will replace them.
426. **Root systems competing for water and nutrients** in the soil grid, not
     just canopies competing for light.
427. Rooting depth by species, so drought sorts the community.
428. Mycorrhizal networks linking trees, moving carbon and nutrients between
     them — a genuinely modern piece of forest ecology and a lovely mechanic.
429. Nitrogen-fixing trees enriching their neighbourhood (§B item 29).
430. Allelopathy: some species chemically suppress competitors beneath them.
431. Litterfall by species, with different decomposition rates — needle litter
     acidifies, broadleaf litter enriches (§B item 147).
432. A litter layer with its own depth, moisture and decomposition state.
433. Deadwood: standing snags and fallen logs as long-lived habitat and fuel.
434. Decay stages for deadwood, visible as colour, moss and collapse.
435. **Windthrow** in storms, weighted by exposure, rooting depth and saturation —
     saturated soil plus wind is what actually flattens forests.
436. Windthrow leaving root plates and pit-and-mound microtopography that
     persists in the terrain for decades.
437. Edge effects: forest margins are windier, drier and brighter, with a
     distinct species mix.
438. Fragmentation consequences as the player clears — edge fraction rises.
439. **Fire.** In a sealed habitat this is not a wildfire mechanic, it is an
     oxygen-budget emergency and a smoke emergency (§J).
440. Fuel load accumulating from litter and deadwood, drying in drought.
441. Fire spread along the fuel field, driven by the banded wind.
442. Fire suppression as an actual colony capability with a cost.
443. Post-fire succession, with fire-adapted species and a nutrient pulse from
     ash.
444. Charcoal in the soil recording past fires (§B, and the chronicle in §W).
445. Smoke as a real atmospheric constituent, degrading visibility and air
     quality until scrubbed.
446. Coppicing and pollarding as sustainable harvest, with regrowth from the
     stool.
447. Timber as a material with grain, strength and species — construction should
     care which tree it came from.
448. Firewood, charcoal and biochar as energy and soil products.
449. Seasoning: freshly cut timber is wet, heavy and weak.
450. Tree rings recording growth conditions — a readable climate archive the
     player can core.
451. Forest inventory tooling: stand basal area, stocking density, mean DBH.
452. Sustainable yield calculation, so the player can log without collapse.
453. Clear-felling consequences flowing straight into §I's downstream coupling —
     flood peaks up, silt load up, soil lost.
454. Selective felling as the alternative, slower and gentler.
455. Replanting with chosen species and spacing.
456. Natural regeneration where seed sources and light allow.
457. Seed rain modelled from parent trees, dispersing along the wind bands
     (§H item 148).
458. Seed banks in the soil, waiting for a light gap.
459. Browsing pressure from fauna suppressing regeneration (§S).
460. Exclosures as a buildable answer to that.
461. Pests and pathogens spreading through connected stands, favoured by
     monoculture (§H item 153).
462. **Rendering: three-tier tree LOD** — full mesh near, cross-billboard mid,
     octahedral impostor far — and the transition distance tuned against the
     tilt-shift blur, which conveniently hides it.
463. Impostor atlases baked at build time per species and size class.
464. GPU instancing via `MultiMesh` for everything past the near tier, with
     per-instance colour and scale variation.
465. A single indirect draw per species per chunk, not per tree.
466. Canopy rendered as a small number of faceted planes rather than leaf cards,
     to stay inside the low-poly language rather than fighting it.
467. Foliage colour varying per instance from a hue jitter seeded by position,
     so a stand does not look like one tree copied.
468. Species silhouette as the primary read at distance — conical, spreading,
     columnar — because at 400 m the shape is all you get.
469. Wind sway as a vertex shader driven by the wind field, amplitude scaled by
     height and flexibility. ORRERY's `windSway` is the reference.
470. Sway phase offset per instance so the stand does not pulse in unison.
471. Gusts propagating across the canopy as a travelling wave, which is what
     makes wind legible from inside a forest.
472. Contact shadows and ambient occlusion under canopies, so trees sit on the
     ground rather than hovering.
473. Light shafts through canopy gaps — in this habitat they come from the axis,
     so they are near-vertical and converge, which no Earth-set game shows.
474. Dappled ground light from a projected canopy pattern, animated with sway.
475. Subsurface scattering approximation on leaves for backlit translucency.
476. Seasonal colour driven by the phenology state (§R), not a date.
477. Leaf fall as particles when senescence completes, accumulating into the
     litter layer.
478. Bare-branch winter silhouettes for deciduous species.
479. Snow load on branches, bending them, shed in wind.
480. Bark texture and colour by species and age, via triplanar detail.
481. Moss and epiphytes on the wet side of trunks, keyed to the moisture field.
482. Forest floor scatter — logs, stones, ferns, litter — instanced from the
     litter and biomass fields so density is meaningful.
483. Forest interior audio: wind in canopy layered by density, creaks, drips.
484. Birdsong density as a readout of forest health (§S).
485. Reduced visibility inside dense stands, both for the player and for fauna.
486. Navigation difficulty in dense understory — forests should slow you.
487. Trails forming where the player repeatedly walks, compacting soil and
     clearing understory.
488. Forest microclimate: cooler, more humid, less windy — measurable in the
     fields, not faked.
489. Carbon storage per stand feeding the habitat carbon budget (§J).
490. Publish the limits: no individual leaves, no true radiative transfer through
     canopy, allometry from fitted curves rather than mechanistic growth.

## R. Growth, lifecycle and season (491–550)

*Growth is the clock that makes a place feel inhabited over time.*

491. **Phenology driven by accumulated degree-days**, not by a calendar date —
     so a warm band leafs out before a cold one.
492. Chilling requirement before bud break, so a plant that never gets cold
     never flowers properly. A real constraint in an engineered climate.
493. Photoperiod response to the light schedule, which the colony controls (§D
     item 72) — changing day length changes what flowers.
494. Germination gated by soil temperature, moisture and light.
495. Dormancy as an explicit state with its own exit conditions.
496. Vernalisation for species that need it.
497. Bolting when stress or day length triggers premature flowering.
498. Flowering as a visible stage with its own geometry and colour.
499. Pollination requiring wind or insects (§S), and failing without them.
500. Fruit set proportional to successful pollination.
501. Ripening as a continuous state visible in colour, not a binary "harvestable".
502. Over-ripeness and rot if left, so timing matters.
503. Senescence and autumn colour driven by the same accumulated state.
504. Abscission dropping leaves and fruit into the litter layer.
505. Perennial storage organs banking carbon for next season's start.
506. Biennials with a two-season cycle the player must plan around.
507. **Growth visible in geometry**, continuously, from the L-system parameters
     (§H item 143) — plants should be seen to grow, not to pop between stages.
508. Growth animation interpolated on the presentation layer, so the sim can
     tick slowly while the visuals stay smooth.
509. Damage recorded in morphology: a browsed shoot regrows crooked.
510. Pruning as a player action that redirects allocation.
511. Training and trellising as modules that change light interception.
512. Grafting to combine rootstock and scion traits.
513. Vegetative propagation from cuttings, cloning a strain exactly.
514. Tissue culture as a late-game way to bank and restore genetics.
515. A seed vault as colony infrastructure — insurance against a bad season.
516. Seed viability declining with storage age and conditions.
517. Crop rotation with real mechanical benefit through soil nitrogen and pest
     cycles (§B item 29, §H item 153).
518. Cover crops and green manure to protect and enrich between plantings.
519. Intercropping with complementary root depths and light requirements.
520. Companion planting effects that are real (shade, nitrogen, pest confusion)
     rather than folklore.
521. Yield as an integral of the growing season, not a lookup — a plant that had
     a bad month shows it.
522. Harvest index varying with stress, so poor conditions shift allocation away
     from the part you wanted.
523. Quality as well as quantity: sugar, protein, fibre from the allocation
     model.
524. Storage and spoilage after harvest, by temperature and humidity.
525. Root cellars and cold stores as modules exploiting the cold endcaps.
526. Fermentation, drying and preserving as processing chains.
527. Nutritional composition feeding a colonist requirement model.
528. Livestock feed requirements linking crops to fauna (§S).
529. Growth stages readable at a glance by silhouette, for field-scale
     assessment.
530. A crop inspector showing the plant's full state and its limiting factor —
     "nitrogen-limited since day 34", which is item 197 applied to a single
     plant.
531. Field-scale aggregate view: mean stage, mean stress, projected yield.
532. Yield forecasting from current state, improving in accuracy as harvest
     nears.
533. A season-in-review summary the player can read afterwards.
534. Multi-year records so long-term soil trends become visible.
535. **Time-lapse view** of a plot over a season — cheap to implement from a
     state history, and enormously satisfying.
536. Growth continuing while away, via the T1 statistical tier, with promotion
     reproducing a plausible current state (§H item 154).
537. The wilderness growing too — abandoned ground should visibly change between
     visits.
538. Succession clocks running everywhere, not only where the player looks (§G
     item 134).
539. Weeds colonising disturbed ground, with real competitive effect.
540. Weeding as maintenance, and abandonment as reversion.
541. Volunteers from last season's dropped seed.
542. Invasive species spreading along wind bands and river corridors — the two
     transport systems the world already has.
543. Species extinction being permanent in a closed system (§S).
544. A living seed and genetics ledger for the whole habitat.
545. Breeding programme UI showing lineage, traits and selection history.
546. Trait heritability and variance so breeding is a real statistical process.
547. Inbreeding depression in small populations, which is the crop-scale rhyme
     of the Crèche's own problem.
548. Mutation rate raised by cosmic radiation (§H item 149), with a shielding
     mechanic that trades safety against variation.
549. The deliberate choice to expose a strain to radiation for variation — the
     game's premise handed to the player as a tool.
550. Publish the limits: phenology from degree-day models, allocation from a
     toy source–sink scheme, no photosynthesis biochemistry.

## S. Fauna and the living world (551–610)

*A landscape without animals is a diagram. And in a closed drum, every animal
is part of the mass budget.*

551. Agent-based fauna at T0, population fields at T1, exactly as
     `SIM_ARCH_BRIEF.md` §3.5 specifies. Never simulate individuals you cannot see.
552. Needs-driven behaviour: hunger, thirst, safety, reproduction — four drives
     produce most of what looks like intelligence.
553. Foraging against the **real biomass field**, so grazing measurably reduces
     vegetation where the herd actually stood.
554. Overgrazing degrading soil and triggering erosion (§F item 104).
555. Rotational grazing as the player's answer, with visible recovery.
556. Herbivore population dynamics coupled to plant biomass, with the lag that
     produces boom and crash.
557. Predators coupled to herbivores, with the classic phase-lagged cycle
     emerging rather than being scripted.
558. Predation pressure changing herbivore *behaviour*, not just numbers —
     landscape of fear, altered grazing distribution.
559. Territories and home ranges, so animals belong somewhere.
560. Herding and flocking with local rules; the drum's geometry makes flocks
     wrap, which is worth seeing.
561. Migration along the wind bands and river corridors — the drum's two
     transport axes.
562. Seasonal movement between the warm middle and the cold ends.
563. **Extinction is permanent** (`REQUIREMENTS.md` B7). No migration from
     elsewhere; there is no elsewhere.
564. Minimum viable population thresholds, below which a species is doomed even
     if individuals remain.
565. A species register showing every population, its trend, and its risk.
566. Decomposers as a functional guild converting carcasses and litter into soil
     organic matter — the loop that closes §J's carbon budget.
567. Carrion attracting scavengers, briefly and visibly.
568. **Pollinators as a real service**: crops that need them fail without them,
     which makes hedgerows and wildflower margins mechanically necessary.
569. Pollinator populations sensitive to pesticide, monoculture and habitat loss.
570. Soil fauna — worms and arthropods — improving structure and reducing
     compaction (§B item 27).
571. Aquatic food webs in rivers and lakes, zoned by depth and flow (§O item 337).
572. Fish requiring migration routes past weirs (§O item 339).
573. Birds as the most visible fauna: perching, foraging, mobbing, and audible
     from far further than they are visible.
574. Birdsong density and diversity as a legible index of ecosystem health.
575. Dawn chorus timed to the light schedule — which the colony sets, so the
     birds wake when engineering says so.
576. Insects as ambient life: swarms over water at dusk, dust motes that turn
     out to be alive.
577. Pest species with population dynamics, favoured by monoculture.
578. Biological control by encouraging a pest's predator, as an alternative to
     intervention.
579. Disease with real transmission through contact networks and density.
580. Zoonosis risk in a closed habitat, which is a genuinely frightening idea
     here and belongs in the fiction.
581. Domestication as a long-horizon project with heritable traits.
582. Livestock with feed requirements that compete with human food (§R item 528).
583. Manure returning nutrients, closing a loop that is otherwise open.
584. Draught animals as an energy source that does not draw on the reactor.
585. Animal tracks and sign the player can read — footprints, droppings,
     browse lines, wallows.
586. Trails worn by repeated animal movement, which the player can follow.
587. Nests, burrows and dens as persistent world features.
588. Burrowing fauna genuinely modifying the density field, digging small
     tunnels — the same CSG system the player uses.
589. Hunting with real population consequences, tracked against sustainable
     yield.
590. Hunting pressure making animals warier and more distant.
591. Animal fright and flight responses to the player, distance-scaled.
592. Fauna avoiding recently disturbed ground, so construction has an ecological
     footprint.
593. Habitat suitability per species from the field state, deciding where
     populations can persist at all.
594. Corridors and connectivity: fragmenting a habitat isolates populations.
595. Keystone species whose removal restructures a whole region.
596. Trophic cascades as an emergent consequence — remove the predator, lose the
     riparian vegetation, lose the riverbank.
597. Introduced species spreading if conditions suit, with no way to recall them.
598. **Rendering: fauna as low-poly articulated rigs** in the same faceted
     language as the colonist, with procedural gaits driven by distance
     travelled (as the player's already is).
599. Herd rendering via `MultiMesh` with per-instance animation phase.
600. Silhouette-first design so species are identifiable at range.
601. Behaviour visible at a distance — a herd running means something happened.
602. Animal audio positioned and occluded, with distance-appropriate content.
603. Faunal density feeding the ambient soundscape automatically.
604. A field guide that fills in as the player observes species.
605. Observation records with location and date, building a personal dataset.
606. Population graphs over time in the survey view.
607. An ecosystem web diagram showing who eats whom, built from actual
     parameters rather than authored art.
608. Alerts when a population crosses a risk threshold.
609. The colony's own dependence on specific species made explicit, so an
     extinction is a *colony* problem and not a nature-documentary sad moment.
610. Publish the limits: no individual metabolism, no genetics for fauna, T1
     populations as fields, behaviour from four drives.

## T. Graphics — the material language (611–680)

*The look is faceted, flat-shaded, tilt-shifted low-poly. The goal is not to
escape that language but to make it carry real information.*

611. **Colour from a compiled table, not from code.** Follow ORRERY's
     `paintEval` exactly: a data file of ramps with conditions, loaded and
     compiled, replacing the if-else chain currently in `color_at()`.
612. Table conditions keyed on the real fields — material, moisture, temperature,
     biomass, wear, depth — so the look is a function of the simulation by
     construction.
613. Hot-reload the paint table, so art iteration does not need a Rust rebuild.
614. Vertex colour by **material** rather than by elevation and slope (§L item
     191), so strata read in every cut face.
615. A per-material triplanar detail overlay — grain, bedding, blockiness —
     applied subtly enough to stay inside the low-poly language.
616. Detail scale that holds up under the tilt-shift blur, which is doing a lot
     of work to hide high-frequency noise.
617. Material blend at contacts driven by a noise-perturbed threshold, so strata
     boundaries are ragged rather than ruled.
618. Wetness darkening from the wetness field (§P item 365).
619. Snow cover from the snow depth field, accumulating on upward faces only —
     the facet normal already tells us which those are.
620. Dust accumulation on flat surfaces in dry bands, cleared by rain.
621. Wear from traffic: ORRERY's `wearAt` applied to paths and thresholds.
622. Vertex ambient occlusion baked per chunk at mesh time, which is cheap on a
     surface-nets mesh and does most of the work of grounding geometry.
623. Curvature-based edge highlighting: convex edges catch light, concave ones
     darken. This is what makes faceted geometry read as *carved*.
624. Slope-based colour shift so cliffs and flats differ without extra data.
625. Aspect-based colour shift relative to the axis strip.
626. Height-based tinting used sparingly and only where a process justifies it.
627. Per-face colour jitter seeded by face index, breaking up flat expanses
     without noise textures.
628. A **restricted palette** documented as a spec, so the world holds together
     rather than accumulating colours per system.
629. A **contrast audit as a CI gate**, following ORRERY's `contrast-audit.md` —
     every UI and legend colour pair checked against a minimum ratio.
630. Colour-blind-safe palettes for every overlay, selectable in the menu that
     now exists.
631. A reduced-motion option covering sway, particles and camera easing —
     ORRERY has `reducedMotion` and we should too.
632. Decals for local detail — scorch, stain, spill, blood, paint — projected
     onto the surface-nets mesh.
633. Player-placed markers and paint as diegetic annotation.
634. Isolines over any field (ORRERY's `isoline.js`): contour the terrain, the
     moisture, the temperature. The cheapest legibility win in the document.
635. Contour rendering directly in the terrain shader from the elevation field,
     toggled per overlay.
636. Overlay mode as a shader branch on the existing terrain material, so every
     field is viewable in the world rather than only on a map.
637. A **clickable legend** that highlights matching ground, following
     `cellMatchesLegend`. The legend becomes a query.
638. Highlight rendering as an additive tint that survives the tilt-shift.
639. Selection outlines on modules and entities, drawn as a depth-aware edge.
640. A blueprint mode for construction: wireframe, grid-snapped, with clearances.
641. Ghost previews for every placeable, not just modules (already done for one).
642. Brush shape preview matching the actual CSG primitive (already done — apply
     the same rule everywhere).
643. Excavation preview showing the volume and material that will be removed.
644. Material yield preview before committing to a dig.
645. Damage and wear on structures, visible before failure.
646. Construction stages visible during building, not a pop-in.
647. Vegetation colour from the actual biomass and stress fields, so a stressed
     field looks stressed.
648. Crop rows aligned to the terrain and to the local frame, which on a cylinder
     means aligned to the axis or the circumference — a distinctive look.
649. Field boundaries and hedgerows as instanced geometry following plot edges.
650. Path and trail rendering from the wear field rather than as authored splines.
651. Water colour from turbidity, depth and temperature (§O items 302–303).
652. Ice rendering distinct from water — flatter, brighter, with cracks.
653. Mud as a distinct surface with its own shading response.
654. Sand and scree with a subtle sparkle at grazing angles.
655. Ore visible in rock faces as coloured inclusions, so prospecting is visual.
656. Strata visible in cave walls, which is the payoff for §A item 14.
657. Cave lighting: the player's lamp, bioluminescence, and light shafts from
     surface openings.
658. Volumetric dust in lamp beams underground.
659. Darkness that is genuinely dark, with a limited-radius light source — caves
     should be frightening.
660. Bioluminescent fungi and organisms as a distinctive cave ecology (§S).
661. Emissive materials for machinery, windows and lamps, integrated with the
     day cycle so the colony lights up at dusk.
662. Window light spilling onto the ground near buildings.
663. Lamp posts as placeable modules with real light radius and power draw.
664. Distant settlement lights visible across the drum at night — one of the most
     evocative things this geometry offers.
665. Smoke and steam from machinery, advected by the wind field.
666. Heat shimmer over hot surfaces.
667. Particle systems driven by simulation state, never spawned arbitrarily.
668. A unified particle budget so effects cannot collectively tank the frame.
669. Screen-space effects kept minimal, since tilt-shift is already the dominant
     post pass.
670. Tilt-shift focus band **following the aim point** rather than fixed at
     screen centre, so what you are working on is sharp.
671. Focus band widening when the camera pulls back, so overview shots stay
     readable.
672. Tilt-shift disabled entirely in the survey and plan views (already done) —
     instruments are not photographs.
673. Depth-of-field quality scaling with distance so distant terrain is cheap.
674. Colour grading per biome, subtle, driven by the same fields.
675. Exposure adaptation as the player moves between bright surface and dark
     cave.
676. Bloom restricted to genuine emitters — the axis strip, lamps, fire.
677. A photo mode with free camera, focus control and a grid, because players
     will want to show this off.
678. Screenshot metadata embedding the seed, position and date, so an image is
     reproducible.
679. A style guide document specifying the palette, facet density, silhouette
     rules and detail budget.
680. Golden-image tests in CI for a fixed set of camera positions, so a shader
     change that breaks the look fails the build.

## U. Graphics — light, sky and water in a cylinder (681–730)

*Almost every standard technique assumes a sky above and a sun in it. We have
neither, and the substitutes are more interesting than the originals.*

681. **There is no skybox and there never will be.** Looking up means looking at
     inhabited ground. Every technique that assumes a sky needs replacing, and
     saying so up front prevents a lot of wasted work.
682. The light source is a **line**, not a point or a direction. Irradiance falls
     off with perpendicular distance from the axis, not with distance squared.
683. Analytic line-light shading: for a segment of the strip, the integral has a
     closed form. Use it rather than sampling.
684. Soft shadows from a line source are elongated along the axis — a
     characteristic look nothing else has.
685. Standard cascaded shadow maps assume a directional light; they do not apply.
     Consider instead a **cylindrical shadow map** parameterised in (θ, z).
686. Or screen-space contact shadows plus a coarse baked occlusion term, which
     may be enough given the faceted style.
687. Terrain self-shadowing computed in the Rust core as a horizon-angle field
     per cell toward the axis — cheap, static per topology, invalidated by edits.
688. That horizon field also gives **direct light hours per cell**, which §H's
     photosynthesis needs anyway. One computation, two consumers.
689. **Bounce light from the far side is a real and dominant term**, not a fudge.
     Half the sky is sunlit farmland 1800 m away, which is why the ambient is
     green. Model it as a hemispherical term tinted by what is actually up there.
690. Sample that bounce from a low-resolution render of the far side, updated
     rarely — a genuine ambient probe that costs almost nothing.
691. Bounce colour changing as the far side changes: burn a forest and the light
     on your face changes. That is a *spectacular* consequence.
692. Volumetric light along the axis strip, strongest in humid air, giving the
     drum its characteristic haze.
693. God rays from the strip through cloud gaps, converging rather than
     diverging — the opposite of every sunbeam the player has seen.
694. Aerial perspective tuned for a 1800 m maximum sight line, which is short
     enough that the haze curve matters a lot.
695. Haze density from the actual humidity and dust fields, not a constant.
696. Distance fog colour from the bounce term, so the far side and the haze agree.
697. Night lighting from the dimmed strip plus settlement lights, never from a
     moon, because there is not one.
698. The strip's dimming schedule visible as a real change in shadow softness and
     colour temperature.
699. Dawn and dusk as ramps in the schedule, with the colour shift already
     implemented, extended to shadow colour.
700. **Water rendering**: screen-space reflections work, but the thing reflected
     is the far side of the drum, which looks extraordinary and should be
     showcased.
701. Planar reflections for large flat lakes, with the reflection plane being a
     cylinder section rather than a plane — a real technical wrinkle worth
     solving once.
702. Refraction and depth-based absorption (§O items 302–304).
703. Caustics projected onto the bed (§O item 305).
704. Shoreline foam driven by the actual water level and terrain gradient.
705. Wave height from wind fetch — and fetch in a drum is bounded by the
     endcaps, so waves are always small. State that rather than adding swell.
706. Flow-aligned surface normals on rivers (§O item 300).
707. Surface tension and meniscus at small scales, for puddles and streams.
708. Ice as a distinct surface with subsurface scattering and internal cracks.
709. Underwater rendering with depth-based colour extinction and reduced
     visibility.
710. Underwater caustics and light shafts, from the axis, which is directly
     above every water body.
711. Wet–dry line rendering that tracks changing water levels precisely.
712. Reflection probes placed per region, updated on a schedule.
713. Probe blending across region boundaries.
714. Irradiance volumes for interiors and caves.
715. Emissive strip geometry with correct bloom falloff and no aliasing at
     distance — the current beam aliases badly at some angles.
716. Lens flare only from the strip, and only when looked at near-directly.
717. Atmospheric scattering along the axis producing a visible glow column in
     humid conditions.
718. Cloud rendering as **banded stripes** (§E item 92), lit from below by the
     strip and above by nothing.
719. Cloud shadows on the ground below, moving with the band — a strong cue for
     the banded climate.
720. Cloud density from the humidity field, so the sky reports the simulation.
721. Precipitation curtains under raining bands (§P item 358).
722. Cloud base at the condensation radius, computed rather than fixed.
723. Fog rendering as a height-fog analogue in cylindrical coordinates, which is
     a radius-fog.
724. Ground mist in hollows, following the terrain concavity.
725. Smoke and dust integrated into the same volumetric pass.
726. A single unified volumetric system for haze, fog, cloud, smoke and dust,
     rather than five effects that disagree.
727. Frustum culling that handles the wrapped world correctly — a chunk at θ=0
     and one at θ=2π are neighbours.
728. Reflection and occlusion queries that respect the wrap.
729. A far-field render for the opposite wall that is genuinely cheap, since it
     is always visible and always distant.
730. Publish the limits: no path tracing, no true volumetric scattering
     integration, bounce as a single hemispherical term.

## V. Graphics — performance, LOD and streaming (731–770)

731. Four terrain LOD tiers by distance, with the transition distances tuned so
     the tilt-shift blur hides every pop.
732. Mesh simplification per tier that **preserves silhouette vertices**
     (§N item 239) and can discard interior detail freely.
733. Transvoxel or a skirt at LOD boundaries to close cracks between tiers.
734. LOD selection per chunk on the shared global lattice, so neighbouring
     chunks at different tiers still agree at their boundary.
735. Asynchronous chunk meshing on worker threads — never on the main thread
     (`SIM_ARCH_BRIEF.md` §5.3). The current implementation still blocks.
736. A meshing job queue with a strict per-frame time budget.
737. Priority by distance and by whether the chunk is in the frustum.
738. Chunk mesh caching so returning to a region does not re-mesh it.
739. Chunk unloading with hysteresis to prevent thrash at the boundary.
740. Memory budget per tier, enforced rather than hoped for.
741. Vertex format compression — quantised positions, octahedral normals, packed
     colour — cutting bandwidth several-fold.
742. Index buffer optimisation for vertex cache locality.
743. Occlusion culling using the terrain itself, which in a valley occludes a
     great deal.
744. Hierarchical Z occlusion for vegetation, which is the densest geometry.
745. GPU-driven culling for instanced foliage.
746. Indirect drawing so instance counts are decided on the GPU.
747. Impostor generation at build time rather than at runtime (§Q item 463).
748. Foliage density falling off with distance in a way that preserves the
     *impression* of density.
749. A separate, much simpler shader for the far-field terrain.
750. Shadow-caster culling, since most geometry never casts a visible shadow.
751. Dynamic resolution scaling under load.
752. A frame-time budget document assigning milliseconds per system, and a
     profiler overlay that shows the actual split against it.
753. The sim's own budget kept to 2–3 ms amortised (`REQUIREMENTS.md` E3).
754. Sim tiers ticking at different rates, with the work spread across frames
     rather than bunched.
755. Rust-side parallelism via a thread pool — with the caveat, learned
     painfully, that `rayon` inside the GDExtension is currently fatal in this
     environment (see `README.md`). Investigate a scoped pool or an explicit
     worker thread instead.
756. SIMD for the soil and moisture diffusion passes, which are the most
     regular loops in the project.
757. Spatial hashing for entity queries, already proven by the edit index.
758. Incremental recomputation everywhere: flow accumulation, lighting, biome
     classification. Never recompute a whole grid when a delta will do.
759. Dirty-region tracking as a shared primitive rather than per-system
     reinvention.
760. Background generation of adjacent regions before the player reaches them.
761. Predictive streaming from the player's movement vector.
762. A loading budget for the first frame, so startup stays under a few seconds.
763. Save/load performance measured, since a mutable world plus strokes plus
     soil deltas is a lot of state.
764. Delta compression on saves, and zstd on the chunk blobs.
765. A telemetry hook recording frame time percentiles during play.
766. Automated performance regression tests on a fixed camera path.
767. A low-spec preset that degrades foliage, volumetrics and LOD distance first,
     since those are the three biggest costs.
768. Steam Deck as an explicit performance target with its own preset
     (`EM_BRIEF.md` §2.1).
769. A performance budget for the two HUD SubViewports, which currently render
     the world a second time.
770. Publish the limits: measured on one machine, in one scene, at one
     resolution — and say which.

## W. Authoring, data tables and fidelity gates (771–800)

*ORRERY's real lesson is not any single system. It is that the discipline
around the systems is what keeps them honest.*

771. A **curated field schema** listing every simulation field with name, kind,
     unit, owner and whether it is saved — ORRERY's `fields.js`, which is what
     makes save migration survivable.
772. A field census script that fails CI when a field is added without being
     registered.
773. **Determinism lint** in CI: no unseeded randomness, no iteration over hash
     maps in simulation code, no float formatting differences.
774. Golden tests: 100 simulated days, replayed, byte-identical
     (`REQUIREMENTS.md` B8).
775. Parity tests between the T0 individual model and the T1 statistical model,
     so promotion and demotion agree.
776. A calibration spine — a table of quantities with target values and
     provenance tags, following `sky-model.md`'s example.
777. `@provenance` annotations on any constant taken from real science, with a
     citation. The ones that are invented get tagged as invented.
778. A **model-limits document** as a first-class deliverable, published beside
     the numbers rather than buried.
779. A single **paint table** compiling to the colour rules (§T item 611).
780. Species, material, module and recipe definitions all as hot-reloadable data.
781. A schema validator for every data file, run in CI.
782. A content pipeline that a designer can use without an engineer
     (`EM_BRIEF.md` R3).
783. An in-game console for inspecting and setting any field at a point.
784. A time-control panel: pause, step, fast-forward, so the slow systems can
     actually be observed and debugged.
785. A scenario loader for reproducing a specific world state in one command.
786. Fixture saves committed to the repo for regression testing.
787. A headless simulation runner for long-horizon experiments without rendering
     — already half-built as `--selftest`.
788. Batch experiments across seeds, reporting distributions rather than
     anecdotes.
789. **A chronicle system** recording notable events per region — ORRERY's
     `whatHappenedHere` — queryable by the player and by the developer.
790. Causal chains attached to every outcome (item 197), stored rather than
     recomputed.
791. An event log the player can scroll: "day 212, band 14, condenser 3 failed".
792. Automated screenshots at fixed points each build, as a visual changelog.
793. Golden-image comparison with a perceptual metric, not pixel equality.
794. A playtest harness that times the first ninety seconds, following ORRERY's
     `?playtest=1` — because delight in the first ninety seconds is rule 3.
795. Instrumented onboarding metrics: how long until the first dig, the first
     crop, the first river diverted.
796. An accessibility brief covering contrast, motion, colour, text size and
     input, with CI gates where they can be automated.
797. A quality register in this same numbered format, so defects get tracked the
     way features do.
798. An architecture ratchet script preventing new cross-layer dependencies —
     specifically, nothing in the renderer may reach into simulation state
     except through the presentation layer.
799. A `NEXT.md` holding the **only** prioritised backlog, capped at ten items,
     so these 800 items never become the work queue.
800. And the rule that governs all of it, taken from ORRERY unchanged: **honesty
     of mechanism beats cosmetic spectacle; legibility beats completeness;
     delight in the first ninety seconds beats feature count; stated limits
     beat implied precision.**

---

## What to build first, out of all 800

Nothing here changes the wave order in `LANDSCAPE_200.md`. It sharpens it.

| Wave | Add from this document | Why |
|---|---|---|
| **1** | 271–274, 300–303, 314 | Once flow routing is live, **draw the rivers**. Channel polylines, a flow-aligned water surface, foam, and sound. This is the moment the world stops being a photograph. |
| **2** | 201–208, 611–616 | Strata plus a compiled paint table. Mountains start reading as geology and the look becomes data. |
| **3** | 411–420, 462–471 | Trees as individuals with light competition, and the three-tier LOD that makes them affordable. |
| **4** | 351–355, 364–372, 377 | Rain that lands where physics says it lands, and a wetness field the world responds to. |
| **5** | 687–691 | The horizon field and the far-side bounce term. One computation serving both photosynthesis and the ambient light, and the light on your face changing when the far side changes. |
| **6** | 771–778, 789–790 | The fidelity gates, before the content that will need them. |

**The single most beautiful thing in this document** is item 691: bounce light
from the opposite wall is physically dominant here, and it is *tinted by what is
growing over there*. Burn a forest on the far side and the light on your hands
changes colour. No open-world game can do that, because none of them are closed.
