//! Data-driven surface colour — ORRERY `paintEval` shape. LANDSCAPE_800 §T 611–616.
//!
//! Rules are a table of conditions + albedo + weight. Evaluation accumulates
//! matching contributions and normalises. No giant if-else in the hot path.

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
    let elev_n = if max_elev > 1e-3 { (elev / max_elev).clamp(0.0, 1.5) } else { 0.0 };
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
                b1 = b0; w1 = w0;
                b0 = Some(r); w0 = w;
            } else if w > w1 {
                b1 = Some(r); w1 = w;
            }
        }
    }

    let Some(top) = b0 else { return material::albedo(mat_id); };
    let Some(second) = b1 else { return top.albedo; };

    // Bias hard toward the winner so transitions are narrow bands rather than a
    // permanent 50/50 mush across the whole surface.
    let t = (w1 / (w0 + w1)).clamp(0.0, 0.5);
    let t = t * t * 4.0 * 0.5; // smooth, and never past half

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
