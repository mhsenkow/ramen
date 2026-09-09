extends RefCounted
## Display cache of the native wood/leaf construction recipe. All distances use
## this same silhouette; nearby material is rasterized from these exact volumes.
const TREE_H := 14.0
const PIGMENTS := [
	Color(0.22, 0.36, 0.27), Color(0.34, 0.48, 0.23),
	Color(0.34, 0.46, 0.29), Color(0.42, 0.44, 0.26),
	Color(0.40, 0.48, 0.26), Color(0.35, 0.48, 0.24),
	Color(0.40, 0.44, 0.25), Color(0.24, 0.40, 0.29),
]

static func from_materials(parts: PackedFloat32Array) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	# Wood is rendered with the same neutral luminance key as the material grid.
	for i in int(parts.size() / 7.0):
		var b := i * 7
		var c := Vector3(parts[b], parts[b+1], parts[b+2])
		var half := Vector3(parts[b+3], parts[b+4], parts[b+5]) * 0.5
		var wood := int(parts[b+6]) == 1
		var corners := [Vector3(-1,-1,-1),Vector3(1,-1,-1),Vector3(1,1,-1),Vector3(-1,1,-1),
			Vector3(-1,-1,1),Vector3(1,-1,1),Vector3(1,1,1),Vector3(-1,1,1)]
		for idx in [0,1,2,0,2,3,5,4,7,5,7,6,4,0,3,4,3,7,1,5,6,1,6,2,3,2,6,3,6,7,4,5,1,4,1,0]:
			st.set_color(Color(1,1,1,0.0 if wood else 1.0))
			st.add_vertex(c + corners[idx] * half)
	st.generate_normals()
	st.index()
	return st.commit()
