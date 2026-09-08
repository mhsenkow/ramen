extends Node3D
const RamaControls = preload("res://scripts/controls.gd")
const RamaBody = preload("res://scripts/avatar/body.gd")
const RamaGait = preload("res://scripts/avatar/gait.gd")
const RamaTrees = preload("res://scripts/trees.gd")
## RAMA CYCLE — MVP-0. The world you can stand in. (REQUIREMENTS.md §G)
##
## Everything geometric comes from the Rust core (rama_sim). This script places
## it, lights it, and gets a camera into it. No terrain maths lives here.

const FAR_NT := 896
const FAR_NZ := 560
const FAR_OFFSET := 1.35     # sit under near chunks so the LOD edge doesn't z-fight
const CHUNK_SPAN := 44.0     # metres per near chunk, both lateral axes
const CHUNK_CELL := 1.4      # voxel size, metres
const CHUNK_RADIUS := 5      # chunks loaded around the player
const UNLOAD_RADIUS := 6
const DIG_SNAP := 1.0        # brush centres land on a 1 m grid
const MID_SPAN := 1500.0     # metres across the mid-detail window
const MID_N := 300           # 5 m cells — between chunk 1.4 m and far 6-10 m
const MID_MOVE := 180.0      # rebuild after this much travel
const NEAR_FADE_START := 130.0
const NEAR_FADE_END := 250.0

var terrain                  # RamaTerrain (Rust)
var P: Dictionary
var spawn: Dictionary
var player: Node3D
var hud: Label
var ui
var loaded := {}
var chunk_fail := {}
var pending: Array = []
var pending_set := {}
var chunk_root: Node3D
var shot_mode := false
var modules: Array = []
var day := 1.0
var clock := 0.0
var post_layer: CanvasLayer
var cloud_node: MeshInstance3D
var axis_mat: StandardMaterial3D  # spine core (kept for compat)
var spine_mat: StandardMaterial3D
var carriage_mat: StandardMaterial3D
var ring_mat_a: StandardMaterial3D
var ring_mat_b: StandardMaterial3D
var day_carriage: MeshInstance3D
var endcap_ring_a: MeshInstance3D
var endcap_ring_b: MeshInstance3D
var carriage_z := 0.0
var last_carriage_steam_z := 1e9
const CARRIAGE_FRAC := 0.10  # fraction of habitat length
const CARRIAGE_PROX_M := 900.0  # RamaSun falloff along z
var dust: MultiMeshInstance3D
var insects: MultiMeshInstance3D
var census := {}
var town_marks: Array = []
var mini_vp: SubViewport
var mini_cam: Camera3D
var mini_overlay: Control
var ship_overlay: Control
var hud_panels: CanvasLayer
var reticle: Control
var home := Vector2.ZERO
var waypoint := Vector2.ZERO
var has_waypoint := false
var menu: CanvasLayer
var audio: Node
var mini_size := 150.0
const DAY_LENGTH := 420.0   # seconds per engineered day
## Fast-day preview: full cycle in ~DAY_LENGTH/FAST_DAY_MULT real seconds.
const FAST_DAY_MULT := 18.0
var day_speed := 1.0
var sky_event := 0  # 0 clear, 1 fog, 2 storm
var sky_intensity := 0.0
var sky_fog := 0.0
var last_sky_event := 0
var chunk_arc := 0.0
var chunk_span := 44.8
var n_around := 128
var n_z_chunks := 134
var flow_refresh_in := -1.0
var biome_tex_rect: TextureRect
var river_root: Node3D
var lake_root: Node3D
var foam_mm: MultiMeshInstance3D
var splash_mm: MultiMeshInstance3D
var stockpile_mm: MultiMeshInstance3D
var agent_mm: MultiMeshInstance3D
## The colony, at two tiers. Far: one MultiMesh per build, so a crowd across a
## field reads as different men rather than a row of capsules. Near: a small
## pool of real articulated rigs, because these are the people you will meet.
var agent_root: Node3D
var agent_far: Dictionary = {}      # archetype -> MultiMeshInstance3D
var agent_far_xf: Dictionary = {}   # archetype -> Array[Transform3D]
var agent_far_col: Dictionary = {}  # archetype -> Array[Color]
var agent_rigs: Array = []
const AGENT_RIGS := 6
const AGENT_RIG_RANGE := 62.0
var work_mm: MultiMeshInstance3D
var roof_mm: MultiMeshInstance3D
var follower_mm: MultiMeshInstance3D
var plot_mm: MultiMeshInstance3D
var carcass_mm: MultiMeshInstance3D
var station_mm: MultiMeshInstance3D
var litter_mm: MultiMeshInstance3D
var rock_prop_mm: MultiMeshInstance3D
var reed_mm: MultiMeshInstance3D
var rain_mm: MultiMeshInstance3D
var grazer_mm: MultiMeshInstance3D
var world_env: WorldEnvironment
var mist_phase := 0.0
var steam_life := 0.0
var steam_mm: MultiMeshInstance3D
var last_day_band := ""
var catchment_root: Node3D
var catchment_phase := 0.0
var autosave_accum := 0.0
var life_refresh_accum := 0.0
var life_phase := 0
var agent_refresh_accum := 0.0
const AUTOSAVE_SECS := 180.0
var map_label: Label
var soil_chip: ColorRect
var soil_chip_label: Label
var grass_mm: MultiMeshInstance3D
var grass_anchor := Vector2(1e9, 1e9)
var mid_mi: MeshInstance3D
var mid_anchor := Vector2(1e9, 1e9)
const GRASS_N := 6400
const GRASS_RADIUS := 30.0
const GRASS_MOVE := 7.0
const GRASS_STRIDE := 6  # theta, z, r, scale, lush, biome_id
var rama_sun: DirectionalLight3D
var bounce_accum := 0.0
var bounce_map: PackedByteArray = PackedByteArray()
var bounce_anchor := Vector2(1e9, 1e9)
var dwelling_mats: Array = []  # ShaderMaterial for dusk emission_boost
var plant_species_near: Array = []  # MultiMeshInstance3D per archetype
var plant_species_mid: Array = []
var threaded_meshing := true
var plant_mm: MultiMeshInstance3D
var plant_mm_mid: MultiMeshInstance3D
var plant_mm_far: MultiMeshInstance3D
var woodscape_mm: MultiMeshInstance3D
var woodscape_refresh_in := 0.0
var woodscape_owns_near := false
var soil_overlay: TextureRect
var soil_mode := 0
var catchment_cache: PackedFloat32Array = PackedFloat32Array()
var catchment_aim := Vector2(-999.0, -999.0)
var sim_accum := 0.0
var last_sim := {}
var pool_refresh_in := -1.0
var wet_refresh_in := -1.0
var splash_life := 0.0
var splash_origin := Vector3.ZERO
var last_pool_cells := -1
var last_pool_depth := -1.0
var wet_chunk_queue: Array = []
var catchment_mat: StandardMaterial3D
var catchment_pending_th := 0.0
var catchment_pending_z := 0.0
var catchment_dirty := false
var catchment_cooldown := 0.0
var visual_phase := 0
var visuals_pending := false
var sim_frame_cooldown := 0  # frames to skip heavy visuals after sim_tick
var water_bio_name := ""
var water_bio_cache_at := Vector2(1e9, 1e9)
var plant_refresh_due := false
var plant_data: PackedFloat32Array = PackedFloat32Array()
var plant_bucket_idx := 0
var plant_fill_j := 0
var plant_indices: Array = []
var last_plants_alive := -1
var plant_anchor := Vector2(-999.0, -999.0)
var remesh_queue: Array = []
var stockpile_refresh_in := -1.0
var dig_held := false
## Hull arc+z of last `_queue_chunks` — ring only rebuilds after real travel.
var stream_anchor := Vector2(-99999.0, -99999.0)
## Shared near-chunk material — one ShaderMaterial for the whole ring.
var _chunk_terrain_mat: ShaderMaterial
var _drum_diag := 0.0
var _sky_poll_accum := 0.0
var _sky_inten_target := 0.0
var _sky_fog_target := 0.0
var _sched_phase := -1.0
var _sched_day := -1.0
var _sched_cz := 1e9
var _last_dusk_boost := -1.0
var _insect_accum := 0.0
var _reduced_motion_applied := -1
var _hud_accum := 0.0
var _audio_accum := 0.0
var _hud_e := 0.0
var _hud_fx := 0.0
var _water_dep := 0.0
var _water_fx := 0.0
const SIM_STEP_DAYS := 0.02  # ~ habitat days per real second at 1x
const CATCHMENT_COOLDOWN := 0.28
const PLANT_FILL_BUDGET := 120
const SAVE_PATH := "user://rama_strokes.bin"
const SAVE_SOIL := "user://rama_soil.bin"
const SAVE_DWELL := "user://rama_dwellings.bin"
const SAVE_WORLD := "user://rama_world.json"
var playtest := false
var playtest_t0_ms := 0
var playtest_lookup_ms := -1
var playtest_walk_ms := -1
var playtest_dig_ms := -1
var returning_player := false
var away_blurb := ""
var grass_n_cap := GRASS_N
var grass_radius_cap := GRASS_RADIUS
var plant_lod_radius := 900.0
var mid_span_eff := MID_SPAN

func _ready() -> void:
	shot_mode = "--shot" in OS.get_cmdline_user_args()
	var selftest := "--selftest" in OS.get_cmdline_user_args()
	var bisect := "--bisect" in OS.get_cmdline_user_args()
	var farprobe := "--farprobe" in OS.get_cmdline_user_args()
	var parade := "--parade" in OS.get_cmdline_user_args()
	var photo := "--photo" in OS.get_cmdline_user_args()
	playtest = "--playtest" in OS.get_cmdline_user_args()
	if "--fast-day" in OS.get_cmdline_user_args():
		day_speed = FAST_DAY_MULT
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--quality="):
			RamaControls.quality = a.split("=")[1]
		if a == "--threaded":
			threaded_meshing = true
		if a == "--no-threaded":
			threaded_meshing = false
	terrain = ClassDB.instantiate("RamaTerrain")
	if terrain.has_method("set_threaded_meshing"):
		terrain.set_threaded_meshing(threaded_meshing)
	var t0 := Time.get_ticks_msec()
	terrain.generate(0)
	P = terrain.params()
	_drum_diag = 0.0
	_chunk_terrain_mat = null
	chunk_arc = CHUNK_SPAN / float(P["radius"])
	chunk_span = float(P["chunk_span"])
	n_around = int(P["chunks_around"])
	n_z_chunks = int(ceil(float(P["length"]) / chunk_span))
	spawn = terrain.find_spawn()
	terrain.seat_neighbour(float(spawn["theta"]), float(spawn["z"]))
	terrain.seed_starter_stations(float(spawn["theta"]), float(spawn["z"]))
	census = terrain.biome_census(260)
	home = Vector2(float(spawn["theta"]), float(spawn["z"]))
	print("[rama] habitat generated in %d ms" % (Time.get_ticks_msec() - t0))
	print("[rama] params: ", P)
	print("[rama] spawn: ", spawn)
	if day_speed > 1.0:
		_apply_day_speed(false)
		print("[rama] fast-day on (×%.0f) — F6 toggles" % day_speed)

	clock = DAY_LENGTH * 0.30   # wake mid-morning, not at midnight
	RamaControls.install()
	menu = load("res://scripts/menu.gd").new()
	menu.world = self
	add_child(menu)
	audio = load("res://scripts/audio.gd").new()
	add_child(audio)
	if not RamaControls.cfg_exists() and not shot_mode and not selftest and not bisect and not farprobe and not parade:
		# First-run a11y + content before wake (§2176–2178).
		call_deferred("_open_first_run")
	returning_player = FileAccess.file_exists(SAVE_WORLD)
	if returning_player and not shot_mode and not selftest and not parade:
		call_deferred("_offer_resume")
	playtest_t0_ms = Time.get_ticks_msec()
	RenderingServer.global_shader_parameter_add("rama_day",
		RenderingServer.GLOBAL_VAR_TYPE_FLOAT, 1.0)
	RenderingServer.global_shader_parameter_add("rama_haze",
		RenderingServer.GLOBAL_VAR_TYPE_FLOAT, 1.0)
	RenderingServer.global_shader_parameter_add("rama_gust",
		RenderingServer.GLOBAL_VAR_TYPE_FLOAT, 0.55)
	RenderingServer.global_shader_parameter_add("rama_bounce_tint",
		RenderingServer.GLOBAL_VAR_TYPE_VEC3, Vector3(0.38, 0.48, 0.36))
	apply_quality()
	_build_env()
	_build_rama_sun()
	_build_far()
	_build_water()
	_build_rivers()
	_build_pools()
	_build_grass()
	_build_foam()
	_build_splash()
	_build_stockpiles()
	_build_litter()
	_build_agents()
	_build_dwellings()
	_build_carcasses()
	_build_stations()
	_build_steam()
	_build_rain()
	_build_grazers()
	_build_endcaps()
	_build_axis_light()
	chunk_root = Node3D.new(); add_child(chunk_root)
	_build_homestead()
	_build_towns()
	_build_clouds()
	_build_dust()
	_build_insects()
	_build_player()
	_build_mid()
	_build_plants()
	_build_woodscape()
	_build_overlays()
	_build_panels()
	player.ghost = _build_ghost()
	_queue_chunks()
	# Fill the whole near ring before the first frame. It used to stream in
	# over the following seconds, which meant the opening shot of the game was
	# the mid tier dithering through the holes — the loudest artefact in the
	# picture. A chunk costs ~2 ms now, so the entire ring is ~0.3 s on the end
	# of a load that already takes seconds.
	stream_anchor = Vector2(float(spawn["theta"]) * float(P["radius"]), float(spawn["z"]))
	var t_ring := Time.get_ticks_msec()
	_pump_chunks(400)
	print("[rama] near ring: %d chunks in %d ms" % [
		loaded.size(), Time.get_ticks_msec() - t_ring])
	# One biosphere tick so rain/soil exist on first frame.
	last_sim = terrain.sim_tick(0.05)
	refresh_agents()
	refresh_dwellings()
	_refresh_litter()
	_refresh_rain()
	_refresh_grazers()
	print("[rama] biosphere: %d plants, lakes=%s, rain=%.3f, agents=%d" % [
		terrain.plant_count(), terrain.lake_count(), float(last_sim.get("rain", 0.0)),
		int(last_sim.get("agents", 0))])
	if selftest:
		if rama_sun:
			rama_sun.shadow_enabled = false
		await _selftest()
	elif farprobe:
		var fp = load("res://scripts/debug/farprobe.gd").new()
		fp.world = self
		add_child(fp)
		fp.run()
	elif bisect:
		var b = load("res://scripts/debug/bisect.gd").new()
		b.world = self
		add_child(b)
		b.run()
	elif parade:
		var pa = load("res://scripts/debug/parade.gd").new()
		pa.world = self
		add_child(pa)
		await pa.run()
	elif shot_mode:
		await _take_shots()
	elif photo:
		# Photo mode hides HUD (§2119).
		if hud_panels:
			hud_panels.visible = false
		if reticle:
			reticle.visible = false
		RamaControls.photo_mode = true

func _open_first_run() -> void:
	if menu:
		menu.open_first_run()

func _offer_resume() -> void:
	# Returning player: load camera/home if save exists (§2180–2182).
	if FileAccess.file_exists(SAVE_PATH):
		load_world()
		if away_blurb != "":
			note(away_blurb)

func apply_quality() -> void:
	## Low-spec / Deck: cut foliage, LOD distance, haze first (§2194–2195).
	var vp := get_viewport()
	match RamaControls.quality:
		"low":
			grass_n_cap = 1600
			grass_radius_cap = 16.0
			plant_lod_radius = 420.0
			mid_span_eff = 900.0
			RenderingServer.global_shader_parameter_set("rama_haze", 0.72)
			if vp:
				vp.msaa_3d = Viewport.MSAA_DISABLED
		"deck":
			grass_n_cap = 2400
			grass_radius_cap = 20.0
			plant_lod_radius = 560.0
			mid_span_eff = 1100.0
			RenderingServer.global_shader_parameter_set("rama_haze", 0.85)
			if vp:
				vp.msaa_3d = Viewport.MSAA_2X
		_:
			grass_n_cap = GRASS_N
			grass_radius_cap = GRASS_RADIUS
			plant_lod_radius = 900.0
			mid_span_eff = MID_SPAN
			RenderingServer.global_shader_parameter_set("rama_haze", 1.0)
			if vp:
				vp.msaa_3d = Viewport.MSAA_2X
	if grass_mm:
		refresh_grass(true)
	if player and terrain:
		plant_data = terrain.plants_lod(player.theta, player.z, plant_lod_radius)

func _selftest() -> void:
	var tt := Time.get_ticks_msec()
	_pump_chunks(40)
	print("[rama] chunk fill 40: %d ms, pending=%d loaded=%d" % [Time.get_ticks_msec() - tt, pending.size(), loaded.size()])
	var tris := 0
	var meshed := 0
	for c in chunk_root.get_children():
		if c is MeshInstance3D and c.mesh:
			meshed += 1
			for si in c.mesh.get_surface_count():
				tris += int(c.mesh.surface_get_array_index_len(si) / 3.0)
	var far: Node = get_node_or_null("FarField")
	var far_tris: int = 0
	if far:
		for c in far.get_children():
			if c is MeshInstance3D and c.mesh:
				far_tris += int(c.mesh.surface_get_array_index_len(0) / 3.0)
	var props := 0
	for c in get_children():
		if c is Node3D: props += c.get_child_count()
	print("\n================ MVP-0 SELFTEST ================")
	print("far field tris      : %d  (%d sectors)" % [far_tris, FAR_SECTORS])
	print("near chunks meshed  : %d  (%d tris, caves included)" % [meshed, tris])
	print("prop nodes placed   : %d" % props)
	print("spawn theta/z/r     : %.4f / %+.1f / %.1f" % [spawn["theta"], spawn["z"], spawn["radius"]])
	print("spawn elevation     : %.1f m above hull floor" % spawn["elevation"])
	if census.has("hypso_below_water"):
		print("hypsometry          : below WL %.1f%%  peaks %.1f%%  elev_hash %s" % [
			float(census.get("hypso_below_water", 0.0)) * 100.0,
			float(census.get("hypso_peak", 0.0)) * 100.0,
			str(census.get("elev_hash", "?"))])
	var max_e_hab: float = float(P.get("max_elevation", 440.0))
	var max_e_seen: float = float(census.get("max_elevation", 0.0))
	if max_e_seen <= 0.0 and census.has("elev_max"):
		max_e_seen = float(census.get("elev_max", 0.0))
	if max_e_seen > 0.0:
		var elev_ok: bool = max_e_seen >= max_e_hab * 0.70
		print("peak elevation      : %.0f / %.0f m — %s" % [
			max_e_seen, max_e_hab, ("PASS" if elev_ok else "FAIL")])
		if not elev_ok:
			push_error("peak elevation %.0f m < 70%% of authored max %.0f — radial/recipe regression" % [
				max_e_seen, max_e_hab])
	if terrain.has_method("province_census"):
		var pc: Dictionary = terrain.province_census()
		var parts: PackedStringArray = []
		for k in pc.keys():
			parts.append("%s=%s" % [str(k), str(pc[k])])
		print("province census     : %s" % " ".join(parts))
	print("player feet radius  : %.2f  (ground %.2f)" % [player.r, ground_at(player.theta, player.z)])
	print("player world pos    : %s" % to_world(player.theta, player.z, player.r))
	print("local up vector     : %s" % up_at(to_world(player.theta, player.z, player.r)))
	if terrain.has_method("selftest_threaded"):
		var thr_ok: bool = terrain.selftest_threaded()
		print("threaded smoke      : %s  (--threaded=%s)" % [
			"PASS" if thr_ok else "FAIL", str(threaded_meshing)])
		if not thr_ok:
			push_error("selftest_threaded failed — do not enable --threaded")
	if rama_sun:
		print("RamaSun             : energy=%.2f shadows=%s dist=%.0f" % [
			rama_sun.light_energy, rama_sun.shadow_enabled,
			rama_sun.directional_shadow_max_distance])
	if terrain.has_method("spine_status"):
		var sp: Dictionary = terrain.spine_status()
		print("photothermal spine  : carriage_z=%.0f vapor=%.3f light=%.2f" % [
			float(sp.get("carriage_z", 0.0)),
			float(sp.get("vapor_rate", 0.0)),
			float(sp.get("light_now", 0.0))])
	# Walk the full 360 and confirm we come back upright. (REQUIREMENTS A2)
	var th := float(spawn["theta"])
	var steps := 720
	var minr := 1e9
	var maxr := -1e9
	for i in steps:
		var a := th + TAU * i / steps
		var g := ground_at(a, spawn["z"])
		minr = min(minr, g); maxr = max(maxr, g)
	print("360 deg traverse    : ground radius %.1f..%.1f m (relief %.1f m) - continuous" % [minr, maxr, maxr - minr])
	# Cave check: is there open space below the surface anywhere near spawn?
	var caves := 0
	for i in 400:
		var a: float = float(spawn["theta"]) + (randf() - 0.5) * 0.5
		var zz: float = float(spawn["z"]) + (randf() - 0.5) * 400.0
		var surf: float = terrain.ground_radius(a, zz)
		for depth in range(6, 46, 2):
			if terrain.density_at(to_world(a, zz, surf + depth)) < 0.0:
				caves += 1
				break
	print("cave hits           : %d / 400 subsurface probes found open space" % caves)
	var segs0: int = int(terrain.river_segments(0.11).size() / 7.0)
	var snap0: PackedInt32Array = terrain.flow_snapshot()
	var sig0: int = terrain.flow_signature()
	var lake0: int = terrain.lake_cells()
	var fill0: float = terrain.fill_depth_mean()
	var best_fx := -1.0
	var best_th := float(spawn["theta"])
	var best_z := float(spawn["z"])
	for i in 200:
		var a: float = float(spawn["theta"]) + (randf() - 0.5) * 0.8
		var zz: float = float(spawn["z"]) + (randf() - 0.5) * 600.0
		var fx: float = terrain.water_flux(a, zz)
		if fx > best_fx and terrain.elevation(a, zz) < 120.0:
			best_fx = fx
			best_th = a
			best_z = zz
	var e0: float = terrain.elevation(best_th, best_z)
	var gr: float = terrain.ground_radius(best_th, best_z)
	var origin: Vector3 = to_world(best_th, best_z, gr - 1.7)
	var hit: Dictionary = terrain.raycast(origin, -up_at(origin), 14.0)
	var dug := 0
	if hit.get("hit", false):
		var pt: Vector3 = hit["point"]
		# Trench along +z across the channel — the one-week test.
		for i in 36:
			var yd: Dictionary = terrain.dig(pt + Vector3(0, 0, i * 1.15 - 18.0), 3.6, DIG_SNAP, false)
			if yd.get("ok", false):
				dug += 1
	var e1: float = terrain.elevation(best_th, best_z)
	terrain.refresh_flow()
	var segs1: int = int(terrain.river_segments(0.11).size() / 7.0)
	var moved: int = terrain.flow_diff(snap0)
	var sig1: int = terrain.flow_signature()
	print("live flow test      : %d digs, elev %.1f -> %.1f m (Δ %.1f), strokes %d" % [
		dug, e0, e1, e0 - e1, terrain.edit_count()])
	print("routing rerouted    : %d cells changed downstream · signature %s · rivers %d -> %d" % [
		moved, ("CHANGED" if sig0 != sig1 else "IDENTICAL"), segs0, segs1])
	print("depression fill     : lake cells %d -> %d · mean fill depth %.4f -> %.4f m" % [
		lake0, terrain.lake_cells(), fill0, terrain.fill_depth_mean()])
	var tick: Dictionary = terrain.sim_tick(1.0)
	print("biosphere 1-day     : plants %d · rain %.3f · moisture %.3f · N %.0f" % [
		tick.get("plants", 0), tick.get("rain", 0.0), tick.get("moisture", 0.0), tick.get("nitrogen", 0.0)])
	# The cast, and what their ground can feed. Capacity is a survey over real
	# arable cells, so a bad site cannot grow however long you wait.
	var dbuf: PackedFloat32Array = terrain.dwellings_lod()
	var dn: int = int(dbuf.size() / 9.0)
	var kind_n := [0, 0, 0]
	var reach_n := [0, 0, 0, 0]
	var cap_sum := 0.0
	var cap_min := 1e9
	var cap_max := -1e9
	for i in dn:
		kind_n[clampi(int(dbuf[i * 9 + 2]), 0, 2)] += 1
		reach_n[clampi(int(dbuf[i * 9 + 6]), 0, 3)] += 1
		var cap: float = dbuf[i * 9 + 5]
		cap_sum += cap
		cap_min = minf(cap_min, cap)
		cap_max = maxf(cap_max, cap)
	print("dwellings sited     : %d  (delve %d · terrace %d · township %d)" % [
		dn, kind_n[0], kind_n[1], kind_n[2]])
	# Builds. Every archetype has to construct, measure finite, and land in the
	# right order — a body system whose numbers quietly go NaN or whose bear
	# measures narrower than its twink is worse than one archetype.
	var b_lines: Array = []
	var b_ok := true
	for arch in RamaBody.ORDER:
		var spec: Dictionary = RamaBody.make(arch)
		var mm: Dictionary = RamaBody.measure(spec)
		for k in mm:
			if typeof(mm[k]) == TYPE_FLOAT and not is_finite(float(mm[k])):
				b_ok = false
				push_error("body %s: %s is not finite" % [arch, k])
		b_lines.append("%s %.2fm sh%.2f w%.2f" % [
			arch.substr(0, 4), float(mm["stature"]), float(mm["sh_w"]), float(mm["waist_w"])])
	print("builds              : %s" % " · ".join(b_lines))
	var jock_v: float = float(RamaBody.measure(RamaBody.make("jock"))["sh_w"]) \
			/ float(RamaBody.measure(RamaBody.make("jock"))["waist_w"])
	var bear_v: float = float(RamaBody.measure(RamaBody.make("bear"))["sh_w"]) \
			/ float(RamaBody.measure(RamaBody.make("bear"))["waist_w"])
	print("shoulder:waist      : jock %.2f · bear %.2f — %s" % [jock_v, bear_v,
		"PASS" if (b_ok and jock_v > 1.45 and bear_v < 1.05) else "FAIL"])
	if not (b_ok and jock_v > 1.45 and bear_v < 1.05):
		push_error("archetype silhouettes collapsed toward each other")
	var seen := {}
	for i in 64:
		seen[agent_archetype(i)] = int(seen.get(agent_archetype(i), 0)) + 1
	# The pause menu is where the build is actually chosen, and it is the one
	# screen the shot harness never opens. Construct it once here so a broken
	# creator fails the selftest rather than the player's first Escape.
	var mnu = load("res://scripts/menu.gd").new()
	mnu.world = self
	add_child(mnu)
	var menu_rows: int = mnu.get_child_count()
	mnu.queue_free()
	print("creator screen      : %s (%d nodes)" % [
		"PASS" if menu_rows > 0 else "FAIL", menu_rows])
	print("colony spread       : %d of %d builds over 64 colonists" % [
		seen.size(), RamaBody.ORDER.size()])
	print("legibility          : legible %d · indirect %d · opaque %d · contested %d" % [
		reach_n[0], reach_n[1], reach_n[2], reach_n[3]])
	if dn > 0:
		print("carrying capacity   : %.1f..%.1f people per site (mean %.1f)" % [
			cap_min, cap_max, cap_sum / float(dn)])
	# Soak the colony forward and see whether the ground actually carries
	# anyone. This is the end-to-end check: real heightfield -> arable survey
	# -> surplus -> people arriving. Destructive, so it runs last.
	for i in 24:
		terrain.sim_tick(5.0)
	refresh_dwellings()
	var sbuf: PackedFloat32Array = terrain.dwellings_lod()
	var grew := 0
	var failed := 0
	var pop := 0.0
	for i in int(sbuf.size() / 9.0):
		var f: float = sbuf[i * 9 + 3]
		pop += f
		if f > 0.5:
			grew += 1
		elif sbuf[i * 9 + 5] < 1.0:
			failed += 1
	print("after 120 days      : %.1f followers · %d sites grew · %d cannot feed one man" % [
		pop, grew, failed])
	print("works / followers   : %d works, %d figures placed" % [
		work_mm.multimesh.instance_count if work_mm else 0,
		follower_mm.multimesh.instance_count if follower_mm else 0])
	await _selftest_far_coverage()
	print("================================================\n")
	get_tree().quit()

## Magenta clear-colour coverage at the long-axis vantage. Catches the
## clockwise-winding regression that made cull_back delete the far wall.
func _selftest_far_coverage() -> void:
	if player == null or player.cam == null:
		print("far coverage        : SKIP (no camera)")
		return
	if player.get("wake_t") != null:
		player.wake_t = 99.0
	if player.get("fade"):
		player.fade.visible = false
	player.theta = 3.063
	player.z = 2724.0
	player.r = ground_at(player.theta, player.z)
	player.vr = 0.0
	player.pitch = 0.30
	player.yaw = PI
	player.eye = 1.72
	terrain.set_player_pos(player.theta, player.z)
	_queue_chunks()
	_pump_chunks(400)
	refresh_mid(true)
	player._update_camera()
	# Shadows + custom light() shaders can stall headless frame_post_draw on
	# first compile. Probe with the lamp off — coverage is a winding test.
	var sun_was := false
	if rama_sun:
		sun_was = rama_sun.shadow_enabled
		rama_sun.shadow_enabled = false
		rama_sun.visible = false
	# Day cycle rewrites world_env every frame — clear colour must live on the camera.
	var cenv := Environment.new()
	cenv.background_mode = Environment.BG_COLOR
	cenv.background_color = Color(1.0, 0.0, 1.0)
	cenv.fog_enabled = false
	cenv.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	cenv.glow_enabled = false
	player.cam.environment = cenv
	var was_post := false
	if post_layer:
		was_post = post_layer.visible
		post_layer.visible = false
	# Headless often never emits frame_post_draw once custom light() shaders are
	# in play. Force a draw, then sample.
	await get_tree().process_frame
	RenderingServer.force_draw()
	await get_tree().process_frame
	var img: Image = get_viewport().get_texture().get_image()
	player.cam.environment = null
	if post_layer:
		post_layer.visible = was_post
	if rama_sun:
		rama_sun.visible = true
		rama_sun.shadow_enabled = sun_was
	if img == null:
		print("far coverage        : SKIP (no viewport image)")
		return
	var n := 0
	var empty := 0
	var y := 0
	while y < img.get_height():
		var x := 0
		while x < img.get_width():
			var c := img.get_pixel(x, y)
			if c.r > 0.5 and c.b > 0.5 and c.g < 0.35:
				empty += 1
			n += 1
			x += 4
		y += 4
	var frac := float(empty) / float(maxi(n, 1))
	print("far coverage        : %.1f%% empty (magenta clear) — %s" % [
		frac * 100.0, ("PASS" if frac < 0.05 else "FAIL")])
	if frac >= 0.05:
		push_error("far-field coverage %.1f%% empty — triangle winding / cull regression" % (frac * 100.0))

# ---------------------------------------------------------------- geometry --

func to_world(theta: float, z: float, r: float) -> Vector3:
	return Vector3(r * cos(theta), r * sin(theta), z)

func up_at(p: Vector3) -> Vector3:
	return Vector3(-p.x, -p.y, 0.0).normalized()

## Local frame on the inner surface: x tangential, y up (inward), z axial.
func frame_at(theta: float, z: float, r: float) -> Transform3D:
	var p := to_world(theta, z, r)
	var up := up_at(p)
	var axial := Vector3(0, 0, 1)
	var tang := up.cross(axial).normalized()
	return Transform3D(Basis(tang, up, axial), p)

func _mesh_from(d: Dictionary) -> ArrayMesh:
	var idx: PackedInt32Array = d["indices"]
	if idx.size() == 0:
		return null
	var arr := []
	arr.resize(Mesh.ARRAY_MAX)
	arr[Mesh.ARRAY_VERTEX] = d["verts"]
	arr[Mesh.ARRAY_NORMAL] = d["normals"]
	arr[Mesh.ARRAY_COLOR] = d["colors"]
	arr[Mesh.ARRAY_INDEX] = idx
	var m := ArrayMesh.new()
	m.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arr)
	return m

func _new_terrain_material() -> ShaderMaterial:
	var sm := ShaderMaterial.new()
	sm.shader = load("res://shaders/terrain.gdshader")
	sm.set_shader_parameter("hab_radius", float(P["radius"]))
	sm.set_shader_parameter("haze_scale", float(P["length"]) * 0.40)
	# Key energy matches RamaSun noon product (~1.77); fill stays emission.
	sm.set_shader_parameter("sun_energy", 1.75)
	sm.set_shader_parameter("bounce_energy", 0.58)
	return sm

func _terrain_material() -> ShaderMaterial:
	# One shared material for every near chunk — N unique ShaderMaterials was
	# pure GPU/CPU waste (same uniforms, same shader).
	# Near must dissolve before mid is fully solid or the two heightfields
	# z-fight into a dark flickering rectangle (the midground tear).
	if _chunk_terrain_mat == null:
		_chunk_terrain_mat = _new_terrain_material()
		_chunk_terrain_mat.set_shader_parameter("far_cut_start", NEAR_FADE_START)
		_chunk_terrain_mat.set_shader_parameter("far_cut_end", NEAR_FADE_END)
		_chunk_terrain_mat.set_shader_parameter("dither_seed", 0.0)
	return _chunk_terrain_mat

func _far_terrain_material() -> ShaderMaterial:
	var sm := _new_terrain_material()
	# Mid retires ~0.44–0.56 of mid_span. Far fades in under that handoff.
	# Different dither_seed so mid/far never discard the same pixels.
	sm.set_shader_parameter("near_fade_start", mid_span_eff * 0.40)
	sm.set_shader_parameter("near_fade_end", mid_span_eff * 0.52)
	sm.set_shader_parameter("dither_seed", 17.0)
	sm.set_shader_parameter("haze_start", 90.0)
	sm.set_shader_parameter("haze_scale", float(P["length"]) * 0.40)
	sm.set_shader_parameter("haze_max", 0.72)
	return sm

# ------------------------------------------------------------------- build --

func drum_diagonal() -> float:
	if _drum_diag > 1.0:
		return _drum_diag
	var r: float = float(P.get("radius", 900.0))
	var L: float = float(P.get("length", 6000.0))
	_drum_diag = sqrt(L * L + (2.0 * r) * (2.0 * r))
	return _drum_diag

## Local sun approximating the axis strip within the tilt-shift focus band
## (LANDSCAPE_3200 §AX). Correct nearby; wrong past ~240 m by design.
func _build_rama_sun() -> void:
	rama_sun = DirectionalLight3D.new()
	rama_sun.name = "RamaSun"
	rama_sun.light_color = Color(1.0, 0.95, 0.86)
	rama_sun.light_energy = 1.65
	rama_sun.shadow_enabled = true
	rama_sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS
	# Shadow distance = diorama focus band (~240 m), not drum size (CALIBRATION.md).
	rama_sun.directional_shadow_max_distance = 240.0
	rama_sun.directional_shadow_split_1 = 0.12
	rama_sun.directional_shadow_split_2 = 0.38
	rama_sun.directional_shadow_blend_splits = true
	rama_sun.shadow_blur = 1.55
	rama_sun.shadow_opacity = 0.66
	rama_sun.light_specular = 0.04
	# Fade cascades before the far wall so map-shaped shadows never appear.
	rama_sun.directional_shadow_pancake_size = 12.0
	add_child(rama_sun)

func _aim_rama_sun() -> void:
	if rama_sun == null or player == null:
		return
	var feet: Vector3 = player.feet_pos()
	var up := up_at(feet)
	# Light comes FROM the day-carriage, not from "straight up".
	#
	# A strip hung dead along the axis gives every hour of every day the same
	# dead-overhead noon: no raking light, no long shadows, no form on a
	# hillside. But the carriage is a segment that TRAVELS, and when it is
	# still down the habitat the light reaching you arrives at an angle. That
	# angle is dawn. Aim at the nearest point of the lit segment.
	var car_half: float = float(P.get("length", 6000.0)) * CARRIAGE_FRAC * 0.5
	var lit_z: float = clampf(feet.z, carriage_z - car_half, carriage_z + car_half)
	var to_strip: Vector3 = (Vector3(0.0, 0.0, lit_z) - feet).normalized()
	# Hold it above the local horizon. Below about twenty degrees a directional
	# light is all shadow-cascade artefact, and the schedule has already dimmed
	# it to nothing by then anyway.
	const MIN_SIN := 0.36
	var vertical: float = to_strip.dot(up)
	var horiz: Vector3 = to_strip - up * vertical
	if vertical < MIN_SIN:
		var hl: float = horiz.length()
		if hl > 1e-5:
			horiz = horiz / hl * sqrt(1.0 - MIN_SIN * MIN_SIN)
		vertical = MIN_SIN
	var l_dir: Vector3 = (horiz + up * vertical).normalized()
	# Godot DirectionalLight shines along local -Z, so +Z points at the light.
	var ref := Vector3(0, 0, 1)
	if absf(l_dir.dot(ref)) > 0.95:
		ref = Vector3(1, 0, 0)
	var right: Vector3 = ref.cross(l_dir).normalized()
	var bitangent: Vector3 = l_dir.cross(right).normalized()
	rama_sun.global_transform = Transform3D(
			Basis(right, bitangent, l_dir), feet + l_dir * 60.0)
	var warm := Color(1.0, 0.94, 0.80).lerp(Color(1.0, 0.55, 0.32), 1.0 - day)
	rama_sun.light_color = warm
	# Bright when the day-carriage is overhead; dim when it's far along z.
	var prox := 1.0
	if P.has("length"):
		prox = 1.0 - clampf(absf(player.z - carriage_z) / CARRIAGE_PROX_M, 0.0, 1.0)
	prox = prox * prox  # sharper overhead falloff
	var local := 0.28 + 0.72 * prox
	rama_sun.light_energy = (0.18 + 1.55 * day) * local
	rama_sun.shadow_opacity = (0.22 + 0.48 * day) * (0.45 + 0.55 * prox)

## Mean linear albedo of the opposite wall → bounce tint (§691 / Wave 4 id map).
func _refresh_bounce_tint() -> void:
	if terrain == null:
		return
	if bounce_map.is_empty():
		bounce_map = terrain.biome_id_map(96, 64)
	if bounce_map.is_empty() or player == null:
		return
	# Biome → authored sRGB albedo (matches paint / BIOME_PLANT roughly).
	var pal := [
		Color(0.12, 0.28, 0.36), # water
		Color(0.14, 0.36, 0.22), # wetland
		Color(0.12, 0.40, 0.18), # riparian
		Color(0.14, 0.32, 0.11), # grassland
		Color(0.38, 0.36, 0.14), # scrub
		Color(0.08, 0.28, 0.12), # forest
		Color(0.55, 0.52, 0.42), # alpine
		Color(0.48, 0.44, 0.38), # bare rock
		Color(0.22, 0.42, 0.16), # farm
	]
	var nt := 96
	var nz := 64
	var th0: float = player.theta
	var sum := Vector3.ZERO
	var count := 0
	for zi in nz:
		for ti in nt:
			var th: float = float(ti) / float(nt) * TAU
			# Opposite half of the drum.
			var dth: float = absf(wrapf(th - th0, -PI, PI))
			if dth < PI * 0.45:
				continue
			var id: int = clampi(int(bounce_map[zi * nt + ti]), 0, pal.size() - 1)
			var c: Color = pal[id]
			# Approximate srgb→linear with the contract gamma.
			sum += Vector3(pow(c.r, 1.95), pow(c.g, 1.95), pow(c.b, 1.95))
			count += 1
	if count < 1:
		return
	var mean := sum / float(count)
	# Keep it in bounce range so energy doesn't blow out.
	mean = mean.lerp(Vector3(0.38, 0.48, 0.36), 0.25)
	RenderingServer.global_shader_parameter_set("rama_bounce_tint", mean)

func _build_env() -> void:
	world_env = WorldEnvironment.new()
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	# Match fog so empty axis-space fills with the same air as hazed geometry.
	# Without this, terrain silhouettes cut hard against clear navy void.
	var air := Color(0.16, 0.22, 0.28)
	env.background_color = air
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.28, 0.34, 0.36)
	env.ambient_light_energy = 0.48
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	env.tonemap_exposure = 0.92
	# Glow catches the axis strip and window emissives — shafts of light
	# without a volumetric pass. Keep bloom low so midtones stay grounded.
	env.glow_enabled = true
	env.glow_intensity = 0.30
	env.glow_bloom = 0.055
	env.glow_hdr_threshold = 1.25
	env.glow_hdr_scale = 0.85
	# Engine fog fills EMPTY air (shader haze only tints geometry). This is what
	# softens the jagged horizon and the endcap rim into the distance.
	env.fog_enabled = true
	env.fog_mode = Environment.FOG_MODE_DEPTH
	env.fog_light_color = air
	env.fog_light_energy = 1.12
	env.fog_density = 0.00065
	env.fog_aerial_perspective = 0.42
	env.fog_sky_affect = 1.0
	env.fog_sun_scatter = 0.10
	# Sharper near-clear / far-fade profile. Makes the tilt-shift focus band
	# pop harder against the soft far field.
	# Start earlier so coarse far silhouettes dissolve before they read as
	# black saw-teeth against the sky.
	env.fog_depth_begin = 90.0
	# Depth fog reaches FULL strength at depth_end and stays there. At 1900 m in
	# a 6.3 km drum that painted every metre past 1.9 km in exactly the
	# background colour — which is why the far half of the habitat read as
	# "not rendering" when the geometry was there and correctly lit the whole
	# time. Derive it from the drum, and never let it saturate.
	env.fog_depth_end = drum_diagonal() * 1.15
	env.fog_depth_curve = 0.75
	world_env.environment = env
	add_child(world_env)

const FAR_SECTORS := 12

func _build_far() -> void:
	var t0 := Time.get_ticks_msec()
	var root := Node3D.new()
	root.name = "FarField"
	add_child(root)
	var tris := 0
	var far_mat := _far_terrain_material()
	# Angular wedges so frustum cull drops sectors behind the camera. One
	# full-drum mesh has an AABB the size of the habitat and never culls.
	for s in FAR_SECTORS:
		var d: Dictionary = terrain.far_mesh_sector(FAR_NT, FAR_NZ, FAR_OFFSET, s, FAR_SECTORS)
		var m := _mesh_from(d)
		if m == null:
			continue
		var mi := MeshInstance3D.new()
		mi.mesh = m
		mi.material_override = far_mat
		mi.name = "FarSector%d" % s
		# Reduced margin — sectors are narrow enough at 900 m radius.
		mi.extra_cull_margin = 50.0
		root.add_child(mi)
		tris += int(d["indices"].size() / 3.0)
	print("[rama] far field: %d tris in %d sectors, %d ms" % [tris, FAR_SECTORS,
			Time.get_ticks_msec() - t0])

func _build_endcaps() -> void:
	var d: Dictionary = terrain.endcap_mesh(FAR_NT)
	var mi := MeshInstance3D.new()
	mi.mesh = _mesh_from(d)
	var sm := ShaderMaterial.new()
	sm.shader = load("res://shaders/endcap.gdshader")
	mi.material_override = sm
	mi.name = "Endcaps"
	add_child(mi)

func _build_water() -> void:
	# Water pools at constant radius — spin gravity makes the surface a cylinder,
	# not a plane. Terrain above the waterline occludes it naturally.
	var r: float = float(P["radius"]) - float(P["water_level"])
	var L: float = P["length"]
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var seg := 256
	for i in seg:
		var a0 := TAU * i / seg
		var a1 := TAU * (i + 1) / seg
		var p00 := Vector3(r * cos(a0), r * sin(a0), -L * 0.5)
		var p10 := Vector3(r * cos(a1), r * sin(a1), -L * 0.5)
		var p01 := Vector3(r * cos(a0), r * sin(a0), L * 0.5)
		var p11 := Vector3(r * cos(a1), r * sin(a1), L * 0.5)
		for v in [p00, p01, p11, p00, p11, p10]:
			st.set_normal(Vector3(-v.x, -v.y, 0).normalized())
			st.add_vertex(v)
	var mi := MeshInstance3D.new()
	mi.mesh = st.commit()
	var sm := ShaderMaterial.new()
	sm.shader = load("res://shaders/water.gdshader")
	var mimg := Image.create_from_data(1024, 640, false, Image.FORMAT_R8,
			terrain.lake_mask(1024, 640))
	# Without mipmaps this minifies into hard blocks across the far side —
	# which is the checkerboard that showed up on the endcap.
	mimg.generate_mipmaps()
	var mtex := ImageTexture.create_from_image(mimg)
	sm.set_shader_parameter("lake_mask", mtex)
	sm.set_shader_parameter("hab_len", float(P["length"]))
	mi.material_override = sm
	mi.name = "Water"
	add_child(mi)

## Live river ribbons from flow accumulation (LANDSCAPE_200.md items 46–47).
## Width follows hydraulic geometry w ∝ Q^0.5. Rebuild after excavation reroutes.
func _build_rivers() -> void:
	if river_root and is_instance_valid(river_root):
		river_root.queue_free()
	river_root = Node3D.new()
	river_root.name = "Rivers"
	add_child(river_root)
	# Higher threshold → major channels only; pools carry standing water.
	var segs: PackedFloat32Array = terrain.river_segments(0.22)
	if segs.is_empty():
		return
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var i := 0
	while i + 6 < segs.size():
		var a := Vector3(segs[i], segs[i + 1], segs[i + 2])
		var b := Vector3(segs[i + 3], segs[i + 4], segs[i + 5])
		var w: float = segs[i + 6]
		i += 7
		var along := b - a
		if along.length_squared() < 0.01:
			continue
		var up := Vector3(-a.x, -a.y, 0.0).normalized()
		var side := along.cross(up).normalized() * (w * 0.5)
		# Sit above the bed so ribbons don't z-fight into dark tiles.
		var up_lift := up * 0.08
		var p0 := a - side + up_lift
		var p1 := a + side + up_lift
		var p2 := b + side + up_lift
		var p3 := b - side + up_lift
		for v in [p0, p1, p2, p0, p2, p3]:
			st.set_normal(up)
			st.set_color(Color(0.20, 0.42, 0.78, 0.62))
			st.add_vertex(v)
	var mi := MeshInstance3D.new()
	mi.mesh = st.commit()
	var sm := ShaderMaterial.new()
	sm.shader = load("res://shaders/channel.gdshader")
	mi.material_override = sm
	mi.extra_cull_margin = 30.0
	river_root.add_child(mi)

## Standing water free surfaces — Minecraft-style flat pools in basins.
## Ground cover: ONE MultiMesh, one draw call, shadows off, rebuilt only when
## the player has actually moved. 6400 tufts x ~10 tris is nothing next to the
## 215k the terrain already costs.
## The second LOD tier: one mesh covering the band between the near chunks and
## the coarse far field, which was previously a flat plate.
func _build_mid() -> void:
	mid_mi = MeshInstance3D.new()
	var mat := _new_terrain_material()
	# Hidden only under the near ring; solid by the time near starts dissolving
	# (NEAR_FADE_START). Different dither_seed from near/far so crossfades
	# can't punch aligned holes through to the void.
	mat.set_shader_parameter("near_fade_start", 70.0)
	mat.set_shader_parameter("near_fade_end", 125.0)
	mat.set_shader_parameter("far_cut_start", mid_span_eff * 0.44)
	mat.set_shader_parameter("far_cut_end", mid_span_eff * 0.56)
	mat.set_shader_parameter("dither_seed", 9.0)
	mat.set_shader_parameter("haze_start", 150.0)
	mat.set_shader_parameter("haze_scale", float(P["length"]) * 0.40)
	mid_mi.material_override = mat
	mid_mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mid_mi.name = "MidField"
	add_child(mid_mi)
	refresh_mid(true)

func refresh_mid(force := false) -> void:
	if mid_mi == null or player == null or terrain == null:
		return
	var here := Vector2(player.theta * float(P["radius"]), player.z)
	if not force and here.distance_to(mid_anchor) < MID_MOVE:
		return
	mid_anchor = here
	var d: Dictionary = terrain.mid_mesh(player.theta, player.z, mid_span_eff, MID_N)
	var m := _mesh_from(d)
	if m != null:
		mid_mi.mesh = m
		if force:
			print("[rama] mid field: %d tris, 1 draw call" % (d["indices"].size() / 3))

func _build_grass() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	# Five tapered blades at uneven angles. More crossings = denser silhouette
	# without more draw calls; irregular spacing kills the "fan card" read.
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var blade_angs := [0.0, 0.55, 1.15, 2.05, 2.75]
	for bi in blade_angs.size():
		var ang: float = float(blade_angs[bi])
		var lean: float = 0.92 + float(bi % 3) * 0.06
		var dx := cos(ang) * 0.48
		var dz := sin(ang) * 0.48
		var nrm := Vector3(-sin(ang), 0.42, cos(ang)).normalized()
		var b0 := Vector3(-dx, 0.0, -dz)
		var b1 := Vector3(dx, 0.0, dz)
		var tip_h := lean
		var t0 := Vector3(-dx * 0.12, tip_h, -dz * 0.12)
		var t1 := Vector3(dx * 0.12, tip_h, dz * 0.12)
		for vtx in [b0, b1, t1, b0, t1, t0]:
			st.set_normal(nrm)
			st.add_vertex(vtx)
	mm.mesh = st.commit()
	mm.instance_count = 0
	grass_mm = MultiMeshInstance3D.new()
	grass_mm.multimesh = mm
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/grass.gdshader")
	grass_mm.material_override = mat
	grass_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	# No custom_aabb: instances sit ~850 m out on the drum wall, so an AABB
	# bounded around this node's own origin culls the entire field away.
	grass_mm.name = "Grass"
	add_child(grass_mm)

const BIOME_GRASS_COL := [
	Color(0.18, 0.38, 0.32), # water edge
	Color(0.10, 0.38, 0.22), # wetland — deep
	Color(0.16, 0.46, 0.18), # riparian
	Color(0.22, 0.42, 0.14), # grassland
	Color(0.46, 0.40, 0.16), # scrub — straw
	Color(0.08, 0.28, 0.10), # forest understorey
	Color(0.34, 0.36, 0.26), # alpine — grey-green
	Color(0.40, 0.36, 0.28), # bare rock sparse
	Color(0.28, 0.48, 0.16), # farm
	Color(0.12, 0.30, 0.20), # swamp — peat olive
	Color(0.30, 0.54, 0.18), # meadow — bright herb
	Color(0.62, 0.50, 0.30), # desert
	Color(0.76, 0.62, 0.36), # dune — warm gold
	Color(0.72, 0.66, 0.48), # shore
]

func refresh_grass(force := false) -> void:
	if grass_mm == null or player == null or terrain == null:
		return
	var here := Vector2(player.theta * float(P["radius"]), player.z)
	if not force and here.distance_to(grass_anchor) < GRASS_MOVE:
		return
	grass_anchor = here
	var buf: PackedFloat32Array = terrain.grass_field(
			player.theta, player.z, grass_radius_cap, grass_n_cap)
	var stride: int = GRASS_STRIDE if buf.size() % GRASS_STRIDE == 0 else 5
	var n: int = int(buf.size() / float(stride))
	# Build buffer once — Wave 1.7 upload path.
	var xforms: Array[Transform3D] = []
	var cols: Array[Color] = []
	xforms.resize(n)
	cols.resize(n)
	for i in n:
		var th: float = buf[i * stride]
		var zz: float = buf[i * stride + 1]
		var rr: float = buf[i * stride + 2]
		var sc: float = buf[i * stride + 3]
		var lush: float = buf[i * stride + 4]
		var bid: int = 3
		if stride >= 6:
			bid = clampi(int(buf[i * stride + 5]), 0, BIOME_GRASS_COL.size() - 1)
		var xf := frame_at(th, zz, rr)
		var yaw: float = fposmod(sc * 97.31 + lush * 41.7, 1.0) * TAU
		xf.basis = xf.basis.rotated(xf.basis.y, yaw)
		var lean: float = (fposmod(sc * 19.3, 1.0) - 0.5) * 0.22
		xf.basis = xf.basis.rotated(xf.basis.z, lean)
		# Biome height: wetland/swamp taller, meadow lush, alpine/scrub/desert shorter.
		var h_mul := 1.0
		match bid:
			1, 9: h_mul = 1.40
			10: h_mul = 1.18
			4, 6, 7: h_mul = 0.62
			5: h_mul = 0.78
			8: h_mul = 0.90
			11, 12: h_mul = 0.35
			13: h_mul = 0.55
		var hgt: float = (0.14 + lush * 0.26 + fposmod(sc * 13.7, 1.0) * 0.12) * h_mul
		var w: float = hgt * (0.62 + fposmod(sc * 5.9, 1.0) * 0.28)
		xf.basis = xf.basis.scaled(Vector3(w, hgt, w))
		xforms[i] = xf
		var base: Color = BIOME_GRASS_COL[bid]
		# Dry grass leans straw, but only half way: full lerp put gold stubble on
		# ground the paint table had already coloured deep green, and the tufts
		# read as a different biome from the field they stand in.
		var col := base.lerp(Color(0.55, 0.48, 0.22), (1.0 - lush) * 0.55)
		var tone: float = fposmod(sc * 7.13, 1.0)
		col = col.lerp(base.lightened(0.12), tone * 0.35)
		col.r += (tone - 0.5) * 0.05
		# Meadow wildflower flecks; swamp stays cool olive.
		if bid == 10 and tone > 0.72:
			col = col.lerp(Color(0.72, 0.42, 0.55), 0.28)
		elif bid == 9:
			col = col.lerp(Color(0.10, 0.28, 0.18), 0.22)
		cols[i] = col
	grass_mm.multimesh.instance_count = n
	for i in n:
		grass_mm.multimesh.set_instance_transform(i, xforms[i])
		grass_mm.multimesh.set_instance_color(i, cols[i])
	if n > 0 and Engine.get_process_frames() < 4:
		print("[rama] grass: %d tufts within %.0f m" % [n, grass_radius_cap])
		print("[rama] grass: %d tufts (%d tris), 1 draw call" % [n, n * 10])

func _build_pools() -> void:
	if lake_root and is_instance_valid(lake_root):
		lake_root.queue_free()
	lake_root = Node3D.new()
	lake_root.name = "Pools"
	add_child(lake_root)
	var d: Dictionary = terrain.lake_mesh()
	var verts: PackedVector3Array = d.get("verts", PackedVector3Array())
	if verts.is_empty():
		return
	var normals: PackedVector3Array = d.get("normals", PackedVector3Array())
	var indices: PackedInt32Array = d.get("indices", PackedInt32Array())
	var colors: PackedColorArray = d.get("colors", PackedColorArray())
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices
	if colors.size() == verts.size():
		arrays[Mesh.ARRAY_COLOR] = colors
	var am := ArrayMesh.new()
	am.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var mi := MeshInstance3D.new()
	mi.mesh = am
	var sm := ShaderMaterial.new()
	sm.shader = load("res://shaders/pool.gdshader")
	mi.material_override = sm
	mi.extra_cull_margin = 40.0
	lake_root.add_child(mi)

func _build_foam() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var disk := SphereMesh.new()
	disk.radius = 0.45
	disk.height = 0.12
	disk.radial_segments = 12
	disk.rings = 4
	mm.mesh = disk
	mm.instance_count = 0
	foam_mm = MultiMeshInstance3D.new()
	foam_mm.multimesh = mm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.distance_fade_mode = BaseMaterial3D.DISTANCE_FADE_PIXEL_DITHER
	mat.distance_fade_min_distance = 55.0
	mat.distance_fade_max_distance = 110.0
	foam_mm.material_override = mat
	foam_mm.name = "ShoreFoam"
	add_child(foam_mm)

func _refresh_foam() -> void:
	if foam_mm == null or player == null:
		return
	# Prefer thin shore (0.12–0.9 m) so Multimesh foam sits on the contact
	# line that the depth-shore shader already paints.
	var pts: PackedFloat32Array = terrain.shore_points(player.theta, player.z, 110.0, 140)
	var n: int = int(pts.size() / 3.0)
	if n < 1:
		foam_mm.multimesh.instance_count = 0
		return
	foam_mm.multimesh.instance_count = n
	for i in n:
		var th: float = pts[i * 3]
		var zz: float = pts[i * 3 + 1]
		var dep: float = pts[i * 3 + 2]
		var gr: float = terrain.ground_radius(th, zz)
		var r: float = gr - dep * 0.95
		var xf := frame_at(th, zz, r)
		# Keep discs small — oversized foam read as white hex plates in air.
		var s: float = clampf(0.7 + (1.0 - clampf(dep / 1.2, 0.0, 1.0)) * 0.7, 0.55, 1.55)
		xf.basis = xf.basis.scaled(Vector3(s, 0.14, s))
		foam_mm.multimesh.set_instance_transform(i, xf)
		var edge: float = 1.0 - clampf(abs(dep - 0.35) / 0.55, 0.0, 1.0)
		var a: float = clampf(0.16 + edge * 0.45, 0.12, 0.58)
		var wet := Color(0.68, 0.76, 0.72, a * 0.9)
		var dry := Color(0.92, 0.97, 1.0, a)
		foam_mm.multimesh.set_instance_color(i, wet.lerp(dry, clampf(dep / 0.9, 0.0, 1.0)))

func _build_litter() -> void:
	# Leaf litter + signature rocks/reeds — ground that isn't bare Multimesh grass.
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	# WHITE, not a leaf tone. A MultiMesh instance colour MULTIPLIES the source
	# mesh's vertex colour, and prop.gdshader then raises the product to 1.95.
	# Litter authored at 0.3 x an instance at 0.3 landed at 0.008 linear — the
	# black pebbles scattered over every grassland. (RENDER_CONTRACT §2009.)
	_add_prism(st, Vector3(0, 0.04, 0), Vector3(0.35, 0.06, 0.22), Color.WHITE)
	st.generate_normals()
	mm.mesh = st.commit()
	mm.instance_count = 0
	litter_mm = MultiMeshInstance3D.new()
	litter_mm.multimesh = mm
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/prop.gdshader")
	mat.set_shader_parameter("haze_start", 40.0)
	mat.set_shader_parameter("haze_end", 160.0)
	litter_mm.material_override = mat
	litter_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	litter_mm.name = "Litter"
	add_child(litter_mm)

	var rm := MultiMesh.new()
	rm.transform_format = MultiMesh.TRANSFORM_3D
	rm.use_colors = true
	var rst := SurfaceTool.new()
	rst.begin(Mesh.PRIMITIVE_TRIANGLES)
	_add_prism(rst, Vector3(0, 0.18, 0), Vector3(0.55, 0.35, 0.40), Color.WHITE)
	rst.generate_normals()
	rm.mesh = rst.commit()
	rm.instance_count = 0
	rock_prop_mm = MultiMeshInstance3D.new()
	rock_prop_mm.multimesh = rm
	var rmat := ShaderMaterial.new()
	rmat.shader = load("res://shaders/prop.gdshader")
	rmat.set_shader_parameter("haze_start", 55.0)
	rmat.set_shader_parameter("haze_end", 220.0)
	rock_prop_mm.material_override = rmat
	rock_prop_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	rock_prop_mm.name = "SignatureRocks"
	add_child(rock_prop_mm)

	var reed := MultiMesh.new()
	reed.transform_format = MultiMesh.TRANSFORM_3D
	reed.use_colors = true
	var rst2 := SurfaceTool.new()
	rst2.begin(Mesh.PRIMITIVE_TRIANGLES)
	_add_prism(rst2, Vector3(0, 0.55, 0), Vector3(0.06, 1.1, 0.06), Color.WHITE)
	# Second stem a shade darker — a RATIO against the instance colour, not a
	# second absolute green.
	_add_prism(rst2, Vector3(0.08, 0.45, 0.04), Vector3(0.05, 0.9, 0.05), Color(0.88, 0.90, 0.88))
	rst2.generate_normals()
	reed.mesh = rst2.commit()
	reed.instance_count = 0
	reed_mm = MultiMeshInstance3D.new()
	reed_mm.multimesh = reed
	var reed_mat := ShaderMaterial.new()
	reed_mat.shader = load("res://shaders/tree.gdshader")
	reed_mat.set_shader_parameter("haze_start", 50.0)
	reed_mat.set_shader_parameter("haze_end", 200.0)
	reed_mat.set_shader_parameter("fade_start", 90.0)
	reed_mat.set_shader_parameter("fade_end", 180.0)
	reed_mat.set_shader_parameter("model_height", 1.2)
	reed_mat.set_shader_parameter("sway", 0.16)
	reed_mm.material_override = reed_mat
	reed_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	reed_mm.name = "Reeds"
	add_child(reed_mm)

func _refresh_litter() -> void:
	if litter_mm == null or player == null or terrain == null:
		return
	var buf: PackedFloat32Array = terrain.grass_field(player.theta, player.z, 28.0, 900)
	var stride: int = 6 if buf.size() % 6 == 0 else 5
	var n: int = int(buf.size() / float(stride))
	var litter_xf: Array = []
	var litter_col: Array = []
	var rock_xf: Array = []
	var rock_col: Array = []
	var reed_xf: Array = []
	var reed_col: Array = []
	for i in n:
		var th: float = buf[i * stride]
		var zz: float = buf[i * stride + 1]
		var gr: float = buf[i * stride + 2]
		var lush: float = buf[i * stride + 4] if stride >= 5 else 0.5
		var bid: int = clampi(int(buf[i * stride + 5]), 0, BIOME_GRASS_COL.size() - 1) if stride >= 6 else 3
		var h: float = _dhash(int(th * 1000.0), int(zz * 10.0))
		# Wetland / riparian / swamp → denser reeds in swamp.
		if bid == 1 or bid == 2 or bid == 9:
			var reed_gate := 0.48 if bid == 9 else 0.62
			if h > reed_gate:
				continue
			var xf := frame_at(th, zz, gr)
			var sc: float = (0.85 if bid == 9 else 0.7) + lush * 0.7
			xf.basis = xf.basis.scaled(Vector3(sc * 0.32, sc * (1.15 if bid == 9 else 1.0), sc * 0.32))
			reed_xf.append(xf)
			if bid == 9:
				reed_col.append(Color(0.10 + lush * 0.06, 0.32 + lush * 0.08, 0.16))
			else:
				reed_col.append(Color(0.14 + lush * 0.08, 0.38 + lush * 0.10, 0.18))
		elif bid == 10:
			# Meadow — flower flecks + pale thatch, not dark leaf litter.
			if h > 0.38:
				continue
			var lxf := frame_at(th, zz, gr + 0.02)
			var ls: float = 0.35 + lush * 0.45
			lxf.basis = lxf.basis.rotated(lxf.basis.y, h * TAU)
			lxf.basis = lxf.basis.scaled(Vector3(ls, 0.55, ls * 0.7))
			litter_xf.append(lxf)
			if h > 0.55:
				litter_col.append(Color(0.78, 0.48, 0.62)) # wildflower
			else:
				litter_col.append(Color(0.48, 0.52, 0.28)) # thatch
		elif bid == 6 or bid == 7 or bid == 11 or bid == 12:
			# Alpine / rock / desert / dune → rocks (sparse in desert).
			if bid >= 11 and h > 0.18:
				continue
			if bid < 11 and h > 0.28:
				continue
			var rxf := frame_at(th, zz, gr)
			var rs: float = 0.55 + h * 1.1
			rxf.basis = rxf.basis.rotated(rxf.basis.y, h * TAU)
			rxf.basis = rxf.basis.scaled(Vector3(rs, rs * 0.7, rs))
			rock_xf.append(rxf)
			if bid >= 11:
				rock_col.append(Color(0.62 + h * 0.08, 0.52, 0.34))
			else:
				rock_col.append(Color(0.46 + h * 0.08, 0.42, 0.36))
		elif bid == 13:
			# Shore litter — sparse pale shells/sticks.
			if h > 0.35:
				continue
			var lxf := frame_at(th, zz, gr + 0.02)
			var ls: float = 0.35 + lush * 0.4
			lxf.basis = lxf.basis.scaled(Vector3(ls, 0.5, ls * 0.7))
			litter_xf.append(lxf)
			litter_col.append(Color(0.62, 0.56, 0.42))
		else:
			if h > 0.22 + lush * 0.25:
				continue
			var lxf := frame_at(th, zz, gr + 0.02)
			var ls: float = 0.45 + lush * 0.55
			lxf.basis = lxf.basis.rotated(lxf.basis.y, h * TAU)
			lxf.basis = lxf.basis.scaled(Vector3(ls, 1.0, ls * 0.7))
			litter_xf.append(lxf)
			litter_col.append(Color(0.30 + lush * 0.08, 0.24 + lush * 0.06, 0.14))
	_fill_mm(litter_mm, litter_xf, litter_col)
	_fill_mm(rock_prop_mm, rock_xf, rock_col)
	_fill_mm(reed_mm, reed_xf, reed_col)

func _fill_mm(mi: MultiMeshInstance3D, xfs: Array, cols: Array) -> void:
	if mi == null:
		return
	var n: int = mini(xfs.size(), 400)
	mi.multimesh.instance_count = n
	for i in n:
		mi.multimesh.set_instance_transform(i, xfs[i])
		mi.multimesh.set_instance_color(i, cols[i])

func _build_rain() -> void:
	# Rain curtains under condensers — thin vertical streaks (Wave 7 / NEXT #9).
	# feature: streak length ~4–8 m; retire past ~180 m
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var qm := QuadMesh.new()
	qm.size = Vector2(0.08, 5.5)
	mm.mesh = qm
	mm.instance_count = 0
	rain_mm = MultiMeshInstance3D.new()
	rain_mm.multimesh = mm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.vertex_color_use_as_albedo = true
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	mat.distance_fade_mode = BaseMaterial3D.DISTANCE_FADE_PIXEL_DITHER
	mat.distance_fade_min_distance = 90.0
	mat.distance_fade_max_distance = 180.0
	rain_mm.material_override = mat
	rain_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	rain_mm.name = "RainCurtains"
	add_child(rain_mm)

func _refresh_rain() -> void:
	if rain_mm == null or terrain == null or player == null:
		return
	var storm_i: float = sky_intensity if sky_event == 2 else 0.0
	var buf: PackedFloat32Array = terrain.condensers_lod()
	var nc: int = int(buf.size() / 4.0)
	# Quiet weather with no nearby condensers — clear and bail.
	if storm_i < 0.05 and nc < 1:
		rain_mm.multimesh.instance_count = 0
		return
	var streaks: Array = []
	var cols: Array = []
	var R: float = float(P["radius"])
	var max_total := 96 if storm_i > 0.08 else 64
	for c in nc:
		if streaks.size() >= max_total:
			break
		var th: float = buf[c * 4]
		var zz: float = buf[c * 4 + 1]
		var power: float = buf[c * 4 + 2]
		var rad: float = maxf(buf[c * 4 + 3], 8.0)
		var dth: float = absf(wrapf(th - player.theta, -PI, PI)) * R
		var dz: float = absf(zz - player.z)
		if dth * dth + dz * dz > 180.0 * 180.0:
			continue
		# One ground sample per condenser — per-streak FFI was a hitch farm.
		var gr0: float = terrain.ground_radius(th, zz)
		var n_s: int = clampi(int(4.0 + power * 7.0 + storm_i * 8.0), 3, 18)
		n_s = mini(n_s, max_total - streaks.size())
		for s in n_s:
			var h: float = _dhash(c * 97 + s, int(zz * 3.0))
			var ath: float = th + ((h - 0.5) * 2.0 * rad) / R
			var az: float = zz + (_dhash(s, c) - 0.5) * rad * 1.4
			var loft: float = 2.0 + h * 6.0
			var xf := frame_at(ath, az, gr0 - loft)
			var fall: float = 1.0 + h * 0.8 + storm_i * 0.7
			xf.basis = xf.basis.scaled(Vector3(0.7 + power * 0.2, fall, 0.7))
			streaks.append(xf)
			var a: float = clampf(0.12 + power * 0.25 + storm_i * 0.18, 0.10, 0.48)
			cols.append(Color(0.72, 0.82, 0.92, a))
	# Ambient storm curtains — approximate height from player.r (no FFI).
	if storm_i > 0.08 and streaks.size() < max_total:
		var n_amb: int = clampi(int(10.0 + storm_i * 28.0), 8, 32)
		n_amb = mini(n_amb, max_total - streaks.size())
		var gr_p: float = player.r
		for s in n_amb:
			var h: float = _dhash(s * 13 + 7, int(player.z) + s)
			var ath: float = player.theta + ((h - 0.5) * 2.0 * 85.0) / R
			var az: float = player.z + (_dhash(s, 41) - 0.5) * 120.0
			var loft: float = 3.0 + h * 8.0
			var xf := frame_at(ath, az, gr_p - loft)
			var fall: float = 1.3 + h * 0.9 + storm_i * 0.7
			xf.basis = xf.basis.scaled(Vector3(0.55, fall, 0.55))
			streaks.append(xf)
			cols.append(Color(0.68, 0.78, 0.90, 0.14 + storm_i * 0.24))
	if streaks.is_empty():
		rain_mm.multimesh.instance_count = 0
		return
	_fill_mm(rain_mm, streaks, cols)

func _build_grazers() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var body := CapsuleMesh.new()
	body.radius = 0.22
	body.height = 0.85
	body.radial_segments = 6
	body.rings = 2
	mm.mesh = body
	mm.instance_count = 0
	grazer_mm = MultiMeshInstance3D.new()
	grazer_mm.multimesh = mm
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/prop.gdshader")
	mat.set_shader_parameter("haze_start", 60.0)
	mat.set_shader_parameter("haze_end", 240.0)
	grazer_mm.material_override = mat
	grazer_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	grazer_mm.name = "Grazers"
	add_child(grazer_mm)

func _refresh_grazers() -> void:
	if grazer_mm == null or player == null or terrain == null:
		return
	var buf: PackedFloat32Array = terrain.grazer_field(player.theta, player.z, 95.0, 80)
	var n: int = mini(int(buf.size() / 5.0), 80)
	grazer_mm.multimesh.instance_count = n
	var R: float = float(P["radius"])
	for i in n:
		var th: float = buf[i * 5]
		var zz: float = buf[i * 5 + 1]
		var dens: float = buf[i * 5 + 2]
		var fear: float = buf[i * 5 + 3]
		var mass: float = buf[i * 5 + 4]
		var ph: float = clock * (0.35 + dens * 0.4) + float(i) * 1.7
		var ath: float = th + cos(ph) * 1.8 * (1.0 - fear) / R
		var az: float = zz + sin(ph * 0.9) * 2.2 * (1.0 - fear)
		# Sample ground at the cell centre, not the jittered orbit — cuts FFI in half
		# visually and the animals still read as grazing.
		var gr: float = terrain.ground_radius(th, zz)
		var xf := frame_at(ath, az, gr)
		var sc: float = clampf(0.55 + mass / 120.0, 0.45, 1.35)
		xf.basis = xf.basis.scaled(Vector3(sc, sc, sc * 1.15))
		grazer_mm.multimesh.set_instance_transform(i, xf)
		var warm: float = dens * (1.0 - fear * 0.5)
		grazer_mm.multimesh.set_instance_color(i, Color(
				0.42 + warm * 0.12, 0.34 + warm * 0.06, 0.24))

func _build_splash() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	# Chips of thrown material, not droplets. Two crossed triangles read as
	# angular debris and cost four verts.
	var cst := SurfaceTool.new()
	cst.begin(Mesh.PRIMITIVE_TRIANGLES)
	for k in 2:
		var a: float = PI * 0.5 * float(k)
		var dx := cos(a) * 0.16
		var dz := sin(a) * 0.16
		var nn := Vector3(-sin(a), 0.5, cos(a)).normalized()
		for v in [Vector3(-dx, 0.0, -dz), Vector3(dx, 0.0, dz), Vector3(0.0, 0.26, 0.0)]:
			cst.set_normal(nn)
			cst.add_vertex(v)
	mm.mesh = cst.commit()
	# Start empty — lingering instance transforms used to reappear as orphan
	# "dig bubbles" after the burst finished.
	mm.instance_count = 0
	splash_mm = MultiMeshInstance3D.new()
	splash_mm.multimesh = mm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.albedo_color = Color(1, 1, 1, 1)
	splash_mm.material_override = mat
	splash_mm.name = "DigSplash"
	splash_mm.visible = false
	splash_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(splash_mm)

func _build_stockpiles() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	# A dug heap is angular rubble, not a ball. A SphereMesh here is what made
	# excavation leave rows of pale eggs on the ground.
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var sides := 7
	var rng0 := RandomNumberGenerator.new()
	rng0.seed = 5150
	var rim: Array = []
	for i in sides:
		var a: float = TAU * float(i) / float(sides)
		var rr: float = 0.42 + rng0.randf() * 0.16
		rim.append(Vector3(cos(a) * rr, rng0.randf() * 0.06, sin(a) * rr))
	var apex := Vector3(rng0.randf_range(-0.06, 0.06), 0.52,
			rng0.randf_range(-0.06, 0.06))
	for i in sides:
		var p0: Vector3 = rim[i]
		var p1: Vector3 = rim[(i + 1) % sides]
		# Side face up to the apex, then a floor triangle so it is closed.
		var n1 := (p1 - p0).cross(apex - p0).normalized()
		for v in [p0, p1, apex]:
			st.set_normal(n1)
			st.add_vertex(v)
		for v in [p1, p0, Vector3(0, 0, 0)]:
			st.set_normal(Vector3.DOWN)
			st.add_vertex(v)
	mm.mesh = st.commit()
	mm.instance_count = 0
	stockpile_mm = MultiMeshInstance3D.new()
	stockpile_mm.multimesh = mm
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/spoil.gdshader")
	stockpile_mm.material_override = mat
	stockpile_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	stockpile_mm.name = "SpoilHeaps"
	add_child(stockpile_mm)

## Agent colonists — same body cue as a spoil mark, taller (LANDSCAPE_2000 Wave 1).
func _build_agents() -> void:
	agent_root = Node3D.new()
	agent_root.name = "Colonists"
	add_child(agent_root)
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/prop.gdshader")
	mat.set_shader_parameter("albedo_scale", 0.92)
	mat.set_shader_parameter("haze_start", 70.0)
	mat.set_shader_parameter("haze_end", 420.0)
	for arch in RamaBody.ORDER:
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_colors = true
		mm.mesh = RamaBody.bake(RamaBody.make(arch))
		mm.instance_count = 0
		var mi := MultiMeshInstance3D.new()
		mi.multimesh = mm
		mi.material_override = mat
		mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		mi.name = "Far_%s" % arch
		agent_root.add_child(mi)
		agent_far[arch] = mi
	# Backwards-compatible handle for anything that still pokes at one node.
	agent_mm = agent_far[RamaBody.ORDER[0]]

	# The near pool. Six rigs is plenty: past sixty metres you cannot read a
	# gait anyway, and the silhouette tier already carries the build.
	for i in AGENT_RIGS:
		var holder := Node3D.new()
		holder.name = "Near%d" % i
		holder.visible = false
		agent_root.add_child(holder)
		agent_rigs.append({
			"node": holder, "rig": {}, "spec": {}, "gait": {},
			"id": -1, "phase": 0.0, "th": 0.0, "z": 0.0,
			"t_th": 0.0, "t_z": 0.0, "yaw": 0.0, "speed": 0.0, "warm": false,
			"gr": 0.0, "gr_age": 99.0,
		})
	# Plot claim — soft radial stain + rim (not a solid green dome).
	var pmm := MultiMesh.new()
	pmm.transform_format = MultiMesh.TRANSFORM_3D
	pmm.use_colors = true
	var disc := CylinderMesh.new()
	disc.top_radius = 1.0
	disc.bottom_radius = 1.0
	disc.height = 0.06
	disc.radial_segments = 32
	disc.rings = 1
	disc.cap_top = true
	disc.cap_bottom = false
	pmm.mesh = disc
	pmm.instance_count = 0
	plot_mm = MultiMeshInstance3D.new()
	plot_mm.multimesh = pmm
	var pmat := ShaderMaterial.new()
	pmat.shader = load("res://shaders/plot.gdshader")
	plot_mm.material_override = pmat
	plot_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	plot_mm.name = "ColonistPlots"
	add_child(plot_mm)

## Which build a colonist has. Stable for the life of the save, because it is
## derived from his id and nothing else — no table to persist, and the man you
## met yesterday is the same shape today.
func agent_archetype(id: int) -> String:
	var h: int = (id * 2654435761) ^ 0x9E3779B9
	h = (h ^ (h >> 15)) * 1274126177
	return RamaBody.ORDER[posmod(h ^ (h >> 13), RamaBody.ORDER.size())]

func refresh_agents() -> void:
	if agent_root == null or terrain == null:
		return
	var buf: PackedFloat32Array = terrain.agents_lod()
	var n: int = int(buf.size() / 9.0)
	if plot_mm:
		plot_mm.multimesh.instance_count = n
	for a in agent_far.keys():
		agent_far_xf[a] = []
		agent_far_col[a] = []

	# Nearest first — the pool goes to whoever you can actually see move.
	var here := Vector2(player.theta if player else 0.0, player.z if player else 0.0)
	var order: Array = []
	for i in n:
		var th: float = buf[i * 9]
		var zz: float = buf[i * 9 + 1]
		order.append([_arc_dist(here.x, here.y, th, zz), i])
	order.sort_custom(func(a, b): return float(a[0]) < float(b[0]))

	# Slots are STICKY. Assigning them nearest-first every refresh meant two men
	# swapping rank rebuilt both rigs — twenty-odd meshes each, several times a
	# second. Keep whoever is already seated and still in range; the rest of the
	# pool goes to the nearest man who has no slot.
	var seated := {}   # id -> slot
	for j in agent_rigs.size():
		if int(agent_rigs[j]["id"]) >= 0:
			seated[int(agent_rigs[j]["id"])] = j
	var taken := {}
	var free_slots: Array = []
	for e in order:
		var d0: float = float(e[0])
		var idx: int = int(e[1])
		var aid: int = int(buf[idx * 9 + 5])
		if seated.has(aid) and d0 < AGENT_RIG_RANGE * 1.3:
			taken[seated[aid]] = aid
	for j in agent_rigs.size():
		if not taken.has(j):
			free_slots.append(j)

	for e in order:
		var dist: float = float(e[0])
		var i: int = int(e[1])
		var th: float = buf[i * 9]
		var zz: float = buf[i * 9 + 1]
		var hunger: float = buf[i * 9 + 2]
		var fatigue: float = buf[i * 9 + 3]
		var mood: float = buf[i * 9 + 4]
		var id: int = int(buf[i * 9 + 5])
		var arch: String = agent_archetype(id)
		# Instance colour TINTS the baked mesh here — it does not replace it, so
		# it stays near white. (RENDER_CONTRACT §2009.)
		var tint := Color(
			clampf(0.94 + mood * 0.10, 0.6, 1.08),
			clampf(0.96 - hunger * 0.10, 0.6, 1.06),
			clampf(0.96 - fatigue * 0.12, 0.6, 1.06))
		var slot: int = -1
		if seated.has(id) and taken.get(seated[id], -1) == id:
			slot = seated[id]
		elif dist < AGENT_RIG_RANGE and not free_slots.is_empty():
			slot = free_slots.pop_front()
			taken[slot] = id
		if slot >= 0:
			_seat_agent_rig(slot, id, arch, th, zz, tint)
		else:
			var gr: float = terrain.ground_radius(th, zz)
			agent_far_xf[arch].append(frame_at(th, zz, gr))
			agent_far_col[arch].append(tint)
		if plot_mm:
			var pth: float = buf[i * 9 + 6]
			var pzz: float = buf[i * 9 + 7]
			var prad: float = maxf(buf[i * 9 + 8], 8.0)
			var pgr: float = terrain.ground_radius(pth, pzz)
			var pxf := frame_at(pth, pzz, pgr - 0.03)
			# Keep full claim radius in sim; visuals stay a soft ground stain.
			pxf.basis = pxf.basis.scaled(Vector3(prad, 1.0, prad))
			plot_mm.multimesh.set_instance_transform(i, pxf)
			plot_mm.multimesh.set_instance_color(i, Color(0.30, 0.55, 0.28, 0.15))

	for a in agent_far.keys():
		var mi: MultiMeshInstance3D = agent_far[a]
		var xfs: Array = agent_far_xf[a]
		mi.multimesh.instance_count = xfs.size()
		for j in xfs.size():
			mi.multimesh.set_instance_transform(j, xfs[j])
			mi.multimesh.set_instance_color(j, agent_far_col[a][j])
	for j in agent_rigs.size():
		if not taken.has(j):
			agent_rigs[j]["node"].visible = false
			agent_rigs[j]["id"] = -1

## Give a pool slot to a colonist, rebuilding the rig only when the man changes.
func _seat_agent_rig(slot: int, id: int, arch: String, th: float, zz: float, _tint: Color) -> void:
	var e: Dictionary = agent_rigs[slot]
	if int(e["id"]) != id:
		e["id"] = id
		# One man, not one of eight statues — `vary` moves every axis a little
		# off the archetype from his id alone.
		var spec: Dictionary = RamaBody.vary(RamaBody.make(arch), id)
		e["spec"] = spec
		e["rig"] = RamaBody.build(e["node"], spec)
		# The archetype's own walk, pulled toward what his actual body implies.
		e["gait"] = RamaGait.blend(RamaGait.make(arch), RamaGait.for_body(spec), 0.35)
		e["warm"] = false
	e["t_th"] = th
	e["t_z"] = zz
	if not bool(e["warm"]):
		e["th"] = th
		e["z"] = zz
		e["warm"] = true
	e["node"].visible = true

## Walk the near colonists toward wherever the sim last put them.
##
## The sim moves an agent a few times a second; a rig that snapped to that would
## teleport and its feet would mean nothing. So each rig CHASES its target at
## its own comfortable speed, and the gait is driven by the distance it actually
## covered — which makes arrival, hesitation and idle fall out for free.
func _tick_agent_rigs(dt: float) -> void:
	if player == null or terrain == null:
		return
	var R: float = float(P["radius"])
	for e in agent_rigs:
		if not bool(e["node"].visible) or (e["rig"] as Dictionary).is_empty():
			continue
		var g: Dictionary = e["gait"]
		var arc: float = wrapf(float(e["t_th"]) - float(e["th"]), -PI, PI) * R
		var dz: float = float(e["t_z"]) - float(e["z"])
		var gap: float = sqrt(arc * arc + dz * dz)
		var top: float = float(g["walk_speed"]) * 1.35
		var step: float = minf(gap, top * dt)
		var moved: float = 0.0
		if gap > 0.05:
			moved = step
			e["th"] = wrapf(float(e["th"]) + (arc / gap) * step / R, -PI, PI)
			e["z"] = float(e["z"]) + (dz / gap) * step
			var want: float = atan2(arc, dz)
			e["yaw"] = lerp_angle(float(e["yaw"]), want, clampf(dt * 6.0, 0.0, 1.0))
		var speed: float = moved / maxf(dt, 0.0001)
		e["speed"] = lerpf(float(e["speed"]), speed, clampf(dt * 8.0, 0.0, 1.0))
		var th: float = float(e["th"])
		var zz: float = float(e["z"])
		e["gr_age"] = float(e.get("gr_age", 99.0)) + dt
		# Re-sample hull height ~8 Hz or after a real step — every-frame FFI
		# for six near rigs was free hitch food on soft terrain.
		if float(e["gr_age"]) > 0.12 or moved > 0.35 or float(e.get("gr", 0.0)) < 1.0:
			e["gr"] = terrain.ground_radius(th, zz)
			e["gr_age"] = 0.0
		var gr: float = float(e["gr"])
		var xf := frame_at(th, zz, gr)
		xf.basis = xf.basis.rotated(xf.basis.y, float(e["yaw"]))
		e["node"].transform = xf
		e["phase"] = RamaGait.advance(float(e["phase"]), g,
				float((e["spec"] as Dictionary)["stature"]), float(e["speed"]), dt)
		e["phase"] = wrapf(float(e["phase"]), 0.0, TAU)
		RamaGait.pose(e["rig"], g, float(e["phase"]), float(e["speed"]),
				float(Time.get_ticks_msec()) * 0.001 + float(e["id"]) * 0.7)

## Dwellings — how each principal lives. A delve is a doorway in a hillside; a
## township is a cluster that grows. Works and followers are both READOUTS of
## the sim, not decoration: a place with six figures around it is feeding six
## people, and one with none has failed. LANDSCAPE_2200 depth-LOD rule.
## Walls are vertical, and the drum lights from the axis — straight down. So a
## wall gets almost no direct light and lands wherever `bounce_tint` puts it,
## which is dark green. These are authored bright on purpose so the AMBIENT
## term reads, not the lit term (§2012: display values, the shader scales them).
const WORK_KIND_COL := [
	Color(0.62, 0.58, 0.53),  # delve — cut stone, spoil-coloured
	Color(0.80, 0.64, 0.46),  # terrace — timber and rammed earth
	Color(0.90, 0.85, 0.73),  # township — plaster, the only pale thing out there
]
## Roofs take the light the walls cannot, so they are authored darker.
const ROOF_TINT := 0.62
## Metres works spread from the hearth, by kind. A delve barely spreads at all.
const WORK_SPREAD := [9.0, 20.0, 34.0]
const WORK_H := 2.4
const ROOF_H := 1.5

func _build_dwellings() -> void:
	var wm := MultiMesh.new()
	wm.transform_format = MultiMesh.TRANSFORM_3D
	wm.use_colors = true
	var hut := BoxMesh.new()
	hut.size = Vector3(2.7, WORK_H, 2.7)
	wm.mesh = hut
	wm.instance_count = 0
	work_mm = MultiMeshInstance3D.new()
	work_mm.multimesh = wm
	var wmat := ShaderMaterial.new()
	wmat.shader = load("res://shaders/prop.gdshader")
	work_mm.material_override = wmat
	work_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	work_mm.name = "Works"
	add_child(work_mm)
	dwelling_mats = [wmat]

	# A box is a crate; a box with a pitched roof is a building. One extra
	# draw call buys the entire silhouette.
	var rm := MultiMesh.new()
	rm.transform_format = MultiMesh.TRANSFORM_3D
	rm.use_colors = true
	var roof := CylinderMesh.new()
	roof.top_radius = 0.0
	roof.bottom_radius = 2.15
	roof.height = ROOF_H
	roof.radial_segments = 4
	roof.rings = 1
	roof.cap_bottom = false
	rm.mesh = roof
	rm.instance_count = 0
	roof_mm = MultiMeshInstance3D.new()
	roof_mm.multimesh = rm
	var rmat := ShaderMaterial.new()
	rmat.shader = load("res://shaders/prop.gdshader")
	roof_mm.material_override = rmat
	roof_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	roof_mm.name = "Roofs"
	add_child(roof_mm)
	dwelling_mats.append(rmat)

	var fm := MultiMesh.new()
	fm.transform_format = MultiMesh.TRANSFORM_3D
	fm.use_colors = true
	var body := CapsuleMesh.new()
	body.radius = 0.25
	body.height = 1.30
	body.radial_segments = 6
	body.rings = 2
	fm.mesh = body
	fm.instance_count = 0
	follower_mm = MultiMeshInstance3D.new()
	follower_mm.multimesh = fm
	var fmat := ShaderMaterial.new()
	fmat.shader = load("res://shaders/prop.gdshader")
	follower_mm.material_override = fmat
	follower_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	follower_mm.name = "Followers"
	add_child(follower_mm)

## Deterministic 0..1 from two ints — same layout every load, no stored state.
func _dhash(a: int, b: int) -> float:
	var h: int = (a * 73856093) ^ (b * 19349663)
	h = (h ^ (h >> 13)) * 1274126177
	return float((h ^ (h >> 16)) & 0xFFFFFF) / 16777215.0

func refresh_dwellings() -> void:
	if work_mm == null or terrain == null:
		return
	var buf: PackedFloat32Array = terrain.dwellings_lod()
	var n: int = int(buf.size() / 9.0)
	var hab_r: float = P["radius"]

	var wx: Array[Transform3D] = []
	var wc: Array[Color] = []
	var rx: Array[Transform3D] = []
	var rc: Array[Color] = []
	var fx: Array[Transform3D] = []
	var fc: Array[Color] = []

	for i in n:
		var th: float = buf[i * 9]
		var zz: float = buf[i * 9 + 1]
		var kind: int = clampi(int(buf[i * 9 + 2]), 0, 2)
		var followers: float = buf[i * 9 + 3]
		var works: int = int(buf[i * 9 + 4])
		var quality: float = buf[i * 9 + 7]
		var spread: float = WORK_SPREAD[kind]
		var base: Color = WORK_KIND_COL[kind]

		for w in works:
			# Golden-angle scatter so a growing township spirals outward
			# instead of stacking rings.
			var a: float = float(w) * 2.39996 + _dhash(i, 0) * TAU
			var rad: float = spread * sqrt((float(w) + 0.6) / maxf(float(works), 1.0))
			var ox: float = cos(a) * rad
			var oz: float = sin(a) * rad
			var wth: float = th + ox / hab_r
			var wzz: float = zz + oz
			# Up on a drum is DECREASING radius, so a thing sits on the ground
			# at (ground - half its height), never ground + anything.
			var gr: float = terrain.ground_radius(wth, wzz)
			var hs: float = 0.55 + _dhash(i, w + 31) * 0.35
			# A delve's works are cut into the hill: squat, and sunk enough to
			# read as a doorway rather than a shed someone left on a mountain.
			var sy: float = 0.55 if kind == 0 else 0.8 + hs * 0.6
			var sxz: float = 1.15 if kind == 0 else 1.0
			var seat: float = WORK_H * 0.5 * sy
			if kind == 0:
				seat *= 0.45
			var xf: Transform3D = frame_at(wth, wzz, gr - seat)
			xf.basis = xf.basis.rotated(xf.basis.y.normalized(), _dhash(i, w + 7) * TAU)
			# scaled() scales in GLOBAL axes; on a cylinder that squashes the
			# box along a world axis instead of its own up. scaled_local() is
			# the one that means "taller".
			xf.basis = xf.basis.scaled_local(Vector3(sxz, sy, sxz))
			wx.append(xf)
			# Poor ground shows on the buildings before it shows in a readout.
			var wear: float = 0.82 + clampf(quality, 0.0, 1.4) * 0.16
			var wcol := Color(base.r * wear, base.g * wear, base.b * wear)
			wc.append(wcol)
			# Roof rides on the wall top, turned 45 deg so its ridge crosses
			# the walls instead of lining up with them.
			var rxf: Transform3D = frame_at(wth, wzz, gr - seat * 2.0 - ROOF_H * 0.5)
			rxf.basis = xf.basis.rotated(xf.basis.y.normalized(), PI * 0.25)
			rxf.basis = rxf.basis.scaled_local(Vector3(sxz, 1.0, sxz))
			rx.append(rxf)
			rc.append(Color(wcol.r * ROOF_TINT, wcol.g * ROOF_TINT, wcol.b * ROOF_TINT))

		var fn_i: int = int(round(followers))
		for f in fn_i:
			var fa: float = _dhash(i, f + 101) * TAU
			var frad: float = 3.0 + _dhash(i, f + 211) * spread * 0.8
			var fth: float = th + cos(fa) * frad / hab_r
			var fzz: float = zz + sin(fa) * frad
			var fgr: float = terrain.ground_radius(fth, fzz)
			var fxf: Transform3D = frame_at(fth, fzz, fgr - 0.72)
			fxf.basis = fxf.basis.rotated(fxf.basis.y.normalized(), _dhash(i, f + 307) * TAU)
			fx.append(fxf)
			# Cooler and plainer than a principal, but still lit like a person.
			# Too dark and a row of them reads as fence posts.
			var v: float = 0.78 + _dhash(i, f + 401) * 0.14
			fc.append(Color(v, v * 0.88, v * 0.78))

	work_mm.multimesh.instance_count = wx.size()
	for i in wx.size():
		work_mm.multimesh.set_instance_transform(i, wx[i])
		work_mm.multimesh.set_instance_color(i, wc[i])
	roof_mm.multimesh.instance_count = rx.size()
	for i in rx.size():
		roof_mm.multimesh.set_instance_transform(i, rx[i])
		roof_mm.multimesh.set_instance_color(i, rc[i])
	follower_mm.multimesh.instance_count = fx.size()
	for i in fx.size():
		follower_mm.multimesh.set_instance_transform(i, fx[i])
		follower_mm.multimesh.set_instance_color(i, fc[i])

func _build_carcasses() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var body := BoxMesh.new()
	body.size = Vector3(0.9, 0.35, 0.55)
	mm.mesh = body
	mm.instance_count = 0
	carcass_mm = MultiMeshInstance3D.new()
	carcass_mm.multimesh = mm
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/prop.gdshader")
	carcass_mm.material_override = mat
	carcass_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	carcass_mm.name = "Carcasses"
	add_child(carcass_mm)

func refresh_carcasses() -> void:
	if carcass_mm == null or terrain == null:
		return
	var buf: PackedFloat32Array = terrain.carcasses_lod()
	var n: int = int(buf.size() / 5.0)
	carcass_mm.multimesh.instance_count = n
	for i in n:
		var th: float = buf[i * 5]
		var zz: float = buf[i * 5 + 1]
		var mass: float = buf[i * 5 + 2]
		var stage: int = int(buf[i * 5 + 4])
		var rad: float = clampf(0.35 + mass * 0.012, 0.4, 1.6)
		var gr: float = terrain.ground_radius(th, zz)
		var xf := frame_at(th, zz, gr - rad * 0.2)
		xf.basis = xf.basis.scaled(Vector3(rad, rad * 0.55, rad * 0.75))
		carcass_mm.multimesh.set_instance_transform(i, xf)
		var col := Color(0.42, 0.28, 0.22)
		if stage == 1:
			col = Color(0.35, 0.32, 0.28)
		elif stage >= 2:
			col = Color(0.72, 0.68, 0.58)
		carcass_mm.multimesh.set_instance_color(i, col)

func _build_stations() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var body := BoxMesh.new()
	body.size = Vector3(1.4, 1.1, 1.4)
	mm.mesh = body
	mm.instance_count = 0
	station_mm = MultiMeshInstance3D.new()
	station_mm.multimesh = mm
	# An unshaded StandardMaterial3D with a default-WHITE albedo_color renders
	# instanced props as flat bright blocks with no shading at all. This was
	# the mystery white cube. Use the shared prop shader instead.
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/prop.gdshader")
	station_mm.material_override = mat
	station_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	station_mm.name = "CraftStations"
	add_child(station_mm)
	refresh_stations()

func refresh_stations() -> void:
	if station_mm == null or terrain == null:
		return
	var buf: PackedFloat32Array = terrain.stations_lod()
	var n: int = int(buf.size() / 4.0)
	station_mm.multimesh.instance_count = n
	for i in n:
		var th: float = buf[i * 4]
		var zz: float = buf[i * 4 + 1]
		var code: int = int(buf[i * 4 + 2])
		var gr: float = terrain.ground_radius(th, zz)
		var xf := frame_at(th, zz, gr - 0.7)
		station_mm.multimesh.set_instance_transform(i, xf)
		var col := Color(0.55, 0.45, 0.35)
		if code == 1:
			col = Color(0.55, 0.62, 0.70)  # kitchen
		elif code == 2:
			col = Color(0.55, 0.32, 0.22)  # kiln
		elif code == 3:
			col = Color(0.35, 0.38, 0.42)  # smelter
		station_mm.multimesh.set_instance_color(i, col)

func _build_steam() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var puff := SphereMesh.new()
	puff.radius = 0.28
	puff.height = 0.40
	puff.radial_segments = 12
	puff.rings = 6
	mm.mesh = puff
	mm.instance_count = 0
	steam_mm = MultiMeshInstance3D.new()
	steam_mm.multimesh = mm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.distance_fade_mode = BaseMaterial3D.DISTANCE_FADE_PIXEL_DITHER
	mat.distance_fade_min_distance = 18.0
	mat.distance_fade_max_distance = 42.0
	steam_mm.material_override = mat
	steam_mm.visible = false
	steam_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	steam_mm.name = "DawnSteam"
	add_child(steam_mm)

## Spoil heaps from dig overflow / dropped pack (LANDSCAPE_1400 item 822).
func refresh_stockpiles() -> void:
	stockpile_refresh_in = -1.0
	if stockpile_mm == null or terrain == null:
		return
	var buf: PackedFloat32Array = terrain.stockpiles()
	var n: int = int(buf.size() / 7.0)
	stockpile_mm.multimesh.instance_count = n
	for i in n:
		var th: float = buf[i * 7]
		var zz: float = buf[i * 7 + 1]
		var mid: int = int(buf[i * 7 + 2])
		var rad: float = maxf(buf[i * 7 + 6], 0.25)
		# Soft visual compress — mass still tracks, pile just reads lower.
		var vr: float = 0.55 + sqrt(rad) * 0.95
		vr = clampf(vr, 0.45, 2.6)
		var gr: float = terrain.ground_radius(th, zz)
		var xf := frame_at(th, zz, gr - vr * 0.18)
		xf.basis = xf.basis.scaled(Vector3(vr, vr * 0.38, vr))
		stockpile_mm.multimesh.set_instance_transform(i, xf)
		var col := Color(0.46, 0.36, 0.26)
		match mid:
			2: col = Color(0.44, 0.30, 0.22)  # clay
			3: col = Color(0.58, 0.46, 0.32)  # sandstone
			4: col = Color(0.28, 0.28, 0.30)  # basalt
			5: col = Color(0.40, 0.26, 0.18)  # ferrous
			6: col = Color(0.72, 0.80, 0.88)  # ice
			100: col = Color(0.30, 0.48, 0.24) # greens
			101: col = Color(0.48, 0.34, 0.20) # timber
			102: col = Color(0.55, 0.48, 0.30) # fibre
			103: col = Color(0.62, 0.52, 0.28) # seed
			104: col = Color(0.28, 0.52, 0.78) # water
		stockpile_mm.multimesh.set_instance_color(i, col)

## Debounced — holding dig used to rebuild the Multimesh every bite.
func schedule_stockpile_refresh() -> void:
	if stockpile_refresh_in < 0.0:
		stockpile_refresh_in = 0.28

func _tick_stockpile_refresh(dt: float) -> void:
	if stockpile_refresh_in < 0.0:
		return
	stockpile_refresh_in -= dt
	if stockpile_refresh_in > 0.0:
		return
	refresh_stockpiles()

const SPLASH_N := 18
const SPLASH_LIFE := 0.38

func spawn_dig_splash(p: Vector3, strength: float) -> void:
	if splash_mm == null:
		return
	splash_origin = p
	splash_life = SPLASH_LIFE
	splash_mm.visible = true
	splash_mm.multimesh.instance_count = SPLASH_N
	var up := Vector3(-p.x, -p.y, 0.0)
	if up.length_squared() < 1e-6:
		up = Vector3.UP
	else:
		up = up.normalized()
	var rng := RandomNumberGenerator.new()
	rng.seed = int(Time.get_ticks_msec())
	for i in SPLASH_N:
		var lateral := Vector3(rng.randf_range(-1, 1), rng.randf_range(-1, 1), rng.randf_range(-1, 1))
		lateral = lateral - up * lateral.dot(up)
		if lateral.length_squared() < 1e-6:
			lateral = up.cross(Vector3.FORWARD)
		lateral = lateral.normalized()
		var dist: float = rng.randf_range(0.25, 1.4 + strength * 0.55)
		var loft: float = rng.randf_range(0.15, 0.85 + strength * 0.25)
		var xf := Transform3D(Basis.IDENTITY, p + lateral * dist + up * loft)
		var s: float = rng.randf_range(0.35, 0.85)
		xf.basis = xf.basis.scaled(Vector3(s, s, s))
		splash_mm.multimesh.set_instance_transform(i, xf)
		splash_mm.multimesh.set_instance_color(i, Color(0.62, 0.82, 0.92, 0.55))
	if audio:
		audio.splash(strength)

func _clear_splash() -> void:
	splash_life = 0.0
	if splash_mm == null:
		return
	splash_mm.visible = false
	splash_mm.multimesh.instance_count = 0

func _tick_splash(dt: float) -> void:
	if splash_mm == null:
		return
	if splash_life <= 0.0:
		if splash_mm.visible or splash_mm.multimesh.instance_count > 0:
			_clear_splash()
		return
	splash_life -= dt
	var fade: float = clampf(splash_life / SPLASH_LIFE, 0.0, 1.0)
	# Ease out so droplets vanish instead of freezing mid-air.
	fade = fade * fade
	var up := Vector3(-splash_origin.x, -splash_origin.y, 0.0)
	if up.length_squared() < 1e-6:
		up = Vector3.UP
	else:
		up = up.normalized()
	var n: int = splash_mm.multimesh.instance_count
	for i in n:
		var xf: Transform3D = splash_mm.multimesh.get_instance_transform(i)
		var outward: Vector3 = xf.origin - splash_origin
		if outward.length_squared() > 1e-6:
			outward = outward.normalized()
		else:
			outward = up
		xf.origin += up * (2.4 * dt) + outward * (3.2 * dt)
		# Shrink as they fade so nothing reads as a stuck bead.
		var s: float = 0.35 + 0.65 * fade
		xf.basis = Basis.IDENTITY.scaled(Vector3(s, s, s))
		splash_mm.multimesh.set_instance_transform(i, xf)
		splash_mm.multimesh.set_instance_color(i, Color(0.62, 0.82, 0.92, 0.55 * fade))
	if splash_life <= 0.0:
		_clear_splash()

func schedule_pool_refresh() -> void:
	pool_refresh_in = 0.55 if dig_held else 0.35

## After digging, elevation changed — recompute live drainage and refresh
## instruments that read it. Debounced so holding dig doesn't hitch every bite.
func schedule_flow_refresh() -> void:
	if flow_refresh_in < 0.0:
		terrain.snapshot_routes()
	# Hold dig: wait until the burst settles before priority-flood + river rebuild.
	flow_refresh_in = 0.70 if dig_held else 0.28

func _tick_flow_refresh(dt: float) -> void:
	if flow_refresh_in < 0.0:
		return
	# Keep postponing while the brush is still carving.
	if dig_held:
		flow_refresh_in = maxf(flow_refresh_in - dt, 0.45)
		return
	flow_refresh_in -= dt
	if flow_refresh_in > 0.0:
		return
	flow_refresh_in = -1.0
	if not terrain.flow_dirty():
		return
	var t0 := Time.get_ticks_msec()
	var changed: bool = terrain.refresh_flow()
	if changed:
		_build_rivers()
		_build_pools()
		_refresh_biome_map()
		census = terrain.biome_census(260)
		print("[rama] flow refreshed in %d ms" % [Time.get_ticks_msec() - t0])

func _tick_pool_refresh(dt: float) -> void:
	if pool_refresh_in < 0.0:
		return
	pool_refresh_in -= dt
	if pool_refresh_in > 0.0:
		return
	pool_refresh_in = -1.0
	_build_pools()
	_refresh_foam()

func _refresh_wet_colors(full_ring: bool = false) -> void:
	# Queue colour patches and drain a few per frame — never remesh 10–48
	# chunks in one hitch (was firing every sim cadence while walking).
	if chunk_root == null or player == null:
		return
	wet_chunk_queue.clear()
	var cap := 48 if full_ring else 14
	var n := 0
	for c in chunk_root.get_children():
		if not (c is MeshInstance3D) or c.mesh == null:
			continue
		wet_chunk_queue.append(c)
		n += 1
		if n >= cap:
			break

func _drain_wet_colors(budget: int = 2) -> void:
	var done := 0
	while wet_chunk_queue.size() > 0 and done < budget:
		var c: MeshInstance3D = wet_chunk_queue.pop_front()
		if c == null or not is_instance_valid(c) or c.mesh == null:
			continue
		var mesh: ArrayMesh = c.mesh
		if mesh.get_surface_count() < 1:
			continue
		var arrays: Array = mesh.surface_get_arrays(0)
		var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		if verts.is_empty():
			continue
		arrays[Mesh.ARRAY_COLOR] = terrain.colors_at(verts)
		var am := ArrayMesh.new()
		am.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		c.mesh = am
		done += 1

func refresh_catchment_ribbon() -> void:
	if catchment_root and is_instance_valid(catchment_root):
		catchment_root.queue_free()
	catchment_root = Node3D.new()
	catchment_root.name = "CatchmentRibbon"
	add_child(catchment_root)
	catchment_mat = null
	if catchment_cache.is_empty() or player == null:
		return
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var ink := Color(0.28, 0.58, 0.52, 0.18)
	var i := 0
	while i + 1 < catchment_cache.size():
		var th: float = catchment_cache[i]
		var zz: float = catchment_cache[i + 1]
		i += 2
		var gr := ground_at(th, zz)
		var p := to_world(th, zz, gr)
		var up := Vector3(-p.x, -p.y, 0.0).normalized()
		var axial := Vector3(0, 0, 1)
		var side := up.cross(axial).normalized() * 1.15
		var lift := up * 0.32
		var a := p - side + lift
		var b := p + side + lift
		var c := p + axial * 1.15 + lift
		var d := p - axial * 1.15 + lift
		for v in [a, b, c, a, d, b]:
			st.set_normal(up)
			st.set_color(ink)
			st.add_vertex(v)
	var mi := MeshInstance3D.new()
	mi.mesh = st.commit()
	catchment_mat = StandardMaterial3D.new()
	catchment_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	catchment_mat.vertex_color_use_as_albedo = true
	catchment_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	catchment_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	catchment_mat.distance_fade_mode = BaseMaterial3D.DISTANCE_FADE_PIXEL_DITHER
	catchment_mat.distance_fade_min_distance = 55.0
	catchment_mat.distance_fade_max_distance = 140.0
	mi.material_override = catchment_mat
	catchment_root.add_child(mi)

func _refresh_biome_map() -> void:
	if biome_tex_rect == null:
		return
	var img := Image.create_from_data(384, 240, false, Image.FORMAT_RGB8,
			terrain.biome_map(384, 240))
	biome_tex_rect.texture = ImageTexture.create_from_image(img)
	if map_label:
		map_label.text = "KEPLER DRUM · live drainage"

func _build_axis_light() -> void:
	# Photothermal Spine: dim always-on core + traveling day-carriage + endcap
	# light rings. Day length is a schedule, not an orbit.
	var half_l: float = float(P["length"]) * 0.49
	var L: float = half_l * 2.0

	# --- SpineCore (night safety fill) ---
	var spine := MeshInstance3D.new()
	spine.mesh = _make_axis_tube(half_l, 4.2, 12,
			Color(0.72, 0.78, 0.88), Color(0.55, 0.62, 0.75))
	spine_mat = _make_emissive_mat(Color(0.75, 0.82, 0.92), 0.55)
	spine.material_override = spine_mat
	spine.name = "SpineCore"
	spine.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(spine)
	axis_mat = spine_mat  # compat: daylight still tweaks "axis"

	# --- DayCarriage (bright traveling segment) ---
	var car_half: float = L * CARRIAGE_FRAC * 0.5
	day_carriage = MeshInstance3D.new()
	day_carriage.mesh = _make_axis_tube(car_half, 7.2, 16,
			Color(1.0, 0.96, 0.88), Color(1.0, 0.90, 0.72))
	carriage_mat = _make_emissive_mat(Color(1.0, 0.94, 0.82), 3.2)
	day_carriage.material_override = carriage_mat
	day_carriage.name = "DayCarriage"
	day_carriage.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(day_carriage)
	carriage_z = -half_l
	day_carriage.position = Vector3(0, 0, carriage_z)

	# --- Endcap light rings (horizon suns / safety) ---
	var ring_z: float = half_l * 0.985
	endcap_ring_a = _make_endcap_ring("EndcapRingNeg", -ring_z)
	endcap_ring_b = _make_endcap_ring("EndcapRingPos", ring_z)
	ring_mat_a = endcap_ring_a.material_override as StandardMaterial3D
	ring_mat_b = endcap_ring_b.material_override as StandardMaterial3D
	add_child(endcap_ring_a)
	add_child(endcap_ring_b)

func _make_emissive_mat(col: Color, energy: float) -> StandardMaterial3D:
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	mat.emission_enabled = true
	mat.emission = col
	mat.emission_energy_multiplier = energy
	mat.albedo_color = col
	return mat

func _make_axis_tube(half_z: float, rad: float, seg: int, warm: Color, cool: Color) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for i in seg:
		var a0 := TAU * float(i) / float(seg)
		var a1 := TAU * float(i + 1) / float(seg)
		var x0 := rad * cos(a0)
		var y0 := rad * sin(a0)
		var x1 := rad * cos(a1)
		var y1 := rad * sin(a1)
		for v_pair in [
			[Vector3(x0, y0, -half_z), cool], [Vector3(x0, y0, half_z), warm],
			[Vector3(x1, y1, half_z), warm], [Vector3(x0, y0, -half_z), cool],
			[Vector3(x1, y1, half_z), warm], [Vector3(x1, y1, -half_z), cool]]:
			var vp: Vector3 = v_pair[0]
			var zt: float = 1.0 - clampf(absf(vp.z) / maxf(half_z, 0.01), 0.0, 1.0)
			st.set_color(warm.lerp(cool, 1.0 - zt * zt))
			st.set_normal(Vector3(vp.x, vp.y, 0).normalized())
			st.add_vertex(vp)
	return st.commit()

func _make_endcap_ring(node_name: String, z: float) -> MeshInstance3D:
	# Thin torus-like ring around the axis at the endcap — "horizon sun".
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var seg := 48
	var r0 := 28.0
	var r1 := 42.0
	var thick := 3.5
	for i in seg:
		var a0 := TAU * float(i) / float(seg)
		var a1 := TAU * float(i + 1) / float(seg)
		# Outer quad strip (face toward mid-habitat).
		var c0 := Vector3(r0 * cos(a0), r0 * sin(a0), 0)
		var c1 := Vector3(r0 * cos(a1), r0 * sin(a1), 0)
		var d0 := Vector3(r1 * cos(a0), r1 * sin(a0), 0)
		var d1 := Vector3(r1 * cos(a1), r1 * sin(a1), 0)
		var nsgn: float = -1.0 if z > 0.0 else 1.0
		var front := Vector3(0, 0, nsgn * thick * 0.5)
		var back := Vector3(0, 0, -nsgn * thick * 0.5)
		var col := Color(1.0, 0.92, 0.78)
		for tri in [
			[c0 + front, d0 + front, d1 + front], [c0 + front, d1 + front, c1 + front],
			[c0 + back, d1 + back, d0 + back], [c0 + back, c1 + back, d1 + back]]:
			for j in 3:
				var vp: Vector3 = tri[j]
				st.set_color(col)
				st.set_normal(Vector3(0, 0, nsgn))
				st.add_vertex(vp)
	var mi := MeshInstance3D.new()
	mi.mesh = st.commit()
	mi.material_override = _make_emissive_mat(Color(1.0, 0.90, 0.75), 1.1)
	mi.name = node_name
	mi.position = Vector3(0, 0, z)
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	return mi

## Day-carriage progress 0..1 along +z with noon linger (dawn wave §BE).
func _carriage_progress(phase: float) -> float:
	if phase < 0.02:
		return 0.0
	if phase >= 0.90:
		return 1.0
	var p: float = (phase - 0.02) / 0.88
	if p < 0.32:
		return smoothstep(0.0, 0.32, p) * 0.40
	if p < 0.58:
		return 0.40 + (p - 0.32) / 0.26 * 0.20  # linger mid-habitat
	return 0.60 + smoothstep(0.58, 1.0, p) * 0.40

func _sync_photothermal_spine(phase: float, band: String) -> void:
	var half_l: float = float(P.get("length", 6000.0)) * 0.49
	var prog: float = _carriage_progress(phase)
	carriage_z = lerpf(-half_l, half_l, prog)
	if day_carriage:
		day_carriage.position = Vector3(0, 0, carriage_z)
	# Faster pulse when day is sped up so the carriage still “breathes”.
	var pulse: float = 1.0 + sin(clock * TAU / maxf(120.0 / maxf(day_speed, 1.0), 8.0)) * 0.05
	var warm := Color(1.0, 0.94, 0.80).lerp(Color(1.0, 0.55, 0.32), 1.0 - day)
	# Sunset hour: copper → rose as carriage exits +z end.
	var sunset: float = 0.0
	if band == "dusk":
		sunset = smoothstep(0.68, 0.86, phase)
		warm = warm.lerp(Color(1.0, 0.58, 0.28), 0.35 + 0.45 * sunset)
		warm = warm.lerp(Color(1.0, 0.42, 0.55), sunset * 0.28)
	elif band == "dawn":
		warm = warm.lerp(Color(1.0, 0.78, 0.55), 0.4)
	# Spine: always-on dim fill.
	if spine_mat:
		spine_mat.emission = Color(0.70, 0.78, 0.90).lerp(warm, day * 0.35 + sunset * 0.25)
		spine_mat.emission_energy_multiplier = (0.35 + 0.45 * day + sunset * 0.55) * pulse
		spine_mat.albedo_color = spine_mat.emission
	# Carriage: bright when in daylight schedule; flare as it leaves (sunset).
	var car_on: float = day
	if phase > 0.88 or phase < 0.02:
		car_on *= 0.15
	if carriage_mat:
		carriage_mat.emission = warm
		var car_e: float = 1.2 + 2.8 * car_on + sunset * 2.2
		carriage_mat.emission_energy_multiplier = car_e * pulse
		carriage_mat.albedo_color = warm
		if day_carriage:
			day_carriage.visible = car_on > 0.05 or day > 0.08 or sunset > 0.05
	# Rings brighten when carriage nears that end (dawn/dusk contact).
	var near_neg: float = 1.0 - clampf(absf(carriage_z - (-half_l)) / 700.0, 0.0, 1.0)
	var near_pos: float = 1.0 - clampf(absf(carriage_z - half_l) / 700.0, 0.0, 1.0)
	if ring_mat_a:
		ring_mat_a.emission = warm.lerp(Color(0.85, 0.88, 1.0), 0.25)
		ring_mat_a.emission_energy_multiplier = (0.55 + 1.8 * near_neg * day + 0.35 * (1.0 - day)
				+ near_neg * sunset * 1.4) * pulse
	if ring_mat_b:
		# +z ring catches the departing carriage — golden-hour flare.
		ring_mat_b.emission = warm.lerp(Color(1.0, 0.72, 0.42), 0.15 + sunset * 0.55)
		ring_mat_b.emission_energy_multiplier = (0.55 + 1.8 * near_pos * day + 0.35 * (1.0 - day)
				+ near_pos * (1.2 + sunset * 3.5)) * pulse
	# Thermohydronic cue: steam when carriage passes the player.
	if player != null and day > 0.15:
		if absf(player.z - carriage_z) < 120.0 and absf(carriage_z - last_carriage_steam_z) > 80.0:
			steam_life = maxf(steam_life, 2.8)
			last_carriage_steam_z = carriage_z
	# Push schedule into sim so plants share the spectacle clock — throttled;
	# every-frame FFI was free hitch food and plants don't need sub-degree phase.
	if terrain != null and terrain.has_method("set_day_schedule"):
		if absf(phase - _sched_phase) > 0.003 or absf(day - _sched_day) > 0.012 \
				or absf(carriage_z - _sched_cz) > 12.0:
			_sched_phase = phase
			_sched_day = day
			_sched_cz = carriage_z
			terrain.set_day_schedule(phase, day, carriage_z)

func _build_clouds() -> void:
	# A band at one radius: in a drum, "altitude" is a radius and the
	# condensers run at a fixed level. (SIM_ARCH_BRIEF.md §3.6)
	var r: float = float(P["radius"]) - 300.0
	var L: float = float(P["length"])
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var seg := 192
	for i in seg:
		var a0 := TAU * i / seg
		var a1 := TAU * (i + 1) / seg
		var p00 := Vector3(r * cos(a0), r * sin(a0), -L * 0.5)
		var p10 := Vector3(r * cos(a1), r * sin(a1), -L * 0.5)
		var p01 := Vector3(r * cos(a0), r * sin(a0), L * 0.5)
		var p11 := Vector3(r * cos(a1), r * sin(a1), L * 0.5)
		for v in [p00, p01, p11, p00, p11, p10]:
			st.set_normal(Vector3(-v.x, -v.y, 0).normalized())
			st.add_vertex(v)
	var mi := MeshInstance3D.new()
	mi.mesh = st.commit()
	var sm := ShaderMaterial.new()
	sm.shader = load("res://shaders/clouds.gdshader")
	mi.material_override = sm
	mi.name = "Clouds"
	cloud_node = mi
	add_child(mi)

func _build_dust() -> void:
	# Motes + pollen near the player. Cheap air life — depth-of-field fodder
	# and the single biggest "there is atmosphere here" cue at walking range.
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var qm := QuadMesh.new()
	qm.size = Vector2(0.055, 0.055)
	mm.mesh = qm
	mm.instance_count = 720
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	for i in mm.instance_count:
		var t := Transform3D()
		t.origin = Vector3(rng.randf_range(-30, 30), rng.randf_range(-6, 20),
				rng.randf_range(-30, 30))
		var s: float = rng.randf_range(0.55, 1.45)
		t.basis = t.basis.scaled(Vector3(s, s, s))
		mm.set_instance_transform(i, t)
		# Mix warm dust with green-gold pollen so the air isn't one tint.
		var pollen: float = rng.randf()
		var col: Color
		if pollen > 0.62:
			col = Color(0.72, 0.88, 0.42, rng.randf_range(0.10, 0.28))
		elif pollen > 0.35:
			col = Color(1.0, 0.94, 0.78, rng.randf_range(0.08, 0.26))
		else:
			col = Color(0.92, 0.96, 1.0, rng.randf_range(0.06, 0.18))
		mm.set_instance_color(i, col)
	dust = MultiMeshInstance3D.new()
	dust.multimesh = mm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.vertex_color_use_as_albedo = true
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	mat.albedo_color = Color(1, 0.97, 0.90)
	dust.material_override = mat
	dust.name = "Dust"
	add_child(dust)

func _build_insects() -> void:
	# Tiny warm flecks that orbit the walk space — birds/insects at miniature
	# scale. Start empty so identity transforms don't park black squares at the
	# axis (or on the first slope the camera sees).
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var qm := QuadMesh.new()
	qm.size = Vector2(0.035, 0.035)
	mm.mesh = qm
	mm.instance_count = 0
	insects = MultiMeshInstance3D.new()
	insects.multimesh = mm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.vertex_color_use_as_albedo = true
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	insects.material_override = mat
	insects.name = "Insects"
	insects.visible = false
	add_child(insects)

func _box(size: Vector3, col: Color, emissive := false) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	var bm := BoxMesh.new()
	bm.size = size
	mi.mesh = bm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = col
	# Distance fade so town blocks dissolve into fog instead of floating as
	# sharp cardboard cutouts against the sky.
	mat.distance_fade_mode = BaseMaterial3D.DISTANCE_FADE_PIXEL_DITHER
	mat.distance_fade_min_distance = 900.0
	mat.distance_fade_max_distance = 2400.0
	if emissive:
		# A near-white albedo PLUS emission at 1.6x plus bloom blows out to a
		# featureless white block — which is exactly what the mystery white
		# cube was. Darken the surface and let a warm emission do the work, so
		# it reads as a lit window rather than a hole in the world.
		mat.albedo_color = Color(col.r * 0.30, col.g * 0.26, col.b * 0.20)
		mat.emission_enabled = true
		mat.emission = Color(col.r, col.g * 0.82, col.b * 0.52)
		mat.emission_energy_multiplier = 0.85
	mi.material_override = mat
	return mi

func ground_at(theta: float, z: float) -> float:
	# Start below local relief so tall cliff faces still ray-hit (Wave 9 / 4001).
	var gr: float = terrain.ground_radius(theta, z)
	var pad: float = 40.0
	if P.has("max_elevation"):
		pad = maxf(40.0, float(P["max_elevation"]) * 0.15)
	var start: float = gr - pad
	return terrain.ground_below(theta, z, start)

func _build_homestead() -> void:
	var th: float = spawn["theta"]
	var z: float = spawn["z"]
	var gr: float = ground_at(th, z)

	# House: sits a few metres from where you wake up.
	var hz := z + 9.0
	var hr := ground_at(th, hz)
	var house := Node3D.new()
	house.transform = frame_at(th, hz, hr)
	add_child(house)
	var walls := _box(Vector3(8.0, 3.4, 6.0), Color(0.42, 0.36, 0.30))
	walls.position = Vector3(0, 1.7, 0)
	house.add_child(walls)
	var roof := _box(Vector3(9.0, 0.5, 7.0), Color(0.30, 0.24, 0.21))
	roof.position = Vector3(0, 3.6, 0)
	house.add_child(roof)
	for dz in [-1.6, 1.6]:
		var win := _box(Vector3(0.1, 0.9, 1.1), Color(1.0, 0.82, 0.52), true)
		win.position = Vector3(4.02, 1.9, dz)
		house.add_child(win)
	var door := _box(Vector3(0.12, 2.0, 1.1), Color(0.22, 0.18, 0.15))
	door.position = Vector3(-4.02, 1.0, 0)
	house.add_child(door)
	# A porch lamp, so there is somewhere to walk back to.
	var lamp := _box(Vector3(0.3, 0.3, 0.3), Color(1.0, 0.78, 0.45), true)
	lamp.position = Vector3(-4.3, 2.4, 0)
	house.add_child(lamp)

	# Farm plots on the alluvial flat the erosion model deposited.
	for i in 4:
		var pz := z - 6.0 - float(i % 2) * 11.0
		var pt: float = th + (float(i >> 1) * 12.0 - 6.0) / P["radius"]
		var pr := ground_at(pt, pz)
		var plot := Node3D.new()
		plot.transform = frame_at(pt, pz, pr)
		add_child(plot)
		var bed := _box(Vector3(9.0, 0.35, 9.0), Color(0.21, 0.15, 0.10))
		bed.position = Vector3(0, 0.1, 0)
		plot.add_child(bed)
		for row in 7:
			var crop := _box(Vector3(8.2, 0.5, 0.35), Color(0.30, 0.46, 0.20))
			crop.position = Vector3(0, 0.45, -3.6 + row * 1.2)
			plot.add_child(crop)

	# Where you wake up.
	var mark := frame_at(th, z, gr)
	spawn["world"] = mark.origin

func _build_towns() -> void:
	# Two settlements: one across the valley, one on the FAR SIDE of the drum,
	# so that looking "up" shows you inhabited ground overhead. (A1/A6)
	var seeds := [
		{"dt": 0.55, "dz": 210.0, "n": 26, "scale": 1.0},
		{"dt": PI,   "dz": -40.0, "n": 42, "scale": 1.5},
		{"dt": 2.1,  "dz": -280.0, "n": 20, "scale": 1.2},
	]
	var rng := RandomNumberGenerator.new()
	rng.seed = 20260906
	for s in seeds:
		for i in s["n"]:
			var th: float = float(spawn["theta"]) + float(s["dt"]) + rng.randf_range(-0.10, 0.10)
			var z: float = float(spawn["z"]) + float(s["dz"]) + rng.randf_range(-120.0, 120.0)
			var gr := ground_at(th, z)
			var h: float = rng.randf_range(6.0, 20.0) * float(s["scale"])
			town_marks.append(Vector2(th, z))
			var b := Node3D.new()
			b.transform = frame_at(th, z, gr)
			add_child(b)
			var body := _box(Vector3(rng.randf_range(6, 14), h, rng.randf_range(6, 14)),
					Color(0.30, 0.30, 0.33))
			body.position = Vector3(0, h * 0.5, 0)
			b.add_child(body)
			var glow := _box(Vector3(0.9, 0.5, 0.9), Color(1.0, 0.80, 0.48), true)
			glow.position = Vector3(0, h + 0.4, 0)
			b.add_child(glow)

# --------------------------------------------------------------- streaming --

func _chunk_key(ti: int, zi: int) -> String:
	return "%d_%d" % [posmod(ti, n_around), zi]

func _queue_chunks() -> void:
	var th: float = player.theta if player else float(spawn["theta"])
	var z: float = player.z if player else float(spawn["z"])
	var idx: Vector2i = terrain.chunk_index(th, z)
	var ti0: int = idx.x
	var zi0: int = idx.y
	# Prefer chunks in front of the camera — Godot still frustum-culls the rest,
	# but we avoid spending meshing budget on dirt behind your head.
	var face_t := 0.0
	var face_z := 0.0
	if player and player.cam:
		var f: Vector3 = -player.cam.global_transform.basis.z
		var tang := Vector3(-sin(th), cos(th), 0.0)
		face_t = f.dot(tang)
		face_z = f.z
	# Scored insert, not append-then-sort. This runs three times a second and
	# `pending` used to be re-sorted whole through a GDScript lambda each time —
	# and the dedupe compared a String against the [ti, zi, score] entries, so
	# it never matched and the queue grew a fresh copy of the ring every pass.
	var added := false
	for dt in range(-CHUNK_RADIUS, CHUNK_RADIUS + 1):
		for dz in range(-CHUNK_RADIUS, CHUNK_RADIUS + 1):
			var k := _chunk_key(ti0 + dt, zi0 + dz)
			if loaded.has(k) or pending_set.has(k):
				continue
			# Lower score = sooner. Facing the look direction gets a bonus.
			var score: float = float(absi(dt) + absi(dz)) \
					- (float(dt) * face_t + float(dz) * face_z) * 0.55
			pending.append([posmod(ti0 + dt, n_around), zi0 + dz, score])
			pending_set[k] = true
			added = true
	if added:
		pending.sort_custom(_by_score)
	_unload_far(ti0, zi0)

static func _by_score(a: Array, b: Array) -> bool:
	return float(a[2]) < float(b[2])

## Milliseconds of a frame that chunk work may spend. Meshing and painting a
## chunk costs ~2-3 ms of the 16.7 ms a 60 Hz frame has; everything else in
## `_process` is microseconds. So this one number is the frame budget.
const CHUNK_MS_BUDGET := 6.0

## The only millisecond-scale work on a frame, under one shared budget.
##
## Remesh first: the bite you just took has to appear under the brush or the
## tool feels detached from the ground. Streaming second, but never starved —
## letting remesh eat the whole budget during a dig used to open holes in the
## ring behind you.
func _tick_streaming() -> void:
	# Look-friction work dropped the old per-20-frame queue from player.gd and
	# nothing replaced it — walk off the boot ring and mid dither punches a
	# void under your feet (near chunks gone, mid discarded inside ~105 m).
	var just_queued := false
	if player != null and P.has("radius"):
		var here := Vector2(player.theta * float(P["radius"]), player.z)
		if here.distance_to(stream_anchor) > CHUNK_SPAN * 0.45:
			stream_anchor = here
			_queue_chunks()
			just_queued = true
	var t0 := Time.get_ticks_usec()
	_drain_remesh(2 if dig_held else 1, t0)
	# After a ring rebuild, spend a little more budget so the hole closes
	# in one or two frames instead of a long dither trail.
	_pump_chunks(5 if just_queued else 2, t0)

func _over_budget(t0: int) -> bool:
	return float(Time.get_ticks_usec() - t0) * 0.001 > CHUNK_MS_BUDGET

## `t0` opts into the frame budget. The bulk fills at load and for screenshots
## pass nothing and run to completion.
func _pump_chunks(budget: int, t0: int = -1) -> void:
	var n := 0
	while pending.size() > 0 and n < budget:
		var e = pending.pop_front()
		var k := _chunk_key(e[0], e[1])
		pending_set.erase(k)
		if loaded.has(k):
			continue
		_build_chunk(e[0], e[1])
		n += 1
		if t0 >= 0 and _over_budget(t0):
			break

func _build_chunk(ti: int, zi: int) -> void:
	var k := _chunk_key(ti, zi)
	if zi < 0 or zi >= n_z_chunks:
		loaded[k] = null
		return
	var old = loaded.get(k)
	var d: Dictionary = terrain.chunk_mesh_at(ti, zi)
	var m := _mesh_from(d)
	if m != null:
		var mi := MeshInstance3D.new()
		mi.mesh = m
		mi.material_override = _terrain_material()
		mi.name = "Chunk" + k
		chunk_root.add_child(mi)
		loaded[k] = mi
		chunk_fail.erase(k)
	else:
		# Soft-fail empty meshes. Permanently caching null left rectangular holes
		# for the whole session when the radial band briefly missed relief.
		var fails := int(chunk_fail.get(k, 0)) + 1
		chunk_fail[k] = fails
		# Retry more for tall massifs — empty mesh often means radial band
		# undersampled once; permanent null punches a visible rectangle.
		if fails >= 6:
			loaded[k] = null
		else:
			pending.push_front([ti, zi, -20.0])
			pending_set[k] = true
	if old != null and is_instance_valid(old):
		old.queue_free()

## Rebuild every chunk the brush touched. Usually one; two at a boundary.
## Queued — sync remesh-per-bite was the main dig hitch.
func rebuild_around(p: Vector3, radius: float) -> void:
	var th := atan2(p.y, p.x)
	var dth: float = radius / float(P["radius"])
	var a: Vector2i = terrain.chunk_index(th - dth, p.z - radius)
	var b: Vector2i = terrain.chunk_index(th + dth, p.z + radius)
	for ti in range(mini(a.x, b.x), maxi(a.x, b.x) + 1):
		for zi in range(mini(a.y, b.y), maxi(a.y, b.y) + 1):
			var k := _chunk_key(ti, zi)
			if not loaded.has(k):
				continue
			if remesh_queue.has(k):
				continue
			remesh_queue.append(k)
	schedule_flow_refresh()

func remesh_backlog() -> int:
	return remesh_queue.size()

func _unload_far(ti0: int, zi0: int) -> void:
	var drop: Array = []
	for k in loaded.keys():
		var parts: PackedStringArray = k.split("_")
		var raw: int = int(parts[0]) - posmod(ti0, n_around)
		if raw > n_around >> 1: raw -= n_around
		if raw < -(n_around >> 1): raw += n_around
		var dt: int = absi(raw)
		var dz: int = absi(int(parts[1]) - zi0)
		if dt > UNLOAD_RADIUS or dz > UNLOAD_RADIUS:
			drop.append(k)
	for k in drop:
		var mi = loaded[k]
		if mi != null and is_instance_valid(mi):
			mi.queue_free()
		loaded.erase(k)

# --------------------------------------------------------------- building --

const MODULES := [
	{"name": "Farm bed", "size": Vector3(9, 0.4, 9), "col": Color(0.21, 0.15, 0.10), "rows": true},
	{"name": "Greenhouse", "size": Vector3(7, 3.2, 10), "col": Color(0.55, 0.72, 0.70), "rows": false},
	{"name": "Condenser", "size": Vector3(4, 5.5, 4), "col": Color(0.42, 0.45, 0.50), "rows": false},
	{"name": "Lamp post", "size": Vector3(0.5, 4.5, 0.5), "col": Color(0.35, 0.33, 0.30), "rows": false},
]

## Prefabs are placed ON terrain, not built FROM it. A colonist installs
## modules; they do not stack dirt. (PD_BRIEF.md — engineered habitat)
func place_module(p: Vector3, kind: int, from_save: bool = false) -> void:
	var spec: Dictionary = MODULES[kind]
	var th := atan2(p.y, p.x)
	var gr := ground_at(th, p.z)
	# Snap to a 2 m grid along the surface so installations line up.
	var arc_step: float = 2.0 / float(P["radius"])
	th = round(th / arc_step) * arc_step
	var z: float = round(p.z / 2.0) * 2.0
	gr = ground_at(th, z)
	var node := Node3D.new()
	node.transform = frame_at(th, z, gr)
	add_child(node)
	var sz: Vector3 = spec["size"]
	var body := _box(sz, spec["col"])
	body.position = Vector3(0, sz.y * 0.5, 0)
	node.add_child(body)
	if spec["rows"]:
		for row in 7:
			var crop := _box(Vector3(sz.x - 0.8, 0.5, 0.35), Color(0.30, 0.46, 0.20))
			crop.position = Vector3(0, sz.y + 0.25, -sz.z * 0.4 + row * (sz.z * 0.8 / 6.0))
			node.add_child(crop)
	if kind == 1:
		# Glass greenhouse — costs pack glass, registers local yield boost.
		if from_save:
			terrain.place_greenhouse_ex(th, z, false)
		else:
			var gh: Dictionary = terrain.place_greenhouse(th, z)
			if not gh.get("ok", false):
				print("[rama] greenhouse needs %.0f kg glass" % float(gh.get("need_kg", 12.0)))
				node.queue_free()
				return
		var gbadge := Label3D.new()
		gbadge.text = "GLASS"
		gbadge.font_size = 42
		gbadge.modulate = Color(0.65, 0.90, 0.85)
		gbadge.position = Vector3(0, sz.y + 0.6, 0)
		gbadge.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		node.add_child(gbadge)
	if kind == 2:
		if from_save:
			terrain.add_condenser(th, z, 1.2)
		elif not terrain.try_add_condenser(th, z, 1.2):
			print("[rama] condenser refused — reactor budget full")
			node.queue_free()
			return
		var badge := Label3D.new()
		badge.text = "COND"
		badge.font_size = 48
		badge.modulate = Color(0.7, 0.85, 1.0)
		badge.position = Vector3(0, sz.y + 0.8, 0)
		badge.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		node.add_child(badge)
	if kind == 3:
		var lamp := _box(Vector3(1.0, 0.6, 1.0), Color(1.0, 0.80, 0.45), true)
		lamp.position = Vector3(0, sz.y + 0.3, 0)
		node.add_child(lamp)
	node.set_meta("kind", kind)
	node.set_meta("theta", th)
	node.set_meta("z", z)
	modules.append(node)

func _build_plants() -> void:
	# Eight growth algorithms × near/mid LOD. Each mesh is a full recursive
	# branching tree (block vocabulary, natural forks). Excavate trunks for timber.
	plant_species_near = []
	plant_species_mid = []
	for kind in 8:
		var near_mesh: ArrayMesh = RamaTrees.mesh_for(kind, 1.0, kind * 104729 + 17)
		var mid_mesh: ArrayMesh = RamaTrees.mesh_for(kind, 0.78, kind * 104729 + 91)
		var near := _make_plant_layer("PlantsNear_%d" % kind, near_mesh, 120.0, 240.0)
		near.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		if near.material_override is ShaderMaterial:
			near.material_override.set_shader_parameter("model_height", RamaTrees.TREE_H)
			near.material_override.set_shader_parameter("sway", 0.045)
		plant_species_near.append(near)
		var mid := _make_plant_layer("PlantsMid_%d" % kind, mid_mesh, 200.0, 360.0)
		mid.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		if mid.material_override is ShaderMaterial:
			mid.material_override.set_shader_parameter("model_height", RamaTrees.TREE_H * 0.78)
			mid.material_override.set_shader_parameter("sway", 0.03)
		plant_species_mid.append(mid)
	plant_mm_far = _make_plant_layer("PlantsFar", RamaTrees.billboard_mesh(), 320.0, 520.0)
	plant_mm_far.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	if plant_mm_far.material_override is ShaderMaterial:
		plant_mm_far.material_override.set_shader_parameter("sway", 0.0)
		plant_mm_far.material_override.set_shader_parameter("model_height", 4.0)
	plant_mm = plant_species_near[0]
	plant_mm_mid = plant_species_mid[0]
	_refresh_plants()

## Minecraft-style connected wood/leaf cubes. Same grid → faces combine.
func _build_woodscape() -> void:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	# Unit cube centred on origin; instance transform places it.
	var e := 0.54
	_add_prism(st, Vector3.ZERO, Vector3(e, e, e), Color(1, 1, 1, 1))
	st.generate_normals()
	var mesh := st.commit()
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	# CUSTOM.x carries how far up its own crown a block sits. A lone instanced
	# cube has no local height for the shader to read, so without this every
	# block shades as ground contact — a uniformly dark, swayless forest.
	mm.use_custom_data = true
	mm.mesh = mesh
	mm.instance_count = 0
	woodscape_mm = MultiMeshInstance3D.new()
	woodscape_mm.multimesh = mm
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/tree.gdshader")
	mat.set_shader_parameter("haze_start", 80.0)
	mat.set_shader_parameter("haze_end", 420.0)
	mat.set_shader_parameter("fade_start", 70.0)
	mat.set_shader_parameter("fade_end", 130.0)
	mat.set_shader_parameter("sway", 0.02)
	# Crown height comes from instance data, not from the cube's own vertices.
	mat.set_shader_parameter("up_from_instance", true)
	woodscape_mm.material_override = mat
	woodscape_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	woodscape_mm.name = "Woodscape"
	add_child(woodscape_mm)
	woodscape_refresh_in = 0.05

## Voxel stands stream within this radius — matched to the sim's near plant
## tier (`plants_lod` r_near) so the blocks replace exactly the Multimesh trees
## that `_hide_near_trees` switches off, with no band left empty between them.
const WOODSCAPE_RADIUS := 92.0
const WOODSCAPE_LIMIT := 12000
## Bark and foliage keys for `tree.gdshader`. RGB is a luminance ratio the
## shader multiplies into `bark_tint` (alpha 0) or reads as pigment (alpha 1),
## so these track the near-tree instance colours; a plain white wood key came
## out as 3.5x bark and read as glowing orange cubes.
const WOOD_KEY := Color(0.30, 0.28, 0.26, 0.0)
const LEAF_KEY := Color(0.16, 0.44, 0.22, 1.0)

func refresh_woodscape(force := false) -> void:
	if woodscape_mm == null or player == null or terrain == null:
		return
	if not force and woodscape_refresh_in > 0.0:
		return
	woodscape_refresh_in = 0.22
	var feet: Vector3 = player.feet_pos() if player.has_method("feet_pos") else player.global_position
	var src: PackedFloat32Array = terrain.woodscape_lod(
			feet.x, feet.y, feet.z, WOODSCAPE_RADIUS, WOODSCAPE_LIMIT)
	var n: int = int(src.size() / 5.0)
	var mm: MultiMesh = woodscape_mm.multimesh
	if n < 1:
		mm.instance_count = 0
		_hide_near_trees(0)
		return
	mm.instance_count = n
	for i in n:
		var s: int = i * 5
		mm.set_instance_transform(i, Transform3D(Basis.IDENTITY,
				Vector3(src[s], src[s + 1], src[s + 2])))
		mm.set_instance_color(i, WOOD_KEY if int(src[s + 3]) == 1 else LEAF_KEY)
		# How far up its own crown this block sits — see `up_from_instance`.
		mm.set_instance_custom_data(i, Color(src[s + 4], 0.0, 0.0, 0.0))
	_hide_near_trees(n)

## One vocabulary at close range: where voxel stands are resident, the near
## Multimesh trees stand down. Hysteresis on the count, because the threshold
## sits right where a stand streams in and a bare comparison flickered the
## whole near tier on and off as you walked.
func _hide_near_trees(blocks: int) -> void:
	if blocks > 240:
		woodscape_owns_near = true
	elif blocks < 60:
		woodscape_owns_near = false
	for mi in plant_species_near:
		if mi:
			mi.visible = not woodscape_owns_near

## Form id from plants_lod (0 conifer … 7 giant). Biome fallback if kind absent.
func _biome_species(bid: int, kind: int = -1) -> int:
	if kind >= 0:
		return clampi(kind, 0, 7)
	match bid:
		5: return 0
		1, 2, 9: return 2
		4, 6, 7, 11, 12: return 3
		13: return 4
		10: return 1
		8: return 5
		_: return 1

## Tree meshes live in `trees.gd` (RamaTrees) — recursive blocky branching.
## Vertex colour convention for tree.gdshader is documented there.

func _add_prism(st: SurfaceTool, center: Vector3, size: Vector3, col: Color) -> void:
	var hx := size.x * 0.5
	var hy := size.y * 0.5
	var hz := size.z * 0.5
	var c := center
	var corners := [
		c + Vector3(-hx, -hy, -hz), c + Vector3(hx, -hy, -hz),
		c + Vector3(hx, hy, -hz), c + Vector3(-hx, hy, -hz),
		c + Vector3(-hx, -hy, hz), c + Vector3(hx, -hy, hz),
		c + Vector3(hx, hy, hz), c + Vector3(-hx, hy, hz),
	]
	var faces := [
		[0, 1, 2, 0, 2, 3], [5, 4, 7, 5, 7, 6],
		[4, 0, 3, 4, 3, 7], [1, 5, 6, 1, 6, 2],
		[3, 2, 6, 3, 6, 7], [4, 5, 1, 4, 1, 0],
	]
	for f in faces:
		for idx in f:
			st.set_color(col)
			st.add_vertex(corners[idx])

func _make_plant_layer(layer_name: String, mesh: Mesh, fade_min: float, fade_max: float) -> MultiMeshInstance3D:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = mesh
	mm.instance_count = 1
	mm.set_instance_transform(0, Transform3D(Basis().scaled(Vector3.ZERO), Vector3.ZERO))
	mm.set_instance_color(0, Color(1, 1, 1, 0))
	var mi := MultiMeshInstance3D.new()
	mi.multimesh = mm
	# Same flat-white-albedo trap the craft stations fell into: unshaded with
	# vertex colours and a default albedo renders vegetation as cardboard.
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/tree.gdshader")
	mat.set_shader_parameter("haze_start", fade_min * 0.6)
	mat.set_shader_parameter("haze_end", fade_max * 3.0)
	mat.set_shader_parameter("fade_start", fade_min)
	mat.set_shader_parameter("fade_end", fade_max)
	mat.set_shader_parameter("model_height", 2.15)
	mat.set_shader_parameter("sway", 0.10)
	mi.material_override = mat
	mi.name = layer_name
	add_child(mi)
	return mi

const PLANT_STRIDE := 8  # theta,z,stem,leaf,alive,lod,biome_id,genome_id
const BIOME_PLANT_COL := [
	Color(0.18, 0.38, 0.36), # water edge
	Color(0.14, 0.42, 0.28), # wetland
	Color(0.12, 0.48, 0.22), # riparian
	Color(0.22, 0.46, 0.16), # grassland
	Color(0.38, 0.40, 0.14), # scrub
	Color(0.08, 0.32, 0.14), # forest
	Color(0.28, 0.36, 0.28), # alpine
	Color(0.42, 0.40, 0.34), # bare rock sparse
	Color(0.30, 0.48, 0.18), # farm
	Color(0.12, 0.36, 0.24), # swamp
	Color(0.24, 0.50, 0.16), # meadow
	Color(0.52, 0.44, 0.22), # desert
	Color(0.62, 0.52, 0.28), # dune
	Color(0.58, 0.54, 0.40), # shore
]

func _refresh_plants() -> void:
	if plant_species_near.is_empty() or player == null:
		return
	last_plants_alive = int(last_sim.get("plants", -1))
	plant_anchor = Vector2(player.theta, player.z)
	plant_data = terrain.plants_lod(player.theta, player.z, plant_lod_radius)
	var stride: int = PLANT_STRIDE if plant_data.size() % PLANT_STRIDE == 0 else 7
	# Buckets: [lod][species] → index list
	var buckets: Array = []
	for _lod in 3:
		var sp: Array = [[], [], [], [], [], [], [], []]
		buckets.append(sp)
	var n: int = int(plant_data.size() / float(stride))
	for i in n:
		var base: int = i * stride
		var lod: int = clampi(int(plant_data[base + 5]), 0, 2)
		var bid: int = clampi(int(plant_data[base + 6]), 0, BIOME_PLANT_COL.size() - 1)
		var genome: int = int(plant_data[base + 7]) if stride >= 8 else -1
		var sp: int = _biome_species(bid, genome)
		buckets[lod][sp].append(i)
	for sp in 8:
		_fill_plant_bucket(plant_species_near[sp], plant_data, buckets[0][sp], 1.0, stride)
		_fill_plant_bucket(plant_species_mid[sp], plant_data, buckets[1][sp], 1.25, stride)
	# Far: merge all species into one billboard layer.
	var far_idx: Array = []
	for sp in 8:
		far_idx.append_array(buckets[2][sp])
	_fill_plant_bucket(plant_mm_far, plant_data, far_idx, 1.7, stride)

func _plant_instance_xform(data: PackedFloat32Array, i: int, scale_boost: float, stride: int = PLANT_STRIDE) -> Transform3D:
	var base: int = i * stride
	var th: float = data[base]
	var zz: float = data[base + 1]
	var stem: float = data[base + 2]
	var leaf: float = data[base + 3]
	var bid: int = clampi(int(data[base + 6]), 0, BIOME_PLANT_COL.size() - 1)
	var kind: int = int(data[base + 7]) if stride >= 8 else -1
	var sp: int = _biome_species(bid, kind)
	var gr: float = terrain.ground_radius(th, zz)
	# Target height metres — saplings ~6 m, canopy giants ~35 m+.
	var h: float = clampf(6.0 + stem * 38.0 + leaf * 10.0, 5.5, 36.0) * scale_boost
	if sp == 3 or sp == 4:
		h = clampf(0.9 + stem * 3.0 + leaf * 1.5, 0.7, 3.8) * scale_boost
	elif sp == 5:
		h = clampf(3.5 + stem * 14.0 + leaf * 4.0, 3.0, 12.0) * scale_boost
	elif sp == 6:
		h = clampf(7.0 + stem * 28.0 + leaf * 8.0, 6.0, 28.0) * scale_boost
	elif sp == 7:
		h = clampf(12.0 + stem * 42.0 + leaf * 12.0, 10.0, 42.0) * scale_boost
	var xf := frame_at(th, zz, gr)
	var jit: float = fposmod(sin(th * 733.1 + zz * 41.7) * 43758.5453, 1.0)
	var jit2: float = fposmod(sin(th * 191.3 - zz * 97.1) * 24634.6345, 1.0)
	xf.basis = xf.basis.rotated(xf.basis.y, jit * TAU)
	# Mild spinward lean — taller trees tip toward +theta (drum rotation).
	var spin_lean: float = (0.035 + stem * 0.04) * (0.7 + jit * 0.6)
	if sp == 6:
		spin_lean *= 1.8
	xf.basis = xf.basis.rotated(xf.basis.z, spin_lean)
	xf.basis = xf.basis.rotated(xf.basis.x, (jit - 0.5) * 0.06)
	var w: float = (1.0 + leaf * 0.55 + stem * 0.45) * scale_boost * (0.88 + jit2 * 0.28)
	if sp == 3:
		w *= 1.2
	elif sp == 4:
		w *= 0.5
	elif sp == 6:
		w *= 1.4
	elif sp == 7:
		w *= 1.55
	var mesh_h: float = RamaTrees.TREE_H
	if sp == 3 or sp == 4:
		mesh_h = 3.0
	elif sp == 5:
		mesh_h = RamaTrees.TREE_H * 0.55
	var s: float = h / mesh_h
	xf.basis = xf.basis.scaled(Vector3(w * s, s, w * s))
	return xf

func _plant_instance_color(data: PackedFloat32Array, i: int, stride: int = PLANT_STRIDE) -> Color:
	var base: int = i * stride
	var stem: float = data[base + 2]
	var leaf: float = data[base + 3]
	var bid: int = clampi(int(data[base + 6]), 0, BIOME_PLANT_COL.size() - 1)
	var th: float = data[base]
	var zz: float = data[base + 1]
	var jit: float = fposmod(sin(th * 733.1 + zz * 41.7) * 43758.5453, 1.0)
	var jit2: float = fposmod(sin(th * 191.3 - zz * 97.1) * 24634.6345, 1.0)
	var base_col: Color = BIOME_PLANT_COL[bid]
	var tint := Color(
		base_col.r + leaf * 0.10 - stem * 0.02 + (jit - 0.5) * 0.08,
		base_col.g + leaf * 0.14 + (jit2 - 0.5) * 0.06,
		base_col.b + stem * 0.03 + (jit - 0.5) * 0.04)
	if jit > 0.82:
		tint = tint.lerp(Color(0.36, 0.34, 0.12), 0.35)
	elif jit < 0.12:
		tint = tint.lerp(Color(0.08, 0.28, 0.18), 0.30)
	return tint

func _fill_plant_bucket(mi: MultiMeshInstance3D, data: PackedFloat32Array, indices: Array, scale_boost: float, stride: int = PLANT_STRIDE) -> void:
	if mi == null:
		return
	var n: int = indices.size()
	if n < 1:
		mi.multimesh.instance_count = 0
		return
	mi.multimesh.instance_count = n
	for j in n:
		var i: int = indices[j]
		mi.multimesh.set_instance_transform(j, _plant_instance_xform(data, i, scale_boost, stride))
		mi.multimesh.set_instance_color(j, _plant_instance_color(data, i, stride))

# ------------------------------------------------------------------ player --

func _build_player() -> void:
	player = load("res://scripts/player.gd").new()
	player.world = self
	# Far plane must clear the whole drum. 5200 m clipped the last kilometre of
	# a 6264 m diagonal, which is the hard curved edge where the land stopped.
	player.cam_far = drum_diagonal() * 1.15
	player.theta = spawn["theta"]
	player.z = spawn["z"]
	player.r = float(spawn["radius"]) - 0.2
	add_child(player)

func _build_overlays() -> void:
	var post := CanvasLayer.new()
	post_layer = post
	post.layer = 1
	var rect := ColorRect.new()
	rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var sm := ShaderMaterial.new()
	sm.shader = load("res://shaders/tiltshift.gdshader")
	rect.material = sm
	post.add_child(rect)
	add_child(post)

	reticle = Control.new()
	reticle.set_script(load("res://scripts/reticle.gd"))
	reticle.world = self
	reticle.set_anchors_preset(Control.PRESET_FULL_RECT)
	reticle.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var rl := CanvasLayer.new()
	rl.layer = 3
	rl.add_child(reticle)
	add_child(rl)

	var hud_layer := CanvasLayer.new()
	hud_layer.layer = 2
	hud = Label.new()
	hud.position = Vector2(22, 16)
	hud.add_theme_font_size_override("font_size", 13)
	hud.add_theme_color_override("font_color", Color(0.86, 0.92, 0.95))
	hud.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.85))
	hud.add_theme_constant_override("outline_size", 5)
	hud_layer.add_child(hud)
	add_child(hud_layer)

func _process(_dt: float) -> void:
	_tick_streaming()
	_tick_daylight(_dt)
	_tick_flow_refresh(_dt)
	_tick_pool_refresh(_dt)
	_tick_stockpile_refresh(_dt)
	_tick_wet_refresh(_dt)
	_drain_wet_colors(1)
	_tick_biosphere(_dt)
	_tick_deferred_visuals()
	_tick_plant_fill()
	woodscape_refresh_in = maxf(woodscape_refresh_in - _dt, 0.0)
	if woodscape_refresh_in <= 0.0:
		refresh_woodscape()
	_tick_catchment(_dt)
	_tick_splash(_dt)
	_tick_catchment_pulse(_dt)
	_tick_autosave(_dt)
	_tick_water_audio(_dt)
	_tick_panels()
	_tick_life_layers(_dt)
	_tick_agent_rigs(_dt)
	refresh_grass()
	refresh_mid()
	if hud and player:
		if ui == null:
			_build_hud()
		hud.visible = false
		_tick_hud(_dt)

func _tick_hud(dt: float) -> void:
	# Clock / toast every frame; soil/weather/inventory FFI at ~8 Hz.
	_hud_accum += dt
	var heavy := _hud_accum >= 0.12 or Engine.get_process_frames() < 4
	if heavy:
		_hud_accum = 0.0
		_hud_e = terrain.elevation(player.theta, player.z)
		_hud_fx = terrain.water_flux(player.theta, player.z)
		_water_dep = terrain.water_depth_at(player.theta, player.z)
		_water_fx = _hud_fx
	_push_hud(_hud_e, _hud_fx, heavy)

func _tick_life_layers(dt: float) -> void:
	# Never stack with a sim frame — that was the audible "every few seconds" dip.
	if sim_frame_cooldown > 0:
		return
	# Spread grazer / rain / litter across frames so they never stack with sim_tick.
	life_refresh_accum += dt
	# Storms want fresher curtains; clear weather can idle longer.
	var period: float = 0.48 if sky_event == 2 else 0.85
	if life_refresh_accum < period:
		return
	life_refresh_accum = 0.0
	match life_phase % 3:
		0:
			_refresh_grazers()
		1:
			_refresh_rain()
		_:
			_refresh_litter()
	life_phase += 1

func _tick_water_audio(dt: float = 0.016) -> void:
	if audio == null or player == null or terrain == null:
		return
	_audio_accum += dt
	if _audio_accum >= 0.14:
		_audio_accum = 0.0
		_water_dep = terrain.water_depth_at(player.theta, player.z)
		_water_fx = terrain.water_flux(player.theta, player.z)
		# Biome reverb — rare; biome_at + ground_at every frame was hitch food.
		var here := Vector2(player.theta, player.z)
		if here.distance_squared_to(water_bio_cache_at) > 0.0004:
			water_bio_cache_at = here
			var bio: Dictionary = terrain.biome_at(player.theta, player.z)
			water_bio_name = str(bio.get("name", ""))
		var ground: float = ground_at(player.theta, player.z)
		var underground: bool = player.r < ground - 2.0
		if audio.has_method("set_biome_reverb"):
			audio.set_biome_reverb(water_bio_name, underground)
	# Cylinder wrap: angular distance around the drum attenuates like a corridor (§2144).
	var wrap_factor := 1.0
	if P.has("radius"):
		var R: float = float(P["radius"])
		var around := fposmod(absf(player.theta), TAU)
		around = minf(around, TAU - around)
		wrap_factor = clampf(1.0 - (around * R) / (R * PI * 0.55), 0.2, 1.0)
	audio.water_ambience(_water_dep, _water_fx, wrap_factor)

func _tick_catchment_pulse(dt: float) -> void:
	# Pulse material only — remeshing the ribbon every few frames was a hitch.
	if catchment_mat == null:
		return
	catchment_phase += dt
	var pulse: float = 0.55 + 0.45 * sin(catchment_phase * 2.4)
	catchment_mat.albedo_color = Color(1.0, 1.0, 1.0, 0.22 + pulse * 0.28)

func _tick_autosave(dt: float) -> void:
	autosave_accum += dt
	if autosave_accum < AUTOSAVE_SECS:
		return
	autosave_accum = 0.0
	_autosave_slot()

func _autosave_slot() -> void:
	# Rotate three versioned slots so a mid-session crash doesn't eat the last F5.
	var slot: int = int(Time.get_unix_time_from_system()) % 3
	var strokes: PackedByteArray = terrain.save_strokes()
	var f := FileAccess.open("user://rama_auto_%d_strokes.bin" % slot, FileAccess.WRITE)
	if f:
		f.store_buffer(strokes)
		f.close()
	var soil: PackedByteArray = terrain.save_soil()
	var fs := FileAccess.open("user://rama_auto_%d_soil.bin" % slot, FileAccess.WRITE)
	if fs:
		fs.store_buffer(soil)
		fs.close()
	var dwell: PackedByteArray = terrain.save_dwellings()
	var fd := FileAccess.open("user://rama_auto_%d_dwell.bin" % slot, FileAccess.WRITE)
	if fd:
		fd.store_buffer(dwell)
		fd.close()
	print("[rama] autosave slot %d" % slot)

func _tick_biosphere(dt: float) -> void:
	sim_accum += dt * SIM_STEP_DAYS
	# Smaller steps more often → soft ticks instead of one fat hitch every ~6s.
	if sim_accum < 0.045:
		return
	var step: float = minf(sim_accum, 0.032)
	sim_accum -= step
	if player != null:
		terrain.set_player_pos(player.theta, player.z)
	last_sim = terrain.sim_tick(step)
	# Hold heavy visuals / life layers off this frame and the next.
	sim_frame_cooldown = 2
	agent_refresh_accum = 0.0
	visuals_pending = true
	visual_phase = 0
	var pools: int = int(last_sim.get("pools", 0))
	var pdepth: float = float(last_sim.get("pool_depth", 0.0))
	if pools != last_pool_cells or absf(pdepth - last_pool_depth) > 0.08:
		last_pool_cells = pools
		last_pool_depth = pdepth
		schedule_pool_refresh()
	if float(last_sim.get("sediment", 0.0)) > 2.0:
		schedule_flow_refresh()

func _tick_deferred_visuals() -> void:
	if sim_frame_cooldown > 0:
		sim_frame_cooldown -= 1
		return
	if not visuals_pending:
		agent_refresh_accum += get_process_delta_time()
		if agent_refresh_accum > 0.5:
			agent_refresh_accum = 0.0
			refresh_agents()
			refresh_dwellings()
			refresh_stockpiles()
			refresh_carcasses()
		return
	match visual_phase:
		0:
			_maybe_queue_plants()
		1:
			refresh_agents()
		2:
			refresh_dwellings()
		3:
			if float(last_sim.get("sediment", 0.0)) > 2.0:
				_refresh_foam()
				_refresh_biome_map()
				_queue_remesh_near_player()
			else:
				wet_refresh_in = 3.2
		4:
			refresh_stockpiles()
			refresh_carcasses()
			_refresh_soil_overlay()
			visuals_pending = false
			return
	visual_phase += 1

func _maybe_queue_plants() -> void:
	if player == null or plant_species_near.is_empty():
		return
	if plant_refresh_due:
		return
	var alive: int = int(last_sim.get("plants", last_plants_alive))
	var anchor := Vector2(player.theta, player.z)
	var moved: float = absf(wrapf(anchor.x - plant_anchor.x, -PI, PI)) * float(P["radius"])
	moved += absf(anchor.y - plant_anchor.y)
	if alive == last_plants_alive and moved < 12.0 and plant_data.size() > 0:
		return
	last_plants_alive = alive
	plant_anchor = anchor
	plant_bucket_idx = 0
	plant_fill_j = 0
	plant_indices = []
	plant_refresh_due = true

## Immediate Multimesh rebuild after felling — don't wait for the next sim census.
func force_plant_refresh() -> void:
	last_plants_alive = -1
	plant_refresh_due = false
	_refresh_plants()
	if terrain != null:
		last_plants_alive = int(terrain.plant_count())
		last_sim["plants"] = last_plants_alive

func _tick_plant_fill() -> void:
	if not plant_refresh_due:
		return
	if remesh_queue.size() > 0:
		return
	if player == null or plant_species_near.is_empty():
		plant_refresh_due = false
		return
	# Full refresh is cheaper than streaming 9 Multimeshes with tiny budgets.
	_refresh_plants()
	plant_refresh_due = false
	plant_indices = []

func _begin_plant_bucket(_b: int) -> void:
	plant_refresh_due = false

func _fill_plant_budget() -> void:
	plant_refresh_due = false

func _tick_wet_refresh(dt: float) -> void:
	if wet_refresh_in < 0.0:
		return
	wet_refresh_in -= dt
	if wet_refresh_in > 0.0:
		return
	wet_refresh_in = -1.0
	_refresh_wet_colors(Engine.get_frames_drawn() % 480 < 6)

func _remesh_near_player() -> void:
	if player == null:
		return
	# Prefer colour patch when only wetness moved; full remesh on sediment.
	if float(last_sim.get("sediment", 0.0)) > 2.0:
		rebuild_around(to_world(player.theta, player.z, player.r), 48.0)
	else:
		_refresh_wet_colors()

func save_world() -> void:
	var strokes: PackedByteArray = terrain.save_strokes()
	var f := FileAccess.open(SAVE_PATH, FileAccess.WRITE)
	if f == null:
		print("[rama] save failed: %s" % SAVE_PATH)
		return
	f.store_buffer(strokes)
	f.close()
	var soil: PackedByteArray = terrain.save_soil()
	var fs := FileAccess.open(SAVE_SOIL, FileAccess.WRITE)
	if fs:
		fs.store_buffer(soil)
		fs.close()
	var dwell: PackedByteArray = terrain.save_dwellings()
	var fd := FileAccess.open(SAVE_DWELL, FileAccess.WRITE)
	if fd:
		fd.store_buffer(dwell)
		fd.close()
	var props := {
		"modules": [],
		"waypoint": [waypoint.x, waypoint.y],
		"has_waypoint": has_waypoint,
		"home": [home.x, home.y],
		"clock": clock,
		"saved_unix": Time.get_unix_time_from_system(),
	}
	if player:
		props["player"] = {
			"theta": player.theta,
			"z": player.z,
			"yaw": player.yaw,
			"pitch": player.pitch,
			"eye": player.eye,
			"view": player.view,
		}
	for m in modules:
		if m == null or not is_instance_valid(m):
			continue
		if not m.has_meta("kind"):
			continue
		props["modules"].append({
			"kind": int(m.get_meta("kind")),
			"theta": float(m.get_meta("theta")),
			"z": float(m.get_meta("z")),
		})
	var fw := FileAccess.open(SAVE_WORLD, FileAccess.WRITE)
	if fw:
		fw.store_string(JSON.stringify(props))
		fw.close()
	print("[rama] saved digs + soil + dwellings + %d modules → user://" % props["modules"].size())

func load_world() -> void:
	if not FileAccess.file_exists(SAVE_PATH):
		print("[rama] no save at %s" % SAVE_PATH)
		return
	var f := FileAccess.open(SAVE_PATH, FileAccess.READ)
	if f == null:
		return
	var strokes: PackedByteArray = f.get_buffer(f.get_length())
	f.close()
	if not terrain.load_strokes(strokes):
		print("[rama] load_strokes failed")
		return
	if FileAccess.file_exists(SAVE_SOIL):
		var fs := FileAccess.open(SAVE_SOIL, FileAccess.READ)
		if fs:
			terrain.load_soil(fs.get_buffer(fs.get_length()))
			fs.close()
	if FileAccess.file_exists(SAVE_DWELL):
		var fd := FileAccess.open(SAVE_DWELL, FileAccess.READ)
		if fd:
			if not terrain.load_dwellings(fd.get_buffer(fd.get_length())):
				print("[rama] load_dwellings failed")
			fd.close()
	if FileAccess.file_exists(SAVE_WORLD):
		var fw := FileAccess.open(SAVE_WORLD, FileAccess.READ)
		if fw:
			var parsed = JSON.parse_string(fw.get_as_text())
			fw.close()
			if typeof(parsed) == TYPE_DICTIONARY:
				_load_world_props(parsed)
	_build_rivers()
	_build_pools()
	_refresh_foam()
	_refresh_biome_map()
	_refresh_soil_overlay()
	_remesh_near_player()
	refresh_dwellings()
	_refresh_litter()
	_refresh_rain()
	_refresh_grazers()
	census = terrain.biome_census(260)
	print("[rama] loaded digs (%d strokes)" % terrain.edit_count())

func _load_world_props(props: Dictionary) -> void:
	for m in modules:
		if m and is_instance_valid(m):
			m.queue_free()
	modules.clear()
	has_waypoint = bool(props.get("has_waypoint", false))
	var wp: Array = props.get("waypoint", [0.0, 0.0])
	if wp.size() >= 2:
		waypoint = Vector2(float(wp[0]), float(wp[1]))
	for spec in props.get("modules", []):
		var kind: int = int(spec.get("kind", 0))
		var th: float = float(spec.get("theta", 0.0))
		var zz: float = float(spec.get("z", 0.0))
		var p := to_world(th, zz, ground_at(th, zz))
		place_module(p, kind, true)
	# Resume camera exactly (§2180).
	var pl: Variant = props.get("player", null)
	if typeof(pl) == TYPE_DICTIONARY and player:
		player.theta = float(pl.get("theta", player.theta))
		player.z = float(pl.get("z", player.z))
		player.yaw = float(pl.get("yaw", player.yaw))
		player.pitch = float(pl.get("pitch", player.pitch))
		player.eye = float(pl.get("eye", player.eye))
		player.view = int(pl.get("view", 0))
		player.r = ground_at(player.theta, player.z)
		player.cam_ready = false
		apply_view(player.view)
		player._update_camera()
	if props.has("clock"):
		var prev: float = float(props["clock"])
		var dt_days: float = clock - prev
		if absf(dt_days) > 0.05:
			away_blurb = "while you were away: %.1f habitat-days passed" % absf(dt_days)
		else:
			var saved_unix: float = float(props.get("saved_unix", 0.0))
			if saved_unix > 0.0:
				var hrs: float = (Time.get_unix_time_from_system() - saved_unix) / 3600.0
				if hrs > 0.25:
					away_blurb = "welcome back — %.1f hours real-time since last save" % hrs
	if props.has("home"):
		var hm: Array = props["home"]
		if hm.size() >= 2:
			home = Vector2(float(hm[0]), float(hm[1]))

func refresh_catchment_at(th: float, zz: float) -> void:
	# Only mark dirty — rebuilding the ribbon from _draw while panning was the
	# hitch that felt like the look stick freezing every few degrees.
	var key := Vector2(snappedf(th, 0.006), snappedf(zz, 10.0))
	if key.distance_squared_to(catchment_aim) < 0.00001 and not catchment_dirty:
		return
	catchment_pending_th = th
	catchment_pending_z = zz
	catchment_aim = key
	catchment_dirty = true

func _tick_catchment(dt: float) -> void:
	if catchment_cooldown > 0.0:
		catchment_cooldown -= dt
		return
	if not catchment_dirty:
		return
	catchment_dirty = false
	catchment_cooldown = CATCHMENT_COOLDOWN
	catchment_cache = terrain.catchment_points(catchment_pending_th, catchment_pending_z, 96)
	refresh_catchment_ribbon()

func _queue_remesh_near_player() -> void:
	if player == null:
		return
	var idx: Vector2i = terrain.chunk_index(player.theta, player.z)
	for dt in range(-1, 2):
		for dz in range(-1, 2):
			var ti: int = posmod(idx.x + dt, n_around)
			var zi: int = idx.y + dz
			var k := _chunk_key(ti, zi)
			if not loaded.has(k):
				continue
			if remesh_queue.has(k):
				continue
			remesh_queue.append(k)

func _drain_remesh(budget: int = 1, t0: int = -1) -> void:
	var n := 0
	while remesh_queue.size() > 0 and n < budget:
		var k: String = remesh_queue.pop_front()
		var parts: PackedStringArray = k.split("_")
		if parts.size() < 2:
			continue
		_build_chunk(int(parts[0]), int(parts[1]))
		n += 1
		if t0 >= 0 and _over_budget(t0):
			break

func _clock_str() -> String:
	var phase: float = fposmod(clock / DAY_LENGTH, 1.0)
	var mins: int = int(phase * 1440.0)
	var t := "%02d:%02d" % [int(mins / 60.0), mins % 60]
	if day_speed > 1.05:
		t += " ×%.0f" % day_speed
	return t

## Panels are declared once, as data. Adding a readout is one add_row call.
func _build_hud() -> void:
	ui = load("res://scripts/ui/hud.gd").new()
	add_child(ui)
	if audio and audio.has_method("_caption"):
		audio.caption_cb = func(t: String):
			note(t)
	if ui.has_method("apply_font_scale"):
		ui.apply_font_scale(RamaControls.font_scale)

	var hab = ui.panel("habitat", "KEPLER DRUM")
	hab.add_row("clock", "time")
	hab.add_row("light", "daylight", true)
	hab.add_row("grav", "gravity")

	var you = ui.panel("you", "COLONIST")
	you.add_row("pos", "position")
	you.add_row("elev", "elevation")
	you.add_row("pack", "pack", true)
	you.add_row("carry", "carrying")
	you.add_row("enc", "encumbrance", true)
	you.add_row("home", "home")

	var grd = ui.panel("ground", "GROUND")
	grd.add_row("biome", "biome")
	grd.add_row("npk", "N-P-K")
	grd.add_row("moist", "moisture", true)
	grd.add_row("ph", "pH")
	grd.add_row("drain", "drainage", true)

	var wx = ui.panel("weather", "WEATHER")
	wx.add_row("sky", "conditions")
	wx.add_row("temp", "temperature")
	wx.add_row("humid", "humidity", true)

	var srv = ui.panel("survey", "HABITAT SURVEY")
	srv.add_row("area", "surface")
	srv.add_row("arable", "arable", true)
	srv.add_row("relief", "relief")
	srv.add_row("water", "open water", true)
	srv.add_row("alluvial", "alluvial flat", true)
	srv.add_row("grass", "grassland", true)
	srv.add_row("upland", "upland", true)
	srv.add_row("rock", "bare rock", true)

	var pln = ui.panel("plan", "LOCAL PLAN")
	pln.add_row("scale", "scale")
	pln.add_row("along", "along drum")
	pln.add_row("around", "around drum")
	pln.add_row("mods", "installed")

	var col = ui.panel("colony", "COLONY")
	col.add_row("power", "power", true)
	col.add_row("air", "air")
	col.add_row("life", "life")
	col.add_row("people", "colonists")

	# Whose place you are standing in. Without this the whole legibility
	# ladder — who is reachable and who is not — is invisible.
	var plc = ui.panel("place", "NEAREST PLACE")
	plc.add_row("who", "who")
	plc.add_row("reach", "reach")
	plc.add_row("people", "people", true)
	plc.add_row("food", "keeping", true)
	plc.add_row("ground", "ground")
	plc.add_row("dist", "distance")

var _place_near := false
var _place_id := -1

## Distance, in metres along the ground, to a point on the drum.
func _arc_dist(th_a: float, z_a: float, th_b: float, z_b: float) -> float:
	var dth: float = wrapf(th_a - th_b, -PI, PI) * float(P["radius"])
	var dz: float = z_a - z_b
	return sqrt(dth * dth + dz * dz)

## Whose ground you are on. Shown within a claim and a bit beyond, so walking
## into a valley tells you who lives in it before you meet him.
const PLACE_SHOW_M := 220.0

func _push_place() -> void:
	var buf: PackedFloat32Array = terrain.dwellings_lod()
	var best := -1
	var best_d := 1e9
	for i in int(buf.size() / 9.0):
		var d: float = _arc_dist(player.theta, player.z, buf[i * 9], buf[i * 9 + 1])
		if d < best_d:
			best_d = d
			best = i
	_place_near = best >= 0 and best_d < PLACE_SHOW_M
	if not _place_near:
		_place_id = -1
		return
	var aid: int = int(buf[best * 9 + 8])
	var info: Dictionary = terrain.dwelling_info(aid)
	if not info.get("ok", false):
		_place_near = false
		return
	_place_id = aid
	ui.put("place", "who", "%s  ·  %s" % [info["name"], info["kind"]])
	var reach: String = str(info.get("reach", "legible"))
	if reach == "contested" and info.has("rival"):
		reach = "contested with %s" % info["rival"]
	ui.put("place", "reach", reach)
	var cap: float = float(info.get("capacity", 0.0))
	var fol: float = float(info.get("followers", 0.0))
	var target: float = maxf(cap - 1.0, 0.0)
	if cap < 1.0:
		ui.put("place", "people", "cannot feed one man", 1.0)
	else:
		ui.put("place", "people", "%d of %d the ground feeds" % [
			int(round(fol)) + 1, int(round(cap))],
			fol / maxf(target, 0.001))
	# Gauge shows SCARCITY, so a full red bar means the same here as it does
	# on every other gauge: at the limit.
	var days: float = float(info.get("days_of_food", 0.0))
	ui.put("place", "food", "%d days of food" % int(days),
		1.0 - clampf(days / 60.0, 0.0, 1.0))
	var q: float = float(info.get("quality", 0.0))
	var qword: String = "poor" if q < 0.5 else ("workable" if q < 0.9 else "good")
	ui.put("place", "ground", "%s  ·  %d works" % [qword, int(info.get("works", 0))])
	ui.put("place", "dist", "%d m" % int(_arc_dist(
		player.theta, player.z, float(info["theta"]), float(info["z"]))))

func _push_hud(e: float, fx: float, heavy: bool = true) -> void:
	var v: int = player.view
	if RamaControls.photo_mode or RamaControls.hud_density == "off":
		ui.visible = false
		return
	ui.visible = true
	# Instruments show habitat-scale readouts; the colonist view shows local.
	ui.show_panel("ground", v == 0)
	ui.show_panel("weather", v == 0)
	ui.show_panel("you", v != 1)
	ui.show_panel("survey", v == 1)
	ui.show_panel("plan", v == 2)
	ui.show_panel("habitat", true)
	ui.show_panel("colony", v == 0)
	ui.show_panel("place", v != 2 and _place_near)
	if ui.has_method("set_density"):
		ui.set_density(RamaControls.hud_density)

	# Always-cheap: clock, light, position — these change every frame in fast-day.
	ui.put("habitat", "clock", _clock_str())
	ui.put("habitat", "light", "%d%%" % int(day * 100.0), day)
	var arc_m: float = fposmod(player.theta * float(P["radius"]), float(P["circumference"]))
	ui.put("you", "pos", "%.0f m around  ·  z %+.0f m" % [arc_m, player.z])
	ui.put("you", "elev", "%.1f m above hull" % e)
	toast_t = maxf(toast_t - get_process_delta_time(), 0.0)
	ui.toast(toast, clampf(toast_t, 0.0, 1.0))
	if not heavy:
		return

	if v == 1:
		var ar: float = float(census.get("arable_km2", 0.0)) / maxf(float(census.get("surface_area_km2", 1.0)), 0.001)
		ui.put("survey", "area", "%.2f km²" % census["surface_area_km2"])
		ui.put("survey", "arable", "%.2f km²  ·  %d%%" % [census["arable_km2"], int(ar * 100.0)], ar)
		ui.put("survey", "relief", "%.0f – %.0f m  ·  mean %.0f" % [
				census["min_elevation"], census["max_elevation"], census["mean_elevation"]])
		# Biome census (14 ids) with legacy bins kept for glance.
		for k in ["water", "wetland", "riparian", "grassland", "scrub",
				"forest", "alpine", "bare_rock", "farm",
				"swamp", "meadow", "desert", "dune", "shore",
				"alluvial", "grass", "upland", "rock"]:
			if not census.has(k):
				continue
			var fr: float = float(census[k])
			ui.put("survey", k, "%.1f%%" % (fr * 100.0), fr)
		if census.has("hypso_below_water"):
			ui.put("survey", "seas", "%.1f%% below WL" % (float(census["hypso_below_water"]) * 100.0),
					float(census["hypso_below_water"]))
		if census.has("hypso_peak"):
			ui.put("survey", "peaks", "%.1f%% high" % (float(census["hypso_peak"]) * 100.0),
					float(census["hypso_peak"]))
	elif v == 2:
		ui.put("plan", "scale", "%d m across" % int(mini_size))
		ui.put("plan", "along", "z %+.0f m" % player.z)
		ui.put("plan", "around", "%.0f of %.0f m" % [
				fposmod(player.theta * float(P["radius"]), float(P["circumference"])),
				P["circumference"]])
		ui.put("plan", "mods", "%d modules  ·  %d strokes" % [modules.size(), terrain.edit_count()])

	ui.put("habitat", "grav", "%.2f m/s²  ·  %.0f m radius" % [P["gravity"], P["radius"]])

	var pk: Dictionary = terrain.inventory()
	var kg: float = float(pk.get("mass_kg", 0.0))
	var maxkg: float = maxf(float(pk.get("max_mass_kg", 90.0)), 1.0)
	var vfrac: float = float(pk.get("volume_frac", 0.0))
	ui.put("you", "pack", "%.1f / %.0f kg  ·  %d%% vol" % [kg, maxkg, int(vfrac * 100.0)],
			maxf(kg / maxkg, vfrac))
	# Top stack so timber / clay reads as what you are carrying, not just weight.
	var top := ""
	var top_kg := 0.0
	for s in pk.get("stacks", []):
		var sk: float = float(s.get("mass_kg", 0.0))
		if sk > top_kg:
			top_kg = sk
			top = str(s.get("name", "?"))
	if top_kg > 0.05:
		ui.put("you", "carry", "%.0f kg %s" % [top_kg, top], top_kg / maxkg)
	else:
		ui.put("you", "carry", "empty")
	# Encumbrance is a movement MULTIPLIER, so show the penalty, not the value.
	var encm: float = float(pk.get("encumbrance", 1.0))
	ui.put("you", "enc", "%d%% speed" % int(encm * 100.0), 1.0 - clampf(encm, 0.0, 1.0))
	ui.put("you", "home", home_bearing().split("\n")[0])

	var bio: Dictionary = terrain.biome_at(player.theta, player.z)
	ui.put("ground", "biome", str(bio.get("name", "—")))
	var so: Dictionary = terrain.soil_at(player.theta, player.z)
	ui.put("ground", "npk", "%.2f  %.2f  %.2f" % [
			so.get("n", 0.0), so.get("p", 0.0), so.get("k", 0.0)])
	var mo: float = float(so.get("moisture", 0.0))
	ui.put("ground", "moist", "%.2f" % mo, mo)
	ui.put("ground", "ph", "%.1f" % so.get("ph", 7.0))
	ui.put("ground", "drain", "%.2f" % fx, fx)

	var w: Dictionary = terrain.weather_at(player.theta, player.z)
	var rain: float = float(w.get("rain", 0.0))
	var sev: int = int(w.get("sky_event", sky_event))
	var sky_lbl := "clear"
	if sev == 1:
		sky_lbl = "fog"
	elif sev == 2:
		sky_lbl = "storm"
	ui.put("weather", "sky", "%s  ·  rain %.2f  ·  band %d" % [
			sky_lbl, rain, int(w.get("band", 0))])
	ui.put("weather", "temp", "%.0f °C" % w.get("temp", 15.0))
	var hu: float = float(w.get("humidity", 0.0))
	ui.put("weather", "humid", "%.2f" % hu, hu)

	var pw: Dictionary = terrain.power_budget()
	var used: float = float(pw.get("used", 0.0))
	var cap: float = maxf(float(pw.get("budget", 24.0)), 0.1)
	ui.put("colony", "power", "%.1f / %.0f MW  ·  %d cond" % [
			used, cap, int(pw.get("condensers", 0))], used / cap)
	var air: Dictionary = terrain.atmosphere()
	ui.put("colony", "air", "O₂ %.1f%%  ·  CO₂ %d ppm" % [
			float(air.get("o2_frac", 0.209)) * 100.0, int(air.get("co2_ppm", 400.0))])
	ui.put("colony", "life", "%d plants  ·  %d chunks" % [terrain.plant_count(), loaded.size()])
	var an: int = int(last_sim.get("agents", 0))
	var fol: float = float(last_sim.get("followers", 0.0))
	ui.put("colony", "people", "%d named  ·  %d following" % [an, int(round(fol))])
	_push_place()




## The overview views are instruments, not photographs: no lens blur, no dust,
## no cloud deck between you and the data.
func apply_view(v: int) -> void:
	if post_layer: post_layer.visible = (v == 0)
	if dust: dust.visible = (v == 0)
	if insects: insects.visible = (v == 0)
	if cloud_node: cloud_node.visible = (v == 0)
	# The survey view is not looking through 2.5 km of air at a photograph;
	# it is reading the habitat. Pull the haze back so the land is legible.
	RenderingServer.global_shader_parameter_set("rama_haze",
		1.0 if v == 0 else (0.45 if v == 1 else 0.10))

## Daylight is a schedule someone set, not an orbit. Dawn and dusk are events
## that choreograph fog, ambient, mist and far-side glow (1191–1195).
func _tick_daylight(dt: float) -> void:
	clock += dt * day_speed
	var phase: float = fposmod(clock / DAY_LENGTH, 1.0)
	# Long day, short dusk, short night: a colony optimises for growing hours.
	day = clamp(smoothstep(0.02, 0.14, phase) - smoothstep(0.70, 0.90, phase), 0.0, 1.0)
	day = 0.06 + 0.94 * day
	RenderingServer.global_shader_parameter_set("rama_day", day)
	# Travelling gust envelope — grass/trees read as one breathing field.
	var gust: float = 0.45 + 0.35 * sin(clock * 0.31) + 0.20 * sin(clock * 0.77 + 1.4)
	# Storm wind shove.
	gust += sky_intensity * (0.55 if sky_event == 2 else 0.15)
	RenderingServer.global_shader_parameter_set("rama_gust", clampf(gust, 0.15, 1.55))
	# Band first so spine/carriage update before RamaSun proximity read.
	var band := "day"
	if phase < 0.12:
		band = "dawn"
	elif phase > 0.88:
		band = "night"
	elif phase > 0.68:
		band = "dusk"
	_sync_photothermal_spine(phase, band)
	_aim_rama_sun()
	_poll_sky_weather(dt)
	bounce_accum += dt
	# Only recompute opposite-wall bounce when we've moved or every ~5 s.
	var need_bounce := bounce_accum > 5.0
	if player != null and not need_bounce:
		var here := Vector2(player.theta, player.z)
		var moved: float = absf(wrapf(here.x - bounce_anchor.x, -PI, PI)) * float(P.get("radius", 900.0))
		moved += absf(here.y - bounce_anchor.y)
		need_bounce = moved > 80.0 and bounce_accum > 1.2
	if need_bounce:
		bounce_accum = 0.0
		if player != null:
			bounce_anchor = Vector2(player.theta, player.z)
		_refresh_bounce_tint()
	# Dusk settlement lights — works glow diegetically (Wave 1.5 / NEXT #5).
	var dusk_boost: float = 0.0
	if phase > 0.68:
		dusk_boost = smoothstep(0.68, 0.92, phase) * 2.4
		# Sunset hour peaks warmer / brighter before night.
		dusk_boost *= 1.0 + smoothstep(0.72, 0.82, phase) * (1.0 - smoothstep(0.86, 0.95, phase)) * 0.55
	if absf(dusk_boost - _last_dusk_boost) > 0.02:
		_last_dusk_boost = dusk_boost
		for mat in dwelling_mats:
			if mat:
				mat.set_shader_parameter("emission_boost", dusk_boost)

	# Band labels for one-shot cues.
	if band != last_day_band:
		if band == "dawn":
			steam_life = 4.5
			print("[rama] dawn — photothermal carriage entering")
		elif band == "dusk":
			print("[rama] dusk — carriage departing / rings holding")
		last_day_band = band

	var env: Environment = world_env.environment if world_env else null
	if env:
		# Valley mist overnight, burns off in bands (1193).
		var mist := 0.0
		if phase < 0.18:
			mist = smoothstep(0.0, 0.08, phase) * (1.0 - smoothstep(0.12, 0.22, phase))
		elif phase > 0.92:
			mist = smoothstep(0.92, 1.0, phase)
		mist += sky_fog * (0.85 if sky_event == 1 else 0.35)
		mist_phase = mist
		var fog_d := 0.00055 + mist * 0.0018 + (1.0 - day) * 0.00035
		fog_d += sky_fog * 0.0028
		if sky_event == 2:
			fog_d += sky_intensity * 0.0011
		# Swamp / wetland local mist near the player — peat air (Wave 4).
		if player != null:
			var bname: String = water_bio_name
			if bname == "swamp" or bname == "wetland":
				fog_d += 0.0009 + mist * 0.0006
			elif bname == "meadow":
				fog_d *= 0.88 # clearer meadow air
			elif bname == "desert" or bname == "dune":
				fog_d += 0.00025 # fine dust haze
		# Carriage overhead: slight thermohydronic haze.
		if player != null:
			var cprox: float = 1.0 - clampf(absf(player.z - carriage_z) / CARRIAGE_PROX_M, 0.0, 1.0)
			fog_d += cprox * day * 0.00035
		env.fog_density = clampf(fog_d, 0.00035, 0.0042)
		# Cool blue night → warm dawn → clear day → copper sunset → night.
		var air_night := Color(0.07, 0.10, 0.18)
		var air_dawn := Color(0.42, 0.48, 0.55)
		var air_day := Color(0.16, 0.22, 0.28)
		var air_dusk := Color(0.38, 0.20, 0.12)
		var air_sunset := Color(0.52, 0.22, 0.14)
		var air: Color
		if phase < 0.14:
			air = air_night.lerp(air_dawn, smoothstep(0.02, 0.12, phase))
		elif phase < 0.22:
			air = air_dawn.lerp(air_day, smoothstep(0.14, 0.22, phase))
		elif phase < 0.68:
			air = air_day
		elif phase < 0.78:
			air = air_day.lerp(air_sunset, smoothstep(0.68, 0.78, phase))
		elif phase < 0.88:
			air = air_sunset.lerp(air_dusk, smoothstep(0.78, 0.88, phase))
		else:
			air = air_dusk.lerp(air_night, smoothstep(0.88, 1.0, phase))
		# Fog banks bleach warm air; storms cool it.
		if sky_event == 1:
			air = air.lerp(Color(0.55, 0.60, 0.68), sky_fog * 0.55)
		elif sky_event == 2:
			air = air.lerp(Color(0.12, 0.14, 0.18), sky_intensity * 0.45)
		env.background_color = air
		env.fog_light_color = air
		env.ambient_light_color = air.lightened(0.12)
		env.ambient_light_energy = 0.22 + 0.38 * day + mist * 0.08 - sky_intensity * 0.06
		env.fog_light_energy = 0.85 + 0.35 * day + sky_fog * 0.25
		# Far-side settlement glow as dusk deepens (1192) — peak at sunset hour.
		var sunset_peak: float = 0.0
		if phase > 0.68 and phase < 0.92:
			sunset_peak = smoothstep(0.70, 0.80, phase) * (1.0 - smoothstep(0.84, 0.94, phase))
		env.glow_intensity = 0.28 + (1.0 - day) * 0.50 + sunset_peak * 0.55
		env.glow_bloom = 0.05 + (1.0 - day) * 0.14 + sunset_peak * 0.12
		env.tonemap_exposure = 0.80 + 0.20 * day - sky_intensity * 0.08 + sunset_peak * 0.06

	# Cloud deck thickens in fog / storms; warm tint already from rama_day.
	if cloud_node and cloud_node.material_override and Engine.get_process_frames() % 4 == 0:
		var cover: float = 0.55 + sky_fog * 0.22 + (sky_intensity * 0.28 if sky_event == 2 else 0.0)
		cloud_node.material_override.set_shader_parameter("cover", clampf(cover, 0.35, 0.92))

	if dust and player and Engine.get_process_frames() % 2 == 0:
		dust.global_position = player.feet_pos()
		# Gentle drift along the drum axis — air moves, and motes that are
		# perfectly still read as stuck.
		var drift := Vector3(0, 0, sin(clock * 0.23) * 0.45) \
				+ Vector3(sin(clock * 0.41) * 0.22, cos(clock * 0.37) * 0.14, 0)
		dust.global_position += drift
		# Don't fight apply_view survey modes — only hide motes in thick fog
		# while the colonist cam is up.
		if post_layer == null or post_layer.visible:
			dust.visible = sky_event != 1 or sky_fog < 0.55
	if insects and player:
		var show_bugs := (post_layer == null or post_layer.visible) \
				and sky_event != 2 and sky_fog < 0.7
		insects.visible = show_bugs
		_insect_accum += dt
		if show_bugs and _insect_accum >= 0.10:
			_insect_accum = 0.0
			var feet: Vector3 = player.feet_pos()
			var up := Vector3(-feet.x, -feet.y, 0.0).normalized()
			var tang := up.cross(Vector3(0, 0, 1)).normalized()
			if tang.length_squared() < 0.01:
				tang = up.cross(Vector3(1, 0, 0)).normalized()
			var axial := tang.cross(up).normalized()
			const INSECT_N := 28
			if insects.multimesh.instance_count != INSECT_N:
				insects.multimesh.instance_count = INSECT_N
				for i in INSECT_N:
					var warm: float = float(i % 5) / 5.0
					insects.multimesh.set_instance_color(i, Color(
							0.42 + warm * 0.28, 0.34 + warm * 0.18, 0.16 + warm * 0.08, 0.28))
			for i in INSECT_N:
				var ph: float = float(i) * 1.7 + clock * (0.9 + float(i % 4) * 0.15)
				var rad: float = 2.5 + float(i % 7) * 0.85
				var loft: float = 0.8 + 0.55 * sin(ph * 1.3) + float(i % 5) * 0.35
				var pos: Vector3 = feet + tang * cos(ph) * rad + axial * sin(ph * 1.15) * rad \
						+ up * loft
				var xf := Transform3D(Basis.IDENTITY, pos)
				var s: float = 0.7 + float(i % 3) * 0.25
				xf.basis = xf.basis.scaled(Vector3(s, s, s))
				insects.multimesh.set_instance_transform(i, xf)

	# Steam off wet ground when the strip comes up (1195).
	if steam_life > 0.0 and steam_mm and player:
		steam_life -= dt
		steam_mm.visible = true
		var base: Vector3 = player.feet_pos()
		var steam_up := Vector3(-base.x, -base.y, 0.0).normalized()
		var wdep: float = terrain.water_depth_at(player.theta, player.z)
		# Soft mist, not white hex plates in front of the camera.
		var a := 0.06 + 0.14 * clampf(steam_life / 4.5, 0.0, 1.0) * (0.35 + minf(wdep, 1.0))
		const STEAM_N := 12
		steam_mm.multimesh.instance_count = STEAM_N
		for i in STEAM_N:
			var ang: float = float(i) / float(STEAM_N) * TAU + clock * 0.35
			var lateral := Vector3(cos(ang), sin(ang), sin(ang * 1.7) * 0.25)
			lateral = (lateral - steam_up * lateral.dot(steam_up)).normalized()
			var loft: float = 0.25 + float(i % 4) * 0.18 + (4.5 - steam_life) * 0.08
			var xf := Transform3D(Basis.IDENTITY, base + lateral * (0.8 + i * 0.22) + steam_up * loft)
			var s: float = 0.35 + (i % 3) * 0.12
			xf.basis = xf.basis.scaled(Vector3(s, s * 1.15, s))
			steam_mm.multimesh.set_instance_transform(i, xf)
			steam_mm.multimesh.set_instance_color(i, Color(0.88, 0.91, 0.94, a))
	elif steam_mm:
		steam_mm.visible = false
		steam_mm.multimesh.instance_count = 0

# ----------------------------------------------------------- hud panels --

## Both instruments live on screen at once: the whole habitat top-right, the
## ground under your feet bottom-right. You can always see where you are in
## both, which is the point — one is context, the other is action.
func _build_panels() -> void:
	hud_panels = CanvasLayer.new()
	hud_panels.layer = 2
	add_child(hud_panels)

	# --- entire drum, unrolled ---
	var mw := 300
	var mh := 190
	var img := Image.create_from_data(384, 240, false, Image.FORMAT_RGB8,
			terrain.biome_map(384, 240))
	var tex := ImageTexture.create_from_image(img)
	var biome_rect := TextureRect.new()
	biome_rect.texture = tex
	biome_rect.stretch_mode = TextureRect.STRETCH_SCALE
	biome_rect.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	biome_rect.position = Vector2(-mw - 18, 18)
	biome_rect.size = Vector2(mw, mh)
	biome_rect.modulate = Color(1, 1, 1, 0.62)
	biome_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hud_panels.add_child(biome_rect)
	biome_tex_rect = biome_rect

	ship_overlay = Control.new()
	ship_overlay.set_script(load("res://scripts/overlay.gd"))
	ship_overlay.world = self
	ship_overlay.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	ship_overlay.position = Vector2(-mw - 18, 18)
	ship_overlay.size = Vector2(mw, mh)
	ship_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hud_panels.add_child(ship_overlay)

	var lbl := Label.new()
	lbl.text = "KEPLER DRUM · live drainage"
	lbl.add_theme_font_size_override("font_size", 10)
	lbl.add_theme_color_override("font_color", Color(0.85, 0.90, 0.94, 0.8))
	lbl.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.8))
	lbl.add_theme_constant_override("outline_size", 4)
	lbl.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	lbl.position = Vector2(-mw - 18, mh + 20)
	hud_panels.add_child(lbl)
	map_label = lbl

	# --- live plan view of the ground you are on ---
	var side := 236
	mini_vp = SubViewport.new()
	mini_vp.size = Vector2i(side, side)
	# Manual cadence: render every 3rd frame, not every frame. Saves a full
	# extra camera pass 2/3 of frames.
	mini_vp.render_target_update_mode = SubViewport.UPDATE_DISABLED
	mini_vp.transparent_bg = false
	mini_vp.own_world_3d = false
	add_child(mini_vp)
	mini_cam = Camera3D.new()
	mini_cam.projection = Camera3D.PROJECTION_ORTHOGONAL
	mini_cam.size = mini_size
	mini_cam.far = 900.0
	mini_vp.add_child(mini_cam)
	mini_vp.world_3d = get_viewport().world_3d

	var mrect := TextureRect.new()
	mrect.texture = mini_vp.get_texture()
	mrect.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	mrect.position = Vector2(-side - 18, -side - 18)
	mrect.size = Vector2(side, side)
	mrect.modulate = Color(1, 1, 1, 0.92)
	mrect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hud_panels.add_child(mrect)

	mini_overlay = Control.new()
	mini_overlay.set_script(load("res://scripts/minimap_overlay.gd"))
	mini_overlay.world = self
	mini_overlay.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	mini_overlay.position = Vector2(-side - 18, -side - 18)
	mini_overlay.size = Vector2(side, side)
	mini_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hud_panels.add_child(mini_overlay)

	# Soil chemistry tint over the live plan (NEXT #5).
	soil_overlay = TextureRect.new()
	soil_overlay.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	soil_overlay.position = Vector2(-side - 18, -side - 18)
	soil_overlay.size = Vector2(side, side)
	soil_overlay.modulate = Color(1, 1, 1, 0.38)
	soil_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	soil_overlay.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	soil_overlay.stretch_mode = TextureRect.STRETCH_SCALE
	hud_panels.add_child(soil_overlay)

	soil_chip = ColorRect.new()
	soil_chip.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	soil_chip.position = Vector2(-side - 18, -38)
	soil_chip.size = Vector2(14, 14)
	soil_chip.color = Color(0.35, 0.55, 0.28, 0.9)
	soil_chip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hud_panels.add_child(soil_chip)
	soil_chip_label = Label.new()
	soil_chip_label.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	soil_chip_label.position = Vector2(-side + 2, -40)
	soil_chip_label.add_theme_font_size_override("font_size", 11)
	soil_chip_label.add_theme_color_override("font_color", Color(0.88, 0.93, 0.95, 0.9))
	soil_chip_label.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.85))
	soil_chip_label.add_theme_constant_override("outline_size", 4)
	soil_chip_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hud_panels.add_child(soil_chip_label)
	_refresh_soil_overlay()

func _refresh_soil_overlay() -> void:
	if soil_overlay == null or terrain == null:
		return
	var raw: PackedByteArray = terrain.soil_map(128, 128, soil_mode)
	# Colour-blind remap (§2106) + optional isolines (§2100).
	var img := Image.create(128, 128, false, Image.FORMAT_RGB8)
	for y in 128:
		for x in 128:
			var i: int = (y * 128 + x) * 3
			var c: Color = RamaControls.remap_overlay_rgb(
					raw[i] / 255.0, raw[i + 1] / 255.0, raw[i + 2] / 255.0)
			img.set_pixel(x, y, c)
	if RamaControls.overlay_contours:
		_draw_overlay_contours(img, raw)
	soil_overlay.texture = ImageTexture.create_from_image(img)
	var names := ["org·N·wet", "organic", "nitrogen", "moisture"]
	var chip_cols := [
		Color(0.40, 0.55, 0.32, 0.95),
		Color(0.42, 0.32, 0.18, 0.95),
		Color(0.28, 0.55, 0.72, 0.95),
		Color(0.22, 0.42, 0.70, 0.95),
	]
	if soil_chip:
		soil_chip.color = chip_cols[soil_mode]
	if soil_chip_label:
		var pal: String = RamaControls.overlay_palette
		var tag: String = "" if pal == "default" else (" · " + pal.left(4))
		soil_chip_label.text = "soil · %s%s  (N)" % [names[soil_mode], tag]

func _draw_overlay_contours(img: Image, raw: PackedByteArray) -> void:
	## Isolines on the primary channel — cheapest legibility win (§2100).
	var w := 128
	var h := 128
	var levels := [64, 128, 192]
	for y in range(1, h - 1):
		for x in range(1, w - 1):
			var i: int = (y * w + x) * 3
			var v: int = raw[i] if soil_mode != 3 else raw[i + 2]
			var vx: int = raw[i + 3] if soil_mode != 3 else raw[i + 5]
			var vy: int = raw[i + w * 3] if soil_mode != 3 else raw[i + w * 3 + 2]
			for L in levels:
				if (v < L and vx >= L) or (v >= L and vx < L) \
						or (v < L and vy >= L) or (v >= L and vy < L):
					img.set_pixel(x, y, Color(0.95, 0.95, 0.90, 1.0))
					break

func cycle_soil_mode() -> void:
	soil_mode = (soil_mode + 1) % 4
	_refresh_soil_overlay()
	var names := ["org·N·wet", "organic", "nitrogen", "moisture"]
	print("[rama] soil overlay: %s" % names[soil_mode])

## A translucent preview of what RIGHT CLICK will install, snapped exactly
## where it will land. Blind placement is a bad way to build anything.
func _build_ghost() -> Node3D:
	var n := Node3D.new()
	var mi := MeshInstance3D.new()
	mi.name = "Body"
	mi.mesh = BoxMesh.new()
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.albedo_color = Color(0.62, 1.0, 0.74, 0.17)
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mi.material_override = mat
	n.add_child(mi)
	var cost := Label3D.new()
	cost.name = "Cost"
	cost.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	cost.font_size = 42
	cost.outline_size = 8
	cost.modulate = Color(0.85, 0.95, 1.0, 0.95)
	cost.position = Vector3(0, 4.2, 0)
	cost.visible = false
	n.add_child(cost)
	n.visible = false
	add_child(n)
	return n

func pose_ghost(g: Node3D, p: Vector3, kind: int) -> void:
	var spec: Dictionary = MODULES[kind]
	var th := atan2(p.y, p.x)
	var arc_step: float = 2.0 / float(P["radius"])
	th = round(th / arc_step) * arc_step
	var z: float = round(p.z / 2.0) * 2.0
	g.transform = frame_at(th, z, ground_at(th, z))
	var mi: MeshInstance3D = g.get_node("Body")
	var sz: Vector3 = spec["size"]
	mi.mesh.size = sz
	mi.position = Vector3(0, sz.y * 0.5, 0)
	var cost: Label3D = g.get_node("Cost")
	if kind == 2:
		const COND_MW := 1.2
		var power: Dictionary = terrain.power_budget()
		var head: float = float(power.get("headroom", 0.0))
		var ok: bool = head + 1e-3 >= COND_MW
		cost.visible = true
		cost.position = Vector3(0, sz.y + 1.4, 0)
		cost.text = "%.1f MW · headroom %.1f%s" % [COND_MW, head, "" if ok else "  FULL"]
		cost.modulate = Color(0.55, 0.95, 0.70) if ok else Color(0.95, 0.45, 0.40)
		var mat: StandardMaterial3D = mi.material_override
		if mat:
			mat.albedo_color = Color(0.55, 1.0, 0.70, 0.22) if ok else Color(1.0, 0.40, 0.35, 0.28)
	else:
		cost.visible = false
		var mat2: StandardMaterial3D = mi.material_override
		if mat2:
			mat2.albedo_color = Color(0.62, 1.0, 0.74, 0.17)

## How far home is, and which way. In a 5.6 km drum you will get lost.
## Drop a marker you can navigate back to. In a 5.6 km drum, "I will remember
## where that was" is not true.
func set_waypoint(th: float, zz: float) -> void:
	waypoint = Vector2(th, zz)
	has_waypoint = true
	if audio: audio.place()

func mini_zoom(dir: int) -> void:
	mini_size = clampf(mini_size * (1.35 if dir > 0 else 1.0 / 1.35), 40.0, 900.0)
	if mini_cam: mini_cam.size = mini_size

## Distance and relative bearing to an arbitrary point on the surface.
func bearing_to(target: Vector2, label: String) -> String:
	var dz: float = target.y - player.z
	var dth: float = wrapf(target.x - player.theta, -PI, PI)
	var arc: float = dth * float(P["radius"])
	var dist: float = sqrt(dz * dz + arc * arc)
	if dist < 25.0:
		return "%s · you are here" % label
	var rel: float = rad_to_deg(wrapf(atan2(arc, dz) - player.yaw, -PI, PI))
	var side := "ahead"
	if rel > 25.0: side = "right %d°" % int(rel)
	elif rel < -25.0: side = "left %d°" % int(-rel)
	return "%s  %s · %.0f m" % [label, side, dist]

func home_bearing() -> String:
	var t := bearing_to(home, "home")
	if has_waypoint:
		t += "\n" + bearing_to(waypoint, "mark")
	return t

## Transient one-line feedback, shown by the HUD toast component.
var toast := ""
var toast_t := 0.0

func note(t: String) -> void:
	toast = t
	toast_t = 3.0

func toggle_fast_day() -> void:
	day_speed = 1.0 if day_speed > 1.05 else FAST_DAY_MULT
	_apply_day_speed(true)

func _apply_day_speed(announce: bool) -> void:
	var pace: float = 1.0 if day_speed <= 1.05 else sqrt(day_speed) * 1.35
	if terrain != null and terrain.has_method("set_spectacle_pace"):
		terrain.set_spectacle_pace(pace)
	if announce:
		if day_speed > 1.05:
			note("fast day — full cycle ~%ds · F6 off" % int(DAY_LENGTH / day_speed))
			print("[rama] fast-day ON ×%.0f (spectacle pace %.1f)" % [day_speed, pace])
		else:
			note("day cycle normal")
			print("[rama] fast-day OFF")

## Smooth sky-event state from sim; toast on sudden fog / rain storm.
func _poll_sky_weather(dt: float) -> void:
	if terrain == null or not terrain.has_method("spine_status"):
		return
	_sky_poll_accum += dt
	if _sky_poll_accum >= 0.12:
		_sky_poll_accum = 0.0
		var st: Dictionary = terrain.spine_status()
		if bool(st.get("ok", false)):
			var ev: int = int(st.get("sky_event", 0))
			_sky_inten_target = float(st.get("sky_intensity", 0.0))
			_sky_fog_target = float(st.get("fog_factor", 0.0))
			if ev != last_sky_event:
				if ev == 1:
					note("fog bank rolling in")
					print("[rama] sky — fog bank")
					steam_life = maxf(steam_life, 3.2)
				elif ev == 2:
					note("rain storm")
					print("[rama] sky — rain storm")
					life_refresh_accum = 1.0
				elif last_sky_event == 1:
					note("fog lifting")
				elif last_sky_event == 2:
					note("storm passing")
				last_sky_event = ev
			sky_event = ev
	# Lerp every frame so fog/storm onset stays smooth between FFI polls.
	var k: float = clampf(dt * 3.2, 0.0, 1.0)
	sky_intensity = lerpf(sky_intensity, _sky_inten_target, k)
	sky_fog = lerpf(sky_fog, _sky_fog_target, k)

func playtest_mark(kind: String) -> void:
	if not playtest:
		return
	var now: int = Time.get_ticks_msec() - playtest_t0_ms
	match kind:
		"lookup":
			if playtest_lookup_ms < 0:
				playtest_lookup_ms = now
		"walk":
			if playtest_walk_ms < 0:
				playtest_walk_ms = now
		"dig":
			if playtest_dig_ms < 0:
				playtest_dig_ms = now
	if playtest_lookup_ms >= 0 and playtest_walk_ms >= 0 and playtest_dig_ms >= 0:
		var row := "playtest,lookup_ms=%d,walk_ms=%d,dig_ms=%d" % [
				playtest_lookup_ms, playtest_walk_ms, playtest_dig_ms]
		print("[rama] ", row)
		DisplayServer.clipboard_set(row)
		playtest = false  # one row

func _tick_panels() -> void:
	if mini_cam == null or player == null:
		return
	var u: Vector3 = player.up()
	var pos: Vector3 = player.feet_pos() + u * 170.0
	var fwd: Vector3 = -u
	var axial := Vector3(0, 0, 1)
	var right := fwd.cross(axial).normalized()
	var tu := right.cross(fwd).normalized()
	mini_cam.transform = Transform3D(Basis(right, tu, -fwd), pos)
	# Overlay redraw every other frame — continuous queue_redraw was cheap but
	# the mini SubViewport + catchment dirty marks were not.
	var frame: int = Engine.get_process_frames()
	if frame % 2 == 0:
		if mini_overlay: mini_overlay.queue_redraw()
		if ship_overlay: ship_overlay.queue_redraw()
		if reticle: reticle.queue_redraw()
	# Render the minimap SubViewport every 3rd frame instead of every frame.
	if frame % 3 == 0 and mini_vp:
		mini_vp.render_target_update_mode = SubViewport.UPDATE_ONCE
	# The panels are for playing, not for the full-screen instruments.
	if hud_panels:
		var panels_on: bool = (player.view == 0) and not RamaControls.photo_mode
		if RamaControls.hud_density == "off":
			panels_on = false
		hud_panels.visible = panels_on
	if reticle:
		reticle.visible = (player.view == 0) and not RamaControls.photo_mode
	_apply_reduced_motion()

func _apply_reduced_motion() -> void:
	## Reduced-motion: kill sway amplitudes (§2107). Only rewrite uniforms when
	## the preference flips — plant layers × every frame was pure busywork.
	var want: int = 1 if RamaControls.reduced_motion else 0
	if want == _reduced_motion_applied:
		return
	_reduced_motion_applied = want
	var sway: float = 0.0 if want == 1 else 0.22
	var tree_sway: float = 0.0 if want == 1 else 0.055
	if grass_mm and grass_mm.material_override is ShaderMaterial:
		(grass_mm.material_override as ShaderMaterial).set_shader_parameter("sway", sway)
	var layers: Array = []
	layers.append_array(plant_species_near)
	layers.append_array(plant_species_mid)
	layers.append(plant_mm_far)
	for mi in layers:
		if mi and mi.material_override is ShaderMaterial:
			(mi.material_override as ShaderMaterial).set_shader_parameter("sway", tree_sway)


# ------------------------------------------------------------- screenshots --

func _take_shots() -> void:
	# Headless uses the dummy renderer — no real viewport texture. Skip rather
	# than hang forever on frame_post_draw (LANDSCAPE_3200 verify note).
	if DisplayServer.get_name() == "headless":
		print("[rama] --shot SKIP under headless (dummy renderer has no viewport)")
		get_tree().quit()
		return
	await get_tree().process_frame
	RenderingServer.force_draw()
	await get_tree().process_frame
	var shot_dir := ProjectSettings.globalize_path("res://../shots")
	var shot_only := PackedStringArray()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--shot-dir="):
			shot_dir = arg.trim_prefix("--shot-dir=")
		elif arg.begins_with("--shot-only="):
			shot_only = arg.trim_prefix("--shot-only=").split(",")
	DirAccess.make_dir_recursive_absolute(shot_dir)
	var shots := [
		{"name": "01_wake", "pitch": -0.05, "yaw": 0.0, "h": 0.6},
		{"name": "02_stand", "pitch": 0.02, "yaw": 0.0, "h": 1.72},
		{"name": "03_lookup", "pitch": 1.02, "yaw": 0.2, "h": 1.72},
		{"name": "04_along", "pitch": 0.10, "yaw": 1.55, "h": 1.72},
		{"name": "05_mineaim", "pitch": -0.62, "yaw": 0.75, "h": 1.72, "dig": true},
		{"name": "06_mined", "pitch": -0.34, "yaw": 0.75, "h": 1.72},
		{"name": "07_level", "pitch": -0.52, "yaw": 2.5, "h": 1.72, "level": true},
		{"name": "08_drum", "pitch": 0.0, "yaw": 0.75, "h": 1.72, "view": 1},
		{"name": "09_map", "pitch": 0.0, "yaw": 0.0, "h": 1.72, "view": 2},
		{"name": "13_across", "pitch": 0.30, "yaw": 0.0, "h": 1.72, "vantage": true},
		# Places, after the colony has had time to grow into them. These two
		# are the whole point of the dwelling layer: a township that filled up
		# and a delve that never could.
		{"name": "10_township", "pitch": 0.0, "h": 1.72, "dwell": 2, "soak": true, "standoff": 34.0},
		{"name": "11_delve", "pitch": 0.0, "h": 1.72, "dwell": 0, "standoff": 26.0},
		# Same township, instruments on — this one is the HUD check.
		{"name": "12_place", "pitch": 0.0, "h": 1.72, "dwell": 2, "standoff": 34.0, "hud": true},
	]
	for s in shots:
		if not shot_only.is_empty() and str(s["name"]) not in shot_only:
			continue
		if s.get("soak", false):
			# Let people actually arrive before photographing where they live.
			for i in 24:
				terrain.sim_tick(5.0)
			refresh_dwellings()
		if s.get("vantage", false):
			# The longest sightline the habitat has — the view that exposed
			# every distance cut-off. Keep it as a regression shot.
			player.theta = 3.063
			player.z = 2724.0
			player.r = ground_at(player.theta, player.z)
			player.vr = 0.0
			player.yaw = PI
			terrain.set_player_pos(player.theta, player.z)
			player._update_camera()
			_queue_chunks()
			_pump_chunks(400)
			refresh_mid(true)
			refresh_grass()
		# These are photographs of a place, not of an interface.
		RamaControls.photo_mode = s.has("dwell") and not s.get("hud", false)
		if s.has("dwell"):
			var want: int = int(s["dwell"])
			var db: PackedFloat32Array = terrain.dwellings_lod()
			var best := -1
			var best_score := -1.0
			for i in int(db.size() / 9.0):
				if int(db[i * 9 + 2]) != want:
					continue
				var score: float = db[i * 9 + 3] + db[i * 9 + 4]
				if score > best_score:
					best_score = score
					best = i
			if best < 0:
				print("[rama] no dwelling of kind %d to shoot" % want)
				continue
			# Stand DOWNHILL of the place and aim at it. A fixed axial offset
			# with a hand-tuned pitch buries the camera in the first hillside
			# it meets — and a delve is by definition on a hillside.
			var sth: float = db[best * 9]
			var szz: float = db[best * 9 + 1]
			var sgr: float = ground_at(sth, szz)
			var standoff: float = float(s.get("standoff", 26.0))
			var bth: float = sth
			var bzz: float = szz - standoff
			var blow: float = -1.0
			for k in 12:
				var a: float = float(k) / 12.0 * TAU
				var cth: float = sth + cos(a) * standoff / P["radius"]
				var czz: float = szz + sin(a) * standoff
				# Larger radius is nearer the hull, i.e. LOWER ground.
				var cg: float = ground_at(cth, czz)
				if cg > blow:
					blow = cg
					bth = cth
					bzz = czz
			player.theta = bth
			player.z = bzz
			player.r = ground_at(bth, bzz)
			player.vr = 0.0
			terrain.set_player_pos(player.theta, player.z)
			# _pump_chunks only drains the queue — the queue itself is built
			# around wherever the player was. Re-queue first, or the near mesh
			# never follows and everything looks like it is floating over the
			# far field.
			player._update_camera()
			_queue_chunks()
			_pump_chunks(400)
			refresh_grass()
			refresh_mid(true)
			refresh_dwellings()
			# Aim the camera at the place instead of guessing a pitch.
			var eye_p: Vector3 = to_world(player.theta, player.z, player.r - float(s["h"]))
			var tgt_p: Vector3 = to_world(sth, szz, sgr - 2.0)
			var dir: Vector3 = (tgt_p - eye_p).normalized()
			var up_v: Vector3 = up_at(eye_p)
			var tan_v := Vector3(-sin(player.theta), cos(player.theta), 0.0)
			player.yaw = atan2(dir.dot(tan_v), dir.z)
			player.pitch = asin(clampf(dir.dot(up_v), -1.0, 1.0))
			s["pitch"] = player.pitch
			s["yaw"] = player.yaw
		player.view = int(s.get("view", 0))
		apply_view(player.view)
		player.cam_ready = false
		player.pitch = s["pitch"]
		player.yaw = float(s.get("yaw", player.yaw))
		player.eye = s["h"]
		player._update_camera()
		if s.get("level", false):
			# Cut a flat building pad with the levelling brush.
			player.level_brush = true
			player.brush = 6.0
			var n2 := 0
			for k in 8:
				var r2: Dictionary = player.aim()
				if not r2.get("hit", false):
					break
				terrain.dig(r2["point"], player.brush, DIG_SNAP, true)
				rebuild_around(r2["point"], player.brush * 2.0 + 2.0)
				n2 += 1
			print("[rama] levelled %d passes, %d strokes" % [n2, terrain.edit_count()])
			player.level_brush = false
		if s.get("dig", false):
			# Drive an actual excavation through the public API.
			var n := 0
			for k in 26:
				var r: Dictionary = player.aim()
				if not r.get("hit", false):
					break
				var pt: Vector3 = r["point"]
				terrain.dig(pt, 3.4, DIG_SNAP, false)
				rebuild_around(pt, 5.4)
				n += 1
			print("[rama] excavated %d bites, %d strokes stored" % [n, terrain.edit_count()])
		await RenderingServer.frame_post_draw
		await RenderingServer.frame_post_draw
		var img := get_viewport().get_texture().get_image()
		var path := shot_dir.path_join("%s.png" % s["name"])
		img.save_png(path)
		# Screenshot metadata (§2120): seed, position, date beside the PNG.
		var meta := {
			"name": s["name"],
			"seed": int(P.get("seed", 0)),
			"theta": player.theta,
			"z": player.z,
			"pitch": player.pitch,
			"yaw": player.yaw,
			"clock": clock,
			"date": Time.get_datetime_string_from_system(true),
		}
		var mf := FileAccess.open(shot_dir.path_join("%s.json" % s["name"]), FileAccess.WRITE)
		if mf:
			mf.store_string(JSON.stringify(meta))
			mf.close()
		print("[rama] shot ", s["name"])
	get_tree().quit()
