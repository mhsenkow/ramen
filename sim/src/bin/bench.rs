//! Headless timing of the sim core. No Godot. (REQUIREMENTS.md E2)
#[path = "../noise.rs"] mod noise;
#[path = "../habitat.rs"] mod habitat;
#[path = "../edits.rs"] mod edits;
#[path = "../flow.rs"] mod flow;
#[path = "../material.rs"] mod material;
#[path = "../terrain.rs"] mod terrain;
#[path = "../mesher.rs"] mod mesher;

use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    let hab = habitat::Habitat::kepler_drum();
    let ter = terrain::Terrain::generate(hab);
    println!("generate (erosion + live flow): {:?}", t0.elapsed());
    println!("  gravity {:.2} m/s2, period {:.1}s, sagitta@32m {:.3}m",
        hab.surface_gravity(), hab.spin_period(), hab.chunk_sagitta(32.0));

    // Far field
    let t1 = Instant::now();
    let (nt, nz) = (720usize, 240usize);
    let mut n = 0;
    for zi in 0..=nz { for ti in 0..nt {
        let z = (zi as f32 / nz as f32 - 0.5) * hab.length;
        let th = ti as f32 / nt as f32 * std::f32::consts::TAU;
        std::hint::black_box(ter.surface_radius(th, z)); n += 1;
    }}
    println!("far field {} verts: {:?}", n, t1.elapsed());

    // Density cost
    let t2 = Instant::now();
    let mut acc = 0.0f32;
    for i in 0..200_000 {
        let th = (i as f32) * 0.0001;
        let p = hab.to_world(th, (i % 900) as f32 - 450.0, hab.radius - 30.0);
        acc += ter.density(p);
    }
    let per = t2.elapsed().as_secs_f64() / 200_000.0 * 1e9;
    println!("density x200k: {:?}  ({:.0} ns/sample) [{}]", t2.elapsed(), per, acc as i32);

    // One near chunk
    let t3 = Instant::now();
    let (tc, zc, span, cell) = (0.5f32, 0.0f32, 52.0f32, 1.25f32);
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for a in 0..5 { for b in 0..5 {
        let th = tc + (a as f32/4.0-0.5)*span/hab.radius;
        let z = zc + (b as f32/4.0-0.5)*span;
        let s = ter.surface_radius(th, z); lo = lo.min(s); hi = hi.max(s);
    }}
    let r_lo = (lo-10.0).max(1.0); let r_hi = (hi+52.0).min(hab.radius);
    let n_lat = (span/cell).ceil() as usize;
    let n_rad = ((r_hi-r_lo)/cell).ceil() as usize;
    println!("chunk grid: {} x {} x {} = {} cells", n_lat, n_rad, n_lat, n_lat*n_rad*n_lat);
    let m = mesher::surface_nets_lattice(
        n_lat, n_rad, n_lat, (1, n_lat as usize - 1),
        |i, j, k| {
            let theta = tc + (i as f32 - 1.0) * cell / hab.radius;
            let z = zc + (k as f32 - 1.0) * cell;
            let r = r_hi - j as f32 * cell;
            hab.to_world(theta, z, r)
        },
        |p| ter.density(p));
    println!("ONE CHUNK: {:?} -> {} verts, {} tris", t3.elapsed(), m.verts.len(), m.indices.len()/3);
    println!("25 chunks would be ~{:.1}s", t3.elapsed().as_secs_f64()*25.0);

    // Mining: raycast, carve, elevation update, flow reroute.
    let mut ter = ter;
    let eye = hab.to_world(0.5, 0.0, ter.surface_radius(0.5, 0.0) - 1.7);
    let up = hab.up_at(eye);
    let dir = [-up[0], -up[1], -up[2]];
    let t4 = Instant::now();
    let hit = ter.raycast(eye, dir, 12.0);
    println!("raycast: {:?} -> {:?}", t4.elapsed(), hit.map(|h| h.0));
    if let Some((p, _, _)) = hit {
        let before = ter.density(p);
        let flux_before = ter.water_flux(0.5, 0.0);
        let elev_before = ter.elevation(0.5, 0.0);
        let t5 = Instant::now();
        // Dig a trench across the slope so drainage must move.
        for i in 0..40 {
            let q = [p[0], p[1], p[2] + i as f32 * 1.2];
            let _ = ter.dig(q, 3.0, 1.0, false);
        }
        println!("40 trench digs: {:?} ({} strokes)", t5.elapsed(), ter.edits.len());
        let elev_mid = ter.elevation(0.5, 0.0);
        let t6 = Instant::now();
        let changed = ter.refresh_flow();
        println!("flow rebuild: {:?} (changed={})", t6.elapsed(), changed);
        let flux_after = ter.water_flux(0.5, 0.0);
        let after = ter.density(p);
        println!("density at hit: {:.2} -> {:.2}  {}", before, after,
            if after < 0.0 { "CARVED" } else { "FAILED" });
        println!("elev at trench: {:.2} -> {:.2} m", elev_before, elev_mid);

        println!("flux at trench: {:.3} -> {:.3}", flux_before, flux_after);

        let mat = material::material_at(&ter, p);
        println!("material at hit: {} (hardness {:.1})",
            material::name(mat), material::info(mat).hardness);

        let segs = flow::river_segments(
            &ter.hab, &ter.elev, &ter.flow.discharge, &ter.flow.down, 0.012,
            None, 0.55, None,
        );
        println!("river segments: {} ({} floats)", segs.len() / 7, segs.len());
    }
}
