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
    /// Vertex albedo, filled by the caller between meshing and flat-shading.
    pub cols: Vec<[f32; 3]>,
    pub indices: Vec<i32>,
}

impl Mesh {
    pub fn empty() -> Self {
        Self {
            verts: vec![],
            normals: vec![],
            ao: vec![],
            cols: vec![],
            indices: vec![],
        }
    }
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

const CORNERS: [[usize; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [0, 1, 0],
    [1, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [0, 1, 1],
    [1, 1, 1],
];
const EDGES: [[usize; 2]; 12] = [
    [0, 1],
    [2, 3],
    [4, 5],
    [6, 7],
    [0, 2],
    [1, 3],
    [4, 6],
    [5, 7],
    [0, 4],
    [1, 5],
    [2, 6],
    [3, 7],
];

/// Twelve directions on a sphere; flipped into the surface hemisphere at use.
/// Two radii give both crevice contact and broad valley shading.
const AO_DIRS: [[f32; 3]; 12] = [
    [0.850, 0.526, 0.000],
    [-0.850, 0.526, 0.000],
    [0.850, -0.526, 0.000],
    [-0.850, -0.526, 0.000],
    [0.000, 0.850, 0.526],
    [0.000, -0.850, 0.526],
    [0.000, 0.850, -0.526],
    [0.000, -0.850, -0.526],
    [0.526, 0.000, 0.850],
    [0.526, 0.000, -0.850],
    [-0.526, 0.000, 0.850],
    [-0.526, 0.000, -0.850],
];
/// Near samples weigh more: contact shadow beats broad shading.
const AO_W: [f32; 2] = [1.6, 1.0];

/// A scalar field already sampled on the lattice, plus its dimensions.
///
/// Sampling is the expensive half of meshing and the caller knows far more
/// about the field than the mesher does — which columns are pure rock, which
/// are pure air, which per-column terms can be hoisted. So the caller fills the
/// buffer and the mesher never evaluates anything: everything below (vertex
/// placement, normals, ambient occlusion) is table lookups into `d`.
pub struct Field {
    /// Cell counts. Samples are `(nx+1) * (ny+1) * (nz+1)`.
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub d: Vec<f32>,
}

impl Field {
    pub fn new(nx: usize, ny: usize, nz: usize) -> Self {
        Self {
            nx,
            ny,
            nz,
            d: vec![0.0; (nx + 1) * (ny + 1) * (nz + 1)],
        }
    }
    #[inline]
    pub fn si(&self, i: usize, j: usize, k: usize) -> usize {
        i + (self.nx + 1) * (j + (self.ny + 1) * k)
    }
    #[inline]
    fn at(&self, i: usize, j: usize, k: usize) -> f32 {
        self.d[self.si(i, j, k)]
    }
    /// Trilinear read with clamped indices. Used for ambient-occlusion probes,
    /// which are the only reads that leave the cell they belong to.
    #[inline]
    fn lerp_at(&self, x: f32, y: f32, z: f32) -> f32 {
        let cx = x.clamp(0.0, self.nx as f32);
        let cy = y.clamp(0.0, self.ny as f32);
        let cz = z.clamp(0.0, self.nz as f32);
        let (i, j, k) = (cx.floor(), cy.floor(), cz.floor());
        let (fx, fy, fz) = (cx - i, cy - j, cz - k);
        let (i, j, k) = (i as usize, j as usize, k as usize);
        let (i1, j1, k1) = (
            (i + 1).min(self.nx),
            (j + 1).min(self.ny),
            (k + 1).min(self.nz),
        );
        let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let c00 = l(self.at(i, j, k), self.at(i1, j, k), fx);
        let c10 = l(self.at(i, j1, k), self.at(i1, j1, k), fx);
        let c01 = l(self.at(i, j, k1), self.at(i1, j, k1), fx);
        let c11 = l(self.at(i, j1, k1), self.at(i1, j1, k1), fx);
        l(l(c00, c10, fy), l(c01, c11, fy), fz)
    }
}

/// Surface nets over a pre-sampled lattice.
///
/// `pos(i,j,k)` maps FRACTIONAL lattice coordinates to world space, so chunks
/// share ONE GLOBAL lattice instead of each rotating its own flat grid. That is
/// what makes neighbouring chunks watertight: identical samples produce
/// identical boundary vertices.
///
/// `emit` is the half-open range of grid indices (in the two TILING axes,
/// i = tangential and k = axial) whose quads this chunk owns. Padding lies
/// outside it, so every quad is emitted exactly once across the whole world —
/// no cracks, no duplicated coplanar geometry.
///
/// `ao_cells` are the two occlusion probe radii, in lattice cells.
pub fn surface_nets_field<P>(f: &Field, emit: (usize, usize), pos: P, ao_cells: [f32; 2]) -> Mesh
where
    P: Fn(f32, f32, f32) -> [f32; 3],
{
    let (nx, ny, nz) = (f.nx, f.ny, f.nz);
    let (sx, sy, sz) = (nx + 1, ny + 1, nz + 1);

    let mut cell_vert = vec![-1i32; nx * ny * nz];
    let ci = |i: usize, j: usize, k: usize| i + nx * (j + ny * k);
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut ao: Vec<f32> = Vec::new();

    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let mut cv = [0.0f32; 8];
                let mut neg = 0;
                for (c, off) in CORNERS.iter().enumerate() {
                    let v = f.at(i + off[0], j + off[1], k + off[2]);
                    cv[c] = v;
                    if v < 0.0 {
                        neg += 1;
                    }
                }
                if neg == 0 || neg == 8 {
                    continue;
                }

                // Average the edge crossings in LATTICE space. Mapping the mean
                // through `pos` once, instead of averaging mapped corners, costs
                // one trig call per vertex instead of up to twenty-four — and the
                // sagitta across a 1.4 m cell is 0.3 mm, so the two agree.
                let (mut acc, mut n) = ([0.0f32; 3], 0.0f32);
                for e in EDGES.iter() {
                    let (a, b) = (cv[e[0]], cv[e[1]]);
                    if (a < 0.0) == (b < 0.0) {
                        continue;
                    }
                    let t = (a / (a - b)).clamp(0.0, 1.0);
                    let (ca, cb) = (CORNERS[e[0]], CORNERS[e[1]]);
                    for x in 0..3 {
                        acc[x] += ca[x] as f32 + (cb[x] as f32 - ca[x] as f32) * t;
                    }
                    n += 1.0;
                }
                if n == 0.0 {
                    continue;
                }
                let lat = [
                    i as f32 + acc[0] / n,
                    j as f32 + acc[1] / n,
                    k as f32 + acc[2] / n,
                ];

                // Orientation only: `flat_shade` recomputes the shading normal
                // from the triangle, and this just tells it which side is out.
                // The cell's own corner gradient is exact enough and free.
                let g = [
                    (cv[1] + cv[3] + cv[5] + cv[7]) - (cv[0] + cv[2] + cv[4] + cv[6]),
                    (cv[2] + cv[3] + cv[6] + cv[7]) - (cv[0] + cv[1] + cv[4] + cv[5]),
                    (cv[4] + cv[5] + cv[6] + cv[7]) - (cv[0] + cv[1] + cv[2] + cv[3]),
                ];
                let m = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt().max(1e-6);
                let nrm = [-g[0] / m, -g[1] / m, -g[2] / m];

                // Ambient occlusion: how much of the hemisphere above this point
                // is rock. Twelve directions, two radii, biased to the outward
                // normal — read straight out of the lattice we already have.
                let mut open = 0.0f32;
                let mut total = 0.0f32;
                for dir in AO_DIRS.iter() {
                    let s = if dir[0] * nrm[0] + dir[1] * nrm[1] + dir[2] * nrm[2] < 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    for (ri, rad) in ao_cells.iter().enumerate() {
                        // Nudge along the normal so we never sample our own surface.
                        let q = [
                            lat[0] + nrm[0] * 0.25 + dir[0] * s * rad,
                            lat[1] + nrm[1] * 0.25 + dir[1] * s * rad,
                            lat[2] + nrm[2] * 0.25 + dir[2] * s * rad,
                        ];
                        let w = AO_W[ri];
                        total += w;
                        if f.lerp_at(q[0], q[1], q[2]) < 0.0 {
                            open += w;
                        }
                    }
                }
                let mut a = if total > 0.0 { open / total } else { 1.0 };
                a = a.clamp(0.0, 1.0).powf(0.85);

                cell_vert[ci(i, j, k)] = verts.len() as i32;
                verts.push(pos(lat[0], lat[1], lat[2]));
                normals.push(nrm);
                ao.push(a);
            }
        }
    }

    let mut indices: Vec<i32> = Vec::new();
    let quad = |a: i32, b: i32, c: i32, d2: i32, flip: bool, out: &mut Vec<i32>| {
        if a < 0 || b < 0 || c < 0 || d2 < 0 {
            return;
        }
        if flip {
            out.extend_from_slice(&[a, b, c, a, c, d2]);
        } else {
            out.extend_from_slice(&[a, c, b, a, d2, c]);
        }
    };

    let (e0, e1) = emit;
    for k in 0..sz {
        for j in 0..sy {
            for i in 0..sx {
                let owned = i >= e0 && i < e1 && k >= e0 && k < e1;
                if !owned {
                    continue;
                }
                let v0 = f.at(i, j, k);
                if i + 1 < sx && j > 0 && k > 0 && j < ny && k < nz {
                    let v1 = f.at(i + 1, j, k);
                    if (v0 < 0.0) != (v1 < 0.0) {
                        quad(
                            cell_vert[ci(i, j - 1, k - 1)],
                            cell_vert[ci(i, j, k - 1)],
                            cell_vert[ci(i, j, k)],
                            cell_vert[ci(i, j - 1, k)],
                            v0 < 0.0,
                            &mut indices,
                        );
                    }
                }
                if j + 1 < sy && i > 0 && k > 0 && i < nx && k < nz {
                    let v1 = f.at(i, j + 1, k);
                    if (v0 < 0.0) != (v1 < 0.0) {
                        quad(
                            cell_vert[ci(i - 1, j, k - 1)],
                            cell_vert[ci(i, j, k - 1)],
                            cell_vert[ci(i, j, k)],
                            cell_vert[ci(i - 1, j, k)],
                            v0 >= 0.0,
                            &mut indices,
                        );
                    }
                }
                if k + 1 < sz && i > 0 && j > 0 && i < nx && j < ny {
                    let v1 = f.at(i, j, k + 1);
                    if (v0 < 0.0) != (v1 < 0.0) {
                        quad(
                            cell_vert[ci(i - 1, j - 1, k)],
                            cell_vert[ci(i, j - 1, k)],
                            cell_vert[ci(i, j, k)],
                            cell_vert[ci(i - 1, j, k)],
                            v0 < 0.0,
                            &mut indices,
                        );
                    }
                }
            }
        }
    }

    Mesh {
        verts,
        normals,
        ao,
        cols: Vec::new(),
        indices,
    }
}

/// Split into per-triangle vertices with face normals. Costs 3x the vertices
/// and buys the faceted low-poly read that suits the diorama camera — and
/// which is what makes a smooth SDF world look hand-cut rather than melted.
pub fn flat_shade(m: Mesh) -> Mesh {
    let mut out = Mesh {
        verts: Vec::with_capacity(m.indices.len()),
        normals: Vec::with_capacity(m.indices.len()),
        ao: Vec::with_capacity(m.indices.len()),
        cols: Vec::with_capacity(m.indices.len()),
        indices: Vec::with_capacity(m.indices.len()),
    };
    for tri in m.indices.chunks_exact(3) {
        let a = m.verts[tri[0] as usize];
        let b = m.verts[tri[1] as usize];
        let c = m.verts[tri[2] as usize];
        let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let mag = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
        let mut n = [n[0] / mag, n[1] / mag, n[2] / mag];
        // Winding alone does not tell us which side is "out". Take the sign
        // from the lattice gradient normal, which does.
        let o = m.normals[tri[0] as usize];
        if n[0] * o[0] + n[1] * o[1] + n[2] * o[2] < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        let base = out.verts.len() as i32;
        out.verts.extend_from_slice(&[a, b, c]);
        out.normals.extend_from_slice(&[n, n, n]);
        if m.ao.len() == m.verts.len() {
            out.ao.extend_from_slice(&[
                m.ao[tri[0] as usize],
                m.ao[tri[1] as usize],
                m.ao[tri[2] as usize],
            ]);
        } else {
            out.ao.extend_from_slice(&[1.0, 1.0, 1.0]);
        }
        if m.cols.len() == m.verts.len() {
            out.cols.extend_from_slice(&[
                m.cols[tri[0] as usize],
                m.cols[tri[1] as usize],
                m.cols[tri[2] as usize],
            ]);
        }
        out.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    out
}
