extends Node3D
const RamaControls = preload("res://scripts/controls.gd")
const RamaBody = preload("res://scripts/avatar/body.gd")
const RamaGait = preload("res://scripts/avatar/gait.gd")
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
## Set by world before _ready, from the habitat's own diagonal.
var cam_far := 5200.0
var body: Node3D
var rig := {}
## Gait PHASE, in radians, one stride per turn. Advanced by distance travelled
## (see `gait.advance`) — never by time, or the feet skate.
var gait := 0.0
var body_spec := {}
var gait_spec := {}
var cam_dist := 4.6
var cam_target := Vector3.ZERO
var cam_ready := false
var view := 0            # 0 colonist · 1 drum · 2 map
var drum_spin := 0.0
var cam_dist_target := 4.6
var last_aim := {}
## Biological material ids from the sim's `economy::bio_id`, and the mass one
## placed block spends. Kept in step with `place_veg_block`, which charges the
## same amounts — if these drift the fill button silently falls back to dirt.
const MAT_GREEN := 100
const MAT_WOOD := 101
const WOOD_BLOCK_KG := 2.8
const LEAF_BLOCK_KG := 0.455
var can_dig := true
var ghost: Node3D
var level_brush := false
var pad_sens := 2.6
var wake_t := 0.0
var woke := false
var fade: ColorRect
var tilt_on := true
var bottle_cd := 0.0
var _step_hard := 1.0
var _step_wet := 0.0
## When true, colonist cam snaps orbit (no soft-follow lag on look).
var _look_dirty := false
var _enc_cache := 1.0
var _enc_age := 99.0

const JUMP := 6.4
## Horizontal m/s by gear (1 stroll · 2 run · 3 cross-drum).
const GEAR_WALK := [8.0, 22.0, 90.0]
const GEAR_RUN := [14.0, 45.0, 220.0]
const GOD_VERT := [28.0, 55.0, 120.0]
var mouse_sens := 0.0026
const ZOOM_MIN := 0.0
const ZOOM_MAX := 7.5

var module := 0
## Seconds left showing the placement ghost after picking a module, so choosing
## one previews where it lands without pinning the overlay on screen forever.
var module_shown := 0.0
const MODULE_PREVIEW_S := 2.5
var brush := 2.6
var recipe_idx := 0
var highlight: MeshInstance3D
var dig_cd := 0.0
var throws: Array = []
var trail: MeshInstance3D
var trail_mesh: ImmediateMesh
## Free-fly / noclip for surveying the drum. F1 toggles.
var god_mode := false
## 0/1/2 → keys 1/2/3. Gear 3 is for crossing the habitat fast.
var speed_gear := 1

func _ready() -> void:
	cam = Camera3D.new()
	cam.fov = RamaControls.fov
	cam.near = 0.08
	cam.far = cam_far
	add_child(cam)
	cam.current = true

	# An articulated colonist, so scale reads and motion has weight.
	body = Node3D.new()
	body.name = "Colonist"
	world.add_child.call_deferred(body)
	_rebuild_body()

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

## Resolve the saved build and put it on the rig.
##
## Three layers, in order: an archetype, an optional blend toward a second one,
## then any hand-set axes. So the character creator can be "pick one of eight",
## "somewhere between these two", or "move this number", and they compose.
func _rebuild_body() -> void:
	var cfg: Dictionary = RamaControls.avatar
	var spec: Dictionary = RamaBody.make(str(cfg.get("archetype", "daddy")))
	var to: String = str(cfg.get("blend_to", ""))
	var mix: float = clampf(float(cfg.get("blend", 0.0)), 0.0, 1.0)
	if to != "" and mix > 0.001 and RamaBody.ARCHETYPES.has(to):
		spec = RamaBody.blend(spec, RamaBody.make(to), mix)
		gait_spec = RamaGait.blend(RamaGait.make(str(cfg["archetype"])),
				RamaGait.make(to), mix)
	else:
		gait_spec = RamaGait.make(str(cfg.get("archetype", "daddy")))
	var axes: Dictionary = cfg.get("axes", {})
	if not axes.is_empty():
		for k in axes:
			spec[k] = axes[k]
		# Hand-built bodies get a hand-built gait: derive one from the shape
		# rather than keep walking like the archetype he no longer is.
		gait_spec = RamaGait.blend(gait_spec, RamaGait.for_body(spec), 0.7)
	body_spec = spec
	rig = RamaBody.build(body, spec)
	# Stand the camera in his own head, not at a constant 1.72.
	eye = float(spec["stature"]) * 0.935

## Cycle the build. The archetype list is a spectrum, not a menu, so stepping
## through it is also the fastest way to see whether a gait reads.
func cycle_body(dir: int) -> void:
	var order: Array = RamaBody.ORDER
	var i: int = order.find(str(RamaControls.avatar.get("archetype", "daddy")))
	i = posmod(i + dir, order.size())
	RamaControls.avatar["archetype"] = order[i]
	RamaControls.avatar["blend_to"] = ""
	RamaControls.avatar["axes"] = {}
	_rebuild_body()
	if world and world.has_method("note"):
		world.note("build: %s" % str(order[i]).replace("_", " "))

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
		# Dig / fill / place come from InputMap (see controls.right_click).
		# Trackpad scrolling arrives in bursts, so steps are small and the
		# actual distance eases toward the target.
		if e.button_index == MOUSE_BUTTON_WHEEL_UP:
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
		# Apply look this frame — waiting for physics made turns feel frictional
		# whenever the sim hitch'd, and soft cam follow compounded it.
		_look_dirty = true
		if view == 0 and woke:
			_update_camera()
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
			KEY_1: speed_gear = 0; world.note("speed 1 — stroll")
			KEY_2: speed_gear = 1; world.note("speed 2 — run")
			KEY_3: speed_gear = 2; world.note("speed 3 — cross-drum")
			KEY_4: module = 0; module_shown = MODULE_PREVIEW_S
			KEY_5: module = 1; module_shown = MODULE_PREVIEW_S
			KEY_6: module = 2; module_shown = MODULE_PREVIEW_S
			KEY_7: module = 3; module_shown = MODULE_PREVIEW_S
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
	# Guest: send intent to host; do not mutate local authority.
	if world.session and world.session.joined:
		if not world.session.can_dig():
			world.note("Host has not granted dig permission (Esc → CO-OP).")
			return
		if remove:
			world.session.request_dig(p, brush, world.DIG_SNAP, level_brush, true)
		else:
			var inv: Dictionary = _inv_for_me()
			var have_wood := 0.0
			var have_leaf := 0.0
			for s in inv.get("stacks", []):
				var mid: int = int(s.get("material_id", -1))
				if mid == MAT_WOOD:
					have_wood = float(s.get("mass_kg", 0.0))
				elif mid == MAT_GREEN:
					have_leaf = float(s.get("mass_kg", 0.0))
			if have_wood >= WOOD_BLOCK_KG or have_leaf >= LEAF_BLOCK_KG:
				var as_leaf: bool = have_wood < WOOD_BLOCK_KG and have_leaf >= LEAF_BLOCK_KG
				world.session.request_place_veg(p, as_leaf)
			else:
				world.session.request_dig(p, brush * 0.8, world.DIG_SNAP, level_brush, false)
		return
	if remove:
		var y: Dictionary = world.terrain.dig(p, brush, world.DIG_SNAP, level_brush)
		if not y.get("ok", false):
			return
		if not hit.get("vegetation", false):
			world.terrain.notify_dig(p, brush)
			world.schedule_pool_refresh()
		world.schedule_stockpile_refresh()
		var blocks: int = int(y.get("blocks", 0))
		if bool(y.get("vegetation", false)) and blocks > 0:
			# Say what came away, since dig no longer fells whole trees and
			# there was no message at all before.
			world.refresh_woodscape(true)
			var got := 0.0
			var what := "leaves"
			for part in y.get("parts", []):
				var mid: int = int(part.get("material_id", -1))
				if mid == MAT_WOOD:
					got += float(part.get("mass_kg", 0.0))
					what = "timber"
				elif mid == MAT_GREEN:
					got += float(part.get("mass_kg", 0.0))
			# Block count, because the size of the bite is the thing the brush
			# control is for and mass alone does not show it — leaves weigh
			# almost nothing however many you clear.
			var bite := "%d blocks" % blocks if blocks > 1 else "1 block"
			# Wood the cut left unsupported came down on its own. Say so
			# separately: it is a pile at the foot of the tree, not something
			# the swing put in your pack, and it can be tonnes.
			var fell_kg: float = float(y.get("felled_kg", 0.0))
			if fell_kg > 0.5:
				world.refresh_stockpiles()
				world.force_plant_refresh()
				world.note("%s · %.0f kg came down · L to take" % [bite, fell_kg])
				if world.audio:
					world.audio.dig(4.0, 0.75)
				world.spawn_dig_chips(p, hit.get("normal", Vector3.UP),
						int(y.get("felled_blocks", 0)), true)
				last_aim["yield_kg"] = float(y.get("mass_kg", 0.0))
				last_aim["yield_accepted"] = float(y.get("accepted", 1.0))
				return
			if float(y.get("accepted", 1.0)) < 0.95:
				world.refresh_stockpiles()
				world.note("%s · pack full · %s piled · L to take" % [bite, what])
			else:
				world.note("%s · +%.1f kg %s" % [bite, got, what])
			last_aim["yield_kg"] = float(y.get("mass_kg", 0.0))
			last_aim["yield_accepted"] = float(y.get("accepted", 1.0))
			if world.audio:
				# Wood, not rock, and no terrain rebuild — nothing moved but
				# blocks. Skipping `rebuild_around` is the point of the branch.
				#
				# Scaled by what the swing actually took: a great axe through a
				# canopy and a notch out of a sapling made exactly the same
				# sound, which flattened the whole size control to cosmetics.
				var heft: float = clampf(pow(float(blocks), 0.34), 1.0, 4.5)
				world.audio.dig(heft, 0.62 if what == "timber" else 0.3)
			world.spawn_dig_chips(p, hit.get("normal", Vector3.UP), blocks,
					what == "timber")
			if world.session and world.session.hosting:
				world.session._flush_host_events()
			return
		# Ice does not survive being lifted out of the ground in a temperate
		# province: it arrives as meltwater at your feet.
		var melt: float = float(y.get("meltwater_l", 0.0))
		if melt > 1.0:
			world.schedule_pool_refresh()
			world.note("meltwater · %.0f L released" % melt)
			world.spawn_dig_splash(p, clampf(melt / 120.0, 0.5, 3.0))
		var trees: int = int(y.get("trees", 0))
		if trees > 0:
			world.force_plant_refresh()
			world.refresh_stockpiles()
			world.refresh_woodscape(true)
			var timber: float = float(y.get("timber_kg", 0.0))
			var kept: float = float(y.get("timber_kept", timber))
			if kept > 0.05 and kept + 0.05 >= timber:
				world.note("harvested · +%.0f kg timber in pack" % kept)
			elif kept > 0.05:
				world.note("harvested · +%.0f kg pack · rest piled · L to take" % kept)
			elif timber > 0.05:
				world.note("pack full · timber piled · L to take")
			else:
				world.note("harvested leaf material")
		elif float(y.get("accepted", 1.0)) < 0.95:
			world.refresh_stockpiles()
			world.note("pack full · spoil piled · L to take")
		last_aim["yield_kg"] = float(y.get("mass_kg", 0.0))
		last_aim["yield_accepted"] = float(y.get("accepted", 1.0))
		last_aim["trees"] = trees
		var wdep: float = world.terrain.water_depth_at(atan2(p.y, p.x), p.z)
		if wdep > 0.05 or world.terrain.water_flux(atan2(p.y, p.x), p.z) > 0.15:
			world.spawn_dig_splash(p, brush * 0.35 + wdep * 0.5)
	else:
		# Prefer placing timber/leaf from pack — blocks combine with neighbours.
		var placed := false
		var inv2: Dictionary = _inv_for_me()
		var have_wood2 := 0.0
		var have_leaf2 := 0.0
		for s2 in inv2.get("stacks", []):
			var mid2: int = int(s2.get("material_id", -1))
			if mid2 == MAT_WOOD:
				have_wood2 = float(s2.get("mass_kg", 0.0))
			elif mid2 == MAT_GREEN:
				have_leaf2 = float(s2.get("mass_kg", 0.0))
		if have_wood2 >= WOOD_BLOCK_KG or have_leaf2 >= LEAF_BLOCK_KG:
			var as_leaf2: bool = have_wood2 < WOOD_BLOCK_KG and have_leaf2 >= LEAF_BLOCK_KG
			var pv: Dictionary = world.terrain.place_veg_block(p, as_leaf2)
			if pv.get("ok", false):
				placed = true
				world.refresh_woodscape(true)
				world.note("placed %s · combines with neighbours" % str(pv.get("kind", "block")))
		if not placed:
			world.terrain.fill(p, brush * 0.8, world.DIG_SNAP, level_brush)
	if not hit.get("vegetation", false):
		world.rebuild_around(p, brush * (2.0 if level_brush else 1.0) + 2.0)
	if world.audio:
		var hard := 1.0
		var pr: Dictionary = world.terrain.probe(p)
		hard = float(pr.get("hardness", 1.0))
		world.audio.dig(brush, hard)
		if world.playtest:
			world.playtest_mark("dig")
	if world.session and world.session.hosting:
		world.session._flush_host_events()

func _inv_for_me() -> Dictionary:
	if world.session and world.session.is_online() and world.session.has_method("inventory_snapshot"):
		var snap: Dictionary = world.session.inventory_snapshot()
		if not snap.is_empty():
			return snap
	if world.terrain.has_method("inventory_for") and world.session and world.session.is_online():
		return world.terrain.inventory_for(world.session.local_actor())
	return world.terrain.inventory()

func _build() -> void:
	if world.session and world.session.joined and not world.session.can_build():
		world.note("Host has not granted build permission.")
		return
	var hit: Dictionary = aim()
	if not hit.get("hit", false):
		return
	world.place_module(hit["point"], module)
	if world.audio:
		world.audio.place()
	if world.session and world.session.hosting:
		world.session._flush_host_events()

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
	var inv: Dictionary = _inv_for_me()
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
		# Reduced motion: snap awake instead of easing (§2107).
		if RamaControls.reduced_motion:
			eye = 1.72
			pitch = 0.04
			if fade:
				fade.color.a = 0.0
			woke = true
		else:
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
	_enc_age += dt
	if _enc_age > 0.25 and world.terrain:
		_enc_age = 0.0
		var inv: Dictionary = _inv_for_me()
		_enc_cache = float(inv.get("encumbrance", 1.0))
	var enc: float = _enc_cache
	var gix: int = clampi(speed_gear, 0, 2)
	var speed: float
	if god_mode:
		speed = (GEAR_RUN[gix] if running else GEAR_WALK[gix])
	else:
		# Same gears on foot — gear 3 still flies you across the drum.
		speed = (GEAR_RUN[gix] if running else GEAR_WALK[gix]) * enc
	var jump_v: float = JUMP * sqrt(enc)
	var old_theta := theta
	var old_z := z
	var old_ground: float = world.ground_at(theta, z)
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
		# Cliff bands: block steps taller than a scramble / slopes that read as wall
		# (LANDSCAPE_4200 §BO 3387). Forces routing to cols instead of climbing faces.
		if on_ground and not god_mode:
			var g1: float = world.ground_at(theta, z)
			var rise: float = g1 - old_ground
			var run: float = maxf(sqrt(d_ax * d_ax + d_tg * d_tg), 0.05)
			var blocked := rise > 1.35 or (rise > 0.55 and rise / run > 1.55)
			if blocked:
				# Try axial-only, then tangent-only — find the col sideways.
				var z_try: float = clamp(old_z + d_ax, -half, half)
				var th_try: float = wrapf(old_theta + d_tg / max(r, 1.0), -PI, PI)
				var g_ax: float = world.ground_at(old_theta, z_try)
				var g_tg: float = world.ground_at(th_try, old_z)
				if g_ax - old_ground <= 1.15 and not ((g_ax - old_ground) > 0.55 and absf(d_ax) > 0.05 and (g_ax - old_ground) / absf(d_ax) > 1.55):
					theta = old_theta
					z = z_try
				elif g_tg - old_ground <= 1.15 and not ((g_tg - old_ground) > 0.55 and absf(d_tg) > 0.05 and (g_tg - old_ground) / absf(d_tg) > 1.55):
					theta = th_try
					z = old_z
				else:
					theta = old_theta
					z = old_z

	var ground: float = world.ground_at(theta, z)
	if god_mode:
		# No spin gravity. Space = toward axis (up), Ctrl = outward (down).
		vr = 0.0
		var up_in := 0.0
		if Input.is_action_pressed(RamaControls.act("jump")):
			up_in -= 1.0
		if Input.is_action_pressed(RamaControls.act("descend")):
			up_in += 1.0
		r = clampf(r + up_in * GOD_VERT[gix] * dt, 40.0, float(world.P["radius"]) - 2.0)
		on_ground = absf(r - ground) < 1.5
	else:
		# Radial motion: spin gravity pulls outward.
		vr += g * dt
		r += vr * dt
		# Airborne Coriolis — the can is spinning. Real miss is ~0.5 m; we
		# push it so a hop lands a step spinward and you *feel* the drum.
		if r < ground - 0.02:
			var feel := 3.4
			var d_arc: float = -2.0 * om * vr * feel * dt
			theta = wrapf(theta + d_arc / max(r, 1.0), -PI, PI)
			ground = world.ground_at(theta, z)
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
		var hard := 1.0
		var wet := 0.0
		if on_ground and Engine.get_physics_frames() % 10 == 0 and world.terrain:
			var pr: Dictionary = world.terrain.probe(feet_pos())
			hard = float(pr.get("hardness", 1.0))
			wet = float(pr.get("moisture", 0.0))
			_step_hard = hard
			_step_wet = wet
		else:
			hard = _step_hard
			wet = _step_wet
		world.audio.footstep(dt, moved if on_ground else 0.0, hard, wet)
	if world.playtest:
		if pitch > 0.55:
			world.playtest_mark("lookup")
		if moved > 0.5:
			world.playtest_mark("walk")

	drum_spin += dt * 0.10
	_update_camera()
	_step_throws(dt)

	# Aim raycast+probe every frame while digging or filling; otherwise every other tick.
	var digging: bool = Input.is_action_pressed(RamaControls.act("dig")) \
			or Input.is_action_pressed(RamaControls.act("fill")) \
			or Input.is_action_pressed(RamaControls.act("place"))
	if digging or Engine.get_physics_frames() % 2 == 0:
		_refresh_aim()

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
	# Soft deadzone + response curve so small stick noise isn't sticky friction.
	var mag := sqrt(lx * lx + ly * ly)
	var dead := 0.12
	if mag < dead:
		return
	var t: float = (mag - dead) / (1.0 - dead)
	t = t * t  # ease-in: fine aim near centre, faster at edge
	lx = lx / mag * t
	ly = ly / mag * t
	var iy: float = -1.0 if RamaControls.invert_y else 1.0
	yaw -= lx * pad_sens * dt
	pitch = clamp(pitch - ly * pad_sens * dt * iy, -1.45, 1.45)
	_look_dirty = true

## One place where every binding is consulted, so keyboard, mouse and gamepad
## all take the same path.
func _actions(dt: float) -> void:
	var A := RamaControls
	if Input.is_action_just_pressed(A.act("pause")):
		world.menu.toggle()
		return
	if world.menu and world.menu.visible:
		return
	if Input.is_action_just_pressed(A.act("god")):
		god_mode = not god_mode
		vr = 0.0
		if god_mode:
			world.note("god mode — 1/2/3 speed · Shift sprint · Space/Ctrl up/down · F1 off")
		else:
			world.note("god mode off")
			_snap_to_ground()
	if Input.is_action_just_pressed(A.act("speed1")):
		speed_gear = 0
		world.note("speed 1 — stroll")
	elif Input.is_action_just_pressed(A.act("speed2")):
		speed_gear = 1
		world.note("speed 2 — run")
	elif Input.is_action_just_pressed(A.act("speed3")):
		speed_gear = 2
		world.note("speed 3 — cross-drum")
	if Input.is_action_just_pressed(A.act("view")):
		view = (view + 1) % 3
		cam_ready = false
		world.apply_view(view)
	if Input.is_action_just_pressed(A.act("undo")):
		_undo()
	if Input.is_action_just_pressed(A.act("place")):
		_build()
	if Input.is_action_just_pressed(A.act("level")):
		level_brush = not level_brush
	if Input.is_action_just_pressed(A.act("throw")):
		_throw()
	if Input.is_action_just_pressed(A.act("harvest")):
		if world.session and world.session.joined:
			if world.session.can_work():
				world.session.request_harvest(theta, z, 5.5)
			else:
				world.note("Host has not granted work permission.")
			return
		var hy: Dictionary = world.terrain.harvest_near(theta, z, 5.5)
		if hy.get("ok", false):
			world.force_plant_refresh()
			world.refresh_stockpiles()
			world.refresh_woodscape(true)
			var timber_h: float = float(hy.get("timber_kg", 0.0))
			if timber_h > 0.05:
				world.note("harvested · +%.0f kg timber" % timber_h)
			else:
				world.note("harvested")
			if world.session and world.session.hosting:
				world.session._flush_host_events()
		else:
			world.note("nothing to harvest")
		return
	if Input.is_action_pressed(A.act("bottle")):
		_bottle_water()
	if Input.is_action_just_pressed(A.act("take")):
		var tk: Dictionary = world.terrain.take_heap(theta, z, 3.5)
		if tk.get("ok", false):
			world.refresh_stockpiles()
			world.note("+%.1f kg %s" % [float(tk["kg"]), tk["material"]])
			if world.audio:
				world.audio.place()
		else:
			world.note("nothing in reach, or pack full")
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
			var craft_scale: float = float(rec.get("max_scale", 0.0))
			if craft_scale >= 0.05:
				if world.session and world.session.joined:
					if world.session.can_work():
						world.session.request_craft(recipe_idx, craft_scale, theta, z)
					else:
						world.note("Host has not granted work permission.")
				else:
					var cr: Dictionary = world.terrain.craft(recipe_idx, craft_scale, theta, z)
					if cr.get("ok", false):
						world.refresh_stockpiles()
						if world.audio:
							world.audio.place()
						print("[rama] craft %s ×%.2f  O₂ −%.1f  CO₂ +%.1f" % [
							cr.get("id", "?"), float(cr.get("scale", 0.0)),
							float(cr.get("o2_used", 0.0)), float(cr.get("co2_made", 0.0))])
						if world.session and world.session.hosting:
							world.session._flush_host_events()
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
		var inv: Dictionary = _inv_for_me()
		# Prefer ash → lime → manure → bone meal from whatever is in the pack.
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
	if Input.is_action_just_pressed(A.act("body_next")):
		cycle_body(1)
	if Input.is_action_just_pressed(A.act("body_prev")):
		cycle_body(-1)
	if Input.is_action_just_pressed(A.act("photo")):
		RamaControls.photo_mode = not RamaControls.photo_mode
		world.note("photo mode" if RamaControls.photo_mode else "HUD back")
	if Input.is_action_just_pressed(A.act("quiet")):
		if world.audio and world.audio.has_method("set_quiet"):
			world.audio.set_quiet(not RamaControls.quiet_mode)
	if Input.is_action_just_pressed(A.act("fastday")):
		if world.has_method("toggle_fast_day"):
			world.toggle_fast_day()
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
			module_shown = MODULE_PREVIEW_S
	dig_cd -= dt
	bottle_cd -= dt
	module_shown = maxf(module_shown - dt, 0.0)
	var digging: bool = Input.is_action_pressed(A.act("dig"))
	var filling: bool = Input.is_action_pressed(A.act("fill"))
	world.dig_held = digging or filling
	if dig_cd <= 0.0 and (digging or filling):
		# Back-pressure: if remesh is behind, bite slower instead of stacking hitch.
		var backlog: int = world.remesh_backlog()
		_edit(not filling)
		dig_cd = 0.28 if backlog >= 4 else (0.20 if backlog >= 2 else 0.16)

func _refresh_aim() -> void:
	var hit: Dictionary = aim()
	last_aim = hit
	can_dig = true
	if hit.get("hit", false):
		var pr: Dictionary = world.terrain.probe(hit["point"])
		can_dig = bool(pr.get("diggable", true))
		if hit.get("vegetation", false):
			can_dig = true
			last_aim["kind"] = hit.get("kind", "timber")
		else:
			last_aim["kind"] = pr.get("kind", "")
	# Heap under your feet, read here rather than from the reticle's _draw:
	# the reticle redraws every frame, and this walks the whole heap list.
	last_aim["heap"] = world.terrain.heap_in_reach(theta, z, 3.5)
	if highlight:
		var show_brush: bool = hit.get("hit", false) and view == 0 \
			and not RamaControls.photo_mode
		highlight.visible = show_brush
		if show_brush:
			# Show the brush's actual shape: a sphere, or the disc the
			# levelling brush cuts. A sphere marker on a disc brush is a lie.
			#
			# On vegetation it also has to show the bite that will actually
			# come away — wood resists, so the axe takes a fraction of the
			# brush — centred in the block rather than on the face the ray
			# struck. Drawing the full sphere on the skin of a trunk promised
			# an armful and delivered a notch.
			var veg: bool = hit.get("vegetation", false)
			var pt: Vector3 = hit.get("bite_at", hit["point"]) if veg else hit["point"]
			var r: float = brush * (float(hit.get("bite_scale", 1.0)) if veg else 1.0)
			var radial := Vector3(-pt.x, -pt.y, 0.0).normalized()
			var ax := Vector3(0, 0, 1)
			var rt := radial.cross(ax).normalized()
			highlight.transform = Transform3D(Basis(rt, radial, ax), pt)
			highlight.scale = Vector3(r, 0.22 if level_brush and not veg else r, r)
	if ghost:
		# Only while you are actually placing. This had no mode gate at all, so
		# the module ghost — a 9 x 9 m farm bed by default — sat translucent
		# over the middle of the screen the entire game, on top of the dig
		# brush sphere. Two big overlays permanently in the sight line.
		var arming: bool = Input.is_action_pressed(RamaControls.act("place")) \
			or module_shown > 0.0
		var show_ghost: bool = arming and hit.get("hit", false) and view == 0 \
			and not RamaControls.photo_mode
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

	cam_dist = lerpf(cam_dist, cam_dist_target, 1.0 if RamaControls.reduced_motion else 0.28)
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

	# Soft follow only while walking. Looking snaps the orbit so turns aren't
	# fighting a 0.28 lerp (that read as stick friction).
	if not cam_ready or _look_dirty or RamaControls.reduced_motion or first_person:
		cam_target = want
		cam_ready = true
		_look_dirty = false
	else:
		cam_target = cam_target.lerp(want, 0.42)
	# Orientation always matches look — never lerp the basis.
	cam.transform = Transform3D(Basis(right, true_up, -f), cam_target)

	if body:
		body.visible = not first_person
		var bp := feet_pos()
		var bf := heading()
		# +Z is the way he faces — the rig has a nose on it now, and the old
		# basis was left-handed, so he walked backwards looking at you.
		body.transform = Transform3D(
				Basis(u.cross(bf).normalized(), u, bf), bp)

## Procedural walk cycle. Nothing is keyframed — the gait phase advances with
## distance travelled, so speed and animation cannot drift apart.
func animate(dt: float, speed: float) -> void:
	if rig.is_empty():
		return
	gait = RamaGait.advance(gait, gait_spec, float(body_spec["stature"]), speed, dt)
	gait = wrapf(gait, 0.0, TAU)
	RamaGait.pose(rig, gait_spec, gait, speed,
			float(Time.get_ticks_msec()) * 0.001, pitch)