extends Control
## Crosshair, heading and scale over the live plan view.
## Soil overlay (organic/N/moisture) sits under this as a TextureRect.

var world

func _draw() -> void:
	if world == null or world.player == null:
		return
	var s := size
	var c := s * 0.5
	draw_rect(Rect2(Vector2.ZERO, s), Color(0.75, 0.82, 0.86, 0.45), false, 1.0)
	draw_line(Vector2(c.x, 6), Vector2(c.x, 14), Color(1, 1, 1, 0.5), 1.0)
	draw_string(ThemeDB.fallback_font, Vector2(c.x - 4, 22), "N",
		HORIZONTAL_ALIGNMENT_LEFT, -1, 11, Color(1, 1, 1, 0.7))
	var yaw: float = world.player.yaw
	var dir := Vector2(sin(yaw), -cos(yaw))
	draw_line(c, c + dir * 15.0, Color(1.0, 0.95, 0.6, 0.95), 2.0)
	draw_circle(c, 3.0, Color(1.0, 0.98, 0.92, 1.0))
	# Scale bar rather than a bare number: a quarter of the view, labelled.
	var bar: float = s.x / 4.0
	var by: float = s.y - 10.0
	draw_line(Vector2(8, by), Vector2(8 + bar, by), Color(1, 1, 1, 0.55), 1.0)
	draw_line(Vector2(8, by - 3), Vector2(8, by + 3), Color(1, 1, 1, 0.55), 1.0)
	draw_line(Vector2(8 + bar, by - 3), Vector2(8 + bar, by + 3), Color(1, 1, 1, 0.55), 1.0)
	draw_string(ThemeDB.fallback_font, Vector2(12, by - 5),
		"%d m" % int(world.mini_size / 4.0),
		HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color(1, 1, 1, 0.6))
	draw_string(ThemeDB.fallback_font, Vector2(s.x - 92, s.y - 8),
		"+/- zoom · , . size",
		HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color(0.85, 0.92, 0.95, 0.45))
	draw_string(ThemeDB.fallback_font, Vector2(8, 14),
		"soil · V cycles",
		HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color(0.85, 0.92, 0.95, 0.55))
	if world.soil_mode == 1:
		draw_string(ThemeDB.fallback_font, Vector2(8, 28), "organic",
			HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color(0.95, 0.55, 0.35, 0.7))
	elif world.soil_mode == 2:
		draw_string(ThemeDB.fallback_font, Vector2(8, 28), "nitrogen",
			HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color(0.45, 0.9, 0.5, 0.7))
	elif world.soil_mode == 3:
		draw_string(ThemeDB.fallback_font, Vector2(8, 28), "moisture",
			HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color(0.4, 0.7, 1.0, 0.7))
	else:
		draw_string(ThemeDB.fallback_font, Vector2(8, 28), "org · N · wet",
			HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color(0.85, 0.92, 0.95, 0.55))
