extends CanvasLayer
## Field instruments docked by purpose, plus a tabbed field book for reading them.
##
## The docked plates are for glancing at while you walk: small, edge-anchored,
## out of the sight line. They are a bad way to actually *read* nine panels of
## soil chemistry, and at the 140% text setting they were clipping.
##
## So `I` opens the field book — one large centred pane with a tab strip, the
## classic tabbed-dialog shape. The same panel objects move into it, so nothing
## upstream changes: `world.gd` still calls `ui.panel()` / `ui.put()` and never
## learns whether a readout is currently docked or on a page.
##
## Keyboard, and only keys the game does not already bind:
##
##   I            open / close        (rebindable — it is a normal action)
##   left/right   previous / next tab
##   home/end     first / last tab
##   up/down      scroll the page
##
## Arrow keys are deliberate: `Tab` is "switch view" and the number rows are
## modules and speed, so both would fire underneath the book. `player.gd` polls
## `Input.is_action_just_pressed` rather than consuming events, so a key bound
## in-game cannot be captured here — the only safe keys are unbound ones.

const PanelScript = preload("res://scripts/ui/hud_panel.gd")
const RamaControls = preload("res://scripts/controls.gd")
## Tab strip and the audited palette are shared with the pause menu, so the two
## panes cannot drift apart and the contrast gate has one file to audit.
const TabStrip = preload("res://scripts/ui/tabs.gd")
const CastCompass = preload("res://scripts/ui/cast_compass.gd")
## The book exists to be readable, so its type is a size up from the plates.
const BOOK_FONT_BOOST := 1.45

var panels := {}
var _root: Control
var _docks := {}
var _home := {}
var _toast: Label
var _toast_bg: PanelContainer
var _compass: CastCompass
var _font_mul := 1.0
var _layout_dirty := true

# --- field book ---
var _book: Control
var _dim: ColorRect
var _frame: PanelContainer
var _scroll: ScrollContainer
var _page: VBoxContainer
var _hint: Label
var _tabs: TabStrip
var _order: Array[String] = []
var _active := 0
var _open := false
## Visibility each panel had before the book took it, so density and
## `show_panel` decisions survive a trip through the pages.
var _pre_visible := {}
var _mouse_before := Input.MOUSE_MODE_CAPTURED
## Panels the docked layout has no room for. Distinct from a density or
## `show_panel` decision: these are still yours to read, just in the book.
var _crowded_out := {}
## Whether each panel is wanted on screen at all, as decided by `show_panel`
## and `set_density`. Distinct from whether it currently fits: `_fit_dock` may
## only ever hide a wanted panel, never reveal an unwanted one. Without this it
## defaulted everything to wanted and resurrected panels the view had hidden —
## which is why LOCAL PLAN appeared, empty, in first person.
var _wanted := {}

# --- pack page ---
## Set by `world.gd` so the pack page can read the live inventory. Everything
## guards on null, so the HUD still works standalone (the render harnesses and
## the load gate instantiate it with no world).
var sim
var _pack_page: VBoxContainer
var _pack_rows: VBoxContainer
var _pack_head: Label
var _pack_due := 0.0

# --- craft page ---
## The whole recipe chain was reachable only by cycling a blind index with
## `[`/`]` and reading the result from stdout. It is a system the player could
## not see. This page lists what each recipe needs, what you have, and what it
## makes.
var _craft_page: VBoxContainer
var _craft_rows: VBoxContainer
var _craft_head: Label
var _craft_sel := 0
var _craft_due := 0.0
var _craft_count := 0

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
	_toast.size_flags_horizontal = Control.SIZE_FILL
	_toast.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	_toast_bg.add_child(_toast)

	_compass = CastCompass.new()
	_compass.name = "CastCompass"
	_root.add_child(_compass)

	_build_book()

# ------------------------------------------------------------ public API --

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
	_wanted[id] = true
	_home[id] = dock
	_order.append(id)
	_add_tab(id)
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
	if p == null:
		return
	_wanted[panel_id] = on
	if _open:
		# The book decides what is on screen while it is up; remember the
		# intent so closing it does not resurrect a hidden panel.
		_pre_visible[panel_id] = on
		return
	if p.visible != on:
		p.visible = on
		_request_layout()

func toast(text: String, alpha: float) -> void:
	_toast_bg.visible = alpha > 0.01
	if _toast.text != text:
		_toast.text = text
		_request_layout()
	_toast_bg.modulate.a = clampf(alpha, 0.0, 1.0)

## Horizon-style strip: cast (and optional home/mark) by relative bearing.
func set_cast_compass(marks: Array) -> void:
	if _compass == null:
		return
	_compass.set_marks(marks)

func show_cast_compass(on: bool) -> void:
	if _compass == null:
		return
	_compass.visible = on

func apply_font_scale(font_mul: float) -> void:
	_font_mul = clampf(font_mul, 0.85, 1.4)
	_toast.add_theme_font_size_override("font_size", roundi(13.0 * _font_mul))
	if _compass != null:
		_compass.set_font_scale(_font_mul)
	var mul: float = _font_mul * (BOOK_FONT_BOOST if _open else 1.0)
	for id in panels: panels[id].apply_font_scale(mul)
	if _hint != null:
		_hint.add_theme_font_size_override("font_size", roundi(11.0 * _font_mul))
	if _tabs != null:
		_tabs.set_font_scale(_font_mul)
	_request_layout()

func set_density(mode: String) -> void:
	_root.visible = mode != "off"
	if mode == "off":
		# Photo mode and HUD-off mean off, book included.
		close_book()
		return
	if _compass != null:
		_compass.visible = true
	if mode == "full": return
	var allow: Array = ["you", "habitat"] if mode == "minimal" else ["you", "habitat", "ground", "colony"]
	for id in panels:
		if id in allow:
			continue
		_wanted[id] = false
		if _open:
			_pre_visible[id] = false
		elif panels[id].visible:
			panels[id].visible = false
			_request_layout()

func show_caption(text: String) -> void:
	toast(text, 1.0)

# ------------------------------------------------------------ field book --

func _build_book() -> void:
	_book = Control.new()
	_book.set_anchors_preset(Control.PRESET_FULL_RECT)
	_book.visible = false
	add_child(_book)

	_dim = ColorRect.new()
	_dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	_dim.color = Color(0.01, 0.02, 0.03, 0.55)
	_dim.mouse_filter = Control.MOUSE_FILTER_STOP
	_book.add_child(_dim)

	_frame = PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = TabStrip.PANE_BG
	sb.border_color = Color(0.70, 0.80, 0.86, 0.30)
	sb.set_border_width_all(1)
	sb.set_corner_radius_all(7)
	sb.content_margin_left = 26
	sb.content_margin_right = 26
	sb.content_margin_top = 20
	sb.content_margin_bottom = 18
	sb.shadow_color = Color(0.0, 0.01, 0.02, 0.35)
	sb.shadow_size = 18
	_frame.add_theme_stylebox_override("panel", sb)
	_book.add_child(_frame)

	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 16)
	_frame.add_child(col)

	var head := HBoxContainer.new()
	head.add_theme_constant_override("separation", 10)
	col.add_child(head)
	var mark := ColorRect.new()
	mark.custom_minimum_size = Vector2(3, 16)
	mark.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	mark.color = TabStrip.ACCENT
	head.add_child(mark)
	var heading := Label.new()
	heading.text = "FIELD BOOK"
	heading.add_theme_font_size_override("font_size", 13)
	heading.add_theme_color_override("font_color", TabStrip.TAB_ON_FG)
	heading.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(heading)

	# The tab strip scrolls rather than clipping: nine tabs at 140% text will
	# not fit a narrow window, and a tab you cannot reach is worse than a scroll.
	var strip_scroll := ScrollContainer.new()
	strip_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	strip_scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	strip_scroll.custom_minimum_size = Vector2(0, 40)
	col.add_child(strip_scroll)
	_tabs = TabStrip.new()
	strip_scroll.add_child(_tabs)
	_tabs.selected.connect(_show_page)

	var rule := HSeparator.new()
	var line := StyleBoxLine.new()
	line.color = Color(0.70, 0.80, 0.86, 0.22)
	line.thickness = 1
	rule.add_theme_stylebox_override("separator", line)
	col.add_child(rule)

	_scroll = ScrollContainer.new()
	_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	col.add_child(_scroll)
	_page = VBoxContainer.new()
	_page.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_page.add_theme_constant_override("separation", 10)
	_scroll.add_child(_page)

	# The one page that is not a `hud_panel`: it lists the pack, which is a
	# living list rather than a fixed set of labelled readouts.
	_pack_page = VBoxContainer.new()
	_pack_page.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_pack_page.add_theme_constant_override("separation", 8)
	_pack_page.visible = false
	_page.add_child(_pack_page)
	_pack_head = Label.new()
	_pack_head.add_theme_font_size_override("font_size", 15)
	_pack_head.add_theme_color_override("font_color", TabStrip.NOTE_FG)
	_pack_page.add_child(_pack_head)
	_pack_rows = VBoxContainer.new()
	_pack_rows.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_pack_rows.add_theme_constant_override("separation", 3)
	_pack_page.add_child(_pack_rows)

	_craft_page = VBoxContainer.new()
	_craft_page.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_craft_page.add_theme_constant_override("separation", 8)
	_craft_page.visible = false
	_page.add_child(_craft_page)
	_craft_head = Label.new()
	_craft_head.add_theme_font_size_override("font_size", 15)
	_craft_head.add_theme_color_override("font_color", TabStrip.NOTE_FG)
	_craft_page.add_child(_craft_head)
	_craft_rows = VBoxContainer.new()
	_craft_rows.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_craft_rows.add_theme_constant_override("separation", 2)
	_craft_page.add_child(_craft_rows)

	_hint = Label.new()
	_hint.text = "←/→ tab   ·   ↑/↓ select   ·   enter craft   ·   I close"
	_hint.add_theme_font_size_override("font_size", 11)
	_hint.add_theme_color_override("font_color", TabStrip.HINT_FG)
	col.add_child(_hint)

func _add_tab(id: String) -> void:
	if _tabs != null:
		_tabs.add_tab(id.to_upper())

## Show one page. Tab order is the order `world.gd` registered panels in.
func _show_page(i: int) -> void:
	var pack_at: int = _order.size()
	var craft_at: int = pack_at + 1
	_active = clampi(i, 0, craft_at)
	var on_pack: bool = _active == pack_at
	var on_craft: bool = _active == craft_at
	for id in _order:
		var p = panels.get(id)
		if p != null:
			p.visible = not on_pack and not on_craft and id == _order[_active]
	if _pack_page != null:
		_pack_page.visible = on_pack
		if on_pack:
			_refresh_pack()
	if _craft_page != null:
		_craft_page.visible = on_craft
		if on_craft:
			_refresh_craft()
	_scroll.scroll_vertical = 0

func _select(i: int) -> void:
	if _tabs == null:
		return
	# Clamp to the TAB count, not the panel count — the pack page is a tab
	# without a panel behind it, so clamping to `_order` excluded it entirely.
	_tabs.select(clampi(i, 0, maxi(_tabs.count() - 1, 0)))

func toggle_book() -> void:
	if _open: close_book()
	else: open_book()

## Appended after the panel tabs, so it is always last however many panels
## `world.gd` registers.
func _ensure_pack_tab() -> void:
	if _tabs == null or _pack_page == null:
		return
	if _tabs.count() == _order.size():
		_tabs.add_tab("PACK")
		_tabs.add_tab("CRAFT")

## One row per stack: what it is, how much, and how coarse. Rebuilt rather than
## diffed — a pack has a handful of stacks, and the page is only up while you
## are reading it.
func _refresh_pack() -> void:
	if _pack_rows == null:
		return
	for c in _pack_rows.get_children():
		c.queue_free()
	if sim == null or not sim.has_method("inventory"):
		_pack_head.text = "no pack"
		return
	var inv: Dictionary = sim.inventory()
	var kg: float = float(inv.get("mass_kg", 0.0))
	var maxkg: float = maxf(float(inv.get("max_mass_kg", 90.0)), 1.0)
	var vf: float = float(inv.get("volume_frac", 0.0))
	_pack_head.text = "%.1f / %.0f kg  ·  %d%% volume  ·  %d%% speed" % [
			kg, maxkg, int(vf * 100.0),
			int(float(inv.get("encumbrance", 1.0)) * 100.0)]
	var stacks: Array = inv.get("stacks", [])
	if stacks.is_empty():
		var none := Label.new()
		none.text = "empty — dig rock, mine a trunk, or fell a tree"
		none.add_theme_font_size_override("font_size", 14)
		none.add_theme_color_override("font_color", TabStrip.HINT_FG)
		_pack_rows.add_child(none)
		return
	# Heaviest first: what you are actually carrying reads top-down.
	var sorted: Array = stacks.duplicate()
	sorted.sort_custom(func(a, b): return float(a.get("mass_kg", 0.0)) > float(b.get("mass_kg", 0.0)))
	for st in sorted:
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 14)
		row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		var name_l := Label.new()
		name_l.text = str(st.get("name", "?"))
		name_l.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		name_l.add_theme_font_size_override("font_size", roundi(15.0 * _font_mul))
		name_l.add_theme_color_override("font_color", Color(0.94, 0.93, 0.86))
		row.add_child(name_l)
		var mass_l := Label.new()
		mass_l.text = "%.1f kg" % float(st.get("mass_kg", 0.0))
		mass_l.custom_minimum_size.x = 90.0 * _font_mul
		mass_l.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		mass_l.add_theme_font_size_override("font_size", roundi(15.0 * _font_mul))
		mass_l.add_theme_color_override("font_color", Color(0.94, 0.93, 0.86))
		row.add_child(mass_l)
		# Grade is why two stacks of the same material are not the same stack.
		var g := Label.new()
		g.text = "grade %.2f" % float(st.get("grade", 0.0))
		g.custom_minimum_size.x = 110.0 * _font_mul
		g.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		g.add_theme_font_size_override("font_size", roundi(13.0 * _font_mul))
		g.add_theme_color_override("font_color", TabStrip.HINT_FG)
		row.add_child(g)
		_pack_rows.add_child(row)

## One row per recipe: whether you can make it, what it needs against what you
## carry, and what comes out. Rebuilt on a refresh rather than diffed — there
## are two dozen recipes and the page is only up while you read it.
func _on_craft() -> bool:
	return _craft_page != null and _craft_page.visible

func _refresh_craft() -> void:
	if _craft_rows == null:
		return
	for c in _craft_rows.get_children():
		c.queue_free()
	if sim == null or not sim.has_method("recipe_count"):
		_craft_head.text = "no workshop"
		_craft_count = 0
		return
	_craft_count = int(sim.recipe_count())
	if _craft_count < 1:
		_craft_head.text = "no recipes"
		return
	_craft_sel = clampi(_craft_sel, 0, _craft_count - 1)
	var ready_n := 0
	for i in _craft_count:
		var r: Dictionary = sim.recipe_at(i)
		if not r.get("ok", false):
			continue
		var max_sc: float = float(r.get("max_scale", 0.0))
		var near: bool = bool(r.get("station_near", true))
		if max_sc >= 0.05 and near:
			ready_n += 1
		_craft_rows.add_child(_craft_row(i, r, max_sc, near))
	_craft_head.text = "%d of %d ready  ·  enter makes the highlighted one" % [
			ready_n, _craft_count]

func _craft_row(i: int, r: Dictionary, max_sc: float, near: bool) -> Control:
	var box := VBoxContainer.new()
	box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_theme_constant_override("separation", 1)
	var picked: bool = i == _craft_sel
	if picked:
		var plate := StyleBoxFlat.new()
		plate.bg_color = TabStrip.TAB_ON_BG
		plate.set_corner_radius_all(4)
		plate.content_margin_left = 10
		plate.content_margin_right = 10
		plate.content_margin_top = 6
		plate.content_margin_bottom = 6
		plate.border_width_left = 3
		plate.border_color = TabStrip.ACCENT
		var pc := PanelContainer.new()
		pc.add_theme_stylebox_override("panel", plate)
		pc.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		pc.add_child(box)
		_fill_craft_row(box, r, max_sc, near, picked)
		return pc
	var pad := MarginContainer.new()
	pad.add_theme_constant_override("margin_left", 13)
	pad.add_theme_constant_override("margin_top", 4)
	pad.add_theme_constant_override("margin_bottom", 4)
	pad.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	pad.add_child(box)
	_fill_craft_row(box, r, max_sc, near, picked)
	return pad

func _fill_craft_row(box: VBoxContainer, r: Dictionary, max_sc: float,
		near: bool, picked: bool) -> void:
	var head := HBoxContainer.new()
	head.add_theme_constant_override("separation", 12)
	head.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_child(head)

	var name_l := Label.new()
	# Recipe ids are snake_case keys; the player should read words.
	name_l.text = str(r.get("id", "?")).replace("_", " ")
	name_l.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_l.add_theme_font_size_override("font_size", roundi(15.0 * _font_mul))
	name_l.add_theme_color_override("font_color",
			Color(0.94, 0.93, 0.86) if picked else TabStrip.TAB_OFF_FG)
	head.add_child(name_l)

	var out_l := Label.new()
	var outs := PackedStringArray()
	for o in r.get("outputs", []):
		outs.append("%.1f kg %s" % [float(o.get("mass_kg", 0.0)), str(o.get("material", "?"))])
	out_l.text = "→ " + (", ".join(outs) if outs.size() > 0 else "—")
	out_l.add_theme_font_size_override("font_size", roundi(13.0 * _font_mul))
	out_l.add_theme_color_override("font_color", Color(0.62, 0.78, 0.64))
	head.add_child(out_l)

	var state := Label.new()
	# Say why, not just no. "0 of 24 ready" with no reason is not a UI.
	if not near:
		state.text = "needs a %s" % str(r.get("station", "station"))
		state.add_theme_color_override("font_color", Color(0.91, 0.65, 0.35))
	elif max_sc < 0.05:
		state.text = "short"
		state.add_theme_color_override("font_color", Color(0.85, 0.55, 0.50))
	else:
		state.text = "ready x%.1f" % max_sc
		state.add_theme_color_override("font_color", Color(0.62, 0.82, 0.66))
	state.custom_minimum_size.x = 132.0 * _font_mul
	state.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	state.add_theme_font_size_override("font_size", roundi(13.0 * _font_mul))
	head.add_child(state)

	# Inputs only for the highlighted recipe: twenty-four expanded rows is a
	# wall of numbers, and you only need the shortfall for the one you want.
	if not picked:
		return
	for inp in r.get("inputs", []):
		var need: float = float(inp.get("mass_kg", 0.0))
		var have: float = float(inp.get("have_kg", 0.0))
		var line := HBoxContainer.new()
		line.add_theme_constant_override("separation", 12)
		line.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		var what := Label.new()
		what.text = "    " + str(inp.get("material", "?"))
		what.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		what.add_theme_font_size_override("font_size", roundi(13.0 * _font_mul))
		what.add_theme_color_override("font_color", TabStrip.HINT_FG)
		line.add_child(what)
		var amt := Label.new()
		var ok: bool = have + 1e-4 >= need
		amt.text = "%.1f / %.1f kg" % [have, need]
		if not ok:
			amt.text += "   short %.1f" % (need - have)
		amt.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		amt.custom_minimum_size.x = 210.0 * _font_mul
		amt.add_theme_font_size_override("font_size", roundi(13.0 * _font_mul))
		amt.add_theme_color_override("font_color",
				TabStrip.HINT_FG if ok else Color(0.91, 0.60, 0.52))
		line.add_child(amt)
		box.add_child(line)

## Make the highlighted recipe at the largest scale the pack allows.
func _craft_selected() -> void:
	if sim == null or not sim.has_method("craft_here") or _craft_count < 1:
		return
	var r: Dictionary = sim.recipe_at(_craft_sel)
	if not r.get("ok", false):
		return
	if not bool(r.get("station_near", true)):
		toast("needs a %s nearby" % str(r.get("station", "station")), 1.0)
		return
	var max_sc: float = float(r.get("max_scale", 0.0))
	if max_sc < 0.05:
		toast("not enough for %s" % str(r.get("id", "that")).replace("_", " "), 1.0)
		return
	var res: Dictionary = sim.craft_here(_craft_sel, max_sc)
	if res.get("ok", false):
		toast("made %s x%.1f" % [str(r.get("id", "?")).replace("_", " "), max_sc], 1.0)
	else:
		toast("could not make it: %s" % str(res.get("error", "unknown")), 1.0)
	_refresh_craft()

func open_book() -> void:
	if _open or _order.is_empty() or not _root.visible:
		return
	if RamaControls.photo_mode or get_tree().paused:
		return
	_open = true
	_pre_visible.clear()
	for id in _order:
		var p = panels.get(id)
		if p == null:
			continue
		_pre_visible[id] = p.visible
		var dock: VBoxContainer = _docks[_home[id]]
		if p.get_parent() == dock:
			dock.remove_child(p)
		p.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		_page.add_child(p)
		p.apply_font_scale(_font_mul * BOOK_FONT_BOOST)
	_ensure_pack_tab()
	_book.visible = true
	_mouse_before = Input.mouse_mode
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	_select(_active)
	_request_layout()

func close_book() -> void:
	if not _open:
		return
	_open = false
	for id in _order:
		var p = panels.get(id)
		if p == null:
			continue
		if p.get_parent() == _page:
			_page.remove_child(p)
		p.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
		_docks[_home[id]].add_child(p)
		p.apply_font_scale(_font_mul)
		p.visible = bool(_pre_visible.get(id, true))
	_book.visible = false
	Input.mouse_mode = _mouse_before
	_request_layout()

func _input(event: InputEvent) -> void:
	if event.is_action_pressed(RamaControls.act("info")):
		toggle_book()
		get_viewport().set_input_as_handled()
		return
	if not _open or not (event is InputEventKey) or not event.pressed:
		return
	var k: InputEventKey = event
	match k.keycode:
		KEY_LEFT:
			_select(posmod(_active - 1, maxi(_tabs.count(), 1)))
		KEY_RIGHT:
			_select(posmod(_active + 1, maxi(_tabs.count(), 1)))
		KEY_HOME:
			_select(0)
		KEY_END:
			_select(_tabs.count() - 1)
		KEY_UP:
			# On the craft page the arrows move the selection; everywhere else
			# they scroll, which is what there is to do on a readout.
			if _on_craft() and _craft_count > 0:
				_craft_sel = posmod(_craft_sel - 1, _craft_count)
				_refresh_craft()
			else:
				_scroll.scroll_vertical -= 48
		KEY_DOWN:
			if _on_craft() and _craft_count > 0:
				_craft_sel = posmod(_craft_sel + 1, _craft_count)
				_refresh_craft()
			else:
				_scroll.scroll_vertical += 48
		KEY_ENTER, KEY_KP_ENTER:
			if _on_craft():
				_craft_selected()
			else:
				return
		KEY_PAGEUP:
			_scroll.scroll_vertical -= int(_scroll.size.y * 0.85)
		KEY_PAGEDOWN:
			_scroll.scroll_vertical += int(_scroll.size.y * 0.85)
		_:
			return
	get_viewport().set_input_as_handled()

func _process(_delta: float) -> void:
	# The pause menu owns the screen when it is up, and photo mode wants it bare.
	if _open and (get_tree().paused or RamaControls.photo_mode):
		close_book()
	elif _open and Input.mouse_mode != Input.MOUSE_MODE_VISIBLE:
		# Something else recaptured the pointer while the book was up — the
		# pause menu's `close()` does exactly that — and the tabs stop being
		# clickable. Cheaper to re-assert it than to coordinate.
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	if _open and _pack_page != null and _pack_page.visible:
		_pack_due -= _delta
		if _pack_due <= 0.0:
			_pack_due = 0.4
			_refresh_pack()
	if _open and _on_craft():
		_craft_due -= _delta
		if _craft_due <= 0.0:
			_craft_due = 0.5
			_refresh_craft()
	if _layout_dirty:
		_layout_dirty = false
		_layout()

# ---------------------------------------------------------------- layout --

func _request_layout() -> void:
	_layout_dirty = true

## Hide trailing panels in a dock until the stack fits `room` pixels.
##
## Only ever hides from the end, so the order `world.gd` declared panels in is
## the priority order — the first ones registered are the ones kept.
func _fit_dock(dock_id: String, room: float) -> void:
	var dock: VBoxContainer = _docks[dock_id]
	var sep: float = dock.get_theme_constant("separation")
	var used := 0.0
	var changed := false
	for child in dock.get_children():
		var p := child as Control
		if p == null:
			continue
		var pid: String = p.id if "id" in p else ""
		# A panel the view does not want stays hidden, and does not consume
		# room that a wanted panel could use.
		if not bool(_wanted.get(pid, true)):
			if p.visible:
				p.visible = false
				changed = true
			continue
		var h: float = p.get_combined_minimum_size().y
		var fits: bool = used + h <= room
		if fits:
			used += h + sep
		if p.visible != fits:
			p.visible = fits
			changed = true
		if fits:
			_crowded_out.erase(pid)
		else:
			_crowded_out[pid] = true
	if changed:
		_request_layout()

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
	# Bottom docks hang from the bottom edge, so they need their own height.
	# `size.y` is whatever the last layout pass left there — zero on the first
	# one — which parked the bottom-centre dock at the TOP of the screen, where
	# it drew straight through the upper-centre dock. `get_combined_minimum_size`
	# is content-derived and correct immediately.
	var low_left: float = _docks.lower_left.get_combined_minimum_size().y
	var low_mid: float = _docks.lower_center.get_combined_minimum_size().y
	# Eight panels do not fit down one edge of a 900 px screen, and at 140% text
	# they do not fit down two. Rather than let the top stack draw straight
	# through the bottom one, drop whatever will not fit — it is still on its
	# page in the book, which is the whole reason the book exists.
	if not _open:
		_fit_dock("upper_left", screen.y - margin * 2.0 - low_left - gap)
		_fit_dock("upper_center", screen.y - margin * 2.0 - low_mid - gap)
		low_left = _docks.lower_left.get_combined_minimum_size().y
		low_mid = _docks.lower_center.get_combined_minimum_size().y
	_docks.upper_left.position = Vector2(margin, margin)
	_docks.lower_left.position = Vector2(margin, screen.y - margin - low_left)
	_docks.upper_center.position = Vector2(center_x, margin)
	_docks.lower_center.position = Vector2(center_x, screen.y - margin - low_mid)
	_docks.right.position = Vector2(screen.x - margin - width, maxf(280.0, screen.y * 0.32))

	# Cast compass — Horizon strip, centred under the top edge, above docks.
	if _compass != null and _compass.visible:
		var cw: float = clampf(screen.x * 0.42, 280.0, 560.0) * _font_mul
		var ch: float = maxf(46.0 * _font_mul, _compass.custom_minimum_size.y)
		_compass.size = Vector2(cw, ch)
		_compass.position = Vector2((screen.x - cw) * 0.5, 8.0)
		# Keep the upper-centre colony plate from sitting under the strip.
		_docks.upper_center.position.y = margin + ch + 6.0
		_docks.upper_left.position.y = margin + ch * 0.35
		_docks.right.position.y = maxf(280.0, screen.y * 0.32) + ch * 0.2

	# Compact banner above the bottom instruments — never a mid-screen slab.
	#
	# Given one fixed width rather than measured twice. An autowrapping Label
	# has a minimum width of a single character, so a PanelContainer told to
	# shrink to its content collapsed onto that minimum and wrapped the message
	# one letter per line — a vertical column of glyphs down the middle of the
	# screen. Deciding the width first and letting the text wrap inside it is
	# both correct and one pass instead of two.
	var toast_w: float = clampf(screen.x * 0.30, 260.0, 460.0) * _font_mul
	toast_w = minf(toast_w, screen.x - margin * 2.0)
	_toast.custom_minimum_size.x = toast_w - 28.0
	_toast_bg.custom_minimum_size.x = toast_w
	_toast_bg.size.x = toast_w
	_toast_bg.reset_size()
	_toast_bg.position = Vector2(
			(screen.x - toast_w) * 0.5,
			screen.y - margin - low_mid - gap - _toast_bg.size.y)

	if _frame != null:
		# Most of the view, which is the point of it, but never edge to edge.
		var bw: float = minf(screen.x * 0.84, 1120.0 * _font_mul)
		var bh: float = minf(screen.y * 0.82, 820.0 * _font_mul)
		bw = minf(bw, screen.x - margin * 2.0)
		bh = minf(bh, screen.y - margin * 2.0)
		_frame.size = Vector2(bw, bh)
		_frame.position = Vector2((screen.x - bw) * 0.5, (screen.y - bh) * 0.5)
