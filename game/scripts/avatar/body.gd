extends RefCounted
## Parametric bodies.
##
## A body is a DICTIONARY OF NUMBERS, not a mesh and not a preset id. Every
## archetype below is a point in that space, so any two of them blend, any of
## them can be nudged per character, and adding an axis costs one line here and
## one line in `measure()` — nothing has to enumerate archetypes.
##
## Ratios, not metres. Everything derives from `stature`, so a 1.68 m twink and
## a 1.98 m lanky are the same construction at different scales and the same
## code poses both. The only metres in this file come out of `measure()`.
##
## The axes are chosen for the range this game actually needs — the spread of
## adult male bodies, from a slight twink to a heavy bear — and they are
## deliberately not "slider 1..12". Each one is something you can see:
##
##   mass    how much of him there is at all
##   muscle  where that mass sits when he lifts:  shoulders, arms, chest, quads
##   soft    where it sits when he doesn't:       belly, hips, face, upper arm
##   taper   shoulder-to-waist; the V
##   limb    leg and arm length as a share of stature — stocky or leggy
##
## `muscle` and `soft` are independent on purpose. A muscle daddy is high on
## both. A bodybuilder is high on one. That distinction is the whole point.

const AXES := {
	"stature": 1.78,      # metres, floor to crown
	"mass": 0.45,         # 0 slight .. 1 heavy
	"muscle": 0.50,       # 0 untrained .. 1 competition
	"soft": 0.35,         # 0 cut .. 1 plush
	"taper": 0.55,        # 0 straight .. 1 extreme V
	"limb": 0.50,         # 0 stocky .. 1 leggy
	"neck": 0.50,         # 0 slender .. 1 traps-to-ears
	"head": 0.55,         # relative head size; larger reads younger / cuter
	"hair": 0.55,         # crown volume
	"beard": 0.20,        # 0 clean .. 1 full
	"fur": 0.20,          # body hair, drawn as a chest patch
	# Authored as DISPLAY values in the 0.45-0.85 band. `prop.gdshader` scales
	# by `albedo_scale` and then raises to 1.95, so anything authored at 0.25
	# lands near black — the mistake that made the props into holes.
	# (RENDER_CONTRACT §2009.)
	"skin": Color(0.82, 0.64, 0.50),
	"hair_col": Color(0.44, 0.32, 0.24),
	"top": Color(0.58, 0.64, 0.70),
	"bottom": Color(0.48, 0.50, 0.56),
	"shoe": Color(0.44, 0.40, 0.38),
}

## The spread. Every one of these is `AXES` with some numbers moved — there is
## no special-case code anywhere for any of them.
const ARCHETYPES := {
	# Small, light, long-legged, narrow. Big head ratio keeps him reading young.
	"twink": {
		"stature": 1.70, "mass": 0.16, "muscle": 0.22, "soft": 0.20,
		"taper": 0.34, "limb": 0.72, "neck": 0.28, "head": 0.66,
		"hair": 0.85, "beard": 0.02, "fur": 0.02,
		"top": Color(0.88, 0.60, 0.70), "bottom": Color(0.54, 0.56, 0.66),
	},
	# Lean and wiry with real muscle under it, and hairy. Not a smaller bear.
	"otter": {
		"stature": 1.76, "mass": 0.32, "muscle": 0.58, "soft": 0.16,
		"taper": 0.60, "limb": 0.60, "neck": 0.44, "head": 0.52,
		"hair": 0.55, "beard": 0.72, "fur": 0.72,
		"top": Color(0.60, 0.70, 0.56), "bottom": Color(0.50, 0.46, 0.42),
	},
	# Everything above the waist. Shoulders nearly a third of his height, and
	# arms that physically cannot hang at his sides — see gait.gd.
	"jock": {
		"stature": 1.80, "mass": 0.64, "muscle": 0.94, "soft": 0.10,
		"taper": 0.94, "limb": 0.40, "neck": 0.82, "head": 0.44,
		"hair": 0.35, "beard": 0.12, "fur": 0.10,
		"top": Color(0.80, 0.78, 0.72), "bottom": Color(0.46, 0.52, 0.60),
	},
	# The jock twenty years on: same frame, more of it, and softer over the top.
	"muscle_daddy": {
		"stature": 1.87, "mass": 0.80, "muscle": 0.86, "soft": 0.34,
		"taper": 0.80, "limb": 0.46, "neck": 0.90, "head": 0.44,
		"hair": 0.40, "beard": 0.58, "fur": 0.62,
		"top": Color(0.56, 0.58, 0.66), "bottom": Color(0.42, 0.44, 0.50),
	},
	# Wide before he is tall. Mass sits low and front; the shoulders do not taper.
	"bear": {
		"stature": 1.84, "mass": 0.92, "muscle": 0.46, "soft": 0.86,
		"taper": 0.26, "limb": 0.42, "neck": 0.74, "head": 0.50,
		"hair": 0.45, "beard": 0.96, "fur": 1.00,
		"top": Color(0.72, 0.52, 0.42), "bottom": Color(0.46, 0.44, 0.42),
	},
	# A bear who has not finished. Shorter, rounder, bouncier.
	"cub": {
		"stature": 1.72, "mass": 0.72, "muscle": 0.38, "soft": 0.76,
		"taper": 0.32, "limb": 0.46, "neck": 0.60, "head": 0.70,
		"hair": 0.62, "beard": 0.62, "fur": 0.70,
		"top": Color(0.86, 0.66, 0.46), "bottom": Color(0.52, 0.54, 0.60),
	},
	# Broad, comfortable, unbothered. The default shape of authority.
	"daddy": {
		"stature": 1.83, "mass": 0.56, "muscle": 0.58, "soft": 0.46,
		"taper": 0.58, "limb": 0.50, "neck": 0.62, "head": 0.48,
		"hair": 0.42, "beard": 0.48, "fur": 0.55,
		"top": Color(0.56, 0.66, 0.76), "bottom": Color(0.44, 0.46, 0.52),
	},
	# All limb and no mass — the long-shanked cartoon alien read. Tiny torso
	# slung between a very long neck and very long legs.
	"lanky": {
		"stature": 1.99, "mass": 0.08, "muscle": 0.16, "soft": 0.05,
		"taper": 0.46, "limb": 0.97, "neck": 0.98, "head": 0.36,
		"hair": 0.30, "beard": 0.05, "fur": 0.02,
		"skin": Color(0.70, 0.76, 0.66),
		"top": Color(0.50, 0.68, 0.70), "bottom": Color(0.40, 0.48, 0.54),
	},
}

## Presentation order — the spectrum, roughly slight to heavy.
const ORDER := ["twink", "lanky", "otter", "jock", "daddy", "muscle_daddy", "cub", "bear"]

static func make(archetype := "daddy") -> Dictionary:
	var b: Dictionary = AXES.duplicate()
	for k in ARCHETYPES.get(archetype, {}):
		b[k] = ARCHETYPES[archetype][k]
	b["name"] = archetype
	return b

## Any two bodies blend, because a body is only numbers. This is what makes the
## eight archetypes a spectrum rather than a menu.
static func blend(a: Dictionary, b: Dictionary, t: float) -> Dictionary:
	var out := {}
	for k in a:
		var va = a[k]
		var vb = b.get(k, va)
		if typeof(va) == TYPE_FLOAT or typeof(va) == TYPE_INT:
			out[k] = lerpf(float(va), float(vb), t)
		elif typeof(va) == TYPE_COLOR:
			out[k] = (va as Color).lerp(vb, t)
		else:
			out[k] = va if t < 0.5 else vb
	out["name"] = a.get("name", "?") if t < 0.5 else b.get("name", "?")
	return out

## One man, not one of eight statues. Same id always gives the same man.
static func vary(base: Dictionary, id: int) -> Dictionary:
	var out: Dictionary = base.duplicate()
	var h := func(salt: int) -> float:
		var x: int = (id * 73856093) ^ (salt * 19349663)
		x = (x ^ (x >> 13)) * 1274126177
		return float((x ^ (x >> 16)) & 0xFFFFFF) / 16777215.0 - 0.5
	out["stature"] = float(out["stature"]) + h.call(1) * 0.13
	for k in ["mass", "muscle", "soft", "taper", "limb", "neck", "hair", "beard", "fur"]:
		out[k] = clampf(float(out[k]) + h.call(hash(k)) * 0.16, 0.0, 1.0)
	var skin: Color = out["skin"]
	# Warm/cool and light/dark, independently — one slider makes ten clones.
	var v: float = 1.0 + h.call(7) * 0.55
	var warm: float = h.call(8) * 0.10
	out["skin"] = Color(
		clampf(skin.r * v + warm, 0.05, 1.0),
		clampf(skin.g * v, 0.04, 1.0),
		clampf(skin.b * v - warm * 0.5, 0.03, 1.0))
	var hc: Color = out["hair_col"]
	out["hair_col"] = hc.lerp(Color(0.82, 0.78, 0.70), maxf(h.call(9), 0.0) * 1.2)
	out["top"] = (out["top"] as Color).lerp(Color(0.9, 0.85, 0.7), h.call(10) * 0.5 + 0.1)
	return out

## Every measurement the rig and the gait need, in metres.
##
## Kept in ONE function so nothing downstream ever re-derives a proportion from
## an axis — a body that measures 1.84 m has a leg, an arm and a stride that
## all agree, and `gait.pose` never has to know what a "bear" is.
static func measure(b: Dictionary) -> Dictionary:
	var s: float = float(b["stature"])
	var mass: float = float(b["mass"])
	var mus: float = float(b["muscle"])
	var soft: float = float(b["soft"])
	var taper: float = float(b["taper"])
	var limb: float = float(b["limb"])

	# Vertical budget. Legs take what limb asks for; the torso gets the rest,
	# so a leggy figure has a short torso rather than growing out of his height.
	var head_h: float = s * (0.108 + 0.036 * float(b["head"]))
	var neck_h: float = s * (0.018 + 0.055 * float(b["neck"]))
	var leg_len: float = s * (0.430 + 0.085 * limb)
	var torso_len: float = maxf(s - head_h - neck_h - leg_len, s * 0.20)

	var thigh: float = leg_len * 0.52
	var shin: float = leg_len * 0.42
	var foot_h: float = leg_len * 0.06

	# Girths. Muscle goes up and out; soft goes forward and down.
	var g: float = 0.60 + mass * 0.80
	# Biacromial plus deltoid. At the top of the range this is a real 0.55 m
	# across, which is what an actual competition build measures — the number
	# has to be allowed to get silly or the archetypes all read as one man.
	var sh_w: float = s * (0.190 + 0.105 * mus + 0.030 * mass)
	var waist_w: float = sh_w * (0.94 - 0.40 * taper + 0.34 * soft)
	var hip_w: float = s * (0.140 + 0.030 * mass + 0.028 * soft)
	var chest_d: float = s * (0.086 + 0.044 * mus + 0.030 * soft)
	# The belly has to be allowed to pass the chest, or a bear is a jock in a
	# brown shirt. This is the shape that separates them in profile.
	var belly_d: float = s * (0.066 + 0.012 * mus + 0.125 * soft)

	# Arms. Upper-arm girth is the loudest single number on a bodybuilder.
	var arm_len: float = s * (0.176 + 0.024 * limb)
	var fore_len: float = s * (0.150 + 0.022 * limb)
	var arm_g: float = s * (0.049 + 0.036 * mus + 0.024 * soft) * g
	var fore_g: float = arm_g * 0.80
	var thigh_g: float = s * (0.072 + 0.040 * mus + 0.034 * soft) * g
	var shin_g: float = thigh_g * 0.70

	return {
		"stature": s,
		"head_h": head_h, "head_w": head_h * 0.82, "head_d": head_h * 0.86,
		"neck_h": neck_h, "neck_w": s * (0.042 + 0.036 * float(b["neck"]) + 0.012 * mus),
		"torso_len": torso_len,
		"chest_h": torso_len * 0.56, "belly_h": torso_len * 0.44,
		"sh_w": sh_w, "waist_w": waist_w, "hip_w": hip_w,
		"glute_d": s * (0.048 + 0.030 * mus + 0.046 * soft),
		"chest_d": chest_d, "belly_d": belly_d,
		"leg_len": leg_len, "thigh": thigh, "shin": shin,
		"thigh_g": thigh_g, "shin_g": shin_g,
		"foot_h": foot_h, "foot_l": s * (0.140 + 0.016 * mass), "foot_w": s * 0.052,
		"arm_len": arm_len, "fore_len": fore_len,
		"arm_g": arm_g, "fore_g": fore_g,
		"hand_l": s * 0.062,
		# Hip joint height: what the controller must stand him on.
		"hip_y": leg_len,
	}

# ------------------------------------------------------------------- build --

static func _box(size: Vector3, col: Color) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var h: Vector3 = size * 0.5
	var faces := [
		[Vector3(0, 0, 1), [Vector3(-h.x, -h.y, h.z), Vector3(h.x, -h.y, h.z), Vector3(h.x, h.y, h.z), Vector3(-h.x, h.y, h.z)]],
		[Vector3(0, 0, -1), [Vector3(h.x, -h.y, -h.z), Vector3(-h.x, -h.y, -h.z), Vector3(-h.x, h.y, -h.z), Vector3(h.x, h.y, -h.z)]],
		[Vector3(1, 0, 0), [Vector3(h.x, -h.y, h.z), Vector3(h.x, -h.y, -h.z), Vector3(h.x, h.y, -h.z), Vector3(h.x, h.y, h.z)]],
		[Vector3(-1, 0, 0), [Vector3(-h.x, -h.y, -h.z), Vector3(-h.x, -h.y, h.z), Vector3(-h.x, h.y, h.z), Vector3(-h.x, h.y, -h.z)]],
		[Vector3(0, 1, 0), [Vector3(-h.x, h.y, h.z), Vector3(h.x, h.y, h.z), Vector3(h.x, h.y, -h.z), Vector3(-h.x, h.y, -h.z)]],
		[Vector3(0, -1, 0), [Vector3(-h.x, -h.y, -h.z), Vector3(h.x, -h.y, -h.z), Vector3(h.x, -h.y, h.z), Vector3(-h.x, -h.y, h.z)]],
	]
	for f in faces:
		var n: Vector3 = f[0]
		var q: Array = f[1]
		# Cheap directional shading baked per face, so a box body still reads as
		# a solid even before the sun reaches it.
		var k: float = 1.0 + n.y * 0.10 - absf(n.x) * 0.06
		var c := Color(col.r * k, col.g * k, col.b * k, col.a)
		for idx in [0, 1, 2, 0, 2, 3]:
			st.set_normal(n)
			st.set_color(c)
			st.add_vertex(q[idx])
	return st.commit()

static func _part(parent: Node3D, size: Vector3, col: Color, pos: Vector3,
		mat: ShaderMaterial, tilt := 0.0) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	mi.mesh = _box(size, col)
	mi.material_override = mat
	mi.position = pos
	# Roll about Z. Only the brows use it, and only slightly — but a box that
	# cannot tilt cannot carry an expression, and everything above the nose is
	# expression.
	if not is_zero_approx(tilt):
		mi.rotation = Vector3(0, 0, tilt)
	parent.add_child(mi)
	return mi

## Build the rig under `parent` and return the joint table.
##
## Joints are named, not indexed, and `gait.pose` only ever asks for names — so
## a body may add parts (a belly, a beard) without the gait knowing.
static func build(parent: Node3D, b: Dictionary, cast_shadow := true) -> Dictionary:
	for c in parent.get_children():
		c.queue_free()
	var m: Dictionary = measure(b)
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/prop.gdshader")
	# Authored display values; prop.gdshader scales then sRGB→linears them.
	mat.set_shader_parameter("albedo_scale", 0.92)
	mat.set_shader_parameter("haze_start", 60.0)
	mat.set_shader_parameter("haze_end", 320.0)

	var skin: Color = b["skin"]
	var top: Color = b["top"]
	var bottom: Color = b["bottom"]
	var shoe: Color = b["shoe"]
	var hair_col: Color = b["hair_col"]
	# Hairy skin reads darker and warmer, not as a decal. Applied to the parts
	# that are actually bare: forearms, hands and shins.
	var fur_skin: Color = skin.lerp(hair_col, float(b["fur"]) * 0.30)
	var rig := {"mat": mat, "measure": m, "body": b}

	# --- pelvis -> spine -> chest ---
	var hip := Node3D.new()
	hip.position = Vector3(0, m["hip_y"], 0)
	parent.add_child(hip)
	rig["hip"] = hip
	_part(hip, Vector3(m["hip_w"] * 1.05, m["torso_len"] * 0.20, m["belly_d"] * 0.86),
			bottom, Vector3(0, m["torso_len"] * 0.06, 0), mat)
	# Seat. Reads only in profile, and profile is most of how a walk reads.
	_part(hip, Vector3(m["hip_w"] * 0.98, m["torso_len"] * 0.17, m["glute_d"]),
			bottom, Vector3(0, m["torso_len"] * 0.01, -m["belly_d"] * 0.34), mat)

	# Belt: the line where top stops and bottom starts. Without it the torso
	# was one unbroken column of colour from collar to knee.
	_part(hip, Vector3(m["hip_w"] * 1.10, m["torso_len"] * 0.055, m["belly_d"] * 0.94),
			bottom.darkened(0.34), Vector3(0, m["torso_len"] * 0.15, 0), mat)

	var spine := Node3D.new()
	spine.position = Vector3(0, m["torso_len"] * 0.14, 0)
	hip.add_child(spine)
	rig["spine"] = spine
	# The belly is its own box so `soft` can push it forward without widening
	# the ribcage — the one shape that separates a bear from a bodybuilder.
	_part(spine, Vector3(m["waist_w"], m["belly_h"], m["belly_d"]),
			top, Vector3(0, m["belly_h"] * 0.45,
					m["belly_d"] * 0.26 * float(b["soft"])), mat)

	var chest := Node3D.new()
	chest.position = Vector3(0, m["belly_h"] * 0.92, 0)
	spine.add_child(chest)
	rig["chest"] = chest
	# The ribcage is NOT the widest part of anybody. Keep it near the waist and
	# put the width in the deltoids, or every build reads as one slab.
	# How much of the width is ribcage rather than shoulder. A cut build is a
	# V — the ribcage stays near the waist and the deltoids carry it. A soft
	# build is a barrel, and the difference between those two is most of what
	# tells a bodybuilder from a bear at a hundred metres.
	var rib_w: float = lerpf(float(m["waist_w"]), float(m["sh_w"]),
			0.14 + 0.22 * float(b["soft"]))
	_part(chest, Vector3(rib_w, m["chest_h"] * 0.98, m["chest_d"]),
			top, Vector3(0, m["chest_h"] * 0.42, 0), mat)
	# Pecs: two of them, with a sternum gap. One full-width shelf across the
	# ribcage put a hard horizontal seam from armpit to armpit, which read as
	# the top edge of a breastplate rather than a chest — and nobody has a
	# single pec. The gap costs one box and gives the torso a centre line.
	var pec_d: float = m["chest_d"] * (0.22 + 0.30 * float(b["muscle"])
			+ 0.16 * float(b["soft"]))
	for side in [-1.0, 1.0]:
		_part(chest, Vector3(rib_w * 0.44, m["chest_h"] * 0.40, pec_d),
				top.lightened(0.05),
				Vector3(side * rib_w * 0.25, m["chest_h"] * 0.62,
						m["chest_d"] * 0.52), mat)
	# Deltoid caps stand PROUD of the ribcage, and their outer edge IS the
	# shoulder width — otherwise the number in `measure` never reaches the eye.
	var dw: float = maxf((float(m["sh_w"]) - rib_w) * 0.5, float(m["sh_w"]) * 0.055)
	for side in [-1.0, 1.0]:
		_part(chest, Vector3(dw, m["chest_h"] * 0.52, m["chest_d"] * 0.96), top,
				Vector3(side * (float(m["sh_w"]) - dw) * 0.5, m["chest_h"] * 0.70, 0), mat)
		# Trapezius: fills the gap from neck to shoulder. Absent on a twink,
		# and on a jock it is most of why he has no neck.
		if float(b["muscle"]) > 0.35 or float(b["neck"]) > 0.5:
			var trap: float = maxf(float(b["muscle"]) - 0.2, 0.0) * 0.55 + float(b["neck"]) * 0.25
			_part(chest, Vector3(rib_w * 0.46, m["chest_h"] * 0.30 * trap, m["chest_d"] * 0.62),
					top.lightened(0.03),
					Vector3(side * rib_w * 0.26, m["chest_h"] * (0.92 + 0.06 * trap), 0), mat)
	# Body hair shows where skin shows. It used to be a flat panel laid over the
	# shirt at chest height, which read as a badge, a hole or a censor bar —
	# never as hair, because clothed chests do not have visible hair. What is
	# left is the tuft in the collar opening; the forearms and shins carry the
	# rest, tinted below (`fur_skin`).
	if float(b["fur"]) > 0.30:
		var fv: float = float(b["fur"])
		_part(chest, Vector3(rib_w * (0.16 + 0.10 * fv), m["chest_h"] * 0.13,
						m["chest_d"] * 0.10),
				hair_col.lerp(skin, 0.22),
				Vector3(0, m["chest_h"] * 0.86, m["chest_d"] * 0.46), mat)

	# --- neck -> head ---
	var neck := Node3D.new()
	neck.position = Vector3(0, m["chest_h"] * 0.92, 0)
	chest.add_child(neck)
	rig["neck"] = neck
	_part(neck, Vector3(m["neck_w"], m["neck_h"], m["neck_w"] * 0.92),
			skin, Vector3(0, m["neck_h"] * 0.5, 0), mat)
	# A collar. `top` and `bottom` were colours painted onto the body boxes, so
	# everyone read as a coloured mannequin rather than a dressed person. These
	# few bands are the cheapest thing that says "clothing": each sits slightly
	# proud of the part underneath, so it catches its own edge of light.
	# Sized off the neck and kept ABOVE the shoulder line. At 1.42x neck width
	# it dipped into the shoulders and stood proud of them, which on a
	# thick-necked build read as a ruff rather than a collar.
	_part(neck, Vector3(m["neck_w"] * 1.16, m["neck_h"] * 0.26, m["neck_w"] * 1.10),
			top.darkened(0.12), Vector3(0, m["neck_h"] * 0.20, 0), mat)
	var head := Node3D.new()
	head.position = Vector3(0, m["neck_h"], 0)
	neck.add_child(head)
	rig["head"] = head
	var hw: float = float(m["head_w"])
	var hh: float = float(m["head_h"])
	var hd: float = float(m["head_d"])
	_part(head, Vector3(hw, hh, hd), skin, Vector3(0, hh * 0.5, 0), mat)
	# A face, at four boxes. Without it these read as blindfolded mannequins,
	# and every one of them is someone you are meant to be able to look at.
	var dark: Color = hair_col.darkened(0.45)
	for side in [-1.0, 1.0]:
		_part(head, Vector3(hw * 0.17, hh * 0.075, hd * 0.06), dark,
				Vector3(side * hw * 0.23, hh * 0.615, hd * 0.5), mat)
	# Brows. The single cheapest piece of expression on a box head: two eyes
	# alone read as a mannequin staring, because a face is mostly what sits
	# above the eyes. Angled from `soft` rather than a new axis — a lean face
	# gets an inward, harder set, a soft one gets a flatter, kinder brow.
	var brow_tilt: float = lerpf(0.20, 0.04, float(b["soft"]))
	var brow_col: Color = hair_col.darkened(0.05)
	for side in [-1.0, 1.0]:
		_part(head, Vector3(hw * 0.24, hh * 0.055, hd * 0.07), brow_col,
				Vector3(side * hw * 0.23, hh * 0.695, hd * 0.5), mat,
				side * brow_tilt)
	_part(head, Vector3(hw * 0.17, hh * 0.16, hd * 0.14), skin.lightened(0.04),
			Vector3(0, hh * 0.505, hd * 0.52), mat)
	# A mouth, unless a full beard has covered it.
	if float(b["beard"]) < 0.55:
		_part(head, Vector3(hw * 0.30, hh * 0.038, hd * 0.05),
				skin.darkened(0.45),
				Vector3(0, hh * 0.335, hd * 0.5), mat)
	if float(b["beard"]) > 0.12:
		var bv: float = float(b["beard"])
		# A shell around the jaw, not a bar hanging off the chin. It leaves the
		# eyes and the bridge of the nose clear, which is what a beard does.
		# Proud of the jaw, not flush with it. At head width the shell sat
		# exactly on the skull's outline, so even a full beard read as a smudge
		# of darker skin rather than hair — the silhouette never changed. It
		# now overhangs, and hangs BELOW the chin as it fills in, which is the
		# part you see against a bright background.
		var jut: float = 1.04 + 0.10 * bv
		var drop: float = hh * (0.06 + 0.20 * bv)
		var beard_col: Color = hair_col.darkened(0.22)
		_part(head, Vector3(hw * jut, hh * (0.22 + 0.30 * bv) + drop, hd * jut),
				beard_col,
				Vector3(0, hh * (0.20 - 0.06 * bv) - drop * 0.5, 0), mat)
		# Moustache, kept separate so it stays put as the beard grows.
		_part(head, Vector3(hw * 0.64, hh * (0.09 + 0.10 * bv), hd * 0.16),
				beard_col, Vector3(0, hh * 0.40, hd * 0.52), mat)
	if float(b["hair"]) > 0.06:
		var hv: float = float(b["hair"])
		# A cap ON TOP of the skull. Sitting it at 0.86 of head height put a
		# dark band straight across the eyes.
		_part(head, Vector3(hw * 1.04, hh * (0.13 + 0.24 * hv), hd * 1.04), hair_col,
				Vector3(0, hh * (1.0 + 0.10 * hv), 0), mat)
		# Sides and back, so a full head of hair has a silhouette from behind.
		_part(head, Vector3(hw * 1.04, hh * (0.14 + 0.40 * hv), hd * (0.16 + 0.16 * hv)),
				hair_col, Vector3(0, hh * (0.88 - 0.14 * hv), -hd * 0.50), mat)
		if hv > 0.45:
			for side in [-1.0, 1.0]:
				_part(head, Vector3(hd * 0.10, hh * (0.22 * hv), hd * 0.86), hair_col,
						Vector3(side * hw * 0.50, hh * (0.80 - 0.10 * hv), -hd * 0.04), mat)

	# --- arms: clavicle -> upper -> fore -> hand ---
	for side in [-1.0, 1.0]:
		var key: int = int(side)
		var clav := Node3D.new()
		# Hang the arm off the shoulder, but never inside the ribcage. Rooting
		# it purely from shoulder width put the inner face of the upper arm
		# behind the ribs on any build that does not taper — so a bear's arms
		# vanished into his torso and he read as one brown mass. The notch
		# between arm and body is most of what makes a limb legible.
		var arm_x: float = maxf(
				float(m["sh_w"]) * 0.5 - float(m["arm_g"]) * 0.46,
				rib_w * 0.5 + float(m["arm_g"]) * 0.5 - float(m["arm_g"]) * 0.18)
		clav.position = Vector3(side * arm_x, m["chest_h"] * 0.72, 0)
		chest.add_child(clav)
		rig["clav%d" % key] = clav
		var upper := Node3D.new()
		clav.add_child(upper)
		rig["arm%d" % key] = upper
		_part(upper, Vector3(m["arm_g"], m["arm_len"], m["arm_g"] * 0.92),
				top.lerp(skin, 0.25), Vector3(0, -m["arm_len"] * 0.5, 0), mat)
		# Cuff: where the sleeve ends and the arm begins. The sleeve/skin colour
		# change alone read as a paint join, not a garment edge.
		_part(upper, Vector3(m["arm_g"] * 1.10, m["arm_len"] * 0.10,
						m["arm_g"] * 1.02),
				top.darkened(0.16), Vector3(0, -m["arm_len"] * 0.94, 0), mat)
		var fore := Node3D.new()
		fore.position = Vector3(0, -m["arm_len"], 0)
		upper.add_child(fore)
		rig["fore%d" % key] = fore
		_part(fore, Vector3(m["fore_g"], m["fore_len"], m["fore_g"] * 0.92),
				fur_skin, Vector3(0, -m["fore_len"] * 0.5, 0), mat)
		var hand := Node3D.new()
		hand.position = Vector3(0, -m["fore_len"], 0)
		fore.add_child(hand)
		rig["hand%d" % key] = hand
		# Wider than the forearm and squarer in plan, or a hand is just a small
		# pale cube floating at hip height and reads as a pocket.
		_part(hand, Vector3(m["fore_g"] * 1.12, m["hand_l"], m["fore_g"] * 0.78),
				fur_skin, Vector3(0, -m["hand_l"] * 0.5, 0), mat)

		# --- legs: thigh -> shin -> foot ---
		var thigh := Node3D.new()
		# Keep a gap between the legs whatever the girth. Rooted at a fixed
		# share of hip width, a heavy build's thighs overlapped each other and
		# the two of them fused into one slab from hip to ankle — the shape of
		# a long skirt, with feet poking out under the hem.
		var stand: float = maxf(float(m["hip_w"]) * 0.30,
				float(m["thigh_g"]) * 0.5 + float(m["stature"]) * 0.016)
		thigh.position = Vector3(side * stand, 0, 0)
		hip.add_child(thigh)
		rig["thigh%d" % key] = thigh
		_part(thigh, Vector3(m["thigh_g"], m["thigh"], m["thigh_g"] * 0.94),
				bottom, Vector3(0, -m["thigh"] * 0.5, 0), mat)
		var shin := Node3D.new()
		shin.position = Vector3(0, -m["thigh"], 0)
		thigh.add_child(shin)
		rig["shin%d" % key] = shin
		_part(shin, Vector3(m["shin_g"], m["shin"], m["shin_g"] * 0.94),
				bottom.lerp(shoe, 0.2), Vector3(0, -m["shin"] * 0.5, 0), mat)
		var foot := Node3D.new()
		foot.position = Vector3(0, -m["shin"], 0)
		shin.add_child(foot)
		rig["foot%d" % key] = foot
		# Trouser hem, then the shoe, then a sole under it. Three tones over
		# 12 cm is what stops the leg ending in a single flat tab.
		_part(shin, Vector3(m["shin_g"] * 1.12, m["shin"] * 0.10, m["shin_g"] * 1.06),
				bottom.darkened(0.22), Vector3(0, -m["shin"] * 0.93, 0), mat)
		_part(foot, Vector3(m["foot_w"], m["foot_h"], m["foot_l"]),
				shoe, Vector3(0, -m["foot_h"] * 0.5, m["foot_l"] * 0.18), mat)
		# Tucked under the shoe, not overhanging it — a sole longer than the
		# shoe reads as a blade sticking out at the toe.
		_part(foot, Vector3(m["foot_w"] * 1.02, m["foot_h"] * 0.26,
						m["foot_l"] * 0.94),
				shoe.darkened(0.40),
				Vector3(0, -m["foot_h"] * 0.87, m["foot_l"] * 0.18), mat)

	var shadow: GeometryInstance3D.ShadowCastingSetting = \
			GeometryInstance3D.SHADOW_CASTING_SETTING_ON if cast_shadow \
			else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	for n in parent.find_children("*", "MeshInstance3D", true, false):
		(n as MeshInstance3D).cast_shadow = shadow
	return rig

## One merged mesh of the same man, standing still.
##
## Built by constructing the real rig and baking it, so the figure you see at
## four hundred metres is the figure you meet at four — a bear reads as a bear
## across a field, which is the whole reason the far tier is per-archetype
## instead of one capsule.
##
## Vertex colours are the real ones, so a MultiMesh using this must keep its
## INSTANCE colour near white and treat it as a tint. (RENDER_CONTRACT §2009.)
static func bake(b: Dictionary) -> ArrayMesh:
	var tmp := Node3D.new()
	var rig: Dictionary = build(tmp, b, false)
	# A neutral standing pose, not a T-pose: arms carried where his build
	# actually carries them.
	var mus: float = float(b["muscle"])
	var soft: float = float(b["soft"])
	var carry: float = 0.04 + mus * 0.30 + soft * 0.13
	for side in [-1.0, 1.0]:
		var k: int = int(side)
		rig["arm%d" % k].rotation = Vector3(0.04, side * mus * 0.28, side * carry)
		rig["fore%d" % k].rotation = Vector3(-(0.16 + mus * 0.16), 0.0, 0.0)
		rig["thigh%d" % k].position.x = side * (0.028 + mus * 0.045 + soft * 0.030) * float(b["stature"])
		rig["thigh%d" % k].rotation = Vector3(0.0, side * (0.06 + mus * 0.16), 0.0)
	# Walk the hierarchy by hand. `global_transform` needs the node to be in the
	# tree and this runs at load, before there is one.
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for c in tmp.get_children():
		if c is Node3D:
			_bake_node(c as Node3D, Transform3D.IDENTITY, st)
	tmp.free()
	return st.commit()

static func _bake_node(node: Node3D, parent_xf: Transform3D, st: SurfaceTool) -> void:
	var xf: Transform3D = parent_xf * node.transform
	if node is MeshInstance3D:
		var src := (node as MeshInstance3D).mesh as ArrayMesh
		if src != null and src.get_surface_count() > 0:
			var arr: Array = src.surface_get_arrays(0)
			var vs: PackedVector3Array = arr[Mesh.ARRAY_VERTEX]
			var ns: PackedVector3Array = arr[Mesh.ARRAY_NORMAL]
			var cs: PackedColorArray = arr[Mesh.ARRAY_COLOR]
			var idx := PackedInt32Array()
			if arr[Mesh.ARRAY_INDEX] != null:
				idx = arr[Mesh.ARRAY_INDEX]
			var count: int = idx.size() if idx.size() > 0 else vs.size()
			for i in count:
				var v: int = idx[i] if idx.size() > 0 else i
				st.set_normal(xf.basis * ns[v])
				st.set_color(cs[v] if cs.size() > v else Color.WHITE)
				st.add_vertex(xf * vs[v])
	for c in node.get_children():
		if c is Node3D:
			_bake_node(c as Node3D, xf, st)
