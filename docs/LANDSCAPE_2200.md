# 200 More: The Craft — Rendering Discipline, Debugging, UI, Audio, Shipping

**Status:** v1.2 · **Date:** 2026-09-07 · **Items 2001–2200**
**Continues:** `LANDSCAPE_200/800/1400/2000.md`
**Subject:** not what to build — how to build it without breaking it.
**Companion:** `RENDER_CONTRACT.md` · `INTENT_STRINGS.md`

---

## Why this register exists

The previous four documents are about *systems*. This one is about the twenty
hours that went into making those systems look and behave correctly, and every
item in §AR and §AS is a bug that actually happened here, with its cause.

Measured state at time of writing:

```
far field       : 1003520 tris in 12 sectors, 159 ms
near chunks     : 58  (215526 tris, caves included)
routing rerouted: 215 cells changed downstream · signature CHANGED
grass           : 3142 tufts (12568 tris), 1 draw call
mid field       : 180000 tris, 1 draw call
```

### The five bug families that cost the most

Every visual defect in this project so far has been one of these. Recognising
the family is most of the fix.

1. **sRGB values fed to a linear `ALBEDO`.** A palette authored as `(0.14,
   0.32, 0.11)` reads as pale sage, not deep green. This bleached the entire
   world, then bleached the spoil heaps, then blackened the water, then
   whitened the craft stations, then flattened the trees — five separate
   discoveries of *one* mistake.
2. **A detail feature with no distance falloff.** Terrain micro-relief, endcap
   hull panelling, the lake mask, plant billboards, pool quads. Each looked
   right up close and aliased into blocks, speckle or streaks at range.
3. **A tuned constant coupled to a parameter that later changed.** `prox` had
   `900.0` baked in from when the drum was 600 m; growing the drum crushed all
   terrain lighting to near-black and looked exactly like a normals bug.
4. **`use_colors` defaulting instances to opaque white.** Any MultiMesh that
   sets `instance_count` at build but assigns colours on refresh renders white
   until that refresh runs.
5. **Unshaded `StandardMaterial3D` + `vertex_color_use_as_albedo` + default
   white `albedo_color`.** Renders props as flat bright blocks with no lighting
   at all — and every static check says "has material, has colour".

### Implementation ledger (v1.2)

| Range | Shipped in tree | Notes |
|---|---|---|
| 2001–2050 | yes / mostly | Shared includes, CI gates, falloffs, water contract |
| 2051–2057 | yes | `--bisect` harness |
| 2058–2081 | mostly | wake skip, selftest, shots + JSON metadata; golden images open |
| 2086–2130 | mostly | HUD density, font scale, CB palettes, contours, contrast CI |
| 2131–2160 | mostly | water beds, reverb, steal, buses/sliders, captions, quiet, wrap |
| 2161–2185 | mostly | `--playtest` CSV, first-run settings, quit-save, camera resume, away blurb |
| 2186–2193 | mostly | craft + contrast CI; persist fixtures; intent-strings stub |
| 2194–2195 | yes | Deck + low-spec export presets + runtime quality |
| 2196 | stub | `docs/INTENT_STRINGS.md` |
| 2197 / 2199 | deferred | external content review / Archive SKU |
| 2198 | yes | public `site/model-limits.html` |
| 2200 | yes | unchanged rule |

---

## AR. Rendering discipline (2001–2050)

2001. **One colour-space contract, written down.** Palettes are authored sRGB;
      every shader converts with `pow(c, 1.95)` before lighting. No exceptions.
      → `docs/RENDER_CONTRACT.md` + `srgb_to_linear()`.
2002. A single shared include for that conversion, so it cannot drift per
      shader. → `game/shaders/rama_color.gdshaderinc` / `rama_light.gdshaderinc`.
2003. Any new shader that touches a palette gets reviewed against 2001 or it
      will repeat the bleach. → CI `tools/check_shader_includes.py`.
2004. **Every detail term has a distance falloff.** No exceptions: noise,
      panels, hashes, masks, billboards, per-cell quads.
2005. State the feature size of each detail term in a comment, and set the
      falloff so it retires before it goes sub-pixel. → `// feature:` tags.
2006. A rule of thumb: a feature of size `s` metres goes sub-pixel at roughly
      `s × screen_width / (2 × tan(fov/2) × 1)` metres. Write the number down.
2007. **Never bake a metre value that depends on habitat parameters.** Scale by
      `hab_radius`, `hab_length` or `max_elevation` instead.
2008. A grep gate in CI for bare float literals above 100.0 in shader files.
      → `tools/check_shader_literals.py` (allowlists uniforms, hab_*, feature notes).
2009. Any MultiMesh that sets `instance_count > 0` at build must also set
      transforms and colours there, even if only to zero and transparent.
2010. A CI check for that pattern — it is statically detectable and it has
      already produced three separate white-block bugs.
      → `tools/check_multimesh_init.py`.
2011. Instanced props use the shared `prop.gdshader`, never a bare
      `StandardMaterial3D`.
2012. Prop instance colours are authored in *albedo* range (0.05–0.35), not
      display range (0.5–0.7), or the shared energy blows them out.
2013. Document that range where the colours are defined. → RENDER_CONTRACT.
2014. Every shader that lights anything shares one light contract: axis
      direction, warm key, cool fill, bounce, haze. → `rama_light.gdshaderinc`.
2015. A shader that opts out of the contract must say why in a comment.
      → endcap documents structural opt-out.
2016. **Baked AO belongs in vertex alpha** and only on tiers that can afford
      it. Far tiers approximate from slope, which has no frequency to alias.
2017. AO multiplies indirect strongly and direct lightly, which is how real
      occlusion behaves.
2018. LOD tiers must shade *identically* at their handoff or the seam is
      visible regardless of how wide the crossfade is.
2019. Which means a new tier needs its lighting matched before its geometry is
      tuned.
2020. Dither-discard, not alpha blend, for LOD crossfades — no sorting cost, no
      overdraw. → `rama_dither_far` / `rama_dither_near`.
2021. State which direction a fade runs. `near_fade` hides a tier *closer* than
      a range; `far_cut` retires it *beyond* one. Getting this backwards is a
      silent full-screen discard.
2022. Distance-fade parameters live next to the tier they belong to, not in a
      global constants block where they drift out of agreement.
2023. **Never set `custom_aabb` around a node's own origin** when its instances
      are placed in world space far from it. That culled the entire grass field.
2024. Prefer letting Godot compute the AABB unless there is a measured reason
      not to.
2025. Post-processing is where global contrast lives. A flat-looking image is
      usually a missing tone curve, not a missing effect.
2026. A filmic S-curve about mid-grey, a print-black lift, grain weighted to
      shadow, and lateral chromatic aberration scaled by blur. Small, and most
      of what separates "rendered" from "photographed".
2027. Saturation belongs in one place. Boosting it in three shaders and again in
      post is how a world goes neon.
2028. Greens specifically want desaturating — real vegetation is olive, never
      emerald.
2029. Aerial perspective is **additive inscatter**: it lifts distant blacks. A
      haze implemented as a tint crushes them instead. → `rama_air_toward`.
2030. Haze colour is a real colour with real value, not a near-white. White haze
      reads as overexposed snow.
2031. Haze brightens toward the light source, because that is where the air is
      scattering from.
2032. Blend the **top two** palette rules, never a weighted average of all
      matches — averaging distinct hues walks everything toward grey.
2033. Blend those two linearly. Squaring the weight makes cells *flip* at
      thresholds and speckles every slope where two materials meet.
2034. Flat-shaded geometry needs per-face variation or large surfaces read as
      one colour. Hash the normal with a coarse position.
2035. And multi-scale mottling on top of that, distance-faded per 2004.
2036. Micro-relief perturbs the *shading* normal, not the geometry. Free
      surface detail, and it must retire before its noise goes sub-pixel.
2037. Wind sway amplitude scales with the **square** of height, so trunks stay
      planted and crowns move.
2038. Sway phase comes from world position, so a stand moves as one gust rather
      than each instance wobbling alone.
2039. Foliage lighting **wraps** — thin translucent leaves have no hard
      terminator. A clipped `N·L` makes leaf cards read as plates.
2040. A canopy gradient dark-at-base to bright-at-crown is the cheapest possible
      vegetation AO and does most of the work.
2041. Per-instance yaw is not optional. Without it any instanced scatter reads
      as one asset stamped on a grid — true of grass, trees and rocks alike.
2042. Per-instance scale and lean jitter on top, from a deterministic hash of
      position so it survives a reload.
2043. Ground cover is one MultiMesh, one draw call, shadows off, rebuilt only
      when the player has actually moved.
2044. Tapered blades, not rectangles. A rectangle reads as a card; a taper reads
      as grass.
2045. Emissive surfaces need a **dark albedo**. Near-white albedo plus emission
      plus bloom is a featureless white block.
2046. Emission colour is warm and specific; emission energy under 1.0 unless
      something is genuinely incandescent.
2047. Water is Beer-Lambert absorption, not a two-colour depth ramp. One `exp()`
      grades continuously and explains why deep water goes blue-green.
2048. Water surface normals come from **analytic derivatives** of the wave sum,
      not from a sine pushed into a normal.
2049. Three wave octaves at different headings, or the surface reads as
      corduroy.
2050. A global water sheet must be clipped to where water actually is. Painting
      every square metre below the waterline turns half a closed habitat blue.

## AS. Debugging methodology (2051–2085)

*Written after four consecutive wrong hypotheses about one white square.*

2051. **Bisect before theorising.** Hiding node groups and measuring found the
      craft-station bug in three renders; four rounds of reading code found
      nothing.
2052. Keep a reusable bisect harness in the project, not reinvented each time.
      → `game/scripts/debug/bisect.gd`.
2053. The harness renders variants in **one** run and prints a text verdict, so
      the answer costs one command rather than five screenshots to eyeball.
      → `Godot --path game -- --bisect`
2054. Choose a metric that can actually see the artifact. Brightness threshold
      for a white block; **luminance variance** for a low-contrast pattern like
      the endcap checkerboard. → `--bisect-metric=bright|var`
2055. Calibrate the metric first — find the brightest or noisiest region
      automatically rather than guessing screen coordinates.
2056. A baseline reading of zero means the metric is wrong, not that the bug is
      absent.
2057. Verify the harness sees the artifact at baseline before trusting any
      variant.
2058. **The wake fade covers the first two seconds** and only skips itself for
      `--shot`. Any new capture mode must clear it or every frame is black.
      → `--bisect` and `--shot` both clear wake.
2059. Godot's stdout is block-buffered: a killed process loses everything.
      `--log-file` is the only reliable capture.
2060. Which means a hang looks identical to a crash unless you use it.
2061. A GDScript parse error leaves Godot sitting on an empty scene **forever**.
      A hang is almost always a parse error.
2062. `class_name` resolution depends on `.godot/global_script_class_cache.cfg`.
      Deleting the cache silently breaks every global class. Use `preload`.
2063. Deleting `.godot/` while a `.gdextension` file is moved aside regenerates
      an extension list without it, and the class vanishes with no error that
      names the cause.
2064. **Overwriting a `.dylib` in place invalidates its ad-hoc code signature**
      and macOS SIGKILLs the loading process with zero output. `rm` + `cp` +
      `codesign --force --sign -`. This is why `build.sh` exists.
2065. Exit 137 with no output is that, or a corrupt import cache, or a resource
      kill — in that order of likelihood.
2066. `rayon` inside the GDExtension is fatal in this environment while being
      fine in a standalone binary. Measure before assuming a library is safe in
      both.
2067. Grep the actual `#[func]` list before writing a binding. Three guessed
      names cost a debugging cycle that ten seconds of grep would have avoided.
2068. `has_method()` guards hide those mistakes rather than surfacing them —
      the HUD showed em-dashes for a week instead of failing loudly.
2069. Prefer failing loudly on a missing binding during development.
2070. A metric that cannot distinguish two causes will mislead. Endcap variance
      could not separate checkerboard from silhouette edge; the eye could.
2071. Confirm a fix by the same measurement that found the bug, then confirm it
      again by looking.
2072. When a fix does not change the symptom, the hypothesis is wrong — do not
      tune the fix.
2073. Record eliminated candidates. Half the cost of the white square was
      re-checking things already ruled out.
2074. State confidence honestly in findings. "Confirmed by test" and "I think"
      are different claims.
2075. A benchmark that isolates the core from the engine is worth its weight —
      `bench.rs` found the 3.5× density hotspot in one run.
2076. Profile before optimising: the far field's cost was `color_at` per vertex,
      not the AO I had just added.
2077. Removing an invisible cost is still correct even when it is not the
      bottleneck.
2078. Keep a determinism test that would notice if a debugging change leaked
      into the sim.
2079. Selftest exit code and error count are the gate before any visual claim.
2080. Screenshots are evidence; render them at a fixed camera so changes are
      comparable.
2081. A fixed shot list doubles as a visual changelog.
2082. Golden-image comparison with a perceptual metric, once the look settles.
2083. When parallel work is touching the same files, re-read before editing —
      a replace target that silently does not match is a wasted round trip.
2084. Verify an edit landed (`grep -c`) rather than assuming the replace
      matched. Two edits in this session silently did nothing.
2085. Publish the debugging limits too: this harness finds *what* draws an
      artifact, never *why*.

## AT. UI and readability (2086–2130)

2086. **Components, not format strings.** The HUD was one thirty-line `%`
      format rebuilt every frame; adding a readout meant editing the string and
      its argument array in lockstep.
2087. A row owns its own state and writes to the scene tree only when its value
      changes.
2088. A panel owns title, style and visibility; adding a readout is one
      `add_row` call.
2089. A manager owns layout and takes values by key, so panels can be reordered
      without touching the code that produces the data.
2090. One gauge colour ramp across the whole UI — green through amber to red —
      so a full bar means one thing everywhere.
2091. Show the tighter of two limits on a combined gauge. The pack binds on
      volume before mass, and mass alone was lying.
2092. Label what a number *means*, not what the variable is called.
      `encumbrance` as a multiplier displayed as a load fraction is backwards.
2093. Panels appear and retire by view, so instruments show habitat-scale data
      and the colonist view shows local.
2094. Every readout in a retired builder must be ported before the builder is
      deleted. Two of three here were live features, not dead code.
2095. A transient toast for feedback that does not belong in a panel.
2096. Reticle carries what you are aiming at, its distance, and whether the
      action is possible — colour-coded.
2097. A brush marker shows the brush's **actual shape**. A sphere marker on a
      disc brush is a lie.
2098. Ghost previews for everything placeable, snapped exactly where it will
      land.
2099. Overlays for every simulated field, reachable from the plan view.
2100. Contours and isolines over any field — the cheapest legibility win
      available.
2101. A clickable legend that highlights matching ground, so the legend is a
      query rather than a key.
2102. Causal chains on every outcome: "−22 %: nitrogen deficit from day 34".
2103. Store the reason, not just the number — it is also the best debugging
      tool the project will have.
2104. A chronicle the player can query at a place.
2105. Contrast audit as a CI gate, not an intention.
2106. Colour-blind-safe palettes for every overlay, selectable.
2107. A reduced-motion option covering sway, particles and camera easing.
      → settings + `RamaControls.reduced_motion`.
2108. Reading-speed-aware text presentation.
2109. Font size scaling that does not break panel layout.
      → `font_scale` persisted (apply pass still thin).
2110. Remappable input, already shipped — keep it as the contract for any new
      action.
2111. Gamepad parity for every action, through the same InputMap path.
2112. Never read a raw keycode in game code.
2113. Trackpad-aware defaults: small scroll steps, eased, with horizontal
      scroll bound to something useful.
2114. Keyboard alternatives for every mouse-button action.
2115. Sensitivity and invert-Y persisted per player.
2116. A pause menu that is reachable, obvious, and does not eat the game.
2117. Settings that persist to `user://` and survive a schema change.
2118. HUD density as a setting: full, compact, minimal, off.
2119. A photo mode that hides all of it. → F11 / `--photo`.
2120. Screenshot metadata embedding seed, position and date.
2121. Panels never overlap the reticle or the aim readout.
2122. Nothing important in the corners a tilt-shift vignette darkens.
2123. Text over world content always gets an outline or a plate.
2124. Numbers get units, always.
2125. Percentages get a gauge; absolute values get a number; rates get both.
2126. Trends matter more than instants for slow systems — an arrow beats a
      digit for soil nitrogen.
2127. Alarm thresholds surface in-fiction before they surface as UI.
2128. A first-run layout that shows less, and reveals panels as systems unlock.
2129. Every panel answerable to "what decision does this help me make".
2130. Cut any readout that fails 2129.

## AU. Audio (2131–2160)

2131. Procedural generation over shipped assets where the parameter matters —
      excavation pitch tracking brush size is something a fixed sample cannot do.
2132. One synthesis module, several voices, so timbre stays coherent.
2133. Filtered noise with an exponential tail for granular material.
2134. A pitch-falling body plus a click for placing something solid.
2135. Material-keyed pitch: hardness maps to fundamental, so stone and regolith
      sound different for free.
2136. Tool impact sound from the material struck, not the tool.
2137. Footsteps by surface and wetness, from fields that already exist.
      → dry / wet / stone streams; probe exposes `moisture`.
2138. Footstep cadence from actual distance travelled, not a timer.
2139. Water sound scaled by local discharge and gradient.
2140. Distinct beds for riffle, pool, cascade and fall, cross-faded by
      proximity.
2141. Wind in vegetation, timbre by foliage type — grass hiss, broadleaf
      rustle, needle roar.
2142. Rain on different surfaces: leaves, water, stone, roof, mud.
2143. Thunder delay computed from distance, properly.
2144. **Echo around the drum.** Sound wraps in a closed cylinder; nothing else
      sounds like this and it is nearly free. → wrap factor on water ambience.
2145. Reverb by biome: forest deadens, open water carries, a cave rings.
2146. Occlusion by terrain, so a river is audible round a corner but not
      through a hill.
2147. Distance-appropriate content: a settlement as murmur, then voices, then
      words.
2148. Insect density rising with temperature, so the air is audibly warmer.
2149. Birdsong density as a legible index of ecosystem health.
2150. Dawn chorus timed to the light schedule the colony sets.
2151. Animals going quiet before weather arrives.
2152. A quiet mode where machinery stops and you notice how loud it was.
2153. Mix buses with headroom, and a measured loudness target.
      → Master / World / UI buses; Master at −6 dB.
2154. No sound plays without a simulation reason.
2155. Audio budget: voices capped, with a stealing policy.
2156. Subtitles and captions for everything meaningful.
2157. Visual alternatives for audio-only information.
2158. Separate sliders for music, world, UI and voice.
2159. A mute-on-focus-loss default.
2160. Publish the limits: synthesis is toy DSP, not physical modelling.

## AV. Onboarding and the first ninety seconds (2161–2185)

*ORRERY's rule 3: delight in the first ninety seconds beats feature count.*

2161. The wake sequence is the hook — you open your eyes on the ground and the
      world curves up around you. Protect it.
2162. Nothing modal before the player has looked up.
2163. Instrument the first ninety seconds: time to first look-up, first walk,
      first dig.
2164. A playtest harness that times it and copies a row for the log.
2165. First dig within two minutes, unprompted, or the affordance is wrong.
2166. The reticle readout teaches material and depth without a tutorial.
2167. Teach the drum's curvature by letting the player throw something.
2168. Teach drainage by letting them cut a trench and watch the stream move —
      the single most convincing thing this game does.
2169. Teach the closed system by showing the ledger when they first fill a pack.
2170. Never explain what the world can demonstrate.
2171. An agent who works nearby is the tutorial for what is possible.
2172. First contact with a colonist inside five minutes.
2173. A first goal that is material and legible: get water to a field.
2174. Progressive panel reveal, per 2128.
2175. No forced tutorial sequence; hints that expire.
2176. A settings pass reachable before play, not only after.
2177. Accessibility options at first run.
2178. Content settings at first run, per `PD_BRIEF.md` §8.
2179. A save on first quit, always.
2180. Resume puts you exactly where you were, including camera.
2181. A short "what happened while you were away" on return.
2182. New-player and returning-player paths differ.
2183. A demo build that is the first ninety seconds and nothing else.
2184. Wishlist conversion measured against that demo.
2185. Willingness to cut anything that does not survive the ninety seconds.

## AW. Shipping (2186–2200)

2186. `NEXT.md` capped at ten remains the only queue. These 2,200 items are a
      register.
2187. Every item traceable to a requirement; every requirement to a test.
2188. `MODEL_LIMITS.md` updated **before** any system claims more precision.
2189. `CALIBRATION.md` carries provenance for every empirical constant, and
      invented values are tagged invented.
2190. `FIELDS.md` registers every field with owner, unit and save status.
2191. Determinism lint and golden tests as gates, not aspirations.
      → craft CI gates landed; sim golden suite still thin.
2192. Save migration tested against committed fixtures every build.
2193. Performance budget per system, enforced, with a profiler overlay showing
      the actual split.
2194. Steam Deck as an explicit target with its own preset.
2195. A low-spec preset that degrades foliage, volumetrics and LOD distance
      first — the three biggest costs.
2196. Localisation planned from the intent layer, not from strings.
2197. An external review of the content policy before launch.
2198. A public model-limits page, because stated limits beat implied precision.
2199. Ship the base game clean; the Archive layer stays a separate SKU.
2200. And the rule, unchanged across all two thousand two hundred: **honesty of
      mechanism beats cosmetic spectacle; legibility beats completeness;
      delight in the first ninety seconds beats feature count; stated limits
      beat implied precision.**

---

## The shortest useful summary

If someone reads one page of this register, make it the five bug families at the
top. Every visual defect in twenty hours of work was one of them, and four of
the five are statically detectable — which means they belong in CI, not in a
debugging session.

**Now they are:** shared includes enforce 1 and 5's cousin; MultiMesh init and
metre-literal CI gates catch 3 and 4; falloff comments + feature tags catch 2.
`Godot --path game -- --bisect` is the harness for everything else.
