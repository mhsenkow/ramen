extends PanelContainer
const RowScript = preload("res://scripts/ui/hud_row.gd")
## A titled group of rows. Panels are the unit of layout, visibility and style,
## so adding a readout is one call rather than an edit to a format string.

var id := ""
var _rows := {}
var _box: VBoxContainer
var _title: Label

func setup(panel_id: String, title: String) -> void:
	id = panel_id
	mouse_filter = Control.MOUSE_FILTER_IGNORE

	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.04, 0.06, 0.08, 0.42)
	sb.border_color = Color(0.70, 0.80, 0.86, 0.16)
	sb.set_border_width_all(1)
	sb.set_corner_radius_all(3)
	sb.content_margin_left = 9
	sb.content_margin_right = 9
	sb.content_margin_top = 6
	sb.content_margin_bottom = 7
	add_theme_stylebox_override("panel", sb)

	_box = VBoxContainer.new()
	_box.add_theme_constant_override("separation", 2)
	add_child(_box)

	_title = Label.new()
	_title.text = title
	_title.add_theme_font_size_override("font_size", 10)
	_title.add_theme_color_override("font_color", Color(0.80, 0.88, 0.94, 0.75))
	_box.add_child(_title)

func add_row(key: String, caption: String, with_bar := false):
	var r = RowScript.new()
	_box.add_child(r)
	r.setup(key, caption, with_bar)
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
	_title.add_theme_font_size_override("font_size", int(10.0 * font_mul))
	for k in _rows:
		var r = _rows[k]
		if r and r.has_method("apply_font_scale"):
			r.apply_font_scale(font_mul)
