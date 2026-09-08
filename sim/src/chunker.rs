//! Building the near-field chunk lattice.
//!
//! Split out of the GDExtension surface so `bin/bench` measures the same code
//! the game runs — a chunk is the single most expensive thing that happens on
//! a frame, and a benchmark of an approximation of it is worth nothing.

use crate::habitat::Habitat;
use crate::mesher;
use crate::terrain::Terrain;

/// The global sample lattice every chunk shares. Tangential steps are chosen so
/// arc length at the hull is ~= the radial/axial cell size.
pub const LATTICE_CELL: f32 = 1.4;
pub const NT_LAT: u32 = 4096;
pub const CHUNK_N: u32 = 32;
/// Lattice cells of padding around a chunk. Sized to the widest ambient
/// occlusion probe (4.8 m) so occlusion is identical on both sides of a seam.
pub const AO_PAD: i64 = 4;

/// High-resolution near-field chunk WITH caves.
///
/// Sampled on ONE GLOBAL CYLINDRICAL LATTICE shared by every chunk, so
/// neighbours produce identical boundary vertices and the world is
/// watertight. Each chunk owns a disjoint set of quads (`emit`), so nothing
/// is drawn twice either. The old per-chunk rotated flat grid left a
/// one-cell crack at every seam.
///
/// The field is filled COLUMN-MAJOR. Everything in `density` that depends
/// only on (theta, z) — surface radius, slope, strata phase, drainage — is
/// then paid once per column instead of once per voxel, and the deep rock
/// and open air either side of the ground band are written from arithmetic
/// alone. Both shortcuts are exact where the mesh can see them; see
/// `t_fast` below for why.
pub fn chunk_mesh(t: &Terrain, ci: i64, cj: i64, workers: usize) -> mesher::Mesh {
    let h = t.hab;
    let cell = LATTICE_CELL;
    let n = CHUNK_N as i64;
    // Ambient occlusion probes read the lattice rather than the field, so
    // the pad has to cover the widest probe (4.8 m) or chunks would occlude
    // differently along their own seams.
    let pad: i64 = AO_PAD;
    let base_ti = ci * n - pad;
    let base_zj = cj * n - pad;

    let theta_of = |ti: i64| -> f32 {
        (ti.rem_euclid(NT_LAT as i64) as f32) / (NT_LAT as f32) * std::f32::consts::TAU
    };
    let z_of = |zj: i64| -> f32 { zj as f32 * cell - h.length * 0.5 };

    // One surface radius per lattice column. This is also the chunk's
    // elevation survey: exact at every point the mesher will ever look at,
    // which is what lets the radial band be tight instead of defensive.
    let n_lat = (n + 2 * pad) as usize; // cells
    let sxy = n_lat + 1; // samples per lateral axis
    let mut surf = vec![0.0f32; sxy * sxy];
    for k in 0..sxy {
        let z = z_of(base_zj + k as i64);
        for i in 0..sxy {
            surf[i + sxy * k] = t.surface_radius(theta_of(base_ti + i as i64), z);
        }
    }
    let (mut r_min, mut r_max) = (f32::MAX, f32::MIN);
    for &v in surf.iter() {
        r_min = r_min.min(v);
        r_max = r_max.max(v);
    }
    // Steepest step between neighbouring columns, diagonals included.
    let mut d_surf = 0.0f32;
    for k in 0..sxy - 1 {
        for i in 0..sxy - 1 {
            let a = surf[i + sxy * k];
            for (di, dk) in [(1usize, 0usize), (0, 1), (1, 1)] {
                d_surf = d_surf.max((a - surf[i + di + sxy * (k + dk)]).abs());
            }
        }
    }

    let e_hi = h.radius - r_min; // highest ground in the chunk
    let e_lo = h.radius - r_max; // lowest

    // Depth past which a sample may be written as plain `r - surf`.
    //
    // Only the cave-tube term carves unedited rock, and it can subtract at
    // most MAX_TUBE_CARVE. So a sample can only be a corner of a surface
    // cell if some neighbour sits within MAX_TUBE_CARVE of the surface; one
    // radial step plus the steepest lateral step bounds how far a neighbour
    // can be. Past that the shortcut cannot move a vertex — only the value
    // of rock that stays rock.
    let t_fast = (Terrain::MAX_TUBE_CARVE + cell + d_surf + 1.0)
        .max(Terrain::SOLID_DEPTH);

    // Ground band: air above the highest ground, cave depth below the lowest.
    let mut e_top = e_hi + 6.0;
    let mut e_bot = e_lo - (Terrain::SOLID_DEPTH + 2.0);

    // Bores and player strokes live outside that band and must pull it open.
    let half_arc = (n as f32 * 0.5 + pad as f32) * cell;
    let th_c = theta_of(base_ti) + (half_arc / h.radius);
    let z_c = z_of(base_zj) + half_arc;
    let feats = t.features_near(th_c, z_c, half_arc, half_arc);
    for f in feats.iter() {
        e_top = e_top.max(f.elev + f.reach);
        e_bot = e_bot.min(f.elev - f.reach);
    }

    let rk_lo = ((e_bot / cell).floor() as i64).max(0);
    let rk_hi = (e_top / cell).ceil() as i64;
    if rk_hi <= rk_lo {
        return mesher::Mesh::empty();
    }
    let base_rk = rk_lo - pad;
    let n_rad = ((rk_hi - rk_lo) + 2 * pad) as usize;

    // Columns a bore or a stroke touches take the full density path, top to
    // bottom: those terms both add and subtract, and neither respects the
    // ground band the shortcuts are derived from.
    let mut exact = vec![false; sxy * sxy];
    if !feats.is_empty() {
        let arc_step = h.radius * std::f32::consts::TAU / NT_LAT as f32;
        for f in feats.iter() {
            let ti = f.theta / std::f32::consts::TAU * NT_LAT as f32;
            // Nearest wrap of this angle to our window.
            let mut li = ti - base_ti as f32;
            let full = NT_LAT as f32;
            while li < -full * 0.5 {
                li += full;
            }
            while li > full * 0.5 {
                li -= full;
            }
            let lk = (f.z + h.length * 0.5) / cell - base_zj as f32;
            let ri = f.reach / arc_step;
            let rk = f.reach / cell;
            let i0 = ((li - ri).floor() as i64).max(0) as usize;
            let i1 = ((li + ri).ceil() as i64).clamp(0, sxy as i64 - 1) as usize;
            let k0 = ((lk - rk).floor() as i64).max(0) as usize;
            let k1 = ((lk + rk).ceil() as i64).clamp(0, sxy as i64 - 1) as usize;
            if i0 > i1 || k0 > k1 {
                continue;
            }
            for k in k0..=k1 {
                for i in i0..=i1 {
                    exact[i + sxy * k] = true;
                }
            }
        }
    }

    let mut field = mesher::Field::new(n_lat, n_rad, n_lat);
    let air = Terrain::AIR_DEPTH;
    let stride = n_lat + 1; // one step in j
    // One axial slice of the lattice — a contiguous run of the field buffer,
    // and the unit of work when this is spread over threads.
    let slice = |block: &mut [f32], k: usize| {
        let z = z_of(base_zj + k as i64);
        for i in 0..sxy {
            let theta = theta_of(base_ti + i as i64);
            let (cs, sn) = (theta.cos(), theta.sin());
            let s = surf[i + sxy * k];
            let force = exact[i + sxy * k];
            let mut col = t.column(theta, z);
            col.features = force;
            for j in 0..=n_rad {
                let r = h.radius - (base_rk + j as i64) as f32 * cell;
                let dgeo = r - s;
                block[i + j * stride] = if !force && (dgeo <= air || dgeo > t_fast) {
                    dgeo
                } else {
                    t.density_col(&mut col, [r * cs, r * sn, z], r)
                };
            }
        }
    };
    let plane = stride * (n_rad + 1); // floats in one axial slice
    if workers > 1 {
        // Scoped threads over disjoint slices of one buffer. Everything read is
        // shared immutably and nothing is written twice, so this is the same
        // borrow the compiler already checks — no locks, no copies. (Wave 1.6
        // established that std::thread inside the GDExtension is fine once the
        // dylib is signed; the exit-137s were codesign, not concurrency.)
        let per = sxy.div_ceil(workers);
        let slice = &slice;
        std::thread::scope(|sc| {
            for (w, block) in field.d.chunks_mut(per * plane).enumerate() {
                sc.spawn(move || {
                    for (n, sub) in block.chunks_mut(plane).enumerate() {
                        slice(sub, w * per + n);
                    }
                });
            }
        });
    } else {
        for (k, sub) in field.d.chunks_mut(plane).enumerate() {
            slice(sub, k);
        }
    }
    let emit = (pad as usize, (pad + n) as usize);
    mesher::surface_nets_field(
        &field,
        emit,
        |i, j, k| {
            let theta = (base_ti as f32 + i) / (NT_LAT as f32) * std::f32::consts::TAU;
            let z = (base_zj as f32 + k) * cell - h.length * 0.5;
            let r = h.radius - (base_rk as f32 + j) * cell;
            h.to_world(theta, z, r)
        },
        [1.6 / cell, 4.8 / cell],
    )
}

/// How many threads to spread one chunk over. One means "stay on this thread".
/// Two cores are left for Godot's own render and physics threads.
pub fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(2).clamp(1, 6))
        .unwrap_or(1)
}

/// The (theta, z) centre of a chunk.
pub fn chunk_centre(t: &Terrain, ci: i64, cj: i64) -> (f32, f32) {
    let n = CHUNK_N as f32;
    let theta = (ci as f32 * n + n * 0.5) / NT_LAT as f32 * std::f32::consts::TAU;
    let z = (cj as f32 * n + n * 0.5) * LATTICE_CELL - t.hab.length * 0.5;
    (theta, z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habitat::Habitat;

    /// The same lattice, filled the slow honest way: `density()` at every point,
    /// no column cache, no deep-rock or open-air shortcut.
    fn reference(t: &Terrain, ci: i64, cj: i64) -> mesher::Mesh {
        let h = t.hab;
        let cell = LATTICE_CELL;
        let n = CHUNK_N as i64;
        let pad = AO_PAD;
        let base_ti = ci * n - pad;
        let base_zj = cj * n - pad;
        let theta_of = |ti: i64| -> f32 {
            (ti.rem_euclid(NT_LAT as i64) as f32) / (NT_LAT as f32) * std::f32::consts::TAU
        };
        let z_of = |zj: i64| -> f32 { zj as f32 * cell - h.length * 0.5 };
        let n_lat = (n + 2 * pad) as usize;
        let sxy = n_lat + 1;
        // Same radial band as the real builder, so only the FILL differs.
        let real = chunk_mesh(t, ci, cj, 1);
        let _ = real;
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for k in 0..sxy {
            for i in 0..sxy {
                let s = t.surface_radius(theta_of(base_ti + i as i64), z_of(base_zj + k as i64));
                lo = lo.min(h.radius - s);
                hi = hi.max(h.radius - s);
            }
        }
        let feats = t.features_near(
            theta_of(base_ti) + ((n as f32 * 0.5 + pad as f32) * cell) / h.radius,
            z_of(base_zj) + (n as f32 * 0.5 + pad as f32) * cell,
            (n as f32 * 0.5 + pad as f32) * cell,
            (n as f32 * 0.5 + pad as f32) * cell,
        );
        let mut e_top = hi + 6.0;
        let mut e_bot = lo - (Terrain::SOLID_DEPTH + 2.0);
        for f in feats.iter() {
            e_top = e_top.max(f.elev + f.reach);
            e_bot = e_bot.min(f.elev - f.reach);
        }
        let rk_lo = ((e_bot / cell).floor() as i64).max(0);
        let rk_hi = (e_top / cell).ceil() as i64;
        let base_rk = rk_lo - pad;
        let n_rad = ((rk_hi - rk_lo) + 2 * pad) as usize;

        let mut field = mesher::Field::new(n_lat, n_rad, n_lat);
        for k in 0..sxy {
            let z = z_of(base_zj + k as i64);
            for i in 0..sxy {
                let theta = theta_of(base_ti + i as i64);
                let (cs, sn) = (theta.cos(), theta.sin());
                let mut col = t.column(theta, z);
                col.features = true;
                for j in 0..=n_rad {
                    let r = h.radius - (base_rk + j as i64) as f32 * cell;
                    let s = field.si(i, j, k);
                    field.d[s] = t.density_col(&mut col, [r * cs, r * sn, z], r);
                }
            }
        }
        mesher::surface_nets_field(
            &field,
            (pad as usize, (pad + n) as usize),
            |i, j, k| {
                let theta = (base_ti as f32 + i) / (NT_LAT as f32) * std::f32::consts::TAU;
                let z = (base_zj as f32 + k) * cell - h.length * 0.5;
                let r = h.radius - (base_rk as f32 + j) * cell;
                h.to_world(theta, z, r)
            },
            [1.6 / cell, 4.8 / cell],
        )
    }

    /// The shortcuts in `chunk_mesh` are claimed to be invisible to the mesh,
    /// not merely close. Hold them to that.
    #[test]
    fn shortcuts_do_not_move_the_surface() {
        let t = Terrain::generate(Habitat::kepler_drum());
        for (ci, cj) in [(0i64, 0i64), (11, 3), (37, -8), (64, 21), (91, -30)] {
            let fast = chunk_mesh(&t, ci, cj, 1);
            let slow = reference(&t, ci, cj);
            assert_eq!(
                fast.verts.len(),
                slow.verts.len(),
                "vertex count at chunk {ci},{cj}"
            );
            assert_eq!(fast.indices, slow.indices, "topology at chunk {ci},{cj}");
            for (a, b) in fast.verts.iter().zip(slow.verts.iter()) {
                for x in 0..3 {
                    assert!(
                        (a[x] - b[x]).abs() < 1e-4,
                        "vertex moved at chunk {ci},{cj}: {a:?} vs {b:?}"
                    );
                }
            }
            for (a, b) in fast.ao.iter().zip(slow.ao.iter()) {
                assert!((a - b).abs() < 1e-3, "ao changed at chunk {ci},{cj}");
            }
        }
    }

    /// A dug shaft leaves the ground band entirely; the chunk still has to mesh it.
    #[test]
    fn player_shaft_is_meshed() {
        let mut t = Terrain::generate(Habitat::kepler_drum());
        let theta = 0.5f32;
        let z = 0.0f32;
        let surf = t.surface_radius(theta, z);
        let idx = t.chunk_of(theta, z);
        let before = chunk_mesh(&t, idx.0, idx.1, 1).indices.len();
        for i in 0..18 {
            let r = surf + 4.0 * i as f32;
            let _ = t.dig(t.hab.to_world(theta, z, r), 3.5, 1.0, false);
        }
        let after = chunk_mesh(&t, idx.0, idx.1, 1);
        assert!(
            after.indices.len() > before,
            "shaft added no geometry ({before} -> {})",
            after.indices.len()
        );
        // The deepest bite is 68 m below the surface — far outside the band a
        // chunk would derive from its own elevations.
        let deepest = after
            .verts
            .iter()
            .map(|v| (v[0] * v[0] + v[1] * v[1]).sqrt())
            .fold(f32::MIN, f32::max);
        assert!(
            deepest > surf + 60.0,
            "shaft floor not meshed: deepest r {deepest}, surface {surf}"
        );
    }
}
