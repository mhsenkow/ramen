extends Node
## Live co-op session — host-authoritative shared drum.
##
## Friend-test UX: invite codes, clipboard copy, UPnP port mapping, on-screen
## banner. Host runs RamaTerrain + sim_tick; guests send intents over ENet.

const RamaBody = preload("res://scripts/avatar/body.gd")
const DEFAULT_PORT := 24567
const POSE_HZ := 20.0
const SCHEMA_FALLBACK := 1
## Crockford-ish alphabet — no I/L/O/U so codes are easy to read aloud.
const _B32 := "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

signal status_changed(msg: String)
signal invite_ready(lan_code: String, wan_code: String)

var world
var peer: ENetMultiplayerPeer
var hosting := false
var joined := false
var port := DEFAULT_PORT
var join_address := "127.0.0.1"
var status_text := "Offline — Esc → CO-OP to invite a friend."
var pose_accum := 0.0
var actor_id := 0
var remotes: Dictionary = {}
## Friend-test defaults: dig/work on so a visit is playable without hunting toggles.
var perms := {
	"look": true,
	"walk": true,
	"work": true,
	"dig": true,
	"build": false,
}
var awaiting_bootstrap := false
var last_disconnect_reason := ""
var reconnect_address := ""
var reconnect_port := DEFAULT_PORT
var _last_inv_snap: Dictionary = {}
var invite_lan := ""
var invite_wan := ""
var invite_tailscale := ""
var upnp_ok := false
var upnp_msg := ""
var using_tailscale := false
var friend_count := 0
var _banner: CanvasLayer
var _banner_label: Label
var _banner_detail: Label
var _upnp_node: UPNP

func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	multiplayer.peer_connected.connect(_on_peer_connected)
	multiplayer.peer_disconnected.connect(_on_peer_disconnected)
	multiplayer.connected_to_server.connect(_on_connected_to_server)
	multiplayer.connection_failed.connect(_on_connection_failed)
	multiplayer.server_disconnected.connect(_on_server_disconnected)
	_build_banner()

func is_online() -> bool:
	return hosting or joined

func is_authority() -> bool:
	return not joined

func local_actor() -> int:
	if not is_online():
		return 0
	if world and world.terrain and world.terrain.has_method("net_actor_id"):
		return int(world.terrain.net_actor_id(multiplayer.get_unique_id()))
	return 0 if multiplayer.get_unique_id() <= 1 else 1000 + multiplayer.get_unique_id()

func schema() -> int:
	if world and world.terrain and world.terrain.has_method("schema_version"):
		return int(world.terrain.schema_version())
	return SCHEMA_FALLBACK

func best_invite() -> String:
	# Tailscale beats WAN/UPnP for cross-country (Seattle↔Michigan).
	if invite_tailscale != "":
		return invite_tailscale
	if invite_wan != "":
		return invite_wan
	return invite_lan

func has_tailscale() -> bool:
	return get_tailscale_ip() != ""

func get_tailscale_ip() -> String:
	return _tailscale_ip()

func set_status(msg: String) -> void:
	status_text = msg
	status_changed.emit(msg)
	_refresh_banner()
	if world and world.has_method("note"):
		world.note(msg)
	print("[coop] ", msg)

func host_session(p: int = DEFAULT_PORT) -> bool:
	if is_online():
		set_status("Already in a session — Leave first.")
		return false
	port = p
	peer = ENetMultiplayerPeer.new()
	var err := peer.create_server(port, 4)
	if err != OK:
		set_status("Could not host — port %d may be in use. Try again." % port)
		peer = null
		return false
	multiplayer.multiplayer_peer = peer
	hosting = true
	joined = false
	actor_id = 0
	friend_count = 0
	if world and world.terrain and world.terrain.has_method("set_actor_pos") and world.player:
		world.terrain.set_actor_pos(0, world.player.theta, world.player.z)
	_build_invites()
	if using_tailscale:
		set_status("Hosting over Tailscale. Copy invite and send it — works Seattle↔Michigan.")
		invite_ready.emit(invite_lan, invite_wan)
		return true
	_try_upnp_async()
	set_status("Different cities? Install Tailscale (both of you) — Esc → CO-OP has the link. Trying router open anyway…")
	invite_ready.emit(invite_lan, invite_wan)
	return true

func join_invite(raw: String) -> bool:
	var parsed := parse_invite(raw)
	if not parsed.get("ok", false):
		set_status(str(parsed.get("error", "That invite doesn't look right.")))
		return false
	return join_session(str(parsed["ip"]), int(parsed["port"]))

func join_session(address: String, p: int = DEFAULT_PORT) -> bool:
	if is_online():
		set_status("Already in a session — Leave first.")
		return false
	join_address = address.strip_edges()
	port = p
	reconnect_address = join_address
	reconnect_port = port
	peer = ENetMultiplayerPeer.new()
	var err := peer.create_client(join_address, port)
	if err != OK:
		set_status("Could not start join to %s." % join_address)
		peer = null
		return false
	multiplayer.multiplayer_peer = peer
	awaiting_bootstrap = true
	set_status("Connecting to friend… (10–20s). Keep this window open.")
	return true

func leave_session() -> void:
	_clear_upnp()
	if peer:
		peer.close()
	multiplayer.multiplayer_peer = null
	peer = null
	hosting = false
	joined = false
	awaiting_bootstrap = false
	friend_count = 0
	invite_lan = ""
	invite_wan = ""
	invite_tailscale = ""
	upnp_ok = false
	upnp_msg = ""
	using_tailscale = false
	_clear_remotes()
	set_status("Left co-op. Use Discord/Steam for voice — the game has no chat.")

func reconnect() -> bool:
	if hosting:
		set_status("You're the host — keep Hosting; your friend should Reconnect.")
		return false
	var addr := reconnect_address
	var p := reconnect_port
	leave_session()
	return join_session(addr, p)

func copy_invite_to_clipboard() -> String:
	var code := best_invite()
	if code == "":
		set_status("Host first, then copy the invite.")
		return ""
	DisplayServer.clipboard_set(code)
	var kind := "Tailscale"
	if code == invite_tailscale:
		kind = "Tailscale"
	elif code == invite_wan and invite_wan != "":
		kind = "internet"
	else:
		kind = "Wi‑Fi"
	set_status("Copied %s invite. Paste it to your friend." % kind)
	return code

func _build_invites() -> void:
	var lan := _lan_ip()
	invite_lan = encode_invite(lan, port)
	invite_wan = ""
	invite_tailscale = ""
	using_tailscale = false
	var ts := _tailscale_ip()
	if ts != "":
		invite_tailscale = encode_invite(ts, port)
		using_tailscale = true
		upnp_msg = "Tailscale detected (%s). Different cities will work — both must be online in Tailscale." % ts

func _try_upnp_async() -> void:
	# Discover can hitch — defer one frame so Host button feels instant.
	call_deferred("_try_upnp")

func _try_upnp() -> void:
	if not hosting:
		return
	_upnp_node = UPNP.new()
	var disc: int = _upnp_node.discover(2000, 2, "InternetGatewayDevice")
	if disc != UPNP.UPNP_RESULT_SUCCESS:
		disc = _upnp_node.discover(2000, 2, "")
	if disc != UPNP.UPNP_RESULT_SUCCESS:
		upnp_ok = false
		upnp_msg = "Router auto-open failed — same Wi‑Fi still works; different houses need port forward UDP %d." % port
		set_status("%s Same-Wi‑Fi invite: %s" % [upnp_msg, invite_lan])
		invite_ready.emit(invite_lan, invite_wan)
		return
	var map_err: int = _upnp_node.add_port_mapping(port, port, "RAMA co-op", "UDP")
	if map_err != UPNP.UPNP_RESULT_SUCCESS:
		# Some gateways want an explicit lease.
		map_err = _upnp_node.add_port_mapping(port, port, "RAMA co-op", "UDP", 0)
	var ext := _upnp_node.query_external_address()
	if ext != "" and not ext.begins_with("0.") and "." in ext:
		invite_wan = encode_invite(ext, port)
	upnp_ok = map_err == UPNP.UPNP_RESULT_SUCCESS and invite_wan != ""
	if upnp_ok:
		upnp_msg = "Router opened UDP %d for internet friends." % port
		set_status("Hosting. Internet invite copied-ready: %s (Wi‑Fi: %s)" % [invite_wan, invite_lan])
	elif invite_wan != "":
		upnp_msg = "Found public IP but port map failed — friend may need you on the same Wi‑Fi."
		set_status("%s Try Wi‑Fi invite: %s" % [upnp_msg, invite_lan])
	else:
		upnp_msg = "No public IP from router — use same Wi‑Fi invite."
		set_status("%s %s" % [upnp_msg, invite_lan])
	invite_ready.emit(invite_lan, invite_wan)

func _clear_upnp() -> void:
	if _upnp_node == null:
		return
	_upnp_node.delete_port_mapping(port, "UDP")
	_upnp_node = null

# ------------------------------------------------------------------ invite --

static func encode_invite(ip: String, p: int) -> String:
	var parts := ip.strip_edges().split(".")
	if parts.size() != 4:
		return ""
	var raw := PackedByteArray()
	for part in parts:
		if not str(part).is_valid_int():
			return ""
		raw.append(clampi(int(part), 0, 255))
	var port_i := clampi(p, 1, 65535)
	raw.append((port_i >> 8) & 0xFF)
	raw.append(port_i & 0xFF)
	var enc := _b32_encode(raw)
	# 6 bytes → 10 base32 chars (48 bits).
	while enc.length() < 10:
		enc += "0"
	enc = enc.substr(0, 10)
	return "RAMA-%s-%s" % [enc.substr(0, 5), enc.substr(5, 5)]

static func parse_invite(raw: String) -> Dictionary:
	var s := raw.strip_edges().to_upper().replace(" ", "")
	if s == "":
		return {"ok": false, "error": "Paste the invite your friend sent."}
	# Accept raw ip or ip:port for power users.
	if s.contains(".") and not s.begins_with("RAMA"):
		var raw_ip := s
		var raw_port := DEFAULT_PORT
		if ":" in s:
			var bits := s.split(":")
			raw_ip = bits[0]
			if bits.size() > 1 and str(bits[1]).is_valid_int():
				raw_port = int(bits[1])
		if raw_ip.split(".").size() == 4:
			return {"ok": true, "ip": raw_ip, "port": raw_port}
		return {"ok": false, "error": "Use a RAMA-XXXXX-XXXXX invite, or an IP address."}
	s = s.replace("RAMA-", "").replace("-", "")
	if s.length() < 10:
		return {"ok": false, "error": "Invite looks incomplete — need RAMA-XXXXX-XXXXX."}
	var body := s.substr(0, 10)
	var bytes := _b32_decode(body)
	if bytes.size() < 6:
		return {"ok": false, "error": "Could not read that invite. Ask them to Copy Invite again."}
	var decoded_ip := "%d.%d.%d.%d" % [bytes[0], bytes[1], bytes[2], bytes[3]]
	var decoded_port := (int(bytes[4]) << 8) | int(bytes[5])
	return {"ok": true, "ip": decoded_ip, "port": decoded_port}

static func _b32_encode(data: PackedByteArray) -> String:
	var out := ""
	var buffer := 0
	var bits_left := 0
	for b in data:
		buffer = (buffer << 8) | (b & 0xFF)
		bits_left += 8
		while bits_left >= 5:
			bits_left -= 5
			var idx: int = (buffer >> bits_left) & 31
			out += _B32[idx]
	if bits_left > 0:
		var idx2: int = (buffer << (5 - bits_left)) & 31
		out += _B32[idx2]
	return out

static func _b32_decode(text: String) -> PackedByteArray:
	var out := PackedByteArray()
	var buffer := 0
	var bits_left := 0
	for ch in text:
		var idx := _B32.find(ch)
		if idx < 0:
			continue
		buffer = (buffer << 5) | idx
		bits_left += 5
		if bits_left >= 8:
			bits_left -= 8
			out.append((buffer >> bits_left) & 0xFF)
	return out

func _lan_ip() -> String:
	var best := ""
	for a in IP.get_local_addresses():
		if a.begins_with("192.168.") or a.begins_with("10."):
			return a
		if a.begins_with("172."):
			var second := int(a.split(".")[1]) if a.split(".").size() > 1 else 0
			if second >= 16 and second <= 31:
				best = a
	if best != "":
		return best
	for a in IP.get_local_addresses():
		if a != "127.0.0.1" and "." in a and not a.contains(":"):
			return a
	return "127.0.0.1"

## Tailscale assigns 100.64.0.0/10 (CGNAT). Prefer that for cross-country co-op.
func _tailscale_ip() -> String:
	for a in IP.get_local_addresses():
		if not a.contains(".") or a.contains(":"):
			continue
		var parts := a.split(".")
		if parts.size() != 4:
			continue
		if not str(parts[0]).is_valid_int() or not str(parts[1]).is_valid_int():
			continue
		var a0 := int(parts[0])
		var a1 := int(parts[1])
		# 100.64.0.0 – 100.127.255.255
		if a0 == 100 and a1 >= 64 and a1 <= 127:
			return a
	return ""

# ------------------------------------------------------------------ banner --

func _build_banner() -> void:
	_banner = CanvasLayer.new()
	_banner.layer = 40
	_banner.visible = false
	add_child(_banner)
	var panel := PanelContainer.new()
	panel.set_anchors_preset(Control.PRESET_TOP_WIDE)
	panel.offset_left = 16
	panel.offset_right = -16
	panel.offset_top = 10
	panel.offset_bottom = 78
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.04, 0.07, 0.10, 0.82)
	sb.border_color = Color(0.55, 0.78, 0.72, 0.45)
	sb.set_border_width_all(1)
	sb.set_corner_radius_all(8)
	sb.content_margin_left = 14
	sb.content_margin_right = 14
	sb.content_margin_top = 8
	sb.content_margin_bottom = 8
	panel.add_theme_stylebox_override("panel", sb)
	_banner.add_child(panel)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 12)
	panel.add_child(row)
	var col := VBoxContainer.new()
	col.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	col.add_theme_constant_override("separation", 2)
	row.add_child(col)
	_banner_label = Label.new()
	_banner_label.add_theme_font_size_override("font_size", 16)
	_banner_label.add_theme_color_override("font_color", Color(0.92, 0.96, 0.94))
	col.add_child(_banner_label)
	_banner_detail = Label.new()
	_banner_detail.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_banner_detail.add_theme_font_size_override("font_size", 13)
	_banner_detail.add_theme_color_override("font_color", Color(0.72, 0.82, 0.78))
	col.add_child(_banner_detail)
	var copy_btn := Button.new()
	copy_btn.text = "Copy invite"
	copy_btn.custom_minimum_size = Vector2(120, 36)
	copy_btn.pressed.connect(func(): copy_invite_to_clipboard())
	row.add_child(copy_btn)
	var leave_btn := Button.new()
	leave_btn.text = "Leave"
	leave_btn.custom_minimum_size = Vector2(80, 36)
	leave_btn.pressed.connect(leave_session)
	row.add_child(leave_btn)

func _refresh_banner() -> void:
	if _banner == null:
		return
	_banner.visible = is_online() or awaiting_bootstrap
	if not _banner.visible:
		return
	if hosting:
		_banner_label.text = "Hosting your drum · %d friend(s) here" % friend_count
		var code := best_invite()
		var via := "Tailscale" if using_tailscale else ("internet" if invite_wan != "" else "local")
		_banner_detail.text = "Invite (%s): %s   ·   Esc → CO-OP" % [via, code if code != "" else "…"]
	elif joined or awaiting_bootstrap:
		_banner_label.text = "Visiting a friend's drum"
		_banner_detail.text = status_text
	else:
		_banner_label.text = "Co-op"
		_banner_detail.text = status_text

func _process(dt: float) -> void:
	if not is_online() or world == null or world.player == null:
		return
	pose_accum += dt
	if pose_accum < 1.0 / POSE_HZ:
		return
	pose_accum = 0.0
	var p: Node = world.player
	_rpc_pose.rpc(local_actor(), p.theta, p.z, p.yaw, p.pitch, p.r)
	if is_authority() and world.terrain and world.terrain.has_method("set_actor_pos"):
		world.terrain.set_actor_pos(local_actor(), p.theta, p.z)
		_flush_host_events()

func _flush_host_events() -> void:
	if not hosting or world == null or world.terrain == null:
		return
	if not world.terrain.has_method("take_mutation_events"):
		return
	var evs: Array = world.terrain.take_mutation_events()
	if evs.is_empty():
		return
	_rpc_events.rpc(evs)

func can_dig() -> bool:
	if is_authority():
		return true
	return bool(perms.get("dig", false))

func can_build() -> bool:
	if is_authority():
		return true
	return bool(perms.get("build", false))

func can_work() -> bool:
	if is_authority():
		return true
	return bool(perms.get("work", false))

func request_dig(p: Vector3, radius: float, snap: float, level: bool, remove: bool) -> void:
	if not can_dig():
		set_status("Your friend hasn't allowed digging yet.")
		return
	_rpc_dig_intent.rpc_id(1, local_actor(), p, radius, snap, level, remove)

func request_harvest(theta: float, z: float, radius: float) -> void:
	if not can_work():
		set_status("Your friend hasn't allowed harvesting yet.")
		return
	_rpc_harvest_intent.rpc_id(1, local_actor(), theta, z, radius)

func request_place_veg(p: Vector3, as_leaf: bool) -> void:
	if not can_build() and not can_dig():
		set_status("Your friend hasn't allowed building yet.")
		return
	_rpc_veg_intent.rpc_id(1, local_actor(), p, as_leaf)

func request_craft(index: int, scale: float, theta: float, z: float) -> void:
	if not can_work():
		set_status("Your friend hasn't allowed crafting yet.")
		return
	_rpc_craft_intent.rpc_id(1, local_actor(), index, scale, theta, z)

func set_perm(key: String, on: bool) -> void:
	if not hosting:
		return
	perms[key] = on
	_rpc_perms.rpc(perms)
	var label: String = str({
		"dig": "digging",
		"work": "harvest & craft",
		"build": "building",
		"walk": "walking",
	}.get(key, key))
	set_status("Friend %s: %s" % [label, "allowed" if on else "blocked"])

func undo_guest() -> void:
	if not hosting or world == null:
		return
	if not world.terrain.has_method("undo_last_guest"):
		return
	var hit: Dictionary = world.terrain.undo_last_guest()
	if hit.get("ok", false):
		world.rebuild_around(hit["point"], float(hit.get("radius", 2.0)) + 2.0)
		_flush_host_events()
		_rpc_undo_stroke.rpc(hit["point"], float(hit.get("radius", 2.0)))
		set_status("Undid your friend's last dig.")
	else:
		set_status("Nothing from your friend to undo right now.")

# ------------------------------------------------------------------ peers --

func _on_peer_connected(id: int) -> void:
	if not hosting:
		return
	friend_count = maxi(friend_count + 1, multiplayer.get_peers().size())
	set_status("Friend joined your drum.")
	if world and world.terrain and world.terrain.has_method("set_actor_pos"):
		var aid: int = int(world.terrain.net_actor_id(id))
		var th: float = float(world.spawn.get("theta", 0.0)) + 0.02
		var zz: float = float(world.spawn.get("z", 0.0)) + 8.0
		world.terrain.set_actor_pos(aid, th, zz)
	_send_bootstrap(id)
	_rpc_perms.rpc_id(id, perms)
	_spawn_remote(id)
	_refresh_banner()

func _on_peer_disconnected(id: int) -> void:
	_despawn_remote(id)
	friend_count = maxi(multiplayer.get_peers().size(), 0)
	if hosting and world and world.terrain and world.terrain.has_method("clear_actor"):
		var aid: int = int(world.terrain.net_actor_id(id))
		world.terrain.clear_actor(aid)
	set_status("Friend left.")
	_refresh_banner()

func _on_connected_to_server() -> void:
	joined = true
	hosting = false
	actor_id = local_actor()
	set_status("Connected — loading their world…")

func _on_connection_failed() -> void:
	joined = false
	awaiting_bootstrap = false
	multiplayer.multiplayer_peer = null
	peer = null
	set_status("Couldn't reach them. Same Wi‑Fi? Or ask them to Host again and send a fresh invite.")

func _on_server_disconnected() -> void:
	last_disconnect_reason = "Host went offline."
	joined = false
	awaiting_bootstrap = false
	_clear_remotes()
	multiplayer.multiplayer_peer = null
	peer = null
	set_status("%s Hit Reconnect if they come back." % last_disconnect_reason)

func _send_bootstrap(peer_id: int) -> void:
	if world == null or world.terrain == null:
		return
	var strokes: PackedByteArray = world.terrain.save_strokes()
	var soil: PackedByteArray = world.terrain.save_soil()
	var wood: PackedByteArray = world.terrain.save_woodscape()
	var dwell: PackedByteArray = world.terrain.save_dwellings()
	var seed_i := int(world.P.get("seed", 0))
	var clock_f := float(world.clock)
	var spawn_th := float(world.spawn.get("theta", 0.0))
	var spawn_z := float(world.spawn.get("z", 0.0))
	_rpc_bootstrap.rpc_id(
		peer_id,
		schema(),
		seed_i,
		strokes,
		soil,
		wood,
		dwell,
		clock_f,
		spawn_th,
		spawn_z,
		JSON.stringify({"modules": _module_payload()}),
	)

func _module_payload() -> Array:
	var out: Array = []
	if world == null:
		return out
	for m in world.modules:
		if m == null or not is_instance_valid(m):
			continue
		if not m.has_meta("kind"):
			continue
		out.append({
			"kind": int(m.get_meta("kind")),
			"theta": float(m.get_meta("theta")),
			"z": float(m.get_meta("z")),
		})
	return out

@rpc("authority", "call_remote", "reliable")
func _rpc_bootstrap(
	schema_v: int,
	seed_i: int,
	strokes: PackedByteArray,
	soil: PackedByteArray,
	wood: PackedByteArray,
	dwell: PackedByteArray,
	clock_f: float,
	spawn_th: float,
	spawn_z: float,
	_modules_json: String,
) -> void:
	if schema_v != schema():
		set_status("Different game builds — both of you need the same version.")
		leave_session()
		return
	awaiting_bootstrap = false
	if seed_i != int(world.P.get("seed", 0)):
		set_status("World seed mismatch — both need the same game build.")
	if not world.terrain.load_strokes(strokes):
		set_status("Couldn't load their digs.")
		return
	if soil.size() > 0:
		world.terrain.load_soil(soil)
	if wood.size() > 0:
		world.terrain.load_woodscape(wood)
	if dwell.size() > 0:
		world.terrain.load_dwellings(dwell)
	world.clock = clock_f
	if world.player:
		world.player.theta = spawn_th + 0.02
		world.player.z = spawn_z + 8.0
		world.player.r = float(world.spawn.get("radius", world.P["radius"])) - 0.2
		var s: Dictionary = world.terrain.find_spawn()
		var sr: float = float(s.get("radius", world.P["radius"]))
		var pt := Vector3(sr * cos(world.player.theta), sr * sin(world.player.theta), world.player.z)
		world.rebuild_around(pt, 80.0)
	world.refresh_woodscape(true)
	world.force_plant_refresh()
	world.schedule_stockpile_refresh()
	_spawn_remote(1)
	if world.terrain.has_method("set_actor_pos"):
		world.terrain.set_actor_pos(local_actor(), world.player.theta, world.player.z)
	set_status("You're in their world. Digging is on unless they turn it off.")

@rpc("any_peer", "call_remote", "unreliable_ordered")
func _rpc_pose(aid: int, theta: float, z: float, yaw: float, _pitch: float, r: float) -> void:
	var from := multiplayer.get_remote_sender_id()
	if from == multiplayer.get_unique_id():
		return
	var node: Node3D = remotes.get(from)
	if node == null:
		_spawn_remote(from)
		node = remotes.get(from)
	if node == null:
		return
	var surf: float = r
	if world.terrain and world.terrain.has_method("surface_radius"):
		surf = float(world.terrain.surface_radius(theta, z))
	node.position = Vector3(surf * cos(theta), surf * sin(theta), z)
	var up := Vector3(cos(theta), sin(theta), 0.0).normalized()
	var east := Vector3(-sin(theta), cos(theta), 0.0)
	var forward := (-east * sin(yaw) + Vector3(0, 0, 1) * cos(yaw)).normalized()
	node.basis = Basis(east, up, forward).orthonormalized()
	node.set_meta("theta", theta)
	node.set_meta("z", z)
	if is_authority() and world.terrain and world.terrain.has_method("set_actor_pos"):
		world.terrain.set_actor_pos(aid, theta, z)

@rpc("any_peer", "call_remote", "reliable")
func _rpc_dig_intent(
	aid: int, p: Vector3, radius: float, snap: float, level: bool, remove: bool
) -> void:
	if not hosting:
		return
	var from := multiplayer.get_remote_sender_id()
	aid = int(world.terrain.net_actor_id(from))
	if not bool(perms.get("dig", false)):
		_rpc_deny.rpc_id(from, "digging")
		return
	if remove:
		var y: Dictionary = world.terrain.dig_as(aid, p, radius, snap, level, true)
		if y.get("ok", false) and not bool(y.get("vegetation", false)):
			world.terrain.notify_dig(p, radius)
			world.schedule_pool_refresh()
			world.rebuild_around(p, radius * (2.0 if level else 1.0) + 2.0)
		elif y.get("ok", false):
			world.refresh_woodscape(true)
		world.schedule_stockpile_refresh()
	else:
		world.terrain.fill_as(aid, p, radius, snap, level)
		world.rebuild_around(p, radius * (2.0 if level else 1.0) + 2.0)
	_flush_host_events()

@rpc("any_peer", "call_remote", "reliable")
func _rpc_harvest_intent(_aid: int, theta: float, z: float, radius: float) -> void:
	if not hosting:
		return
	if not bool(perms.get("work", false)):
		return
	var from := multiplayer.get_remote_sender_id()
	var aid: int = int(world.terrain.net_actor_id(from))
	if world.terrain.has_method("harvest_near_as"):
		world.terrain.harvest_near_as(aid, theta, z, radius)
	else:
		world.terrain.harvest_near(theta, z, radius)
	world.force_plant_refresh()
	_flush_host_events()

@rpc("any_peer", "call_remote", "reliable")
func _rpc_veg_intent(_aid: int, p: Vector3, as_leaf: bool) -> void:
	if not hosting:
		return
	if not (bool(perms.get("build", false)) or bool(perms.get("dig", false))):
		return
	var pv: Dictionary = world.terrain.place_veg_block(p, as_leaf)
	if pv.get("ok", false):
		world.refresh_woodscape(true)
		_rpc_veg_echo.rpc(p, as_leaf, true)
	_flush_host_events()

@rpc("any_peer", "call_remote", "reliable")
func _rpc_craft_intent(_aid: int, index: int, scale: float, theta: float, z: float) -> void:
	if not hosting:
		return
	if not bool(perms.get("work", false)):
		return
	var from := multiplayer.get_remote_sender_id()
	var aid: int = int(world.terrain.net_actor_id(from))
	if world.terrain.has_method("craft_as"):
		world.terrain.craft_as(aid, index, scale, theta, z)
	_flush_host_events()

@rpc("authority", "call_remote", "reliable")
func _rpc_events(evs: Array) -> void:
	if hosting:
		return
	for ev in evs:
		if typeof(ev) != TYPE_DICTIONARY:
			continue
		_apply_event(ev)

@rpc("authority", "call_remote", "reliable")
func _rpc_perms(p: Dictionary) -> void:
	perms = p
	set_status("Host updated what you can do.")

@rpc("authority", "call_remote", "reliable")
func _rpc_deny(what: String) -> void:
	set_status("Not allowed: %s" % what)

@rpc("authority", "call_remote", "reliable")
func _rpc_undo_stroke(point: Vector3, radius: float) -> void:
	if hosting:
		return
	var hit: Dictionary = world.terrain.undo_dig()
	if hit.get("ok", false):
		world.rebuild_around(hit["point"], float(hit.get("radius", radius)) + 2.0)
	else:
		world.rebuild_around(point, radius + 2.0)

@rpc("authority", "call_remote", "reliable")
func _rpc_veg_echo(p: Vector3, as_leaf: bool, _ok: bool) -> void:
	if hosting:
		return
	world.terrain.place_veg_block(p, as_leaf)
	world.refresh_woodscape(true)

func _apply_event(ev: Dictionary) -> void:
	var kind: String = str(ev.get("kind", ""))
	match kind:
		"stroke":
			var dig: bool = bool(ev.get("dig", true))
			var level: bool = bool(ev.get("level", false))
			var r: float = float(ev.get("radius", 1.0))
			var c := Vector3(float(ev.cx), float(ev.cy), float(ev.cz))
			if world.terrain.has_method("append_stroke"):
				world.terrain.append_stroke(
					ev.cx, ev.cy, ev.cz, ev.radius, dig, level, ev.ux, ev.uy, ev.uz)
			world.rebuild_around(c, r * (2.0 if level else 1.0) + 2.0)
		"inventory":
			if int(ev.get("actor", 0)) == local_actor():
				_last_inv_snap = ev
		"woodscape":
			world.refresh_woodscape(true)
		"heaps":
			world.schedule_stockpile_refresh()
		_:
			pass

func inventory_snapshot() -> Dictionary:
	if not _last_inv_snap.is_empty() and int(_last_inv_snap.get("actor", -1)) == local_actor():
		return _last_inv_snap
	if world and world.terrain:
		if world.terrain.has_method("inventory_for"):
			return world.terrain.inventory_for(local_actor())
		return world.terrain.inventory()
	return {}

func _spawn_remote(peer_id: int) -> void:
	if remotes.has(peer_id) or world == null:
		return
	if peer_id == multiplayer.get_unique_id():
		return
	var body := Node3D.new()
	body.name = "Remote_%d" % peer_id
	world.add_child(body)
	var spec: Dictionary = RamaBody.vary(RamaBody.make("otter"), peer_id)
	RamaBody.build(body, spec)
	var th: float = float(world.spawn.get("theta", 0.0))
	var zz: float = float(world.spawn.get("z", 0.0))
	var sr: float = float(world.spawn.get("radius", world.P["radius"]))
	body.position = Vector3(sr * cos(th), sr * sin(th), zz)
	remotes[peer_id] = body

func _despawn_remote(peer_id: int) -> void:
	var node: Node = remotes.get(peer_id)
	if node and is_instance_valid(node):
		node.queue_free()
	remotes.erase(peer_id)

func _clear_remotes() -> void:
	for id in remotes.keys():
		_despawn_remote(id)
	remotes.clear()
