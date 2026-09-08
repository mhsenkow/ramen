extends Node
## Visual bisect harness (LANDSCAPE_2200 §2051–2057).
##
## Hides node groups one at a time in a single run and prints a text verdict.
## Finds *what* draws an artifact, never *why* (§2085).
##
## Metrics:
##   bright  — fraction of pixels above a luminance threshold (white blocks)
##   var     — luminance variance in the calibrated ROI (low-contrast patterns)
##
## Usage: Godot --path game -- --bisect
## Optional: --bisect-metric=bright|var

const GROUPS := [
	{"name": "FarField", "path": "FarField"},
	{"name": "MidField", "path": "MidField"},
	{"name": "Grass", "path": "Grass"},
	{"name": "ShoreFoam", "path": "ShoreFoam"},
	{"name": "Clouds", "path": "Clouds"},
	{"name": "Dust", "path": "Dust"},
	{"name": "DawnSteam", "path": "DawnSteam"},
	{"name": "WaterSheet", "path": "Water"},
	{"name": "Endcaps", "path": "Endcaps"},
	{"name": "Pools", "path": "Pools"},
	{"name": "Rivers", "path": "Rivers"},
	{"name": "PlantsNear", "path": "PlantsNear"},
	{"name": "PlantsMid", "path": "PlantsMid"},
	{"name": "PlantsFar", "path": "PlantsFar"},
	{"name": "Stations", "path": "Stations"},
	{"name": "Agents", "path": "Agents"},
	{"name": "ColonistPlots", "path": "ColonistPlots"},
	{"name": "Homestead", "path": "Homestead"},
	{"name": "Towns", "path": "Towns"},
	{"name": "HUD", "kind": "hud"},
]

var world: Node

func run() -> void:
	var args := OS.get_cmdline_user_args()
	var metric := "bright"
	for a in args:
		if a.begins_with("--bisect-metric="):
			metric = a.split("=")[1]
	print("\n================ VISUAL BISECT (%s) ================" % metric)
	# Skip wake fade — same contract as --shot (§2058).
	if world.player and world.player.get("wake_t") != null:
		world.player.wake_t = 99.0
		if world.player.get("fade"):
			world.player.fade.visible = false
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw

	var base_img: Image = world.get_viewport().get_texture().get_image()
	var roi := _calibrate_roi(base_img, metric)
	var base := _measure(base_img, metric, roi)
	print("baseline %s = %.6f  roi=%s" % [metric, base, str(roi)])
	if base <= 0.000001 and metric == "bright":
		print("WARN: baseline is zero — metric may be wrong for this artifact (§2056)")
	elif base <= 0.000001:
		print("WARN: baseline is zero — variance ROI may miss the pattern (§2056)")

	var best_name := ""
	var best_drop := 0.0
	var results: Array = []
	for g in GROUPS:
		var node: Node = _resolve(g)
		if node == null:
			results.append({"name": g["name"], "skip": true})
			continue
		var was: bool = node.visible
		node.visible = false
		await RenderingServer.frame_post_draw
		await RenderingServer.frame_post_draw
		var img: Image = world.get_viewport().get_texture().get_image()
		var v := _measure(img, metric, roi)
		var drop: float = base - v
		results.append({"name": g["name"], "value": v, "drop": drop})
		print("  hide %-14s  %s=%.6f  Δ=%+.6f" % [g["name"], metric, v, drop])
		node.visible = was
		if drop > best_drop:
			best_drop = drop
			best_name = g["name"]

	print("------------------------------------------------")
	if best_name == "":
		print("VERDICT: no hide reduced the metric — artifact may be env/post/clear colour")
	else:
		print("VERDICT: strongest reduction from hiding '%s' (Δ=%.6f)" % [best_name, best_drop])
		print("         This names WHAT draws it, not WHY (§2085).")
	print("================================================\n")
	world.get_tree().quit()

func _resolve(g: Dictionary) -> Node:
	if g.get("kind", "") == "hud":
		return world.get_node_or_null("HUD") if world.has_node("HUD") else world.get("hud_panels")
	var p: String = g.get("path", g["name"])
	var n: Node = world.get_node_or_null(p)
	if n:
		return n
	# Fuzzy: first child whose name contains the path token.
	for c in world.get_children():
		if p.to_lower() in String(c.name).to_lower():
			return c
	return null

func _calibrate_roi(img: Image, metric: String) -> Rect2i:
	## Find the brightest / noisiest region automatically (§2055).
	var w: int = img.get_width()
	var h: int = img.get_height()
	var bw: int = maxi(w / 8, 32)
	var bh: int = maxi(h / 8, 32)
	var best := Rect2i(0, 0, bw, bh)
	var best_score := -1.0
	var y := 0
	while y + bh <= h:
		var x := 0
		while x + bw <= w:
			var r := Rect2i(x, y, bw, bh)
			var s := _measure(img, metric, r)
			if s > best_score:
				best_score = s
				best = r
			x += bw
		y += bh
	return best

func _measure(img: Image, metric: String, roi: Rect2i) -> float:
	if metric == "var":
		return _luminance_variance(img, roi)
	return _bright_fraction(img, roi, 0.92)

func _luma(c: Color) -> float:
	return c.r * 0.299 + c.g * 0.587 + c.b * 0.114

func _bright_fraction(img: Image, roi: Rect2i, thresh: float) -> float:
	var n := 0
	var hit := 0
	for y in range(roi.position.y, roi.position.y + roi.size.y):
		for x in range(roi.position.x, roi.position.x + roi.size.x):
			if _luma(img.get_pixel(x, y)) >= thresh:
				hit += 1
			n += 1
	return float(hit) / float(maxi(n, 1))

func _luminance_variance(img: Image, roi: Rect2i) -> float:
	var n := 0
	var sum := 0.0
	var sum2 := 0.0
	for y in range(roi.position.y, roi.position.y + roi.size.y):
		for x in range(roi.position.x, roi.position.x + roi.size.x):
			var L := _luma(img.get_pixel(x, y))
			sum += L
			sum2 += L * L
			n += 1
	if n < 2:
		return 0.0
	var mean := sum / float(n)
	return maxf(sum2 / float(n) - mean * mean, 0.0)
