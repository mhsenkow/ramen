//! Naive surface nets. Smooth SDF surfaces, not cubes (REQUIREMENTS.md A4).
//! Chunks are FLAT Euclidean grids placed by rotation — the curvature of the
//! drum is below terrain noise at chunk scale (EM_BRIEF.md §4 / A3), so nothing
//! here knows it is on a cylinder.

pub struct Mesh {
    pub verts: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Baked ambient occlusion, 0 = fully enclosed, 1 = open sky. Rides in the
    /// vertex colour's alpha channel. Flat-shaded terrain without contact
    /// darkening reads as clay; this is what puts it on the ground.
    pub ao: Vec<f32>,
    pub indices: Vec<i32>,
}

impl Mesh {
    pub fn empty() -> Self { Self { verts: vec![], normals: vec![], ao: vec![], indices: vec![] } }
    pub fn is_empty(&self) -> bool { self.indices.is_empty() }
}

/// Surface nets over an arbitrary sample lattice.
///
/// `pos(i,j,k)` gives the world position of a lattice point, so chunks can
/// share ONE GLOBAL lattice instead of each rotating its own flat grid. That is
/// what makes neighbouring chunks watertight: identical samples produce
/// identical boundary vertices.
///
/// `emit` is the half-open range of grid indices (in the two TILING axes,
/// i = tangential and k = axial) whose quads this chunk owns.
/// Padding lies outside it, so every quad is emitted exactly once across the
/// whole world — no cracks, no duplicated coplanar geometry.
pub fn surface_nets_lattice<P, F>(
    nx: usize, ny: usize, nz: usize,
    emit: (usize, usize),
    pos: P,
    density: F,
) -> Mesh
where P: Fn(usize, usize, usize) -> [f32; 3], F: Fn([f32; 3]) -> f32 {
    let (sx, sy, sz) = (nx + 1, ny + 1, nz + 1);

    let mut d = vec![0.0f32; sx * sy * sz];
    let si = |i: usize, j: usize, k: usize| i + sx * (j + sy * k);
    for k in 0..sz { for j in 0..sy { for i in 0..sx {
        d[si(i, j, k)] = density(pos(i, j, k));
    }}}

    const CORNERS: [[usize; 3]; 8] = [
        [0,0,0],[1,0,0],[0,1,0],[1,1,0],[0,0,1],[1,0,1],[0,1,1],[1,1,1]];
    const EDGES: [[usize; 2]; 12] = [
        [0,1],[2,3],[4,5],[6,7], [0,2],[1,3],[4,6],[5,7], [0,4],[1,5],[2,6],[3,7]];

    // Sixteen directions on a sphere; flipped into the surface hemisphere at
    // use. Two radii give both crevice contact and broad valley shading.
    const AO_DIRS: [[f32; 3]; 12] = [
        [ 0.850,  0.526,  0.000], [-0.850,  0.526,  0.000],
        [ 0.850, -0.526,  0.000], [-0.850, -0.526,  0.000],
        [ 0.000,  0.850,  0.526], [ 0.000, -0.850,  0.526],
        [ 0.000,  0.850, -0.526], [ 0.000, -0.850, -0.526],
        [ 0.526,  0.000,  0.850], [ 0.526,  0.000, -0.850],
        [-0.526,  0.000,  0.850], [-0.526,  0.000, -0.850],
    ];
    const AO_R: [f32; 2] = [1.6, 4.8];

    let mut cell_vert = vec![-1i32; nx * ny * nz];
    let ci = |i: usize, j: usize, k: usize| i + nx * (j + ny * k);
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut ao: Vec<f32> = Vec::new();

    for k in 0..nz { for j in 0..ny { for i in 0..nx {
        let mut cv = [0.0f32; 8];
        let mut neg = 0;
        for (c, off) in CORNERS.iter().enumerate() {
            let v = d[si(i + off[0], j + off[1], k + off[2])];
            cv[c] = v;
            if v < 0.0 { neg += 1; }
        }
        if neg == 0 || neg == 8 { continue; }

        let (mut acc, mut n) = ([0.0f32; 3], 0.0f32);
        for e in EDGES.iter() {
            let (a, b) = (cv[e[0]], cv[e[1]]);
            if (a < 0.0) == (b < 0.0) { continue; }
            let t = (a / (a - b)).clamp(0.0, 1.0);
            let (ca, cb) = (CORNERS[e[0]], CORNERS[e[1]]);
            let pa = pos(i + ca[0], j + ca[1], k + ca[2]);
            let pb = pos(i + cb[0], j + cb[1], k + cb[2]);
            for x in 0..3 { acc[x] += pa[x] + (pb[x] - pa[x]) * t; }
            n += 1.0;
        }
        if n == 0.0 { continue; }
        let v = [acc[0]/n, acc[1]/n, acc[2]/n];
        let h = 0.6f32;
        let g = [
            density([v[0]+h, v[1], v[2]]) - density([v[0]-h, v[1], v[2]]),
            density([v[0], v[1]+h, v[2]]) - density([v[0], v[1]-h, v[2]]),
            density([v[0], v[1], v[2]+h]) - density([v[0], v[1], v[2]-h]),
        ];
        let m = (g[0]*g[0] + g[1]*g[1] + g[2]*g[2]).sqrt().max(1e-6);
        let nrm = [-g[0]/m, -g[1]/m, -g[2]/m];

        // Ambient occlusion: how much of the hemisphere above this point is
        // rock. Twelve directions, two radii, biased to the outward normal.
        let mut open = 0.0f32;
        let mut total = 0.0f32;
        for dir in AO_DIRS.iter() {
            let s = if dir[0]*nrm[0] + dir[1]*nrm[1] + dir[2]*nrm[2] < 0.0 { -1.0 } else { 1.0 };
            let d3 = [dir[0]*s, dir[1]*s, dir[2]*s];
            for (ri, rad) in AO_R.iter().enumerate() {
                // Nudge along the normal so we never sample our own surface.
                let q = [
                    v[0] + nrm[0] * 0.35 + d3[0] * rad,
                    v[1] + nrm[1] * 0.35 + d3[1] * rad,
                    v[2] + nrm[2] * 0.35 + d3[2] * rad,
                ];
                // Near samples weigh more: contact shadow beats broad shading.
                let w = if ri == 0 { 1.6 } else { 1.0 };
                total += w;
                if density(q) < 0.0 { open += w; }
            }
        }
        let mut a = if total > 0.0 { open / total } else { 1.0 };
        a = a.clamp(0.0, 1.0).powf(0.85);

        cell_vert[ci(i, j, k)] = verts.len() as i32;
        verts.push(v);
        normals.push(nrm);
        ao.push(a);
    }}}

    let mut indices: Vec<i32> = Vec::new();
    let quad = |a: i32, b: i32, c: i32, d2: i32, flip: bool, out: &mut Vec<i32>| {
        if a < 0 || b < 0 || c < 0 || d2 < 0 { return; }
        if flip { out.extend_from_slice(&[a, b, c, a, c, d2]); }
        else    { out.extend_from_slice(&[a, c, b, a, d2, c]); }
    };

    let (e0, e1) = emit;
    for k in 0..sz { for j in 0..sy { for i in 0..sx {
        let v0 = d[si(i, j, k)];
        let owned = i >= e0 && i < e1 && k >= e0 && k < e1;
        if owned && i + 1 < sx && j > 0 && k > 0 && j < ny && k < nz {
            let v1 = d[si(i+1, j, k)];
            if (v0 < 0.0) != (v1 < 0.0) {
                quad(cell_vert[ci(i, j-1, k-1)], cell_vert[ci(i, j, k-1)],
                     cell_vert[ci(i, j, k)],     cell_vert[ci(i, j-1, k)],
                     v0 < 0.0, &mut indices);
            }
        }
        if owned && j + 1 < sy && i > 0 && k > 0 && i < nx && k < nz {
            let v1 = d[si(i, j+1, k)];
            if (v0 < 0.0) != (v1 < 0.0) {
                quad(cell_vert[ci(i-1, j, k-1)], cell_vert[ci(i, j, k-1)],
                     cell_vert[ci(i, j, k)],     cell_vert[ci(i-1, j, k)],
                     v0 >= 0.0, &mut indices);
            }
        }
        if owned && k + 1 < sz && i > 0 && j > 0 && i < nx && j < ny {
            let v1 = d[si(i, j, k+1)];
            if (v0 < 0.0) != (v1 < 0.0) {
                quad(cell_vert[ci(i-1, j-1, k)], cell_vert[ci(i, j-1, k)],
                     cell_vert[ci(i, j, k)],     cell_vert[ci(i-1, j, k)],
                     v0 < 0.0, &mut indices);
            }
        }
    }}}

    Mesh { verts, normals, ao, indices }
}

/// Split into per-triangle vertices with face normals. Costs 3x the vertices
/// and buys the faceted low-poly read that suits the diorama camera — and
/// which is what makes a smooth SDF world look hand-cut rather than melted.
pub fn flat_shade(m: Mesh) -> Mesh {
    let mut out = Mesh { verts: Vec::with_capacity(m.indices.len()),
                         normals: Vec::with_capacity(m.indices.len()),
                         ao: Vec::with_capacity(m.indices.len()),
                         indices: Vec::with_capacity(m.indices.len()) };
    for tri in m.indices.chunks_exact(3) {
        let a = m.verts[tri[0] as usize];
        let b = m.verts[tri[1] as usize];
        let c = m.verts[tri[2] as usize];
        let e1 = [b[0]-a[0], b[1]-a[1], b[2]-a[2]];
        let e2 = [c[0]-a[0], c[1]-a[1], c[2]-a[2]];
        let n = [e1[1]*e2[2] - e1[2]*e2[1],
                 e1[2]*e2[0] - e1[0]*e2[2],
                 e1[0]*e2[1] - e1[1]*e2[0]];
        let mag = (n[0]*n[0] + n[1]*n[1] + n[2]*n[2]).sqrt().max(1e-9);
        let mut n = [n[0]/mag, n[1]/mag, n[2]/mag];
        // Winding alone does not tell us which side is "out". Take the sign
        // from the analytic gradient normal, which does.
        let o = m.normals[tri[0] as usize];
        if n[0]*o[0] + n[1]*o[1] + n[2]*o[2] < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        let base = out.verts.len() as i32;
        out.verts.extend_from_slice(&[a, b, c]);
        out.normals.extend_from_slice(&[n, n, n]);
        if m.ao.len() == m.verts.len() {
            out.ao.extend_from_slice(&[
                m.ao[tri[0] as usize], m.ao[tri[1] as usize], m.ao[tri[2] as usize]]);
        } else {
            out.ao.extend_from_slice(&[1.0, 1.0, 1.0]);
        }
        out.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    out
}
