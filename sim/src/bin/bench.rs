//! Headless timing of the sim core. No Godot. (REQUIREMENTS.md E2)
#[path = "../agent.rs"]
mod agent;
#[path = "../biome.rs"]
mod biome;
#[path = "../biosphere.rs"]
mod biosphere;
#[path = "../chronicle.rs"]
mod chronicle;
#[path = "../chunker.rs"]
mod chunker;
#[path = "../dwelling.rs"]
mod dwelling;
#[path = "../economy.rs"]
mod economy;
#[path = "../edits.rs"]
mod edits;
#[path = "../erosion.rs"]
mod erosion;
#[path = "../far_field.rs"]
mod far_field;
#[path = "../flow.rs"]
mod flow;
#[path = "../forest.rs"]
mod forest;
#[path = "../habitat.rs"]
mod habitat;
#[path = "../lakes.rs"]
mod lakes;
#[path = "../material.rs"]
mod material;
#[path = "../mesher.rs"]
mod mesher;
#[path = "../noise.rs"]
mod noise;
#[path = "../paint.rs"]
mod paint;
#[path = "../persist.rs"]
mod persist;
#[path = "../plant.rs"]
mod plant;
#[path = "../province.rs"]
mod province;
#[path = "../soil.rs"]
mod soil;
#[path = "../sph.rs"]
mod sph;
#[path = "../terrain.rs"]
mod terrain;
#[path = "../tree_form.rs"]
mod tree_form;
#[path = "../trophic.rs"]
mod trophic;
#[path = "../weather.rs"]
mod weather;
#[path = "../woodscape.rs"]
mod woodscape;

use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    let hab = habitat::Habitat::kepler_drum();
    let ter = terrain::Terrain::generate(hab);
    println!("generate (erosion + live flow): {:?}", t0.elapsed());
    println!(
        "  gravity {:.2} m/s2, period {:.1}s, sagitta@32m {:.3}m",
        hab.surface_gravity(),
        hab.spin_period(),
        hab.chunk_sagitta(32.0)
    );

    // Far field
    let t1 = Instant::now();
    let (nt, nz) = (720usize, 240usize);
    let mut n = 0;
    for zi in 0..=nz {
        for ti in 0..nt {
            let z = (zi as f32 / nz as f32 - 0.5) * hab.length;
            let th = ti as f32 / nt as f32 * std::f32::consts::TAU;
            std::hint::black_box(ter.surface_radius(th, z));
            n += 1;
        }
    }
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
    println!(
        "density x200k: {:?}  ({:.0} ns/sample) [{}]",
        t2.elapsed(),
        per,
        acc as i32
    );

    // One near chunk — the real path the game runs, not an approximation of it.
    // This is the single most expensive thing that can land on a frame.
    let samples = [
        (0i64, 0i64),
        (11, 3),
        (37, -8),
        (64, 21),
        (91, -30),
        (120, 47),
    ];
    let workers = chunker::worker_count();
    let bio = biosphere::Biosphere::new(&ter);
    for &nw in &[1usize, workers] {
        let (mut worst, mut total) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
        let (mut mesh_t, mut paint_t, mut shade_t) = (
            std::time::Duration::ZERO,
            std::time::Duration::ZERO,
            std::time::Duration::ZERO,
        );
        let (mut vsum, mut tsum) = (0usize, 0usize);
        for (ci, cj) in samples {
            let t3 = Instant::now();
            let mut m = chunker::chunk_mesh(&ter, ci, cj, nw);
            let a = t3.elapsed();
            let t4 = Instant::now();
            let (th_c, z_c) = chunker::chunk_centre(&ter, ci, cj);
            m.cols = vec![[0.0f32; 3]; m.verts.len()];
            let run = |out: &mut [[f32; 3]], from: usize| {
                let mut win = paint::BiomeWindow::new(ter.hab.radius, th_c, z_c, 46.0);
                for (i, c) in out.iter_mut().enumerate() {
                    *c = paint::vertex_color(
                        &ter,
                        Some(&bio),
                        Some(&mut win),
                        m.verts[from + i],
                        m.normals[from + i],
                    );
                }
            };
            if nw > 1 {
                let per = m.verts.len().div_ceil(nw);
                std::thread::scope(|sc| {
                    for (w, block) in m.cols.chunks_mut(per).enumerate() {
                        let run = &run;
                        sc.spawn(move || run(block, w * per));
                    }
                });
            } else {
                run(&mut m.cols, 0);
            }
            let b = t4.elapsed();
            let t5 = Instant::now();
            let m = mesher::flat_shade(m);
            let c = t5.elapsed();
            let dt = a + b + c;
            mesh_t += a;
            paint_t += b;
            shade_t += c;
            worst = worst.max(dt);
            total += dt;
            vsum += m.verts.len();
            tsum += m.indices.len() / 3;
        }
        let k = samples.len() as u32;
        println!(
            "CHUNK x{} ({} worker{}): mean {:?}, worst {:?} -> {} verts, {} tris",
            samples.len(),
            nw,
            if nw == 1 { "" } else { "s" },
            total / k,
            worst,
            vsum,
            tsum
        );
        println!(
        "  mesh {:?} + paint {:?} + flat-shade {:?} per chunk · worst is {:.0}% of a 16.7 ms frame",
        mesh_t / k,
        paint_t / k,
        shade_t / k,
        worst.as_secs_f64() / 0.0166667 * 100.0
    );
    }

    // What painting a vertex actually spends its time on.
    {
        let mut pts: Vec<[f32; 3]> = Vec::new();
        for i in 0..20_000 {
            let th = (i as f32) * 0.000_37;
            let z = ((i % 900) as f32 - 450.0) * 3.0;
            pts.push(hab.to_world(th, z, ter.surface_radius(th, z) + 0.5));
        }
        let piece = |name: &str, f: &dyn Fn() -> f32| {
            let t = Instant::now();
            let v = std::hint::black_box(f());
            let _ = v;
            println!(
                "  {:>18}: {:>7.0} ns/vertex",
                name,
                t.elapsed().as_secs_f64() / 20_000.0 * 1e9
            );
        };
        println!("paint breakdown (20k surface points):");
        piece("material_at", &|| {
            pts.iter()
                .map(|p| material::material_at(&ter, *p) as f32)
                .sum()
        });
        piece("biome_at", &|| {
            pts.iter()
                .map(|p| {
                    let (th, z, _) = hab.to_cyl(*p);
                    bio.biome_at(&ter, th, z) as f32
                })
                .sum()
        });
        piece("soil.sample", &|| {
            pts.iter()
                .map(|p| {
                    let (th, z, _) = hab.to_cyl(*p);
                    bio.soil.sample(th, z).moisture
                })
                .sum()
        });
        piece("water_flux", &|| {
            pts.iter()
                .map(|p| {
                    let (th, z, _) = hab.to_cyl(*p);
                    ter.water_flux(th, z)
                })
                .sum()
        });
        piece("in_lake", &|| {
            pts.iter()
                .map(|p| {
                    let (th, z, _) = hab.to_cyl(*p);
                    ter.in_lake(th, z) as i32 as f32
                })
                .sum()
        });
        piece("water.sample_depth", &|| {
            pts.iter()
                .map(|p| {
                    let (th, z, _) = hab.to_cyl(*p);
                    let x = th.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
                        * terrain::NT as f32;
                    let y = ((z / hab.length + 0.5) * terrain::NZ as f32)
                        .clamp(0.0, (terrain::NZ - 1) as f32);
                    bio.water.sample_depth(x, y)
                })
                .sum()
        });
        piece("vertex_color (raw)", &|| {
            pts.iter()
                .map(|p| paint::vertex_color(&ter, Some(&bio), None, *p, hab.up_at(*p))[0])
                .sum()
        });
        piece("province_at", &|| {
            pts.iter()
                .map(|p| {
                    let (th, z, _) = hab.to_cyl(*p);
                    province::province_at(&hab, th, z).weight(province::id::DUNE_SEA)
                })
                .sum()
        });
        piece("elevation0", &|| {
            pts.iter()
                .map(|p| {
                    let (th, z, _) = hab.to_cyl(*p);
                    ter.elevation0(th, z)
                })
                .sum()
        });
    }

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
        println!(
            "40 trench digs: {:?} ({} strokes)",
            t5.elapsed(),
            ter.edits.len()
        );
        let elev_mid = ter.elevation(0.5, 0.0);
        let t6 = Instant::now();
        let changed = ter.refresh_flow();
        println!("flow rebuild: {:?} (changed={})", t6.elapsed(), changed);
        let flux_after = ter.water_flux(0.5, 0.0);
        let after = ter.density(p);
        println!(
            "density at hit: {:.2} -> {:.2}  {}",
            before,
            after,
            if after < 0.0 { "CARVED" } else { "FAILED" }
        );
        println!("elev at trench: {:.2} -> {:.2} m", elev_before, elev_mid);

        println!("flux at trench: {:.3} -> {:.3}", flux_before, flux_after);

        let mat = material::material_at(&ter, p);
        println!(
            "material at hit: {} (hardness {:.1})",
            material::name(mat),
            material::info(mat).hardness
        );

        let segs = flow::river_segments(
            &ter.hab,
            &ter.elev,
            &ter.flow.discharge,
            &ter.flow.down,
            0.012,
            None,
            0.55,
            None,
        );
        println!("river segments: {} ({} floats)", segs.len() / 7, segs.len());

        bench_forest(&ter);
    }

    /// Stage 0 forest baseline: population shape and the cost of measuring it.
    fn bench_forest(ter: &terrain::Terrain) {
        let t1 = Instant::now();
        let bio = biosphere::Biosphere::new(ter);
        let bio_ms = t1.elapsed().as_secs_f64() * 1000.0;
        let t2 = Instant::now();
        let st = forest::stats(&bio.plants, &ter.hab);
        let stat_ms = t2.elapsed().as_secs_f64() * 1000.0;
        println!("forest: biosphere {bio_ms:.0} ms | stats {stat_ms:.1} ms");
        println!("forest: {}", forest::summary(&st));
    }
}
