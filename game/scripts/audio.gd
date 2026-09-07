extends Node
## Procedural sound. No assets to ship, and the excavation click can be tied
## directly to brush size, which a fixed sample cannot do.

var players: Array[AudioStreamPlayer] = []
var next := 0
var dig_s: AudioStreamWAV
var place_s: AudioStreamWAV
var undo_s: AudioStreamWAV
var step_s: AudioStreamWAV
var water_s: AudioStreamWAV
var splash_s: AudioStreamWAV
var water_p: AudioStreamPlayer
var step_t := 0.0

func _ready() -> void:
	for i in 8:
		var p := AudioStreamPlayer.new()
		add_child(p)
		players.append(p)
	dig_s = _noise_burst(0.20, 0.55, 2200.0, 0.9)
	place_s = _thunk(0.28, 150.0, 60.0)
	undo_s = _thunk(0.16, 420.0, 700.0)
	step_s = _noise_burst(0.09, 0.22, 900.0, 0.5)
	water_s = _water_loop(2.4)
	splash_s = _noise_burst(0.18, 0.35, 3400.0, 0.7)
	water_p = AudioStreamPlayer.new()
	water_p.stream = water_s
	water_p.volume_db = -80.0
	add_child(water_p)
	water_p.play()

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

func _water_loop(dur: float) -> AudioStreamWAV:
	var rate := 22050
	var n := int(dur * rate)
	var d := PackedFloat32Array()
	d.resize(n)
	var rng := RandomNumberGenerator.new()
	rng.seed = 4242
	var lp := 0.0
	for i in n:
		var t := float(i) / float(rate)
		lp += (rng.randf_range(-1.0, 1.0) - lp) * 0.08
		var wave := sin(t * 18.0) * 0.08 + sin(t * 41.0 + 1.2) * 0.05
		d[i] = (lp * 0.55 + wave) * 0.35
	var w := _wav(d, rate)
	w.loop_mode = AudioStreamWAV.LOOP_FORWARD
	w.loop_begin = 0
	w.loop_end = n
	return w

func play(s: AudioStreamWAV, pitch := 1.0, vol_db := -6.0) -> void:
	if s == null:
		return
	var p := players[next]
	next = (next + 1) % players.size()
	p.stream = s
	p.pitch_scale = pitch
	p.volume_db = vol_db
	p.play()

func dig(brush: float, hardness := 1.0) -> void:
	var pitch := clampf(1.55 - brush * 0.10 - hardness * 0.08, 0.45, 1.5)
	play(dig_s, pitch, -7.0)

func place() -> void: play(place_s, randf_range(0.94, 1.06), -5.0)
func undo() -> void: play(undo_s, 1.0, -12.0)

func water_ambience(depth_m: float, flux: float) -> void:
	if water_p == null:
		return
	var wet := clampf(depth_m * 0.55 + flux * 0.35, 0.0, 1.0)
	var target := lerpf(-52.0, -14.0, wet)
	water_p.volume_db = lerpf(water_p.volume_db, target, 0.08)
	water_p.pitch_scale = lerpf(0.85, 1.15, clampf(flux * 0.6 + depth_m * 0.15, 0.0, 1.0))

func splash(strength := 1.0) -> void:
	play(splash_s, randf_range(0.9, 1.25), -10.0 + strength * 2.0)

func footstep(dt: float, speed: float) -> void:
	if speed < 0.5:
		step_t = 0.0
		return
	step_t += dt * speed
	if step_t > 2.6:
		step_t = 0.0
		play(step_s, randf_range(0.85, 1.15), -20.0)
