extends HBoxContainer
## A readout keeps its caption, value and optional gauge together when text scales.

const NAME_W := 82
const BAR_W := 96
var key := ""
var _name_label: Label
var _value_label: Label
var _bar_bg: ColorRect
var _bar_fill: ColorRect
var _last := ""
var _last_frac := -1.0

func setup(row_key: String, caption: String, with_bar: bool) -> void:
	key = row_key
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_theme_constant_override("separation", 14)
	_name_label = Label.new()
	_name_label.text = caption
	_name_label.custom_minimum_size = Vector2(NAME_W, 0)
	_name_label.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
	_name_label.add_theme_font_size_override("font_size", 11)
	_name_label.add_theme_color_override("font_color", Color(0.62, 0.70, 0.76))
	add_child(_name_label)
	var readout := VBoxContainer.new()
	readout.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	readout.add_theme_constant_override("separation", 4)
	add_child(readout)
	_value_label = Label.new()
	_value_label.text = "—"
	_value_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_value_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	_value_label.add_theme_font_size_override("font_size", 13)
	_value_label.add_theme_color_override("font_color", Color(0.94, 0.93, 0.86))
	readout.add_child(_value_label)
	if with_bar:
		_bar_bg = ColorRect.new()
		_bar_bg.custom_minimum_size = Vector2(0, 3)
		_bar_bg.color = Color(0.62, 0.70, 0.76, 0.17)
		_bar_bg.mouse_filter = Control.MOUSE_FILTER_IGNORE
		readout.add_child(_bar_bg)
		_bar_fill = ColorRect.new()
		_bar_fill.color = Color(0.62, 0.78, 0.64)
		_bar_fill.mouse_filter = Control.MOUSE_FILTER_IGNORE
		_bar_fill.size = Vector2(0, 3)
		_bar_bg.add_child(_bar_fill)
		_bar_bg.resized.connect(_resize_gauge)

func put(text: String, frac := -1.0, tint := Color(0, 0, 0, 0)) -> void:
	if text != _last:
		_value_label.text = text
		_last = text
		if key == "carry":
			visible = not text.is_empty()
	if _bar_fill != null and not is_equal_approx(frac, _last_frac):
		_last_frac = frac
		_resize_gauge()
		var f: float = clampf(frac, 0.0, 1.0)
		# Preserve the shared scarcity / capacity meaning: sage through amber.
		_bar_fill.color = Color(0.62, 0.78, 0.64).lerp(
				Color(0.91, 0.65, 0.35), smoothstep(0.55, 0.95, f))
	if tint.a > 0.0:
		_value_label.add_theme_color_override("font_color", tint)

func _resize_gauge() -> void:
	if _bar_fill:
		_bar_fill.size = Vector2(_bar_bg.size.x * clampf(_last_frac, 0.0, 1.0), 3)

func apply_font_scale(font_mul: float) -> void:
	_name_label.custom_minimum_size.x = NAME_W * font_mul
	_name_label.add_theme_font_size_override("font_size", roundi(11.0 * font_mul))
	_value_label.add_theme_font_size_override("font_size", roundi(13.0 * font_mul))
