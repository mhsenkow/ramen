extends RefCounted
## Every binding in one place, registered into Godot's InputMap so the game
## never reads a raw keycode. That is what makes rebinding and gamepad support
## fall out for free instead of being a second input path to maintain.

const CFG := "user://controls.cfg"

# id, label, default key, default joypad button (or -1), joypad axis spec
const ACTIONS := [
	{"id": "forward",   "label": "Move forward",   "key": KEY_W,      "axis": [JOY_AXIS_LEFT_Y, -1.0]},
	{"id": "back",      "label": "Move back",      "key": KEY_S,      "axis": [JOY_AXIS_LEFT_Y, 1.0]},
	{"id": "left",      "label": "Strafe left",    "key": KEY_A,      "axis": [JOY_AXIS_LEFT_X, -1.0]},
	{"id": "right",     "label": "Strafe right",   "key": KEY_D,      "axis": [JOY_AXIS_LEFT_X, 1.0]},
	{"id": "jump",      "label": "Jump",           "key": KEY_SPACE,  "btn": JOY_BUTTON_A},
	{"id": "run",       "label": "Run",            "key": KEY_SHIFT,  "btn": JOY_BUTTON_LEFT_STICK},
	{"id": "dig",       "label": "Excavate",       "key": KEY_F,      "mouse": MOUSE_BUTTON_LEFT,  "axis": [JOY_AXIS_TRIGGER_RIGHT, 1.0]},
	{"id": "place",     "label": "Install module", "key": KEY_G,      "mouse": MOUSE_BUTTON_RIGHT, "axis": [JOY_AXIS_TRIGGER_LEFT, 1.0]},
	{"id": "fill",      "label": "Add material",   "key": KEY_R,      "btn": JOY_BUTTON_X},
	{"id": "undo",      "label": "Undo excavation","key": KEY_Z,      "btn": JOY_BUTTON_B},
	{"id": "brush_up",  "label": "Brush larger",   "key": KEY_E,      "btn": JOY_BUTTON_RIGHT_SHOULDER},
	{"id": "brush_down","label": "Brush smaller",  "key": KEY_Q,      "btn": JOY_BUTTON_LEFT_SHOULDER},
	{"id": "level",     "label": "Brush shape",    "key": KEY_C,      "btn": JOY_BUTTON_Y},
	{"id": "waypoint",  "label": "Drop waypoint",  "key": KEY_B,      "btn": JOY_BUTTON_DPAD_UP},
	{"id": "map_in",    "label": "Plan zoom in",   "key": KEY_EQUAL,  "btn": JOY_BUTTON_DPAD_RIGHT},
	{"id": "map_out",   "label": "Plan zoom out",  "key": KEY_MINUS,  "btn": JOY_BUTTON_DPAD_LEFT},
	{"id": "view",      "label": "Switch view",    "key": KEY_TAB,    "btn": JOY_BUTTON_RIGHT_STICK},
	{"id": "throw",     "label": "Throw object",   "key": KEY_T,      "btn": JOY_BUTTON_DPAD_DOWN},
	{"id": "harvest",   "label": "Harvest plant",  "key": KEY_H,      "btn": JOY_BUTTON_DPAD_RIGHT},
	{"id": "drop",      "label": "Drop pack",      "key": KEY_X,      "btn": JOY_BUTTON_DPAD_LEFT},
	{"id": "craft",     "label": "Craft recipe",   "key": KEY_K,      "btn": -1},
	{"id": "craft_prev","label": "Prev recipe",    "key": KEY_BRACKETLEFT, "btn": -1},
	{"id": "craft_next","label": "Next recipe",    "key": KEY_BRACKETRIGHT, "btn": -1},
	{"id": "amend",     "label": "Amend soil",     "key": KEY_M,      "btn": -1},
	{"id": "scrub",     "label": "Toggle scrubber","key": KEY_U,      "btn": -1},
	{"id": "eat",       "label": "Eat ramen",      "key": KEY_J,      "btn": -1},
	{"id": "station",   "label": "Place station",  "key": KEY_Y,      "btn": -1},
	{"id": "tilt",      "label": "Tilt-shift",     "key": KEY_P,      "btn": -1},
	{"id": "pause",     "label": "Menu",           "key": KEY_ESCAPE, "btn": JOY_BUTTON_START},
	{"id": "mod1",      "label": "Module 1",       "key": KEY_1,      "btn": -1},
	{"id": "mod2",      "label": "Module 2",       "key": KEY_2,      "btn": -1},
	{"id": "mod3",      "label": "Module 3",       "key": KEY_3,      "btn": -1},
	{"id": "mod4",      "label": "Module 4",       "key": KEY_4,      "btn": -1},
	{"id": "save",      "label": "Save digs",      "key": KEY_F5,     "btn": -1},
	{"id": "load",      "label": "Load digs",      "key": KEY_F9,     "btn": -1},
	{"id": "bottle",    "label": "Scoop / pour water", "key": KEY_V, "btn": -1},
	{"id": "soil",      "label": "Soil overlay",   "key": KEY_N,      "btn": -1},
]

static var overrides := {}
static var sensitivity := 0.0026
static var invert_y := false
static var fov := 62.0

static func act(id: String) -> String:
	return "rama_" + id

static func install() -> void:
	load_cfg()
	for a in ACTIONS:
		var name := act(a["id"])
		if InputMap.has_action(name):
			InputMap.erase_action(name)
		InputMap.add_action(name, 0.25)
		var k := int(overrides.get(a["id"], a["key"]))
		var ev := InputEventKey.new()
		ev.physical_keycode = k as Key
		InputMap.action_add_event(name, ev)
		if a.has("mouse"):
			var m := InputEventMouseButton.new()
			m.button_index = a["mouse"] as MouseButton
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

static func rebind(id: String, keycode: int) -> void:
	overrides[id] = keycode
	install()
	save_cfg()

static func binding_name(id: String) -> String:
	for a in ACTIONS:
		if a["id"] == id:
			var k := int(overrides.get(id, a["key"]))
			var s := OS.get_keycode_string(k)
			if a.has("mouse"):
				s += "  /  " + ("Left click" if a["mouse"] == MOUSE_BUTTON_LEFT else "Right click")
			return s
	return "?"

static func save_cfg() -> void:
	var c := ConfigFile.new()
	for id in overrides:
		c.set_value("keys", id, overrides[id])
	c.set_value("look", "sensitivity", sensitivity)
	c.set_value("look", "invert_y", invert_y)
	c.set_value("look", "fov", fov)
	c.save(CFG)

static func load_cfg() -> void:
	var c := ConfigFile.new()
	if c.load(CFG) != OK:
		return
	overrides = {}
	if c.has_section("keys"):
		for id in c.get_section_keys("keys"):
			overrides[id] = c.get_value("keys", id)
	sensitivity = c.get_value("look", "sensitivity", sensitivity)
	invert_y = c.get_value("look", "invert_y", invert_y)
	fov = c.get_value("look", "fov", fov)

static func reset() -> void:
	overrides = {}
	sensitivity = 0.0026
	invert_y = false
	fov = 62.0
	install()
	save_cfg()
