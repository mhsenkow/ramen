extends RefCounted
## Procedural blocky trees — Minecraft vocabulary, natural branching.
##
## Each form runs its own growth algorithm: apical leader, forking oak,
## weeping, columnar, open crown, deliquescent, spin-twisted, orchard.
## Wood and leaf are material keys (alpha 0 / 1) for `tree.gdshader`.
## Authored mesh height is ~TREE_H metres so instance scale stays near 1.
##
## Foliage is voxel-merged rather than stacked: leaf blocks land on one lattice
## per tree and only faces without a neighbour become triangles. A solid cube
## per leaf put the giant form at 79k triangles — three quarters of them
## interior faces no camera can reach — and the mid LOD tier draws hundreds of
## those at once. Merging also fuses crowns that overlap between branches
## instead of leaving them to z-fight.
##
## BLOCK_BUDGET then caps the recursive forms, so a deliquescent giant spends
## its branching on silhouette and stops rather than doubling all the way down.

const TREE_H := 14.0
const STEP := 0.55

## Wood blocks one mesh may emit before budding stops, at full detail. Leaf
## cells are cheap once merged, so only the branch recursion needs a ceiling.
const BLOCK_BUDGET := 460

## Leaf lattice edge at full detail. Both this and the budget scale with the
## LOD's `detail`, in opposite directions — a coarser tier wants FEWER, BIGGER
## blocks. Scaling the cell down alongside detail (as the pre-merge code did)
## silently made the mid tier finer than near, which face-merging then turned
## into the mid mesh costing more triangles than the near one.
const LEAF_CELL := 0.72

static func _leaf(shade: float) -> Color:
	return Color(shade, shade, shade, 1.0)

static func _wood(shade: float) -> Color:
	return Color(shade, shade, shade, 0.0)

static func _fruit(shade: float) -> Color:
	return Color(shade, shade, shade, 0.5)

static func mesh_for(kind: int, detail: float, seed_val: int = 0) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var rng: int = seed_val if seed_val != 0 else (kind * 7919 + 104729)
	# One lattice for every leaf cluster in this tree, so crowns that overlap
	# between branches merge into a single hull instead of stacking cubes.
	var lod: float = clampf(detail, 0.4, 1.0)
	var ctx := {
		"cell": LEAF_CELL / lod,
		"leaf": {},
		"wood": 0,
		"budget": maxi(48, roundi(float(BLOCK_BUDGET) * lod)),
	}
	match kind:
		0:
			_grow_excurrent(st, ctx, detail, rng)
		2:
			_grow_weeping(st, ctx, detail, rng)
		3:
			_grow_scrub(st, ctx, detail, rng)
		4:
			_grow_reed(st, ctx, detail, rng)
		5:
			_grow_orchard(st, ctx, detail, rng)
		6:
			if (rng & 1) == 0:
				_grow_twisted(st, ctx, detail, rng)
			else:
				_grow_open_crown(st, ctx, detail, rng)
		7:
			_grow_giant(st, ctx, detail, rng)
		_:
			_grow_decurrent(st, ctx, detail, rng)
	_emit_leaf_hull(st, ctx)
	st.generate_normals()
	return st.commit()

static func billboard_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	_add_prism(st, Vector3(0, 0.9, 0), Vector3(0.55, 1.8, 0.55), _wood(1.0))
	_add_prism(st, Vector3(0, 2.6, 0), Vector3(2.4, 2.8, 0.12), _leaf(0.95))
	_add_prism(st, Vector3(0, 3.4, 0), Vector3(1.6, 1.6, 0.12), _leaf(1.08))
	st.generate_normals()
	return st.commit()

# ----------------------------------------------------------------- primitives

static func _add_prism(st: SurfaceTool, center: Vector3, size: Vector3, col: Color) -> void:
	var hx := size.x * 0.5
	var hy := size.y * 0.5
	var hz := size.z * 0.5
	var c := center
	var corners := [
		c + Vector3(-hx, -hy, -hz), c + Vector3(hx, -hy, -hz),
		c + Vector3(hx, hy, -hz), c + Vector3(-hx, hy, -hz),
		c + Vector3(-hx, -hy, hz), c + Vector3(hx, -hy, hz),
		c + Vector3(hx, hy, hz), c + Vector3(-hx, hy, hz),
	]
	var faces := [
		[0, 1, 2, 0, 2, 3], [5, 4, 7, 5, 7, 6],
		[4, 0, 3, 4, 3, 7], [1, 5, 6, 1, 6, 2],
		[3, 2, 6, 3, 6, 7], [4, 5, 1, 4, 1, 0],
	]
	for f in faces:
		for idx in f:
			st.set_color(col)
			st.add_vertex(corners[idx])

static func _block(st: SurfaceTool, p: Vector3, edge: float, col: Color) -> void:
	_add_prism(st, p, Vector3(edge, edge, edge), col)

## A wood block, counted against the branching budget.
static func _timber(st: SurfaceTool, ctx: Dictionary, center: Vector3, size: Vector3,
		col: Color) -> void:
	ctx.wood += 1
	_add_prism(st, center, size, col)

static func _rand(rng: int) -> Array:
	var n: int = (rng * 1664525 + 1013904223) & 0x7fffffff
	return [n, float(n % 10000) / 10000.0]

# -------------------------------------------------------------- leaf lattice

## Record a leaf blob on this tree's shared lattice. Nothing is emitted here —
## `_emit_leaf_hull` turns the accumulated cells into a surface once the whole
## tree is grown, so overlapping clusters cost one shell between them.
static func _leaf_cluster(_st: SurfaceTool, ctx: Dictionary, at: Vector3, rad: float,
		_detail: float, rng: int) -> int:
	var r := rng
	var cell: float = ctx.cell
	var leaf: Dictionary = ctx.leaf
	var reach: int = maxi(1, int(round(rad / cell)))
	var base := Vector3i(roundi(at.x / cell), roundi(at.y / cell), roundi(at.z / cell))
	var lim: float = float(reach) + 0.2
	for ix in range(-reach, reach + 1):
		for iy in range(-reach, reach + 1):
			for iz in range(-reach, reach + 1):
				var d: float = sqrt(float(ix * ix + iy * iy + iz * iz))
				if d > lim:
					continue
				var roll: Array = _rand(r)
				r = int(roll[0])
				if float(roll[1]) < 0.18 and d > 0.6:
					continue
				# Brightest shade wins where crowns meet, so a merged cell
				# reads as canopy top rather than the deep shade of one lobe.
				var shade: float = 0.86 + float(iy + reach) * 0.05 + float(roll[1]) * 0.08
				var key := base + Vector3i(ix, iy, iz)
				if shade > float(leaf.get(key, 0.0)):
					leaf[key] = shade
	return r

## Faces of one lattice cell: axis neighbour, then four corners wound
## counter-clockwise seen from outside.
const _HULL_FACES := [
	[Vector3i(1, 0, 0), Vector3(1, -1, -1), Vector3(1, 1, -1), Vector3(1, 1, 1), Vector3(1, -1, 1)],
	[Vector3i(-1, 0, 0), Vector3(-1, -1, 1), Vector3(-1, 1, 1), Vector3(-1, 1, -1), Vector3(-1, -1, -1)],
	[Vector3i(0, 1, 0), Vector3(-1, 1, 1), Vector3(1, 1, 1), Vector3(1, 1, -1), Vector3(-1, 1, -1)],
	[Vector3i(0, -1, 0), Vector3(-1, -1, -1), Vector3(1, -1, -1), Vector3(1, -1, 1), Vector3(-1, -1, 1)],
	[Vector3i(0, 0, 1), Vector3(-1, -1, 1), Vector3(1, -1, 1), Vector3(1, 1, 1), Vector3(-1, 1, 1)],
	[Vector3i(0, 0, -1), Vector3(1, -1, -1), Vector3(-1, -1, -1), Vector3(-1, 1, -1), Vector3(1, 1, -1)],
]

## Emit only the lattice faces that have no neighbour — the canopy hull.
static func _emit_leaf_hull(st: SurfaceTool, ctx: Dictionary) -> void:
	var leaf: Dictionary = ctx.leaf
	if leaf.is_empty():
		return
	var cell: float = ctx.cell
	var h: float = cell * 0.48
	for key in leaf:
		var k: Vector3i = key
		var centre := Vector3(float(k.x), float(k.y), float(k.z)) * cell
		var col := _leaf(leaf[key])
		for face in _HULL_FACES:
			if leaf.has(k + face[0]):
				continue
			var quad := [
				centre + face[1] * h, centre + face[2] * h,
				centre + face[3] * h, centre + face[4] * h,
			]
			for idx in [0, 1, 2, 0, 2, 3]:
				st.set_color(col)
				st.add_vertex(quad[idx])

# ------------------------------------------------------------------ branches

## Walk a branch as stacked timber cubes; spawn laterals by algorithm.
static func _branch(
		st: SurfaceTool,
		ctx: Dictionary,
		origin: Vector3,
		dir: Vector3,
		length: float,
		thick: float,
		depth: int,
		algo: int,
		detail: float,
		rng: int,
		droop: float = 0.0
) -> int:
	var r := rng
	var d := dir.normalized()
	var steps: int = maxi(2, int(round(length / STEP)))
	var p := origin
	var t0 := thick
	for s in steps:
		var t: float = float(s) / float(steps)
		var thick_now: float = lerpf(t0, t0 * 0.55, t)
		d = (d + Vector3(0.0, -droop * 0.08, 0.0)).normalized()
		if algo == 6:
			var yaw: float = 0.18 * sin(t * TAU + float(depth))
			d = Basis(Vector3.UP, yaw) * d
		p += d * STEP
		_timber(st, ctx, p, Vector3(thick_now, thick_now * 1.05, thick_now),
				_wood(1.05 - float(depth) * 0.06))
		var bud := false
		var whorl_every: int = maxi(2, int(ceil(float(steps) / 4.0)))
		match algo:
			0:
				bud = depth < 3 and t > 0.35 and s % whorl_every == 0
			1:
				bud = depth < 4 and t > 0.25 and t < 0.85 and (s % 3 == 0)
			2:
				bud = depth < 3 and t > 0.4 and s % 2 == 0
			3:
				bud = depth < 2 and t > 0.5 and s % 4 == 0
			5:
				bud = depth < 5 and t > 0.15 and s % 2 == 0
			6:
				bud = depth < 4 and t > 0.3 and s % 3 == 0
			_:
				bud = depth < 3 and t > 0.4 and s % 3 == 0
		# The budget stops the heavily-forking forms once the silhouette reads.
		# Without it a deliquescent giant doubles to the depth limit and lands
		# at tens of thousands of triangles for every instance on screen.
		if bud and thick_now > 0.22 and int(ctx.wood) < int(ctx.budget):
			var roll: Array = _rand(r)
			r = int(roll[0])
			var ang: float = float(roll[1]) * TAU
			var elev: float = 0.15
			match algo:
				0:
					elev = 0.05 + float(roll[1]) * 0.15
				1:
					elev = 0.25 + float(roll[1]) * 0.35
				2:
					elev = -0.35 - float(roll[1]) * 0.4
				5:
					elev = 0.35 + float(roll[1]) * 0.4
				6:
					elev = 0.2
			var side := Vector3(cos(ang), elev, sin(ang)).normalized()
			if side.dot(d) > 0.75:
				side = (side + Vector3(cos(ang + 1.2), 0.1, sin(ang + 1.2))).normalized()
			var child_len: float = length * (0.42 + float(roll[1]) * 0.28) * detail
			var child_thick: float = thick_now * 0.62
			var child_droop: float = droop
			if algo == 2:
				child_droop = 1.4
			elif algo == 0:
				child_droop = 0.15
			r = _branch(st, ctx, p, side, child_len, child_thick, depth + 1, algo,
					detail, r, child_droop)
	if depth >= 1 or length < 2.5:
		var leaf_r: float = (0.9 + thick * 1.6) * detail * (1.15 if depth >= 2 else 0.85)
		r = _leaf_cluster(st, ctx, p + d * 0.3, leaf_r, detail, r)
	return r

# ---------------------------------------------------------------- algorithms

static func _grow_excurrent(st: SurfaceTool, ctx: Dictionary, detail: float, rng: int) -> void:
	var h: float = TREE_H * detail
	var trunk := 0.95
	_timber(st, ctx, Vector3(0, h * 0.28, 0), Vector3(trunk, h * 0.56, trunk), _wood(1.0))
	var tip := Vector3(0, h * 0.55, 0)
	var r := _branch(st, ctx, tip, Vector3(0, 1, 0), h * 0.48, trunk * 0.7, 0, 0, detail, rng, 0.05)
	for wi in 5:
		var y: float = h * (0.28 + float(wi) * 0.12)
		var rad: float = (2.4 - float(wi) * 0.35) * detail
		r = _leaf_cluster(st, ctx, Vector3(0, y, 0), rad * 0.55, detail * 0.9, r)

static func _grow_decurrent(st: SurfaceTool, ctx: Dictionary, detail: float, rng: int) -> void:
	var h: float = TREE_H * detail
	var trunk := 1.05
	_timber(st, ctx, Vector3(0, h * 0.18, 0), Vector3(trunk, h * 0.36, trunk), _wood(1.0))
	var r := rng
	for i in 3:
		var roll: Array = _rand(r)
		r = int(roll[0])
		var a: float = float(i) * TAU / 3.0 + float(roll[1]) * 0.4
		var dir := Vector3(cos(a) * 0.55, 0.85, sin(a) * 0.55).normalized()
		r = _branch(st, ctx, Vector3(0, h * 0.34, 0), dir, h * (0.55 + float(roll[1]) * 0.2),
				trunk * 0.72, 0, 1, detail, r, 0.12)
	r = _branch(st, ctx, Vector3(0, h * 0.36, 0), Vector3(0, 1, 0), h * 0.4, trunk * 0.55,
			0, 1, detail, r, 0.08)

static func _grow_weeping(st: SurfaceTool, ctx: Dictionary, detail: float, rng: int) -> void:
	var h: float = TREE_H * 0.85 * detail
	_timber(st, ctx, Vector3(0, h * 0.28, 0), Vector3(0.75, h * 0.56, 0.75), _wood(1.05))
	var r := rng
	for i in 5:
		var a: float = float(i) * TAU / 5.0
		var dir := Vector3(cos(a) * 0.7, 0.35, sin(a) * 0.7).normalized()
		r = _branch(st, ctx, Vector3(0, h * 0.55, 0), dir, h * 0.7, 0.48, 0, 2, detail, r, 1.5)

static func _grow_open_crown(st: SurfaceTool, ctx: Dictionary, detail: float, rng: int) -> void:
	var h: float = TREE_H * 0.9 * detail
	_timber(st, ctx, Vector3(0, h * 0.32, 0), Vector3(0.65, h * 0.64, 0.65), _wood(1.0))
	var r := _branch(st, ctx, Vector3(0, h * 0.62, 0), Vector3(0.8, 0.35, 0.15).normalized(),
			h * 0.35, 0.42, 0, 3, detail, rng, 0.05)
	var cy := h * 0.78
	for i in 7:
		var a: float = float(i) * TAU / 7.0
		var ox := cos(a) * 2.2 * detail
		var oz := sin(a) * 2.2 * detail
		r = _leaf_cluster(st, ctx, Vector3(ox, cy, oz), 1.3 * detail, detail, r)
	r = _leaf_cluster(st, ctx, Vector3(0, cy + 0.4, 0), 1.6 * detail, detail, r)

static func _grow_twisted(st: SurfaceTool, ctx: Dictionary, detail: float, rng: int) -> void:
	var h: float = TREE_H * detail
	var trunk := 0.85
	var r := rng
	var p := Vector3.ZERO
	var steps: int = int(h * 0.55 / STEP)
	for s in steps:
		var t: float = float(s) / float(maxi(steps - 1, 1))
		var yaw: float = t * 0.85
		var ox: float = sin(yaw) * 0.9 * detail
		var oz: float = (1.0 - cos(yaw)) * 0.45 * detail
		p = Vector3(ox, float(s) * STEP + STEP * 0.5, oz)
		var thick: float = lerpf(trunk, trunk * 0.55, t)
		_timber(st, ctx, p, Vector3(thick, STEP * 1.05, thick), _wood(1.0 - t * 0.1))
	r = _branch(st, ctx, p, Vector3(0.4, 0.9, 0.2).normalized(), h * 0.4, trunk * 0.55,
			0, 6, detail, r, 0.12)
	r = _branch(st, ctx, p, Vector3(-0.35, 0.85, -0.25).normalized(), h * 0.38, trunk * 0.5,
			0, 6, detail, r, 0.1)
	r = _leaf_cluster(st, ctx, p + Vector3(0, 1.2, 0), 2.0 * detail, detail, r)

static func _grow_giant(st: SurfaceTool, ctx: Dictionary, detail: float, rng: int) -> void:
	var h: float = TREE_H * 1.35 * detail
	var trunk := 1.55
	_timber(st, ctx, Vector3(0, h * 0.3, 0), Vector3(trunk, h * 0.6, trunk), _wood(1.0))
	for i in 4:
		var a: float = float(i) * TAU / 4.0
		_timber(st, ctx, Vector3(cos(a) * 0.7, h * 0.08, sin(a) * 0.7),
				Vector3(0.7, h * 0.16, 0.7), _wood(0.92))
	var r := rng
	for i in 4:
		var a: float = float(i) * TAU / 4.0 + 0.3
		var dir := Vector3(cos(a) * 0.5, 0.9, sin(a) * 0.5).normalized()
		r = _branch(st, ctx, Vector3(0, h * 0.55, 0), dir, h * 0.55, trunk * 0.55,
				0, 5, detail, r, 0.1)
	r = _branch(st, ctx, Vector3(0, h * 0.58, 0), Vector3(0, 1, 0), h * 0.45, trunk * 0.45,
			0, 5, detail, r, 0.05)

static func _grow_orchard(st: SurfaceTool, ctx: Dictionary, detail: float, rng: int) -> void:
	var h: float = TREE_H * 0.55 * detail
	_timber(st, ctx, Vector3(0, h * 0.28, 0), Vector3(0.6, h * 0.56, 0.6), _wood(1.0))
	_branch(st, ctx, Vector3(0, h * 0.5, 0), Vector3(0, 1, 0), h * 0.45, 0.4, 0, 1, detail, rng, 0.1)
	# Fruit is its own material key, so it stays a solid block off the lattice.
	for i in 4:
		var a: float = float(i) * 1.7
		_block(st, Vector3(cos(a) * 1.1 * detail, h * 0.85, sin(a) * 1.1 * detail),
				0.35 * detail, _fruit(0.95 + float(i % 2) * 0.12))

static func _grow_scrub(st: SurfaceTool, ctx: Dictionary, detail: float, rng: int) -> void:
	var r := rng
	for i in 5:
		var roll: Array = _rand(r)
		r = int(roll[0])
		var a: float = float(i) * 1.35 + float(roll[1])
		var ox := cos(a) * 0.55 * detail
		var oz := sin(a) * 0.55 * detail
		_timber(st, ctx, Vector3(ox, 0.7 * detail, oz), Vector3(0.35, 1.4 * detail, 0.35),
				_wood(1.1))
		r = _leaf_cluster(st, ctx, Vector3(ox, 1.5 * detail, oz), 0.9 * detail, detail, r)

static func _grow_reed(st: SurfaceTool, ctx: Dictionary, detail: float, _rng: int) -> void:
	for i in 7:
		var a: float = float(i) * 1.05
		var ox := cos(a) * 0.35 * detail
		var oz := sin(a) * 0.35 * detail
		var h: float = (2.2 + float(i % 3) * 0.55) * detail
		# A blade carries the leaf key but stays off the lattice — merging a
		# stalk into a blob would lose the whole reed silhouette.
		_add_prism(st, Vector3(ox, h * 0.5, oz), Vector3(0.14, h, 0.14), _leaf(0.9 + float(i % 3) * 0.06))
		_timber(st, ctx, Vector3(ox, h + 0.12, oz), Vector3(0.28, 0.28, 0.28), _wood(1.4))
