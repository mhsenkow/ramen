extends Node3D
const RamaControls = preload("res://scripts/controls.gd")
## Cylinder-aware third-person-ish controller.
##
## There is no world "up" here. Up is toward the axis and it rotates under you
## as you walk around the drum. Everything below works in (theta, z, r) and only
## converts to world space at the end. (REQUIREMENTS.md A2)

var world                    # World node
var theta := 0.0
var z := 0.0
var r := 0.0                 # feet radius; larger = further out = "lower"
var vr := 0.0                # radial velocity, positive = falling outward
var yaw := 0.0
var pitch := 0.0
var eye := 1.72
var on_ground := false
var cam: Camera3D
var body: Node3D
var rig := {}
var gait := 0.0
var cam_dist := 4.6
var cam_target := Vector3.ZERO
var cam_ready := false
var view := 0            # 0 colonist · 1 drum · 2 map
var drum_spin := 0.0
var cam_dist_target := 4.6
var last_aim := {}
var can_dig := true
var ghost: Node3D
var level_brush := false
var pad_sens := 2.6
var wake_t := 0.0
var woke := false
var fade: ColorRect
var tilt_on := true
var bottle_cd := 0.0

const WALK := 6.2
const RUN := 11.5
const JUMP := 6.4
var mouse_sens := 0.0026
const ZOOM_MIN := 0.0
const ZOOM_MAX := 7.5

var module := 0
var brush := 2.6
var recipe_idx := 0
var highlight: MeshInstance3D
var dig_cd := 0.0
var throws: Array = []
var trail: MeshInstance3D
var trail_mesh: ImmediateMesh

func _ready() -> void:
	cam = Camera3D.new()
	cam.fov = RamaControls.fov
	cam.near = 0.08
	cam.far = 5200.0
	add_child(cam)
	cam.current = true

	# An articulated colonist, so scale reads and motion has weight.
	body = Node3D.new()
	world.add_child.call_deferred(body)
	var skin := Color(0.76, 0.58, 0.44)
	var jacket := Color(0.32, 0.38, 0.44)
	var trousers := Color(0.22, 0.26, 0.32)
	var boot := Color(0.20, 0.18, 0.17)
	rig["hip"] = _part(body, Vector3(0.42, 0.22, 0.26), trousers, Vector3(0, 0.92, 0))
	rig["torso"] = _part(rig["hip"], Vector3(0.46, 0.56, 0.28), jacket, Vector3(0, 0.38, 0))
	rig["head"] = _part(rig["torso"], Vector3(0.26, 0.28, 0.26), skin, Vector3(0, 0.42, 0), true)
	for side in [-1.0, 1.0]:
		var key: String = "arm%d" % int(side)
		var pivot := Node3D.new()
		pivot.position = Vector3(side * 0.31, 0.22, 0)
		rig["torso"].add_child(pivot)
		rig[key] = pivot
		_part(pivot, Vector3(0.15, 0.52, 0.16), skin, Vector3(0, -0.26, 0), true)
		var lkey: String = "leg%d" % int(side)
		var lp := Node3D.new()
		lp.position = Vector3(side * 0.13, -0.10, 0)
		rig["hip"].add_child(lp)
		rig[lkey] = lp
		_part(lp, Vector3(0.18, 0.66, 0.20), trousers, Vector3(0, -0.35, 0))
		_part(lp, Vector3(0.20, 0.14, 0.30), boot, Vector3(0, -0.72, 0.04))

	trail_mesh = ImmediateMesh.new()
	trail = MeshInstance3D.new()
	trail.mesh = trail_mesh
	var tm := StandardMaterial3D.new()
	tm.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	tm.vertex_color_use_as_albedo = true
	tm.emission_enabled = true
	tm.emission_energy_multiplier = 2.0
	trail.material_override = tm
	world.add_child.call_deferred(trail)

	# Where the brush will bite.
	highlight = MeshInstance3D.new()
	var hs := SphereMesh.new()
	hs.radial_segments = 14
	hs.rings = 8
	highlight.mesh = hs
	var hm := StandardMaterial3D.new()
	hm.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	hm.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	hm.albedo_color = Color(1.0, 0.95, 0.6, 0.13)
	hm.cull_mode = BaseMaterial3D.CULL_DISABLED
	highlight.material_override = hm
	highlight.visible = false
	world.add_child.call_deferred(highlight)

	if not ("--shot" in OS.get_cmdline_user_args()):
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
		_build_fade()
	else:
		woke = true
	_snap_to_ground()
	_update_camera()

func _part(parent: Node3D, size: Vector3, col: Color, pos: Vector3, is_skin := false) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	var bm := BoxMesh.new()
	bm.size = size
	mi.mesh = bm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = col
	# Skin-tone parts get faint emission so they catch axis light even when
	# the body is in its own AO. Real miniature figures always have a painted
	# highlight.
	if is_skin:
		mat.emission_enabled = true
		mat.emission = col.lightened(0.12)
		mat.emission_energy_multiplier = 0.35
	mi.material_override = mat
	mi.position = pos
	parent.add_child(mi)
	return mi

func _build_fade() -> void:
	var cl := CanvasLayer.new()
	cl.layer = 3
	fade = ColorRect.new()
	fade.set_anchors_preset(Control.PRESET_FULL_RECT)
	fade.color = Color(0, 0, 0, 1)
	fade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	cl.add_child(fade)
	add_child(cl)

func _snap_to_ground() -> void:
	r = world.ground_at(theta, z)
	vr = 0.0
	on_ground = true

# --------------------------------------------------------------- geometry --

func tangent() -> Vector3:
	return Vector3(-sin(theta), cos(theta), 0.0)

func up() -> Vector3:
	return Vector3(-cos(theta), -sin(theta), 0.0)

func feet_pos() -> Vector3:
	return Vector3(r * cos(theta), r * sin(theta), z)

func heading() -> Vector3:
	# yaw 0 faces +z (along the drum); yaw sweeps toward the tangential.
	return (Vector3(0, 0, 1) * cos(yaw) + tangent() * sin(yaw)).normalized()

# ------------------------------------------------------------------ input --

func _input(e: InputEvent) -> void:
	if e is InputEventMouseButton and e.pressed and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		if e.button_index == MOUSE_BUTTON_LEFT:
			_edit(true)
		elif e.button_index == MOUSE_BUTTON_RIGHT:
			_build()
		# Trackpad scrolling arrives in bursts, so steps are small and the
		# actual distance eases toward the target.
		elif e.button_index == MOUSE_BUTTON_WHEEL_UP:
			cam_dist_target = clampf(cam_dist_target - 0.28, ZOOM_MIN, ZOOM_MAX)
		elif e.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			cam_dist_target = clampf(cam_dist_target + 0.28, ZOOM_MIN, ZOOM_MAX)
		# Two-finger horizontal scroll sizes the brush — native on a trackpad.
		elif e.button_index == MOUSE_BUTTON_WHEEL_LEFT:
			brush = clampf(brush - 0.18, 1.2, 7.0)
		elif e.button_index == MOUSE_BUTTON_WHEEL_RIGHT:
			brush = clampf(brush + 0.18, 1.2, 7.0)
	if e is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		var sens: float = RamaControls.sensitivity
		var iy: float = -1.0 if RamaControls.invert_y else 1.0
		yaw -= e.relative.x * sens
		pitch = clamp(pitch - e.relative.y * sens * iy, -1.45, 1.45)
	elif false and e is InputEventKey and e.pressed and not e.echo:
		match e.keycode:
			KEY_ESCAPE:
				Input.mouse_mode = (Input.MOUSE_MODE_VISIBLE
					if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED
					else Input.MOUSE_MODE_CAPTURED)
			KEY_T:
				_throw()
			KEY_Q:
				brush = clampf(brush - 0.6, 1.2, 7.0)
			KEY_E:
				brush = clampf(brush + 0.6, 1.2, 7.0)
			KEY_R:
				_edit(false)
			KEY_TAB:
				view = (view + 1) % 3
				cam_ready = false
				world.apply_view(view)
			KEY_1: module = 0
			KEY_2: module = 1
			KEY_3: module = 2
			KEY_4: module = 3
			KEY_F:
				_edit(true)
			KEY_G:
				_build()
			KEY_Z:
				_undo()
			KEY_BRACKETLEFT:
				mouse_sens = maxf(mouse_sens - 0.0004, 0.0006)
			KEY_BRACKETRIGHT:
				mouse_sens = minf(mouse_sens + 0.0004, 0.0090)
			KEY_P:
				tilt_on = not tilt_on
				for c in world.get_children():
					if c is CanvasLayer and c.layer == 1:
						c.visible = tilt_on

## Where the player is looking, and how far.
func aim() -> Dictionary:
	var u := up()
	var f := heading()
	var right := f.cross(u).normalized()
	f = f.rotated(right, pitch).normalized()
	var eye_pos := feet_pos() + u * eye
	return world.terrain.raycast(eye_pos, f, 40.0)

## Dig (or fill). The brush centre snaps to a 1 m grid so each bite reads as a
## discrete chunk of rock, while the underlying field stays smooth SDF.
## Dig returns mass into the pack (LANDSCAPE_1400 §X) — overflow becomes spoil.
func _edit(remove: bool) -> void:
	var hit: Dictionary = aim()
	if not hit.get("hit", false):
		return
	var p: Vector3 = hit["point"] if remove else hit["air"]
	if remove:
		var y: Dictionary = world.terrain.dig(p, brush, world.DIG_SNAP, level_brush)
		if not y.get("ok", false):
			return
		world.terrain.notify_dig(p, brush)
		world.schedule_pool_refresh()
		world.schedule_stockpile_refresh()
		var wdep: float = world.terrain.water_depth_at(atan2(p.y, p.x), p.z)
		if wdep > 0.05 or world.terrain.water_flux(atan2(p.y, p.x), p.z) > 0.15:
			world.spawn_dig_splash(p, brush * 0.35 + wdep * 0.5)
		# Brief yield flash in the aim readout.
		last_aim["yield_kg"] = float(y.get("mass_kg", 0.0))
		last_aim["yield_accepted"] = float(y.get("accepted", 1.0))
	else:
		world.terrain.fill(p, brush * 0.8, world.DIG_SNAP, level_brush)
	world.rebuild_around(p, brush * (2.0 if level_brush else 1.0) + 2.0)
	if world.audio:
		var hard := 1.0
		var pr: Dictionary = world.terrain.probe(p)
		hard = float(pr.get("hardness", 1.0))
		world.audio.dig(brush, hard)

func _build() -> void:
	var hit: Dictionary = aim()
	if not hit.get("hit", false):
		return
	world.place_module(hit["point"], module)
	if world.audio:
		world.audio.place()

func _undo() -> void:
	var hit: Dictionary = world.terrain.undo_dig()
	if hit.get("ok", false):
		world.rebuild_around(hit["point"], float(hit["radius"]) + 2.0)
		if world.audio:
			world.audio.undo()

func _throw() -> void:
	var p := feet_pos() + up() * (eye - 0.1) + heading() * 0.6
	var v := (heading() * 26.0 + up() * 7.5)
	var ball := MeshInstance3D.new()
	var sm := SphereMesh.new()
	sm.radius = 0.22
	sm.height = 0.44
	ball.mesh = sm
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = Color(1.0, 0.85, 0.35)
	mat.emission_enabled = true
	mat.emission = Color(1.0, 0.8, 0.3)
	mat.emission_energy_multiplier = 2.0
	ball.material_override = mat
	world.add_child(ball)
	throws.append({"p": p, "v": v, "node": ball, "life": 9.0, "path": [p]})

## Scoop standing water into the pack, or pour carried water onto the aim point.
func _bottle_water() -> void:
	if bottle_cd > 0.0:
		return
	bottle_cd = 0.22
	var hit: Dictionary = aim()
	var th := theta
	var zz := z
	var splash_p := feet_pos()
	if hit.get("hit", false):
		var p: Vector3 = hit["point"]
		th = atan2(p.y, p.x)
		zz = p.z
		splash_p = p
	var depth: float = world.terrain.water_depth_at(th, zz)
	var inv: Dictionary = world.terrain.inventory()
	var have_water := 0.0
	for s in inv.get("stacks", []):
		if int(s.get("material_id", -1)) == 104:
			have_water = float(s.get("mass_kg", 0.0))
			break
	# Prefer scoop when standing water is present; otherwise pour if we carry any.
	if depth >= 0.12:
		var sc: Dictionary = world.terrain.scoop_water(th, zz, 8.0)
		if sc.get("ok", false):
			world.schedule_pool_refresh()
			world.spawn_dig_splash(splash_p, 0.55 + float(sc.get("liters", 0.0)) * 0.04)
			if world.audio:
				world.audio.splash(0.6)
	elif have_water >= 0.2:
		var pr: Dictionary = world.terrain.pour_water(th, zz, minf(8.0, have_water))
		if pr.get("ok", false):
			world.schedule_pool_refresh()
			world.spawn_dig_splash(splash_p, 0.45 + float(pr.get("liters", 0.0)) * 0.04)
			if world.audio:
				world.audio.splash(0.5)

# ------------------------------------------------------------------- tick --

func _physics_process(dt: float) -> void:
	if not woke:
		wake_t += dt
		# Waking up: eyes open on the ground, then you get to your feet.
		var k: float = clamp(wake_t / 3.4, 0.0, 1.0)
		eye = lerp(0.28, 1.72, ease(k, 0.4))
		pitch = lerp(-0.30, 0.04, ease(k, 0.5))
		if fade:
			fade.color.a = clamp(1.0 - wake_t / 2.2, 0.0, 1.0)
		if wake_t > 3.9:
			woke = true
		_snap_to_ground()
		_update_camera()
		_step_throws(dt)
		return

	var om: float = world.P["omega"]
	var g: float = om * om * r

	# Movement in the local tangent plane.
	var mv := Input.get_vector(RamaControls.act("left"), RamaControls.act("right"),
			RamaControls.act("forward"), RamaControls.act("back"))
	var fwd := -mv.y
	var side := mv.x
	var running: bool = Input.is_action_pressed(RamaControls.act("run"))
	var inv: Dictionary = world.terrain.inventory()
	var enc: float = float(inv.get("encumbrance", 1.0))
	var speed: float = (RUN if running else WALK) * enc
	var jump_v: float = JUMP * sqrt(enc)
	if fwd != 0.0 or side != 0.0:
		var mag := sqrt(fwd * fwd + side * side)
		if mag > 1.0:
			fwd /= mag; side /= mag
		# right = f x u = sin(yaw)*axial - cos(yaw)*tangent. The old signs were
		# the negative of that, which mirrored A and D.
		var d_ax := (fwd * cos(yaw) + side * sin(yaw)) * speed * dt
		var d_tg := (fwd * sin(yaw) - side * cos(yaw)) * speed * dt
		var half: float = float(world.P["length"]) * 0.48
		z = clamp(z + d_ax, -half, half)
		theta = wrapf(theta + d_tg / max(r, 1.0), -PI, PI)

	# Radial motion: spin gravity pulls outward.
	vr += g * dt
	r += vr * dt
	var ground: float = world.ground_at(theta, z)
	if r >= ground:
		r = ground
		vr = 0.0
		on_ground = true
		if Input.is_action_pressed(RamaControls.act("jump")):
			vr = -jump_v
			on_ground = false
	else:
		on_ground = false

	_pad_look(dt)
	_actions(dt)
	var moved: float = 0.0
	if fwd != 0.0 or side != 0.0:
		moved = speed
	animate(dt, moved)
	if world.audio:
		world.audio.footstep(dt, moved if on_ground else 0.0)
	drum_spin += dt * 0.10
	_update_camera()
	_step_throws(dt)

	# Aim raycast+probe every frame while digging; otherwise every other tick.
	var digging: bool = Input.is_action_pressed(RamaControls.act("dig")) \
			or Input.is_action_pressed(RamaControls.act("place"))
	if digging or Engine.get_physics_frames() % 2 == 0:
		_refresh_aim()

	if Engine.get_process_frames() % 20 == 0:
		world._queue_chunks()

func _step_throws(dt: float) -> void:
	var omega: float = world.P["omega"]
	trail_mesh.clear_surfaces()
	var any := false
	for t in throws:
		if t["life"] <= 0.0:
			continue
		var p: Vector3 = t["p"]
		var v: Vector3 = t["v"]
		# In the rotating frame: centrifugal outward + Coriolis. (A5)
		var rad := Vector3(p.x, p.y, 0.0)
		var rl: float = maxf(rad.length(), 0.001)
		var centrifugal: Vector3 = rad / rl * (omega * omega * rl)
		var coriolis := Vector3(2.0 * omega * v.y, -2.0 * omega * v.x, 0.0)
		v += (centrifugal + coriolis) * dt
		p += v * dt
		t["v"] = v; t["p"] = p
		t["life"] -= dt
		t["node"].position = p
		t["path"].append(p)
		if t["path"].size() > 260:
			t["path"].pop_front()
		# Stop when it meets the ground.
		var th := atan2(p.y, p.x)
		if rl >= world.ground_at(th, p.z) - 0.2:
			t["v"] = Vector3.ZERO
		any = true
	if any:
		trail_mesh.surface_begin(Mesh.PRIMITIVE_LINES)
		for t in throws:
			var path: Array = t["path"]
			for i in range(1, path.size()):
				var a := float(i) / path.size()
				trail_mesh.surface_set_color(Color(1.0, 0.75, 0.25, a))
				trail_mesh.surface_add_vertex(path[i - 1])
				trail_mesh.surface_set_color(Color(1.0, 0.75, 0.25, a))
				trail_mesh.surface_add_vertex(path[i])
		trail_mesh.surface_end()
	for i in range(throws.size() - 1, -1, -1):
		if throws[i]["life"] <= 0.0:
			throws[i]["node"].queue_free()
			throws.remove_at(i)

## One raycast per frame, shared by the reticle, the brush marker and the
## module ghost. Three separate raycasts for the same ray would be silly.
## Right stick look. Read as raw axes rather than actions because look wants
## an analog rate, not a boolean.
func _pad_look(dt: float) -> void:
	var lx := Input.get_joy_axis(0, JOY_AXIS_RIGHT_X)
	var ly := Input.get_joy_axis(0, JOY_AXIS_RIGHT_Y)
	if absf(lx) < 0.16: lx = 0.0
	if absf(ly) < 0.16: ly = 0.0
	if lx == 0.0 and ly == 0.0:
		return
	var iy: float = -1.0 if RamaControls.invert_y else 1.0
	yaw -= lx * pad_sens * dt
	pitch = clamp(pitch - ly * pad_sens * dt * iy, -1.45, 1.45)

## One place where every binding is consulted, so keyboard, mouse and gamepad
## all take the same path.
func _actions(dt: float) -> void:
	var A := RamaControls
	if Input.is_action_just_pressed(A.act("pause")):
		world.menu.toggle()
		return
	if world.menu and world.menu.visible:
		return
	if Input.is_action_just_pressed(A.act("view")):
		view = (view + 1) % 3
		cam_ready = false
		world.apply_view(view)
	if Input.is_action_just_pressed(A.act("undo")):
		_undo()
	if Input.is_action_just_pressed(A.act("fill")):
		_edit(false)
	if Input.is_action_just_pressed(A.act("place")):
		_build()
	if Input.is_action_just_pressed(A.act("level")):
		level_brush = not level_brush
	if Input.is_action_just_pressed(A.act("throw")):
		_throw()
	if Input.is_action_just_pressed(A.act("harvest")):
		var hy: Dictionary = world.terrain.harvest_near(theta, z, 4.5)
		if hy.get("ok", false):
			world.refresh_stockpiles()
			if world.audio:
				world.audio.place()
	if Input.is_action_pressed(A.act("bottle")):
		_bottle_water()
	if Input.is_action_just_pressed(A.act("drop")):
		var dy: Dictionary = world.terrain.drop_inventory(theta, z)
		if dy.get("ok", false):
			world.refresh_stockpiles()
			if world.audio:
				world.audio.place()
	if Input.is_action_just_pressed(A.act("craft_prev")):
		var n: int = int(world.terrain.recipe_count())
		if n > 0:
			recipe_idx = (recipe_idx + n - 1) % n
	if Input.is_action_just_pressed(A.act("craft_next")):
		var n2: int = int(world.terrain.recipe_count())
		if n2 > 0:
			recipe_idx = (recipe_idx + 1) % n2
	if Input.is_action_just_pressed(A.act("craft")):
		var rec: Dictionary = world.terrain.recipe_at(recipe_idx)
		if bool(rec.get("needs_station", false)) and not bool(rec.get("station_near", true)):
			print("[rama] need %s station nearby" % str(rec.get("station", "?")))
		else:
			var scale: float = float(rec.get("max_scale", 0.0))
			if scale >= 0.05:
				var cr: Dictionary = world.terrain.craft(recipe_idx, scale, theta, z)
				if cr.get("ok", false):
					world.refresh_stockpiles()
					if world.audio:
						world.audio.place()
					print("[rama] craft %s ×%.2f  O₂ −%.1f  CO₂ +%.1f" % [
						cr.get("id", "?"), float(cr.get("scale", 0.0)),
						float(cr.get("o2_used", 0.0)), float(cr.get("co2_made", 0.0))])
				elif str(cr.get("error", "")) == "need_station":
					print("[rama] need %s station nearby" % str(cr.get("station", "?")))
	if Input.is_action_just_pressed(A.act("eat")):
		var em: Dictionary = world.terrain.eat_meal()
		if em.get("ok", false):
			print("[rama] ate ramen grade %.0f%% · satiety %.0f%%" % [
				float(em.get("grade", 0.0)) * 100.0, float(em.get("satiety", 0.0)) * 100.0])
			if world.audio:
				world.audio.place()
		else:
			print("[rama] no ramen in pack")
	if Input.is_action_just_pressed(A.act("station")):
		# Cycle kitchen → kiln → smelter based on recipe station hint.
		var st := "kitchen"
		var hint: Dictionary = world.terrain.recipe_at(recipe_idx)
		var hs: String = str(hint.get("station", "kitchen"))
		if hs in ["kiln", "smelter", "kitchen"]:
			st = hs
		var ps: Dictionary = world.terrain.place_station(st, theta, z)
		if ps.get("ok", false):
			world.refresh_stations()
			print("[rama] placed %s" % st)
			if world.audio:
				world.audio.place()
		else:
			print("[rama] place %s failed: %s" % [st, str(ps.get("error", "?"))])
	if Input.is_action_just_pressed(A.act("amend")):
		# Prefer ash → lime → manure → bone meal from whatever is in the pack.
		var inv: Dictionary = world.terrain.inventory()
		var prefer := [115, 116, 120, 121]  # ash, lime, manure, bone meal
		var chosen := -1
		for s in inv.get("stacks", []):
			var mid: int = int(s.get("material_id", -1))
			if mid in prefer and float(s.get("mass_kg", 0.0)) >= 0.2:
				chosen = mid
				break
		if chosen >= 0:
			var am: Dictionary = world.terrain.amend_soil(theta, z, chosen, 2.0)
			if am.get("ok", false) and world.audio:
				world.audio.place()
	if Input.is_action_just_pressed(A.act("scrub")):
		var atmo: Dictionary = world.terrain.atmosphere()
		var cur: float = float(atmo.get("scrub_mw", 0.0))
		var nxt: float = 0.0 if cur > 0.1 else 2.5
		world.terrain.set_scrub_mw(nxt)
		print("[rama] scrubber %.1f MW" % nxt)
	if Input.is_action_just_pressed(A.act("tilt")):
		tilt_on = not tilt_on
		for c in world.get_children():
			if c is CanvasLayer and c.layer == 1:
				c.visible = tilt_on
	if Input.is_action_just_pressed(A.act("waypoint")):
		world.set_waypoint(theta, z)
	if Input.is_action_just_pressed(A.act("save")):
		world.save_world()
	if Input.is_action_just_pressed(A.act("load")):
		world.load_world()
	if Input.is_action_just_pressed(A.act("soil")):
		world.cycle_soil_mode()
	if Input.is_action_just_pressed(A.act("map_in")):
		world.mini_zoom(-1)
	if Input.is_action_just_pressed(A.act("map_out")):
		world.mini_zoom(1)
	if Input.is_action_pressed(A.act("brush_up")):
		brush = clampf(brush + 3.4 * dt, 1.2, 7.0)
	if Input.is_action_pressed(A.act("brush_down")):
		brush = clampf(brush - 3.4 * dt, 1.2, 7.0)
	for i in 4:
		if Input.is_action_just_pressed(A.act("mod%d" % (i + 1))):
			module = i
	dig_cd -= dt
	bottle_cd -= dt
	var digging: bool = Input.is_action_pressed(A.act("dig"))
	world.dig_held = digging
	if digging and dig_cd <= 0.0:
		# Back-pressure: if remesh is behind, bite slower instead of stacking hitch.
		var backlog: int = world.remesh_backlog()
		_edit(true)
		dig_cd = 0.28 if backlog >= 4 else (0.20 if backlog >= 2 else 0.16)

func _refresh_aim() -> void:
	var hit: Dictionary = aim()
	last_aim = hit
	can_dig = true
	if hit.get("hit", false):
		var pr: Dictionary = world.terrain.probe(hit["point"])
		can_dig = bool(pr.get("diggable", true))
		last_aim["kind"] = pr.get("kind", "")
	if highlight:
		var show_brush: bool = hit.get("hit", false) and view == 0
		highlight.visible = show_brush
		if show_brush:
			# Show the brush's actual shape: a sphere, or the disc the
			# levelling brush cuts. A sphere marker on a disc brush is a lie.
			var pt: Vector3 = hit["point"]
			var radial := Vector3(-pt.x, -pt.y, 0.0).normalized()
			var ax := Vector3(0, 0, 1)
			var rt := radial.cross(ax).normalized()
			highlight.transform = Transform3D(Basis(rt, radial, ax), pt)
			highlight.scale = Vector3(brush, 0.22 if level_brush else brush, brush)
	if ghost:
		var show_ghost: bool = hit.get("hit", false) and view == 0
		ghost.visible = show_ghost
		if show_ghost:
			world.pose_ghost(ghost, hit["point"], module)

func _update_camera() -> void:
	match view:
		1: _cam_drum(); return
		2: _cam_map(); return
	_cam_colonist()

## DRUM — the whole habitat at once. Looking ACROSS the drum, not down it:
## from near the axis every wall is edge-on and catches no light, so the camera
## stands well out toward one side and faces the opposite wall, which then fills
## the frame lit and visibly curved.
func _cam_drum() -> void:
	cam.projection = Camera3D.PROJECTION_PERSPECTIVE
	cam.fov = 88.0
	var L: float = float(world.P["length"])
	var R: float = float(world.P["radius"])
	var eye_z: float = clampf(z, -L * 0.5 + 200.0, L * 0.5 - 200.0)
	var off := Vector3(cos(drum_spin), sin(drum_spin), 0.0) * (R * 0.78)
	var pos := Vector3(off.x, off.y, eye_z)
	# Aim through the axis at the far wall, tilted along the drum so its length
	# is legible too.
	var look := Vector3(-off.x, -off.y, 0).normalized() * R + Vector3(0, 0, eye_z + L * 0.22)
	var fwd := (look - pos).normalized()
	var up_ref := Vector3(-off.x, -off.y, 0).normalized()
	var right := fwd.cross(up_ref).normalized()
	if right.length() < 0.01:
		right = Vector3(0, 0, 1).cross(fwd).normalized()
	var tu := right.cross(fwd).normalized()
	cam.transform = Transform3D(Basis(right, tu, -fwd), pos)
	if body: body.visible = false

## MAP — orthographic, straight down the local radial. Because the patch under
## you is nearly flat at this scale, this reads as an honest plan view.
func _cam_map() -> void:
	cam.projection = Camera3D.PROJECTION_ORTHOGONAL
	cam.size = 190.0
	var u := up()
	var pos := feet_pos() + u * 150.0
	var fwd := -u                       # look back down at the ground
	var axial := Vector3(0, 0, 1)       # north is +z along the drum
	var right := fwd.cross(axial).normalized()
	var tu := right.cross(fwd).normalized()
	cam.transform = Transform3D(Basis(right, tu, -fwd), pos)
	if body: body.visible = true

func _cam_colonist() -> void:
	cam.projection = Camera3D.PROJECTION_PERSPECTIVE
	cam.fov = RamaControls.fov
	var u := up()
	var f := heading()
	var right := f.cross(u).normalized()
	f = f.rotated(right, pitch).normalized()
	var true_up := right.cross(f).normalized()
	var eye_pos := feet_pos() + u * eye

	cam_dist = lerpf(cam_dist, cam_dist_target, 0.22)
	var first_person: bool = cam_dist < 0.9
	var want: Vector3 = eye_pos
	if not first_person:
		want = eye_pos - f * cam_dist + u * (0.30 + cam_dist * 0.16)
		# Above ground means a SMALLER radius here — a radial clamp, not a
		# height clamp. There is no world up to clamp against.
		var c_r: float = sqrt(want.x * want.x + want.y * want.y)
		var c_ground: float = world.ground_at(atan2(want.y, want.x), want.z)
		if c_r > c_ground - 0.9:
			var k: float = (c_ground - 0.9) / maxf(c_r, 0.001)
			want = Vector3(want.x * k, want.y * k, want.z)

	# Smooth follow, so the camera has a little weight.
	if not cam_ready:
		cam_target = want
		cam_ready = true
	else:
		cam_target = cam_target.lerp(want, 0.28)
	cam.transform = Transform3D(Basis(right, true_up, -f), cam_target)

	if body:
		body.visible = not first_person
		var bp := feet_pos()
		var bf := heading()
		var br := bf.cross(u).normalized()
		body.transform = Transform3D(Basis(br, u, br.cross(u).normalized()), bp)

## Procedural walk cycle. Nothing is keyframed — the gait phase advances with
## distance travelled, so speed and animation cannot drift apart.
func animate(dt: float, speed: float) -> void:
	if rig.is_empty():
		return
	gait += speed * dt * 1.9
	var moving: float = clampf(speed / 6.2, 0.0, 1.6)
	var swing: float = sin(gait) * 0.62 * moving
	var swing2: float = sin(gait + PI) * 0.62 * moving
	rig["leg1"].rotation = Vector3(swing, 0, 0)
	rig["leg-1"].rotation = Vector3(swing2, 0, 0)
	rig["arm1"].rotation = Vector3(swing2 * 0.75, 0, 0)
	rig["arm-1"].rotation = Vector3(swing * 0.75, 0, 0)
	# Bob twice per stride, and lean into the run.
	var bob: float = abs(sin(gait)) * 0.07 * moving
	rig["hip"].position.y = 0.92 - bob
	rig["torso"].rotation = Vector3(moving * 0.16, 0, 0)
	# Head tilts toward the camera pitch, so the figure reads as looking
	# where you look.
	if rig.has("head"):
		rig["head"].rotation.x = pitch * 0.3
	# Idle breath when still.
	if moving < 0.05:
		rig["torso"].scale.y = 1.0 + sin(Time.get_ticks_msec() * 0.0018) * 0.012
