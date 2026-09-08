# 1000 More: Making the Kepler Drum Visible

**Status:** v1.0 · **Date:** 2026-09-07 · **Items 2201–3200**
**Continues:** `LANDSCAPE_200/800/1400/2000/2200.md`
**Subject:** the next art pass: light, distance, materials, life, and the frame budget that pays for them.
**Companion:** `RENDER_CONTRACT.md` · `NEXT.md` · `CALIBRATION.md` · `MODEL_LIMITS.md` · `FIELDS.md`

---

## Why this register exists

The simulation is now ahead of the picture. The tree knows drainage, crops,
biomes, dwellings, plants, herd signs, weather and time, but screenshots still
read too often as boxes and cones under no light. The Kepler Drum is mechanically
interesting; it now needs a visual contract strong enough that a player can read
the mechanism before any HUD explains it.

Measured state at time of writing:

```
far field       : ~1M tris in 12 sectors
near chunks     : ~58 meshed chunks, caves included
grass           : MultiMesh cap 6400, one draw call
plants          : sim population target ~12000, LOD streamed near player
light           : no Light3D in the world scene
materials       : terrain, trees, grass, water, props all unshaded/shader-lit
shadows         : off for instanced ground cover and current scenery path
anti-aliasing   : MSAA 2x
post            : Filmic tonemap + glow + depth fog
not enabled     : SSAO, SDFGI, TAA, volumetric fog
```

### Holistic read

- **No light source.** The shaders fake a day direction, but Godot has no sun,
  no real shadow grammar, and no shared belief about where light comes from.
- **The far side is navy.** Depth fog helps empty air, but the opposite wall
  still reads as a dark backing plate instead of inhabited landscape.
- **Biomes are not visual.** The sim has IDs; the picture has mostly one green,
  one cone tree, one clay terrain and some density.
- **The same cone trees repeat.** Plant genetics exist, but mesh vocabulary is
  too small for individual stands to matter.
- **The terrain is clay.** Slopes, soil, wetness, dig freshness and strata are
  data, but they are not yet surface language.
- **Water is flat.** It marks lakes and rivers, but does not catch sky, banks,
  foam, current, wind or depth.
- **The performance ceiling is near.** The picture can get richer only if mesh
  generation, sim ticks and scatter uploads move off the main choke points.

### Distance-band diagram in prose

**Near, 0–60 m:** hands, plants, grass, rocks, water edges, tool marks, footsteps,
wetness, and individual colonists. This band earns touch.

**Focus, 60–300 m:** fields, tree stands, huts, herds, river bends, crop rows,
mist pockets, bridge lines, cliff faces, and the player's next decision. This
band earns legibility.

**Far, 300–6000 m:** the opposite wall is sky and map at once: farmland
patchwork, settlements, glinting rivers, weather fronts, cloud shadows, endcap
windows and biome colour masses. This band earns awe.

### Implementation ledger (v1.0)

| Range | Shipped in tree | Wave 1 target |
|---|---|---|
| 2201–2280 | mostly open | real sun node, shared light vector, shader key term |
| 2281–2360 | partly stubbed | far farmland, settlement glow, anti-alias theta bands |
| 2361–2440 | mostly open | slope split, soil colour, wet-darkening |
| 2441–2540 | mostly open | biome palette table and HUD census colours |
| 2541–2620 | mostly open | six tree archetypes and genome-to-species table |
| 2621–2690 | partly stubbed | biome grass, litter, rocks, dithered ring edge |
| 2691–2770 | partly stubbed | strip sparkle, river glints, edge foam |
| 2771–2850 | mostly open | rain curtains, valley mist, dawn wave |
| 2851–2910 | partly stubbed | visible herds, birds, insects, carcass signs |
| 2911–2970 | partly stubbed | articulated colonist, huts from dwelling data |
| 2971–3020 | partly shipped | aim-follow focus, LUT, photo orbit |
| 3021–3110 | mostly open | threading retest, buffer uploads, budget overlay |
| 3111–3150 | mostly open | golden images and legibility gates |
| 3151–3200 | open | ten-wave sequence, first week = Wave 1 |

---

## AX. Light you can believe (2201–2280)

2201. Add one `DirectionalLight3D` named `RamaSun`; no second sun, no mystery fill node. → `game/scripts/world.gd`
2202. Aim `RamaSun` along `-up_at(player)` so noon feels axis-born, not world-Y-born. → `world.gd`
2203. Keep terrain `render_mode unshaded`; light remains explicit in shader code. → `terrain.gdshader`
2204. Pass the sun vector as `rama_key_dir` to every shader using the light contract. → `rama_light.gdshaderinc`
2205. Key light is `max(dot(N, L), 0.0) * ATTENUATION`, not a hand-tuned brighten. → `rama_light.gdshaderinc`
2206. Use Godot `ATTENUATION` in `light()` where spatial shader lighting is enabled. → `rama_light.gdshaderinc`
2207. Leave opt-out shaders documented with the reason and visual substitute. → `RENDER_CONTRACT.md`
2208. Treat axis strip glow as fill/haze `EMISSION`, not a second directional key. → `terrain.gdshader`
2209. Give far haze its own energy curve so fog does not fake direct sunlight. → `world.gd`
2210. Drive sun colour from day cycle: warm dawn, neutral noon, copper dusk. → `world.gd`
2211. Drive sun energy from day cycle and record the calibrated lux fiction. → `CALIBRATION.md`
2212. Add `rama_bounce_tint` sampled from far-side biome mix. → `paint.rs`
2213. Bounce tint lifts shadowed terrain toward the opposite wall colour. → `rama_light.gdshaderinc`
2214. Keep bounce below key by contract; no biome may glow like a lamp. → `RENDER_CONTRACT.md`
2215. Add a cool air fill term from the cylinder axis direction. → `rama_light.gdshaderinc`
2216. Make fill visible only where `N·L` is weak, preserving day shape. → `terrain.gdshader`
2217. Add a debug toggle for key, fill, bounce and haze separately. → `world.gd`
2218. Print light vector and energy in shot metadata. → `world.gd`
2219. Bind plant, grass, tree, prop and water shaders to the same uniforms. → `world.gd`
2220. Keep old baked ambient values behind one compatibility constant during Wave 1. → `rama_light.gdshaderinc`
2221. Remove per-shader private light colours after the shared include lands. → `terrain.gdshader`
2222. Add shadow budget: two splits, practical reach about 240 m. → `world.gd`
2223. Align split distance to the tilt-shift focus band, not maximum view distance. → `world.gd`
2224. Disable shadows on grass and tiny litter until a measured need appears. → `grass.gdshader`
2225. Allow tree trunks and huts to cast, crowns and blades to receive only. → `tree.gdshader`
2226. Keep far field shadowless; haze and bounce carry the wall. → `paint.rs`
2227. Add contact darkening under huts as baked vertex or decal, not full shadow maps. → `world.gd`
2228. Add cloud-shadow multiplier as a low-frequency term before detailed clouds exist. → `paint.rs`
2229. Cloud shadows affect ground and water, not UI overlays. → `world.gd`
2230. Set sun angular size fiction in docs even if Godot light stays sharp. → `MODEL_LIMITS.md`
2231. Add optional volumetric fog only above low-spec preset. → `world.gd`
2232. Volumetric fog density starts from current depth fog colour and extends it. → `world.gd`
2233. Keep volumetric disabled on Steam Deck preset unless frame budget proves room. → `NEXT.md`
2234. Put fog terms in metres, scaled by `hab_radius` and `hab_length`. → `world.gd`
2235. Add dawn backscatter toward the axis strip, distance-faded. → `rama_light.gdshaderinc`
2236. Prevent backscatter from whitening nearby hands and tools. → `player.gd`
2237. Tune tree leaf wrap after key light, not before. → `tree.gdshader`
2238. Tune grass translucency with the same leaf wrap constant. → `grass.gdshader`
2239. Add water sun glitter threshold from key direction and view direction. → `water.gdshader`
2240. Use one `rama_time_of_day` uniform across terrain, water and props. → `world.gd`
2241. Store light state in screenshot JSON for golden comparisons. → `world.gd`
2242. Add a no-light capture mode to prove shader fallbacks are still legible. → `world.gd`
2243. Add a sun-vector overlay arrow in debug HUD. → `world.gd`
2244. Use biome bounce only beyond near band to avoid noisy local colour shifts. → `terrain.gdshader`
2245. Make caves opt out and document cave lighting as a separate contract. → `RENDER_CONTRACT.md`
2246. Keep interior lamps emissive and local; they do not become a second sun. → `prop.gdshader`
2247. Settlement lights bloom through glow, not through raised albedo. → `world.gd`
2248. Add per-material roughness fiction even if shaders stay unshaded. → `RENDER_CONTRACT.md`
2249. Water receives key as sparkle, terrain receives key as value, plants as translucency. → `rama_light.gdshaderinc`
2250. Add a light sanity test: noon top, dusk side, night dim, no inverted sun. → `tools/`
2251. Record the current no-Light3D state as baseline before adding the node. → `docs/CALIBRATION.md`
2252. Keep the background colour tied to fog light colour after sun changes. → `world.gd`
2253. Add shadow fade before 240 m so the far wall never shows map cascades. → `world.gd`
2254. Avoid SDFGI for now; closed-cylinder bounce is authored, not solved. → `MODEL_LIMITS.md`
2255. Avoid SSAO for Wave 1; baked contact terms are cheaper and more controlled. → `NEXT.md`
2256. Avoid TAA until grass and water motion vectors are understood. → `MODEL_LIMITS.md`
2257. Keep MSAA 2x as the low-spec baseline. → `world.gd`
2258. Create high preset with MSAA 4x only after vegetation aliasing is measured. → `NEXT.md`
2259. Add luminance clamp for emissive windows before glow. → `prop.gdshader`
2260. Use `day` parameter already driving dusk glow as the light source clock. → `world.gd`
2261. Make storm light desaturate before it darkens. → `world.gd`
2262. Make rain reduce sparkle and raise air fill. → `water.gdshader`
2263. Make fog colour warm at dawn and blue-grey under rain. → `world.gd`
2264. Add a `rama_light_debug` material switch for false-colour direct/fill/bounce. → `rama_light.gdshaderinc`
2265. Document every shader still using private lighting after Wave 1. → `RENDER_CONTRACT.md`
2266. Move hard-coded terrain sun constants into the include. → `terrain.gdshader`
2267. Move tree light constants into the include. → `tree.gdshader`
2268. Move grass light constants into the include. → `grass.gdshader`
2269. Move prop light constants into the include. → `prop.gdshader`
2270. Give water a separate Fresnel path but the same key vector. → `water.gdshader`
2271. Add night minimum exposure; the player must still read ground. → `world.gd`
2272. Add lantern dominance at night by lowering sun, not boosting bloom globally. → `world.gd`
2273. Use axis-window emission as a compositional line in photo mode. → `world.gd`
2274. Make shadow acne a calibration entry, not a hidden bias tweak. → `CALIBRATION.md`
2275. Put cascade sizes and bias in the render contract. → `RENDER_CONTRACT.md`
2276. Add one fixed shot that looks toward the sun and one away from it. → `tools/`
2277. Add a capture note when volumetric fog is disabled by quality preset. → `world.gd`
2278. Keep Godot light and shader light synchronized from one setter. → `world.gd`
2279. Delete any duplicate sun node a scene import creates. → `world.gd`
2280. Done means the same hill has a lit side, a shadow side, bounce, and readable haze. → `NEXT.md`

## AY. The far side as the sky (2281–2360)

2281. Treat the far wall as sky-map and landscape at once. → `paint.rs`
2282. Paint farmland patchwork from simulated fertile, settled cells. → `paint.rs`
2283. Use long rectangular crop parcels aligned to local slope and water access. → `paint.rs`
2284. Keep parcel edges metre-scaled so they do not alias into radial spokes. → `paint.rs`
2285. Add a `theta * N` frequency audit for every far stripe. → `paint.rs`
2286. Kill radial streak aliasing by banning high-contrast periodic theta bands. → `paint.rs`
2287. Dither field boundaries with low-frequency cell noise, distance-faded. → `terrain.gdshader`
2288. Add fallow, crop, wet crop and harvested palette states. → `paint.rs`
2289. Tie farm colour to soil moisture and nitrogen, not random green. → `soil.rs`
2290. Add far-side roads as thin desaturated lines between dwellings. → `paint.rs`
2291. Fade roads by distance before they become stair-step noise. → `terrain.gdshader`
2292. Use `dwellings_lod()` to seed settlement light clusters. → `sim/src/lib.rs`
2293. Turn settlement lights on at dusk from dwellings, not from painted dots. → `world.gd`
2294. Cluster lights by town, with warmer core and sparse edge. → `world.gd`
2295. Keep settlement bloom capped; glow suggests habitation, not neon. → `prop.gdshader`
2296. Add window rows on far hut blocks only when cluster density warrants it. → `world.gd`
2297. Make far villages visible as value structure in daytime. → `paint.rs`
2298. Add smoke or heat shimmer above active settlements only in focus/far band. → `world.gd`
2299. Add river glints from water mask, sun vector and view vector. → `paint.rs`
2300. Let river glints blink by wind phase, not by random frame noise. → `water.gdshader`
2301. Use lake-mask depressions to catch pale mist on the opposite wall. → `paint.rs`
2302. Make far lakes darker at centre and brighter at sun-facing edge. → `water.gdshader`
2303. Add distant shoreline sand line where wetness and slope meet. → `paint.rs`
2304. Add endcap windows as low-frequency arcs, not checkerboard glitter. → `endcap.gdshader`
2305. Reduce endcap contrast where fog already carries depth. → `endcap.gdshader`
2306. Add endcap maintenance lights that follow the day cycle. → `world.gd`
2307. Make cloud shadows crawl across the far wall at readable speed. → `clouds.gdshader`
2308. Cloud shadows must be kilometres wide in fiction, never cell-sized freckles. → `paint.rs`
2309. Add storm-front darkening as a broad band along habitat axis. → `world.gd`
2310. Add dawn wave: far side warms before near grass catches up. → `world.gd`
2311. Retune haze after farmland coverage, because new value mass changes contrast. → `world.gd`
2312. Avoid whitening the far wall; lift black with coloured air. → `rama_light.gdshaderinc`
2313. Add a far-legibility debug shot from ground looking across the cylinder. → `tools/`
2314. Add a plan-view far material thumbnail for every biome. → `paint.rs`
2315. Keep opposite-wall crop parcels bigger than the minimum visible pixel pitch. → `CALIBRATION.md`
2316. Add colour hierarchy: water brightest glints, fields mid, forest dark, cliffs sharp. → `RENDER_CONTRACT.md`
2317. Make biome boundaries visible as mass, not outlines. → `biome.rs`
2318. Paint scarp faces as value breaks along far ridges. → `paint.rs`
2319. Add snow or alpine lichen only where biome and elevation agree. → `biome.rs`
2320. Add desert or dry scarp dust tint only where moisture is low. → `soil.rs`
2321. Use same nine biome IDs as `biome.rs`; no render-only biome enum. → `biome.rs`
2322. Add far orchards as dotted grids only inside settlement farm halos. → `plant.rs`
2323. Distance-fade orchard dots before they become moire. → `terrain.gdshader`
2324. Add paddies as reflective rectangles in wet lowlands. → `paint.rs`
2325. Restrict paddies to water-managed communities, not any wet flat. → `dwellings_lod`
2326. Add burn scars or disturbed soil from chronicle events when available. → `paint.rs`
2327. Mark active dig scars as fresh high-value faces for a few days. → `paint.rs`
2328. Add old dig scars as weathered muted patches. → `soil.rs`
2329. Far roads should curve with terrain and drainage, never ignore slope. → `paint.rs`
2330. Far bridges are tiny bright strokes only where roads cross water. → `paint.rs`
2331. Add a theatre-mode camera shot that frames near ground and far wall together. → `world.gd`
2332. Validate far side in grayscale; it must read without hue. → `tools/`
2333. Validate far side in colour-blind palette; crop and forest must still separate. → `tools/`
2334. Add far material ID output for screenshot probes. → `terrain.gdshader`
2335. Add biome mass percentages to shot metadata. → `world.gd`
2336. Make paint pass deterministic by seed and cell coordinate. → `paint.rs`
2337. Cache far paint lookups per sector to avoid per-vertex recomputation. → `paint.rs`
2338. Keep far shader branch count low; the far wall is already ~1M tris. → `terrain.gdshader`
2339. Add coverage masks before detail masks; broad shape first. → `paint.rs`
2340. Add a visual budget: no more than three far patterns visible at once. → `RENDER_CONTRACT.md`
2341. Crop rows visible in focus band become crop masses in far band. → `paint.rs`
2342. Forests visible as individual crowns in focus become velvet masses in far band. → `plant.rs`
2343. Cliffs visible as outcrops in focus become rim-light bands in far band. → `terrain.gdshader`
2344. Water visible as waves in focus becomes glint lines in far band. → `water.gdshader`
2345. Towns visible as huts in focus become warm constellations in far band. → `world.gd`
2346. Add far-side bounce sampling from the rendered biome map, not a fixed blue. → `paint.rs`
2347. Clamp bounce saturation so farmland does not paint shadows lime. → `rama_light.gdshaderinc`
2348. Add sunset silhouettes for ridge crests against hazed air. → `terrain.gdshader`
2349. Keep silhouettes one-pixel-stable by using geometry, not noisy alpha. → `paint.rs`
2350. Add far wall exposure test for midnight, dawn, noon and storm. → `tools/`
2351. Remove any full-cylinder radial debug pattern from release materials. → `terrain.gdshader`
2352. Add far clouds as slow masks crossing the wall, not full volumetric meshes. → `clouds.gdshader`
2353. Cloud masks project in habitat coordinates so they wrap correctly. → `clouds.gdshader`
2354. Add endcap atmospheric falloff so windows recede rather than tile. → `endcap.gdshader`
2355. Give far water a separate glint atlas if shader ALU becomes too high. → `water.gdshader`
2356. Add perf counter for far paint time per sector. → `world.gd`
2357. Add an artist note for acceptable far-wall darkness. → `CALIBRATION.md`
2358. Re-run screenshots after haze retune, because coverage changes perceived exposure. → `tools/`
2359. Done means the opposite wall reads inhabited before the player moves. → `NEXT.md`
2360. Done also means no visible theta spokes, checkerboard shimmer or navy plate. → `RENDER_CONTRACT.md`

## AZ. Terrain surface (2361–2440)

2361. Split terrain visual material into cliff, slope and flat before adding detail. → `terrain.gdshader`
2362. Derive cliff from normal steepness and curvature, not height alone. → `terrain.gdshader`
2363. Derive slope from drainage and grade so gullies read as landforms. → `sim/src/soil.rs`
2364. Keep flats calm; they carry farms, paths and water marks. → `paint.rs`
2365. Add rock strata bands from `material_at`, distance-faded. → `terrain.gdshader`
2366. Make strata follow elevation and local normal, not screen-space noise. → `terrain.gdshader`
2367. Use fresh dig faces to expose brighter cut rock. → `world.gd`
2368. Weather fresh rock toward local soil over simulated days. → `soil.rs`
2369. Add scree below cliffs from slope instability. → `paint.rs`
2370. Dither scree density so it breaks clay planes without sparkling. → `terrain.gdshader`
2371. Add outcrop masks where rock is shallow under soil. → `soil.rs`
2372. Outcrops use fewer, larger shapes in far band. → `paint.rs`
2373. Soil colour comes from `soil.rs` fields, not terrain height palette. → `soil.rs`
2374. Moisture darkens soil by value and lowers saturation. → `terrain.gdshader`
2375. Wet-darkening fades out on vertical cliffs unless water is running there. → `terrain.gdshader`
2376. Nitrogen-rich cultivated soil warms slightly but stays plausible. → `soil.rs`
2377. Dead soil shifts grey-brown and loses grass density. → `plant.rs`
2378. Add compacted path material from repeated agent/player travel. → `world.gd`
2379. Paths lighten when dusty and darken when wet. → `terrain.gdshader`
2380. Add erosion streaks below channel cuts. → `paint.rs`
2381. Make erosion streaks follow flow direction from the routing field. → `sim/src/lib.rs`
2382. Add bank undercut colour where water velocity is high. → `water.gdshader`
2383. Add mud flats at seasonal lake margins. → `paint.rs`
2384. Add salt or mineral bloom only if chemistry field exists. → `FIELDS.md`
2385. Add cave mouth darkening with a smooth ambient falloff. → `terrain.gdshader`
2386. Cave interiors opt into a separate low-light material. → `RENDER_CONTRACT.md`
2387. Add bevel-like normal shading on voxel-ish cuts. → `terrain.gdshader`
2388. Use shader normal perturbation for small gravel; do not increase mesh density. → `terrain.gdshader`
2389. Retire gravel perturbation before it aliases in focus band. → `terrain.gdshader`
2390. Add large boulder props where outcrop and scree masks agree. → `world.gd`
2391. Boulders share terrain palette and light include. → `prop.gdshader`
2392. Add clay cracking only in dry flats, distance-faded hard. → `terrain.gdshader`
2393. Add moss/lichen tint on wet shaded cliffs in matching biomes. → `biome.rs`
2394. Add alpine lichen on cold exposed rock. → `biome.rs`
2395. Add red scarp oxide tint in dry high-mineral zones. → `soil.rs`
2396. Add terrain material debug overlay keyed to cliff/slope/flat. → `world.gd`
2397. Add probe readout for soil colour source under reticle. → `world.gd`
2398. Store terrain material ID in screenshot metadata. → `world.gd`
2399. Add a grayscale terrain shot; surface must read without vegetation. → `tools/`
2400. Add a wet/dry A-B shot after rain. → `tools/`
2401. Keep channel walls visually fresh for a short decay window. → `soil.rs`
2402. Weather old channels differently from natural stream beds. → `paint.rs`
2403. Add cultivated ridge/furrow relief through shader normals. → `plot.gdshader`
2404. Make furrows align to plot orientation, not global axes. → `world.gd`
2405. Add terrace edge material where agents or player build slopes. → `world.gd`
2406. Terrace edges receive contact dirt at the base. → `terrain.gdshader`
2407. Add spoil heaps with matching source material, not generic brown. → `spoil.gdshader`
2408. Spoil heaps weather from fresh cut to dusty pile. → `soil.rs`
2409. Add rubble flecks around construction sites. → `world.gd`
2410. Keep rubble as instanced props with shadows off until measured. → `prop.gdshader`
2411. Add thaw/frost colour only through weather state. → `world.gd`
2412. Frost highlights high exposed flats at dawn. → `terrain.gdshader`
2413. Do not add snow unless temperature persistence supports it. → `MODEL_LIMITS.md`
2414. Add material-at caching for render queries to protect far mesh time. → `paint.rs`
2415. Share palette constants with biome palette table. → `RENDER_CONTRACT.md`
2416. Add `terrain_surface_kind` to field registry if persisted. → `FIELDS.md`
2417. Keep all terrain albedos below display-white; light creates brightness. → `RENDER_CONTRACT.md`
2418. Add slope AO only as low-frequency term. → `terrain.gdshader`
2419. Add concavity darkening in gullies, not everywhere. → `terrain.gdshader`
2420. Add convex rim catchlights on ridges facing the sun. → `terrain.gdshader`
2421. Use bounce tint to keep shadowed cliff faces coloured. → `rama_light.gdshaderinc`
2422. Preserve biome hue under terrain material variation. → `biome.rs`
2423. Make high-frequency rock detail vanish by 120 m. → `terrain.gdshader`
2424. Make medium strata survive to 600 m. → `terrain.gdshader`
2425. Make broad landform value survive to the far wall. → `paint.rs`
2426. Add terrain material sample to photo metadata. → `world.gd`
2427. Add authoring comments for every distance at which a terrain feature retires. → `terrain.gdshader`
2428. Test a dug face near water, in forest, and on dry scarp. → `tools/`
2429. Add terrain palette migration note for old screenshots. → `CALIBRATION.md`
2430. Avoid random per-vertex colour noise on flat farms. → `paint.rs`
2431. Prefer large mottled patches over speckle on the far wall. → `terrain.gdshader`
2432. Add anti-moire clamp for repeated strata bands. → `terrain.gdshader`
2433. Keep caves visible through value and silhouette before adding props. → `terrain.gdshader`
2434. Add reticle text for "fresh cut", "wet bank", "old path". → `world.gd`
2435. Use the same surface kind for footsteps and visual material. → `player.gd`
2436. Let audio query wetness and material rather than duplicating rules. → `FIELDS.md`
2437. Add doc examples for three terrain reads: cliff, farm, stream bank. → `RENDER_CONTRACT.md`
2438. Done means screenshots no longer read as smooth clay. → `NEXT.md`
2439. Done means new cuts visibly differ from old weathered ground. → `CALIBRATION.md`
2440. Done means terrain alone explains slope, wetness and material at a glance. → `RENDER_CONTRACT.md`

## BA. Biomes you can read at a glance (2441–2540)

2441. Define one visual palette table for the same nine biome IDs as `biome.rs`. → `biome.rs`
2442. Palette columns are ground, grass, understorey, rock, water edge and signature. → `RENDER_CONTRACT.md`
2443. HUD census uses the exact palette swatches from the render table. → `world.gd`
2444. No render-only biome names; every visual label maps to sim ID. → `biome.rs`
2445. Add ecotone blend width in metres, not percent of cell. → `biome.rs`
2446. Ecotone blends ground first, then understorey, then signature objects. → `paint.rs`
2447. Make grass hue shift by biome before density changes. → `grass.gdshader`
2448. Make understorey density shift by biome before adding unique meshes. → `plant.rs`
2449. Make rock tint obey terrain material under biome tint. → `terrain.gdshader`
2450. Add wetland reeds as first signature object. → `plant.rs`
2451. Reeds spawn on shallow water edges and high moisture flats. → `plant.rs`
2452. Reeds sway vertically less than grass and bend with wind. → `grass.gdshader`
2453. Add willow silhouettes in wet lowland biomes. → `plant.rs`
2454. Willows use drooping crown archetype, not cone tree scaling. → `world.gd`
2455. Add meadow flowers as low saturated flecks in temperate grassland. → `grass.gdshader`
2456. Flower flecks retire by 70 m to avoid confetti far wall. → `grass.gdshader`
2457. Add thornbush in dry scrub. → `plant.rs`
2458. Thornbush silhouette is low, irregular and sparse. → `world.gd`
2459. Add canopy mass for humid forest. → `plant.rs`
2460. Canopy colour is dark olive at base, sunlit yellow-green at crown. → `tree.gdshader`
2461. Add alpine lichen for cold exposed rock. → `biome.rs`
2462. Alpine lichen is surface tint, not new geometry. → `terrain.gdshader`
2463. Add scarp palette: red-brown rock, pale dry grass, black crevice. → `biome.rs`
2464. Scarp signature is cliff face and scree, not plant count. → `paint.rs`
2465. Add paddies for wet cultivated biome. → `paint.rs`
2466. Paddies are reflective flats with raised earthen edges. → `plot.gdshader`
2467. Add orchard rows for settled fertile biome. → `plant.rs`
2468. Orchard trees use regular spacing only where dwellings support it. → `dwellings_lod`
2469. Add riparian strip palette independent of surrounding biome. → `paint.rs`
2470. Riparian strips follow water and override dry grass locally. → `plant.rs`
2471. Add burnt or disturbed biome overlay as temporary state. → `paint.rs`
2472. Disturbance never changes biome ID; it is a visual layer. → `FIELDS.md`
2473. Add biome ID to plant LOD stride if absent. → `sim/src/lib.rs`
2474. Use `PLANT_STRIDE` biome slot already documented for shade choice. → `world.gd`
2475. Make biome count visible in a debug census panel. → `world.gd`
2476. Census colours match ground palette, not arbitrary UI rainbow. → `world.gd`
2477. Add biome thumbnail strip to photo metadata. → `world.gd`
2478. Add a nine-biome golden shot grid. → `tools/`
2479. Measure Delta E between adjacent biome palettes. → `tools/`
2480. Require biome contrast to survive Filmic tonemap. → `tools/`
2481. Add colour-blind alternate palette only if contrast fails. → `RENDER_CONTRACT.md`
2482. Keep biome greens desaturated; avoid emerald grass. → `grass.gdshader`
2483. Wetland ground is dark green-brown, not blue. → `biome.rs`
2484. Dry scrub ground is dusty ochre, not orange. → `biome.rs`
2485. Forest ground is low-value leaf litter under crown density. → `biome.rs`
2486. Grassland ground shows warm soil between blades. → `biome.rs`
2487. Alpine ground is pale rock plus lichen, with sparse grass. → `biome.rs`
2488. Cultivated ground follows crop state more than natural biome. → `paint.rs`
2489. Riparian water edges get saturated green only in narrow bands. → `plant.rs`
2490. Cliff biome emphasises value contrast over hue. → `terrain.gdshader`
2491. Add biome-specific rock props from one shared mesh set. → `world.gd`
2492. Rock prop tint comes from surface material and biome table. → `prop.gdshader`
2493. Add biome-specific deadwood only where tree density supports it. → `plant.rs`
2494. Deadwood uses grey low-emission material and lies with slope. → `world.gd`
2495. Add flower season parameter from day cycle. → `plant.rs`
2496. Flower season affects density and hue, not biome identity. → `FIELDS.md`
2497. Add drought stress desaturation across all living biome palettes. → `plant.rs`
2498. Add rain saturation recovery with wet-darkening. → `terrain.gdshader`
2499. Add snow/frost placeholder only as disabled palette slot. → `MODEL_LIMITS.md`
2500. Add biome edge debugging in plan overlay. → `world.gd`
2501. Use top-two biome blending only, matching previous palette rule. → `biome.rs`
2502. Reject weighted average of all biomes; it greys ecotones. → `RENDER_CONTRACT.md`
2503. Add stable hash variation inside a biome to avoid flat paint. → `paint.rs`
2504. Keep variation lower than biome contrast. → `CALIBRATION.md`
2505. Add soil moisture as secondary palette axis. → `soil.rs`
2506. Add fertility as tertiary palette axis only for cultivated land. → `soil.rs`
2507. Add biome signature object cap per chunk. → `world.gd`
2508. Never let signature objects outnumber readable ground cover. → `RENDER_CONTRACT.md`
2509. Use MultiMesh per signature family, not per biome if material can vary. → `world.gd`
2510. Encode instance colour with biome palette, avoiding material explosion. → `world.gd`
2511. Add biome-to-audio mapping so visible place sounds like itself. → `FIELDS.md`
2512. Wetland insects are dense and high; alpine air is sparse and low. → `world.gd`
2513. Forest bird density follows canopy health. → `plant.rs`
2514. Dry scrub wind hiss replaces lush rustle. → `world.gd`
2515. Add biome names to reticle only in debug, not default HUD. → `world.gd`
2516. Let players infer biome before seeing the name. → `RENDER_CONTRACT.md`
2517. Add biome transition shots across wetland-to-grassland. → `tools/`
2518. Add biome transition shots across grassland-to-scarp. → `tools/`
2519. Add biome transition shots across forest-to-cultivated. → `tools/`
2520. Add far-wall biome mass validation. → `tools/`
2521. Make far biomes read as large colour continents. → `paint.rs`
2522. Make near biomes read as material and object choices. → `world.gd`
2523. Make focus biomes read as silhouettes and understorey. → `plant.rs`
2524. Add palette provenance to calibration docs. → `CALIBRATION.md`
2525. Tag invented palette values until screenshot-calibrated. → `CALIBRATION.md`
2526. Add biome palette export for designers. → `tools/`
2527. Keep palette data in Rust or shared JSON, not copied into three shaders. → `biome.rs`
2528. Generate shader uniforms from palette source if duplication becomes real. → `tools/`
2529. Add material preview scene for all biome palettes. → `game/`
2530. Add failing test if a biome ID lacks a visual palette. → `tools/`
2531. Add failing test if HUD census colour differs from visual palette. → `tools/`
2532. Add failing test if biome count changes without docs update. → `tools/`
2533. Add model-limit note that biomes are gameplay categories, not botany. → `MODEL_LIMITS.md`
2534. Keep signature objects suggestive rather than encyclopedic. → `RENDER_CONTRACT.md`
2535. Add biome restoration visual: damaged, recovering, mature. → `plant.rs`
2536. Restoration stages use density and colour before new assets. → `grass.gdshader`
2537. Add screenshot caption naming biome evidence under reticle. → `world.gd`
2538. Done means a player can name wetland, forest, scrub and farm from a still. → `NEXT.md`
2539. Done means biome HUD confirms what the eye already saw. → `world.gd`
2540. Done means every biome visual claim traces back to `biome.rs`. → `biome.rs`

## BB. Trees and plants as individuals (2541–2620)

2541. Replace single cone vocabulary with six first-pass tree archetype meshes. → `world.gd`
2542. First six archetypes: cone, broadleaf, willow, palm/reed cluster, thorn, snag. → `plant.rs`
2543. Follow with four second-pass archetypes after palette validation. → `NEXT.md`
2544. Second four: orchard, alpine krummholz, canopy giant, young sapling. → `plant.rs`
2545. Add `genome_id` lookup into a species/form table. → `plant.rs`
2546. Species table drives archetype, bark, leaf palette and size curve. → `plant.rs`
2547. Carbon drives height and crown volume, not just colour. → `plant.rs`
2548. Water stress drives leaf loss and dullness. → `plant.rs`
2549. Nitrogen stress drives yellowing before death. → `plant.rs`
2550. Age drives trunk thickness and branch count. → `plant.rs`
2551. Dead plants become snags or litter, not instant disappearance. → `world.gd`
2552. Snags persist by biome and decay timer. → `plant.rs`
2553. Fallen logs spawn where snags decay on steep or windy ground. → `world.gd`
2554. LOD shade must match near mesh shade at handoff. → `tree.gdshader`
2555. Far tree impostors use same palette and light include. → `tree.gdshader`
2556. Mid LOD uses simplified crown silhouettes, not scaled cones. → `world.gd`
2557. Near LOD gets trunk lean from deterministic genome hash. → `plant.rs`
2558. Crown asymmetry follows prevailing wind and slope. → `plant.rs`
2559. Add branch fork jitter from `genome_id`, stable across reload. → `world.gd`
2560. Add biome species distribution table. → `biome.rs`
2561. Wetland prefers reeds and willows. → `biome.rs`
2562. Forest prefers broadleaf and canopy giant. → `biome.rs`
2563. Dry scrub prefers thorn and low twisted forms. → `biome.rs`
2564. Alpine prefers krummholz and sparse lichen props. → `biome.rs`
2565. Cultivated zones allow orchard form when dwellings support it. → `plant.rs`
2566. Add per-instance wind phase from world position. → `tree.gdshader`
2567. Wind bends crown more than trunk. → `tree.gdshader`
2568. Gusts move stands coherently, not every plant alone. → `world.gd`
2569. Reduce wind under dense canopy. → `plant.rs`
2570. Add leaf wrap lighting shared with grass. → `rama_light.gdshaderinc`
2571. Add underside darkening to crowns for volume. → `tree.gdshader`
2572. Add trunk contact shadow as vertex colour or shader term. → `tree.gdshader`
2573. Keep plant shadows off until tree trunks have budget. → `world.gd`
2574. Allow large trunks to cast in high preset only. → `world.gd`
2575. Add fruit/flower seasonal marker only for species that supports it. → `plant.rs`
2576. Fruit markers must be gameplay-queryable if harvestable. → `FIELDS.md`
2577. Add seedling ring around mature trees where dispersal supports it. → `plant.rs`
2578. Seedlings use a separate low-cost grass-like MultiMesh. → `world.gd`
2579. Add browsed/deer-damaged shape for grazer pressure. → `plant.rs`
2580. Add burn/drought scar material variants. → `tree.gdshader`
2581. Add tree archetype ID to plant LOD buffer. → `sim/src/lib.rs`
2582. Keep stride versioned if plant LOD buffer changes. → `FIELDS.md`
2583. Add debug colour by archetype. → `world.gd`
2584. Add census count by archetype and biome. → `world.gd`
2585. Add golden shot for single tree against sky wall. → `tools/`
2586. Add golden shot for dense forest across LOD boundary. → `tools/`
2587. Add golden shot for wetland reed edge. → `tools/`
2588. Add golden shot for dry thorn scrub. → `tools/`
2589. Add golden shot for dead snag and fallen log. → `tools/`
2590. Use MultiMesh custom data for archetype parameters where possible. → `world.gd`
2591. Avoid spawning thousands of scene nodes for trees. → `world.gd`
2592. Batch by mesh archetype and material family. → `world.gd`
2593. Keep one shader for tree families unless silhouette needs a new mesh. → `tree.gdshader`
2594. Add plant upload budget per refresh. → `world.gd`
2595. Spread plant LOD rebuilds over frames if buffer is large. → `world.gd`
2596. Add plant distance ring edge dithering. → `tree.gdshader`
2597. Avoid alpha-blend leaves until sorting cost is measured. → `tree.gdshader`
2598. Use dithered cutout for leaf cards if cards are introduced. → `tree.gdshader`
2599. Add low-poly branch geometry only in near 0–60 m. → `world.gd`
2600. Keep focus-band silhouettes strong; leaf detail can vanish. → `world.gd`
2601. Add planted orchard rows as recognisable human pattern. → `plant.rs`
2602. Orchard row spacing follows plot geometry. → `plot.gdshader`
2603. Add individual name/debug ID for selected notable trees. → `world.gd`
2604. Notable trees are sim facts only if persisted. → `FIELDS.md`
2605. Add damage feedback when player cuts or digs near roots. → `plant.rs`
2606. Root damage changes form slowly, not instantly. → `plant.rs`
2607. Add sapling-to-tree transition visual. → `plant.rs`
2608. Add plant death transition visual before removal. → `plant.rs`
2609. Match audio wind rustle to archetype. → `world.gd`
2610. Match footstep litter to local plant community. → `player.gd`
2611. Add code comments for archetype units and ranges. → `plant.rs`
2612. Add model-limit note: archetypes imply species, not taxonomy. → `MODEL_LIMITS.md`
2613. Add asset budget: first pass uses generated meshes only. → `NEXT.md`
2614. Add exportable seed list for reproducing odd trees. → `tools/`
2615. Add crash guard for unknown genome ID in debug builds. → `plant.rs`
2616. Unknown species falls back visibly to sapling placeholder, not cone forest. → `world.gd`
2617. Done means no screenshot shows one cone tree repeated unchanged. → `NEXT.md`
2618. Done means plant health, biome and age alter silhouette. → `plant.rs`
2619. Done means LOD swaps do not change plant brightness. → `tree.gdshader`
2620. Done means the forest looks simulated because it is. → `RENDER_CONTRACT.md`

## BC. The ground layer (2621–2690)

2621. Make grass palette biome-driven before increasing blade count. → `grass.gdshader`
2622. Grass density comes from plant health, moisture and disturbance. → `plant.rs`
2623. Add blade height variation by soil fertility and shade. → `grass.gdshader`
2624. Add dry grass straw tint in stressed cells. → `grass.gdshader`
2625. Add wet grass darkening after rain and near water. → `grass.gdshader`
2626. Add flowers as sparse custom-colour grass instances. → `world.gd`
2627. Flowers spawn from biome and season, not random decoration. → `plant.rs`
2628. Add leaf litter layer under forests. → `world.gd`
2629. Litter colour follows local tree archetype and decay. → `plant.rs`
2630. Add small rocks as instanced props where scree/outcrop masks agree. → `world.gd`
2631. Rock instances share terrain material tint. → `prop.gdshader`
2632. Add logs where dead trees have fallen. → `plant.rs`
2633. Logs align to slope and last wind direction. → `world.gd`
2634. Add reed stubble and mud at trampled wetland edges. → `plant.rs`
2635. Add wear paths from repeated player and agent movement. → `world.gd`
2636. Wear paths reduce grass density first, then expose soil. → `grass.gdshader`
2637. Wet wear paths become dark slick mud. → `terrain.gdshader`
2638. Dry wear paths become pale dust. → `terrain.gdshader`
2639. Add crop rows in cultivated plots. → `plot.gdshader`
2640. Crop rows follow plot orientation and row spacing. → `world.gd`
2641. Crop rows fade to farm colour mass in far band. → `paint.rs`
2642. Add mulch or tilled soil state for recently worked plots. → `soil.rs`
2643. Tilled soil has visible furrow normals, not extra mesh. → `plot.gdshader`
2644. Add dithered ring edge to grass MultiMesh visibility. → `grass.gdshader`
2645. Grass ring dither must be stable in world space. → `grass.gdshader`
2646. Reduce grass density under water and buildings. → `world.gd`
2647. Reduce grass density on steep cliffs. → `terrain.gdshader`
2648. Increase grass clumping with low-frequency noise. → `grass.gdshader`
2649. Keep clump noise below biome-level contrast. → `RENDER_CONTRACT.md`
2650. Add ground-cover refresh budget overlay. → `world.gd`
2651. Upload grass buffer only when anchor moves beyond threshold. → `world.gd`
2652. Split ground cover into grass, litter and stone MultiMeshes if needed. → `world.gd`
2653. Keep each ground layer one draw call per archetype. → `world.gd`
2654. Add near-only mushrooms or small flora in wet shaded forest. → `plant.rs`
2655. Small flora retire by 40 m. → `grass.gdshader`
2656. Add insect spark/flicker layer only where biome supports it. → `world.gd`
2657. Insect visuals must match BF audio and density. → `FIELDS.md`
2658. Add pebble scale variation from deterministic hash. → `world.gd`
2659. Add blade yaw and lean jitter already required by render contract. → `grass.gdshader`
2660. Add grass shadow illusion through base darkening, not shadow maps. → `grass.gdshader`
2661. Add contact flattening around huts and paths. → `world.gd`
2662. Add player footprint marks as short-lived wear decals or field changes. → `player.gd`
2663. Footprints only persist if the simulation stores disturbance. → `FIELDS.md`
2664. Add rain ripple marks in mud flats. → `terrain.gdshader`
2665. Add pollen/seed drift only after wind has direction. → `world.gd`
2666. Keep particles capped and quality-gated. → `NEXT.md`
2667. Add reticle ground readout: grass, litter, rock, mud, crop. → `world.gd`
2668. Add debug false colour for ground-cover source. → `world.gd`
2669. Add golden shot for grass ring edge while moving. → `tools/`
2670. Add golden shot for crop rows and wear path crossing. → `tools/`
2671. Add golden shot for litter under canopy. → `tools/`
2672. Add golden shot for wet mud after rain. → `tools/`
2673. Add golden shot for dry scrub sparse cover. → `tools/`
2674. Keep ground-cover albedo dark enough for light to matter. → `RENDER_CONTRACT.md`
2675. Keep flowers low-count and high-meaning. → `plant.rs`
2676. Add harvest stubble after crop collection. → `plot.gdshader`
2677. Add trampling by grazers to alter grass height. → `plant.rs`
2678. Add seasonal litter pulse after leaf drop if season exists. → `FIELDS.md`
2679. Add low-spec toggle to halve ground-cover instance cap. → `world.gd`
2680. Add high preset to expand radius before count. → `world.gd`
2681. Measure CPU cost of `grass_field` at 6400 cap. → `sim/src/lib.rs`
2682. Cache ground cover by chunk if rebuild cost exceeds budget. → `world.gd`
2683. Avoid per-frame randomization; all jitter comes from coordinate hash. → `grass.gdshader`
2684. Share wind phase with plants. → `world.gd`
2685. Add one material include for ground-cover lighting. → `rama_light.gdshaderinc`
2686. Done means near ground rewards looking down. → `NEXT.md`
2687. Done means the ring edge is invisible during normal walking. → `grass.gdshader`
2688. Done means biomes differ before tree silhouettes appear. → `biome.rs`
2689. Done means performance stays one draw call per layer. → `world.gd`
2690. Done means ground cover is simulation evidence, not carpet. → `RENDER_CONTRACT.md`

## BD. Water that shines (2691–2770)

2691. Add sun sparkle from axis strip and key light direction. → `water.gdshader`
2692. Sparkle is view-dependent, narrow and distance-stable. → `water.gdshader`
2693. Use analytic wave derivatives for sparkle normals. → `water.gdshader`
2694. Add fake planar reflection of the far wall on still lakes. → `water.gdshader`
2695. Reflection samples broad far colour, not a screen-space mirror. → `paint.rs`
2696. Fade reflection by wave chop and view angle. → `water.gdshader`
2697. Add river glints along flow direction. → `paint.rs`
2698. Flow glints stretch downstream, not across banks. → `water.gdshader`
2699. Add foam where velocity and slope cross threshold. → `sim/src/lib.rs`
2700. Foam appears at cascades, banks and obstacles first. → `water.gdshader`
2701. Foam decays downstream using flow distance. → `paint.rs`
2702. Add Beer-Lambert sheet absorption for depth colour. → `water.gdshader`
2703. Shallow water is transparent green-brown over bed. → `water.gdshader`
2704. Deep water shifts blue-green and loses bed detail. → `water.gdshader`
2705. Add wet sand/soil rim at lake margins. → `terrain.gdshader`
2706. Wet rim expands after rain and retreats as soil dries. → `soil.rs`
2707. Add current lines on rivers from velocity field. → `water.gdshader`
2708. Current lines retire before sub-pixel shimmer. → `water.gdshader`
2709. Add small standing waves at constrictions. → `sim/src/lib.rs`
2710. Add waterfall/cascade spray only where terrain drop warrants it. → `world.gd`
2711. Keep spray particles quality-gated. → `NEXT.md`
2712. Add rain dimples on water during weather. → `water.gdshader`
2713. Rain dimples use temporal phase, not random flicker. → `water.gdshader`
2714. Add wind direction to wave headings. → `world.gd`
2715. Wave amplitude follows weather wind and shelter. → `water.gdshader`
2716. Sheltered ponds stay smoother than open lakes. → `paint.rs`
2717. Add bank vegetation reflection tint in wetlands. → `plant.rs`
2718. Add water audio parameters from same flow and foam fields. → `FIELDS.md`
2719. Match riffle sound with visible foam and glint. → `world.gd`
2720. Match pool sound with smooth dark surface. → `world.gd`
2721. Match cascade sound with spray and white water. → `world.gd`
2722. Add reticle readout for depth, flow and wet bank. → `world.gd`
2723. Add water debug overlay for depth, velocity and foam. → `world.gd`
2724. Add golden shot for still lake reflection. → `tools/`
2725. Add golden shot for river bend glint. → `tools/`
2726. Add golden shot for cascade foam. → `tools/`
2727. Add golden shot for rain on pond. → `tools/`
2728. Add golden shot for wet shoreline after storm. → `tools/`
2729. Keep water albedo dark; brightness comes from reflection and glint. → `RENDER_CONTRACT.md`
2730. Prevent water sheet from covering non-water low ground. → `water.gdshader`
2731. Clip water by lake/river masks rather than elevation alone. → `paint.rs`
2732. Add shoreline feather based on slope and depth. → `water.gdshader`
2733. Add mud opacity near banks. → `water.gdshader`
2734. Add algae tint only where nutrient and stillness support it. → `soil.rs`
2735. Algae is gameplay state if it affects water quality. → `FIELDS.md`
2736. Add night water highlights from settlement lights only near settlements. → `world.gd`
2737. Far water glints can imply rivers before geometry resolves. → `paint.rs`
2738. Focus water gets wave shape; far water gets value strokes. → `water.gdshader`
2739. Near water gets edge foam, transparency and bed colour. → `water.gdshader`
2740. Add bridge/structure reflection as simple vertical smear if built. → `prop.gdshader`
2741. Add wake ripples around player only if water interaction exists. → `player.gd`
2742. Add animal drinking ripples when herds reach water. → `world.gd`
2743. Add irrigation channels with narrower sparkle and muddy banks. → `channel.gdshader`
2744. Channel water should share the global water light contract. → `channel.gdshader`
2745. Add ditch dryness state: cracked bed versus flowing line. → `soil.rs`
2746. Add floodplain sheen after overflow. → `paint.rs`
2747. Floodplain sheen decays with infiltration. → `soil.rs`
2748. Add foam MultiMesh budget and cap. → `world.gd`
2749. Add splash MultiMesh budget and cap. → `world.gd`
2750. Reuse existing `foam_mm` and `splash_mm` paths before adding new nodes. → `world.gd`
2751. Add water material constants to render contract. → `RENDER_CONTRACT.md`
2752. Add calibration note for absorption coefficients. → `CALIBRATION.md`
2753. Tag invented water coefficients until visually measured. → `CALIBRATION.md`
2754. Add low-spec water mode: no reflection, glints only. → `world.gd`
2755. Add high water mode: reflection plus rain dimples. → `world.gd`
2756. Avoid screen-space reflections; closed drum reflection can be authored cheaper. → `MODEL_LIMITS.md`
2757. Avoid transparent overdraw storms by limiting water layers. → `water.gdshader`
2758. Add sorting test for water over terrain and channels. → `tools/`
2759. Add alpha contract for water and foam. → `RENDER_CONTRACT.md`
2760. Add temporal stability test for glitter. → `tools/`
2761. Clamp sparkle density by distance. → `water.gdshader`
2762. Sparkles stretch with motion blur only if post supports it. → `tiltshift.gdshader`
2763. Add photo-mode sparkle freeze for comparable captures. → `world.gd`
2764. Add fog interaction: distant water brightens toward air, not black. → `rama_light.gdshaderinc`
2765. Add storm interaction: water darkens but glints under breaks. → `world.gd`
2766. Done means a lake reads wet in a still screenshot. → `NEXT.md`
2767. Done means rivers reveal flow direction without UI. → `water.gdshader`
2768. Done means audio and visible water agree. → `FIELDS.md`
2769. Done means water is not a flat blue sheet. → `RENDER_CONTRACT.md`
2770. Done means the axis strip can sparkle on the ground. → `water.gdshader`

## BE. Weather and air as spectacle (2771–2850)

2771. Add rain curtains as camera-facing bands in near/focus distance. → `world.gd`
2772. Rain curtains follow wind and fade before the far wall. → `world.gd`
2773. Rain density comes from weather state, not cosmetic timer. → `sim/src/lib.rs`
2774. Add rain streak brightness from key/fill light. → `rama_light.gdshaderinc`
2775. Add ground wetting response synchronized with rain onset. → `soil.rs`
2776. Add water ripple response synchronized with rain onset. → `water.gdshader`
2777. Add volumetric cloud layer as broad habitat-coordinate masks. → `clouds.gdshader`
2778. Clouds move along wind and wrap around cylinder coordinates. → `clouds.gdshader`
2779. Cloud shadows use same masks at terrain scale. → `paint.rs`
2780. Cloud opacity affects sun energy slightly, not just ground colour. → `world.gd`
2781. Add valley mist in `lake_mask` depressions. → `paint.rs`
2782. Mist gathers in low wet basins at dawn. → `world.gd`
2783. Mist lifts as sun energy rises. → `world.gd`
2784. Mist thins under strong wind. → `world.gd`
2785. Mist is near/focus spectacle; far haze remains shader air. → `RENDER_CONTRACT.md`
2786. Add dawn wave along habitat axis, about 20 seconds across the visible span. → `world.gd`
2787. Dawn wave warms far wall before local ground reaches full energy. → `world.gd`
2788. Add dusk wave in reverse, settlement lights appearing behind it. → `world.gd`
2789. Add dust motes following wind in dry biomes. → `world.gd`
2790. Dust rises from paths, dry fields and herd movement. → `plant.rs`
2791. Dust density lowers in rain and wetlands. → `world.gd`
2792. Add pollen/seed drift in flowering season. → `plant.rs`
2793. Pollen is subtle and quality-gated. → `NEXT.md`
2794. Add storm front as dark broad ceiling/far-wall band. → `clouds.gdshader`
2795. Storm front sound and visible cloud arrive together. → `world.gd`
2796. Add lightning only after cloud and sound timing are deterministic. → `MODEL_LIMITS.md`
2797. Add thunder delay from visible strike distance if lightning ships. → `world.gd`
2798. Add air colour curve for clear, dusty, rainy and misty states. → `world.gd`
2799. Air colour feeds background, fog and shader haze from one source. → `world.gd`
2800. Add humidity as a visual parameter only if sim exposes it. → `FIELDS.md`
2801. Add reduced-motion setting for rain, dust and mist drift. → `world.gd`
2802. Reduced motion keeps weather legible through opacity and colour. → `RENDER_CONTRACT.md`
2803. Add low-spec weather mode: colour, fog, wetness, no particles. → `world.gd`
2804. Add high weather mode: rain curtains, mist sheets, dust and cloud shadows. → `world.gd`
2805. Keep weather particle counts capped per band. → `world.gd`
2806. Use MultiMesh for rain/dust sheets where possible. → `world.gd`
2807. Avoid thousands of particle nodes. → `world.gd`
2808. Add wind vector debug arrow to HUD. → `world.gd`
2809. Add weather state to screenshot metadata. → `world.gd`
2810. Add golden shot for clear dawn wave. → `tools/`
2811. Add golden shot for wet misty basin. → `tools/`
2812. Add golden shot for dry dust path. → `tools/`
2813. Add golden shot for rain over water. → `tools/`
2814. Add golden shot for cloud shadow on far wall. → `tools/`
2815. Add luminance histogram gate for storm scenes. → `tools/`
2816. Storm scenes must stay readable, not just dark. → `RENDER_CONTRACT.md`
2817. Add fog depth retune after real light lands. → `world.gd`
2818. Add fog depth retune after far coverage lands. → `world.gd`
2819. Add fog depth retune after biome palettes land. → `world.gd`
2820. Add weather-to-audio bus sends for rain and wind. → `world.gd`
2821. Rain on leaves only audible where plant cover exists. → `plant.rs`
2822. Rain on roofs only audible where dwellings exist. → `dwellings_lod`
2823. Wind roar follows canopy and valley shape. → `world.gd`
2824. Add insect quieting before storm. → `world.gd`
2825. Add bird quieting before storm. → `world.gd`
2826. Add cloud-edge brightening toward sun. → `clouds.gdshader`
2827. Add crepuscular suggestion through haze, not expensive shafts. → `rama_light.gdshaderinc`
2828. Volumetric fog remains optional until frame budget proves it. → `MODEL_LIMITS.md`
2829. Add cloud mask frequency comments and falloff distances. → `clouds.gdshader`
2830. Add rain streak feature-size comments. → `world.gd`
2831. Add mist sheet feature-size comments. → `world.gd`
2832. Add dust mote feature-size comments. → `world.gd`
2833. Add deterministic phase from world time for all weather visuals. → `world.gd`
2834. Freeze weather phase in golden-image mode. → `world.gd`
2835. Add save/load continuity for weather visuals if state persists. → `FIELDS.md`
2836. Do not invent forecast precision beyond sim state. → `MODEL_LIMITS.md`
2837. Add reticle/world panel "rain arriving" only if weather model supports it. → `world.gd`
2838. Add visible wind over grass before adding dramatic clouds. → `grass.gdshader`
2839. Add visible wind over water before adding dramatic clouds. → `water.gdshader`
2840. Add visible wind over trees before adding dramatic clouds. → `tree.gdshader`
2841. Keep spectacle tied to fields the player can affect. → `RENDER_CONTRACT.md`
2842. Add smoke/haze from settlements only where fuel use exists. → `FIELDS.md`
2843. Add dust from construction only where work sites exist. → `world.gd`
2844. Add breath/steam only in cold state if temperature exists. → `MODEL_LIMITS.md`
2845. Add photo-mode weather intensity slider for captures, not gameplay state. → `world.gd`
2846. Done means weather changes the ground, sound and light together. → `NEXT.md`
2847. Done means air explains distance without hiding the far wall. → `world.gd`
2848. Done means no weather effect is a disconnected overlay. → `RENDER_CONTRACT.md`
2849. Done means low-spec still gets atmosphere. → `world.gd`
2850. Done means spectacle remains honest to the sim. → `MODEL_LIMITS.md`

## BF. Life you can see (2851–2910)

2851. Add grazer herds as MultiMesh silhouettes, not individual nodes. → `world.gd`
2852. Herd positions come from sim forage and water needs. → `sim/src/lib.rs`
2853. Herd density is lower in poor biomes and drought. → `biome.rs`
2854. Grazers avoid steep cliffs and active settlements. → `sim/src/lib.rs`
2855. Grazers visibly move as groups with leader/follower offsets. → `world.gd`
2856. Grazers leave trampled grass and paths where persistence exists. → `plant.rs`
2857. Herd fear avoidance uses player/agent proximity. → `sim/src/lib.rs`
2858. Fear response is a turn and drift, not teleporting. → `world.gd`
2859. Add birds as far/focus flock strokes near water and canopy. → `world.gd`
2860. Bird density follows biome and ecosystem health. → `plant.rs`
2861. Birds lift before storms and settle at dusk. → `world.gd`
2862. Add insects as tiny near-band flickers and audio density. → `world.gd`
2863. Insects vary by biome: wetland swarm, meadow drift, alpine sparse. → `biome.rs`
2864. Add carcass scavengers where animal death is simulated. → `sim/src/lib.rs`
2865. Carcasses use existing `carcass_mm` path before new systems. → `world.gd`
2866. Scavengers appear only around carcass events. → `world.gd`
2867. Add fish surface breaks only where water ecology exists. → `MODEL_LIMITS.md`
2868. Add pollinator clusters around flowering plants. → `plant.rs`
2869. Pollinators are visual hints if pollination is simulated. → `FIELDS.md`
2870. Add night moths near settlement lights. → `world.gd`
2871. Night insects must not imply dangerous mechanics unless present. → `MODEL_LIMITS.md`
2872. Add animal silhouettes visible against far wall. → `world.gd`
2873. Use strong readable shapes before animation complexity. → `RENDER_CONTRACT.md`
2874. Add two-frame or shader-phase walk cycles for distant herds. → `world.gd`
2875. Add near-band simple leg animation for grazers if budget remains. → `world.gd`
2876. Add flock turn animation from shared phase. → `world.gd`
2877. Add perch points on trees and roofs. → `world.gd`
2878. Perch points come from visible structures, not invisible anchors. → `prop.gdshader`
2879. Add water-drinking behaviour at safe banks. → `sim/src/lib.rs`
2880. Drinking creates small ripples if water interaction is enabled. → `water.gdshader`
2881. Add grazing visual: head-down states in fields. → `world.gd`
2882. Grazing should lower local grass only if sim supports it. → `FIELDS.md`
2883. Add herd debug overlay with goal and fear vector. → `world.gd`
2884. Add animal count to ecosystem census. → `world.gd`
2885. Add golden shot for herd crossing focus band. → `tools/`
2886. Add golden shot for birds over wetland. → `tools/`
2887. Add golden shot for insects at dusk. → `tools/`
2888. Add golden shot for carcass/scavenger event if simulated. → `tools/`
2889. Cap visible animals by distance band and quality preset. → `world.gd`
2890. Batch animal MultiMeshes by archetype and palette. → `world.gd`
2891. Keep animal update tick lower than render tick where possible. → `world.gd`
2892. Interpolate animal transforms between sim ticks. → `world.gd`
2893. Do not let animals break determinism. → `sim/src/lib.rs`
2894. Add save persistence only for simulated herds, not cosmetic flocks. → `FIELDS.md`
2895. Add model-limit note for implied ecology. → `MODEL_LIMITS.md`
2896. Animal audio uses same density and position as visuals. → `world.gd`
2897. Herd bells or calls are settlement-specific only if domestication exists. → `MODEL_LIMITS.md`
2898. Add predator silhouettes only after mechanics support them. → `NEXT.md`
2899. Add player affordance hints through animal avoidance routes. → `world.gd`
2900. Show fear avoidance so the world notices the player. → `sim/src/lib.rs`
2901. Add dead-zone around camera to avoid insects crossing UI constantly. → `world.gd`
2902. Respect reduced motion for flocks and insect flicker. → `world.gd`
2903. Add animal field IDs to `FIELDS.md` before persistence. → `FIELDS.md`
2904. Add perf counter for animal transform upload. → `world.gd`
2905. Done means a still frame contains non-human life in the right biomes. → `NEXT.md`
2906. Done means moving life changes with weather and time. → `world.gd`
2907. Done means animals leave visible consequences where simulated. → `plant.rs`
2908. Done means life is legible at 60, 300 and 600 m. → `RENDER_CONTRACT.md`
2909. Done means no decorative animal lies about ecology. → `MODEL_LIMITS.md`
2910. Done means the biosphere is visible without opening the HUD. → `world.gd`

## BG. The colonist and the built world (2911–2970)

2911. Replace colonist marker with a simple articulated body. → `world.gd`
2912. Body needs head, torso, arms, legs and tool silhouette. → `world.gd`
2913. Animation can be procedural two-bone poses before imported rigs. → `world.gd`
2914. Walk cycle speed follows actual movement distance. → `world.gd`
2915. Work pose follows task type: dig, carry, plant, build, rest. → `world.gd`
2916. Colonist colour palette separates clothing from skin and tool. → `prop.gdshader`
2917. Colonist material uses the shared light contract. → `rama_light.gdshaderinc`
2918. Add gaze/aim direction toward current task target. → `world.gd`
2919. Add idle fidgets only after work/readability poses land. → `NEXT.md`
2920. Agent position and job come from sim, not animation guesses. → `sim/src/lib.rs`
2921. Add debug label that can be hidden in photo mode. → `world.gd`
2922. Add colonist selection ring respecting tilt-shift focus. → `world.gd`
2923. Add packed carried items as visible crates/bundles. → `world.gd`
2924. Carried item material comes from inventory material. → `FIELDS.md`
2925. Add paths worn by repeated colonist travel. → `world.gd`
2926. Add group work silhouettes around construction sites. → `world.gd`
2927. Build huts from material-driven modules, not generic boxes. → `world.gd`
2928. Mud, reed, timber, stone and metal modules need distinct silhouettes. → `prop.gdshader`
2929. Hut material selection follows local available resources. → `sim/src/lib.rs`
2930. Roof shape varies by climate and material. → `world.gd`
2931. Use existing `roof_mm` before adding scene nodes. → `world.gd`
2932. Town layout comes from sim dwellings, roads and work sites. → `dwellings_lod`
2933. Dwellings in focus band become hut clusters. → `world.gd`
2934. Dwellings in far band become lights and value blocks. → `paint.rs`
2935. Add lanterns at doors and paths at dusk. → `world.gd`
2936. Lantern light is emissive glow, not global exposure. → `prop.gdshader`
2937. Add window glow by occupancy/time. → `dwellings_lod`
2938. Add smoke/steam only where cooking or heating exists. → `FIELDS.md`
2939. Add storage piles using existing stockpile MultiMesh. → `world.gd`
2940. Stockpile colour and shape follow material stored. → `prop.gdshader`
2941. Add work stations with readable silhouettes, not white blocks. → `world.gd`
2942. Work station albedo must be dark under emission. → `RENDER_CONTRACT.md`
2943. Add bridges or culverts where paths cross water. → `world.gd`
2944. Bridge material follows settlement material table. → `prop.gdshader`
2945. Add fences or field markers only where ownership/crop logic exists. → `FIELDS.md`
2946. Add module silhouettes for habitat infrastructure. → `world.gd`
2947. Infrastructure modules should read by outline at 300 m. → `RENDER_CONTRACT.md`
2948. Add construction stage visuals: foundation, frame, shell, occupied. → `world.gd`
2949. Stage comes from build progress, not random decoration. → `sim/src/lib.rs`
2950. Add abandoned/ruined state only if sim can create it. → `MODEL_LIMITS.md`
2951. Add photo shot of a colonist working near a hut. → `tools/`
2952. Add photo shot of a town at dusk. → `tools/`
2953. Add photo shot of fields and settlement on far wall. → `tools/`
2954. Add material palette table for built modules. → `RENDER_CONTRACT.md`
2955. Add dwellings LOD buffer version if fields change. → `sim/src/lib.rs`
2956. Add performance budget for town prop upload. → `world.gd`
2957. Batch roofs, walls, lanterns and stockpiles separately. → `world.gd`
2958. Avoid a scene node per dwelling at far distance. → `world.gd`
2959. Add collision only for near built objects. → `world.gd`
2960. Add nav/placement respect for visible huts. → `sim/src/lib.rs`
2961. Add reticle readout for dwelling material and occupancy. → `world.gd`
2962. Add settlement census panel using same dwellings data. → `world.gd`
2963. Keep romance/social UI grounded in visible work history. → `PD_BRIEF.md`
2964. Add model-limit note for colonist animation simplicity. → `MODEL_LIMITS.md`
2965. Respect reduced motion for idle animation and lantern flicker. → `world.gd`
2966. Done means towns are sim dwellings, not painted scenery. → `NEXT.md`
2967. Done means huts reveal local material economy. → `sim/src/lib.rs`
2968. Done means a colonist's current task is readable from posture. → `world.gd`
2969. Done means dusk settlements make the drum feel inhabited. → `world.gd`
2970. Done means built forms never lie about the underlying sim. → `RENDER_CONTRACT.md`

## BH. Camera, post, diorama (2971–3020)

2971. Upgrade tilt-shift to aim-follow focus, not fixed screen centre. → `tiltshift.gdshader`
2972. Focus distance follows reticle target with damped interpolation. → `world.gd`
2973. Keep near hands and target readable during focus pulls. → `player.gd`
2974. Add focus debug overlay in photo mode. → `world.gd`
2975. Reduce blur in active digging/building to protect control. → `world.gd`
2976. Respect reduced motion by slowing or disabling focus pulls. → `world.gd`
2977. Add LUT grade after Filmic, before grain/vignette. → `tiltshift.gdshader`
2978. LUT grade should warm highlights and cool distant shadows. → `RENDER_CONTRACT.md`
2979. Keep saturation boost single-source in post. → `tiltshift.gdshader`
2980. Add golden-hour preset driven by day cycle, not manual filter. → `world.gd`
2981. Golden hour warms key and windows while preserving biome hue. → `rama_light.gdshaderinc`
2982. Add photo orbit around player/target. → `world.gd`
2983. Photo orbit keeps the drum curvature in frame by default. → `world.gd`
2984. Add rule-of-thirds guides in photo mode only. → `world.gd`
2985. Add horizon/axis guide for closed-cylinder composition. → `world.gd`
2986. Add film grain with reduced-motion/static option. → `tiltshift.gdshader`
2987. Grain is luminance-aware and subtle, matching existing post style. → `tiltshift.gdshader`
2988. Reduce chromatic aberration at UI and reticle. → `tiltshift.gdshader`
2989. Keep vignette away from critical HUD corners. → `world.gd`
2990. Add exposure compensation per time of day. → `world.gd`
2991. Add histogram debug readout in photo mode. → `world.gd`
2992. Add screenshot metadata for camera, focus, LUT and exposure. → `world.gd`
2993. Add fixed shot list: wake, near ground, focus town, far wall, storm, night. → `tools/`
2994. Add `--photo` capture that hides HUD but preserves world state. → `world.gd`
2995. Add `--shot` determinism for animated post phase. → `world.gd`
2996. Freeze grain seed for golden images. → `tiltshift.gdshader`
2997. Freeze weather and glitter phase for golden images. → `world.gd`
2998. Add camera collision smoothing around huts and cliffs. → `world.gd`
2999. Add camera minimum height over water/ground for orbit mode. → `world.gd`
3000. Add field-of-view preset for diorama view. → `world.gd`
3001. Diorama view emphasizes curvature without breaking controls. → `world.gd`
3002. Add low-FOV inspect mode for near details. → `world.gd`
3003. Add accessibility toggle for depth-of-field blur. → `world.gd`
3004. Add accessibility toggle for bloom intensity. → `world.gd`
3005. Add accessibility toggle for camera shake if any is added. → `world.gd`
3006. Add photo captions from current reticle fields. → `world.gd`
3007. Captions must cite sim fields, not prose inventions. → `FIELDS.md`
3008. Add UI-safe colour grade for overlays. → `tiltshift.gdshader`
3009. Add nighttime exposure floor so navigation remains fair. → `world.gd`
3010. Add dawn/dusk exposure transition without visible pumping. → `world.gd`
3011. Add tone-curve calibration images to docs. → `CALIBRATION.md`
3012. Avoid post hiding material mistakes; debug bypass remains one key. → `world.gd`
3013. Add compare mode: raw, lit, post. → `world.gd`
3014. Add screenshot filename tags for seed, time and preset. → `world.gd`
3015. Add panorama capture only after fixed shots are stable. → `NEXT.md`
3016. Done means screenshots look intentional before manual cropping. → `NEXT.md`
3017. Done means tilt-shift helps read scale, not hide defects. → `tiltshift.gdshader`
3018. Done means post is a single coherent grade. → `RENDER_CONTRACT.md`
3019. Done means reduced-motion players keep the same information. → `world.gd`
3020. Done means the drum photographs like a miniature world. → `world.gd`

## BI. Performance that pays for all of it (3021–3110)

3021. Retest threaded meshing after the prior codesign exit-137 failure. → `sim/build.sh`
3022. Document the exact macOS failure mode before changing threading again. → `CALIBRATION.md`
3023. Prototype `std::thread` mesher outside GDExtension first. → `sim/src/lib.rs`
3024. Codesign rebuilt dylibs with `rm` + `cp` + `codesign`, never overwrite. → `sim/build.sh`
3025. Add isolated benchmark for far-sector meshing. → `sim/src/bench.rs`
3026. Add isolated benchmark for near chunk meshing. → `sim/src/bench.rs`
3027. Add isolated benchmark for `color_at`/paint lookup. → `paint.rs`
3028. Keep `rayon` behind a feature flag until GDExtension load is proven. → `Cargo.toml`
3029. Add runtime log stating whether threaded mesher is enabled. → `world.gd`
3030. Add fallback to single-thread mesher on thread init failure. → `sim/src/lib.rs`
3031. Do not silently change determinism with threading. → `sim/src/lib.rs`
3032. Add deterministic ordering test for threaded mesh output. → `tools/`
3033. Move sim tick to worker thread only after render-thread ownership is clear. → `world.gd`
3034. Sim tick worker returns immutable snapshots to Godot. → `sim/src/lib.rs`
3035. Avoid calling Godot objects from Rust worker threads. → `MODEL_LIMITS.md`
3036. Add ring buffer for completed sim snapshots. → `world.gd`
3037. Drop stale snapshots rather than stalling frame. → `world.gd`
3038. Add SPH regional update within 512 m of player/action. → `sim/src/lib.rs`
3039. Keep far hydrology coarse unless player changes drainage. → `sim/src/lib.rs`
3040. Add dirty-region propagation for drainage changes. → `sim/src/lib.rs`
3041. Add cost counters for water, soil, plants, agents and paint. → `sim/src/lib.rs`
3042. Surface counters in frame budget overlay. → `world.gd`
3043. Add GPU-time and CPU-time split if Godot exposes it. → `world.gd`
3044. Add visible frame budget: target, sim, mesh, upload, draw, post. → `world.gd`
3045. Add warning when any system exceeds budget for 30 frames. → `world.gd`
3046. Convert plant MultiMesh updates to buffer writes where available. → `world.gd`
3047. Use `multimesh_set_buffer` equivalent for bulk transforms. → `world.gd`
3048. Avoid per-instance setter loops for thousands of grass blades. → `world.gd`
3049. Pack transform and colour data tightly for upload. → `world.gd`
3050. Split static and dynamic instances so unchanged props do not upload. → `world.gd`
3051. Move grass generation to GPU shader where field query permits. → `grass.gdshader`
3052. Keep CPU grass fallback for compatibility. → `world.gd`
3053. Add grass density texture or buffer fed by sim chunks. → `plant.rs`
3054. Add soil overlay shader instead of repainting terrain vertices every tick. → `terrain.gdshader`
3055. Soil overlay samples moisture/fertility texture. → `soil.rs`
3056. Update soil texture by dirty rectangle. → `world.gd`
3057. Move biome palette lookup out of per-vertex hot path when cached. → `paint.rs`
3058. Cache far sector colour buffers by seed, time band and dirty state. → `paint.rs`
3059. Add async far-sector rebuild queue. → `world.gd`
3060. Prioritize sectors in view direction and near horizon. → `world.gd`
3061. Rebuild one far sector per frame under load. → `world.gd`
3062. Add near chunk mesh cache keyed by chunk coordinate and edit version. → `world.gd`
3063. Evict chunks by distance and last visible time. → `world.gd`
3064. Add mid-field LOD generation budget. → `world.gd`
3065. Merge small prop batches by material and distance band. → `world.gd`
3066. Reduce shader branch divergence in terrain material selection. → `terrain.gdshader`
3067. Bake biome/material IDs into vertex channels where stable. → `paint.rs`
3068. Use textures for dynamic fields, vertex colours for static fields. → `RENDER_CONTRACT.md`
3069. Add material quality tiers: full, reduced, debug. → `world.gd`
3070. Low tier disables volumetric, reflection and high particle counts first. → `world.gd`
3071. Medium tier keeps light, biomes, water glints and ground cover. → `world.gd`
3072. High tier adds richer weather and shadows. → `world.gd`
3073. Add automatic quality hint from first 10 seconds of frame timings. → `world.gd`
3074. Never change quality silently without notifying player. → `world.gd`
3075. Add Deck preset to cap plant count and shadow distance. → `world.gd`
3076. Add CPU spike capture around digging. → `tools/`
3077. Add CPU spike capture around entering town. → `tools/`
3078. Add CPU spike capture around storm onset. → `tools/`
3079. Add GPU spike capture around far-wall view. → `tools/`
3080. Add benchmark for 12-sector far field build. → `tools/`
3081. Add benchmark for 58 near chunks plus caves. → `tools/`
3082. Add benchmark for 6400 grass instances. → `tools/`
3083. Add benchmark for 12000 plant sim population. → `tools/`
3084. Add budget table to docs with ms per system. → `CALIBRATION.md`
3085. Mark invented budgets until measured on target hardware. → `CALIBRATION.md`
3086. Keep visual waves small and reversible for profiling. → `NEXT.md`
3087. Add one toggle per new visual family. → `world.gd`
3088. Toggle names match section codes AX through BH. → `world.gd`
3089. Add capture matrix for toggles to isolate regressions. → `tools/`
3090. Add memory counter for mesh buffers and MultiMesh buffers. → `world.gd`
3091. Add allocator sanity check in Rust hot paths. → `sim/src/lib.rs`
3092. Avoid heap allocation inside per-cell paint loops. → `paint.rs`
3093. Avoid string creation inside per-frame GDScript loops. → `world.gd`
3094. Pool temporary arrays for LOD rebuilds. → `world.gd`
3095. Version buffer layouts explicitly. → `FIELDS.md`
3096. Add crash log when GDExtension method is missing, not silent fallback. → `world.gd`
3097. Keep `has_method()` guards for release degradation only. → `world.gd`
3098. Add performance selftest command. → `world.gd`
3099. Add CI smoke test for headless shot under time limit. → `tools/`
3100. Add CI check that shader includes compile after light changes. → `tools/`
3101. Add CI check for forbidden per-instance setter loops in hot paths. → `tools/`
3102. Add CI check for high bare shader constants without feature notes. → `tools/`
3103. Add profiler overlay screenshot to performance docs. → `CALIBRATION.md`
3104. Add before/after table for each wave's frame cost. → `NEXT.md`
3105. Stop a wave if visual gain costs more than its budget. → `NEXT.md`
3106. Prefer replacing fake cost with real value over stacking effects. → `RENDER_CONTRACT.md`
3107. Done means Wave 1 does not lower baseline below target FPS. → `NEXT.md`
3108. Done means the frame budget tells which system paid for each spectacle. → `world.gd`
3109. Done means threading is measured, signed and reversible. → `sim/build.sh`
3110. Done means richness is bought with architecture, not hope. → `MODEL_LIMITS.md`

## BJ. Measuring looks (3111–3150)

3111. Add golden images for fixed camera list. → `tools/`
3112. Golden images store seed, time, weather, quality, camera and light state. → `world.gd`
3113. Add perceptual diff threshold after the look stabilizes. → `tools/`
3114. Add luminance histogram for every golden image. → `tools/`
3115. Histogram gate catches navy far wall and bleached grass. → `tools/`
3116. Add far-legibility gate based on value contrast of wall regions. → `tools/`
3117. Far gate must detect fields, water, forest and settlement masses. → `tools/`
3118. Add biome Delta E contrast measurement between neighbouring IDs. → `tools/`
3119. Delta E runs after Filmic/post, not raw shader colour. → `tools/`
3120. Add colour-blind simulation pass for biome maps. → `tools/`
3121. Add water sparkle temporal stability metric. → `tools/`
3122. Sparkle metric rejects frame-to-frame white noise. → `tools/`
3123. Add grass ring edge motion metric. → `tools/`
3124. Ring edge metric rejects visible density popping. → `tools/`
3125. Add LOD shade-match metric for trees. → `tools/`
3126. Tree metric compares near and mid samples at handoff. → `tools/`
3127. Add shadow sanity image for noon and dusk. → `tools/`
3128. Shadow sanity checks direction, softness and reach. → `tools/`
3129. Add weather readability metric for storm scenes. → `tools/`
3130. Storm metric requires navigable midtone range. → `tools/`
3131. Add HUD/reticle contrast check against world backgrounds. → `tools/`
3132. Add screenshot crop panels for near, focus and far bands. → `tools/`
3133. Add human-readable report with pass/fail and thumbnails. → `tools/`
3134. Keep thresholds in `CALIBRATION.md`, not hidden in scripts. → `CALIBRATION.md`
3135. Mark subjective thresholds as provisional until playtested. → `CALIBRATION.md`
3136. Add command to capture all gates in one run. → `tools/`
3137. Add CI mode with fewer shots and lower runtime. → `tools/`
3138. Add local mode with full shot matrix. → `tools/`
3139. Add regression labels by section code AX through BI. → `tools/`
3140. Add failure advice that points to likely shader/system file. → `tools/`
3141. Add measurement for no-Light3D baseline before Wave 1. → `tools/`
3142. Add measurement after sun lands, before palette changes. → `tools/`
3143. Add measurement after far-wall coverage fix. → `tools/`
3144. Add measurement after biome palettes. → `tools/`
3145. Add measurement after water and weather. → `tools/`
3146. Never bless a screenshot that only works from one camera. → `RENDER_CONTRACT.md`
3147. Never tune by memory; compare fixed captures. → `CALIBRATION.md`
3148. Done means looks have tests without pretending taste is fully objective. → `MODEL_LIMITS.md`
3149. Done means visual regressions are cheaper to find than to argue. → `tools/`
3150. Done means screenshots become evidence, not vibes. → `RENDER_CONTRACT.md`

## BK. Sequencing (3151–3200)

3151. Wave 1 starts with light because every later colour depends on it. → `NEXT.md`
3152. Wave 1 adds `RamaSun`, shared uniforms and shader key/fill/bounce. → `world.gd`
3153. Wave 1 retunes fog only after light is visible. → `world.gd`
3154. Wave 1 adds two fixed shots: toward sun and away from sun. → `tools/`
3155. Wave 1 done means screenshots no longer look unlit. → `NEXT.md`
3156. Wave 2 paints the far wall as inhabited landscape. → `paint.rs`
3157. Wave 2 adds farmland patchwork, rivers and settlement glow. → `paint.rs`
3158. Wave 2 kills theta streaks before adding more detail. → `terrain.gdshader`
3159. Wave 2 retunes haze after coverage changes. → `world.gd`
3160. Wave 2 done means the far wall reads before HUD. → `NEXT.md`
3161. Wave 3 fixes terrain surface. → `terrain.gdshader`
3162. Wave 3 adds slope split, strata, wet-darkening and fresh cuts. → `soil.rs`
3163. Wave 3 adds reticle surface readout for debug. → `world.gd`
3164. Wave 3 done means terrain is no longer clay. → `NEXT.md`
3165. Wave 4 makes biomes legible. → `biome.rs`
3166. Wave 4 adds the nine-ID palette table and HUD census match. → `world.gd`
3167. Wave 4 adds first signature objects: reeds, flowers, thorn, lichen. → `plant.rs`
3168. Wave 4 adds Delta E and colour-blind checks. → `tools/`
3169. Wave 4 done means biome names confirm, not explain. → `NEXT.md`
3170. Wave 5 replaces repeated trees. → `plant.rs`
3171. Wave 5 builds six archetype meshes and genome species lookup. → `world.gd`
3172. Wave 5 adds plant LOD shade matching and wind coherence. → `tree.gdshader`
3173. Wave 5 adds snags and deadwood if decay state is available. → `plant.rs`
3174. Wave 5 done means no repeated cone forest dominates. → `NEXT.md`
3175. Wave 6 enriches ground and water together. → `grass.gdshader`
3176. Wave 6 adds biome grass, litter, rocks, crop rows and ring dither. → `world.gd`
3177. Wave 6 adds sparkle, foam, reflection and wet shoreline. → `water.gdshader`
3178. Wave 6 matches water audio to visible flow. → `FIELDS.md`
3179. Wave 6 done means near ground and water reward inspection. → `NEXT.md`
3180. Wave 7 adds weather, life and built-world readability. → `world.gd`
3181. Wave 7 adds rain curtains, mist, dust and dawn wave. → `clouds.gdshader`
3182. Wave 7 adds herds, birds, insects and carcass signs where simulated. → `sim/src/lib.rs`
3183. Wave 7 adds colonist articulation and material-driven huts. → `world.gd`
3184. Wave 7 done means the drum feels inhabited in motion. → `NEXT.md`
3185. Wave 8 is camera and post polish. → `tiltshift.gdshader`
3186. Wave 8 adds aim-follow focus, LUT grade, photo orbit and reduced-motion parity. → `world.gd`
3187. Wave 8 done means screenshots compose themselves honestly. → `NEXT.md`
3188. Wave 9 is performance recovery and architecture. → `sim/src/lib.rs`
3189. Wave 9 retests threading, workers, bulk buffers and dirty-region updates. → `world.gd`
3190. Wave 9 done means visual richness has measured budget. → `CALIBRATION.md`
3191. Wave 10 is measurement, gates and documentation. → `tools/`
3192. Wave 10 adds golden images, histograms, far gates and biome contrast gates. → `tools/`
3193. Wave 10 updates render contract, fields, calibration and model limits. → `RENDER_CONTRACT.md`
3194. Wave 10 done means future changes cannot silently flatten the world. → `NEXT.md`
3195. First week is Wave 1 only: light, shared uniforms, fog retune, fixed shots. → `NEXT.md`
3196. Do not start assets before light and palette contracts are stable. → `RENDER_CONTRACT.md`
3197. Do not start volumetrics before frame budget overlay exists. → `world.gd`
3198. Do not start new ecology visuals before fields can back their claims. → `FIELDS.md`
3199. Sequence rule: broad light and mass first, individual detail second. → `NEXT.md`
3200. And the rule, unchanged: **honesty of mechanism beats cosmetic spectacle; legibility beats completeness; delight in the first ninety seconds beats feature count; stated limits beat implied precision.**

---

## What to build first, out of all 1000

| Wave | Build first | Done when |
|---|---|---|
| 1 | Real sun contract: `DirectionalLight3D`, shared light uniforms, key/fill/bounce, fog retune | The same hill has a lit side, shadow side, bounce and readable haze |
| 2 | Far wall coverage: farmland, rivers, settlement glow, anti-theta-streak paint | The opposite wall reads inhabited without shimmer or navy plate |
| 3 | Terrain material read: slope split, strata, wet-darkening, fresh cuts | Ground no longer looks like smooth clay |
| 4 | Biome readability: nine-ID palette, ecotones, signature objects, HUD census match | Biomes are identifiable before labels |
| 5 | Plant individuality: six archetypes, genome species table, shade-matched LOD | Forests stop reading as repeated cones |
| 6 | Ground and water: biome grass, litter, crop rows, sparkle, foam, wet shoreline | Near band rewards inspection and water shines |
| 7 | Weather, life, settlements: rain/mist/dust, herds/birds/insects, articulated colonists, huts | The drum feels inhabited and reactive in motion |

## The shortest useful summary

Start with light, because every screenshot is currently solving around its
absence. Then make the far wall inhabited, make terrain and biomes readable, and
only then spend geometry on individual trees, ground clutter, water, weather,
animals and towns; every step must point back to the sim or say plainly that it
is only a visual convention.
