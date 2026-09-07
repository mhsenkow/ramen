extends Control
## Markers drawn over the habitat map: where you are on the unrolled drum, and
## which way you are facing. Kept as a separate pass so the map texture itself
## stays a pure read of the terrain.

var world
var show_frame := true

func _draw() -> void:
	if world == null or world.player == null:
		return
	var s := size
	if show_frame:
		draw_rect(Rect2(Vector2.ZERO, s), Color(0, 0, 0, 0.30), true)
		draw_rect(Rect2(Vector2.ZERO, s), Color(0.75, 0.82, 0.86, 0.45), false, 1.0)
		# Faint grid lines at z-axis thirds for spatial reference.
		for i in [1, 2]:
			var gy: float = s.y * float(i) / 3.0
			draw_line(Vector2(0, gy), Vector2(s.x, gy), Color(0.65, 0.72, 0.78, 0.18), 1.0)
	var L: float = float(world.P["length"])
	var u: float = fposmod(world.player.theta, TAU) / TAU
	var v: float = clamp(world.player.z / L + 0.5, 0.0, 1.0)
	var p := Vector2(u * s.x, v * s.y)

	# Settlements, so the map shows somewhere to go.
	for m in world.town_marks:
		var mu: float = fposmod(m.x, TAU) / TAU
		var mv: float = clamp(m.y / L + 0.5, 0.0, 1.0)
		draw_circle(Vector2(mu * s.x, mv * s.y), 2.0, Color(1.0, 0.84, 0.52, 0.85))

	if world.has_waypoint:
		var wu: float = fposmod(world.waypoint.x, TAU) / TAU
		var wv: float = clamp(world.waypoint.y / L + 0.5, 0.0, 1.0)
		var wp := Vector2(wu * s.x, wv * s.y)
		draw_line(wp + Vector2(-5, -5), wp + Vector2(5, 5), Color(0.4, 1.0, 0.7, 0.95), 1.6)
		draw_line(wp + Vector2(-5, 5), wp + Vector2(5, -5), Color(0.4, 1.0, 0.7, 0.95), 1.6)

	# Facing wedge, then the marker itself.
	var yaw: float = world.player.yaw
	var dir := Vector2(sin(yaw), cos(yaw))
	draw_line(p, p + dir * 11.0, Color(1, 1, 1, 0.9), 1.5)
	draw_circle(p, 3.4, Color(0.05, 0.06, 0.08, 0.9))
	draw_circle(p, 2.2, Color(1.0, 0.98, 0.92, 1.0))
