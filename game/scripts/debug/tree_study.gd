extends Node
## Reproducible tree material / distance study. Does not save the user's world.
## Godot --path game -- --shot --tree-study --shot-dir=/tmp/rama-trees
const Controls = preload("res://scripts/controls.gd")
var world
var output := "/tmp/rama-trees"

func run() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--shot-dir="):
			output = arg.trim_prefix("--shot-dir=")
	DirAccess.make_dir_recursive_absolute(output)
	Controls.photo_mode = true
	world.player.set_physics_process(false)
	world.player.body.visible = false
	world.reticle.visible = false
	world.hud_panels.visible = false
	world.post_layer.visible = false
	if world.ui: world.ui.visible = false
	var data: PackedFloat32Array = world.terrain.plants_lod(world.player.theta, world.player.z, 900)
	var picked := -1
	for i in int(data.size() / 9.0):
		var form: int = int(data[i*9+7])
		if form in [0,1,2,6,7]:
			picked = i
			break
	if picked < 0:
		push_error("Tree study: no canopy tree found")
		get_tree().quit(1)
		return
	var th: float = data[picked*9]
	var zz: float = data[picked*9+1]
	var base: Vector3 = world.to_world(th, zz, world.terrain.ground_radius(th,zz))
	var up: Vector3 = world.up_at(base)
	var side: Vector3 = up.cross(Vector3(0,0,1)).normalized()
	var target := base + up * 9.0
	world.player.theta = th
	world.player.z = zz
	world.player.r = Vector2(base.x,base.y).length()
	world._queue_chunks()
	world._pump_chunks(400)
	world.refresh_mid(true)
	world.refresh_grass(true)
	var costs: Array = []
	for i in 12:
		var started := Time.get_ticks_usec()
		world.refresh_woodscape(true)
		costs.append((Time.get_ticks_usec()-started)/1000.0)
	world.set_process(false)
	if world.ui: world.ui.visible = false
	world.player.highlight.visible = false
	world.player.ghost.visible = false
	print("[tree-study] build ms ", costs)
	for distance in [28.0, 65.0, 82.0, 110.0, 220.0]:
		world.player.cam.global_position = target + side * distance + Vector3(0,0,-distance*0.22) + up * 2.0
		world.player.cam.look_at(target, up)
		await _capture("tree_%03d" % int(distance))
	# Same camera, independent representations: directly exposes silhouette drift.
	world.player.cam.global_position = target + side * 70.0 + up * 2.0
	world.player.cam.look_at(target, up)
	world.woodscape_mm.material_override.set_shader_parameter("fade_start", 500.0)
	world.woodscape_mm.material_override.set_shader_parameter("fade_end", 600.0)
	for layer in world.plant_species_near: layer.visible = false
	await _capture("tree_material")
	world.woodscape_mm.visible = false
	for layer in world.plant_species_near:
		layer.visible = true
		layer.material_override.set_shader_parameter("proxy", false)
	await _capture("tree_proxy")
	# Measure the unchanged path separately from initial population.
	var start := Time.get_ticks_usec()
	for i in 30: world.terrain.woodscape_mesh(world.player.feet_pos(),98.0,world.woodscape_revision)
	print("[tree-study] unchanged update avg ms ", (Time.get_ticks_usec()-start)/30000.0)
	print("[tree-study] blocks ",world.terrain.woodscape_count()," material triangles ",world.woodscape_mm.mesh.surface_get_array_index_len(0) / 3)
	get_tree().quit()

func _capture(label: String) -> void:
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	img.save_png(output.path_join(label+".png"))
	print("[tree-study] ",label)
