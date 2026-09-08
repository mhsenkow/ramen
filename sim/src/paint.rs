//! Data-driven surface colour — ORRERY `paintEval` shape. LANDSCAPE_800 §T 611–616.
//!
//! Rules are a table of conditions + albedo + weight. Evaluation accumulates
//! matching contributions and normalises. No giant if-else in the hot path.

use crate::biosphere::Biosphere;
use crate::terrain::Terrain;

use crate::material;

/// One paint contribution. Thresholds `< 0` mean "ignore this axis".
/// `when_mat < 0` matches any material.
#[derive(Clone, Copy, Debug)]
pub struct PaintRule {
    pub when_slope_gt: f32,
    pub when_elev_gt: f32,
    pub when_flux_gt: f32,
    pub when_mat: i16,
    pub albedo: [f32; 3],
    pub weight: f32,
}

/// Default look: rock, grass, soil, channel, alpine.
pub const DEFAULT_TABLE: &[PaintRule] = &[
    // Soil / soft substrate (material-keyed bases).
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::REGOLITH as i16,
        albedo: [0.42, 0.34, 0.24],
        weight: 0.55,
    },
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::SEDIMENT as i16,
        albedo: [0.38, 0.30, 0.18],
        weight: 0.70,
    },
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::CLAY as i16,
        albedo: [0.40, 0.28, 0.20],
        weight: 0.70,
    },
    // Grass on soft mats (gentle implied by competing rock rule).
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::REGOLITH as i16,
        albedo: [0.14, 0.32, 0.11],
        weight: 1.0,
    },
    // Hard rock materials.
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::SANDSTONE as i16,
        albedo: [0.55, 0.42, 0.28],
        weight: 1.0,
    },
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::BASALT as i16,
        albedo: [0.22, 0.22, 0.24],
        weight: 1.0,
    },
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::FERROUS as i16,
        albedo: [0.35, 0.22, 0.16],
        weight: 1.0,
    },
    // Cliff / steep rock wash (any mat).
    PaintRule {
        when_slope_gt: 0.45,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: -1,
        albedo: [0.48, 0.44, 0.38],
        weight: 1.5,
    },
    // Channel ribbon.
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: 0.55,
        when_mat: -1,
        albedo: [0.12, 0.28, 0.36],
        weight: 1.6,
    },
    // Alpine / high ground.
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: 0.55,
        when_flux_gt: -1.0,
        when_mat: -1,
        albedo: [0.64, 0.60, 0.48],
        weight: 1.25,
    },
    // Farm / arable flats — base green that color_at overlays with patchwork.
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::SEDIMENT as i16,
        albedo: [0.20, 0.40, 0.14],
        weight: 0.85,
    },
    // Sand / beach / dune.
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::SAND as i16,
        albedo: [0.78, 0.66, 0.44],
        weight: 1.35,
    },
    // Peat / swamp flats — dark olive, gentle + wet.
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: 0.42,
        when_mat: material::id::SEDIMENT as i16,
        albedo: [0.16, 0.28, 0.18],
        weight: 1.05,
    },
    // Meadow herb — bright lime on low gentle ground.
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::REGOLITH as i16,
        albedo: [0.28, 0.52, 0.18],
        weight: 0.72,
    },
    // Salt / alkali crust — arid flats, low flux.
    PaintRule {
        when_slope_gt: -1.0,
        when_elev_gt: -1.0,
        when_flux_gt: -1.0,
        when_mat: material::id::CLAY as i16,
        albedo: [0.72, 0.70, 0.62],
        weight: 0.55,
    },
];

/// Evaluate `DEFAULT_TABLE`.
pub fn paint(slope: f32, elev: f32, flux: f32, mat_id: u8, max_elev: f32) -> [f32; 3] {
    paint_table(DEFAULT_TABLE, slope, elev, flux, mat_id, max_elev)
}

pub fn paint_table(
    table: &[PaintRule],
    slope: f32,
    elev: f32,
    flux: f32,
    mat_id: u8,
    max_elev: f32,
) -> [f32; 3] {
    let elev_n = if max_elev > 1e-3 {
        (elev / max_elev).clamp(0.0, 1.5)
    } else {
        0.0
    };
    let slope = slope.clamp(0.0, 1.0);
    let flux = flux.clamp(0.0, 1.0);

    // Take the two strongest rules and blend ONLY those.
    //
    // A weighted average over every matching rule is what made the whole world
    // pale: averaging grass, sediment, rock and upland together walks the colour
    // toward grey, because averaging distinct hues always does. ORRERY's
    // paintEval lerps along a ramp between two entries; it never sums the table.
    let (mut b0, mut w0) = (None::<&PaintRule>, 0.0f32);
    let (mut b1, mut w1) = (None::<&PaintRule>, 0.0f32);

    for r in table {
        if let Some(w) = rule_weight(r, slope, elev_n, flux, mat_id) {
            if w > w0 {
                b1 = b0;
                w1 = w0;
                b0 = Some(r);
                w0 = w;
            } else if w > w1 {
                b1 = Some(r);
                w1 = w;
            }
        }
    }

    let Some(top) = b0 else {
        return material::albedo(mat_id);
    };
    let Some(second) = b1 else {
        return top.albedo;
    };

    // Blend the top two linearly, capped at half. Squaring this biased so hard
    // toward the winner that cells FLIPPED at rule thresholds instead of
    // gradating, which speckled every slope where rock met grass. Linear keeps
    // transitions narrow without making them binary.
    let t = (w1 / (w0 + w1)).clamp(0.0, 0.5);

    [
        top.albedo[0] + (second.albedo[0] - top.albedo[0]) * t,
        top.albedo[1] + (second.albedo[1] - top.albedo[1]) * t,
        top.albedo[2] + (second.albedo[2] - top.albedo[2]) * t,
    ]
}

fn rule_weight(r: &PaintRule, slope: f32, elev_n: f32, flux: f32, mat_id: u8) -> Option<f32> {
    if r.when_mat >= 0 && r.when_mat as u8 != mat_id {
        return None;
    }

    let mut w = r.weight;

    if r.when_slope_gt >= 0.0 {
        if slope <= r.when_slope_gt {
            return None;
        }
        w *= ramp_gt(slope, r.when_slope_gt);
    }
    if r.when_elev_gt >= 0.0 {
        if elev_n <= r.when_elev_gt {
            return None;
        }
        w *= ramp_gt(elev_n, r.when_elev_gt);
    }
    if r.when_flux_gt >= 0.0 {
        if flux <= r.when_flux_gt {
            return None;
        }
        w *= ramp_gt(flux, r.when_flux_gt);
    }

    // Soft mats: fade grass/soil toward rock as slope rises (ramp mix).
    if r.when_mat >= 0
        && r.when_mat as u8 <= material::id::CLAY
        && r.when_slope_gt < 0.0
        && r.when_elev_gt < 0.0
        && r.when_flux_gt < 0.0
    {
        w *= (1.0 - slope * 1.35).clamp(0.05, 1.0);
        // Prefer grass (higher green) on lower elevations.
        if r.albedo[1] > r.albedo[0] + 0.1 {
            w *= (1.0 - (elev_n - 0.3).max(0.0) * 1.8).clamp(0.1, 1.0);
        }
    }

    Some(w.max(0.0))
}

#[inline]
fn ramp_gt(v: f32, thresh: f32) -> f32 {
    let span = (1.0 - thresh).max(0.08);
    ((v - thresh) / span).clamp(0.12, 1.0)
}

/// Vertex colour via paint table + wetness (LANDSCAPE_800 §T / §L).
///
/// Free-standing so `bin/bench` can price it: painting a chunk costs more than
/// meshing it, and that is not obvious from the call site.
pub fn vertex_color(
    t: &Terrain,
    bio_opt: Option<&Biosphere>,
    win: Option<&mut BiomeWindow>,
    p: [f32; 3],
    n: [f32; 3],
) -> [f32; 3] {
    let (theta, z, r) = t.hab.to_cyl(p);
    let surf = t.surface_radius(theta, z);
    let elev = t.hab.radius - r;
    let below = r - surf;
    let mat = crate::material::material_at(t, p);
    let alb = crate::material::albedo(mat);

    if below > 1.5 {
        // Strata bands on cave walls (item 201/14).
        let band = ((below * 0.08).sin() * 0.5 + 0.5) as f32;
        let k = (1.0 - (below / 70.0).clamp(0.0, 0.55)) as f32;
        let shade = 0.85 + 0.15 * band;
            return [alb[0] * k * shade, alb[1] * k * shade, alb[2] * k * shade];
    }

    let up = t.hab.up_at(p);
    let slope = 1.0 - (n[0] * up[0] + n[1] * up[1] + n[2] * up[2]).clamp(-1.0, 1.0);
    let flux = t.water_flux(theta, z);
    let lake = t.in_lake(theta, z);
    let w = t.hab.water_level;

    if elev < w || lake {
        // Wet basin under the free surface — cool damp sand, not near-black.
        // When the pool sheet is thin or missing this must still read wet,
        // never as a dig void (BotW lake beds stay readable under water).
        let mud = [
            (alb[0] * 0.45 + 0.22).clamp(0.0, 1.0),
            (alb[1] * 0.42 + 0.24).clamp(0.0, 1.0),
            (alb[2] * 0.38 + 0.26).clamp(0.0, 1.0),
        ];
        return mud;
    }

    let mut col = paint(slope, elev, flux, mat, t.hab.max_elevation);
    let prov = crate::province::province_at(&t.hab, theta, z);
    let farm_w = prov.weight(crate::province::id::FARMLAND);
    let city_w = prov.weight(crate::province::id::CITY);
    // Farm patchwork — hashed field tones so the far wall reads as paddies
    // rather than one green wash (LANDSCAPE_3200 Wave 1.5). Province farmland
    // forces the same grid even before the emergent FARM biome catches up.
    if let Some(bio) = bio_opt {
        // One scratch classifier for this vertex: the window when the caller
        // gave us one, the raw path otherwise.
        let mut scratch = BiomeWindow::new(t.hab.radius, theta, z, 0.0);
        let w = win.unwrap_or(&mut scratch);
        let bid = w.at(t, bio, theta, z);
        // Biome albedo pull — swamp peat, meadow herb, desert salt, dune gold, shore.
        {
            let pull = match bid {
                crate::biome::id::SWAMP => ([0.14, 0.26, 0.18], 0.42),
                crate::biome::id::MEADOW => ([0.30, 0.54, 0.20], 0.38),
                crate::biome::id::DESERT => ([0.68, 0.56, 0.36], 0.40),
                crate::biome::id::DUNE => ([0.80, 0.68, 0.44], 0.48),
                crate::biome::id::SHORE => ([0.74, 0.66, 0.50], 0.45),
                crate::biome::id::FOREST => ([0.10, 0.28, 0.12], 0.28),
                crate::biome::id::WETLAND => ([0.18, 0.34, 0.22], 0.30),
                _ => ([0.0, 0.0, 0.0], 0.0),
            };
            if pull.1 > 0.0 {
                for i in 0..3 {
                    col[i] = col[i] + (pull.0[i] - col[i]) * pull.1;
                }
            }
            // Salt crust sparkle on arid flats with tiny flux.
            if matches!(bid, crate::biome::id::DESERT | crate::biome::id::DUNE) && flux < 0.12 && slope < 0.25
            {
                let salt = [0.78, 0.76, 0.68];
                for i in 0..3 {
                    col[i] = col[i] + (salt[i] - col[i]) * 0.22;
                }
            }
        }
        let want_farm = bid == crate::biome::id::FARM || farm_w > 0.42;
        if want_farm && city_w < 0.55 {
            let cell_m = if farm_w > 0.42 { 22.0 } else { 18.0 };
            let cell_th = (theta * t.hab.radius / cell_m).floor();
            let cell_z = (z / cell_m).floor();
            let h = {
                let mut x = (cell_th as i32 as u32).wrapping_mul(0x9E3779B1)
                    ^ (cell_z as i32 as u32).wrapping_mul(0x85EBCA77);
                x ^= x >> 15;
                x = x.wrapping_mul(0x2C1B3C6D);
                x ^= x >> 13;
                x
            };
            let tone = (h & 0xFF) as f32 / 255.0;
            // Alternate cooler wet paddies / warmer dry plots.
            let a = [0.16, 0.38, 0.14];
            let b = [0.28, 0.42, 0.12];
            let hedgerow = ((h >> 8) & 0xFF) as f32 / 255.0;
            let mix = if farm_w > 0.42 {
                0.55 + 0.45 * farm_w
            } else {
                1.0
            };
            for i in 0..3 {
                let field = a[i] + (b[i] - a[i]) * tone;
                col[i] = col[i] + (field - col[i]) * mix;
            }
            // Dark hedgerow lines on cell edges.
            let edge_th = (theta * t.hab.radius).rem_euclid(cell_m);
            let edge_z = z.rem_euclid(cell_m);
            if edge_th < 0.85 || edge_z < 0.85 || hedgerow > 0.92 {
                col[0] *= 0.85;
                col[1] *= 0.85;
                col[2] *= 0.85;
            }
        }
        // City block network — pale plaster lots + dark road seams, ~90 m.
        if city_w > 0.40 {
            let block = 90.0;
            let cell_th = (theta * t.hab.radius / block).floor();
            let cell_z = (z / block).floor();
            let h = {
                let mut x = (cell_th as i32 as u32).wrapping_mul(0x51A7C170)
                    ^ (cell_z as i32 as u32).wrapping_mul(0xC0FFEE11);
                x ^= x >> 14;
                x = x.wrapping_mul(0x2C1B3C6D);
                x ^= x >> 13;
                x
            };
            let tone = (h & 0xFF) as f32 / 255.0;
            let lot = [0.62 + 0.12 * tone, 0.60 + 0.10 * tone, 0.56 + 0.08 * tone];
            let road = [0.28, 0.28, 0.30];
            let edge_th = (theta * t.hab.radius).rem_euclid(block);
            let edge_z = z.rem_euclid(block);
            let on_road = edge_th < 4.5 || edge_z < 4.5 || edge_th > block - 4.5 || edge_z > block - 4.5;
            let target = if on_road { road } else { lot };
            let mix = (0.50 + 0.50 * city_w).min(0.92);
            for i in 0..3 {
                col[i] = col[i] + (target[i] - col[i]) * mix;
            }
        }
        let mut edge_col = [0.0f32; 3];
        let mut edge_n = 0.0f32;
        let dth = 8.0 / t.hab.radius;
        for &(oth, oz) in &[
            (theta + dth, z),
            (theta - dth, z),
            (theta, z + 8.0),
            (theta, z - 8.0),
        ] {
            let nb = bio.biome_at(t, oth, oz);
            if nb != bid {
                let bc = crate::biome::color(nb);
                edge_col[0] += bc[0];
                edge_col[1] += bc[1];
                edge_col[2] += bc[2];
                edge_n += 1.0;
            }
        }
        if edge_n > 0.0 {
            for i in 0..3 {
                let bc = edge_col[i] / edge_n;
                col[i] = col[i] + (bc - col[i]) * 0.25;
            }
        }
    } else if farm_w > 0.42 || city_w > 0.40 {
        // No biosphere yet (far mesh): still show engineered grids from province.
        if farm_w > 0.42 && city_w < 0.55 {
            let cell_m = 22.0;
            let cell_th = (theta * t.hab.radius / cell_m).floor();
            let cell_z = (z / cell_m).floor();
            let h = {
                let mut x = (cell_th as i32 as u32).wrapping_mul(0x9E3779B1)
                    ^ (cell_z as i32 as u32).wrapping_mul(0x85EBCA77);
                x ^= x >> 15;
                x = x.wrapping_mul(0x2C1B3C6D);
                x ^= x >> 13;
                x
            };
            let tone = (h & 0xFF) as f32 / 255.0;
            let a = [0.16, 0.38, 0.14];
            let b = [0.28, 0.42, 0.12];
            for i in 0..3 {
                let field = a[i] + (b[i] - a[i]) * tone;
                col[i] = col[i] + (field - col[i]) * (0.55 + 0.45 * farm_w);
            }
        }
        if city_w > 0.40 {
            let block = 90.0;
            let edge_th = (theta * t.hab.radius).rem_euclid(block);
            let edge_z = z.rem_euclid(block);
            let on_road = edge_th < 4.5 || edge_z < 4.5 || edge_th > block - 4.5 || edge_z > block - 4.5;
            let target = if on_road {
                [0.28, 0.28, 0.30]
            } else {
                [0.66, 0.64, 0.60]
            };
            let mix = (0.50 + 0.50 * city_w).min(0.92);
            for i in 0..3 {
                col[i] = col[i] + (target[i] - col[i]) * mix;
            }
        }
    }
    if slope > 0.35 {
        let band = (elev * 0.09).sin() * 0.5 + 0.5;
        let k = band * 0.15;
        for i in 0..3 {
            col[i] = col[i] + (alb[i] - col[i]) * k;
        }
    }
    let elev0 = t.elevation0(theta, z);
    if elev0 - elev > 0.4 {
        let fresh = ((elev0 - elev) / 8.0).clamp(0.0, 1.0);
        for i in 0..3 {
            col[i] = col[i] + (alb[i] - col[i]) * fresh;
        }
    }
    // Wetness from soil + surface-water shore band (item 306/307).
    let mut wet = if let Some(bio) = bio_opt {
        bio.soil.sample(theta, z).moisture
    } else {
        (flux * 1.3).clamp(0.0, 1.0)
    };
    if let Some(bio) = bio_opt {
        let th = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
            * crate::terrain::NT as f32;
        let zz = ((z / t.hab.length + 0.5) * crate::terrain::NZ as f32)
            .clamp(0.0, (crate::terrain::NZ - 1) as f32);
        let shore = bio.water.sample_depth(th, zz);
        // Dark wet bank within ~1.2 m of standing water.
        if shore > 0.02 && shore < 1.4 {
            wet = wet.max((1.0 - shore / 1.4) * 0.85);
        } else if shore >= 1.4 {
            wet = wet.max(0.7);
        }
    }
    // Soft wet darken — 0.34 crushed banks toward black voids beside pools.
    let dark = 1.0 - (0.38 * wet * (1.0 - slope.clamp(0.0, 1.0) * 0.7));
    col[0] *= dark;
    col[1] *= dark * (1.0 - 0.05 * wet);
    col[2] *= dark;

    let ndl = (n[0] * up[0] + n[1] * up[1] + n[2] * up[2]).clamp(0.0, 1.0);
    // Light lives in the shader now (AO, warm key, cool fill). Keep only a
    // whisper of baked orientation so facets differ before shading.
    let shade = (0.94 + 0.10 * ndl).clamp(0.90, 1.04);
    [col[0] * shade, col[1] * shade, col[2] * shade]
}

/// Biome ids over a chunk-sized window, on a coarse grid, filled on demand.
///
/// `vertex_color` reads the biome under the vertex and at four points 8 m out,
/// to find ecotone edges. One classification is a soil sample, five elevation
/// samples and two weather lookups — doing that five times per vertex was most
/// of what it cost to paint a chunk. The field varies over tens of metres, so a
/// 2.5 m grid loses nothing the eye can find, and only the cells actually
/// touched are ever computed.
pub struct BiomeWindow {
    /// Arc metres at the hull, and axial metres, of the grid origin.
    arc0: f32,
    z0: f32,
    n: usize,
    step: f32,
    radius: f32,
    theta_c: f32,
    /// 255 = not yet classified.
    ids: Vec<u8>,
}

impl BiomeWindow {
    const STEP: f32 = 2.5;

    /// A window `half` metres either side of (theta_c, z_c).
    pub fn new(radius: f32, theta_c: f32, z_c: f32, half: f32) -> Self {
        let n = ((half * 2.0 / Self::STEP).ceil() as usize) + 2;
        Self {
            arc0: -half - Self::STEP,
            z0: z_c - half - Self::STEP,
            n,
            step: Self::STEP,
            radius,
            theta_c,
            ids: vec![255; n * n],
        }
    }

    /// Biome id at a point, classifying it the first time and remembering it.
    /// Points outside the window fall through to a direct classification.
    pub fn at(&mut self, t: &Terrain, bio: &Biosphere, theta: f32, z: f32) -> u8 {
        let arc = crate::terrain::wrap_pi(theta - self.theta_c) * self.radius;
        let fi = (arc - self.arc0) / self.step;
        let fk = (z - self.z0) / self.step;
        if fi < 0.0 || fk < 0.0 {
            return bio.biome_at(t, theta, z);
        }
        let (i, k) = (fi as usize, fk as usize);
        if i >= self.n || k >= self.n {
            return bio.biome_at(t, theta, z);
        }
        let s = i + self.n * k;
        if self.ids[s] == 255 {
            // Classify at the cell centre so neighbouring vertices agree.
            let cz = self.z0 + (k as f32 + 0.5) * self.step;
            let cth = self.theta_c + (self.arc0 + (i as f32 + 0.5) * self.step) / self.radius;
            self.ids[s] = bio.biome_at(t, cth, cz);
        }
        self.ids[s]
    }
}
