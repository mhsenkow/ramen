extends Node3D
const RamaControls = preload("res://scripts/controls.gd")
## RAMA CYCLE — MVP-0. The world you can stand in. (REQUIREMENTS.md §G)
##
## Everything geometric comes from the Rust core (rama_sim). This script places
## it, lights it, and gets a camera into it. No terrain maths lives here.

const FAR_NT := 1280
const FAR_NZ := 800
const FAR_OFFSET := 1.15     # sit under near chunks so the LOD edge doesn't z-fight
const CHUNK_SPAN := 44.0     # metres per near chunk, both lateral axes
const CHUNK_CELL := 1.4      # voxel size, metres
const CHUNK_RADIUS := 5      # chunks loaded around the player
const UNLOAD_RADIUS := 6
const DIG_SNAP := 1.0        # brush centres land on a 1 m grid
const NEAR_FADE_START := 150.0
const NEAR_FADE_END := 235.0

var terrain                  # RamaTerrain (Rust)
var P: Dictionary
var spawn: Dictionary
var player: Node3D
var hud: Label
var loaded := {}
var pending: Array = []
var chunk_root: Node3D
var shot_mode := false
var modules: Array = []
var day := 1.0
var clock := 0.0
var post_layer: CanvasLayer
var cloud_node: MeshInstance3D
var axis_mat: StandardMaterial3D
var dust: MultiMeshInstance3D
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
var plot_mm: MultiMeshInstance3D
var carcass_mm: MultiMeshInstance3D
var station_mm: MultiMeshInstance3D
var world_env: WorldEnvironment
var mist_phase := 0.0
var steam_life := 0.0
var steam_mm: MultiMeshInstance3D
var last_day_band := ""
var catchment_root: Node3D
var catchment_phase := 0.0
var autosave_accum := 0.0
const AUTOSAVE_SECS := 180.0
var map_label: Label
var soil_chip: ColorRect
var soil_chip_label: Label
var grass_mm: MultiMeshInstance3D
var grass_anchor := Vector2(1e9, 1e9)
const GRASS_N := 5200
const GRASS_RADIUS := 27.0
const GRASS_MOVE := 7.0
var plant_mm: MultiMeshInstance3D
var plant_mm_mid: MultiMeshInstance3D
var plant_mm_far: MultiMeshInstance3D
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
const SIM_STEP_DAYS := 0.02  # ~ habitat days per real second at 1x
const CATCHMENT_COOLDOWN := 0.28
const PLANT_FILL_BUDGET := 120
const SAVE_PATH := "user://rama_strokes.bin"
const SAVE_SOIL := "user://rama_soil.bin"
const SAVE_WORLD := "user://rama_world.json"

func _ready() -> void:
	shot_mode = "--shot" in OS.get_cmdline_user_args()
	var selftest := "--selftest" in OS.get_cmdline_user_args()
	terrain = ClassDB.instantiate("RamaTerrain")
	var t0 := Time.get_ticks_msec()
	terrain.generate(0)
	P = terrain.params()
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

	clock = DAY_LENGTH * 0.30   # wake mid-morning, not at midnight
	RamaControls.install()
	menu = load("res://scripts/menu.gd").new()
	menu.world = self
	add_child(menu)
	audio = load("res://scripts/audio.gd").new()
	add_child(audio)
	RenderingServer.global_shader_parameter_add("rama_day",
		RenderingServer.GLOBAL_VAR_TYPE_FLOAT, 1.0)
	RenderingServer.global_shader_parameter_add("rama_haze",
		RenderingServer.GLOBAL_VAR_TYPE_FLOAT, 1.0)
	_build_env()
	_build_far()
	_build_water()
	_build_rivers()
	_build_pools()
	_build_grass()
	_build_foam()
	_build_splash()
	_build_stockpiles()
	_build_agents()
	_build_carcasses()
	_build_stations()
	_build_steam()
	_build_endcaps()
	_build_axis_light()
	chunk_root = Node3D.new(); add_child(chunk_root)
	_build_homestead()
	_build_towns()
	_build_clouds()
	_build_dust()
	_build_player()
	_build_plants()
	_build_overlays()
	_build_panels()
	player.ghost = _build_ghost()
	_queue_chunks()
	# Only the chunks you are standing on block the first frame. The rest
	# stream in over the following seconds.
	_pump_chunks(18)
	# One biosphere tick so rain/soil exist on first frame.
	last_sim = terrain.sim_tick(0.05)
	refresh_agents()
	print("[rama] biosphere: %d plants, lakes=%s, rain=%.3f, agents=%d" % [
		terrain.plant_count(), terrain.lake_count(), float(last_sim.get("rain", 0.0)),
		int(last_sim.get("agents", 0))])
	if selftest:
		_selftest()
	elif shot_mode:
		_take_shots()

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
	print("player feet radius  : %.2f  (ground %.2f)" % [player.r, ground_at(player.theta, player.z)])
	print("player world pos    : %s" % to_world(player.theta, player.z, player.r))
	print("local up vector     : %s" % up_at(to_world(player.theta, player.z, player.r)))
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
	print("================================================\n")
	get_tree().quit()

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

func _terrain_material() -> ShaderMaterial:
	var sm := ShaderMaterial.new()
	sm.shader = load("res://shaders/terrain.gdshader")
	sm.set_shader_parameter("hab_radius", float(P["radius"]))
	return sm

func _far_terrain_material() -> ShaderMaterial:
	var sm := _terrain_material()
	# Hide coarse far LOD under the near ring, and haze it harder so the
	# remaining silhouette dissolves instead of reading as a resolution cliff.
	sm.set_shader_parameter("near_fade_start", NEAR_FADE_START)
	sm.set_shader_parameter("near_fade_end", NEAR_FADE_END)
	sm.set_shader_parameter("haze_start", 90.0)
	sm.set_shader_parameter("haze_end", 1800.0)
	sm.set_shader_parameter("haze_max", 0.98)
	return sm

# ------------------------------------------------------------------- build --

func _build_env() -> void:
	world_env = WorldEnvironment.new()
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	# Match fog so empty axis-space fills with the same air as hazed geometry.
	# Without this, terrain silhouettes cut hard against clear navy void.
	var air := Color(0.16, 0.22, 0.28)
	env.background_color = air
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.26, 0.32, 0.34)
	env.ambient_light_energy = 0.42
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	env.tonemap_exposure = 0.88
	# Tighter glow: raised threshold only catches axis strip and emissives.
	# Lower bloom prevents midtone wash that made the scene milky.
	env.glow_enabled = true
	env.glow_intensity = 0.24
	env.glow_bloom = 0.04
	env.glow_hdr_threshold = 1.4
	env.glow_hdr_scale = 0.7
	# Engine fog fills EMPTY air (shader haze only tints geometry). This is what
	# softens the jagged horizon and the endcap rim into the distance.
	env.fog_enabled = true
	env.fog_mode = Environment.FOG_MODE_DEPTH
	env.fog_light_color = air
	env.fog_light_energy = 1.08
	env.fog_density = 0.0007
	env.fog_aerial_perspective = 0.55
	env.fog_sky_affect = 1.0
	env.fog_sun_scatter = 0.06
	# Sharper near-clear / far-fade profile. Makes the tilt-shift focus band
	# pop harder against the soft far field.
	env.fog_depth_curve = 0.55
	# Start earlier so coarse far silhouettes dissolve before they read as
	# black saw-teeth against the sky.
	env.fog_depth_begin = 100.0
	env.fog_depth_end = 1800.0
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
	var mimg := Image.create_from_data(512, 320, false, Image.FORMAT_R8,
			terrain.lake_mask(512, 320))
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
		# Sit tighter on the bed so ribbons don't float as blue tiles.
		var up_lift := up * 0.02
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
## the player has actually moved. 2600 tufts x 4 tris is nothing next to the
## 215k the terrain already costs.
func _build_grass() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	# Three crossed blades, each TAPERED to a point. A rectangle reads as a
	# card; a taper reads as grass. Six triangles per tuft.
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for blade in 3:
		var ang: float = PI / 3.0 * float(blade)
		var dx := cos(ang) * 0.5
		var dz := sin(ang) * 0.5
		var nrm := Vector3(-sin(ang), 0.45, cos(ang)).normalized()
		# Wide at the base, pinched at the tip.
		var b0 := Vector3(-dx, 0.0, -dz)
		var b1 := Vector3(dx, 0.0, dz)
		var t0 := Vector3(-dx * 0.18, 1.0, -dz * 0.18)
		var t1 := Vector3(dx * 0.18, 1.0, dz * 0.18)
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

func refresh_grass(force := false) -> void:
	if grass_mm == null or player == null or terrain == null:
		return
	var here := Vector2(player.theta * float(P["radius"]), player.z)
	if not force and here.distance_to(grass_anchor) < GRASS_MOVE:
		return
	grass_anchor = here
	var buf: PackedFloat32Array = terrain.grass_field(
			player.theta, player.z, GRASS_RADIUS, GRASS_N)
	var n: int = int(buf.size() / 5.0)
	grass_mm.multimesh.instance_count = n
	for i in n:
		var th: float = buf[i * 5]
		var zz: float = buf[i * 5 + 1]
		var rr: float = buf[i * 5 + 2]
		var sc: float = buf[i * 5 + 3]
		var lush: float = buf[i * 5 + 4]
		var xf := frame_at(th, zz, rr)
		# Ankle-to-knee, not hedge-height. Width well under height so a tuft
		# reads as blades rather than as a billboard.
		# Random yaw per tuft. Without this every blade faces the same way and
		# the field reads as a printed grid rather than as ground cover.
		var yaw: float = fposmod(sc * 97.31 + lush * 41.7, 1.0) * TAU
		xf.basis = xf.basis.rotated(xf.basis.y, yaw)
		var hgt: float = 0.13 + lush * 0.20 + fposmod(sc * 13.7, 1.0) * 0.09
		xf.basis = xf.basis.scaled(Vector3(hgt * 0.75, hgt, hgt * 0.75))
		grass_mm.multimesh.set_instance_transform(i, xf)
		# Lush ground is greener and darker; dry ground is paler and yellower.
		var tone: float = fposmod(sc * 7.13, 1.0)
		var col := Color(0.26, 0.44, 0.17).lerp(Color(0.52, 0.50, 0.24), 1.0 - lush)
		col = col.lerp(Color(0.34, 0.52, 0.22), tone * 0.55)
		grass_mm.multimesh.set_instance_color(i, col)
	if n > 0 and Engine.get_process_frames() < 4:
		print("[rama] grass: %d tufts (%d tris), 1 draw call" % [n, n * 4])

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
	disk.radius = 0.55
	disk.height = 0.18
	disk.radial_segments = 8
	disk.rings = 3
	mm.mesh = disk
	mm.instance_count = 1
	# use_colors defaults instances to opaque WHITE. Park it invisibly until a
	# refresh assigns real values.
	mm.set_instance_transform(0, Transform3D(Basis().scaled(Vector3.ZERO), Vector3.ZERO))
	mm.set_instance_color(0, Color(1, 1, 1, 0))
	foam_mm = MultiMeshInstance3D.new()
	foam_mm.multimesh = mm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.distance_fade_mode = BaseMaterial3D.DISTANCE_FADE_PIXEL_DITHER
	mat.distance_fade_min_distance = 70.0
	mat.distance_fade_max_distance = 140.0
	foam_mm.material_override = mat
	foam_mm.name = "ShoreFoam"
	add_child(foam_mm)

func _refresh_foam() -> void:
	if foam_mm == null or player == null:
		return
	# Prefer thin shore (0.12–0.9 m) so Multimesh foam sits on the contact
	# line that the depth-shore shader already paints.
	var pts: PackedFloat32Array = terrain.shore_points(player.theta, player.z, 110.0, 160)
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
		# Flatter, wider discs along the waterline.
		var s: float = clampf(0.85 + (1.0 - clampf(dep / 1.2, 0.0, 1.0)) * 0.9, 0.7, 2.4)
		xf.basis = xf.basis.scaled(Vector3(s, 0.22, s))
		foam_mm.multimesh.set_instance_transform(i, xf)
		var edge: float = 1.0 - clampf(abs(dep - 0.35) / 0.55, 0.0, 1.0)
		var a: float = clampf(0.18 + edge * 0.55, 0.15, 0.72)
		foam_mm.multimesh.set_instance_color(i, Color(0.90, 0.95, 0.98, a))

func _build_splash() -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var ball := SphereMesh.new()
	ball.radius = 0.14
	ball.height = 0.28
	ball.radial_segments = 6
	ball.rings = 3
	mm.mesh = ball
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
	# Flatter mound — sphere stacks used to read as a tan debug blob.
	var heap := SphereMesh.new()
	heap.radius = 0.52
	heap.height = 0.55
	heap.radial_segments = 12
	heap.rings = 6
	mm.mesh = heap
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
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var body := CapsuleMesh.new()
	body.radius = 0.28
	body.height = 1.45
	body.radial_segments = 8
	body.rings = 2
	mm.mesh = body
	mm.instance_count = 0
	agent_mm = MultiMeshInstance3D.new()
	agent_mm.multimesh = mm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	agent_mm.material_override = mat
	agent_mm.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	agent_mm.name = "Colonists"
	add_child(agent_mm)
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

func refresh_agents() -> void:
	if agent_mm == null or terrain == null:
		return
	var buf: PackedFloat32Array = terrain.agents_lod()
	var n: int = int(buf.size() / 9.0)
	agent_mm.multimesh.instance_count = n
	if plot_mm:
		plot_mm.multimesh.instance_count = n
	for i in n:
		var th: float = buf[i * 9]
		var zz: float = buf[i * 9 + 1]
		var hunger: float = buf[i * 9 + 2]
		var fatigue: float = buf[i * 9 + 3]
		var mood: float = buf[i * 9 + 4]
		var pth: float = buf[i * 9 + 6]
		var pzz: float = buf[i * 9 + 7]
		var prad: float = maxf(buf[i * 9 + 8], 8.0)
		var gr: float = terrain.ground_radius(th, zz)
		var xf := frame_at(th, zz, gr - 0.95)
		agent_mm.multimesh.set_instance_transform(i, xf)
		# Warm work-clothes tint; fatigue darkens, mood warms.
		var col := Color(
			0.55 + mood * 0.25,
			0.38 + (1.0 - hunger) * 0.12,
			0.28 + (1.0 - fatigue) * 0.08
		)
		agent_mm.multimesh.set_instance_color(i, col)
		if plot_mm:
			var pgr: float = terrain.ground_radius(pth, pzz)
			var pxf := frame_at(pth, pzz, pgr - 0.03)
			# Keep full claim radius in sim; visuals stay a soft ground stain.
			pxf.basis = pxf.basis.scaled(Vector3(prad, 1.0, prad))
			plot_mm.multimesh.set_instance_transform(i, pxf)
			# A claim marker should be a tint on the ground, not a veil floating
			# over it. 0.55 alpha read as milky plastic.
			plot_mm.multimesh.set_instance_color(i, Color(0.30, 0.55, 0.28, 0.15))

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
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
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
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
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
	puff.radius = 0.35
	puff.height = 0.55
	puff.radial_segments = 6
	puff.rings = 3
	mm.mesh = puff
	mm.instance_count = 16
	# use_colors defaults every instance to opaque WHITE. Any MultiMesh that
	# sets instance_count at build but assigns colours only on refresh renders
	# as white boxes until that refresh runs.
	for i in mm.instance_count:
		mm.set_instance_transform(i, Transform3D(Basis().scaled(Vector3.ZERO), Vector3.ZERO))
		mm.set_instance_color(i, Color(0.90, 0.94, 0.97, 0.0))
	steam_mm = MultiMeshInstance3D.new()
	steam_mm.multimesh = mm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	steam_mm.material_override = mat
	steam_mm.visible = false
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
	# The sun is a strip running down the axis. Day length is a lighting
	# schedule someone set, not an orbit. Keep emission modest — the strip
	# should read as a warm line, not a nuclear bloom that whites out the sky.
	var half_l: float = float(P["length"]) * 0.49
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var seg := 16
	var rad := 5.5
	for i in seg:
		var a0 := TAU * float(i) / seg
		var a1 := TAU * float(i + 1) / seg
		var x0 := rad * cos(a0)
		var y0 := rad * sin(a0)
		var x1 := rad * cos(a1)
		var y1 := rad * sin(a1)
		# Warm/cool gradient along length: centre warm, ends cooler (Coriolis
		# weather means the ends are cloudier). Vertex colour gradient.
		var warm := Color(1.0, 0.96, 0.88)
		var cool := Color(0.88, 0.92, 0.98)
		for v_pair in [[Vector3(x0, y0, -half_l), cool], [Vector3(x0, y0, half_l), cool],
						[Vector3(x1, y1, half_l), cool], [Vector3(x0, y0, -half_l), cool],
						[Vector3(x1, y1, half_l), cool], [Vector3(x1, y1, -half_l), cool]]:
			var vp: Vector3 = v_pair[0]
			# Centre is warm, ends are cool.
			var zt: float = 1.0 - clampf(absf(vp.z) / half_l, 0.0, 1.0)
			st.set_color(warm.lerp(cool, 1.0 - zt * zt))
			st.set_normal(Vector3(vp.x, vp.y, 0).normalized())
			st.add_vertex(vp)
	var mi := MeshInstance3D.new()
	mi.mesh = st.commit()
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	mat.emission_enabled = true
	mat.emission = Color(1.0, 0.92, 0.78)
	mat.emission_energy_multiplier = 1.6
	mi.material_override = mat
	axis_mat = mat
	mi.name = "AxisLight"
	add_child(mi)

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
	# Motes near the player. Cheap, and they do more for the sense of air
	# than anything else this scene can afford. More motes at smaller scale
	# sells depth-of-field better.
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	var qm := QuadMesh.new()
	qm.size = Vector2(0.06, 0.06)
	mm.mesh = qm
	mm.instance_count = 480
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	for i in mm.instance_count:
		var t := Transform3D()
		t.origin = Vector3(rng.randf_range(-26, 26), rng.randf_range(-8, 18),
				rng.randf_range(-26, 26))
		mm.set_instance_transform(i, t)
		mm.set_instance_color(i, Color(1, 0.96, 0.88, rng.randf_range(0.08, 0.30)))
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
	var start: float = terrain.ground_radius(theta, z) - 40.0
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
	for dt in range(-CHUNK_RADIUS, CHUNK_RADIUS + 1):
		for dz in range(-CHUNK_RADIUS, CHUNK_RADIUS + 1):
			var k := _chunk_key(ti0 + dt, zi0 + dz)
			if loaded.has(k) or pending.has(k):
				continue
			var dist: int = absi(dt) + absi(dz)
			# Lower score = sooner. Facing the look direction gets a bonus.
			var aim: float = -(float(dt) * face_t + float(dz) * face_z) * 0.55
			pending.append([posmod(ti0 + dt, n_around), zi0 + dz, dist, aim])
	pending.sort_custom(func(a, b):
		return (float(a[2]) + float(a[3])) < (float(b[2]) + float(b[3]))
	)
	_unload_far(ti0, zi0)

func _pump_chunks(budget: int) -> void:
	var n := 0
	while pending.size() > 0 and n < budget:
		var e = pending.pop_front()
		var k := _chunk_key(e[0], e[1])
		if loaded.has(k):
			continue
		_build_chunk(e[0], e[1])
		n += 1

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
	else:
		loaded[k] = null
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
	# Morphology tiers: near = stem+canopy, mid = tapered prism, far = billboard card.
	plant_mm = _make_plant_layer("PlantsNear", _make_tree_mesh(1.0), 95.0, 190.0)
	plant_mm_mid = _make_plant_layer("PlantsMid", _make_tree_mesh(0.72), 170.0, 300.0)
	plant_mm_far = _make_plant_layer("PlantsFar", _make_billboard_mesh(), 280.0, 450.0)
	_refresh_plants()

func _make_tree_mesh(detail: float) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	# Trunk
	var trunk_h := 1.0
	var trunk_r := 0.12
	_add_prism(st, Vector3(0, trunk_h * 0.5, 0), Vector3(trunk_r, trunk_h, trunk_r),
			Color(0.28, 0.18, 0.10))
	# Crossed canopy cards
	var canopy := Color(0.14, 0.36, 0.16)
	var ch := 0.85 * detail
	var cw := 0.95 * detail
	_add_quad(st, Vector3(0, trunk_h + ch * 0.35, 0), Vector3(cw, ch, 0.04), canopy)
	_add_quad(st, Vector3(0, trunk_h + ch * 0.35, 0), Vector3(0.04, ch, cw), canopy)
	if detail > 0.85:
		_add_prism(st, Vector3(0, trunk_h + ch * 0.55, 0), Vector3(cw * 0.45, ch * 0.5, cw * 0.45),
				Color(0.12, 0.32, 0.14))
	st.generate_normals()
	return st.commit()

func _make_billboard_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	_add_quad(st, Vector3(0, 0.7, 0), Vector3(1.4, 1.4, 0.05), Color(0.16, 0.34, 0.14))
	st.generate_normals()
	return st.commit()

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

func _add_quad(st: SurfaceTool, center: Vector3, size: Vector3, col: Color) -> void:
	var hx := size.x * 0.5
	var hy := size.y * 0.5
	var hz := size.z * 0.5
	var p := [
		center + Vector3(-hx, -hy, -hz),
		center + Vector3(hx, -hy, -hz),
		center + Vector3(hx, hy, hz),
		center + Vector3(-hx, hy, hz),
	]
	for idx in [0, 1, 2, 0, 2, 3]:
		st.set_color(col)
		st.add_vertex(p[idx])

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
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	mat.distance_fade_mode = BaseMaterial3D.DISTANCE_FADE_PIXEL_DITHER
	mat.distance_fade_min_distance = fade_min
	mat.distance_fade_max_distance = fade_max
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mi.material_override = mat
	mi.name = layer_name
	add_child(mi)
	return mi

const PLANT_STRIDE := 7  # theta,z,stem,leaf,alive,lod,biome_id
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
]

func _refresh_plants() -> void:
	# Immediate full refresh (startup / load). Streaming path is _tick_plant_fill.
	if plant_mm == null or player == null:
		return
	last_plants_alive = int(last_sim.get("plants", -1))
	plant_anchor = Vector2(player.theta, player.z)
	plant_data = terrain.plants_lod(player.theta, player.z, 900)
	var buckets: Array = [[], [], []]
	var n: int = int(plant_data.size() / float(PLANT_STRIDE))
	for i in n:
		var lod: int = clampi(int(plant_data[i * PLANT_STRIDE + 5]), 0, 2)
		buckets[lod].append(i)
	_fill_plant_bucket(plant_mm, plant_data, buckets[0], 1.0)
	_fill_plant_bucket(plant_mm_mid, plant_data, buckets[1], 1.25)
	_fill_plant_bucket(plant_mm_far, plant_data, buckets[2], 1.7)

func _fill_plant_bucket(mi: MultiMeshInstance3D, data: PackedFloat32Array, indices: Array, scale_boost: float) -> void:
	if mi == null:
		return
	var n: int = indices.size()
	if n < 1:
		mi.multimesh.instance_count = 0
		return
	mi.multimesh.instance_count = n
	for j in n:
		var i: int = indices[j]
		var base: int = i * PLANT_STRIDE
		var th: float = data[base]
		var zz: float = data[base + 1]
		var stem: float = data[base + 2]
		var leaf: float = data[base + 3]
		var bid: int = clampi(int(data[base + 6]), 0, BIOME_PLANT_COL.size() - 1)
		var gr: float = terrain.ground_radius(th, zz)
		var h: float = clampf(0.55 + stem * 3.8 + leaf * 1.2, 0.7, 7.5) * scale_boost
		var xf := frame_at(th, zz, gr)
		var w: float = (0.55 + leaf * 1.05) * scale_boost
		xf.basis = xf.basis.scaled(Vector3(w, h, w))
		mi.multimesh.set_instance_transform(j, xf)
		var base_col: Color = BIOME_PLANT_COL[bid]
		var tint := Color(
			base_col.r + leaf * 0.10 - stem * 0.02,
			base_col.g + leaf * 0.14,
			base_col.b + stem * 0.03)
		mi.multimesh.set_instance_color(j, tint)

# ------------------------------------------------------------------ player --

func _build_player() -> void:
	player = load("res://scripts/player.gd").new()
	player.world = self
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

	var ui := CanvasLayer.new()
	ui.layer = 2
	hud = Label.new()
	hud.position = Vector2(22, 16)
	hud.add_theme_font_size_override("font_size", 13)
	hud.add_theme_color_override("font_color", Color(0.86, 0.92, 0.95))
	hud.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.85))
	hud.add_theme_constant_override("outline_size", 5)
	ui.add_child(hud)
	add_child(ui)

func _process(_dt: float) -> void:
	# Dig bursts: clear remesh backlog first (2/frame). Idle: keep streaming.
	var remesh_budget := 2 if dig_held or remesh_queue.size() > 0 else 1
	_pump_chunks(1 if remesh_queue.is_empty() else 0)
	_drain_remesh(remesh_budget)
	_tick_daylight(_dt)
	_tick_flow_refresh(_dt)
	_tick_pool_refresh(_dt)
	_tick_stockpile_refresh(_dt)
	_tick_wet_refresh(_dt)
	_drain_wet_colors(1)
	_tick_biosphere(_dt)
	_tick_deferred_visuals()
	_tick_plant_fill()
	_tick_catchment(_dt)
	_tick_splash(_dt)
	_tick_catchment_pulse(_dt)
	_tick_autosave(_dt)
	_tick_water_audio()
	_tick_panels()
	refresh_grass()
	if hud and player:
		var e: float = terrain.elevation(player.theta, player.z)
		var fx: float = terrain.water_flux(player.theta, player.z)
		match player.view:
			1:
				hud.text = _hud_drum()
			2:
				hud.text = _hud_map(e, fx)
			_:
				hud.text = _hud_colonist(e, fx)

func _tick_water_audio() -> void:
	if audio == null or player == null or terrain == null:
		return
	var dep: float = terrain.water_depth_at(player.theta, player.z)
	var fx: float = terrain.water_flux(player.theta, player.z)
	audio.water_ambience(dep, fx)

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
	print("[rama] autosave slot %d" % slot)

func _tick_biosphere(dt: float) -> void:
	sim_accum += dt * SIM_STEP_DAYS
	if sim_accum < 0.06:
		return
	var step: float = minf(sim_accum, 0.12)
	sim_accum = 0.0
	if player != null:
		terrain.set_player_pos(player.theta, player.z)
	last_sim = terrain.sim_tick(step)
	refresh_agents()
	refresh_stockpiles()
	refresh_carcasses()
	# Defer Godot remesh/plant work off the sim frame so they never stack.
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
	if not visuals_pending:
		return
	match visual_phase:
		0:
			_maybe_queue_plants()
		1:
			# Foam/pools already debounce via schedule_pool_refresh — don't rebuild
			# shore Multimesh on every quiet sim beat.
			if float(last_sim.get("sediment", 0.0)) > 2.0:
				_refresh_foam()
				_refresh_biome_map()
				# Don't sync-remesh a ring of chunks — queue one-at-a-time.
				_queue_remesh_near_player()
			else:
				wet_refresh_in = 2.5
		2:
			_refresh_soil_overlay()
			visuals_pending = false
			return
	visual_phase += 1

func _maybe_queue_plants() -> void:
	if player == null or plant_mm == null:
		return
	if plant_refresh_due:
		return
	var alive: int = int(last_sim.get("plants", last_plants_alive))
	var anchor := Vector2(player.theta, player.z)
	var moved: float = absf(wrapf(anchor.x - plant_anchor.x, -PI, PI)) * float(P["radius"])
	moved += absf(anchor.y - plant_anchor.y)
	# Skip full plant rebuild if nothing meaningful changed.
	if alive == last_plants_alive and moved < 12.0 and plant_data.size() > 0:
		return
	last_plants_alive = alive
	plant_anchor = anchor
	plant_bucket_idx = 0
	plant_fill_j = 0
	plant_indices = []
	plant_refresh_due = true

func _tick_plant_fill() -> void:
	if not plant_refresh_due:
		return
	# Don't compete with chunk remesh — wait until the backlog drains.
	if remesh_queue.size() > 0:
		return
	if player == null or plant_mm == null:
		plant_refresh_due = false
		return
	# First frame: pull LOD sample from Rust, then stream transforms in budgets.
	if plant_indices.is_empty() and plant_fill_j == 0 and plant_bucket_idx == 0:
		plant_data = terrain.plants_lod(player.theta, player.z, 900)
		var buckets: Array = [[], [], []]
		var n: int = int(plant_data.size() / float(PLANT_STRIDE))
		for i in n:
			var lod: int = clampi(int(plant_data[i * PLANT_STRIDE + 5]), 0, 2)
			buckets[lod].append(i)
		plant_indices = buckets
		_begin_plant_bucket(0)
		return
	_fill_plant_budget()

func _begin_plant_bucket(b: int) -> void:
	plant_bucket_idx = b
	plant_fill_j = 0
	var layers: Array = [plant_mm, plant_mm_mid, plant_mm_far]
	var boosts: Array = [1.0, 1.25, 1.7]
	if b >= layers.size():
		plant_refresh_due = false
		plant_indices = []
		return
	var mi: MultiMeshInstance3D = layers[b]
	var indices: Array = plant_indices[b] if b < plant_indices.size() else []
	if mi == null:
		_begin_plant_bucket(b + 1)
		return
	if indices.is_empty():
		mi.multimesh.instance_count = 0
		_begin_plant_bucket(b + 1)
		return
	mi.multimesh.instance_count = indices.size()
	# stash boost on the multimesh via meta for the fill loop
	mi.set_meta("plant_boost", boosts[b])

func _fill_plant_budget() -> void:
	var layers: Array = [plant_mm, plant_mm_mid, plant_mm_far]
	if plant_bucket_idx >= layers.size():
		plant_refresh_due = false
		plant_indices = []
		return
	var mi: MultiMeshInstance3D = layers[plant_bucket_idx]
	var indices: Array = plant_indices[plant_bucket_idx]
	if mi == null or indices.is_empty():
		_begin_plant_bucket(plant_bucket_idx + 1)
		return
	var boost: float = float(mi.get_meta("plant_boost", 1.0))
	var n: int = indices.size()
	var done := 0
	while plant_fill_j < n and done < PLANT_FILL_BUDGET:
		var i: int = indices[plant_fill_j]
		var base: int = i * PLANT_STRIDE
		var th: float = plant_data[base]
		var zz: float = plant_data[base + 1]
		var stem: float = plant_data[base + 2]
		var leaf: float = plant_data[base + 3]
		var bid: int = clampi(int(plant_data[base + 6]), 0, BIOME_PLANT_COL.size() - 1)
		# ground_radius only — ground_below per plant was a multi-ms hitch.
		var gr: float = terrain.ground_radius(th, zz)
		var h: float = clampf(0.55 + stem * 3.8 + leaf * 1.2, 0.7, 7.5) * boost
		var xf := frame_at(th, zz, gr)
		var w: float = (0.55 + leaf * 1.05) * boost
		xf.basis = xf.basis.scaled(Vector3(w, h, w))
		mi.multimesh.set_instance_transform(plant_fill_j, xf)
		var base_col: Color = BIOME_PLANT_COL[bid]
		mi.multimesh.set_instance_color(plant_fill_j, Color(
			base_col.r + leaf * 0.10 - stem * 0.02,
			base_col.g + leaf * 0.14,
			base_col.b + stem * 0.03))
		plant_fill_j += 1
		done += 1
	if plant_fill_j >= n:
		_begin_plant_bucket(plant_bucket_idx + 1)

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
	var props := {
		"modules": [],
		"waypoint": [waypoint.x, waypoint.y],
		"has_waypoint": has_waypoint,
		"home": [home.x, home.y],
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
	print("[rama] saved digs + soil + %d modules → user://" % props["modules"].size())

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

func _drain_remesh(budget: int = 1) -> void:
	var n := 0
	while remesh_queue.size() > 0 and n < budget:
		var k: String = remesh_queue.pop_front()
		var parts: PackedStringArray = k.split("_")
		if parts.size() < 2:
			continue
		_build_chunk(int(parts[0]), int(parts[1]))
		n += 1

func _clock_str() -> String:
	var phase: float = fposmod(clock / DAY_LENGTH, 1.0)
	var mins: int = int(phase * 1440.0)
	return "%02d:%02d" % [int(mins / 60.0), mins % 60]

func _hud_colonist(e: float, fx: float) -> String:
	var aim_mat := ""
	if player.last_aim.get("hit", false):
		var pr: Dictionary = terrain.probe(player.last_aim["point"])
		var grade: float = float(pr.get("ore_grade", 0.0))
		aim_mat = "LOOKING   %s  (hardness %.1f · %.0f kg/m³" % [
			pr.get("material", pr.get("kind", "?")), pr.get("hardness", 1.0),
			pr.get("bulk_kg_m3", 0.0)]
		if grade > 0.02:
			aim_mat += " · ore %.0f%%" % (grade * 100.0)
		aim_mat += ")"
		if player.last_aim.has("yield_kg"):
			aim_mat += "\nLAST DIG  %.1f kg  (kept %.0f%%)" % [
				float(player.last_aim["yield_kg"]),
				float(player.last_aim.get("yield_accepted", 1.0)) * 100.0]
	else:
		aim_mat = "LOOKING   —"
	var inv: Dictionary = terrain.inventory()
	var stacks: Array = inv.get("stacks", [])
	var pack_line := "PACK      %.1f / %.0f kg · %.0f / %.0f L · enc %.0f%%" % [
		float(inv.get("mass_kg", 0.0)), float(inv.get("max_mass_kg", 45.0)),
		float(inv.get("loose_m3", 0.0)) * 1000.0, float(inv.get("max_volume_m3", 0.04)) * 1000.0,
		float(inv.get("encumbrance", 1.0)) * 100.0]
	if stacks.size() > 0:
		var bits: PackedStringArray = PackedStringArray()
		for s in stacks:
			var bit := "%s %.1fkg" % [s.get("name", "?"), float(s.get("mass_kg", 0.0))]
			if float(s.get("grade", 0.0)) > 0.02:
				bit += " @%.0f%%" % (float(s.get("grade", 0.0)) * 100.0)
			bits.append(bit)
		pack_line += "\n          " + ", ".join(bits)
	var atmo: Dictionary = terrain.atmosphere()
	var atmo_line := "AIR       O₂ %.0f kg (%.1f%%) · CO₂ %.0f ppm · scrub %.1f MW · GH %d" % [
		float(atmo.get("o2_kg", 0.0)), float(atmo.get("o2_frac", 0.0)) * 100.0,
		float(atmo.get("co2_ppm", 0.0)), float(atmo.get("scrub_mw", 0.0)),
		int(terrain.greenhouse_count())]
	var npp: Dictionary = terrain.npp_at(player.theta, player.z)
	var npp_line := "NPP       %.0f g/m²/yr · producer %.2f · fear %.2f · max fauna ~%.0f kg" % [
		float(npp.get("npp", 0.0)), float(npp.get("producer", 0.0)),
		float(npp.get("fear", 0.0)), float(last_sim.get("max_fauna_kg", 0.0))]
	npp_line += "\n          carcasses %d · kills %d · mean fear %.2f" % [
		int(last_sim.get("carcasses", 0)), int(last_sim.get("kills", 0)),
		float(last_sim.get("mean_fear", 0.0))]
	var rec: Dictionary = terrain.recipe_at(player.recipe_idx)
	var near_st := "ok" if (not bool(rec.get("needs_station", false)) or bool(rec.get("station_near", true))) else "NEED"
	var craft_line := "CRAFT     [%d/%d] %s @%s [%s]  scale %.0f%%" % [
		player.recipe_idx + 1, int(terrain.recipe_count()),
		str(rec.get("id", "?")), str(rec.get("station", "?")), near_st,
		float(rec.get("max_scale", 0.0)) * 100.0]
	var led: Dictionary = terrain.materials_ledger()
	var ledger_line := "LEDGER    pack %.1f kg · heaps %.1f kg · satiety %.0f%% · stations %d" % [
		float(led.get("pack_mass_kg", 0.0)), float(led.get("heap_mass_kg", 0.0)),
		float(led.get("satiety", 0.0)) * 100.0, int(led.get("stations", 0))]
	var agents_n: int = int(last_sim.get("agents", 0))
	var agent_line := "COLONISTS %d" % agents_n
	if agents_n > 0:
		var nearest_i := 0
		var nearest_d := 1.0e9
		var abuf: PackedFloat32Array = terrain.agents_lod()
		var an: int = int(abuf.size() / 9.0)
		for i in an:
			var dth: float = absf(wrapf(abuf[i * 9] - player.theta, -PI, PI)) * float(P["radius"])
			var dz: float = abuf[i * 9 + 1] - player.z
			var dd: float = dth * dth + dz * dz
			if dd < nearest_d:
				nearest_d = dd
				nearest_i = i
		var nm: String = str(terrain.agent_name(nearest_i))
		var said: String = str(terrain.agent_line(nearest_i))
		var cite: Dictionary = terrain.affinity_cite(nearest_i)
		var aff: float = float(cite.get("score", terrain.affinity_with(nearest_i)))
		agent_line += " · nearest %s (%.0f m · affinity %.1f)" % [nm, sqrt(nearest_d), aff]
		if bool(cite.get("has_cite", false)):
			agent_line += "\n          cite d%.0f %s — %s" % [
				float(cite.get("day", 0.0)), str(cite.get("kind", "?")), str(cite.get("label", ""))]
		if said != "":
			agent_line += "\n          " + said
	var chron: PackedStringArray = terrain.chronicle_latest(1)
	var chron_line := "CHRONICLE —"
	if chron.size() > 0:
		chron_line = "CHRONICLE " + chron[0]
	var soil: Dictionary = terrain.soil_at(player.theta, player.z)
	var wx: Dictionary = terrain.weather_at(player.theta, player.z)
	var bio: Dictionary = terrain.biome_at(player.theta, player.z)
	var power: Dictionary = terrain.power_budget()
	var wdep: float = terrain.water_depth_at(player.theta, player.z)
	return ("KEPLER DRUM  %s   daylight %d percent\n"
		+ "  radius %.0f m · length %.0f m · circumference %.0f m\n"
		+ "  gravity %.2f m/s² · spin period %.1f s · seed 0x%X\n\n"
		+ "POSITION  θ %.3f · z %+.0f m · elev %.1f m · drainage %.2f · water %.2f m\n"
		+ "SOIL      N %.2f  P %.2f  K %.2f  moist %.2f  pH %.1f\n"
		+ "WEATHER   rain %.2f  temp %.0f  humid %.2f  band %d  · %s\n"
		+ "POWER     %.1f / %.1f MW  (headroom %.1f)  · condensers %d\n"
		+ "LIFE      %d plants · lakes %d · pools %d (%.1f m) · stock %.0f t · flow %s\n"
		+ "%s\n%s\n%s\n%s\n%s\n%s\n%s\n\n"
		+ "%s\n%s\n\n"
		+ "WASD move · Shift run · Space jump · scroll zoom\n"
		+ "LEFT CLICK / F  dig      RIGHT CLICK / G  install %s\n"
		+ "H harvest · X drop · K craft · J eat · Y station · M amend · U scrub\n"
		+ "Q/E or side-scroll brush %.1f m (%s)\n"
		+ "C brush · B mark · V scoop/pour · N soil · +/- zoom · 1-4 module\n"
		+ "F5 save · F9 load · TAB view · Esc menu"
	) % [_clock_str(), int(day * 100.0),
		P["radius"], P["length"], P["circumference"],
		P["gravity"], P["spin_period"], P["seed"],
		player.theta, player.z, e, fx, wdep,
		soil.get("n", 0.0), soil.get("p", 0.0), soil.get("k", 0.0),
		soil.get("moisture", 0.0), soil.get("ph", 7.0),
		wx.get("rain", 0.0), wx.get("temp", 0.0), wx.get("humidity", 0.0),
		int(wx.get("band", 0)), str(bio.get("name", "?")),
		float(power.get("used", 0.0)), float(power.get("budget", 0.0)),
		float(power.get("headroom", 0.0)), int(power.get("condensers", 0)),
		terrain.plant_count(), terrain.lake_count(),
		int(last_sim.get("pools", 0)), float(last_sim.get("pool_depth", 0.0)),
		float(last_sim.get("water_stock", 0.0)) / 1000.0,
		("rerouting…" if terrain.flow_dirty() else "live"),
		pack_line, atmo_line, npp_line, craft_line, ledger_line, agent_line, chron_line,
		home_bearing(), aim_mat,
		MODULES[player.module]["name"], player.brush,
		"levelling" if player.level_brush else "sphere"]

func _pct(frac: float) -> String:
	return "%5.1f percent" % (frac * 100.0)

func _bar(frac: float) -> String:
	var n: int = int(round(frac * 24.0))
	return "█".repeat(n) + "·".repeat(24 - n)

func _hud_drum() -> String:
	return ("KEPLER DRUM — habitat survey        %s\n"
		+ "══════════════════════════════════════════════\n"
		+ "  surface area      %.2f km²\n"
		+ "  arable            %.2f km²   (%d percent of surface)\n"
		+ "  relief            %.0f – %.0f m   mean %.0f m\n"
		+ "  waterline         %.0f m\n\n"
		+ "BIOME COVER\n"
		+ "  open water    %s %s\n"
		+ "  alluvial flat %s %s\n"
		+ "  grassland     %s %s\n"
		+ "  upland        %s %s\n"
		+ "  bare rock     %s %s\n\n"
		+ "  Relief is engineered then weathered: structural ribs\n"
		+ "  and shaped high ground. Drainage is LIVE — dig a trench\n"
		+ "  and watch the rivers move. Materials have hardness.\n\n"
		+ "TAB switch view"
	) % [_clock_str(), census["surface_area_km2"], census["arable_km2"],
		int((float(census["alluvial"]) + float(census["grass"])) * 100.0),
		census["min_elevation"], census["max_elevation"], census["mean_elevation"],
		P["water_level"],
		_bar(census["water"]), _pct(census["water"]),
		_bar(census["alluvial"]), _pct(census["alluvial"]),
		_bar(census["grass"]), _pct(census["grass"]),
		_bar(census["upland"]), _pct(census["upland"]),
		_bar(census["rock"]), _pct(census["rock"])]

func _hud_map(e: float, fx: float) -> String:
	var arc_m: float = player.theta * float(P["radius"])
	return ("LOCAL PLAN — 190 m across        %s\n"
		+ "══════════════════════════════════════\n"
		+ "  along drum   z %+.0f m\n"
		+ "  around drum  %.0f m of %.0f m\n"
		+ "  elevation    %.1f m above hull floor\n"
		+ "  drainage     %.2f   %s\n"
		+ "  modules      %d installed\n"
		+ "  excavation   %d strokes\n\n"
		+ "  Dig a trench across a slope — drainage is live.\n"
		+ "  North is +z, along the axis. East wraps the drum.\n\n"
		+ "TAB switch view"
	) % [_clock_str(), player.z, fposmod(arc_m, float(P["circumference"])), P["circumference"],
		e, fx,
		("channel" if fx > 0.55 else ("alluvial — good ground" if fx > 0.40 else "dry slope")),
		modules.size(), terrain.edit_count()]

## The overview views are instruments, not photographs: no lens blur, no dust,
## no cloud deck between you and the data.
func apply_view(v: int) -> void:
	if post_layer: post_layer.visible = (v == 0)
	if dust: dust.visible = (v == 0)
	if cloud_node: cloud_node.visible = (v == 0)
	# The survey view is not looking through 2.5 km of air at a photograph;
	# it is reading the habitat. Pull the haze back so the land is legible.
	RenderingServer.global_shader_parameter_set("rama_haze",
		1.0 if v == 0 else (0.45 if v == 1 else 0.10))

## Daylight is a schedule someone set, not an orbit. Dawn and dusk are events
## that choreograph fog, ambient, mist and far-side glow (1191–1195).
func _tick_daylight(dt: float) -> void:
	clock += dt
	var phase: float = fposmod(clock / DAY_LENGTH, 1.0)
	# Long day, short dusk, short night: a colony optimises for growing hours.
	day = clamp(smoothstep(0.02, 0.14, phase) - smoothstep(0.70, 0.90, phase), 0.0, 1.0)
	day = 0.06 + 0.94 * day
	RenderingServer.global_shader_parameter_set("rama_day", day)

	# Band labels for one-shot cues.
	var band := "day"
	if phase < 0.12:
		band = "dawn"
	elif phase > 0.88:
		band = "night"
	elif phase > 0.68:
		band = "dusk"
	if band != last_day_band:
		if band == "dawn":
			steam_life = 4.5
			print("[rama] dawn — mist lifting")
		elif band == "dusk":
			print("[rama] dusk — far side waking")
		last_day_band = band

	var env: Environment = world_env.environment if world_env else null
	if env:
		# Valley mist overnight, burns off in bands (1193).
		var mist := 0.0
		if phase < 0.18:
			mist = smoothstep(0.0, 0.08, phase) * (1.0 - smoothstep(0.12, 0.22, phase))
		elif phase > 0.92:
			mist = smoothstep(0.92, 1.0, phase)
		mist_phase = mist
		var fog_d := 0.00055 + mist * 0.0018 + (1.0 - day) * 0.00035
		env.fog_density = fog_d
		# Cool blue night → warm dawn → clear day → amber dusk (1191 / 1192).
		var air_night := Color(0.07, 0.10, 0.18)
		var air_dawn := Color(0.42, 0.48, 0.55)
		var air_day := Color(0.16, 0.22, 0.28)
		var air_dusk := Color(0.28, 0.18, 0.14)
		var air: Color
		if phase < 0.14:
			air = air_night.lerp(air_dawn, smoothstep(0.02, 0.12, phase))
		elif phase < 0.22:
			air = air_dawn.lerp(air_day, smoothstep(0.14, 0.22, phase))
		elif phase < 0.70:
			air = air_day
		elif phase < 0.88:
			air = air_day.lerp(air_dusk, smoothstep(0.70, 0.88, phase))
		else:
			air = air_dusk.lerp(air_night, smoothstep(0.88, 1.0, phase))
		env.background_color = air
		env.fog_light_color = air
		env.ambient_light_color = air.lightened(0.12)
		env.ambient_light_energy = 0.22 + 0.38 * day + mist * 0.08
		env.fog_light_energy = 0.85 + 0.35 * day
		# Far-side settlement glow as dusk deepens (1192).
		env.glow_intensity = 0.22 + (1.0 - day) * 0.45
		env.glow_bloom = 0.04 + (1.0 - day) * 0.12
		env.tonemap_exposure = 0.78 + 0.18 * day

	if axis_mat:
		var warm := Color(1.0, 0.94, 0.80).lerp(Color(1.0, 0.55, 0.32), 1.0 - day)
		if band == "dusk":
			warm = warm.lerp(Color(1.0, 0.72, 0.45), 0.55)
		axis_mat.emission = warm
		axis_mat.emission_energy_multiplier = 0.35 + 1.5 * day + (0.8 if band == "dusk" else 0.0)
		# Slow emission pulse — real fusion-lit strips have regulation ripple.
		# The eye reads a perfectly constant light as artificial in a bad way.
		var pulse: float = 1.0 + sin(clock * TAU / 120.0) * 0.05
		axis_mat.emission_energy_multiplier *= pulse
		axis_mat.albedo_color = warm
	if dust and player:
		dust.global_position = player.feet_pos()
		# Gentle drift along the drum axis — air moves, and motes that are
		# perfectly still read as stuck.
		var drift := Vector3(0, 0, sin(clock * 0.23) * 0.3) + Vector3(sin(clock * 0.41) * 0.15, cos(clock * 0.37) * 0.1, 0)
		dust.global_position += drift

	# Steam off wet ground when the strip comes up (1195).
	if steam_life > 0.0 and steam_mm and player:
		steam_life -= dt
		steam_mm.visible = true
		var base: Vector3 = player.feet_pos()
		var up := Vector3(-base.x, -base.y, 0.0).normalized()
		var wdep: float = terrain.water_depth_at(player.theta, player.z)
		var a := 0.12 + 0.25 * clampf(steam_life / 4.5, 0.0, 1.0) * (0.4 + wdep)
		for i in 16:
			var ang: float = float(i) / 16.0 * TAU + clock * 0.4
			var lateral := Vector3(cos(ang), sin(ang), sin(ang * 1.7) * 0.3)
			lateral = (lateral - up * lateral.dot(up)).normalized()
			var loft: float = 0.4 + float(i % 5) * 0.35 + (4.5 - steam_life) * 0.15
			var xf := Transform3D(Basis.IDENTITY, base + lateral * (1.2 + i * 0.35) + up * loft)
			var s: float = 0.6 + (i % 3) * 0.25
			xf.basis = xf.basis.scaled(Vector3(s, s * 1.3, s))
			steam_mm.multimesh.set_instance_transform(i, xf)
			steam_mm.multimesh.set_instance_color(i, Color(0.85, 0.88, 0.92, a))
	elif steam_mm:
		steam_mm.visible = false

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
	var img := Image.create_from_data(128, 128, false, Image.FORMAT_RGB8,
			terrain.soil_map(128, 128, soil_mode))
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
		soil_chip_label.text = "soil · %s  (V)" % names[soil_mode]

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
	if hud_panels: hud_panels.visible = (player.view == 0)

# ------------------------------------------------------------- screenshots --

func _take_shots() -> void:
	await RenderingServer.frame_post_draw
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
	]
	for s in shots:
		player.view = int(s.get("view", 0))
		apply_view(player.view)
		player.cam_ready = false
		player.pitch = s["pitch"]
		player.yaw = s["yaw"]
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
		img.save_png("/Users/powerox/ramen/shots/%s.png" % s["name"])
		print("[rama] shot ", s["name"])
	get_tree().quit()
