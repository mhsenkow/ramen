# 200 Ways to Make This a Landscape

**Status:** v1.0 · **Date:** 2026-09-06
**Subject:** turning Kepler Drum from a shaped surface into a coupled, material-
driven biosphere where regions act on each other.
**Companions:** `SIM_ARCH_BRIEF.md`, `REQUIREMENTS.md`, `README.md`

---

## Where we actually are

Honest diagnosis first, because most of what follows depends on it.

| Layer | State |
|---|---|
| `elev[1536×1024]` | Generated once. Ridged noise + engineered ribs + 520k erosion droplets. **Never changes again.** |
| `flux[1536×1024]` | Droplet water accumulation, normalised to 0..1. **Baked at generation, read-only, decoupled from everything.** |
| `density(p)` | Surface radius − caves − tunnels + player CSG strokes. The only volumetric truth. |
| Materials | **None.** Colour is computed per-vertex from elevation/slope/flux. There is no material field, no strata, no ore. |
| Soil | **None.** `SIM_ARCH_BRIEF.md` §3.3 specifies NPK, moisture, organic matter, microbes. Not one line exists. |
| Water | One cylinder at a fixed radius. No lakes, no levels, no flow, no volume. |
| Weather | `rama_day` global uniform driving brightness, and a cosmetic cloud band. Zero coupling to terrain. |
| Vegetation | Four static green boxes. The carbon-allocation model in §3.1 is unwritten. |
| Regional coupling | **Zero.** Nothing upstream affects anything downstream. |

So: we have a convincing *photograph* of a weathered landscape and none of the
processes that would keep it one. Everything below is about closing that gap.

### The three changes that matter most

Everything else is detail beside these.

1. **Make `flux` live.** It is currently a fossil of generation-time erosion. If
   it were recomputed from the *current* surface — including player excavation —
   then digging a trench would reroute a river, and the entire game becomes
   watershed engineering. This is the single highest-leverage item in the
   document (§C, items 39–48).
2. **Add a material field.** Density is a scalar; a landscape is not. Strata,
   regolith depth, ore, clay, gravel — mining becomes meaningful, cliffs read
   geologically, and soil gets a parent material to weather from (§A).
3. **Let Coriolis run the weather.** At ω = 0.1018 rad/s the drum's Coriolis
   parameter is f = 2ω = 0.204 s⁻¹, against Earth's ~1×10⁻⁴. Rossby number for
   a 5 m/s wind over a 1 km feature is **0.025** — overwhelmingly
   rotation-dominated. Air in this drum *cannot* flow across the cylinder; it
   organises into axial bands. That is a climate no Earth-set game can have, it
   falls out of physics we already simulate for thrown objects, and it makes
   every region's weather a function of its neighbours' (§E).

---

## A. Material substrate and stratigraphy (1–20)

*The missing layer under everything. Currently `density()` returns "solid or
not"; it should also answer "solid **what**".*

1. Add `material(p) -> u8` alongside `density(p)`, derived not stored — the same
   trick that keeps terrain free. Strata are a function of depth below the
   original (pre-edit) surface plus a few noise fields.
2. Define a starting palette of eight: regolith, sediment, clay, sandstone,
   basalt, ferrous ore, ice, structural alloy. Eight fits a nibble and covers
   every gameplay need for a year.
3. Derive **regolith depth** from slope and drainage: thin on steep ground,
   thick where the erosion pass deposited. `flux` and the elevation gradient
   already carry both signals.
4. Below regolith, place a **sediment wedge** whose thickness follows basin
   depth — deep under the alluvial flats, absent on ridges. This is what makes
   digging in a valley feel different from digging on a hill.
5. Bedrock beneath that, banded by depth with a low-frequency 3D warp so strata
   undulate rather than lying in perfect shells.
6. Give each material a **hardness**, and make dig cost scale with it: the
   levelling brush cuts regolith in one pass and bedrock in five.
7. Give each material a **cohesion**, which later drives collapse (§K) — sand
   slumps, sandstone stands.
8. Give each material an **albedo and roughness pair**, so the faceted shading
   distinguishes them without textures.
9. Ore veins as their own SDF primitive: capsules along a 3D noise flow field,
   not scattered blobs. Veins should be *followable*, which is the whole
   pleasure of mining.
10. Bias ore toward the artifact tunnels — the builders bored where the ore was.
    This retro-justifies the tunnel graph and makes exploration pay.
11. Ice lenses at depth near the endcaps, where the hull radiates most heat.
    Gives a water source that is not the river, and a reason to go to the ends.
12. **Clay where drainage stalls**: high `flux` plus low slope plus long
    residence. Clay is the material that makes soil hold water (§B).
13. A `strata_at(theta, z) -> PackedByteArray` call returning the vertical
    column, for a geologist's-eye readout in the HUD.
14. Colour cave walls by the material actually exposed rather than by a depth
    ramp. Right now a cave in sandstone looks like a cave in basalt.
15. Excavated material should **yield** its type into an inventory. Digging is
    currently pure subtraction with nothing produced.
16. Spoil: material removed has to go somewhere. Depositing it should raise
    terrain elsewhere, at reduced density (bulking factor ~1.25 for rock).
17. **Weathering converts material over time**: exposed bedrock → regolith →
    soil, at a rate set by moisture and freeze-thaw. This is the slow clock that
    makes the world feel alive between visits.
18. Structural alloy (the ribs, the hull) is undiggable and should *sound* and
    spark differently — the audio system can pitch-shift per material for free.
19. A material's **nutrient contribution** on weathering: basalt releases
    phosphorus and potassium generously, sandstone almost nothing. This is where
    §B's soil chemistry gets its inputs in a closed system.
20. Store the material palette in a hot-reloadable TOML, so designers tune
    hardness, colour and nutrient yield without a rebuild.

## B. Soil as a living grid (21–38)

*`SIM_ARCH_BRIEF.md` §3.3 specifies this and it is the layer that most games
skip. It is where farming stops being a timer.*

21. Allocate the soil grid at half the elevation resolution (768×512) as a
    struct-of-arrays: `n, p, k, organic, moisture, ph, compaction, microbes`.
    Eight f32 fields is 12.6 MB — trivial.
22. Initialise from process, not noise: nutrients from the **parent material**
    below (item 19), organic matter from accumulated `flux` deposition.
23. **Moisture is the coupling variable.** It receives rain (§D), loses to
    evaporation and transpiration, and moves laterally down the elevation
    gradient. Everything else keys off it.
24. Lateral moisture diffusion weighted by material permeability — clay holds,
    gravel drains. This alone produces boggy hollows and dry ridges.
25. Nutrient leaching: heavy rain moves soluble nitrogen downslope, which is
    why river flats are fertile and hilltops are not. Emergent, not authored.
26. **pH drifts** from parent material and rainfall; it gates which crops will
    grow and gives lime a reason to exist as a craftable amendment.
27. Compaction rises where the player walks and where machinery sits, and falls
    with freeze-thaw and root action. Paths should visibly harden.
28. Microbial biomass as a slow integrator of organic matter and moisture; it
    sets mineralisation rate, which is how organic matter becomes plant-available
    nitrogen.
29. Legume crops fix atmospheric nitrogen — the only route by which nitrogen
    enters the soil pool in a closed system without mining it. Makes rotation a
    real strategy rather than a nicety.
30. Compost and waste return: every harvest exported from a plot is nutrient
    removed. Track it, and make the return loop visible.
31. Soil temperature as a separate slow field driven by the axis light schedule
    and depth, gating germination.
32. **Erosion of soil itself**: bare wet steep soil moves downhill and is lost
    from the plot. Ground cover prevents it. This is the mechanic that teaches
    the player why hedgerows exist.
33. A soil probe tool that reads the full column at a point, as diegetic
    instrumentation (`REQUIREMENTS.md` D2).
34. Overlay views per field — moisture, nitrogen, pH, compaction — on the
    existing plan-view SubViewport. It is already a camera; give it a shader
    that samples the soil grid.
35. Soil deltas persist as sparse overrides over the derived baseline, exactly
    like `Edits` does for terrain. Never store the whole grid.
36. Nutrient **budget assertion in CI**: total habitat nitrogen invariant across
    100 simulated days (`REQUIREMENTS.md` B5). Make it a golden test, per
    ORRERY's practice.
37. Soil should be visibly different where it is good: darker, crumbier, and
    the faceted shading can carry that with vertex colour alone.
38. Digging **destroys soil structure**. Excavating a field and refilling it
    should leave subsoil on top and a multi-season recovery — a real cost to
    careless terraforming.

## C. Hydrology — routing, rivers, lakes, groundwater (39–64)

*The largest section, because water is what connects regions.*

39. **Replace baked `flux` with a live flow-accumulation pass.** Sort cells by
    elevation once, then accumulate downhill in a single sweep — O(n log n) for
    the sort and O(n) for the accumulation. On 1536×1024 this is well under a
    second.
40. Recompute flow accumulation **incrementally on excavation**: only the cells
    downstream of an edit change. Maintain a downstream pointer per cell and
    walk it.
41. Use D8 or D-infinity flow direction. D-infinity gives smoother, more
    convincing divergence on fans and costs little more.
42. **Fill depressions** properly (Planchon–Darboux or priority-flood) so
    endorheic basins are identified rather than swallowing flow silently. A
    filled depression *is* a lake.
43. Every filled depression becomes a **lake entity** with its own surface
    radius, volume, inflow and outflow — replacing the single global water
    cylinder that currently pretends every lake is at 22 m.
44. Lake level rises and falls with the balance of inflow, evaporation and
    outflow, so a dammed valley fills over days and you can watch it.
45. Lake outflow spills at the lowest point on the rim, which means damming
    changes where the river goes. This is the payoff for items 39–43 together.
46. **Render rivers as actual geometry** — a ribbon mesh along the flow path,
    width from discharge — instead of the blue tint on terrain vertices we
    currently fake.
47. River width from discharge via a hydraulic geometry relation (w ∝ Q^0.5).
    One line, and it makes tributaries visibly merge into something larger.
48. River **audio** scaled by local discharge: the existing procedural noise
    generator already produces filtered noise; loop it, filter by width.
49. Groundwater as a second, slower grid: recharge from soil moisture, flow
    down the water-table gradient, discharge at springs where the table meets
    the surface.
50. **Springs where a cave intersects the water table** — instantly justifies
    both cave systems and gives underground exploration a reason.
51. Dig below the water table and the hole floods. This is the single most
    satisfying consequence excavation could have, and the pieces exist.
52. Well construction as a module: siting depends on the water-table depth the
    player can only learn by probing.
53. Seasonal discharge variation driven by the rainfall schedule (§D), so rivers
    swell and shrink and floodplains flood.
54. **Flood events** when discharge exceeds channel capacity: water spreads over
    the mapped floodplain, deposits silt, and enriches soil. Destructive and
    beneficial at once, which is the correct relationship.
55. Sediment carried in suspension proportional to discharge and slope, deposited
    where velocity drops — this is what actually builds the alluvial flats the
    player farms on (couples to §F).
56. Deltas where rivers meet lakes: deposition fans that grow over play-time.
    Slow, visible, and free once §F exists.
57. Bank erosion that migrates channels laterally over long timescales, leaving
    oxbows and terraces.
58. Evaporation rate from lake surface area, temperature and wind — the loss
    term that makes the water budget close (§J).
59. Water temperature as a field, driving where ice forms near the cold endcaps.
60. Snow accumulation on high ground when soil temperature is low enough, and
    **melt as a spring pulse** in the discharge record.
61. Irrigation channels as a player-dug feature that the flow router honours
    automatically — dig a ditch from a river to a field and water arrives,
    because the router only knows about the surface.
62. Waterlogging as a soil failure state: too much moisture and roots suffocate.
    Drainage becomes a thing the player builds.
63. A **catchment overlay**: highlight every cell draining to the point under
    the cursor. This is the single most illuminating instrument the game could
    have, and it is a flood-fill on the downstream pointers.
64. Publish the hydrology's limits: no true unsaturated-zone physics, no
    Manning's-equation channel routing, daily timestep. Per ORRERY's rule 4.

## D. Weather as engineered infrastructure (65–86)

*The setting's gift: weather here is not natural, it is a machine somebody runs
and the player can seize.*

65. Model **condensers as placed entities** with a position, a power draw and a
    radius of effect. Rain is their output, not a global scalar.
66. Humidity as a coarse field (192×128) advected by wind, sourced by
    evaporation, sunk by condensation at the condensers.
67. Condensation requires humidity **and** a cold surface: the condenser
    provides the cold, the habitat provides the humidity. Run condensers in a
    dry band and you get nothing.
68. **Rain falls from the cloud band radius**, not from a shader. Cloud opacity
    should read the humidity field rather than a noise function.
69. Rain intensity map accumulates into soil moisture per cell, so the ground
    under a working condenser is genuinely wetter.
70. **Rain drifts.** A droplet falling ~250 m at ~7 m/s terminal velocity
    experiences Coriolis acceleration 2ωv ≈ 1.4 m/s²; sideways terminal drift is
    roughly v_fall × (a/g) ≈ 1.1 m/s over ~36 s — **tens of metres spinward**.
    Site your condenser upwind of the field you mean to water. Compute it with
    the throw integrator we already have.
71. The **power budget is the constraint**: reactor output is finite, and every
    condenser, every lamp hour, every heater comes out of the same pool.
72. Daylight length becomes a **policy the colony votes on** rather than a
    constant — long days grow more but cost power and dry the soil.
73. Light intensity as a settable schedule with a ramp, so dawn and dusk are
    decisions with a cost.
74. Heaters and radiators along the hull setting a baseline temperature field;
    the endcaps radiate and are permanently cooler.
75. **Temperature gradient along the axis** as a first-class field: ends cold,
    middle warm. This alone produces different biomes at different z without
    authoring any.
76. Fog when humidity is high and temperature drops — cheap, atmospheric, and
    an honest consequence of two fields already present.
77. Frost events when soil temperature drops below zero: crop damage, and a
    reason to watch the schedule.
78. Wind driven by temperature differences between habitat regions, then
    deflected hard by Coriolis (§E).
79. Dust when soil is dry, bare and windy — the existing dust motes become a
    readout of soil condition rather than decoration.
80. A **weather forecast console**: because the weather is a machine, the
    forecast is deterministic, and the player can genuinely predict it. That is
    a much better fantasy than gambling on a random number.
81. Storms as a failure mode: condenser stack over-running, humidity spiking,
    heavy rain triggering the flood path in item 54.
82. Maintenance and failure: a condenser that has run at full power for a
    hundred days breaks, and the region downwind of it dries out.
83. Historical weather log per region, so the player can look up why a harvest
    failed rather than guessing.
84. Atmospheric pressure: at ω = 0.1018 rad/s the gradient from hull to axis is
    only ~5 %, and ~2.3 % over the 235 m of relief. **Do not fake thin air on
    mountaintops** — state the limit instead. Honesty of mechanism over
    spectacle.
85. Air composition as habitat-scale scalars (O₂, CO₂, N₂) that vegetation and
    colonists actually move, tying weather to the closed system in §J.
86. CO₂ enrichment as a real agricultural lever, because in a sealed drum you
    *can* raise it — and it costs you elsewhere.

## E. Coriolis climate — the drum's signature (87–100)

*Nothing else in the document is as distinctive as this, and it is nearly free
because the maths is already in the throw integrator.*

87. Apply `−2Ω × v` to the wind field. With f = 2ω = 0.204 s⁻¹ against Earth's
    1×10⁻⁴, this is not a correction — it is the dominant term.
88. Rossby number for a 5 m/s wind over a 1 km feature is **0.025**. The
    consequence is stark: **air cannot cross the drum.** Flow organises into
    bands running along the axis.
89. Therefore weather comes in **axial stripes**. Rain bands are long thin
    ribbons running the length of the habitat, and neighbouring stripes can have
    completely different climates.
90. Which means a region's weather is set by its *circumferential* neighbours,
    not its axial ones — the exact opposite of Earth intuition, and a genuinely
    novel thing to learn.
91. Ekman-like surface friction turning the wind slightly across the bands,
    which is what lets any cross-drum transport happen at all.
92. Cloud bands should therefore be **stripes, not patches** — a direct fix to
    the current noise-based cloud shader, and more physically honest.
93. Spinward and antispinward become real directions with real asymmetry. Give
    them in-fiction names the colonists use.
94. Thrown, dropped and fired objects already curve; make **falling rain, dust,
    smoke and spores** use the same integrator so the whole world agrees.
95. Long-range projectile and ballistic delivery becomes a genuine skill, since
    the deflection is large and deterministic.
96. Coriolis on rivers is negligible at these speeds — say so in the model-limits
    doc rather than adding a term nobody can see.
97. Habitat spin-rate as a *changeable parameter*: spin the drum up and gravity,
    weather banding and Coriolis all shift together. An enormous late-game lever.
98. Show the banding on the habitat map as a wind overlay — the unrolled
    projection is the natural place for it, since bands become straight lines.
99. Teach the banding diegetically: a colonist agronomist who explains why the
    farm two hundred metres spinward gets rain and yours does not.
100. Aircraft, balloons and anything airborne must obey the same field, which
     makes flight in the drum a distinctive problem rather than a reskin.

## F. Live erosion and sediment transport (101–118)

*Erosion currently runs 520,000 droplets once and then stops forever. The
landscape is a fossil of a process rather than a product of one.*

101. Run a **slow continuous erosion tick** on the elevation grid — a few
     thousand droplets per in-game day, not half a million at once. The terrain
     keeps changing while you live in it.
102. Weight droplet spawning by the actual rainfall map (§D), so erosion happens
     where it rains rather than uniformly.
103. Couple erosion strength to **material hardness** (§A): the same rainfall
     carves regolith and barely touches basalt, which is what produces cliffs,
     benches and hard-rock waterfalls.
104. Couple it to **ground cover**: vegetated slopes erode an order of magnitude
     slower. Clear a hillside and watch it gully.
105. Track eroded mass as **actual sediment**, not just a height decrement, so
     it can be carried, deposited and accounted for.
106. Deposit sediment where flow velocity drops — fans at slope breaks, silt in
     slack water, deltas at lakes.
107. Sediment **fills lakes and reservoirs** over long timescales, which gives
     dams a maintenance cost and a lifetime.
108. Thermal erosion (talus): slopes above the material's angle of repose slump
     toward it, producing scree at cliff feet.
109. Freeze-thaw shattering near the cold endcaps, converting bedrock to
     regolith faster there — a regional difference that emerges rather than
     being painted.
110. Undercut collapse: erode the base of a slope and the material above it
     fails. Ties directly into §K's excavation collapse.
111. Knickpoint retreat, so waterfalls migrate upstream through soft rock and
     stall on hard — the mechanism that makes river profiles look right.
112. Erosion must **re-mesh affected chunks** through the same path player edits
     already use (`rebuild_around`), so there is one code path for terrain
     change.
113. Bound the whole thing with a per-frame time budget and a dirty-chunk queue,
     as `SIM_ARCH_BRIEF.md` §5.3 requires. Erosion is never allowed to spike a
     frame.
114. Keep it deterministic: seeded per-region droplet streams, so the same
     habitat erodes the same way in a replay (`REQUIREMENTS.md` B8).
115. Only tick erosion at T0 and T1 detail; at T2 apply an aggregate statistical
     lowering so the far side still ages.
116. Expose an **erosion overlay** showing net gain and loss per cell — the
     player should be able to see where their land is going.
117. Terracing and contour bunding as buildable modules that measurably reduce
     the local erosion rate. Real agronomy, real mechanic.
118. Publish the limits: droplet erosion is a toy of fluvial geomorphology; it
     produces plausible drainage networks, not correct ones.

## G. Biomes as emergent state, not labels (119–136)

*Right now `biome_map` is an if-else on elevation, slope and flux. A biome
should be a consequence, never a category.*

119. **Delete the classifier.** Replace it with a Whittaker-style lookup on the
     two variables that actually decide vegetation: moisture and temperature —
     both of which §B and §D now produce.
120. Add soil depth and pH as third and fourth axes, so a wet warm place on bare
     rock is not a forest.
121. Biome should be **read out, not stored**: derive it from field state at
     query time, exactly as material and terrain already are.
122. Which makes biomes **shift on their own** as climate changes. Turn the
     condensers off in one band and watch it convert to scrub over seasons.
123. Ecotones — gradients rather than boundaries — because the underlying fields
     are continuous. No polygon edges anywhere.
124. Give each biome an **indicator species** the player can learn to read:
     seeing a particular sedge means the water table is high here.
125. Habitat map colouring should sample the real fields, so the map is a
     genuine instrument rather than a decorative legend.
126. Biome-appropriate ambient audio, generated by filtering the existing noise
     source differently per biome.
127. **Riparian corridors** as their own emergent biome — narrow, wet, following
     the river geometry from §C. The most distinctive-looking places in the
     world, for free.
128. Wetlands where drainage stalls and clay has accumulated: high biomass, poor
     access, valuable resources.
129. Rain-shadow drylands on the antispinward side of high ground, given the
     banded wind of §E.
130. Alpine zone on the high ground, defined by temperature rather than an
     elevation threshold.
131. Bare rock and scree as legitimate biomes with their own sparse life, not
     merely "not vegetation".
132. Cave biomes: lightless, humid, with fungal and detritus-based food webs
     fed by what washes in from above.
133. **Human-modified biome** as an explicit state — farmland is a biome the
     colony maintains against succession, and it reverts if abandoned.
134. Succession: cleared land goes bare → weedy → scrub → mature over seasons.
     Abandonment should be visible.
135. Track disturbance history per cell so succession has a clock to run on.
136. A biome-transition log the player can read: "band 14 shifted from grassland
     to scrub over the last thirty days" — legibility per ORRERY's rule 2.

## H. Vegetation: the carbon model, finally (137–154)

*`SIM_ARCH_BRIEF.md` §3.1 specifies this in detail and it remains unwritten. It
is roughly three hundred lines and it is the heart of the whole design.*

137. Implement source–sink carbon allocation: ten floats per plant — carbon
     pool, biomass in root/leaf/stem/reproductive, water status, nitrogen
     status, age, stress accumulator, genome reference.
138. Photosynthesis from **light actually received**, which in a drum means the
     axis strip's schedule, shading from terrain, and shading from neighbours.
139. Allocation by demand, so a shaded plant goes leggy reaching for light with
     nobody having written a rule that says so. This is the proof the model
     works — make it the first test.
140. Self-thinning in crowded stands: the losers were out-shaded, not flagged.
141. Drought stress aborting fruit to save the root, which is exactly what real
     plants do under water deficit.
142. Nutrient limitation producing small pale thick leaves rather than a
     yield multiplier applied at harvest.
143. Parametric **L-system geometry driven by the simulated state**: internode
     count from stem biomass, leaf area from leaf biomass. Morphology becomes a
     readout of history.
144. A small number of morphology bins with instanced rendering, not a unique
     mesh per plant. Generate geometry only for T0 plants in view.
145. Root depth as a real variable that determines which soil layer a plant
     draws from — deep-rooted species survive drought that kills shallow ones.
146. Transpiration returning water to the humidity field, closing the loop back
     to §D. Forests make their own rain, even here.
147. Litterfall feeding soil organic matter (§B), which is how a mature stand
     builds the soil it stands on.
148. Seed dispersal by wind, using the banded Coriolis field — plants spread
     *along* bands far more readily than across them, which shapes the whole
     ecology.
149. Crop genetics as a small trait vector with recombination and mutation, and
     **cosmic radiation raising the mutation rate** — the game's founding premise
     applied to the plants (`SIM_ARCH_BRIEF.md` §3.4).
150. Selective breeding as a long-horizon mechanic that rhymes with the Crèche.
     One mechanic, two scales, no dialogue needed to make the point.
151. Trait trade-offs so no strain is strictly best: yield against hardiness,
     maturation speed against nutrient demand.
152. Perennials versus annuals as a genuine strategic choice about soil
     stability and labour.
153. Pests and disease as populations with their own dynamics, favoured by
     monoculture — which gives polyculture a mechanical reason to exist.
154. Demote plants to statistics outside T0 and re-instantiate them
     deterministically on return, per the promotion/demotion contract in
     `SIM_ARCH_BRIEF.md` §1.

## I. Regional coupling — how places affect each other (155–170)

*This is what the user actually asked for. Everything above is machinery; this
section is the point.*

155. Define **named regions** as watershed catchments derived from the flow
     router, not as painted zones. A region is "everywhere that drains to here",
     which is the only definition that means anything.
156. Regions therefore have a genuine **upstream and downstream**, and the
     relationship is computed, not authored.
157. Upstream deforestation increases downstream flood peaks and silt load.
     Clear a hillside and someone else's field silts up.
158. Upstream irrigation withdrawal reduces downstream flow. Water becomes a
     shared, contested, finite thing.
159. Upstream soil loss becomes downstream soil gain — nutrient wealth
     physically moves between regions and the total is conserved.
160. Upstream mining tailings contaminate downstream water and soil, with a
     decay time long enough to matter.
161. **Rain shadow**: high ground in one band changes what the next band
     downwind receives (§E), so an alpine region creates a dryland neighbour.
162. Terrain modification alters the wind field, so building a ridge changes
     someone's weather.
163. Lake surface area drives evaporation, which raises regional humidity, which
     changes rainfall — a genuine feedback loop with the sign the real one has.
164. Vegetation cover changes albedo and evapotranspiration, changing local
     temperature. Forests are cooler and wetter, and it emerges.
165. Pollen, spores and seeds move along bands (item 148), so ecological
     invasion follows the wind, not the map.
166. Fauna migrate between regions on resource gradients, coupling populations
     that never touch directly.
167. A **region inspector** showing inflows and outflows: water in, water out,
     nutrient in, nutrient out, energy in. The instrument that makes coupling
     legible.
168. A dependency graph view: which regions are upstream of the one you are
     standing in, and which depend on it.
169. Cumulative-impact readout, so the player can see that this valley is dry
     *because* of what they did two hundred metres spinward.
170. Because the drum is closed and small, **every region is downstream of every
     other one eventually.** Say this out loud in the fiction — it is the
     colony's central political fact.

## J. The closed system: budgets and conservation (171–180)

*Conservation of mass is the cheapest possible source of conviction, and this
setting hands it to us.*

171. A habitat-scale ledger for water, nitrogen, phosphorus, potassium, carbon
     and energy. Twenty-odd scalars, ticked daily.
172. Every transaction moves mass between pools; nothing is created. Enforce it
     with an assertion, not an intention.
173. **CI test: total nitrogen invariant across 100 simulated days**
     (`REQUIREMENTS.md` B5), run as a golden test on every commit.
174. Losses are the interesting part: water locked in ice, nutrients buried
     below the root zone, carbon sequestered in peat. Make them recoverable at a
     cost.
175. The **Crèche competes for the same budget** as agriculture — every colonist
     is nutrients that are not in the soil. This is where §B meets `PD_BRIEF.md`.
176. Energy as the master constraint: reactor output caps lighting, condensers,
     heating and industry simultaneously.
177. A budget dashboard in the habitat survey view, next to the biome census
     already there.
178. Trend arrows over the last N days, because a slowly falling phosphorus
     stock is the kind of thing a player must be able to notice.
179. Alarm thresholds with in-fiction warnings from the colony's own systems.
180. Terminal failure states that are *slow and legible*: the colony does not die
     suddenly, it runs down, and you can see it coming for fifty days.

## K. Player impact: excavation as terraforming (181–190)

*We already have a working CSG excavation system. It currently changes nothing
but geometry.*

181. **Excavation must update the elevation grid**, not only the density field.
     Until it does, digging cannot affect drainage, and drainage is everything.
182. Then re-run flow accumulation downstream of the edit (item 40) — dig a
     trench and the river genuinely moves.
183. Dig below the water table and the hole fills (item 51). This is the single
     most convincing consequence available to us.
184. Cut a hillside and destabilise it: material above the angle of repose slumps
     into the excavation over days (item 108).
185. Excavated spoil must be placed somewhere, with a bulking factor — you
     cannot make material disappear in a closed system.
186. Removing regolith exposes bedrock, which then weathers back to regolith over
     a long clock. Damage is recoverable but slow.
187. Levelling for a building pad **compacts the soil under it** and destroys
     structure — building has an agronomic cost.
188. Terracing a slope reduces its erosion rate measurably, so the levelling
     brush becomes an agricultural tool rather than only a construction one.
189. A **before-and-after comparison** on the habitat map, so the player can see
     the cumulative shape of what they have done to the drum.
190. Long-horizon consequence: a century after the player's excavation, the
     drainage they created is simply the way the landscape is. Show it in the
     endgame.

## L. Rendering the material world (191–196)

191. Vertex colour by **material** rather than by elevation-and-slope rules, so
     strata read as strata in every cut face.
192. Triplanar detail overlays per material — grain, bedding, blockiness — while
     staying inside the faceted low-poly language.
193. Wetness darkening driven by the soil moisture field, so the landscape
     visibly responds to rain within minutes.
194. Snow, frost and dry-season colour shifts driven by the same fields, not by
     a season index.
195. Vegetation density and colour instanced from the biome and biomass fields,
     so what you see is what the simulation holds.
196. A **material legend** in the survey view mapping colours to materials, and
     honest about which are inferred rather than simulated.

## M. Instrumentation, legibility and honesty (197–200)

*ORRERY's prioritisation rule, applied: honesty of mechanism over spectacle,
legibility over completeness, delight in the first ninety seconds, and stated
limits over implied precision.*

197. **Every yield, failure and change carries its causal chain.** "−22 %:
     nitrogen deficit from day 34." Store the reason, not just the number — it
     is also the best debugging tool the project will have.
198. Overlays for every simulated field, reachable from the plan view that
     already exists: moisture, nitrogen, temperature, flow, erosion, biomass,
     catchment. Oxygen Not Included's overlay system is the model.
199. A published **model-limits document** stating plainly what the biosphere
     does not claim: no unsaturated-zone physics, no channel hydraulics, toy
     fluvial geomorphology, traits not genomes, six-scalar atmosphere. It buys
     enormous credibility and costs one page.
200. **A determinism lint and golden tests in CI from the first commit** — not
     as an aspiration but as a gate, exactly as ORRERY runs `determinism-lint`,
     `golden`, `parity` and `calibrate`. Every field registered in a curated
     schema with owner, unit and save status, so the save-migration requirement
     stays survivable.

---

## Suggested order of attack

The dependency chain matters more than the list order.

| Wave | Items | Why first |
|---|---|---|
| **1** | 39–45, 181–183 | Live flow routing + excavation updating elevation. Unlocks the entire premise; everything downstream needs it. |
| **2** | 1–10, 21–25 | Material field and soil grid. The substrate every later system reads. |
| **3** | 137–142 | The carbon-allocation plant model, headless first. One week, and it tells you whether the game works. |
| **4** | 65–71, 87–92 | Condensers and Coriolis banding. Weather becomes a machine the player operates. |
| **5** | 101–107 | Live erosion, now that materials and rainfall exist to drive it. |
| **6** | 119–124, 155–160 | Emergent biomes and regional coupling — which by this point mostly *fall out* rather than needing to be built. |
| **7** | 197–200 | Instrumentation and the honesty gates. Before content, not after. |

**The one-week test:** implement items 39, 40 and 181 alone — live flow
accumulation, incremental update, and excavation writing to the elevation grid.
Then dig a trench across a slope and watch the stream move. If that feels as
good as it should, the other 197 items are worth building.
