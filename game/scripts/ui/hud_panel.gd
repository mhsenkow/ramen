extends PanelContainer
const RowScript = preload("res://scripts/ui/hud_row.gd")
## Quiet instrument plates with stable widths and a clear title/readout hierarchy.

var id := ""
var _rows := {}
var _box: VBoxContainer
var _title: Label
var _font_mul := 1.0

func setup(panel_id: String, title: String) -> void:
	id = panel_id
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	var sb := StyleBoxFlat.new()
	# This base pair is audited by tools/check_ui_contrast.py. A dense plate
	# keeps labels readable against both bright fields and shadowed rock.
	sb.bg_color = Color(0.04, 0.06, 0.08, 0.92)
	sb.border_color = Color(0.70, 0.80, 0.86, 0.22)
	sb.set_border_width_all(1)
	sb.set_corner_radius_all(5)
	sb.content_margin_left = 16
	sb.content_margin_right = 16
	sb.content_margin_top = 13
	sb.content_margin_bottom = 14
	sb.shadow_color = Color(0.01, 0.02, 0.02, 0.16)
	sb.shadow_size = 8
	add_theme_stylebox_override("panel", sb)
	_box = VBoxContainer.new()
	_box.add_theme_constant_override("separation", 6)
	add_child(_box)
	var heading := HBoxContainer.new()
	heading.add_theme_constant_override("separation", 9)
	_box.add_child(heading)
	var marker := ColorRect.new()
	marker.custom_minimum_size = Vector2(3, 10)
	marker.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	marker.color = Color(0.87, 0.67, 0.39) if id in ["habitat", "you", "place"] else Color(0.61, 0.77, 0.67)
	marker.mouse_filter = Control.MOUSE_FILTER_IGNORE
	heading.add_child(marker)
	_title = Label.new()
	_title.text = title
	_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_title.add_theme_font_size_override("font_size", 11)
	_title.add_theme_color_override("font_color", Color(0.80, 0.88, 0.94))
	heading.add_child(_title)
	var rule := HSeparator.new()
	var line := StyleBoxLine.new()
	line.color = Color(0.70, 0.80, 0.86, 0.14)
	line.thickness = 1
	rule.add_theme_stylebox_override("separator", line)
	rule.add_theme_constant_override("separation", 7)
	_box.add_child(rule)

func add_row(key: String, caption: String, with_bar := false):
	var r = RowScript.new()
	_box.add_child(r)
	r.setup(key, caption, with_bar)
	r.apply_font_scale(_font_mul)
	_rows[key] = r
	return r

func put(key: String, text: String, frac := -1.0) -> void:
	var r = _rows.get(key)
	if r != null:
		r.put(text, frac)

func set_title(t: String) -> void:
	if _title.text != t:
		_title.text = t

func apply_font_scale(font_mul: float) -> void:
	_font_mul = font_mul
	_title.add_theme_font_size_override("font_size", roundi(11.0 * font_mul))
	for k in _rows:
		_rows[k].apply_font_scale(font_mul)
