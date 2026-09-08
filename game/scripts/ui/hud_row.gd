extends HBoxContainer
## One labelled value, optionally with a fill bar.
##
## The HUD was a single thirty-line format string rebuilt every frame. A row
## owns its own state and only touches the scene tree when its value actually
## changes, which is both cheaper and far easier to restyle.

var key := ""
var _name_label: Label
var _value_label: Label
var _bar_bg: ColorRect
var _bar_fill: ColorRect
var _last := ""
var _last_frac := -1.0

const NAME_W := 96
const BAR_W := 74

func setup(row_key: String, caption: String, with_bar: bool) -> void:
	key = row_key
	add_theme_constant_override("separation", 6)

	_name_label = Label.new()
	_name_label.text = caption
	_name_label.custom_minimum_size = Vector2(NAME_W, 0)
	_name_label.add_theme_font_size_override("font_size", 11)
	_name_label.add_theme_color_override("font_color", Color(0.62, 0.70, 0.76))
	add_child(_name_label)

	if with_bar:
		var holder := Control.new()
		holder.custom_minimum_size = Vector2(BAR_W, 10)
		_bar_bg = ColorRect.new()
		_bar_bg.color = Color(1, 1, 1, 0.10)
		_bar_bg.position = Vector2(0, 3)
		_bar_bg.size = Vector2(BAR_W, 5)
		holder.add_child(_bar_bg)
		_bar_fill = ColorRect.new()
		_bar_fill.color = Color(0.62, 0.82, 0.66, 0.85)
		_bar_fill.position = Vector2(0, 3)
		_bar_fill.size = Vector2(0, 5)
		holder.add_child(_bar_fill)
		add_child(holder)

	_value_label = Label.new()
	_value_label.add_theme_font_size_override("font_size", 11)
	_value_label.add_theme_color_override("font_color", Color(0.92, 0.95, 0.97))
	add_child(_value_label)

## Only writes when something changed — a HUD that rebuilds every frame is the
## most common source of avoidable per-frame allocation in a Godot project.
func put(text: String, frac := -1.0, tint := Color(0, 0, 0, 0)) -> void:
	if text != _last:
		_value_label.text = text
		_last = text
	if _bar_fill != null and not is_equal_approx(frac, _last_frac):
		_last_frac = frac
		var f: float = clampf(frac, 0.0, 1.0)
		_bar_fill.size = Vector2(BAR_W * f, 5)
		# Green through amber to red as a gauge fills. Encumbrance, power and
		# satiety all read the same way, so the colour means one thing.
		_bar_fill.color = Color(0.55, 0.82, 0.60, 0.85).lerp(
				Color(0.92, 0.62, 0.34, 0.9), smoothstep(0.55, 0.95, f))
	if tint.a > 0.0:
		_value_label.add_theme_color_override("font_color", tint)

func apply_font_scale(font_mul: float) -> void:
	var s: int = int(11.0 * font_mul)
	_name_label.add_theme_font_size_override("font_size", s)
	_value_label.add_theme_font_size_override("font_size", s)
