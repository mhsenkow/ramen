extends Control
## Centre reticle. Catchment ribbon lives on the terrain; HUD dots stay light.

var world

func _draw() -> void:
	if world == null or world.player == null:
		return
	var p = world.player
	if p.view != 0:
		return
	var c := size * 0.5
	var aim: Dictionary = p.last_aim
	var live: bool = aim.get("hit", false)
	var col := Color(1, 1, 1, 0.55)
	if live:
		col = Color(1.0, 0.86, 0.45, 0.95) if p.can_dig else Color(1.0, 0.42, 0.36, 0.95)

	# Dark shadow outline behind each arm for readability against any background.
	var shadow := Color(0.0, 0.0, 0.0, 0.45)
	for a in [Vector2(1, 0), Vector2(-1, 0), Vector2(0, 1), Vector2(0, -1)]:
		draw_line(c + a * 4.5, c + a * 12.5, shadow, 2.5)
	for a in [Vector2(1, 0), Vector2(-1, 0), Vector2(0, 1), Vector2(0, -1)]:
		draw_line(c + a * 5.0, c + a * 12.0, col, 1.0)
	draw_circle(c, 1.4, col)

	# Brush radius ring — see the excavation footprint without reading the number.
	if live and p.brush > 0.5:
		var ring_r: float = clampf(p.brush * 3.2, 4.0, 24.0)
		draw_arc(c, ring_r, 0.0, TAU, 32, Color(col.r, col.g, col.b, 0.22), 1.0)

	if live:
		var pt: Vector3 = aim.get("point", Vector3.ZERO)
		world.refresh_catchment_at(atan2(pt.y, pt.x), pt.z)

	if not live:
		return
	var kind: String = str(aim.get("kind", ""))
	var dist: float = float(aim.get("distance", 0.0))
	var txt := "%s   %.1f m" % [kind, dist]
	if not p.can_dig:
		txt += "   (cannot excavate)"
	# Spoil / timber heaps at your feet — L takes them into the pack. Sampled
	# by _refresh_aim; _draw must stay off the sim's FFI surface.
	var heap: Dictionary = aim.get("heap", {})
	if heap.get("ok", false):
		txt += "   ·  L take %.0f kg %s" % [float(heap.get("kg", 0.0)), str(heap.get("material", "?"))]
	var f := ThemeDB.fallback_font
	var w := f.get_string_size(txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 12).x
	# Outline for readability.
	draw_string(f, c + Vector2(-w * 0.5 + 1, 35), txt,
		HORIZONTAL_ALIGNMENT_LEFT, -1, 12, Color(0, 0, 0, 0.6))
	draw_string(f, c + Vector2(-w * 0.5, 34), txt,
		HORIZONTAL_ALIGNMENT_LEFT, -1, 12, Color(0.95, 0.97, 1.0, 0.92))
	var b := "brush %.1f m" % p.brush
	var bw := f.get_string_size(b, HORIZONTAL_ALIGNMENT_LEFT, -1, 11).x
	draw_string(f, c + Vector2(-bw * 0.5, 50), b,
		HORIZONTAL_ALIGNMENT_LEFT, -1, 11, Color(0.85, 0.90, 0.95, 0.70))

