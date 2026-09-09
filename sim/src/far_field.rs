//! Far-field and endcap heightfield meshes — pure Rust, engine-agnostic.
//! Lifted out of the GDExtension surface so headless / bench can reuse them.

use crate::mesher::Mesh;
use crate::terrain::Terrain;

/// Coarse whole-drum surface. Heightfield only, no caves.
pub fn far_mesh(t: &Terrain, nt: usize, nz: usize, r_offset: f32) -> Mesh {
    far_mesh_sector(t, nt, nz, r_offset, 0, 1)
}

/// One angular sector of the far field (with one-cell overlap for seams).
pub fn far_mesh_sector(
    t: &Terrain,
    nt_full: usize,
    nz: usize,
    r_offset: f32,
    sector: usize,
    n_sectors: usize,
) -> Mesh {
    let h = t.hab;
    let nt_full = nt_full.max(8);
    let nz = nz.max(4);
    let n_sectors = n_sectors.max(1);
    let sector = sector % n_sectors;
    let per = (nt_full + n_sectors - 1) / n_sectors;
    let t0 = sector * per;
    let t1 = if sector + 1 == n_sectors {
        nt_full + 1
    } else {
        ((sector + 1) * per + 1).min(nt_full + 1)
    };
    let nt_sec = t1 - t0;
    if nt_sec < 2 {
        return Mesh::empty();
    }

    let mut m = Mesh::empty();
    let blur = |th: f32, z: f32| -> f32 {
        let e = 1.6f32;
        let a = t.surface_radius(th, z);
        let b = t.surface_radius(th + e / h.radius, z);
        let c = t.surface_radius(th - e / h.radius, z);
        let d = t.surface_radius(th, z + e);
        let f = t.surface_radius(th, z - e);
        (a * 0.40 + (b + c + d + f) * 0.15) + r_offset
    };
    for zi in 0..=nz {
        let z = (zi as f32 / nz as f32 - 0.5) * h.length;
        for ti in t0..t1 {
            let th = (ti % nt_full) as f32 / nt_full as f32 * std::f32::consts::TAU;
            let r = blur(th, z);
            let p = h.to_world(th, z, r);
            m.verts.push(p);
            m.ao.push(1.0);
            let e = 1.2f32;
            let dt = (blur(th + e / h.radius, z) - blur(th - e / h.radius, z)) / (2.0 * e);
            let dz = (blur(th, z + e) - blur(th, z - e)) / (2.0 * e);
            let up = h.up_at(p);
            let tang = [-th.sin(), th.cos(), 0.0];
            let axial = [0.0, 0.0, 1.0];
            let mut n = [0.0f32; 3];
            for i in 0..3 {
                n[i] = up[i] + tang[i] * dt + axial[i] * dz;
            }
            let mag = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
            m.normals.push([n[0] / mag, n[1] / mag, n[2] / mag]);
        }
    }
    let vid = |ti: usize, zi: usize| (zi * nt_sec + ti) as i32;
    for zi in 0..nz {
        for ti in 0..nt_sec - 1 {
            let (a, b) = (vid(ti, zi), vid(ti + 1, zi));
            let (c, d2) = (vid(ti + 1, zi + 1), vid(ti, zi + 1));
            // Clockwise for Godot ArrayMesh; Three.js default is CCW front —
            // the web shell flips side if needed.
            m.indices.extend_from_slice(&[a, b, c, a, c, d2]);
        }
    }
    m
}

/// Drum endcaps so a look-up can't see out the open cylinder ends.
pub fn endcap_mesh(t: &Terrain, nt: usize) -> Mesh {
    let h = t.hab;
    let nt = nt.max(8);
    let mut m = Mesh::empty();
    for end in [-1.0f32, 1.0f32] {
        let z = end * (h.length * 0.5 - 2.0);
        let base = m.verts.len() as i32;
        m.verts.push([0.0, 0.0, z]);
        m.ao.push(1.0);
        m.normals.push([0.0, 0.0, -end]);
        for ti in 0..nt {
            let th = ti as f32 / nt as f32 * std::f32::consts::TAU;
            let r = t.surface_radius(th, z) + 1.2;
            m.verts.push(h.to_world(th, z, r));
            m.ao.push(1.0);
            m.normals.push([0.0, 0.0, -end]);
        }
        for ti in 0..nt {
            let a = base;
            let b = base + 1 + ti as i32;
            let c = base + 1 + ((ti + 1) % nt) as i32;
            if end > 0.0 {
                m.indices.extend_from_slice(&[a, c, b]);
            } else {
                m.indices.extend_from_slice(&[a, b, c]);
            }
        }
    }
    m
}
