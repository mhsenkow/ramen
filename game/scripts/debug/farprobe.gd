extends Node
## Far-field term tap + void coverage gate.
## NO material override, NO render_mode change, NO fog change — only the `dbg`
## uniform already compiled into terrain.gdshader.
##
## Magenta clear coverage fails hard (≥5% empty) so massif radial-band holes
## and winding regressions cannot ship silently (LANDSCAPE_4200 §BW / 4135).

var world: Node

func run() -> void:
	print("\n================ FAR TERM TAP ================")
	if world.player and world.player.get("wake_t") != null:
		world.player.wake_t = 99.0
		if world.player.get("fade"):
			world.player.fade.visible = false
	world.player.theta = 3.063
	world.player.z = 2724.0
	world.player.r = world.ground_at(world.player.theta, world.player.z)
	world.player.vr = 0.0
	world.player.pitch = 0.30
	# Long axis is yaw PI — yaw 0 faces the near endcap ~274 m away and filled
	# every previous tap image, making dbg reads look identical.
	world.player.yaw = PI
	world.terrain.set_player_pos(world.player.theta, world.player.z)
	world._queue_chunks()
	world._pump_chunks(400)
	world.refresh_mid(true)
	world.player._update_camera()
	var ui = world.get("ui")
	if ui:
		ui.visible = false
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw

	var ff: Node = world.get_node_or_null("FarField")
	var taps := {0: "final", 7: "albedo", 3: "lit", 4: "ndl", 6: "hazef"}
	for k in taps:
		_tap(ff, k)
		await RenderingServer.frame_post_draw
		await RenderingServer.frame_post_draw
		var im: Image = world.get_viewport().get_texture().get_image()
		im.save_png("/Users/powerox/ramen/shots/tap_%s.png" % taps[k])
		print("  tap %s" % taps[k])
	_tap(ff, 0)

	var void_frac: float = await _void_coverage()
	print("far void coverage   : %.1f%% empty — %s" % [
		void_frac * 100.0, ("PASS" if void_frac < 0.05 else "FAIL")])
	print("=============================================\n")
	if void_frac >= 0.05:
		push_error("farprobe void %.1f%% — massif radial band / winding regression" % (void_frac * 100.0))
		world.get_tree().quit(1)
	else:
		world.get_tree().quit(0)

## Magenta clear-colour coverage at the long-axis vantage (same as selftest).
func _void_coverage() -> float:
	if world.player == null or world.player.cam == null:
		return 0.0
	var sun = world.get("rama_sun")
	var sun_was := false
	if sun:
		sun_was = sun.shadow_enabled
		sun.shadow_enabled = false
		sun.visible = false
	var cenv := Environment.new()
	cenv.background_mode = Environment.BG_COLOR
	cenv.background_color = Color(1.0, 0.0, 1.0)
	cenv.fog_enabled = false
	cenv.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	cenv.glow_enabled = false
	world.player.cam.environment = cenv
	var post = world.get("post_layer")
	var was_post := false
	if post:
		was_post = post.visible
		post.visible = false
	await world.get_tree().process_frame
	RenderingServer.force_draw()
	await world.get_tree().process_frame
	var img: Image = world.get_viewport().get_texture().get_image()
	world.player.cam.environment = null
	if post:
		post.visible = was_post
	if sun:
		sun.visible = true
		sun.shadow_enabled = sun_was
	if img == null:
		return 0.0
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
	return float(empty) / float(maxi(n, 1))

func _tap(n: Node, v: int) -> void:
	if n is GeometryInstance3D and n.material_override is ShaderMaterial:
		n.material_override.set_shader_parameter("dbg", v)
	for c in n.get_children():
		_tap(c, v)
