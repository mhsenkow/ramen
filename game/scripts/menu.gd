extends CanvasLayer
const RamaControls = preload("res://scripts/controls.gd")
## Pause menu: rebindable controls, look settings, and a way out.

var world
var panel: PanelContainer
var rows := {}
var listening := ""
var status: Label

func _ready() -> void:
	layer = 10
	process_mode = Node.PROCESS_MODE_ALWAYS
	visible = false

	var dim := ColorRect.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	dim.color = Color(0.02, 0.03, 0.05, 0.72)
	add_child(dim)

	panel = PanelContainer.new()
	panel.set_anchors_preset(Control.PRESET_CENTER)
	panel.position = Vector2(-300, -290)
	panel.custom_minimum_size = Vector2(600, 580)
	add_child(panel)

	var scroll := ScrollContainer.new()
	panel.add_child(scroll)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(576, 0)
	box.add_theme_constant_override("separation", 6)
	scroll.add_child(box)

	_head(box, "KEPLER DRUM — settings")
	_note(box, "Click a binding, then press a key. Gamepad bindings are fixed.")

	for a in RamaControls.ACTIONS:
		var row := HBoxContainer.new()
		var l := Label.new()
		l.text = a["label"]
		l.custom_minimum_size = Vector2(240, 0)
		row.add_child(l)
		var b := Button.new()
		b.text = RamaControls.binding_name(a["id"])
		b.custom_minimum_size = Vector2(250, 0)
		var id: String = a["id"]
		b.pressed.connect(func(): _listen(id))
		row.add_child(b)
		rows[id] = b
		box.add_child(row)

	_head(box, "Look")
	_slider(box, "Sensitivity", 0.0006, 0.0090, RamaControls.sensitivity,
		func(v): RamaControls.sensitivity = v; RamaControls.save_cfg())
	_slider(box, "Field of view", 50.0, 100.0, RamaControls.fov,
		func(v): RamaControls.fov = v; RamaControls.save_cfg())
	var inv := CheckBox.new()
	inv.text = "Invert vertical look"
	inv.button_pressed = RamaControls.invert_y
	inv.toggled.connect(func(v): RamaControls.invert_y = v; RamaControls.save_cfg())
	box.add_child(inv)

	status = Label.new()
	status.text = ""
	status.add_theme_color_override("font_color", Color(1.0, 0.85, 0.5))
	box.add_child(status)

	var bar := HBoxContainer.new()
	var resume := Button.new()
	resume.text = "Resume"
	resume.pressed.connect(close)
	bar.add_child(resume)
	var reset := Button.new()
	reset.text = "Reset to defaults"
	reset.pressed.connect(func():
		RamaControls.reset()
		for id in rows: rows[id].text = RamaControls.binding_name(id)
		status.text = "Defaults restored.")
	bar.add_child(reset)
	var quit := Button.new()
	quit.text = "Quit"
	quit.pressed.connect(func(): get_tree().quit())
	bar.add_child(quit)
	box.add_child(bar)

func _head(box: VBoxContainer, t: String) -> void:
	var l := Label.new()
	l.text = "\n" + t
	l.add_theme_font_size_override("font_size", 17)
	box.add_child(l)

func _note(box: VBoxContainer, t: String) -> void:
	var l := Label.new()
	l.text = t
	l.add_theme_color_override("font_color", Color(0.72, 0.78, 0.84))
	box.add_child(l)

func _slider(box: VBoxContainer, label: String, lo: float, hi: float,
		val: float, cb: Callable) -> void:
	var row := HBoxContainer.new()
	var l := Label.new()
	l.text = label
	l.custom_minimum_size = Vector2(240, 0)
	row.add_child(l)
	var s := HSlider.new()
	s.min_value = lo
	s.max_value = hi
	s.step = (hi - lo) / 100.0
	s.value = val
	s.custom_minimum_size = Vector2(250, 0)
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

func open() -> void:
	visible = true
	listening = ""
	status.text = ""
	get_tree().paused = true
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE

func close() -> void:
	visible = false
	get_tree().paused = false
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED

func toggle() -> void:
	if visible: close()
	else: open()
