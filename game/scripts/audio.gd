extends Node
## Procedural sound. No assets to ship (§2131–2160).
## Toy DSP, not physical modelling (§2160).

const RamaControls = preload("res://scripts/controls.gd")

const BUS_MASTER := "Master"
const BUS_WORLD := "World"
const BUS_UI := "UI"
const BUS_MUSIC := "Music"
const BUS_VOICE := "Voice"
const VOICE_CAP := 8

var players: Array[AudioStreamPlayer] = []
var dig_s: AudioStreamWAV
var place_s: AudioStreamWAV
var undo_s: AudioStreamWAV
var step_dry: AudioStreamWAV
var step_wet: AudioStreamWAV
var step_stone: AudioStreamWAV
var splash_s: AudioStreamWAV
var water_beds: Dictionary = {}  # name -> AudioStreamPlayer
var step_t := 0.0
var muted_by_focus := false
var reverb: AudioEffectReverb
var caption_cb: Callable = Callable()
var last_caption := ""

func _ready() -> void:
	_ensure_buses()
	_apply_volumes()
	for i in VOICE_CAP:
		var p := AudioStreamPlayer.new()
		p.bus = BUS_WORLD
		add_child(p)
		players.append(p)
	dig_s = _noise_burst(0.20, 0.55, 2200.0, 0.9)
	place_s = _thunk(0.28, 150.0, 60.0)
	undo_s = _thunk(0.16, 420.0, 700.0)
	step_dry = _noise_burst(0.09, 0.22, 900.0, 0.5)
	step_wet = _noise_burst(0.11, 0.28, 1400.0, 0.45)
	step_stone = _noise_burst(0.07, 0.14, 2400.0, 0.55)
	splash_s = _noise_burst(0.18, 0.35, 3400.0, 0.7)
	# Distinct water beds cross-faded by depth/flux (§2140).
	_add_water_bed("riffle", _water_loop(2.1, 0.12, 28.0, 0.40))
	_add_water_bed("pool", _water_loop(2.8, 0.05, 14.0, 0.28))
	_add_water_bed("cascade", _water_loop(1.8, 0.18, 55.0, 0.55))
	_add_water_bed("fall", _water_loop(2.0, 0.22, 90.0, 0.70))
	get_window().focus_exited.connect(_on_focus_lost)
	get_window().focus_entered.connect(_on_focus_gained)

func _add_water_bed(bed_name: String, stream: AudioStreamWAV) -> void:
	var p := AudioStreamPlayer.new()
	p.stream = stream
	p.bus = BUS_WORLD
	p.volume_db = -80.0
	add_child(p)
	p.play()
	water_beds[bed_name] = p

func _ensure_buses() -> void:
	for bus_name in [BUS_WORLD, BUS_UI, BUS_MUSIC, BUS_VOICE]:
		if AudioServer.get_bus_index(bus_name) < 0:
			AudioServer.add_bus()
			var i := AudioServer.bus_count - 1
			AudioServer.set_bus_name(i, bus_name)
			AudioServer.set_bus_send(i, BUS_MASTER)
	var mi := AudioServer.get_bus_index(BUS_MASTER)
	if mi >= 0:
		AudioServer.set_bus_volume_db(mi, -6.0)
	# Reverb on World — driven per biome (§2145).
	var wi := AudioServer.get_bus_index(BUS_WORLD)
	if wi >= 0 and AudioServer.get_bus_effect_count(wi) == 0:
		reverb = AudioEffectReverb.new()
		reverb.room_size = 0.35
		reverb.damping = 0.55
		reverb.wet = 0.08
		reverb.dry = 0.92
		AudioServer.add_bus_effect(wi, reverb)

func set_quiet(on: bool) -> void:
	## Machinery / world hush so you notice how loud it was (§2152).
	RamaControls.quiet_mode = on
	var wi := AudioServer.get_bus_index(BUS_WORLD)
	if wi >= 0:
		AudioServer.set_bus_mute(wi, on)
	_caption("quiet" if on else "sound returns")

func apply_volumes() -> void:
	_apply_volumes()

func _apply_volumes() -> void:
	_set_bus_linear(BUS_WORLD, RamaControls.vol_world)
	_set_bus_linear(BUS_UI, RamaControls.vol_ui)
	_set_bus_linear(BUS_MUSIC, RamaControls.vol_music)
	_set_bus_linear(BUS_VOICE, RamaControls.vol_voice)

func _set_bus_linear(bus_name: String, lin: float) -> void:
	var i := AudioServer.get_bus_index(bus_name)
	if i < 0:
		return
	var v: float = clampf(lin, 0.0, 1.0)
	AudioServer.set_bus_volume_db(i, linear_to_db(maxf(v, 0.0001)) if v > 0.001 else -80.0)
	AudioServer.set_bus_mute(i, v <= 0.001)

func _on_focus_lost() -> void:
	if not RamaControls.mute_on_focus_loss:
		return
	muted_by_focus = true
	AudioServer.set_bus_mute(AudioServer.get_bus_index(BUS_MASTER), true)

func _on_focus_gained() -> void:
	if muted_by_focus:
		muted_by_focus = false
		AudioServer.set_bus_mute(AudioServer.get_bus_index(BUS_MASTER), false)

func set_biome_reverb(biome_name: String, underground: bool) -> void:
	if reverb == null:
		return
	# Forest deadens, open water carries, cave rings (§2145).
	if underground:
		reverb.room_size = 0.85
		reverb.damping = 0.25
		reverb.wet = 0.32
		reverb.dry = 0.70
	elif biome_name in ["forest", "wetland", "riparian"]:
		reverb.room_size = 0.25
		reverb.damping = 0.80
		reverb.wet = 0.06
		reverb.dry = 0.95
	elif biome_name in ["water"]:
		reverb.room_size = 0.55
		reverb.damping = 0.35
		reverb.wet = 0.18
		reverb.dry = 0.85
	elif biome_name in ["alpine", "bare rock", "bare_rock"]:
		reverb.room_size = 0.70
		reverb.damping = 0.30
		reverb.wet = 0.22
		reverb.dry = 0.80
	else:
		reverb.room_size = 0.35
		reverb.damping = 0.55
		reverb.wet = 0.08
		reverb.dry = 0.92

func _caption(text: String) -> void:
	if not RamaControls.captions:
		return
	last_caption = text
	if caption_cb.is_valid():
		caption_cb.call(text)

func _wav(data: PackedFloat32Array, rate: int) -> AudioStreamWAV:
	var bytes := PackedByteArray()
	bytes.resize(data.size() * 2)
	for i in data.size():
		var v: int = clampi(int(clampf(data[i], -1.0, 1.0) * 32767.0), -32768, 32767)
		bytes.encode_s16(i * 2, v)
	var w := AudioStreamWAV.new()
	w.format = AudioStreamWAV.FORMAT_16_BITS
	w.mix_rate = rate
	w.stereo = false
	w.data = bytes
	return w

func _noise_burst(dur: float, decay: float, cutoff: float, grit: float) -> AudioStreamWAV:
	var rate := 22050
	var n := int(dur * rate)
	var d := PackedFloat32Array()
	d.resize(n)
	var rng := RandomNumberGenerator.new()
	rng.seed = 12345
	var lp := 0.0
	var a: float = clampf(cutoff / float(rate), 0.02, 0.9)
	for i in n:
		var t := float(i) / n
		var env: float = exp(-t / maxf(decay, 0.01)) * (1.0 - t)
		lp += (rng.randf_range(-1.0, 1.0) - lp) * a
		d[i] = lp * env * grit
	return _wav(d, rate)

func _thunk(dur: float, f0: float, f1: float) -> AudioStreamWAV:
	var rate := 22050
	var n := int(dur * rate)
	var d := PackedFloat32Array()
	d.resize(n)
	var phase := 0.0
	for i in n:
		var t := float(i) / n
		var f: float = lerpf(f0, f1, t)
		phase += TAU * f / rate
		var env: float = exp(-t * 6.0)
		d[i] = sin(phase) * env * 0.55
	return _wav(d, rate)

func _water_loop(dur: float, grit: float, swirl: float, level: float) -> AudioStreamWAV:
	var rate := 22050
	var n := int(dur * rate)
	var d := PackedFloat32Array()
	d.resize(n)
	var rng := RandomNumberGenerator.new()
	rng.seed = int(swirl * 100.0) + 4242
	var lp := 0.0
	for i in n:
		var t := float(i) / float(rate)
		lp += (rng.randf_range(-1.0, 1.0) - lp) * grit
		var wave := sin(t * swirl) * 0.08 + sin(t * swirl * 2.2 + 1.2) * 0.05
		d[i] = (lp * 0.55 + wave) * level
	var w := _wav(d, rate)
	w.loop_mode = AudioStreamWAV.LOOP_FORWARD
	w.loop_begin = 0
	w.loop_end = n
	return w

## Voice steal: prefer free slot, else quietest / oldest (§2155).
func play(s: AudioStreamWAV, pitch := 1.0, vol_db := -6.0, priority := 1) -> void:
	if s == null:
		return
	var pick: AudioStreamPlayer = null
	var best_score := -1e9
	for p in players:
		if not p.playing:
			pick = p
			break
		# Steal lowest: quiet + low remaining + low priority tag.
		var score: float = -p.volume_db - float(p.get_meta("prio", 1)) * 4.0 \
				- p.get_playback_position()
		if score > best_score:
			best_score = score
			pick = p
	if pick == null:
		return
	pick.stream = s
	pick.pitch_scale = pitch
	pick.volume_db = vol_db
	pick.set_meta("prio", priority)
	pick.play()

func dig(brush: float, hardness := 1.0) -> void:
	var pitch := clampf(1.55 - brush * 0.10 - hardness * 0.08, 0.45, 1.5)
	play(dig_s, pitch, -7.0, 2)
	_caption("excavating")

func place() -> void:
	play(place_s, randf_range(0.94, 1.06), -5.0, 2)
	_caption("placed")

func undo() -> void:
	play(undo_s, 1.0, -12.0, 1)
	_caption("undone")

func water_ambience(depth_m: float, flux: float, wrap_factor := 1.0) -> void:
	# Cross-fade beds by local hydrology (§2140); wrap_factor = drum corridor (§2144).
	var w: float = clampf(wrap_factor, 0.15, 1.0)
	var pool_w := clampf(depth_m * 0.55, 0.0, 1.0) * (1.0 - clampf(flux * 0.4, 0.0, 0.7))
	var riffle_w := clampf(flux * 0.8 * (1.0 - depth_m * 0.25), 0.0, 1.0)
	var cascade_w := clampf(flux * 1.2 - 0.45, 0.0, 1.0) * clampf(depth_m * 0.4 + 0.3, 0.0, 1.0)
	var fall_w := clampf(flux * 1.5 - 1.0, 0.0, 1.0)
	var weights := {
		"pool": pool_w,
		"riffle": riffle_w,
		"cascade": cascade_w,
		"fall": fall_w,
	}
	var sum := 0.001
	for k in weights:
		sum += float(weights[k])
	for k in water_beds:
		var p: AudioStreamPlayer = water_beds[k]
		var frac: float = float(weights.get(k, 0.0)) / sum
		var target := lerpf(-60.0, -12.0, frac * w)
		p.volume_db = lerpf(p.volume_db, target, 0.08)
		p.pitch_scale = lerpf(0.88, 1.12, clampf(flux * 0.5 + depth_m * 0.12, 0.0, 1.0))

func splash(strength := 1.0) -> void:
	play(splash_s, randf_range(0.9, 1.25), -10.0 + strength * 2.0, 2)
	_caption("splash")

func footstep(dt: float, speed: float, hardness := 1.0, wetness := 0.0) -> void:
	if speed < 0.5:
		step_t = 0.0
		return
	step_t += dt * speed
	if step_t > 2.6:
		step_t = 0.0
		var s: AudioStreamWAV = step_dry
		var pitch := randf_range(0.85, 1.15)
		var vol := -20.0
		var cap := "footstep"
		if wetness > 0.45:
			s = step_wet
			pitch *= lerpf(1.0, 0.82, wetness)
			vol = -16.0
			cap = "wet footstep"
		elif hardness > 1.4:
			s = step_stone
			pitch *= 1.08
			vol = -18.0
			cap = "stone footstep"
		play(s, pitch, vol, 0)
		_caption(cap)
