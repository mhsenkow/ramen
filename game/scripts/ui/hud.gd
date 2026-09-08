extends CanvasLayer
## HUD manager. Owns panels, lays them out, and takes values by key.
##
## The old HUD was one Label whose text was rebuilt from a thirty-line format
## string every frame — impossible to restyle, impossible to reorder, and it
## reallocated the whole thing to change one number.

const RowScript = preload("res://scripts/ui/hud_row.gd")
const PanelScript = preload("res://scripts/ui/hud_panel.gd")

var panels := {}
var _left: VBoxContainer
var _toast: Label
var _toast_bg: PanelContainer

func _ready() -> void:
	layer = 2

	_left = VBoxContainer.new()
	_left.set_anchors_preset(Control.PRESET_TOP_LEFT)
	_left.position = Vector2(16, 14)
	_left.add_theme_constant_override("separation", 6)
	_left.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_left)

	# Transient feedback, centred low so it reads without hunting for it.
	_toast_bg = PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.05, 0.07, 0.09, 0.62)
	sb.set_corner_radius_all(3)
	sb.content_margin_left = 12
	sb.content_margin_right = 12
	sb.content_margin_top = 5
	sb.content_margin_bottom = 6
	_toast_bg.add_theme_stylebox_override("panel", sb)
	_toast_bg.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	_toast_bg.position = Vector2(-110, -140)
	_toast_bg.custom_minimum_size = Vector2(220, 0)
	_toast_bg.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_toast_bg.visible = false
	add_child(_toast_bg)
	_toast = Label.new()
	_toast.add_theme_font_size_override("font_size", 12)
	_toast.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_toast_bg.add_child(_toast)

func panel(id: String, panel_title: String):
	var p := PanelScript.new()
	_left.add_child(p)
	p.setup(id, panel_title)
	panels[id] = p
	return p

func put(panel_id: String, key: String, text: String, frac := -1.0) -> void:
	var p = panels.get(panel_id)
	if p != null:
		p.put(key, text, frac)

func title(panel_id: String, t: String) -> void:
	var p = panels.get(panel_id)
	if p != null:
		p.set_title(t)

func show_panel(panel_id: String, on: bool) -> void:
	var p = panels.get(panel_id)
	if p != null:
		p.visible = on

func toast(text: String, alpha: float) -> void:
	if alpha <= 0.01:
		_toast_bg.visible = false
		return
	_toast_bg.visible = true
	if _toast.text != text:
		_toast.text = text
	_toast_bg.modulate = Color(1, 1, 1, clampf(alpha, 0.0, 1.0))

func apply_font_scale(font_mul: float) -> void:
	var s: float = clampf(font_mul, 0.85, 1.4)
	_toast.add_theme_font_size_override("font_size", int(12.0 * s))
	for id in panels:
		var p = panels[id]
		if p and p.has_method("apply_font_scale"):
			p.apply_font_scale(s)

func set_density(mode: String) -> void:
	## HUD density (§2118). Only further-restricts; view gating runs first.
	_left.visible = mode != "off"
	if mode == "off" or mode == "full":
		return
	var allow: Array = ["you", "habitat"] if mode == "minimal" \
			else ["you", "habitat", "ground", "colony"]
	for id in panels:
		if panels[id].visible and id not in allow:
			panels[id].visible = false

func show_caption(text: String) -> void:
	toast(text, 1.0)
