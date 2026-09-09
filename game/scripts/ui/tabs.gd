extends HBoxContainer
## A tab strip, and the one home for the HUD's audited colours.
##
## Shared by the pause menu and the field book so the two cannot drift apart —
## and so `tools/check_ui_contrast.py` has a single file to audit instead of
## chasing the same literals through several StyleBoxes.
##
## Selection is driven from the outside: `select()` sets it, `selected` reports
## it. The buttons take `FOCUS_NONE` on purpose, because a focused Button
## consumes the arrow keys that the containers use to move between tabs.

signal selected(index: int)

# Audited pairs — keep tools/check_ui_contrast.py in step with these.
const PLATE_BG := Color(0.04, 0.06, 0.08, 0.96)
const PANE_BG := Color(0.04, 0.06, 0.08, 0.97)
const TAB_ON_BG := Color(0.16, 0.20, 0.24, 1.0)
const TAB_OFF_BG := Color(0.07, 0.09, 0.11, 0.90)
const TAB_ON_FG := Color(0.96, 0.97, 0.98)
const TAB_OFF_FG := Color(0.74, 0.81, 0.87)
const HINT_FG := Color(0.72, 0.79, 0.85)
const HEAD_FG := Color(0.95, 0.97, 1.00)
const NOTE_FG := Color(0.78, 0.84, 0.90)
const ACCENT := Color(0.87, 0.67, 0.39)

var active := 0
var _buttons: Array[Button] = []
var _font_mul := 1.0

func _init() -> void:
	add_theme_constant_override("separation", 4)

func count() -> int:
	return _buttons.size()

func add_tab(label: String) -> void:
	var b := Button.new()
	b.text = label
	# A focused Button eats the arrow keys the pane navigates with.
	b.focus_mode = Control.FOCUS_NONE
	b.mouse_filter = Control.MOUSE_FILTER_STOP
	var i := _buttons.size()
	b.pressed.connect(func(): select(i))
	add_child(b)
	_buttons.append(b)
	_restyle()

func select(i: int) -> void:
	if _buttons.is_empty():
		return
	active = clampi(i, 0, _buttons.size() - 1)
	_restyle()
	selected.emit(active)

func next() -> void:
	if not _buttons.is_empty():
		select(posmod(active + 1, _buttons.size()))

func prev() -> void:
	if not _buttons.is_empty():
		select(posmod(active - 1, _buttons.size()))

func set_font_scale(mul: float) -> void:
	_font_mul = mul
	_restyle()

## The active tab carries three redundant cues — a lighter plate, an accent
## underline and brighter text — because colour alone is not a state indicator
## (WCAG 1.4.1), and a player with the palette turned down still has two.
func _restyle() -> void:
	for i in _buttons.size():
		var b: Button = _buttons[i]
		var on: bool = i == active
		var sb := StyleBoxFlat.new()
		sb.bg_color = TAB_ON_BG if on else TAB_OFF_BG
		sb.set_corner_radius_all(4)
		sb.content_margin_left = 15
		sb.content_margin_right = 15
		sb.content_margin_top = 8
		sb.content_margin_bottom = 8
		if on:
			sb.border_width_bottom = 3
			sb.border_color = ACCENT
		for state in ["normal", "hover", "pressed", "focus", "disabled"]:
			b.add_theme_stylebox_override(state, sb)
		b.add_theme_color_override("font_color", TAB_ON_FG if on else TAB_OFF_FG)
		b.add_theme_color_override("font_hover_color", TAB_ON_FG)
		b.add_theme_font_size_override("font_size", roundi(13.0 * _font_mul))
