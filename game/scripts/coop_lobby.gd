extends CanvasLayer
## Co-op lobby modal — guides Tailscale setup, then Host / Join.
##
## Cross-country (Seattle↔Michigan) needs Tailscale. This screen finds it,
## waits while you install, then Hosts with the right invite.

const TabStrip = preload("res://scripts/ui/tabs.gd")

var world
var panel: PanelContainer
var title_l: Label
var body_l: Label
var status_l: Label
var ts_badge: Label
var invite_l: Label
var join_edit: LineEdit
var host_btn: Button
var copy_btn: Button
var join_btn: Button
var _poll_accum := 0.0
var _open := false

func _ready() -> void:
	layer = 55
	process_mode = Node.PROCESS_MODE_ALWAYS
	visible = false
	_build()

func _build() -> void:
	var dim := ColorRect.new()
	dim.set_anchors_preset(Control.PRESET_FULL_RECT)
	dim.color = Color(0.02, 0.03, 0.05, 0.78)
	dim.gui_input.connect(func(e):
		if e is InputEventMouseButton and e.pressed:
			close())
	add_child(dim)

	panel = PanelContainer.new()
	panel.set_anchors_preset(Control.PRESET_CENTER)
	panel.custom_minimum_size = Vector2(560, 520)
	panel.offset_left = -280
	panel.offset_top = -260
	panel.offset_right = 280
	panel.offset_bottom = 260
	var sb := StyleBoxFlat.new()
	sb.bg_color = TabStrip.PANE_BG
	sb.border_color = Color(0.55, 0.78, 0.72, 0.40)
	sb.set_border_width_all(1)
	sb.set_corner_radius_all(10)
	sb.content_margin_left = 28
	sb.content_margin_right = 28
	sb.content_margin_top = 22
	sb.content_margin_bottom = 22
	panel.add_theme_stylebox_override("panel", sb)
	add_child(panel)

	var root := VBoxContainer.new()
	root.add_theme_constant_override("separation", 12)
	panel.add_child(root)

	title_l = Label.new()
	title_l.text = "Play with a friend"
	title_l.add_theme_font_size_override("font_size", 26)
	title_l.add_theme_color_override("font_color", Color(0.95, 0.97, 0.92))
	root.add_child(title_l)

	body_l = Label.new()
	body_l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body_l.text = "Different cities need a tiny free app called Tailscale so your computers can find each other."
	body_l.add_theme_font_size_override("font_size", 15)
	body_l.add_theme_color_override("font_color", Color(0.78, 0.86, 0.82))
	root.add_child(body_l)

	ts_badge = Label.new()
	ts_badge.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	ts_badge.add_theme_font_size_override("font_size", 17)
	ts_badge.add_theme_color_override("font_color", Color(1.0, 0.88, 0.55))
	ts_badge.text = "Checking connection…"
	root.add_child(ts_badge)

	var ts_row := HBoxContainer.new()
	ts_row.add_theme_constant_override("separation", 10)
	var open_ts := Button.new()
	open_ts.text = "Install Tailscale"
	open_ts.custom_minimum_size = Vector2(0, 42)
	open_ts.add_theme_font_size_override("font_size", 16)
	open_ts.pressed.connect(func():
		OS.shell_open("https://tailscale.com/download")
		_set_status("Browser opened. Install → sign in → come back here. We'll keep checking.")
		_refresh_network())
	ts_row.add_child(open_ts)
	var recheck := Button.new()
	recheck.text = "I've installed it — recheck"
	recheck.custom_minimum_size = Vector2(0, 42)
	recheck.add_theme_font_size_override("font_size", 16)
	recheck.pressed.connect(func():
		_refresh_network()
		if world and world.session and world.session.has_tailscale():
			_set_status("Tailscale found. You can Host now.")
		else:
			_set_status("Still not found. Is Tailscale running and signed in?")
	)
	ts_row.add_child(recheck)
	root.add_child(ts_row)

	var sep := HSeparator.new()
	root.add_child(sep)

	var host_head := Label.new()
	host_head.text = "You host"
	host_head.add_theme_font_size_override("font_size", 18)
	host_head.add_theme_color_override("font_color", TabStrip.HEAD_FG)
	root.add_child(host_head)

	var host_row := HBoxContainer.new()
	host_row.add_theme_constant_override("separation", 10)
	host_btn = Button.new()
	host_btn.text = "Host my drum"
	host_btn.custom_minimum_size = Vector2(0, 44)
	host_btn.add_theme_font_size_override("font_size", 17)
	host_btn.pressed.connect(_on_host)
	host_row.add_child(host_btn)
	copy_btn = Button.new()
	copy_btn.text = "Copy invite"
	copy_btn.custom_minimum_size = Vector2(0, 44)
	copy_btn.add_theme_font_size_override("font_size", 17)
	copy_btn.pressed.connect(func():
		if world and world.session:
			world.session.copy_invite_to_clipboard()
			_set_status(world.session.status_text)
			_refresh_invite())
	host_row.add_child(copy_btn)
	root.add_child(host_row)

	invite_l = Label.new()
	invite_l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	invite_l.add_theme_font_size_override("font_size", 20)
	invite_l.add_theme_color_override("font_color", Color(0.95, 0.98, 0.85))
	invite_l.text = "Invite appears after you Host."
	root.add_child(invite_l)

	var join_head := Label.new()
	join_head.text = "Friend joins"
	join_head.add_theme_font_size_override("font_size", 18)
	join_head.add_theme_color_override("font_color", TabStrip.HEAD_FG)
	root.add_child(join_head)

	join_edit = LineEdit.new()
	join_edit.placeholder_text = "Paste RAMA-XXXXX-XXXXX"
	join_edit.custom_minimum_size = Vector2(0, 40)
	root.add_child(join_edit)

	var join_row := HBoxContainer.new()
	join_row.add_theme_constant_override("separation", 10)
	join_btn = Button.new()
	join_btn.text = "Join friend"
	join_btn.custom_minimum_size = Vector2(0, 44)
	join_btn.add_theme_font_size_override("font_size", 17)
	join_btn.pressed.connect(_on_join)
	join_row.add_child(join_btn)
	var leave_btn := Button.new()
	leave_btn.text = "Leave"
	leave_btn.custom_minimum_size = Vector2(0, 44)
	leave_btn.pressed.connect(func():
		if world and world.session:
			world.session.leave_session()
			_set_status(world.session.status_text)
			_refresh_invite()
			_refresh_network())
	join_row.add_child(leave_btn)
	var close_btn := Button.new()
	close_btn.text = "Close"
	close_btn.custom_minimum_size = Vector2(0, 44)
	close_btn.pressed.connect(close)
	join_row.add_child(close_btn)
	root.add_child(join_row)

	status_l = Label.new()
	status_l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status_l.add_theme_font_size_override("font_size", 14)
	status_l.add_theme_color_override("font_color", Color(0.85, 0.92, 0.88))
	status_l.text = ""
	root.add_child(status_l)

func open() -> void:
	_open = true
	visible = true
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	# Don't pause the whole tree — Tailscale poll + Host need the session alive,
	# and the friend may already be connecting.
	if get_tree().paused and world and world.menu and world.menu.visible:
		pass
	elif not get_tree().paused:
		pass
	_hook_session()
	_refresh_network()
	_refresh_invite()
	_set_status("Checking whether Tailscale is on this computer…")

func close() -> void:
	_open = false
	visible = false
	if world and world.menu and not world.menu.visible:
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED

func toggle() -> void:
	if visible:
		close()
	else:
		open()

func _process(dt: float) -> void:
	if not visible:
		return
	_poll_accum += dt
	if _poll_accum < 1.5:
		return
	_poll_accum = 0.0
	# Keep hunting for Tailscale while the modal is open (install-in-progress).
	if world and world.session and not world.session.is_online():
		var had: bool = world.session.using_tailscale or world.session.has_tailscale()
		_refresh_network()
		if not had and world.session.has_tailscale():
			_set_status("Tailscale just appeared — you're ready to Host.")

func _hook_session() -> void:
	if world == null or world.session == null:
		return
	if not world.session.status_changed.is_connected(_on_status):
		world.session.status_changed.connect(_on_status)
	if not world.session.invite_ready.is_connected(_on_invite):
		world.session.invite_ready.connect(_on_invite)

func _on_status(msg: String) -> void:
	_set_status(msg)
	_refresh_invite()
	_refresh_network()

func _on_invite(_a: String, _b: String) -> void:
	_refresh_invite()
	_refresh_network()

func _set_status(msg: String) -> void:
	if status_l:
		status_l.text = msg

func _refresh_network() -> void:
	if world == null or world.session == null or ts_badge == null:
		return
	var s = world.session
	# Refresh detection even before hosting.
	var ts_ip: String = ""
	if s.has_method("get_tailscale_ip"):
		ts_ip = s.get_tailscale_ip()
	elif s.has_method("has_tailscale") and s.has_tailscale():
		ts_ip = "(connected)"
	if ts_ip != "":
		ts_badge.text = "Tailscale found · %s — different cities will work." % ts_ip
		ts_badge.add_theme_color_override("font_color", Color(0.55, 0.95, 0.70))
		body_l.text = "You're set for long-distance. Host, copy the invite, send it in Discord. Friend needs Tailscale on too."
		if host_btn:
			host_btn.disabled = false
	else:
		ts_badge.text = "Tailscale not found yet — needed for Seattle ↔ Michigan (or any different Wi‑Fi)."
		ts_badge.add_theme_color_override("font_color", Color(1.0, 0.88, 0.55))
		body_l.text = "1) Install Tailscale on both computers  2) Sign in  3) Add each other  4) Come back and Recheck  5) Host"
		# Still allow host for same-LAN testing.
		if host_btn:
			host_btn.disabled = false

func _refresh_invite() -> void:
	if invite_l == null or world == null or world.session == null:
		return
	var s = world.session
	var lines: PackedStringArray = []
	if s.invite_tailscale != "":
		lines.append("Send this:  %s" % s.invite_tailscale)
	elif s.best_invite() != "":
		lines.append("Send this:  %s" % s.best_invite())
	if s.invite_lan != "" and s.invite_tailscale == "":
		lines.append("(Wi‑Fi only backup: %s)" % s.invite_lan)
	if lines.is_empty():
		invite_l.text = "Invite appears after you Host."
	else:
		invite_l.text = "\n".join(lines)

func _on_host() -> void:
	if world == null or world.session == null:
		return
	if not world.session.has_tailscale():
		_set_status("No Tailscale yet — Host will only work on the same Wi‑Fi. Install Tailscale for Michigan.")
	world.session.host_session()
	_set_status(world.session.status_text)
	_refresh_invite()
	_refresh_network()
	if world.session.best_invite() != "":
		world.session.copy_invite_to_clipboard()
		_set_status("Hosting — invite copied. Paste it in Discord.")

func _on_join() -> void:
	if world == null or world.session == null:
		return
	if not world.session.has_tailscale():
		_set_status("Tip: join works best with Tailscale on. Continuing anyway…")
	world.session.join_invite(join_edit.text)
	_set_status(world.session.status_text)

func _unhandled_input(e: InputEvent) -> void:
	if not visible:
		return
	if e.is_action_pressed("ui_cancel") or (
			e is InputEventKey and e.pressed and not e.echo
			and (e as InputEventKey).keycode == KEY_ESCAPE):
		close()
		get_viewport().set_input_as_handled()
