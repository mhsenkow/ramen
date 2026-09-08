extends RefCounted
## Parametric locomotion.
##
## Same idea as `body.gd`: a gait is a DICTIONARY OF NUMBERS and an archetype is
## a point in that space. `pose()` never asks what kind of man it is posing.
##
## Two rules the whole file is built on:
##
## 1. **Phase advances with DISTANCE, not time.** Stride length is a real
##    measurement in metres, so the foot cannot skate — walk, run, uphill,
##    encumbered, it is the same relationship. Speed changes the gait by
##    changing stride and cadence, which is what it does to a person.
##
## 2. **Every event has a place in the cycle.** `_bump` puts a movement at a
##    phase — heel strike at 0, toe-off just before π, knee peak in mid-swing —
##    instead of layering sines and hoping. That is why these read as different
##    walks rather than the same walk at different amplitudes.
##
## Phase 0 is initial contact for the right foot. 0..π is right stance,
## π..2π is right swing. Left is the same, half a cycle later.
##
## Sign convention, body-local (+X right, +Y up, +Z forward): a positive
## rotation about +X carries tops forward and tips backward. So a thigh flexes
## forward at NEGATIVE rotation.x and a torso leans forward at POSITIVE.

const AXES := {
	# --- footfall ---------------------------------------------------------
	"stride": 0.82,        # stride length in statures, at walking pace
	"track": 0.10,         # stance width in statures, ankle to ankle
	"cross": 0.0,          # statures the foot lands INSIDE the hip line (+) or out (−)
	"toe_out": 0.10,       # rad, duck-footedness
	# --- pelvis -----------------------------------------------------------
	"hip_sway": 0.020,     # statures of lateral shift onto the stance leg
	"hip_drop": 0.06,      # rad the pelvis drops on the swing side
	"pelvis_yaw": 0.12,    # rad the stance hip rotates back
	"waddle": 0.0,         # rad of whole-body lateral roll, once per stride
	# --- spine and shoulders ----------------------------------------------
	"chest_counter": 0.14, # rad the chest yaws against the pelvis. 0 = locked.
	"shoulder_roll": 0.03, # rad the clavicles see-saw
	"lean": 0.06,          # rad forward at walking pace
	"chest_lift": 0.0,     # rad of static chest-up posture
	"chin": 0.0,           # rad of static head pitch; + is chin up
	"head_lead": 0.0,      # rad the neck carries the head ahead of the chest
	# --- arms -------------------------------------------------------------
	"arm_swing": 0.42,     # rad amplitude
	"arm_carry": 0.06,     # rad of abduction — how far the arms hang from the body
	"arm_fwd": 0.02,       # rad of static forward carry
	"arm_twist": 0.0,      # rad of internal rotation; palms turning backward
	"elbow": 0.18,         # rad of base flex
	"elbow_swing": 0.30,   # rad of extra flex on the forward swing
	"wrist": 0.0,          # rad of static wrist break — a held hand, not a fist
	"hand_splay": 0.0,     # rad the hands turn outward
	# --- legs -------------------------------------------------------------
	"knee_lift": 0.85,     # rad of peak knee flex in mid-swing
	"knee_stance": 0.12,   # rad of knee flex under load just after contact
	"heel": 0.35,          # dorsiflexion before contact — a heel strike
	"toe": 0.55,           # plantarflexion at toe-off — a push
	# --- vertical ---------------------------------------------------------
	"bob": 0.020,          # statures, twice per stride
	"settle": 0.010,       # statures of extra dip on each foot plant
	"bounce": 0.0,         # 0..1 how much of the run is flight
	# --- character --------------------------------------------------------
	"swish": 0.0,          # 0..1 pelvic figure-eight, crossover, loose wrists
	"walk_speed": 1.45,    # m/s where this walk is comfortable
	"run_speed": 5.2,      # m/s where the run is comfortable
	"idle_shift": 1.0,     # how much he moves while standing still
	"idle_cock": 0.0,      # rad of hip cocked onto one leg at rest
}

const ARCHETYPES := {
	# Quick, short, and all hips. Feet land on a line — that crossover is what
	# makes the walk read from behind. Arms loose and swinging across.
	"twink": {
		"stride": 0.70, "track": 0.045, "cross": 0.030, "toe_out": 0.06,
		"hip_sway": 0.034, "hip_drop": 0.13, "pelvis_yaw": 0.22,
		"chest_counter": 0.24, "shoulder_roll": 0.06, "lean": 0.03,
		"arm_swing": 0.62, "arm_carry": 0.03, "elbow": 0.24, "elbow_swing": 0.42,
		"wrist": 0.30, "knee_lift": 0.95, "heel": 0.48, "toe": 0.62,
		"bob": 0.028, "settle": 0.006, "bounce": 0.55, "swish": 0.90,
		"walk_speed": 1.60, "run_speed": 5.0, "idle_cock": 0.10, "idle_shift": 1.4,
	},
	# Nothing wasted. The baseline a body with no strong axis falls back to.
	"otter": {
		"stride": 0.86, "track": 0.085, "hip_sway": 0.020, "hip_drop": 0.06,
		"pelvis_yaw": 0.15, "chest_counter": 0.17, "lean": 0.08,
		"arm_swing": 0.50, "arm_carry": 0.07, "elbow": 0.22, "elbow_swing": 0.34,
		"knee_lift": 0.92, "heel": 0.40, "toe": 0.62,
		"bob": 0.022, "settle": 0.009, "bounce": 0.45,
		"walk_speed": 1.55, "run_speed": 5.8,
	},
	# The lats do two things. They hold the arms off the ribs — he cannot swing
	# them inward, so the swing is small and the carry is huge — and they lock
	# the thorax to the pelvis, so almost nothing counter-rotates. What is left
	# is a wide, braced, shoulder-driven roll. Everyone recognises it.
	"jock": {
		"stride": 0.86, "track": 0.155, "cross": -0.035, "toe_out": 0.20,
		"hip_sway": 0.024, "hip_drop": 0.025, "pelvis_yaw": 0.07,
		"chest_counter": 0.03, "shoulder_roll": 0.115, "lean": 0.04,
		"chest_lift": 0.07, "chin": 0.03,
		"arm_swing": 0.24, "arm_carry": 0.34, "arm_fwd": 0.10, "arm_twist": 0.30,
		"elbow": 0.34, "elbow_swing": 0.14,
		"knee_lift": 0.72, "knee_stance": 0.16, "heel": 0.30, "toe": 0.48,
		"bob": 0.014, "settle": 0.018, "bounce": 0.30,
		"walk_speed": 1.45, "run_speed": 5.4, "idle_shift": 0.6,
	},
	# The jock's build, twenty years of not hurrying. Longer stride, slower
	# cadence, more weight in every plant, and he leans back into it.
	"muscle_daddy": {
		"stride": 0.95, "track": 0.150, "cross": -0.030, "toe_out": 0.22,
		"hip_sway": 0.026, "hip_drop": 0.035, "pelvis_yaw": 0.08,
		"chest_counter": 0.05, "shoulder_roll": 0.10, "lean": 0.01,
		"chest_lift": 0.09, "chin": 0.05,
		"arm_swing": 0.26, "arm_carry": 0.31, "arm_fwd": 0.08, "arm_twist": 0.26,
		"elbow": 0.30, "elbow_swing": 0.16,
		"knee_lift": 0.66, "knee_stance": 0.18, "heel": 0.26, "toe": 0.44,
		"bob": 0.013, "settle": 0.023, "bounce": 0.20,
		"walk_speed": 1.30, "run_speed": 4.8, "idle_shift": 0.5,
	},
	# A waddle is not a wobble. The mass is too wide to swing through the
	# midline, so instead of passing the leg under the body he rolls the body
	# over the leg — big lateral roll, short stride, wide track, and a real
	# drop onto each foot. The shoulders rock because the chest does not fight it.
	"bear": {
		"stride": 0.66, "track": 0.175, "cross": -0.055, "toe_out": 0.26,
		"hip_sway": 0.040, "hip_drop": 0.115, "pelvis_yaw": 0.06,
		"chest_counter": 0.04, "shoulder_roll": 0.055, "waddle": 0.105,
		"lean": 0.03, "chest_lift": 0.02, "chin": 0.02,
		"arm_swing": 0.26, "arm_carry": 0.30, "arm_fwd": 0.13, "arm_twist": 0.14,
		"elbow": 0.26, "elbow_swing": 0.16,
		"knee_lift": 0.58, "knee_stance": 0.22, "heel": 0.22, "toe": 0.38,
		"bob": 0.013, "settle": 0.030, "bounce": 0.12,
		"walk_speed": 1.20, "run_speed": 4.2, "idle_shift": 0.7,
	},
	# The waddle with the brakes off.
	"cub": {
		"stride": 0.68, "track": 0.150, "cross": -0.040, "toe_out": 0.22,
		"hip_sway": 0.038, "hip_drop": 0.105, "pelvis_yaw": 0.11,
		"chest_counter": 0.10, "shoulder_roll": 0.06, "waddle": 0.085,
		"lean": 0.05, "arm_swing": 0.40, "arm_carry": 0.22, "arm_fwd": 0.09,
		"elbow": 0.24, "elbow_swing": 0.26,
		"knee_lift": 0.76, "knee_stance": 0.18, "heel": 0.30, "toe": 0.48,
		"bob": 0.024, "settle": 0.020, "bounce": 0.40,
		"walk_speed": 1.45, "run_speed": 4.9, "idle_shift": 1.2,
	},
	# Unhurried and slightly amused. A swagger is shoulder roll without the
	# bracing — the one thing the jock's lats will not let him do.
	"daddy": {
		"stride": 0.90, "track": 0.115, "toe_out": 0.16,
		"hip_sway": 0.024, "hip_drop": 0.055, "pelvis_yaw": 0.12,
		"chest_counter": 0.12, "shoulder_roll": 0.085, "lean": 0.03,
		"chest_lift": 0.04, "chin": 0.03,
		"arm_swing": 0.36, "arm_carry": 0.16, "arm_fwd": 0.05, "arm_twist": 0.14,
		"elbow": 0.26, "elbow_swing": 0.22,
		"knee_lift": 0.76, "knee_stance": 0.14, "heel": 0.32, "toe": 0.52,
		"bob": 0.018, "settle": 0.014, "bounce": 0.28,
		"walk_speed": 1.35, "run_speed": 5.0, "idle_shift": 0.8, "idle_cock": 0.06,
	},
	# Long shanks, no mass to carry, so the cycle is slow and enormous: a huge
	# stride, the knee coming up to the chest, near-straight arms swinging in
	# full arcs, and real hang time between plants. The head arrives first and
	# the rest of him catches up.
	"lanky": {
		"stride": 1.22, "track": 0.075, "cross": 0.020, "toe_out": 0.04,
		"hip_sway": 0.014, "hip_drop": 0.045, "pelvis_yaw": 0.17,
		"chest_counter": 0.20, "shoulder_roll": 0.04,
		"lean": 0.11, "chin": -0.06, "head_lead": 0.16,
		"arm_swing": 0.78, "arm_carry": 0.11, "arm_fwd": -0.04,
		"elbow": 0.06, "elbow_swing": 0.12, "wrist": -0.22, "hand_splay": 0.42,
		"knee_lift": 1.32, "knee_stance": 0.06, "heel": 0.30, "toe": 0.78,
		"bob": 0.034, "settle": 0.005, "bounce": 0.95,
		"walk_speed": 1.85, "run_speed": 7.0, "idle_shift": 1.1,
	},
}

static func make(archetype := "otter") -> Dictionary:
	var g: Dictionary = AXES.duplicate()
	for k in ARCHETYPES.get(archetype, {}):
		g[k] = ARCHETYPES[archetype][k]
	g["name"] = archetype
	return g

static func blend(a: Dictionary, b: Dictionary, t: float) -> Dictionary:
	var out := {}
	for k in a:
		var va = a[k]
		if typeof(va) == TYPE_FLOAT or typeof(va) == TYPE_INT:
			out[k] = lerpf(float(va), float(b.get(k, va)), t)
		else:
			out[k] = va if t < 0.5 else b.get(k, va)
	out["name"] = a.get("name", "?") if t < 0.5 else b.get("name", "?")
	return out

## A gait for a body that matches no archetype.
##
## The build decides most of it, so a slider-built character still moves like
## the body he has: mass widens the track and shortens the stride, muscle holds
## the arms out and locks the thorax, limb length lengthens the stride and slows
## the cadence. Nobody has to author a gait to get a plausible one.
static func for_body(b: Dictionary) -> Dictionary:
	var mass: float = float(b["mass"])
	var mus: float = float(b["muscle"])
	var soft: float = float(b["soft"])
	var limb: float = float(b["limb"])
	var g: Dictionary = make("otter")
	var bulk: float = maxf(mus, soft * 0.85)
	g["stride"] = 0.70 + limb * 0.42 - mass * 0.16
	g["track"] = 0.050 + bulk * 0.090 + mass * 0.045
	g["cross"] = 0.035 - bulk * 0.075 - mass * 0.020
	g["toe_out"] = 0.05 + bulk * 0.16
	g["hip_sway"] = 0.036 - limb * 0.020 + soft * 0.012
	g["hip_drop"] = 0.13 - mus * 0.10 + soft * 0.05
	g["pelvis_yaw"] = 0.22 - bulk * 0.15
	# The lats: arms off the ribs, thorax locked to the pelvis.
	g["chest_counter"] = 0.24 - mus * 0.21
	g["arm_carry"] = 0.03 + mus * 0.31 + soft * 0.14
	g["arm_fwd"] = 0.01 + soft * 0.12 + mus * 0.06
	g["arm_twist"] = mus * 0.30
	g["arm_swing"] = 0.66 - bulk * 0.40
	g["shoulder_roll"] = 0.03 + mus * 0.09
	g["waddle"] = maxf(soft - 0.55, 0.0) * 0.26
	g["knee_lift"] = 0.58 + limb * 0.62 - mass * 0.22
	g["knee_stance"] = 0.06 + mass * 0.16
	g["settle"] = 0.004 + mass * 0.026
	g["bob"] = 0.030 - mass * 0.018
	g["bounce"] = clampf(0.95 - mass * 0.85, 0.08, 0.95)
	g["walk_speed"] = 1.15 + limb * 0.60 - mass * 0.15
	g["run_speed"] = 4.0 + limb * 3.0 - mass * 0.9
	g["name"] = "derived"
	return g

# -------------------------------------------------------------------- pose --

## A movement placed AT a phase, wrapped so events near 0 survive the seam.
static func _bump(p: float, at: float, width: float) -> float:
	var d: float = wrapf(p - at, -PI, PI) / width
	return exp(-0.5 * d * d)

## How far the phase advances for `dist` metres travelled.
##
## Stride lengthens with speed up to a point and then cadence takes over — the
## same trade a person makes, and the reason a run is not a fast walk.
static func advance(phase: float, g: Dictionary, stature: float, speed: float, dt: float) -> float:
	var ws: float = maxf(float(g["walk_speed"]), 0.2)
	var frac: float = clampf(speed / ws, 0.0, 2.4)
	var stride: float = stature * float(g["stride"]) * (0.62 + 0.38 * minf(frac, 1.0)
			+ 0.30 * maxf(frac - 1.0, 0.0))
	return phase + (speed * dt / maxf(stride, 0.05)) * TAU

## Pose the rig. `phase` from `advance`, `speed` in m/s, `t` seconds for idle.
static func pose(rig: Dictionary, g: Dictionary, phase: float, speed: float,
		t: float, look_pitch := 0.0) -> void:
	if rig.is_empty() or not rig.has("hip"):
		return
	var m: Dictionary = rig["measure"]
	var stature: float = float(m["stature"])
	var ws: float = maxf(float(g["walk_speed"]), 0.2)
	var rs: float = maxf(float(g["run_speed"]), ws + 0.4)
	# `moving` ramps the whole gait in off a standstill; `run` blends the walk
	# into the run. Two separate weights, because a slow walk is not a small run.
	var moving: float = clampf(speed / (ws * 0.55), 0.0, 1.0)
	var eff: float = clampf(speed / ws, 0.0, 2.2)
	var run: float = clampf((speed - ws * 1.05) / (rs - ws * 1.05), 0.0, 1.0)
	var amp: float = moving * (0.72 + 0.42 * minf(eff, 1.6))

	var idle: float = 1.0 - moving
	var breath: float = sin(t * 1.15)
	var sway_slow: float = sin(t * 0.47) * float(g["idle_shift"]) * idle

	# --- pelvis -----------------------------------------------------------
	var hip: Node3D = rig["hip"]
	var sway: float = sin(phase) * float(g["hip_sway"]) * stature * amp
	# The figure-eight: a second lateral beat at twice the stride. This, not
	# amplitude, is what separates a sway from a switch.
	sway += sin(phase * 2.0 + 0.6) * float(g["hip_sway"]) * stature * 0.45 \
			* float(g["swish"]) * amp
	var bob: float = -float(g["bob"]) * stature * (0.5 + 0.5 * cos(phase * 2.0)) * amp
	# Weight arriving: a dip just after each contact, once per foot.
	bob -= float(g["settle"]) * stature * amp \
			* (_bump(phase, 0.42, 0.55) + _bump(phase, 0.42 + PI, 0.55))
	# Flight: at a run the whole body leaves the ground between plants.
	bob += float(g["bounce"]) * run * float(g["bob"]) * stature * 1.9 \
			* maxf(absf(sin(phase)) - 0.35, 0.0)
	hip.position = Vector3(sway + sway_slow * 0.006 * stature, float(m["hip_y"]) + bob, 0.0)
	hip.rotation = Vector3(
		0.0,
		-float(g["pelvis_yaw"]) * cos(phase) * amp,
		# Drop on the swing side, plus the whole-body roll of a waddle.
		(float(g["hip_drop"]) * amp + float(g["idle_cock"]) * idle) * sin(phase + PI * 0.5 * idle)
			+ float(g["waddle"]) * sin(phase) * amp)

	# --- spine, chest, head ----------------------------------------------
	var lean: float = float(g["lean"]) * (0.5 + 0.9 * eff) + run * 0.16
	rig["spine"].rotation = Vector3(lean, 0.0, 0.0)
	rig["spine"].scale = Vector3(1.0, 1.0 + breath * 0.010 * idle, 1.0)
	var chest: Node3D = rig["chest"]
	chest.rotation = Vector3(
		float(g["chest_lift"]) * -1.0,
		float(g["chest_counter"]) * cos(phase) * amp,
		# The chest only partly resists the pelvis; what it fails to cancel is
		# the shoulder rock you see on a heavy walk.
		-float(g["waddle"]) * sin(phase) * amp * 0.45)
	rig["neck"].rotation = Vector3(float(g["head_lead"]) * (0.4 + 0.6 * eff), 0.0, 0.0)
	rig["head"].rotation = Vector3(
		clampf(look_pitch * 0.32, -0.5, 0.5) - float(g["chin"]) - float(g["head_lead"]) * 0.55,
		-float(g["chest_counter"]) * cos(phase) * amp * 0.4 + sway_slow * 0.04,
		0.0)

	# --- limbs ------------------------------------------------------------
	for side in [-1.0, 1.0]:
		var key: int = int(side)
		var lp: float = phase if side > 0.0 else phase + PI

		# Thigh: forward at contact, back at toe-off, driven through the swing.
		var flex: float = float(g["knee_lift"]) * 0.40 * cos(lp) * amp
		flex += float(g["knee_lift"]) * 0.30 * _bump(lp, PI * 1.45, 0.9) * amp * (0.6 + 0.7 * run)
		var thigh: Node3D = rig["thigh%d" % key]
		# Track and crossover: where the foot lands relative to the hip.
		var stance: float = clampf(sin(lp), 0.0, 1.0)
		var track_x: float = side * (float(g["track"]) * 0.5 - float(g["cross"]) * stance) * stature
		thigh.position = Vector3(track_x, 0.0, 0.0)
		thigh.rotation = Vector3(-flex, side * float(g["toe_out"]) * 0.5, 0.0)

		# Knee: loaded just after contact, folded through mid-swing.
		var knee: float = float(g["knee_stance"]) * _bump(lp, 0.45, 0.6) * amp
		knee += float(g["knee_lift"]) * (0.85 + 0.5 * run) * _bump(lp, PI * 1.42, 0.85) * amp
		knee += 0.04
		rig["shin%d" % key].rotation = Vector3(knee, 0.0, 0.0)

		# Foot: toes up to meet the ground, hard push to leave it.
		var toes: float = float(g["heel"]) * _bump(lp, -0.30, 0.55) * amp
		toes += float(g["heel"]) * 0.45 * _bump(lp, PI * 1.35, 0.7) * amp
		var push: float = float(g["toe"]) * _bump(lp, PI - 0.22, 0.42) * amp * (1.0 + run * 0.5)
		rig["foot%d" % key].rotation = Vector3(-toes + push - knee * 0.55 + flex * 0.35,
				side * float(g["toe_out"]), 0.0)

		# Clavicle: the see-saw. On a braced torso this is most of the motion.
		rig["clav%d" % key].rotation = Vector3(
			0.0,
			float(g["arm_swing"]) * 0.18 * cos(lp) * amp,
			side * (float(g["shoulder_roll"]) * sin(phase) * amp))

		# Upper arm. Contralateral: this arm goes BACK as this leg goes forward,
		# which is `cos(lp)` with no phase offset — positive rotation.x carries
		# a hanging limb backward.
		#
		# `arm_carry` is not decoration: it is why a big man's arms swing a
		# little and hang far from his ribs, and a slight man's swing a lot and
		# brush them. Composed as an explicit basis — leaving it to Euler order
		# let the twist rotate the swing plane, and the forearms flew outward.
		var back: float = float(g["arm_swing"]) * cos(lp) * amp * (1.0 + run * 0.45)
		var carry: float = float(g["arm_carry"]) + float(g["arm_carry"]) * 0.25 * idle
		rig["arm%d" % key].basis = Basis(Vector3(0, 0, 1), side * carry) \
				* Basis(Vector3(1, 0, 0), back - float(g["arm_fwd"]) + breath * 0.006 * idle)
		# Elbow folds on the FORWARD swing, and locks toward 90° at a run.
		var elbow: float = float(g["elbow"]) + float(g["elbow_swing"]) \
				* clampf(-cos(lp), 0.0, 1.0) * amp
		elbow += run * (1.35 - float(g["elbow"])) * 0.75
		# Pronation lives at the forearm, and spins it about ITS OWN axis — so
		# the bend comes first. The other order swung a bent forearm sideways
		# and crossed both hands over the crotch.
		rig["fore%d" % key].basis = Basis(Vector3(1, 0, 0), -elbow) \
				* Basis(Vector3(0, 1, 0), side * float(g["arm_twist"]))
		rig["hand%d" % key].rotation = Vector3(
			-float(g["wrist"]) * (0.6 + 0.4 * float(g["swish"])),
			side * float(g["hand_splay"]),
			side * float(g["wrist"]) * float(g["swish"]) * 0.8)
