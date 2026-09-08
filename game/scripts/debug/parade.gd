extends Node
## Build-and-gait contact sheet (`--parade`).
##
## A still cannot show a walk, so this shows the walk as a strip: every
## archetype at a different point in its own cycle, side on and then head on.
## Read it like a rotoscope — if two builds hold the same pose at different
## phases, their gaits are not actually different and one of them is lying.

const RamaBody = preload("res://scripts/avatar/body.gd")
const RamaGait = preload("res://scripts/avatar/gait.gd")
const RamaControls = preload("res://scripts/controls.gd")

var world: Node
var _rigs: Array = []
var _cam: Camera3D

var _z0 := 0.0

func run() -> void:
	# Same contract as `--shot`: no wake fade, no instruments in the frame.
	RamaControls.photo_mode = true
	if world.player:
		world.player.wake_t = 99.0
		world.player.woke = true
		if world.player.fade:
			world.player.fade.visible = false
		world.player.body.visible = false
		# Take him out of the tree entirely. `_update_camera` runs every physics
		# frame and re-shows the body, so a `visible = false` before the capture
		# is a race the capture loses.
		if world.player.body.get_parent():
			world.player.body.get_parent().remove_child(world.player.body)
	if world.hud_panels:
		world.hud_panels.visible = false
	if world.reticle:
		world.reticle.visible = false
	if world.mini_overlay:
		world.mini_overlay.visible = false
	if world.soil_overlay:
		world.soil_overlay.visible = false
	if world.ship_overlay:
		world.ship_overlay.visible = false
	# The colony walks around; a contact sheet should hold still.
	if world.agent_root:
		world.agent_root.visible = false

	var th: float = world.player.theta
	var z0: float = world.player.z
	_z0 = z0
	var R: float = float(world.P["radius"])
	var order: Array = RamaBody.ORDER
	# A line along the drum axis, spaced so nobody overlaps a bear.
	for i in order.size():
		var arch: String = order[i]
		var holder := Node3D.new()
		world.add_child(holder)
		var spec: Dictionary = RamaBody.make(arch)
		var rig: Dictionary = RamaBody.build(holder, spec)
		var zz: float = z0 + (float(i) - (order.size() - 1) * 0.5) * 1.5
		var gr: float = world.ground_at(th, zz)
		holder.transform = world.frame_at(th, zz, gr)
		_rigs.append({"node": holder, "rig": rig, "spec": spec, "base": holder.transform,
				"gait": RamaGait.make(arch), "arch": arch})

	# Gait sheet: each at a different phase. Build sheet: all at the same one,
	# so the comparison is of shapes and not of moments.
	await _sheet("14_builds_side", 0.0, 9.0, 1.15, false)
	await _sheet("15_builds_front", PI * 0.5, 9.5, 1.15, true)
	# Same man, whole cycle: eight frames of one archetype, so a single gait can
	# be read end to end rather than guessed at from one pose.
	for arch in ["jock", "bear", "twink", "lanky"]:
		await _cycle_sheet(arch, th, z0)
	print("[rama] parade written")
	world.get_tree().quit()

func _pose_all(phase_offset: float, same_phase := false) -> void:
	for i in _rigs.size():
		var e: Dictionary = _rigs[i]
		var g: Dictionary = e["gait"]
		# Each at his own comfortable pace, and each at a different phase, so
		# one strip shows eight builds AND eight moments of the cycle.
		var ph: float = phase_offset + (0.0 if same_phase
				else float(i) / float(_rigs.size()) * TAU)
		RamaGait.pose(e["rig"], g, ph, float(g["walk_speed"]), 0.0)

## `face` turns the FIGURES, not the camera — spinning the camera round a line
## of men just looks down the line.
func _sheet(name: String, face: float, dist: float, height: float,
		same_phase := false) -> void:
	_pose_all(0.0, same_phase)
	for e in _rigs:
		e["node"].visible = true
		var base: Transform3D = e["base"]
		e["node"].transform = Transform3D(base.basis.rotated(base.basis.y, face), base.origin)
	await _shoot(name, 0.0, dist, height, _z0)

func _cycle_sheet(arch: String, th: float, z0: float) -> void:
	# Hide everyone but one man, then walk him through the cycle in place.
	var pick: Dictionary = {}
	for e in _rigs:
		e["node"].visible = str(e["arch"]) == arch
		if str(e["arch"]) == arch:
			pick = e
	if pick.is_empty():
		return
	var clones: Array = []
	var g: Dictionary = pick["gait"]
	for i in 8:
		var holder := Node3D.new()
		world.add_child(holder)
		var rig: Dictionary = RamaBody.build(holder, pick["spec"])
		var zz: float = z0 + (float(i) - 3.5) * 1.5
		holder.transform = world.frame_at(th, zz, world.ground_at(th, zz))
		RamaGait.pose(rig, g, float(i) / 8.0 * TAU, float(g["walk_speed"]), 0.0)
		clones.append(holder)
	pick["node"].visible = false
	await _shoot("16_cycle_%s" % arch, 0.0, 9.5, 1.15, z0)
	for c in clones:
		c.queue_free()

## The sheet gets its OWN camera.
##
## Borrowing the player's meant borrowing his body with it: `_update_camera`
## re-shows the figure every physics frame, so he kept walking into the middle
## of the line at twice everyone else's apparent size.
func _shoot(name: String, _yaw_off: float, dist: float, height: float, z_at: float) -> void:
	if _cam == null:
		_cam = Camera3D.new()
		_cam.fov = 52.0
		_cam.near = 0.05
		_cam.far = world.drum_diagonal() * 1.15
		world.add_child(_cam)
	# He stays put: the near chunk ring follows him, and walking him away took
	# the ground with it.
	world.player.z = z_at
	world.player.r = world.ground_at(world.player.theta, world.player.z)

	var th: float = world.player.theta
	var frame: Transform3D = world.frame_at(th, z_at, world.ground_at(th, z_at))
	var up: Vector3 = frame.basis.y
	var tang: Vector3 = frame.basis.x
	var target: Vector3 = frame.origin + up * (height * 0.82)
	_cam.current = true
	_cam.look_at_from_position(target - tang * dist + up * 0.30, target, up)
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw
	var img: Image = world.get_viewport().get_texture().get_image()
	img.save_png("res://../shots/%s.png" % name)
	print("[rama] parade shot %s" % name)
