extends CanvasLayer
const RamaBody = preload("res://scripts/avatar/body.gd")
const RamaControls = preload("res://scripts/controls.gd")
const TabStrip = preload("res://scripts/ui/tabs.gd")
## Pause menu: rebindable controls, look/a11y/audio/content settings.
##
## Seven sections on one tabbed pane rather than one scroll a metre long. It was
## a fixed 800 x 720 panel, and the project sets no viewport stretch, so on any
## display bigger than the 1600 x 900 design size it shrank to a stamp in the
## middle of the screen. Sized from the viewport now, and laid out again on
## resize.
##
## Keyboard: left/right change tab, home/end jump to the ends, Esc resumes.
## Arrow keys are ignored while a binding is listening, or you could not bind
## an arrow key to anything.

var world
var panel: PanelContainer
var rows := {}
var listening := ""
var status: Label
var first_run_mode := false
var _tabs: TabStrip
## When the menu last closed itself. `player.gd::_actions` polls
## `is_action_just_pressed("pause")` and calls `toggle()`, and a press stays
## "just pressed" for the whole frame — so closing here unpaused the tree and
## the player then re-opened the menu with the same keystroke. Input being
## marked handled does not affect polling, so the guard has to live here.
var _closed_at := -1.0
const REOPEN_LOCKOUT_MS := 250.0
var _pages: Array[Control] = []
var _stack: Control
var _hint: Label
var build_stats: Label
var _coop_status: Label
var _coop_invite: Label
var _coop_hint: Label
var _coop_step: Label
var _coop_join_edit: LineEdit
var _coop_perm_checks: Dictionary = {}
var _coop_tab_index := -1
var _coop_distance: Label

func _ready() -> void:
	layer = 10
	process_mode = Node.PROCESS_MODE_ALWAYS
	visible = false

	var dim := ColorRect.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	dim.color = Color(0.02, 0.03, 0.05, 0.82)
	add_child(dim)
	get_viewport().size_changed.connect(_relayout)

	panel = PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = TabStrip.PANE_BG
	sb.border_color = Color(0.70, 0.80, 0.86, 0.30)
	sb.set_border_width_all(1)
	sb.set_corner_radius_all(7)
	sb.content_margin_left = 30
	sb.content_margin_right = 30
	sb.content_margin_top = 22
	sb.content_margin_bottom = 20
	panel.add_theme_stylebox_override("panel", sb)
	add_child(panel)

	var frame := VBoxContainer.new()
	frame.add_theme_constant_override("separation", 14)
	panel.add_child(frame)

	var titlebar := HBoxContainer.new()
	titlebar.add_theme_constant_override("separation", 10)
	frame.add_child(titlebar)
	var mark := ColorRect.new()
	mark.custom_minimum_size = Vector2(3, 18)
	mark.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	mark.color = TabStrip.ACCENT
	titlebar.add_child(mark)
	var titel := Label.new()
	titel.text = "KEPLER DRUM — settings"
	titel.add_theme_font_size_override("font_size", 20)
	titel.add_theme_color_override("font_color", TabStrip.HEAD_FG)
	titel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	titlebar.add_child(titel)

	# Seven tabs will not fit a narrow window at 140% text, so the strip
	# scrolls rather than clipping a tab out of reach.
	var strip_scroll := ScrollContainer.new()
	strip_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	strip_scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	strip_scroll.custom_minimum_size = Vector2(0, 44)
	frame.add_child(strip_scroll)
	_tabs = TabStrip.new()
	strip_scroll.add_child(_tabs)
	_tabs.selected.connect(_show_page)

	var rule := HSeparator.new()
	var line := StyleBoxLine.new()
	line.color = Color(0.70, 0.80, 0.86, 0.22)
	line.thickness = 1
	rule.add_theme_stylebox_override("separator", line)
	frame.add_child(rule)

	_stack = Control.new()
	_stack.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_stack.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	frame.add_child(_stack)

	var box := _page("CONTROLS")
	_note(box, "Click a binding, then press a key. Gamepad bindings are fixed.")

	# Forty-nine bindings down one column is a long scroll past a lot of empty
	# pane. Two columns halves it and uses the width the pane now has.
	var grid := GridContainer.new()
	grid.columns = 2
	grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	grid.add_theme_constant_override("h_separation", 26)
	grid.add_theme_constant_override("v_separation", 6)
	box.add_child(grid)
	for a in RamaControls.ACTIONS:
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 14)
		row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		var l := Label.new()
		l.text = a["label"]
		l.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		l.add_theme_font_size_override("font_size", 16)
		row.add_child(l)
		var b := Button.new()
		b.text = RamaControls.binding_name(a["id"])
		b.custom_minimum_size = Vector2(190, 34)
		b.add_theme_font_size_override("font_size", 15)
		var id: String = a["id"]
		b.pressed.connect(func(): _listen(id))
		row.add_child(b)
		rows[id] = b
		grid.add_child(row)

	# The character creator. Three layers, in the order they compose: pick a
	# build, lean it toward a second one, then move any individual number. The
	# camera is over his shoulder while this is open, so it is all live.
	box = _page("BUILD")
	build_stats = Label.new()
	build_stats.add_theme_font_size_override("font_size", 15)
	build_stats.add_theme_color_override("font_color", TabStrip.HEAD_FG)
	build_stats.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(build_stats)
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

	box = _page("LOOK")
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

	box = _page("ACCESS")
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

	box = _page("AUDIO")
	_slider(box, "World", 0.0, 1.0, RamaControls.vol_world, func(v):
		RamaControls.vol_world = v; RamaControls.save_cfg(); _audio_vols())
	_slider(box, "UI", 0.0, 1.0, RamaControls.vol_ui, func(v):
		RamaControls.vol_ui = v; RamaControls.save_cfg(); _audio_vols())
	_slider(box, "Music", 0.0, 1.0, RamaControls.vol_music, func(v):
		RamaControls.vol_music = v; RamaControls.save_cfg(); _audio_vols())
	_slider(box, "Voice", 0.0, 1.0, RamaControls.vol_voice, func(v):
		RamaControls.vol_voice = v; RamaControls.save_cfg(); _audio_vols())

	box = _page("GRAPHICS")
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

	box = _page("CONTENT")
	box.add_child(_check("Archive memories enabled", RamaControls.archive_enabled, func(v):
		RamaControls.archive_enabled = v; RamaControls.save_cfg()))
	_note(box, "Romance intensity")
	_option(box, ["full", "fade_to_black", "off"], RamaControls.romance_intensity, func(t):
		RamaControls.romance_intensity = t; RamaControls.save_cfg())

	box = _page("CO-OP")
	_coop_tab_index = _pages.size() - 1
	_note(box, "Play together on one drum. Voice: Discord — the game has no chat.")

	_coop_distance = Label.new()
	_coop_distance.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_coop_distance.add_theme_font_size_override("font_size", 15)
	_coop_distance.add_theme_color_override("font_color", Color(1.0, 0.88, 0.55))
	_coop_distance.text = "Different cities (Seattle ↔ Michigan): both install Tailscale first — free, ~2 minutes."
	box.add_child(_coop_distance)
	var ts_row := HBoxContainer.new()
	ts_row.add_theme_constant_override("separation", 10)
	var ts_btn := Button.new()
	ts_btn.text = "Open Tailscale download"
	ts_btn.custom_minimum_size = Vector2(0, 40)
	ts_btn.add_theme_font_size_override("font_size", 16)
	ts_btn.pressed.connect(func():
		OS.shell_open("https://tailscale.com/download")
		status.text = "Install Tailscale → sign in same way on both PCs → Host again.")
	ts_row.add_child(ts_btn)
	var ts_help := Button.new()
	ts_help.text = "How (30 sec)"
	ts_help.custom_minimum_size = Vector2(0, 40)
	ts_help.pressed.connect(func():
		status.text = "1) Both install Tailscale  2) Sign in (Google/GitHub/Microsoft)  3) Friend accepts your share / you add them  4) Host → Copy invite → they Join")
	ts_row.add_child(ts_help)
	box.add_child(ts_row)
	_note(box, "Then: Host → Copy invite → send in Discord → friend Joins. No port forwarding.")

	# Keep the CO-OP tab as a shortcut into the lobby.
	var open_lobby_btn := Button.new()
	open_lobby_btn.text = "Open co-op lobby (recommended)"
	open_lobby_btn.custom_minimum_size = Vector2(0, 44)
	open_lobby_btn.add_theme_font_size_override("font_size", 17)
	open_lobby_btn.pressed.connect(func():
		close()
		if world and world.coop_lobby:
			world.coop_lobby.open())
	box.add_child(open_lobby_btn)

	_coop_step = Label.new()
	_coop_step.text = "① Host my drum  →  ② Copy invite  →  ③ Friend pastes & Joins"
	_coop_step.add_theme_font_size_override("font_size", 15)
	_coop_step.add_theme_color_override("font_color", Color(0.90, 0.94, 0.88))
	box.add_child(_coop_step)

	var host_row := HBoxContainer.new()
	host_row.add_theme_constant_override("separation", 10)
	var host_btn := Button.new()
	host_btn.text = "1. Host my drum"
	host_btn.custom_minimum_size = Vector2(0, 44)
	host_btn.add_theme_font_size_override("font_size", 17)
	host_btn.pressed.connect(func():
		if world == null or world.session == null:
			status.text = "No session."
			return
		world.session.host_session()
		_refresh_coop_ui()
		status.text = world.session.status_text)
	host_row.add_child(host_btn)
	var copy_btn := Button.new()
	copy_btn.text = "2. Copy invite"
	copy_btn.custom_minimum_size = Vector2(0, 44)
	copy_btn.add_theme_font_size_override("font_size", 17)
	copy_btn.pressed.connect(func():
		if world and world.session:
			world.session.copy_invite_to_clipboard()
			_refresh_coop_ui()
			status.text = world.session.status_text)
	host_row.add_child(copy_btn)
	box.add_child(host_row)

	_coop_invite = Label.new()
	_coop_invite.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_coop_invite.add_theme_font_size_override("font_size", 22)
	_coop_invite.add_theme_color_override("font_color", Color(0.95, 0.98, 0.85))
	_coop_invite.text = "Invite appears here after you Host."
	box.add_child(_coop_invite)

	_coop_hint = Label.new()
	_coop_hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_coop_hint.add_theme_font_size_override("font_size", 13)
	_coop_hint.add_theme_color_override("font_color", TabStrip.HINT_FG)
	_coop_hint.text = "Same house Wi‑Fi works without Tailscale. Cross-country needs Tailscale (button above)."
	box.add_child(_coop_hint)

	_note(box, "Friend: paste the invite they sent you")
	_coop_join_edit = LineEdit.new()
	_coop_join_edit.placeholder_text = "Paste RAMA-XXXXX-XXXXX here"
	_coop_join_edit.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_coop_join_edit.custom_minimum_size = Vector2(0, 40)
	box.add_child(_coop_join_edit)
	var join_row := HBoxContainer.new()
	join_row.add_theme_constant_override("separation", 10)
	var join_btn := Button.new()
	join_btn.text = "3. Join friend"
	join_btn.custom_minimum_size = Vector2(0, 44)
	join_btn.add_theme_font_size_override("font_size", 17)
	join_btn.pressed.connect(func():
		if world == null or world.session == null:
			return
		world.session.join_invite(_coop_join_edit.text)
		_refresh_coop_ui()
		status.text = world.session.status_text)
	join_row.add_child(join_btn)
	var leave_btn := Button.new()
	leave_btn.text = "Leave"
	leave_btn.custom_minimum_size = Vector2(0, 44)
	leave_btn.pressed.connect(func():
		if world and world.session:
			world.session.leave_session()
			_refresh_coop_ui()
			status.text = world.session.status_text)
	join_row.add_child(leave_btn)
	var recon_btn := Button.new()
	recon_btn.text = "Reconnect"
	recon_btn.custom_minimum_size = Vector2(0, 44)
	recon_btn.pressed.connect(func():
		if world and world.session:
			world.session.reconnect()
			_refresh_coop_ui()
			status.text = world.session.status_text)
	join_row.add_child(recon_btn)
	box.add_child(join_row)

	_coop_status = Label.new()
	_coop_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_coop_status.add_theme_font_size_override("font_size", 15)
	_coop_status.add_theme_color_override("font_color", Color(0.85, 0.92, 0.88))
	_coop_status.text = "Offline."
	box.add_child(_coop_status)

	_note(box, "What your friend may do (you are Host)")
	_coop_perm_checks = {}
	for key in ["dig", "work", "build"]:
		var k: String = key
		var labels := {"dig": "Digging & filling", "work": "Harvest & craft", "build": "Place modules"}
		var default_on: bool = k != "build"
		var chk := _check(labels[k], default_on, func(v):
			if world and world.session:
				world.session.set_perm(k, v)
				_refresh_coop_ui())
		_coop_perm_checks[k] = chk
		box.add_child(chk)
	var undo_guest := Button.new()
	undo_guest.text = "Undo friend's last dig"
	undo_guest.pressed.connect(func():
		if world and world.session:
			world.session.undo_guest()
			status.text = world.session.status_text
			_refresh_coop_ui())
	box.add_child(undo_guest)

	var footer := VBoxContainer.new()
	footer.add_theme_constant_override("separation", 8)
	frame.add_child(footer)

	_hint = Label.new()
	_hint.text = "←/→ tab   ·   home/end first/last   ·   Esc resume"
	_hint.add_theme_font_size_override("font_size", 13)
	_hint.add_theme_color_override("font_color", TabStrip.HINT_FG)
	footer.add_child(_hint)

	status = Label.new()
	status.text = ""
	status.add_theme_font_size_override("font_size", 15)
	status.add_theme_color_override("font_color", Color(1.0, 0.85, 0.5))
	footer.add_child(status)

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
	var invite := Button.new()
	invite.text = "Invite friend"
	invite.custom_minimum_size = Vector2(0, 40)
	invite.add_theme_font_size_override("font_size", 16)
	invite.pressed.connect(func():
		close()
		if world and world.coop_lobby:
			world.coop_lobby.open()
		else:
			open_coop())
	bar.add_child(invite)
	var quit := Button.new()
	quit.text = "Quit"
	quit.custom_minimum_size = Vector2(0, 40)
	quit.add_theme_font_size_override("font_size", 16)
	quit.pressed.connect(func():
		if world and world.has_method("save_world"):
			world.save_world()
		get_tree().quit())
	bar.add_child(quit)
	footer.add_child(bar)
	_refresh_build_stats()
	_tabs.select(0)
	_relayout()
	call_deferred("_hook_session")

## Add a tab and return the VBox its controls go into.
##
## Each page is a full-rect scroller inside `_stack`, so pages do not resize
## each other and a long one (BUILD has fourteen sliders) scrolls on its own.
func _page(tab: String) -> VBoxContainer:
	var sc := ScrollContainer.new()
	sc.set_anchors_preset(Control.PRESET_FULL_RECT)
	sc.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	sc.visible = _pages.is_empty()
	_stack.add_child(sc)
	# Keep content clear of the scrollbar; without it the right-hand column of
	# bindings ran under the track.
	var pad := MarginContainer.new()
	pad.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	pad.add_theme_constant_override("margin_right", 18)
	sc.add_child(pad)
	var box := VBoxContainer.new()
	box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_theme_constant_override("separation", 10)
	pad.add_child(box)
	_pages.append(sc)
	_tabs.add_tab(tab)
	return box

func _show_page(i: int) -> void:
	for k in _pages.size():
		_pages[k].visible = k == i
	if i < _pages.size():
		(_pages[i] as ScrollContainer).scroll_vertical = 0

## Most of the view, and re-derived on resize. The old fixed 800 x 720 was a
## stamp on anything larger than the design resolution.
func _relayout() -> void:
	if panel == null:
		return
	var screen := get_viewport().get_visible_rect().size
	var fs: float = clampf(RamaControls.font_scale, 0.85, 1.4)
	var margin: float = 28.0
	var w: float = minf(screen.x * 0.80, 1240.0 * fs)
	var h: float = minf(screen.y * 0.86, 940.0 * fs)
	w = minf(w, screen.x - margin * 2.0)
	h = minf(h, screen.y - margin * 2.0)
	panel.size = Vector2(w, h)
	panel.position = Vector2((screen.x - w) * 0.5, (screen.y - h) * 0.5)
	_tabs.set_font_scale(fs)

## The resolved value of one build axis — the hand-set override if there is one,
## otherwise whatever the archetype (and blend) currently make it.
func _axis(axis_name: String) -> float:
	var axes: Dictionary = RamaControls.avatar.get("axes", {})
	if axes.has(axis_name):
		return float(axes[axis_name])
	return float(_resolved_body().get(axis_name, 0.5))

## Archetype, blended, with any hand-set axis on top — the same body the rig
## builds. One resolver, so the readout and the man cannot disagree.
func _resolved_body() -> Dictionary:
	var spec: Dictionary = RamaBody.make(str(RamaControls.avatar.get("archetype", "daddy")))
	var to: String = str(RamaControls.avatar.get("blend_to", ""))
	var mix: float = float(RamaControls.avatar.get("blend", 0.0))
	if to != "" and mix > 0.001 and RamaBody.ARCHETYPES.has(to):
		spec = RamaBody.blend(spec, RamaBody.make(to), mix)
	for k in RamaControls.avatar.get("axes", {}):
		spec[k] = RamaControls.avatar["axes"][k]
	return spec

## What the numbers add up to. Sliders say what you asked for; this says what
## you got — a build editor should show the body's actual proportions, not just
## the inputs that produced them.
func _refresh_build_stats() -> void:
	if build_stats == null:
		return
	var m: Dictionary = RamaBody.measure(_resolved_body())
	var reach: float = float(m["arm_len"]) + float(m["fore_len"]) + float(m["hand_l"])
	build_stats.text = ("%.2f m tall  ·  shoulders %.0f cm  ·  waist %.0f cm  ·  "
			+ "chest depth %.0f cm  ·  reach %.0f cm  ·  head %.0f%% of height") % [
			float(m["stature"]), float(m["sh_w"]) * 100.0, float(m["waist_w"]) * 100.0,
			float(m["chest_d"]) * 100.0, reach * 100.0,
			float(m["head_h"]) / maxf(float(m["stature"]), 0.01) * 100.0]

func _set_axis(axis_name: String, v: float) -> void:
	var axes: Dictionary = RamaControls.avatar.get("axes", {})
	axes[axis_name] = v
	RamaControls.avatar["axes"] = axes
	_apply_build()

func _apply_build() -> void:
	_refresh_build_stats()
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
	l.add_theme_color_override("font_color", TabStrip.HEAD_FG)
	box.add_child(l)

func _note(box: VBoxContainer, t: String) -> void:
	var l := Label.new()
	l.text = t
	l.add_theme_font_size_override("font_size", 14)
	l.add_theme_color_override("font_color", TabStrip.NOTE_FG)
	box.add_child(l)

func _slider(box: VBoxContainer, label: String, lo: float, hi: float,
		val: float, cb: Callable) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 16)
	var l := Label.new()
	l.text = label
	l.custom_minimum_size = Vector2(240, 0)
	l.add_theme_font_size_override("font_size", 16)
	row.add_child(l)
	var s := HSlider.new()
	s.min_value = lo
	s.max_value = hi
	s.step = (hi - lo) / 100.0
	s.value = val
	s.custom_minimum_size = Vector2(360, 28)
	s.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(s)
	# A body editor with fourteen unlabelled sliders is a guessing game: you
	# could not see what Muscle was set to, only where the handle happened to
	# sit. Metres read as metres; the 0-1 axes read as percentages, because
	# "0.62" means nothing next to a word like Soft.
	var readout := Label.new()
	readout.custom_minimum_size = Vector2(76, 0)
	readout.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	readout.add_theme_font_size_override("font_size", 15)
	readout.add_theme_color_override("font_color", TabStrip.NOTE_FG)
	var fmt := func(v: float) -> String:
		return "%.2f m" % v if hi > 1.5 else "%d%%" % roundi(v * 100.0)
	readout.text = fmt.call(val)
	row.add_child(readout)
	s.value_changed.connect(func(v: float):
		readout.text = fmt.call(v)
		cb.call(v))
	row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_child(row)

func _listen(id: String) -> void:
	listening = id
	status.text = "Press a key for '%s' ...  (Esc cancels)" % id
	rows[id].text = "…"

func _input(e: InputEvent) -> void:
	if not visible:
		return
	# Closing is the menu's own job. `player.gd` polls the pause action, but
	# `open()` pauses the tree and the player is not PROCESS_MODE_ALWAYS — so
	# once the menu was up nothing was left running to notice the key. Esc
	# opened it and then could not close it.
	#
	# The rebound pause key works, and literal Escape always does too, so there
	# is no way to bind yourself out of the menu.
	var pause_act: String = RamaControls.act("pause")
	var pause_hit: bool = InputMap.has_action(pause_act) and e.is_action_pressed(pause_act)
	if listening == "" and (pause_hit
			or (e is InputEventKey and e.pressed and not e.echo
				and (e as InputEventKey).keycode == KEY_ESCAPE)):
		close()
		get_viewport().set_input_as_handled()
		return
	# Tab navigation is off while a binding is listening, or an arrow key could
	# never be bound to anything.
	if listening == "" and e is InputEventKey and e.pressed and not e.echo:
		match (e as InputEventKey).keycode:
			KEY_LEFT:
				_tabs.prev()
			KEY_RIGHT:
				_tabs.next()
			KEY_HOME:
				_tabs.select(0)
			KEY_END:
				_tabs.select(_tabs.count() - 1)
			_:
				return
		get_viewport().set_input_as_handled()
		return
	if listening == "":
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

func open_coop() -> void:
	# Prefer the dedicated lobby modal (Tailscale check + Host/Join).
	if world and world.coop_lobby:
		if visible:
			close()
		world.coop_lobby.open()
		return
	_hook_session()
	open()
	if _coop_tab_index >= 0:
		_tabs.select(_coop_tab_index)
		_show_page(_coop_tab_index)
	_refresh_coop_ui()
	status.text = "Invite a friend — Host, Copy, send the code."

func _hook_session() -> void:
	if world == null or world.session == null:
		return
	if world.session.status_changed.is_connected(_on_coop_status):
		return
	world.session.status_changed.connect(_on_coop_status)
	world.session.invite_ready.connect(_on_coop_invite)

func _on_coop_status(_m: String) -> void:
	_refresh_coop_ui()

func _on_coop_invite(_a: String, _b: String) -> void:
	_refresh_coop_ui()

func _refresh_coop_ui() -> void:
	if _coop_status == null:
		return
	if world == null or world.session == null:
		_coop_status.text = "Offline."
		return
	var s = world.session
	_coop_status.text = s.status_text
	var lines: PackedStringArray = []
	if s.invite_tailscale != "":
		lines.append("Tailscale invite (use this):  %s" % s.invite_tailscale)
	if s.invite_wan != "":
		lines.append("Internet invite:  %s" % s.invite_wan)
	if s.invite_lan != "":
		lines.append("Same Wi‑Fi only:  %s" % s.invite_lan)
	if lines.is_empty():
		_coop_invite.text = "Invite appears here after you Host."
	else:
		_coop_invite.text = "\n".join(lines)
	if s.using_tailscale:
		_coop_hint.text = "Tailscale is live — Copy invite and send it. Friend must also be online in Tailscale."
		if _coop_distance:
			_coop_distance.text = "Tailscale detected — Seattle ↔ Michigan should work."
			_coop_distance.add_theme_color_override("font_color", Color(0.55, 0.95, 0.70))
	elif s.upnp_msg != "":
		_coop_hint.text = s.upnp_msg
	elif s.hosting:
		_coop_hint.text = "No Tailscale yet. Cross-country will likely fail — install Tailscale on both PCs, then Host again."
	elif s.joined:
		_coop_hint.text = "You're visiting. Esc resumes play."
	else:
		_coop_hint.text = "Cross-country: both install Tailscale first. Same house: Host → Copy → Join is enough."
		if _coop_distance:
			_coop_distance.text = "Different cities (Seattle ↔ Michigan): both install Tailscale first — free, ~2 minutes."
			_coop_distance.add_theme_color_override("font_color", Color(1.0, 0.88, 0.55))
	for k in _coop_perm_checks:
		var chk: CheckBox = _coop_perm_checks[k]
		if chk and s.perms.has(k):
			chk.set_pressed_no_signal(bool(s.perms[k]))

func open() -> void:
	visible = true
	listening = ""
	_relayout()
	_hook_session()
	if not first_run_mode:
		status.text = ""
	get_tree().paused = true
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE

func close() -> void:
	_closed_at = Time.get_ticks_msec()
	visible = false
	get_tree().paused = false
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
	if first_run_mode:
		first_run_mode = false
		RamaControls.save_cfg()

func toggle() -> void:
	if visible:
		close()
		return
	# Swallow the reopen that the same keystroke would otherwise cause.
	if _closed_at >= 0.0 and Time.get_ticks_msec() - _closed_at < REOPEN_LOCKOUT_MS:
		return
	open()
