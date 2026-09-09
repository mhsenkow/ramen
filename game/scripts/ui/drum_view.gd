extends RefCounted
## A very un-detailed 3D read of the whole ship: the drum as an oblique tube,
## with you somewhere on the inside of it.
##
## Drawn with hand-rolled projection into an existing `_draw()` rather than a
## SubViewport with its own camera and meshes. The map is a schematic — a dozen
## ellipses and some straight lines — and a second 3D render pass to produce it
## would cost more than the entire rest of the HUD.
##
## ## Projection
##
## A cylinder seen obliquely: its circular section projects to an ellipse whose
## major axis is perpendicular to the drum axis and whose minor axis lies along
## it. So for a point at angle `th` and normalised axial position `zn`:
##
##     screen = centre + ax·(zn·half_len) + pp·(R·cos th) + ax·(R·SQUASH·sin th)
##
## `sin th` doubles as depth: positive is the near wall, negative the far one,
## which is all the hidden-line removal a schematic needs — far geometry is
## simply drawn first and dimmer.
##
## ## Why the axis is compressed
##
## Kepler Drum is 900 m in radius and 6000 m long. To scale that is a long thin
## pipe, unreadable at 300 px. `AXIS_SPAN` squashes it to roughly 2.6 : 1, which
## reads as a drum. This view is for "where am I on the ship", not for measuring
## it — the unrolled map is the one that keeps proportion.

const SQUASH := 0.34      ## Ellipse minor/major ratio — the viewing obliquity.
const AXIS_SPAN := 0.78   ## Fraction of the control's width the tube occupies.
const RIM := Color(0.78, 0.86, 0.92, 0.85)
const RIM_FAR := Color(0.55, 0.64, 0.72, 0.34)
const RIB := Color(0.62, 0.72, 0.80, 0.40)
const RIB_FAR := Color(0.50, 0.58, 0.66, 0.16)
const SKIN := Color(0.24, 0.31, 0.30, 0.42)

## Screen position of a point on the inner surface, and its depth.
## `th` radians around, `zn` in -0.5..0.5 along the axis.
static func project(size: Vector2, th: float, zn: float) -> Array:
	var c := size * 0.5
	# Axis runs across the control with a slight tilt so it reads as 3D.
	var ax := Vector2(0.9781, -0.2079)  # ~12 degrees
	var pp := Vector2(-ax.y, ax.x)
	var half_len: float = size.x * AXIS_SPAN * 0.5
	var r: float = minf(size.y * 0.34, size.x * 0.17)
	var p: Vector2 = c + ax * (zn * half_len * 2.0) \
			+ pp * (r * cos(th)) + ax * (r * SQUASH * sin(th))
	return [p, sin(th)]

static func _ring(ci: CanvasItem, size: Vector2, zn: float, col: Color,
		col_far: Color, width: float) -> void:
	var steps := 28
	var prev: Vector2 = project(size, 0.0, zn)[0]
	for i in range(1, steps + 1):
		var th: float = float(i) / float(steps) * TAU
		var got: Array = project(size, th, zn)
		var here: Vector2 = got[0]
		# Depth of the segment midpoint decides near or far.
		var mid: float = sin(th - PI / float(steps))
		ci.draw_line(prev, here, col if mid >= 0.0 else col_far, width)
		prev = here

## Draw the whole schematic. `world` supplies the player, towns and waypoint.
static func draw_ship(ci: CanvasItem, size: Vector2, world) -> void:
	if world == null or world.player == null:
		return
	var L: float = float(world.P["length"])
	var zn_of := func(z: float) -> float:
		return clampf(z / L, -0.5, 0.5)

	# Faint skin so the tube reads as a solid object, not a wire sketch. Two
	# quads along the near wall, which is all the fill an un-detailed view wants.
	var band := PackedVector2Array()
	for i in 13:
		band.append(project(size, PI * float(i) / 12.0, -0.5)[0])
	for i in 13:
		band.append(project(size, PI * float(12 - i) / 12.0, 0.5)[0])
	ci.draw_colored_polygon(band, SKIN)

	# Longitudinal ribs. Far ones first, so near ones draw over them.
	for pass_near in [false, true]:
		for i in 8:
			var th: float = float(i) / 8.0 * TAU
			var near: bool = sin(th) >= 0.0
			if near != pass_near:
				continue
			var a: Vector2 = project(size, th, -0.5)[0]
			var b: Vector2 = project(size, th, 0.5)[0]
			ci.draw_line(a, b, RIB if near else RIB_FAR, 1.0)

	# End caps and a couple of intermediate rings.
	for zn in [-0.5, -0.1667, 0.1667, 0.5]:
		var end: bool = absf(zn) > 0.4
		_ring(ci, size, zn, RIM if end else RIB, RIM_FAR if end else RIB_FAR,
				1.6 if end else 1.0)

	# Settlements.
	for m in world.town_marks:
		var got: Array = project(size, m.x, zn_of.call(m.y))
		var col := Color(1.0, 0.84, 0.52, 0.9 if got[1] >= 0.0 else 0.30)
		ci.draw_circle(got[0], 2.0, col)

	if world.has_waypoint:
		var w: Array = project(size, world.waypoint.x, zn_of.call(world.waypoint.y))
		var wp: Vector2 = w[0]
		var wc := Color(0.4, 1.0, 0.7, 0.95 if w[1] >= 0.0 else 0.35)
		ci.draw_line(wp + Vector2(-4, -4), wp + Vector2(4, 4), wc, 1.6)
		ci.draw_line(wp + Vector2(-4, 4), wp + Vector2(4, -4), wc, 1.6)

	# You. Drawn last so nothing hides it, but dimmed on the far wall so the
	# reading "I am round the back of the drum" survives.
	var me: Array = project(size, world.player.theta, zn_of.call(world.player.z))
	var mp: Vector2 = me[0]
	var front: bool = me[1] >= 0.0
	ci.draw_circle(mp, 4.2, Color(0.05, 0.06, 0.08, 0.9 if front else 0.5))
	ci.draw_circle(mp, 2.6, Color(1.0, 0.98, 0.92, 1.0 if front else 0.55))
	# A tick toward the axis: which way is "up" for you, on a wall.
	var inward: Array = project(size, world.player.theta, zn_of.call(world.player.z))
	var centre := size * 0.5
	var toward: Vector2 = (centre - inward[0]).normalized()
	ci.draw_line(mp, mp + toward * 7.0,
			Color(1.0, 0.95, 0.6, 0.9 if front else 0.4), 1.4)
