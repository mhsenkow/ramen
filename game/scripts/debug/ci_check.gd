extends SceneTree
## Headless load gate: every script compiles, every shader compiles, every scene
## resolves, and the native extension is present. Run by `tools/check_godot.py`.
##
## Godot --headless --path game --script res://scripts/debug/ci_check.gd
##
## ## Why this is not wired into world.gd
##
## Deliberately a `SceneTree` script rather than a `--flag` on the main scene.
## It must not boot `main.tscn`: generating a 900 x 6000 m drum takes minutes and
## `--selftest` has been observed hanging indefinitely, so a gate that booted the
## game could never be trusted to fail fast. This runs in under a second and
## needs no world.
##
## ## Why `load() == null` is NOT the test
##
## Godot returns a usable-looking object for a file it failed to compile. A
## broken script and a broken shader both come back non-null — a null check
## looks like a gate and catches nothing. So:
##
## * scripts  — `reload()` returns an error code (43 = parse error, 0 = OK)
## * shaders  — no in-engine predicate exists; a valid shader may legitimately
##              declare zero uniforms, so uniform count proves nothing. Loading
##              one *does* make Godot print `SHADER ERROR`, and the Python
##              wrapper fails the gate on that output. Hence the wrapper: it is
##              load-bearing for shaders, not decoration.

const SCRIPT_DIRS := ["res://scripts"]
const SHADER_DIRS := ["res://shaders"]
const SCENES := ["res://main.tscn", "res://probe.tscn"]
## Classes the GDExtension must register. A stale or missing librama_sim leaves
## every one of these absent while scripts still compile fine.
const NATIVE_CLASSES := ["RamaTerrain"]

var _problems: Array[String] = []
var _self_path := ""

func _init() -> void:
	# reload() on the script currently executing fails with ERR_BUSY; skip it.
	# It has demonstrably compiled — it is running.
	var own = get_script()
	if own != null:
		_self_path = (own as Script).resource_path
	var scripts := _collect(SCRIPT_DIRS, ".gd")
	var shaders := _collect(SHADER_DIRS, [".gdshader", ".gdshaderinc"])
	print("ci_check: %d scripts, %d shaders, %d scenes" % [
			scripts.size(), shaders.size(), SCENES.size()])

	for path in scripts:
		_check_script(path)
	# Loading a shader is what triggers compilation, and therefore what makes
	# Godot emit SHADER ERROR for the wrapper to catch.
	for path in shaders:
		_check_shader(path)
	for path in SCENES:
		_check_scene(path)
	_check_native()

	if _problems.is_empty():
		print("\nci_check OK — %d scripts, %d shaders, %d scenes, extension present"
				% [scripts.size(), shaders.size(), SCENES.size()])
		quit(0)
		return
	print("\nci_check: %d problem(s) reported" % _problems.size())
	for p in _problems:
		print("  - ", p)
	quit(1)

func _fail(msg: String) -> void:
	_problems.append(msg)
	# Prefix the wrapper greps for. Distinct from the informational
	# "ci_check:" line, or the gate flags its own progress report as a fault.
	printerr("ci_check FAIL: %s" % msg)

## Recurse a res:// directory. `suffix` is a String or an Array of them.
func _collect(roots: Array, suffix) -> Array[String]:
	var wanted: Array = suffix if suffix is Array else [suffix]
	var out: Array[String] = []
	var queue: Array = roots.duplicate()
	while not queue.is_empty():
		var dir_path: String = queue.pop_back()
		var dir := DirAccess.open(dir_path)
		if dir == null:
			_fail("cannot open directory %s" % dir_path)
			continue
		dir.list_dir_begin()
		var name := dir.get_next()
		while name != "":
			if name.begins_with("."):
				name = dir.get_next()
				continue
			var full: String = dir_path.path_join(name)
			if dir.current_is_dir():
				queue.append(full)
			else:
				for w in wanted:
					if name.ends_with(w):
						out.append(full)
						break
			name = dir.get_next()
		dir.list_dir_end()
	out.sort()
	return out

func _check_script(path: String) -> void:
	if path == _self_path:
		return
	var res = load(path)
	if res == null:
		_fail("%s did not load" % path)
		return
	if not (res is GDScript):
		_fail("%s loaded as %s, not GDScript" % [path, res.get_class()])
		return
	# The real test. `load()` hands back an object even for a parse error.
	var err: int = (res as GDScript).reload()
	if err != OK:
		_fail("%s failed to compile (reload error %d)" % [path, err])

func _check_shader(path: String) -> void:
	if path.ends_with(".gdshaderinc"):
		# Includes are compiled through their includers; loading one alone is
		# not meaningful. Presence is all we assert.
		if not ResourceLoader.exists(path):
			_fail("%s missing" % path)
		return
	var res = load(path)
	if res == null:
		_fail("%s did not load" % path)
		return
	if not (res is Shader):
		_fail("%s loaded as %s, not Shader" % [path, res.get_class()])
		return
	var shader := res as Shader
	if shader.code.strip_edges().is_empty():
		_fail("%s has empty code" % path)
		return
	# This call is the whole point: loading a Shader does not compile it, so a
	# syntax error stays silent until something queries it. Reading `.code`
	# is not enough — the uniform list is. The count is not the assertion (a
	# valid shader may declare none); provoking SHADER ERROR is.
	shader.get_shader_uniform_list()

func _check_scene(path: String) -> void:
	if not ResourceLoader.exists(path):
		_fail("%s missing" % path)
		return
	var res = load(path)
	if res == null or not (res is PackedScene):
		_fail("%s did not load as a PackedScene" % path)
		return
	# Never instantiate: that runs _ready and boots world generation.
	if not (res as PackedScene).can_instantiate():
		_fail("%s cannot be instantiated (broken node or resource path)" % path)

func _check_native() -> void:
	for cls in NATIVE_CLASSES:
		if not ClassDB.class_exists(cls):
			_fail("GDExtension class %s not registered — librama_sim missing or stale" % cls)
