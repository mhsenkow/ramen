extends Control
## Horizon-style cast compass — a thin strip at the top of the view that shows
## where the romanceable colonists sit relative to your facing yaw.
##
## Marks slide left/right by relative bearing. Only the forward hemisphere
## (±90°) is drawn on the bar; targets behind you sit as edge chevrons so you
## still know which way to turn. The mark nearest your look-axis grows a
## distance readout.

const FOV_DEG := 90.0
const BAR_H := 28.0
const ICON_R := 9.0

## Each mark: { "id": int, "name": String, "rel": float degrees, "dist": float metres }
var marks: Array = []
var _font_mul := 1.0

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	custom_minimum_size = Vector2(320, BAR_H + 18)

func set_font_scale(mul: float) -> void:
	_font_mul = clampf(mul, 0.85, 1.4)
	custom_minimum_size = Vector2(320.0 * _font_mul, (BAR_H + 18.0) * _font_mul)
	queue_redraw()

func set_marks(next: Array) -> void:
	marks = next
	queue_redraw()

func _draw() -> void:
	var w: float = size.x
	var h: float = size.y
	if w < 40.0:
		return
	var cx: float = w * 0.5
	var cy: float = 14.0 * _font_mul
	var half: float = w * 0.48

	# Soft bar.
	var bar := Rect2(cx - half, cy - 3.0 * _font_mul, half * 2.0, 6.0 * _font_mul)
	draw_rect(bar, Color(0.04, 0.06, 0.08, 0.72), true)
	draw_rect(bar, Color(0.78, 0.84, 0.90, 0.35), false, 1.0)

	# Forward tick.
	draw_line(
			Vector2(cx, cy - 9.0 * _font_mul),
			Vector2(cx, cy + 9.0 * _font_mul),
			Color(0.95, 0.92, 0.82, 0.95),
			1.6)

	# Pick the mark closest to look-axis for the distance callout.
	var focus := -1
	var focus_abs := 999.0
	for i in marks.size():
		var rel: float = absf(float(marks[i].get("rel", 999.0)))
		if rel < focus_abs:
			focus_abs = rel
			focus = i

	for i in marks.size():
		var m: Dictionary = marks[i]
		var rel: float = float(m.get("rel", 0.0))
		var mark_name: String = str(m.get("name", "?"))
		var dist: float = float(m.get("dist", 0.0))
		var col: Color = m.get("color", Color(0.65, 0.82, 1.0))
		var letter: String = str(m.get("letter", ""))
		if letter == "":
			letter = mark_name.substr(0, 1).to_upper() if mark_name.length() > 0 else "?"

		var on_bar: bool = absf(rel) <= FOV_DEG
		var x: float
		if on_bar:
			x = cx + (rel / FOV_DEG) * half
		else:
			# Behind / off-axis: pin to the nearer edge as a turn hint.
			x = cx + (FOV_DEG if rel > 0.0 else -FOV_DEG) / FOV_DEG * half

		var r: float = ICON_R * _font_mul
		if i == focus and on_bar:
			r *= 1.25
		var alpha: float = 1.0 if on_bar else 0.55
		var fill := Color(col.r, col.g, col.b, alpha)
		var ink := Color(0.06, 0.07, 0.09, alpha)

		draw_circle(Vector2(x, cy), r + 1.5 * _font_mul, Color(0.04, 0.05, 0.07, 0.85 * alpha))
		draw_circle(Vector2(x, cy), r, fill)
		# Initial.
		var fs: int = roundi(11.0 * _font_mul)
		draw_string(
				ThemeDB.fallback_font,
				Vector2(x - fs * 0.28, cy + fs * 0.35),
				letter,
				HORIZONTAL_ALIGNMENT_LEFT,
				-1,
				fs,
				ink)

		if not on_bar:
			# Edge chevron pointing off-bar.
			var dir: float = 1.0 if rel > 0.0 else -1.0
			var tip := Vector2(x + dir * (r + 5.0 * _font_mul), cy)
			draw_colored_polygon(
					PackedVector2Array([
						tip,
						tip + Vector2(-dir * 5.0, -4.0) * _font_mul,
						tip + Vector2(-dir * 5.0, 4.0) * _font_mul,
					]),
					fill)

		if i == focus and on_bar:
			var label: String = "%s · %dm" % [mark_name, int(dist)]
			if dist < 25.0:
				label = "%s · here" % mark_name
			var lfs: int = roundi(11.0 * _font_mul)
			var tw: float = ThemeDB.fallback_font.get_string_size(label, HORIZONTAL_ALIGNMENT_LEFT, -1, lfs).x
			var lx: float = clampf(x - tw * 0.5, 4.0, w - tw - 4.0)
			var ly: float = cy + r + 12.0 * _font_mul
			draw_string(
					ThemeDB.fallback_font,
					Vector2(lx, ly),
					label,
					HORIZONTAL_ALIGNMENT_LEFT,
					-1,
					lfs,
					Color(0.94, 0.93, 0.86, 0.95))
