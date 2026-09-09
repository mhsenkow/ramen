extends RefCounted
## Every binding in one place, registered into Godot's InputMap so the game
## never reads a raw keycode. That is what makes rebinding and gamepad support
## fall out for free instead of being a second input path to maintain.

const CFG := "user://controls.cfg"

const ACTIONS := [
	{"id": "forward",   "label": "Move forward",   "key": KEY_W,      "axis": [JOY_AXIS_LEFT_Y, -1.0]},
	{"id": "back",      "label": "Move back",      "key": KEY_S,      "axis": [JOY_AXIS_LEFT_Y, 1.0]},
	{"id": "left",      "label": "Strafe left",    "key": KEY_A,      "axis": [JOY_AXIS_LEFT_X, -1.0]},
	{"id": "right",     "label": "Strafe right",   "key": KEY_D,      "axis": [JOY_AXIS_LEFT_X, 1.0]},
	{"id": "jump",      "label": "Jump",           "key": KEY_SPACE,  "btn": JOY_BUTTON_A},
	{"id": "run",       "label": "Run",            "key": KEY_SHIFT,  "btn": JOY_BUTTON_LEFT_STICK},
	{"id": "dig",       "label": "Excavate",       "mouse": MOUSE_BUTTON_LEFT,  "axis": [JOY_AXIS_TRIGGER_RIGHT, 1.0]},
	{"id": "place",     "label": "Install module", "key": KEY_G,      "axis": [JOY_AXIS_TRIGGER_LEFT, 1.0]},
	{"id": "fill",      "label": "Add material",   "key": KEY_R,      "btn": JOY_BUTTON_X},
	{"id": "undo",      "label": "Undo excavation","key": KEY_Z,      "btn": JOY_BUTTON_B},
	{"id": "brush_up",  "label": "Brush larger",   "key": KEY_E,      "btn": JOY_BUTTON_RIGHT_SHOULDER},
	{"id": "brush_down","label": "Brush smaller",  "key": KEY_Q,      "btn": JOY_BUTTON_LEFT_SHOULDER},
	{"id": "level",     "label": "Brush shape",    "key": KEY_C,      "btn": JOY_BUTTON_Y},
	{"id": "waypoint",  "label": "Drop waypoint",  "key": KEY_B,      "btn": JOY_BUTTON_DPAD_UP},
	{"id": "map_in",    "label": "Plan zoom in",   "key": KEY_EQUAL,  "btn": JOY_BUTTON_DPAD_RIGHT},
	{"id": "map_out",   "label": "Plan zoom out",  "key": KEY_MINUS,  "btn": JOY_BUTTON_DPAD_LEFT},
	{"id": "view",      "label": "Switch view",    "key": KEY_TAB,    "btn": JOY_BUTTON_RIGHT_STICK},
	# No gamepad button left free; rebindable from the menu like any other.
	{"id": "info",      "label": "Field book",     "key": KEY_I},
	{"id": "map_mode",  "label": "Map: 3D / flat", "key": KEY_O},
	{"id": "map_small", "label": "Maps smaller",   "key": KEY_COMMA},
	{"id": "map_big",   "label": "Maps larger",    "key": KEY_PERIOD},
	{"id": "throw",     "label": "Throw object",   "key": KEY_T,      "btn": JOY_BUTTON_DPAD_DOWN},
	{"id": "harvest",   "label": "Fell tree",      "key": KEY_H,      "btn": JOY_BUTTON_MISC1},
	{"id": "drop",      "label": "Drop pack",      "key": KEY_X,      "btn": JOY_BUTTON_BACK},
	{"id": "take",      "label": "Take from heap",  "key": KEY_L,      "btn": JOY_BUTTON_GUIDE},
	{"id": "craft",     "label": "Craft recipe",   "key": KEY_K,      "btn": JOY_BUTTON_PADDLE1},
	{"id": "craft_prev","label": "Prev recipe",    "key": KEY_BRACKETLEFT, "btn": JOY_BUTTON_PADDLE2},
	{"id": "craft_next","label": "Next recipe",    "key": KEY_BRACKETRIGHT, "btn": JOY_BUTTON_PADDLE3},
	{"id": "amend",     "label": "Amend soil",     "key": KEY_M,      "btn": JOY_BUTTON_PADDLE4},
	{"id": "scrub",     "label": "Toggle scrubber","key": KEY_U,      "btn": JOY_BUTTON_TOUCHPAD},
	{"id": "eat",       "label": "Eat ramen",      "key": KEY_J,      "btn": JOY_BUTTON_MISC1},
	{"id": "station",   "label": "Place station",  "key": KEY_Y,      "btn": JOY_BUTTON_GUIDE},
	{"id": "tilt",      "label": "Tilt-shift",     "key": KEY_P,      "btn": JOY_BUTTON_TOUCHPAD},
	{"id": "pause",     "label": "Menu",           "key": KEY_ESCAPE, "btn": JOY_BUTTON_START},
	{"id": "mod1",      "label": "Module 1",       "key": KEY_4,      "btn": JOY_BUTTON_PADDLE1},
	{"id": "mod2",      "label": "Module 2",       "key": KEY_5,      "btn": JOY_BUTTON_PADDLE2},
	{"id": "mod3",      "label": "Module 3",       "key": KEY_6,      "btn": JOY_BUTTON_PADDLE3},
	{"id": "mod4",      "label": "Module 4",       "key": KEY_7,      "btn": JOY_BUTTON_PADDLE4},
	{"id": "speed1",    "label": "Speed gear 1",   "key": KEY_1,      "btn": -1},
	{"id": "speed2",    "label": "Speed gear 2",   "key": KEY_2,      "btn": -1},
	{"id": "speed3",    "label": "Speed gear 3",   "key": KEY_3,      "btn": -1},
	{"id": "save",      "label": "Save digs",      "key": KEY_F5,     "btn": JOY_BUTTON_BACK},
	{"id": "load",      "label": "Load digs",      "key": KEY_F9,     "btn": JOY_BUTTON_GUIDE},
	{"id": "bottle",    "label": "Scoop / pour water", "key": KEY_V, "btn": JOY_BUTTON_MISC1},
	{"id": "ignite",    "label": "Ignite / fire",      "key": KEY_F, "btn": -1},
	{"id": "douse",     "label": "Douse fire",         "key": KEY_9, "btn": -1},
	{"id": "pour_lava", "label": "Pour lava",          "key": KEY_0, "btn": -1},
	{"id": "soil",      "label": "Soil overlay",   "key": KEY_N,      "btn": JOY_BUTTON_TOUCHPAD},
	{"id": "photo",     "label": "Photo mode",     "key": KEY_F11,    "btn": JOY_BUTTON_MISC1},
	{"id": "quiet",     "label": "Quiet mode",     "key": KEY_F10,    "btn": JOY_BUTTON_TOUCHPAD},
	{"id": "fastday",   "label": "Fast day cycle", "key": KEY_F6,     "btn": -1},
	{"id": "god",       "label": "God mode (fly)", "key": KEY_F1,     "btn": -1},
	{"id": "body_prev", "label": "Previous build", "key": KEY_F2,     "btn": -1},
	{"id": "body_next", "label": "Next build",     "key": KEY_F3,     "btn": -1},
	{"id": "descend",   "label": "Descend / crouch","key": KEY_CTRL,  "btn": JOY_BUTTON_LEFT_SHOULDER},
]

static var overrides := {}
static var sensitivity := 0.0026
static var invert_y := false
static var fov := 62.0
static var reduced_motion := false
static var hud_density := "full"
static var font_scale := 1.0
static var photo_mode := false
static var mute_on_focus_loss := true
static var captions := true
static var vol_world := 1.0
static var vol_ui := 1.0
static var vol_music := 0.7
static var vol_voice := 1.0
static var quality := "high"  # high | low | deck
## When true, quality is chosen from GPU/CPU probes (and may drop further if FPS tanks).
static var quality_auto := true
static var quality_reason := ""
static var overlay_palette := "default"  # default | deuteranopia | protanopia | achroma
static var overlay_contours := true
static var archive_enabled := true  # content setting stub (PD §8)
static var romance_intensity := "full"  # full | fade_to_black | off
static var first_run_done := false
## What right-click does while looking: fill dirt, or install the selected module.
static var right_click := "fill"  # fill | place

static var quiet_mode := false

## Pick a quality tier from export flags + GPU/CPU. Safe default for friends on
## weak laptops: prefer something that boots over something pretty.
static func detect_quality() -> String:
	if OS.has_feature("deck"):
		quality_reason = "Steam Deck export"
		return "deck"
	if OS.has_feature("lowspec"):
		quality_reason = "low-spec export"
		return "low"
	if OS.has_feature("mobile"):
		quality_reason = "mobile"
		return "low"

	var cores := OS.get_processor_count()
	var gpu := RenderingServer.get_video_adapter_name()
	var name := gpu.to_lower()
	var dtype := RenderingServer.get_video_adapter_type()

	# Apple Silicon is "integrated" but strong — don't punish M-series.
	if "apple m" in name or name.begins_with("apple "):
		# Base M1 (8-core) still fine at deck; Pro/Max/Ultra stay high.
		if "m1" in name and "pro" not in name and "max" not in name and "ultra" not in name:
			quality_reason = "Apple M1"
			return "deck"
		quality_reason = "Apple Silicon (%s)" % gpu
		return "high"

	# Software / VM / known-weak iGPUs — boot first.
	for t in [
		"llvmpipe", "swiftshader", "microsoft basic", "gdi generic",
		"uhd graphics", "hd graphics", "iris plus", "radeon vega",
		"radeon graphics", "mali-", "adreno", "intel(r) hd",
	]:
		if t in name:
			quality_reason = "weak GPU (%s)" % gpu
			return "low"
	if "iris xe" in name:
		quality_reason = "Intel Iris Xe"
		return "deck"

	if dtype == RenderingDevice.DEVICE_TYPE_CPU:
		quality_reason = "software renderer"
		return "low"
	if dtype == RenderingDevice.DEVICE_TYPE_INTEGRATED_GPU:
		quality_reason = "integrated GPU (%s)" % gpu
		return "deck" if cores >= 6 else "low"

	if cores <= 2:
		quality_reason = "%d CPU cores" % cores
		return "low"
	if cores <= 4:
		quality_reason = "%d CPU cores" % cores
		return "deck"

	var sz := DisplayServer.screen_get_size()
	if sz.x * sz.y >= 3840 * 2160 and cores < 8:
		quality_reason = "4K on mid CPU"
		return "deck"

	quality_reason = "GPU OK (%s, %d cores)" % [gpu, cores]
	return "high"

## The player's build. An archetype, optionally blended toward a second one,
## then any individual axes moved by hand — the whole of `avatar/body.gd`'s
## space is reachable and all three layers persist. `axes` wins over the blend
## so a hand-set number stays where it was put.
static var avatar := {
	"archetype": "daddy",
	"blend_to": "",
	"blend": 0.0,
	"axes": {},
}

static func act(id: String) -> String:
	return "rama_" + id

static func cfg_exists() -> bool:
	return FileAccess.file_exists(CFG)

static func install() -> void:
	load_cfg()
	# Fire moved to F; dig is mouse-only. Drop stale overrides that would
	# steal F back for excavate or leave ignite on the old 8.
	if int(overrides.get("dig", -1)) == KEY_F:
		overrides.erase("dig")
	if int(overrides.get("ignite", -1)) == KEY_8:
		overrides.erase("ignite")
	for a in ACTIONS:
		var name := act(a["id"])
		if InputMap.has_action(name):
			InputMap.erase_action(name)
		InputMap.add_action(name, 0.25)
		if a.has("key") or overrides.has(a["id"]):
			var k := int(overrides.get(a["id"], a.get("key", KEY_NONE)))
			if k != KEY_NONE and k > 0:
				var ev := InputEventKey.new()
				ev.physical_keycode = k as Key
				InputMap.action_add_event(name, ev)
		if a.has("mouse"):
			var m := InputEventMouseButton.new()
			m.button_index = a["mouse"] as MouseButton
			InputMap.action_add_event(name, m)
		elif (a["id"] == "fill" and right_click == "fill") \
				or (a["id"] == "place" and right_click == "place"):
			var m := InputEventMouseButton.new()
			m.button_index = MOUSE_BUTTON_RIGHT
			InputMap.action_add_event(name, m)
		if a.has("btn") and int(a["btn"]) >= 0:
			var b := InputEventJoypadButton.new()
			b.button_index = a["btn"] as JoyButton
			InputMap.action_add_event(name, b)
		if a.has("axis"):
			var j := InputEventJoypadMotion.new()
			j.axis = a["axis"][0] as JoyAxis
			j.axis_value = a["axis"][1]
			InputMap.action_add_event(name, j)
	# First launch (or quality set to Auto): probe hardware. Export presets win.
	if OS.has_feature("deck") or OS.has_feature("lowspec"):
		if quality_auto or quality == "high":
			quality = "deck" if OS.has_feature("deck") else "low"
			quality_auto = true
			quality_reason = "export preset"
	elif quality_auto:
		quality = detect_quality()

static func rebind(id: String, keycode: int) -> void:
	overrides[id] = keycode
	install()
	save_cfg()

static func binding_name(id: String) -> String:
	for a in ACTIONS:
		if a["id"] == id:
			var parts: PackedStringArray = PackedStringArray()
			if a.has("key") or overrides.has(id):
				var k := int(overrides.get(id, a.get("key", KEY_NONE)))
				if k != KEY_NONE and k > 0:
					parts.append(OS.get_keycode_string(k))
			if a.has("mouse"):
				parts.append("Left click" if a["mouse"] == MOUSE_BUTTON_LEFT else "Right click")
			elif (id == "fill" and right_click == "fill") \
					or (id == "place" and right_click == "place"):
				parts.append("Right click")
			if parts.is_empty():
				return "?"
			return "  /  ".join(parts)
	return "?"

static func save_cfg() -> void:
	var c := ConfigFile.new()
	for id in overrides:
		c.set_value("keys", id, overrides[id])
	c.set_value("look", "sensitivity", sensitivity)
	c.set_value("look", "invert_y", invert_y)
	c.set_value("look", "fov", fov)
	c.set_value("a11y", "reduced_motion", reduced_motion)
	c.set_value("a11y", "hud_density", hud_density)
	c.set_value("a11y", "font_scale", font_scale)
	c.set_value("a11y", "mute_on_focus_loss", mute_on_focus_loss)
	c.set_value("a11y", "captions", captions)
	c.set_value("audio", "world", vol_world)
	c.set_value("audio", "ui", vol_ui)
	c.set_value("audio", "music", vol_music)
	c.set_value("audio", "voice", vol_voice)
	c.set_value("gfx", "quality", quality)
	c.set_value("gfx", "quality_auto", quality_auto)
	c.set_value("gfx", "overlay_palette", overlay_palette)
	c.set_value("gfx", "overlay_contours", overlay_contours)
	c.set_value("content", "archive_enabled", archive_enabled)
	c.set_value("content", "romance_intensity", romance_intensity)
	c.set_value("look", "right_click", right_click)
	c.set_value("avatar", "archetype", avatar["archetype"])
	c.set_value("avatar", "blend_to", avatar["blend_to"])
	c.set_value("avatar", "blend", avatar["blend"])
	c.set_value("avatar", "axes", avatar["axes"])
	c.set_value("meta", "first_run_done", true)
	c.save(CFG)
	first_run_done = true

static func load_cfg() -> void:
	var c := ConfigFile.new()
	if c.load(CFG) != OK:
		first_run_done = false
		quality_auto = true
		return
	first_run_done = bool(c.get_value("meta", "first_run_done", true))
	overrides = {}
	if c.has_section("keys"):
		for id in c.get_section_keys("keys"):
			overrides[id] = c.get_value("keys", id)
	sensitivity = c.get_value("look", "sensitivity", sensitivity)
	invert_y = c.get_value("look", "invert_y", invert_y)
	fov = c.get_value("look", "fov", fov)
	reduced_motion = c.get_value("a11y", "reduced_motion", reduced_motion)
	hud_density = str(c.get_value("a11y", "hud_density", hud_density))
	font_scale = float(c.get_value("a11y", "font_scale", font_scale))
	mute_on_focus_loss = c.get_value("a11y", "mute_on_focus_loss", mute_on_focus_loss)
	captions = c.get_value("a11y", "captions", captions)
	vol_world = float(c.get_value("audio", "world", vol_world))
	vol_ui = float(c.get_value("audio", "ui", vol_ui))
	vol_music = float(c.get_value("audio", "music", vol_music))
	vol_voice = float(c.get_value("audio", "voice", vol_voice))
	quality = str(c.get_value("gfx", "quality", quality))
	# Old configs have no quality_auto key — keep their saved tier (don't re-probe).
	quality_auto = bool(c.get_value("gfx", "quality_auto", false))
	overlay_palette = str(c.get_value("gfx", "overlay_palette", overlay_palette))
	overlay_contours = bool(c.get_value("gfx", "overlay_contours", overlay_contours))
	archive_enabled = bool(c.get_value("content", "archive_enabled", archive_enabled))
	romance_intensity = str(c.get_value("content", "romance_intensity", romance_intensity))
	avatar = {
		"archetype": str(c.get_value("avatar", "archetype", "daddy")),
		"blend_to": str(c.get_value("avatar", "blend_to", "")),
		"blend": float(c.get_value("avatar", "blend", 0.0)),
		"axes": c.get_value("avatar", "axes", {}),
	}
	right_click = str(c.get_value("look", "right_click", right_click))
	if right_click not in ["fill", "place"]:
		right_click = "fill"

static func reset() -> void:
	avatar = {"archetype": "daddy", "blend_to": "", "blend": 0.0, "axes": {}}
	overrides = {}
	sensitivity = 0.0026
	invert_y = false
	fov = 62.0
	reduced_motion = false
	hud_density = "full"
	font_scale = 1.0
	mute_on_focus_loss = true
	photo_mode = false
	captions = true
	vol_world = 1.0
	vol_ui = 1.0
	vol_music = 0.7
	vol_voice = 1.0
	quality_auto = true
	quality = detect_quality()
	overlay_palette = "default"
	overlay_contours = true
	archive_enabled = true
	romance_intensity = "full"
	right_click = "fill"
	install()
	save_cfg()

## Remap overlay RGB for colour-vision deficiency (§2106).
static func remap_overlay_rgb(r: float, g: float, b: float) -> Color:
	match overlay_palette:
		"deuteranopia":
			return Color(0.625 * r + 0.375 * g, 0.70 * r + 0.30 * g, b)
		"protanopia":
			return Color(0.567 * r + 0.433 * g, 0.558 * r + 0.442 * g, 0.242 * g + 0.758 * b)
		"achroma":
			var y := 0.299 * r + 0.587 * g + 0.114 * b
			return Color(y, y, y)
		_:
			return Color(r, g, b)
