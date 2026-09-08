extends CanvasLayer
## Field instruments are docked by purpose, leaving the sight line unobstructed.
const PanelScript = preload("res://scripts/ui/hud_panel.gd")
var panels := {}
var _root: Control
var _docks := {}
var _toast: Label
var _toast_bg: PanelContainer
var _font_mul := 1.0
var _layout_dirty := true

func _ready() -> void:
	layer = 2
	_root = Control.new()
	_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_root)
	for dock in ["upper_left", "lower_left", "upper_center", "lower_center", "right"]:
		var box := VBoxContainer.new()
		box.add_theme_constant_override("separation", 12)
		box.mouse_filter = Control.MOUSE_FILTER_IGNORE
		_root.add_child(box)
		box.resized.connect(_request_layout)
		_docks[dock] = box
	_root.resized.connect(_request_layout)
	_toast_bg = PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.05, 0.07, 0.09, 0.82)
	sb.border_color = Color(0.87, 0.67, 0.39, 0.55)
	sb.border_width_left = 3
	sb.set_corner_radius_all(4)
	sb.content_margin_left = 14
	sb.content_margin_right = 14
	sb.content_margin_top = 6
	sb.content_margin_bottom = 6
	_toast_bg.add_theme_stylebox_override("panel", sb)
	_toast_bg.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_toast_bg.visible = false
	_root.add_child(_toast_bg)
	_toast = Label.new()
	_toast.add_theme_font_size_override("font_size", 13)
	_toast.add_theme_color_override("font_color", Color(0.94, 0.93, 0.86))
	_toast.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_toast.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_toast.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	_toast.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	_toast_bg.add_child(_toast)

func panel(id: String, panel_title: String):
	var dock := "upper_left"
	if id == "you": dock = "lower_left"
	elif id == "colony": dock = "upper_center"
	elif id == "ground": dock = "lower_center"
	elif id == "place": dock = "right"
	var p := PanelScript.new()
	_docks[dock].add_child(p)
	p.setup(id, panel_title)
	p.apply_font_scale(_font_mul)
	panels[id] = p
	_request_layout()
	return p

func put(panel_id: String, key: String, text: String, frac := -1.0) -> void:
	var p = panels.get(panel_id)
	if p != null: p.put(key, text, frac)

func title(panel_id: String, t: String) -> void:
	var p = panels.get(panel_id)
	if p != null: p.set_title(t)

func show_panel(panel_id: String, on: bool) -> void:
	var p = panels.get(panel_id)
	if p != null and p.visible != on:
		p.visible = on
		_request_layout()

func toast(text: String, alpha: float) -> void:
	_toast_bg.visible = alpha > 0.01
	if _toast.text != text:
		_toast.text = text
		_request_layout()
	_toast_bg.modulate.a = clampf(alpha, 0.0, 1.0)

func apply_font_scale(font_mul: float) -> void:
	_font_mul = clampf(font_mul, 0.85, 1.4)
	_toast.add_theme_font_size_override("font_size", roundi(13.0 * _font_mul))
	for id in panels: panels[id].apply_font_scale(_font_mul)
	_request_layout()

func set_density(mode: String) -> void:
	_root.visible = mode != "off"
	if mode == "off" or mode == "full": return
	var allow: Array = ["you", "habitat"] if mode == "minimal" else ["you", "habitat", "ground", "colony"]
	for id in panels:
		if panels[id].visible and id not in allow:
			panels[id].visible = false
			_request_layout()

func show_caption(text: String) -> void:
	toast(text, 1.0)

func _request_layout() -> void:
	_layout_dirty = true

func _process(_delta: float) -> void:
	if _layout_dirty:
		_layout_dirty = false
		_layout()

func _layout() -> void:
	var screen := get_viewport().get_visible_rect().size
	var margin := 24.0 if screen.x >= 1440.0 else 18.0
	var gap := 18.0
	# Values wrap inside their lane, including the 140% text setting on Deck.
	var width: float = minf(356.0 + (_font_mul - 1.0) * 90.0, (screen.x - margin * 2.0 - gap * 2.0) / 3.0)
	for id in _docks:
		var dock: VBoxContainer = _docks[id]
		dock.custom_minimum_size.x = width
		dock.size.x = width
	var center_x := (screen.x - width) * 0.5
	_docks.upper_left.position = Vector2(margin, margin)
	_docks.lower_left.position = Vector2(margin, screen.y - margin - _docks.lower_left.size.y)
	_docks.upper_center.position = Vector2(center_x, margin)
	_docks.lower_center.position = Vector2(center_x, screen.y - margin - _docks.lower_center.size.y)
	_docks.right.position = Vector2(screen.x - margin - width, maxf(280.0, screen.y * 0.32))
	# Compact banner above the bottom instruments — never a mid-screen slab.
	var toast_max_w := minf(320.0, screen.x - 2.0 * (width + margin + gap))
	_toast.custom_minimum_size.x = 0.0
	_toast_bg.custom_minimum_size = Vector2(0, 0)
	_toast_bg.reset_size()
	if _toast_bg.size.x > toast_max_w:
		_toast.custom_minimum_size.x = toast_max_w - 28.0
		_toast_bg.reset_size()
	_toast_bg.position = Vector2(
			(screen.x - _toast_bg.size.x) * 0.5,
			screen.y - margin - 88.0 - _toast_bg.size.y)
