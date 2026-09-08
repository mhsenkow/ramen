extends CanvasLayer
const RamaBody = preload("res://scripts/avatar/body.gd")
const RamaControls = preload("res://scripts/controls.gd")
## Pause menu: rebindable controls, look/a11y/audio/content settings.

var world
var panel: PanelContainer
var rows := {}
var listening := ""
var status: Label
var first_run_mode := false

func _ready() -> void:
	layer = 10
	process_mode = Node.PROCESS_MODE_ALWAYS
	visible = false

	var dim := ColorRect.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	dim.color = Color(0.02, 0.03, 0.05, 0.82)
	add_child(dim)

	panel = PanelContainer.new()
	panel.set_anchors_preset(Control.PRESET_CENTER)
	panel.position = Vector2(-400, -360)
	panel.custom_minimum_size = Vector2(800, 720)
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.06, 0.08, 0.11, 0.94)
	sb.set_corner_radius_all(6)
	sb.content_margin_left = 28
	sb.content_margin_right = 28
	sb.content_margin_top = 22
	sb.content_margin_bottom = 22
	panel.add_theme_stylebox_override("panel", sb)
	add_child(panel)

	var scroll := ScrollContainer.new()
	panel.add_child(scroll)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(720, 0)
	box.add_theme_constant_override("separation", 10)
	scroll.add_child(box)

	_head(box, "KEPLER DRUM — settings")
	_note(box, "Click a binding, then press a key. Gamepad bindings are fixed.")

	for a in RamaControls.ACTIONS:
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 16)
		var l := Label.new()
		l.text = a["label"]
		l.custom_minimum_size = Vector2(280, 0)
		l.add_theme_font_size_override("font_size", 16)
		row.add_child(l)
		var b := Button.new()
		b.text = RamaControls.binding_name(a["id"])
		b.custom_minimum_size = Vector2(300, 36)
		b.add_theme_font_size_override("font_size", 15)
		var id: String = a["id"]
		b.pressed.connect(func(): _listen(id))
		row.add_child(b)
		rows[id] = b
		box.add_child(row)

	# The character creator. Three layers, in the order they compose: pick a
	# build, lean it toward a second one, then move any individual number. The
	# camera is over his shoulder while this is open, so it is all live.
	_head(box, "Build")
	_note(box, "Archetype — the spectrum, slight to heavy")
	_option(box, RamaBody.ORDER, str(RamaControls.avatar.get("archetype", "daddy")), func(t):
		RamaControls.avatar["archetype"] = t
		RamaControls.avatar["axes"] = {}
		_apply_build())
	_note(box, "Blend toward")
	var blend_items: Array = ["none"] + Array(RamaBody.ORDER)
	var bt: String = str(RamaControls.avatar.get("blend_to", ""))
	_option(box, blend_items, bt if bt != "" else "none", func(t):
		RamaControls.avatar["blend_to"] = "" if t == "none" else t
		_apply_build())
	_slider(box, "Blend", 0.0, 1.0, float(RamaControls.avatar.get("blend", 0.0)), func(v):
		RamaControls.avatar["blend"] = v
		_apply_build())
	_note(box, "Or move any number yourself")
	_slider(box, "Height (m)", 1.52, 2.10, _axis("stature"), func(v):
		_set_axis("stature", v))
	for ax in ["mass", "muscle", "soft", "taper", "limb", "neck", "head", "hair", "beard", "fur"]:
		var label: String = ax.capitalize()
		_slider(box, label, 0.0, 1.0, _axis(ax), func(v): _set_axis(ax, v))

	_head(box, "Look")
	_note(box, "Right-click action")
	_option(box, ["fill", "place"], RamaControls.right_click, func(t):
		RamaControls.right_click = t
		RamaControls.install()
		RamaControls.save_cfg()
		_refresh_bindings())
	_slider(box, "Sensitivity", 0.0006, 0.0090, RamaControls.sensitivity,
		func(v): RamaControls.sensitivity = v; RamaControls.save_cfg())
	_slider(box, "Field of view", 50.0, 100.0, RamaControls.fov,
		func(v): RamaControls.fov = v; RamaControls.save_cfg())
	box.add_child(_check("Invert vertical look", RamaControls.invert_y, func(v):
		RamaControls.invert_y = v; RamaControls.save_cfg()))

	_head(box, "Accessibility")
	box.add_child(_check("Reduced motion (no sway)", RamaControls.reduced_motion, func(v):
		RamaControls.reduced_motion = v; RamaControls.save_cfg()))
	box.add_child(_check("Mute when window loses focus", RamaControls.mute_on_focus_loss, func(v):
		RamaControls.mute_on_focus_loss = v; RamaControls.save_cfg()))
	box.add_child(_check("Captions for meaningful sounds", RamaControls.captions, func(v):
		RamaControls.captions = v; RamaControls.save_cfg()))
	_note(box, "HUD density: full → compact → minimal → off")
	_option(box, ["full", "compact", "minimal", "off"], RamaControls.hud_density, func(t):
		RamaControls.hud_density = t; RamaControls.save_cfg())
	_slider(box, "Font scale", 0.85, 1.4, RamaControls.font_scale, func(v):
		RamaControls.font_scale = v
		RamaControls.save_cfg()
		if world and world.ui and world.ui.has_method("apply_font_scale"):
			world.ui.apply_font_scale(v))

	_head(box, "Audio")
	_slider(box, "World", 0.0, 1.0, RamaControls.vol_world, func(v):
		RamaControls.vol_world = v; RamaControls.save_cfg(); _audio_vols())
	_slider(box, "UI", 0.0, 1.0, RamaControls.vol_ui, func(v):
		RamaControls.vol_ui = v; RamaControls.save_cfg(); _audio_vols())
	_slider(box, "Music", 0.0, 1.0, RamaControls.vol_music, func(v):
		RamaControls.vol_music = v; RamaControls.save_cfg(); _audio_vols())
	_slider(box, "Voice", 0.0, 1.0, RamaControls.vol_voice, func(v):
		RamaControls.vol_voice = v; RamaControls.save_cfg(); _audio_vols())

	_head(box, "Graphics")
	_note(box, "Quality: high → low → deck (foliage / LOD first)")
	_option(box, ["high", "low", "deck"], RamaControls.quality, func(t):
		RamaControls.quality = t; RamaControls.save_cfg()
		if world and world.has_method("apply_quality"):
			world.apply_quality())
	_note(box, "Overlay palette (colour-blind safe)")
	_option(box, ["default", "deuteranopia", "protanopia", "achroma"],
			RamaControls.overlay_palette, func(t):
		RamaControls.overlay_palette = t; RamaControls.save_cfg()
		if world: world._refresh_soil_overlay())
	box.add_child(_check("Overlay contours / isolines", RamaControls.overlay_contours, func(v):
		RamaControls.overlay_contours = v; RamaControls.save_cfg()
		if world: world._refresh_soil_overlay()))

	_head(box, "Content (PD §8)")
	box.add_child(_check("Archive memories enabled", RamaControls.archive_enabled, func(v):
		RamaControls.archive_enabled = v; RamaControls.save_cfg()))
	_note(box, "Romance intensity")
	_option(box, ["full", "fade_to_black", "off"], RamaControls.romance_intensity, func(t):
		RamaControls.romance_intensity = t; RamaControls.save_cfg())

	status = Label.new()
	status.text = ""
	status.add_theme_font_size_override("font_size", 15)
	status.add_theme_color_override("font_color", Color(1.0, 0.85, 0.5))
	box.add_child(status)

	var bar := HBoxContainer.new()
	bar.add_theme_constant_override("separation", 14)
	var resume := Button.new()
	resume.text = "Resume"
	resume.custom_minimum_size = Vector2(0, 40)
	resume.add_theme_font_size_override("font_size", 16)
	resume.pressed.connect(close)
	bar.add_child(resume)
	var reset := Button.new()
	reset.text = "Reset to defaults"
	reset.custom_minimum_size = Vector2(0, 40)
	reset.add_theme_font_size_override("font_size", 16)
	reset.pressed.connect(func():
		RamaControls.reset()
		_refresh_bindings()
		status.text = "Defaults restored."
		_audio_vols())
	bar.add_child(reset)
	var quit := Button.new()
	quit.text = "Quit"
	quit.custom_minimum_size = Vector2(0, 40)
	quit.add_theme_font_size_override("font_size", 16)
	quit.pressed.connect(func():
		if world and world.has_method("save_world"):
			world.save_world()
		get_tree().quit())
	bar.add_child(quit)
	box.add_child(bar)

## The resolved value of one build axis — the hand-set override if there is one,
## otherwise whatever the archetype (and blend) currently make it.
func _axis(name: String) -> float:
	var axes: Dictionary = RamaControls.avatar.get("axes", {})
	if axes.has(name):
		return float(axes[name])
	var spec: Dictionary = RamaBody.make(str(RamaControls.avatar.get("archetype", "daddy")))
	var to: String = str(RamaControls.avatar.get("blend_to", ""))
	var mix: float = float(RamaControls.avatar.get("blend", 0.0))
	if to != "" and mix > 0.001 and RamaBody.ARCHETYPES.has(to):
		spec = RamaBody.blend(spec, RamaBody.make(to), mix)
	return float(spec.get(name, 0.5))

func _set_axis(name: String, v: float) -> void:
	var axes: Dictionary = RamaControls.avatar.get("axes", {})
	axes[name] = v
	RamaControls.avatar["axes"] = axes
	_apply_build()

func _apply_build() -> void:
	RamaControls.save_cfg()
	if world and world.player and world.player.has_method("_rebuild_body"):
		world.player._rebuild_body()

func _audio_vols() -> void:
	if world and world.audio and world.audio.has_method("apply_volumes"):
		world.audio.apply_volumes()

func _refresh_bindings() -> void:
	for id in rows:
		rows[id].text = RamaControls.binding_name(id)

func _option(box: VBoxContainer, items: Array, current: String, cb: Callable) -> void:
	var dens := OptionButton.new()
	dens.custom_minimum_size = Vector2(0, 36)
	dens.add_theme_font_size_override("font_size", 15)
	var sel := 0
	for i in items.size():
		dens.add_item(str(items[i]))
		if str(items[i]) == current:
			sel = i
	dens.select(sel)
	dens.item_selected.connect(func(i): cb.call(dens.get_item_text(i)))
	box.add_child(dens)

func _check(text: String, pressed: bool, cb: Callable) -> CheckBox:
	var c := CheckBox.new()
	c.text = text
	c.button_pressed = pressed
	c.custom_minimum_size = Vector2(0, 32)
	c.add_theme_font_size_override("font_size", 15)
	c.toggled.connect(cb)
	return c

func _head(box: VBoxContainer, t: String) -> void:
	var l := Label.new()
	l.text = "\n" + t
	l.add_theme_font_size_override("font_size", 22)
	l.add_theme_color_override("font_color", Color(0.95, 0.97, 1.0))
	box.add_child(l)

func _note(box: VBoxContainer, t: String) -> void:
	var l := Label.new()
	l.text = t
	l.add_theme_font_size_override("font_size", 14)
	l.add_theme_color_override("font_color", Color(0.78, 0.84, 0.90))
	box.add_child(l)

func _slider(box: VBoxContainer, label: String, lo: float, hi: float,
		val: float, cb: Callable) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 16)
	var l := Label.new()
	l.text = label
	l.custom_minimum_size = Vector2(280, 0)
	l.add_theme_font_size_override("font_size", 16)
	row.add_child(l)
	var s := HSlider.new()
	s.min_value = lo
	s.max_value = hi
	s.step = (hi - lo) / 100.0
	s.value = val
	s.custom_minimum_size = Vector2(360, 28)
	s.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	s.value_changed.connect(cb)
	row.add_child(s)
	box.add_child(row)

func _listen(id: String) -> void:
	listening = id
	status.text = "Press a key for '%s' ...  (Esc cancels)" % id
	rows[id].text = "…"

func _input(e: InputEvent) -> void:
	if not visible or listening == "":
		return
	if e is InputEventKey and e.pressed and not e.echo:
		if e.keycode == KEY_ESCAPE:
			rows[listening].text = RamaControls.binding_name(listening)
			status.text = "Cancelled."
		else:
			RamaControls.rebind(listening, e.physical_keycode)
			for id in rows:
				rows[id].text = RamaControls.binding_name(id)
			status.text = "Bound."
		listening = ""
		get_viewport().set_input_as_handled()

func open_first_run() -> void:
	first_run_mode = true
	status.text = "First run — set accessibility & content, then Resume."
	open()

func open() -> void:
	visible = true
	listening = ""
	if not first_run_mode:
		status.text = ""
	get_tree().paused = true
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE

func close() -> void:
	visible = false
	get_tree().paused = false
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
	if first_run_mode:
		first_run_mode = false
		RamaControls.save_cfg()

func toggle() -> void:
	if visible: close()
	else: open()
