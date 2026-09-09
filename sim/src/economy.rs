//! Material economy — dig yields, inventory, stockpiles, recipes.
//! LANDSCAPE_1400.md §X items 801–806, 813, 820–822, 831–832.
//!
//! Closed-habitat premise: every gram is already here. Digging moves mass from
//! the density field into a carryable pool; recipes transfer between pools and
//! must balance.

use crate::edits::Stroke;
use crate::material::{self, id as mid};
use crate::plant::Plant;
use crate::terrain::Terrain;

/// In-place bulk density (kg/m³) and excavation bulking factor (item 803–804).
#[derive(Clone, Copy, Debug)]
pub struct Phys {
    pub bulk_kg_m3: f32,
    pub bulking: f32,
}

pub const PHYS: [Phys; 10] = [
    Phys {
        bulk_kg_m3: 1600.0,
        bulking: 1.20,
    }, // regolith
    Phys {
        bulk_kg_m3: 1500.0,
        bulking: 1.18,
    }, // sediment
    Phys {
        bulk_kg_m3: 1800.0,
        bulking: 1.22,
    }, // clay
    Phys {
        bulk_kg_m3: 2300.0,
        bulking: 1.25,
    }, // sandstone
    Phys {
        bulk_kg_m3: 3000.0,
        bulking: 1.30,
    }, // basalt
    Phys {
        bulk_kg_m3: 3500.0,
        bulking: 1.28,
    }, // ferrous ore
    Phys {
        bulk_kg_m3: 917.0,
        bulking: 1.05,
    }, // ice
    Phys {
        bulk_kg_m3: 7800.0,
        bulking: 1.00,
    }, // structural alloy
    Phys {
        bulk_kg_m3: 1550.0,
        bulking: 1.15,
    }, // sand
    Phys {
        bulk_kg_m3: 1350.0,
        bulking: 1.18,
    }, // coal — light for rock, which is why a sack of it is worth carrying
];

#[inline]
pub fn phys(id: u8) -> Phys {
    PHYS[(id as usize).min(PHYS.len() - 1)]
}

/// Continuous ore grade 0..1 from vein-core distance (item 806–807).
/// Non-ore materials return 0.
pub fn ore_grade(t: &Terrain, p: [f32; 3]) -> f32 {
    use crate::noise::fbm3;
    let (theta, z, r) = t.hab.to_cyl(p);
    let elev = t.elevation0(theta, z);
    let surf0 = t.hab.radius - elev;
    let below = (r - surf0).max(0.0);
    if below <= 4.0 {
        return 0.0;
    }
    let s = 0.018;
    let v1 = fbm3(p[0] * s, p[1] * s, p[2] * s, 4, t.hab.seed ^ 0x0E01);
    let v2 = fbm3(
        p[0] * s + 17.0,
        p[1] * s - 9.0,
        p[2] * s + 3.0,
        4,
        t.hab.seed ^ 0x0E02,
    );
    let vein = ((v1 - 0.5).abs()).max((v2 - 0.5).abs());
    let near_tunnel = t.near_tunnel(p, 14.0);
    let thresh = if near_tunnel { 0.045 } else { 0.028 };
    if vein >= thresh {
        return 0.0;
    }
    // Core → high grade; fringe → low. Peak ~0.65 at the axis of the capsule.
    let tnorm = (1.0 - vein / thresh).clamp(0.0, 1.0);
    0.08 + 0.57 * tnorm * tnorm
}

/// One material fraction of a dig (or harvest) yield.
#[derive(Clone, Copy, Debug, Default)]
pub struct YieldPart {
    pub material_id: u8,
    /// In-place solid volume removed (m³).
    pub volume_m3: f32,
    pub mass_kg: f32,
    /// Loose / broken volume after bulking (m³).
    pub loose_m3: f32,
    /// Ore grade (0 for non-ore).
    pub grade: f32,
}

#[derive(Clone, Debug, Default)]
pub struct DigYield {
    pub parts: Vec<YieldPart>,
    pub total_mass_kg: f32,
    pub total_loose_m3: f32,
    pub total_volume_m3: f32,
}

impl DigYield {
    /// Recompute the totals after the parts have been scaled in place.
    ///
    /// Needed because ice can melt out of a yield between excavation and the
    /// pack, and the totals are what the HUD and the pack both read.
    pub fn retotal(&mut self) {
        self.parts
            .retain(|p| p.mass_kg > 1e-6 || p.volume_m3 > 1e-8);
        self.total_mass_kg = self.parts.iter().map(|p| p.mass_kg).sum();
        self.total_loose_m3 = self.parts.iter().map(|p| p.loose_m3).sum();
        self.total_volume_m3 = self.parts.iter().map(|p| p.volume_m3).sum();
    }

    /// Fold another yield into this one (mine trees into a dig bite).
    pub fn append(&mut self, other: &DigYield) {
        for part in &other.parts {
            self.push(*part);
        }
    }

    pub(crate) fn push(&mut self, part: YieldPart) {
        if part.volume_m3 <= 1e-8 && part.mass_kg <= 1e-6 {
            return;
        }
        if let Some(existing) = self
            .parts
            .iter_mut()
            .find(|p| p.material_id == part.material_id)
        {
            let m0 = existing.mass_kg;
            let m1 = part.mass_kg;
            let g0 = existing.grade;
            let g1 = part.grade;
            existing.volume_m3 += part.volume_m3;
            existing.mass_kg += part.mass_kg;
            existing.loose_m3 += part.loose_m3;
            if m0 + m1 > 1e-6 {
                existing.grade = (g0 * m0 + g1 * m1) / (m0 + m1);
            }
        } else {
            self.parts.push(part);
        }
        self.total_mass_kg += part.mass_kg;
        self.total_loose_m3 += part.loose_m3;
        self.total_volume_m3 += part.volume_m3;
    }
}

/// Would a dig stroke remove solid at `p`? Mirrors `Edits::apply` for dig.
pub fn stroke_removes(s: &Stroke, p: [f32; 3]) -> bool {
    let rel = [p[0] - s.c[0], p[1] - s.c[1], p[2] - s.c[2]];
    if s.level {
        let h = rel[0] * s.up[0] + rel[1] * s.up[1] + rel[2] * s.up[2];
        let ax = [
            rel[0] - s.up[0] * h,
            rel[1] - s.up[1] * h,
            rel[2] - s.up[2] * h,
        ];
        let rad = (ax[0] * ax[0] + ax[1] * ax[1] + ax[2] * ax[2]).sqrt();
        let disc = s.radius - rad;
        // Dig removes everything above the plane inside the disc.
        disc > 0.0 && h > 0.0
    } else {
        let dist = (rel[0] * rel[0] + rel[1] * rel[1] + rel[2] * rel[2]).sqrt();
        dist < s.radius
    }
}

/// Integrate the stroke against the *current* density field (item 801–802, 805).
/// Call **before** adding the stroke to `edits`.
pub fn integrate_dig_yield(t: &Terrain, stroke: &Stroke) -> DigYield {
    let mut y = DigYield::default();
    let reach = if stroke.level {
        stroke.radius * 1.7 + 0.5
    } else {
        stroke.radius + 0.5
    };
    // Lattice step: fine enough for half-brush cliff cuts, cheap enough per dig.
    let step = (stroke.radius * 0.28).clamp(0.40, 0.85);
    let cell = step * step * step;
    let n = ((reach / step).ceil() as i32).max(2);
    // Sized from PHYS so adding a stratum does not silently fold it into the
    // last bucket — `.min(N)` below used to clamp anything new onto sand.
    const NMAT: usize = PHYS.len();
    let mut grade_acc = [0.0f32; NMAT];
    let mut grade_w = [0.0f32; NMAT];
    let mut vol = [0.0f32; NMAT];

    for iz in -n..=n {
        for iy in -n..=n {
            for ix in -n..=n {
                let p = [
                    stroke.c[0] + ix as f32 * step,
                    stroke.c[1] + iy as f32 * step,
                    stroke.c[2] + iz as f32 * step,
                ];
                if !stroke_removes(stroke, p) {
                    continue;
                }
                if t.density(p) <= 0.0 {
                    continue;
                }
                let mat = material::material_at(t, p);
                if mat == mid::ALLOY {
                    continue; // undiggable — density may still read solid near hull
                }
                let mi = (mat as usize).min(NMAT - 1);
                vol[mi] += cell;
                if mat == mid::FERROUS {
                    let g = ore_grade(t, p);
                    grade_acc[mi] += g * cell;
                    grade_w[mi] += cell;
                }
            }
        }
    }

    for id in 0u8..NMAT as u8 {
        let v = vol[id as usize];
        if v <= 1e-8 {
            continue;
        }
        let ph = phys(id);
        let grade = if grade_w[id as usize] > 1e-8 {
            grade_acc[id as usize] / grade_w[id as usize]
        } else {
            0.0
        };
        y.push(YieldPart {
            material_id: id,
            volume_m3: v,
            mass_kg: v * ph.bulk_kg_m3,
            loose_m3: v * ph.bulking,
            grade,
        });
    }
    y
}

// ---------------------------------------------------------------------------
// Vegetation harvest (item 813)
// ---------------------------------------------------------------------------

/// Biomass pools as carryable materials. Carbon-allocation pools are treated as
/// dry-mass kilograms with a fixed scale (authored, not molecular).
pub mod bio_id {
    pub const GREEN: u8 = 100; // leaf + soft tissue
    pub const WOOD: u8 = 101; // stem
    pub const FIBRE: u8 = 102; // root / bast proxy
    pub const SEED: u8 = 103; // reproductive
    /// Carried liquid water (LANDSCAPE_1400 item 817) — not a dig solid.
    pub const WATER: u8 = 104;
    // Foraged parts. `GREEN` used to stand for every soft tissue on every
    // plant, so it was simultaneously the broth base, the tare base and the
    // bowl topping — which meant no plant was worth walking to. These split
    // that pool by what the plant actually is; they do not add mass to a
    // harvest, they divide it (see `harvest_plant_as`).
    pub const FLOWER: u8 = 105;
    pub const FRUIT: u8 = 106;
    pub const VEG: u8 = 107;
    pub const GRASS: u8 = 108;
    /// Molten rock. Not a stratum: nothing in an artificial drum melts on its
    /// own, so this only exists downstream of a smelter (`melt_basalt`).
    pub const LAVA: u8 = 109;
}

/// Crafted / processed material ids (Wave 2).
pub mod craft_id {
    pub const CHARCOAL: u8 = 110;
    pub const CERAMIC: u8 = 111;
    pub const GLASS: u8 = 112;
    pub const IRON: u8 = 113;
    pub const SLAG: u8 = 114;
    pub const ASH: u8 = 115;
    pub const LIME: u8 = 116;
    pub const GRAVEL: u8 = 117;
    pub const DUST: u8 = 118;
    pub const SAND: u8 = 119;
    pub const MANURE: u8 = 120;
    pub const BONE_MEAL: u8 = 121;
    /// Mass that left solids into atmosphere — not a carryable stack.
    pub const GAS_LOSS: u8 = 122;
    // Cooking — the ramen livelihood loop (garden → kitchen).
    pub const FLOUR: u8 = 130;
    pub const OIL: u8 = 131;
    pub const BROTH: u8 = 132;
    pub const NOODLES: u8 = 133;
    pub const TARE: u8 = 134;
    pub const RAMEN: u8 = 135;
    pub const RICH_RAMEN: u8 = 136;
    pub const TEA: u8 = 137;
    pub const THATCH: u8 = 138;
}

pub fn bio_name(id: u8) -> &'static str {
    match id {
        bio_id::GREEN => "greens",
        bio_id::WOOD => "timber",
        bio_id::FIBRE => "fibre",
        bio_id::SEED => "seed",
        bio_id::WATER => "water",
        bio_id::FLOWER => "flowers",
        bio_id::FRUIT => "fruit",
        bio_id::VEG => "vegetables",
        bio_id::GRASS => "grass",
        bio_id::LAVA => "molten rock",
        craft_id::CHARCOAL => "charcoal",
        craft_id::CERAMIC => "ceramic",
        craft_id::GLASS => "glass",
        craft_id::IRON => "iron",
        craft_id::SLAG => "slag",
        craft_id::ASH => "ash",
        craft_id::LIME => "lime",
        craft_id::GRAVEL => "gravel",
        craft_id::DUST => "dust",
        craft_id::SAND => "sand",
        craft_id::MANURE => "manure",
        craft_id::BONE_MEAL => "bone meal",
        craft_id::GAS_LOSS => "gas_loss",
        craft_id::FLOUR => "flour",
        craft_id::OIL => "oil",
        craft_id::BROTH => "broth",
        craft_id::NOODLES => "noodles",
        craft_id::TARE => "tare",
        craft_id::RAMEN => "ramen",
        craft_id::RICH_RAMEN => "rich ramen",
        craft_id::TEA => "tea",
        craft_id::THATCH => "thatch",
        // Only defer to the geological palette for ids that actually live in
        // it. `material::name` clamps out-of-range ids onto the last stratum,
        // so a bio or craft id missing an arm above used to surface as "sand",
        // and became "coal" the moment a stratum was appended. Naming the gap
        // is the only version of this that fails loudly.
        _ if (id as usize) < material::PALETTE.len() => material::name(id),
        _ => "?",
    }
}

/// Fraction of excavated ice that arrives as water rather than as ice, at a
/// given air temperature in °C.
///
/// The drum's ice lenses sit 8–40 m down near the endcaps, which is the only
/// place cold enough to keep them. Cut into one in a temperate province and
/// you have opened a freezer into a 26 °C room: most of what you lift out is
/// meltwater before you can stack it. That is also where a habitat's water
/// reserve is — buried ice is stored water, and this is the tap.
///
/// Latent heat is not modelled. What is modelled is that ice is the one
/// stratum whose *identity* depends on staying cold, which no other material
/// in the palette does.
pub fn ice_melt_fraction(temp_c: f32) -> f32 {
    if temp_c <= 0.0 {
        return 0.0;
    }
    // Full melt by ~14 °C; a cold morning leaves you carrying most of it.
    (temp_c / 14.0).clamp(0.0, 0.92)
}

pub fn material_id_by_name(name: &str) -> Option<u8> {
    Some(match name {
        "regolith" => mid::REGOLITH,
        "sediment" => mid::SEDIMENT,
        "clay" => mid::CLAY,
        "sandstone" => mid::SANDSTONE,
        "basalt" => mid::BASALT,
        "ferrous ore" => mid::FERROUS,
        "ice" => mid::ICE,
        "greens" => bio_id::GREEN,
        "timber" => bio_id::WOOD,
        "fibre" => bio_id::FIBRE,
        "seed" => bio_id::SEED,
        "water" => bio_id::WATER,
        "flowers" => bio_id::FLOWER,
        "fruit" => bio_id::FRUIT,
        "vegetables" => bio_id::VEG,
        "grass" => bio_id::GRASS,
        "molten rock" => bio_id::LAVA,
        "coal" => mid::COAL,
        "charcoal" => craft_id::CHARCOAL,
        "ceramic" => craft_id::CERAMIC,
        "glass" => craft_id::GLASS,
        "iron" => craft_id::IRON,
        "slag" => craft_id::SLAG,
        "ash" => craft_id::ASH,
        "lime" => craft_id::LIME,
        "gravel" => craft_id::GRAVEL,
        "dust" => craft_id::DUST,
        "sand" => craft_id::SAND,
        "manure" => craft_id::MANURE,
        "bone meal" => craft_id::BONE_MEAL,
        "gas_loss" => craft_id::GAS_LOSS,
        "flour" => craft_id::FLOUR,
        "oil" => craft_id::OIL,
        "broth" => craft_id::BROTH,
        "noodles" => craft_id::NOODLES,
        "tare" => craft_id::TARE,
        "ramen" => craft_id::RAMEN,
        "rich ramen" => craft_id::RICH_RAMEN,
        "tea" => craft_id::TEA,
        "thatch" => craft_id::THATCH,
        _ => return None,
    })
}

/// Display colour for any carryable id, across all three namespaces.
///
/// `world.gd` carries a hand-written `match` from id to `Color` for heap and
/// stack tinting, defaulting to dirt brown. That default is why every material
/// added since it was written — coal, fruit, grass, molten rock — piles up
/// looking like spoil. Exposing the colour here means the table can be deleted
/// rather than extended a tenth time, and a new material gets a look by
/// declaring one instead of by remembering to edit GDScript.
pub fn display_albedo(id: u8) -> [f32; 3] {
    match id {
        bio_id::GREEN => [0.30, 0.48, 0.24],
        bio_id::WOOD => [0.48, 0.34, 0.20],
        bio_id::FIBRE => [0.55, 0.48, 0.30],
        bio_id::SEED => [0.62, 0.52, 0.28],
        bio_id::WATER => [0.28, 0.52, 0.78],
        bio_id::FLOWER => [0.78, 0.52, 0.68],
        bio_id::FRUIT => [0.76, 0.34, 0.26],
        bio_id::VEG => [0.52, 0.56, 0.24],
        bio_id::GRASS => [0.62, 0.64, 0.32],
        // Reads as heat, not as rock — the one carryable that should look hot.
        bio_id::LAVA => [0.95, 0.42, 0.12],
        craft_id::CHARCOAL => [0.14, 0.13, 0.13],
        craft_id::CERAMIC => [0.68, 0.46, 0.36],
        craft_id::GLASS => [0.70, 0.82, 0.84],
        craft_id::IRON => [0.52, 0.53, 0.56],
        craft_id::SLAG => [0.32, 0.30, 0.30],
        craft_id::ASH => [0.72, 0.71, 0.68],
        craft_id::LIME => [0.90, 0.89, 0.84],
        craft_id::GRAVEL => [0.52, 0.48, 0.44],
        craft_id::DUST => [0.60, 0.56, 0.50],
        craft_id::SAND => [0.72, 0.62, 0.42],
        craft_id::MANURE => [0.34, 0.26, 0.18],
        craft_id::BONE_MEAL => [0.86, 0.84, 0.76],
        craft_id::FLOUR => [0.92, 0.89, 0.80],
        craft_id::OIL => [0.80, 0.68, 0.24],
        craft_id::BROTH => [0.66, 0.44, 0.20],
        craft_id::NOODLES => [0.88, 0.78, 0.50],
        craft_id::TARE => [0.34, 0.22, 0.16],
        craft_id::RAMEN => [0.82, 0.60, 0.34],
        craft_id::RICH_RAMEN => [0.88, 0.64, 0.32],
        craft_id::TEA => [0.58, 0.42, 0.22],
        craft_id::THATCH => [0.74, 0.64, 0.36],
        _ if (id as usize) < material::PALETTE.len() => material::albedo(id),
        _ => [0.46, 0.36, 0.26],
    }
}

pub fn bio_phys(id: u8) -> Phys {
    match id {
        bio_id::GREEN => Phys {
            bulk_kg_m3: 400.0,
            bulking: 1.0,
        },
        bio_id::WOOD => Phys {
            bulk_kg_m3: 650.0,
            bulking: 1.0,
        },
        bio_id::FIBRE => Phys {
            bulk_kg_m3: 300.0,
            bulking: 1.4,
        },
        bio_id::SEED => Phys {
            bulk_kg_m3: 550.0,
            bulking: 1.0,
        },
        bio_id::WATER => Phys {
            bulk_kg_m3: 1000.0,
            bulking: 1.0,
        },
        // Foraged parts. Bulking is what makes these awkward rather than heavy:
        // a pack fills on volume long before mass when you carry petals.
        bio_id::FLOWER => Phys {
            bulk_kg_m3: 180.0,
            bulking: 1.6,
        },
        bio_id::FRUIT => Phys {
            bulk_kg_m3: 700.0,
            bulking: 1.05,
        },
        bio_id::VEG => Phys {
            bulk_kg_m3: 600.0,
            bulking: 1.1,
        },
        bio_id::GRASS => Phys {
            bulk_kg_m3: 120.0,
            bulking: 1.8,
        },
        bio_id::LAVA => Phys {
            bulk_kg_m3: 2700.0,
            bulking: 1.0,
        },
        craft_id::TEA => Phys {
            bulk_kg_m3: 950.0,
            bulking: 1.0,
        },
        craft_id::THATCH => Phys {
            bulk_kg_m3: 160.0,
            bulking: 1.5,
        },
        craft_id::CHARCOAL => Phys {
            bulk_kg_m3: 250.0,
            bulking: 1.1,
        },
        craft_id::CERAMIC => Phys {
            bulk_kg_m3: 2000.0,
            bulking: 1.0,
        },
        craft_id::GLASS => Phys {
            bulk_kg_m3: 2500.0,
            bulking: 1.0,
        },
        craft_id::IRON => Phys {
            bulk_kg_m3: 7800.0,
            bulking: 1.0,
        },
        craft_id::SLAG => Phys {
            bulk_kg_m3: 1600.0,
            bulking: 1.2,
        },
        craft_id::ASH => Phys {
            bulk_kg_m3: 600.0,
            bulking: 1.3,
        },
        craft_id::LIME => Phys {
            bulk_kg_m3: 1200.0,
            bulking: 1.15,
        },
        craft_id::GRAVEL => Phys {
            bulk_kg_m3: 1600.0,
            bulking: 1.2,
        },
        craft_id::DUST => Phys {
            bulk_kg_m3: 1100.0,
            bulking: 1.4,
        },
        craft_id::SAND => Phys {
            bulk_kg_m3: 1600.0,
            bulking: 1.15,
        },
        craft_id::MANURE => Phys {
            bulk_kg_m3: 500.0,
            bulking: 1.2,
        },
        craft_id::BONE_MEAL => Phys {
            bulk_kg_m3: 900.0,
            bulking: 1.1,
        },
        craft_id::FLOUR => Phys {
            bulk_kg_m3: 600.0,
            bulking: 1.2,
        },
        craft_id::OIL => Phys {
            bulk_kg_m3: 920.0,
            bulking: 1.0,
        },
        craft_id::BROTH => Phys {
            bulk_kg_m3: 1000.0,
            bulking: 1.0,
        },
        craft_id::NOODLES => Phys {
            bulk_kg_m3: 700.0,
            bulking: 1.1,
        },
        craft_id::TARE => Phys {
            bulk_kg_m3: 1100.0,
            bulking: 1.0,
        },
        craft_id::RAMEN => Phys {
            bulk_kg_m3: 950.0,
            bulking: 1.0,
        },
        craft_id::RICH_RAMEN => Phys {
            bulk_kg_m3: 950.0,
            bulking: 1.0,
        },
        _ => phys(id),
    }
}

/// Convert plant carbon pools → loose harvest mass (item 813).
/// What is left of one organism after `taken_kg` of its woody and leaf material
/// has already been removed by any path.
///
/// The forest plan requires block excavation and whole-plant felling to consume
/// **one** ledger: chopping a trunk into blocks and then felling the remains
/// must not pay out the intact tree a second time. Only the above-ground woody
/// and leaf fractions are scaled down — roots and seed are not in the block
/// grid, so chopping branches has not touched them.
///
/// `taken_kg` comes from `woodscape::Woodscape::taken_kg`, which is durable:
/// a touched stand is never pruned and its ledger rides along in the save.
pub fn harvest_plant_remaining(p: &Plant, taken_kg: f32) -> DigYield {
    let full = harvest_plant(p);
    let above: f32 = full
        .parts
        .iter()
        .filter(|x| matches!(x.material_id, bio_id::WOOD | bio_id::GREEN))
        .map(|x| x.mass_kg)
        .sum();
    if above <= 1e-4 {
        return full;
    }
    let left = (1.0 - (taken_kg / above)).clamp(0.0, 1.0);
    if left >= 0.999 {
        return full;
    }
    let mut out = DigYield::default();
    for part in &full.parts {
        let woody = matches!(part.material_id, bio_id::WOOD | bio_id::GREEN);
        let k = if woody { left } else { 1.0 };
        if k <= 0.0 {
            continue;
        }
        out.push(YieldPart {
            material_id: part.material_id,
            volume_m3: part.volume_m3 * k,
            mass_kg: part.mass_kg * k,
            loose_m3: part.loose_m3 * k,
            grade: part.grade,
        });
    }
    out
}

/// Above-ground woody + leaf kilograms of an intact organism — the size of the
/// ledger entry that fully fells it.
pub fn plant_above_ground_kg(p: &Plant) -> f32 {
    harvest_plant(p)
        .parts
        .iter()
        .filter(|x| matches!(x.material_id, bio_id::WOOD | bio_id::GREEN))
        .map(|x| x.mass_kg)
        .sum()
}

pub fn harvest_plant(p: &Plant) -> DigYield {
    // Form from the genome, matching `tree_form`'s own `genome_id % 8`, so a
    // caller that has not yet plumbed the real form still gets the right kind
    // of produce off the right silhouette.
    harvest_plant_as(p, (p.genome_id % 8) as u8)
}

/// What one plant gives when you take it, split by what the plant *is*.
///
/// The soft-tissue pool is **divided, not multiplied**: every form below hands
/// back the same total leaf mass, apportioned differently. Foraging is meant to
/// become a choice about where you walk, not a larger number for the same walk,
/// and the mass-balance test downstream depends on that restraint.
///
/// `form` is a `tree_form` id — 0 conifer, 1 broadleaf, 2 willow, 3 scrub,
/// 4 reed, 5 orchard/farm, 6 acacia, 7 giant.
pub fn harvest_plant_as(p: &Plant, form: u8) -> DigYield {
    let mut y = DigYield::default();
    // Pools are dimensionless carbon; scale so a mature stand yields kilograms.
    // Timber is the mineable bulk — leaf/fibre ride along as bycatch.
    const KG: f32 = 12.0;
    let leaf_kg = p.leaf * KG * 1.4;

    // Shares of the soft-tissue pool: (greens, flowers, fruit, veg, grass).
    // Each row sums to 1.0 — that is the invariant, and `harvest_splits_sum`
    // asserts it rather than trusting the arithmetic to stay right by eye.
    let (green, flower, fruit, veg, grass) = match form {
        0 => (0.85, 0.00, 0.00, 0.00, 0.15), // conifer — needles, little else
        1 => (0.55, 0.10, 0.25, 0.00, 0.10), // broadleaf — some fruit
        2 => (0.70, 0.10, 0.00, 0.00, 0.20), // willow — withies, no fruit
        3 => (0.25, 0.30, 0.05, 0.00, 0.40), // scrub — flowers and dry grass
        4 => (0.20, 0.05, 0.00, 0.00, 0.75), // reed — essentially all grass
        5 => (0.10, 0.10, 0.45, 0.35, 0.00), // orchard / farm — the larder
        6 => (0.45, 0.35, 0.05, 0.00, 0.15), // acacia — heavy bloom
        _ => (0.75, 0.05, 0.10, 0.00, 0.10), // giant and anything new
    };

    let parts = [
        (bio_id::FIBRE, p.root * KG * 0.85),
        (bio_id::WOOD, p.stem * KG * 5.5),
        (bio_id::SEED, p.repro * KG * 1.2),
        (bio_id::GREEN, leaf_kg * green),
        (bio_id::FLOWER, leaf_kg * flower),
        (bio_id::FRUIT, leaf_kg * fruit),
        (bio_id::VEG, leaf_kg * veg),
        (bio_id::GRASS, leaf_kg * grass),
    ];
    for (id, mass) in parts {
        if mass < 0.05 {
            continue;
        }
        let ph = bio_phys(id);
        let vol = mass / ph.bulk_kg_m3.max(1.0);
        y.push(YieldPart {
            material_id: id,
            volume_m3: vol,
            mass_kg: mass,
            loose_m3: vol * ph.bulking,
            grade: 0.0,
        });
    }
    y
}

/// The soft-tissue shares every form apportions its leaf pool by.
/// Exposed so the invariant can be tested rather than eyeballed.
pub fn soft_tissue_shares(form: u8) -> [f32; 5] {
    let p = Plant {
        leaf: 1.0 / (12.0 * 1.4),
        root: 0.0,
        stem: 0.0,
        repro: 0.0,
        ..Default::default()
    };
    let y = harvest_plant_as(&p, form);
    let of = |id: u8| {
        y.parts
            .iter()
            .filter(|x| x.material_id == id)
            .map(|x| x.mass_kg)
            .sum::<f32>()
    };
    [
        of(bio_id::GREEN),
        of(bio_id::FLOWER),
        of(bio_id::FRUIT),
        of(bio_id::VEG),
        of(bio_id::GRASS),
    ]
}

// ---------------------------------------------------------------------------
// Inventory + stockpiles (items 820–822)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct Stack {
    pub material_id: u8,
    pub mass_kg: f32,
    pub loose_m3: f32,
    pub grade: f32,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    pub stacks: Vec<Stack>,
    pub max_mass_kg: f32,
    pub max_volume_m3: f32,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            stacks: Vec::new(),
            // Room for a couple felled trunks plus a dig bite — timber is bulky.
            max_mass_kg: 90.0,
            max_volume_m3: 0.18,
        }
    }
}

impl Inventory {
    pub fn mass_kg(&self) -> f32 {
        self.stacks.iter().map(|s| s.mass_kg).sum()
    }

    pub fn volume_m3(&self) -> f32 {
        self.stacks.iter().map(|s| s.loose_m3).sum()
    }

    pub fn mass_frac(&self) -> f32 {
        (self.mass_kg() / self.max_mass_kg.max(1e-6)).clamp(0.0, 2.0)
    }

    pub fn volume_frac(&self) -> f32 {
        (self.volume_m3() / self.max_volume_m3.max(1e-6)).clamp(0.0, 2.0)
    }

    /// Encumbrance multiplier on walk/jump (item 821). Uses spin-gravity so
    /// the same load weighs less toward the axis.
    pub fn encumbrance(&self, gravity: f32) -> f32 {
        let load = self.mass_frac().max(self.volume_frac());
        let g_scale = (gravity / 9.81).clamp(0.35, 1.2);
        let burden = (load * g_scale).clamp(0.0, 1.5);
        (1.0 - 0.55 * burden).clamp(0.35, 1.0)
    }

    /// Try to add a yield. Returns the accepted fraction of mass (0..1).
    pub fn try_add(&mut self, y: &DigYield) -> f32 {
        if y.total_mass_kg <= 1e-6 {
            return 1.0;
        }
        let room_m = (self.max_mass_kg - self.mass_kg()).max(0.0);
        let room_v = (self.max_volume_m3 - self.volume_m3()).max(0.0);
        let frac_m = if y.total_mass_kg > 1e-6 {
            room_m / y.total_mass_kg
        } else {
            1.0
        };
        let frac_v = if y.total_loose_m3 > 1e-6 {
            room_v / y.total_loose_m3
        } else {
            1.0
        };
        let frac = frac_m.min(frac_v).clamp(0.0, 1.0);
        if frac <= 1e-6 {
            return 0.0;
        }
        for part in &y.parts {
            self.add_stack(
                part.material_id,
                part.mass_kg * frac,
                part.loose_m3 * frac,
                part.grade,
            );
        }
        frac
    }

    pub fn add_stack(&mut self, material_id: u8, mass_kg: f32, loose_m3: f32, grade: f32) {
        if mass_kg <= 1e-6 {
            return;
        }
        if let Some(s) = self
            .stacks
            .iter_mut()
            .find(|s| s.material_id == material_id)
        {
            let m0 = s.mass_kg;
            s.grade = if m0 + mass_kg > 1e-6 {
                (s.grade * m0 + grade * mass_kg) / (m0 + mass_kg)
            } else {
                grade
            };
            s.mass_kg += mass_kg;
            s.loose_m3 += loose_m3;
        } else {
            self.stacks.push(Stack {
                material_id,
                mass_kg,
                loose_m3,
                grade,
            });
        }
    }

    /// Drop everything as one mixed stockpile payload. Clears the pack.
    pub fn take_all(&mut self) -> DigYield {
        let mut y = DigYield::default();
        for s in self.stacks.drain(..) {
            y.push(YieldPart {
                material_id: s.material_id,
                volume_m3: s.loose_m3 / bio_phys(s.material_id).bulking.max(1.0),
                mass_kg: s.mass_kg,
                loose_m3: s.loose_m3,
                grade: s.grade,
            });
        }
        y
    }

    pub fn mass_of(&self, material_id: u8) -> f32 {
        self.stacks
            .iter()
            .filter(|s| s.material_id == material_id)
            .map(|s| s.mass_kg)
            .sum()
    }

    /// Remove up to `mass_kg` of a material. Returns actual mass taken.
    pub fn take_mass(&mut self, material_id: u8, mass_kg: f32) -> f32 {
        let need = mass_kg.max(0.0);
        if need <= 1e-9 {
            return 0.0;
        }
        let mut left = need;
        let mut i = 0;
        while i < self.stacks.len() {
            if self.stacks[i].material_id != material_id {
                i += 1;
                continue;
            }
            let take = self.stacks[i].mass_kg.min(left);
            if take <= 1e-9 {
                i += 1;
                continue;
            }
            let frac = take / self.stacks[i].mass_kg.max(1e-9);
            self.stacks[i].mass_kg -= take;
            self.stacks[i].loose_m3 = (self.stacks[i].loose_m3 * (1.0 - frac)).max(0.0);
            left -= take;
            if self.stacks[i].mass_kg < 1e-4 {
                self.stacks.remove(i);
            } else {
                i += 1;
            }
            if left <= 1e-9 {
                break;
            }
        }
        need - left
    }
}

#[derive(Clone, Debug)]
pub struct Stockpile {
    pub theta: f32,
    pub z: f32,
    pub material_id: u8,
    pub mass_kg: f32,
    pub loose_m3: f32,
    pub grade: f32,
}

impl Stockpile {
    /// Heap radius grows with loose volume (visible spoil, item 822).
    pub fn radius(&self) -> f32 {
        // Conical spoil heap: V ≈ π r³ / 3 → r = cbrt(3V/π).
        (self.loose_m3 * 3.0 / std::f32::consts::PI)
            .cbrt()
            .clamp(0.25, 4.0)
    }
}

/// Take from the nearest heap into the pack, up to whatever room is left.
///
/// Without this, dropping or spilling is one-way: mass stays in the ledger as
/// "heaps" but leaves play permanently, which a closed system cannot afford.
/// Returns (material_id, mass taken) when something moved.
pub fn take_from_heap(
    heaps: &mut Vec<Stockpile>,
    inv: &mut Inventory,
    hab_radius: f32,
    theta: f32,
    z: f32,
    reach: f32,
) -> Option<(u8, f32)> {
    let room_mass = (inv.max_mass_kg - inv.mass_kg()).max(0.0);
    let room_vol = (inv.max_volume_m3 - inv.volume_m3()).max(0.0);
    if room_mass < 0.05 || room_vol < 1e-5 {
        return None;
    }

    // Nearest heap within reach, measured along the surface.
    let mut best: Option<(usize, f32)> = None;
    for (i, h) in heaps.iter().enumerate() {
        let dth = {
            let x = (h.theta - theta).rem_euclid(std::f32::consts::TAU);
            let x = x.min(std::f32::consts::TAU - x);
            x * hab_radius
        };
        let dz = h.z - z;
        let d2 = dth * dth + dz * dz;
        if d2 <= reach * reach && best.map_or(true, |(_, b)| d2 < b) {
            best = Some((i, d2));
        }
    }
    let (idx, _) = best?;

    let h = &mut heaps[idx];
    if h.mass_kg <= 1e-4 {
        heaps.remove(idx);
        return None;
    }
    // Take the largest share that fits BOTH limits, keeping the heap's own
    // mass-to-volume ratio so density stays consistent.
    let by_mass = (room_mass / h.mass_kg).min(1.0);
    let by_vol = if h.loose_m3 > 1e-6 {
        (room_vol / h.loose_m3).min(1.0)
    } else {
        1.0
    };
    let frac = by_mass.min(by_vol).clamp(0.0, 1.0);
    let take_m = h.mass_kg * frac;
    let take_v = h.loose_m3 * frac;
    if take_m < 0.02 {
        return None;
    }
    let mid = h.material_id;
    let grade = h.grade;
    h.mass_kg -= take_m;
    h.loose_m3 -= take_v;
    if h.mass_kg <= 0.02 {
        heaps.remove(idx);
    }
    inv.add_stack(mid, take_m, take_v, grade);
    Some((mid, take_m))
}

/// Merge spill into a nearby same-material heap, or push a new one.
/// Rapid dig with a full pack used to spawn one sphere per bite.
pub fn deposit_heap(
    heaps: &mut Vec<Stockpile>,
    hab_radius: f32,
    theta: f32,
    z: f32,
    material_id: u8,
    mass_kg: f32,
    loose_m3: f32,
    grade: f32,
) {
    if mass_kg < 0.02 {
        return;
    }
    const MERGE_M: f32 = 3.5;
    let merge_r2 = MERGE_M * MERGE_M;
    for h in heaps.iter_mut() {
        if h.material_id != material_id {
            continue;
        }
        let dth = {
            let x = (h.theta - theta).rem_euclid(std::f32::consts::TAU);
            let x = x.min(std::f32::consts::TAU - x);
            x * hab_radius
        };
        let dz = h.z - z;
        if dth * dth + dz * dz <= merge_r2 {
            let m0 = h.mass_kg;
            h.grade = if m0 + mass_kg > 1e-6 {
                (h.grade * m0 + grade * mass_kg) / (m0 + mass_kg)
            } else {
                grade
            };
            h.mass_kg += mass_kg;
            h.loose_m3 += loose_m3;
            // Keep the pile centred on the bulk of the mass.
            let w = (m0 / (m0 + mass_kg)).clamp(0.0, 1.0);
            h.theta = h.theta * w + theta * (1.0 - w);
            h.z = h.z * w + z * (1.0 - w);
            return;
        }
    }
    heaps.push(Stockpile {
        theta,
        z,
        material_id,
        mass_kg,
        loose_m3,
        grade,
    });
    // Hard cap — if somehow still exploding, fold the smallest into its nearest neighbour.
    const CAP: usize = 36;
    while heaps.len() > CAP {
        let mut smallest = 0usize;
        for i in 1..heaps.len() {
            if heaps[i].mass_kg < heaps[smallest].mass_kg {
                smallest = i;
            }
        }
        let victim = heaps.swap_remove(smallest);
        if heaps.is_empty() {
            heaps.push(victim);
            break;
        }
        let mut best = 0usize;
        let mut best_d = f32::MAX;
        for (i, h) in heaps.iter().enumerate() {
            let dth = {
                let x = (h.theta - victim.theta).rem_euclid(std::f32::consts::TAU);
                let x = x.min(std::f32::consts::TAU - x);
                x * hab_radius
            };
            let dz = h.z - victim.z;
            let d2 = dth * dth + dz * dz;
            if d2 < best_d {
                best_d = d2;
                best = i;
            }
        }
        let host = &mut heaps[best];
        let m0 = host.mass_kg;
        host.grade = if m0 + victim.mass_kg > 1e-6 {
            (host.grade * m0 + victim.grade * victim.mass_kg) / (m0 + victim.mass_kg)
        } else {
            victim.grade
        };
        host.mass_kg += victim.mass_kg;
        host.loose_m3 += victim.loose_m3;
    }
}

// ---------------------------------------------------------------------------
// Atmosphere ledger (items 843–844)
// ---------------------------------------------------------------------------

/// Sealed-drum breathable pool. Masses are kilograms of gas.
#[derive(Clone, Debug)]
pub struct Atmosphere {
    pub o2_kg: f32,
    pub co2_kg: f32,
    pub n2_kg: f32,
    /// Scrubber power currently allocated (MW) — converts CO₂→O₂ slowly.
    pub scrub_mw: f32,
}

impl Default for Atmosphere {
    fn default() -> Self {
        // Rough free-air inventory for a sealed habitat colony zone — finite
        // and noticeable when industry scales (item 843).
        Self {
            o2_kg: 180_000.0,
            co2_kg: 600.0,
            n2_kg: 680_000.0,
            scrub_mw: 0.0,
        }
    }
}

impl Atmosphere {
    pub fn o2_frac(&self) -> f32 {
        let t = (self.o2_kg + self.co2_kg + self.n2_kg).max(1.0);
        self.o2_kg / t
    }

    pub fn co2_ppm(&self) -> f32 {
        let t = (self.o2_kg + self.co2_kg + self.n2_kg).max(1.0);
        self.co2_kg / t * 1.0e6
    }

    /// Apply combustion / smelting arithmetic (item 843).
    pub fn apply_combustion(&mut self, o2_consume: f32, co2_emit: f32) -> bool {
        if o2_consume > self.o2_kg + 1e-3 {
            return false;
        }
        self.o2_kg = (self.o2_kg - o2_consume).max(0.0);
        self.co2_kg += co2_emit.max(0.0);
        true
    }

    /// Atmospheric scrubbing: power buys O₂ back from CO₂ (item 844).
    /// ~0.15 kg CO₂ scrubbed per MW per day → O₂ returned at 32/44 stoichiometry.
    pub fn tick_scrub(&mut self, dt_days: f32, power_mw: f32) {
        let rate = power_mw.max(0.0) * 0.15 * dt_days.max(0.0);
        let take = rate.min(self.co2_kg);
        self.co2_kg -= take;
        self.o2_kg += take * (32.0 / 44.0);
        self.scrub_mw = power_mw.max(0.0);
    }
}

// ---------------------------------------------------------------------------
// Craft runner (items 839–855)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct CraftReport {
    pub recipe_id: &'static str,
    pub scale: f32,
    pub produced: DigYield,
    pub spilled: DigYield,
    pub o2_used: f32,
    pub co2_made: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CraftError {
    UnknownRecipe,
    MissingInputs,
    InsufficientOxygen,
    InvalidScale,
}

#[derive(Clone, Copy, Debug)]
pub struct RecipeIO {
    pub material: &'static str,
    pub mass_kg: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub id: &'static str,
    pub station: &'static str,
    pub energy_kj: f32,
    pub time_s: f32,
    /// O₂ consumed per full batch (item 843).
    pub o2_kg: f32,
    /// CO₂ emitted per full batch.
    pub co2_kg: f32,
    pub inputs: &'static [RecipeIO],
    pub outputs: &'static [RecipeIO],
}

/// Authored recipes — hot-reload path later; mass balance enforced by test.
pub const RECIPES: &[Recipe] = &[
    Recipe {
        id: "crush_rock",
        station: "crusher",
        energy_kj: 40.0,
        time_s: 8.0,
        o2_kg: 0.0,
        co2_kg: 0.0,
        inputs: &[RecipeIO {
            material: "sandstone",
            mass_kg: 10.0,
        }],
        outputs: &[
            RecipeIO {
                material: "gravel",
                mass_kg: 9.4,
            },
            RecipeIO {
                material: "dust",
                mass_kg: 0.6,
            },
        ],
    },
    Recipe {
        id: "mill_sand",
        station: "crusher",
        energy_kj: 30.0,
        time_s: 10.0,
        o2_kg: 0.0,
        co2_kg: 0.0,
        inputs: &[RecipeIO {
            material: "gravel",
            mass_kg: 8.0,
        }],
        outputs: &[
            RecipeIO {
                material: "sand",
                mass_kg: 7.5,
            },
            RecipeIO {
                material: "dust",
                mass_kg: 0.5,
            },
        ],
    },
    Recipe {
        id: "charcoal_kiln",
        station: "kiln",
        energy_kj: 20.0,
        time_s: 90.0,
        o2_kg: 1.2,
        co2_kg: 2.8,
        inputs: &[RecipeIO {
            material: "timber",
            mass_kg: 10.0,
        }],
        outputs: &[
            RecipeIO {
                material: "charcoal",
                mass_kg: 2.5,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 7.5,
            },
        ],
    },
    Recipe {
        id: "fire_clay",
        station: "kiln",
        energy_kj: 120.0,
        time_s: 45.0,
        o2_kg: 2.5,
        co2_kg: 3.4,
        inputs: &[
            RecipeIO {
                material: "clay",
                mass_kg: 8.0,
            },
            RecipeIO {
                material: "charcoal",
                mass_kg: 1.5,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "ceramic",
                mass_kg: 7.2,
            },
            RecipeIO {
                material: "ash",
                mass_kg: 0.4,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 1.9,
            },
        ],
    },
    Recipe {
        id: "smelt_ferrous",
        station: "smelter",
        energy_kj: 800.0,
        time_s: 120.0,
        o2_kg: 8.0,
        co2_kg: 11.0,
        inputs: &[
            RecipeIO {
                material: "ferrous ore",
                mass_kg: 20.0,
            },
            RecipeIO {
                material: "charcoal",
                mass_kg: 5.0,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "iron",
                mass_kg: 8.0,
            },
            RecipeIO {
                material: "slag",
                mass_kg: 12.5,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 4.5,
            },
        ],
    },
    Recipe {
        id: "burn_lime",
        station: "kiln",
        energy_kj: 200.0,
        time_s: 60.0,
        o2_kg: 3.0,
        co2_kg: 5.5,
        inputs: &[
            RecipeIO {
                material: "sediment",
                mass_kg: 10.0,
            },
            RecipeIO {
                material: "charcoal",
                mass_kg: 2.0,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "lime",
                mass_kg: 5.6,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 6.4,
            },
        ],
    },
    Recipe {
        id: "melt_glass",
        station: "kiln",
        energy_kj: 400.0,
        time_s: 90.0,
        o2_kg: 4.0,
        co2_kg: 5.5,
        inputs: &[
            RecipeIO {
                material: "sand",
                mass_kg: 8.0,
            },
            RecipeIO {
                material: "lime",
                mass_kg: 1.5,
            },
            RecipeIO {
                material: "charcoal",
                mass_kg: 2.0,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "glass",
                mass_kg: 8.8,
            },
            RecipeIO {
                material: "ash",
                mass_kg: 0.5,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 2.2,
            },
        ],
    },
    Recipe {
        id: "compost_greens",
        station: "compost",
        energy_kj: 0.0,
        time_s: 30.0,
        o2_kg: 0.4,
        co2_kg: 0.55,
        inputs: &[RecipeIO {
            material: "greens",
            mass_kg: 5.0,
        }],
        outputs: &[
            RecipeIO {
                material: "manure",
                mass_kg: 3.8,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 1.2,
            },
        ],
    },
    Recipe {
        id: "bone_meal_mix",
        station: "mill",
        energy_kj: 10.0,
        time_s: 15.0,
        o2_kg: 0.0,
        co2_kg: 0.0,
        inputs: &[
            RecipeIO {
                material: "seed",
                mass_kg: 2.0,
            },
            RecipeIO {
                material: "ash",
                mass_kg: 1.0,
            },
        ],
        outputs: &[RecipeIO {
            material: "bone meal",
            mass_kg: 3.0,
        }],
    },
    // --- Cooking / ramen (garden → bowl) ---
    Recipe {
        id: "mill_flour",
        station: "kitchen",
        energy_kj: 15.0,
        time_s: 20.0,
        o2_kg: 0.0,
        co2_kg: 0.0,
        inputs: &[RecipeIO {
            material: "seed",
            mass_kg: 4.0,
        }],
        outputs: &[
            RecipeIO {
                material: "flour",
                mass_kg: 3.6,
            },
            RecipeIO {
                material: "dust",
                mass_kg: 0.4,
            },
        ],
    },
    Recipe {
        id: "press_oil",
        station: "kitchen",
        energy_kj: 10.0,
        time_s: 25.0,
        o2_kg: 0.0,
        co2_kg: 0.0,
        inputs: &[RecipeIO {
            material: "seed",
            mass_kg: 5.0,
        }],
        outputs: &[
            RecipeIO {
                material: "oil",
                mass_kg: 1.5,
            },
            RecipeIO {
                material: "manure",
                mass_kg: 3.5,
            }, // press cake → feed/compost
        ],
    },
    Recipe {
        id: "simmer_broth",
        station: "kitchen",
        energy_kj: 40.0,
        time_s: 60.0,
        o2_kg: 0.3,
        co2_kg: 0.4,
        inputs: &[
            RecipeIO {
                material: "greens",
                mass_kg: 3.0,
            },
            RecipeIO {
                material: "bone meal",
                mass_kg: 1.0,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "broth",
                mass_kg: 3.5,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 0.5,
            },
        ],
    },
    Recipe {
        id: "roll_noodles",
        station: "kitchen",
        energy_kj: 5.0,
        time_s: 30.0,
        o2_kg: 0.0,
        co2_kg: 0.0,
        inputs: &[
            RecipeIO {
                material: "flour",
                mass_kg: 3.0,
            },
            RecipeIO {
                material: "oil",
                mass_kg: 0.2,
            },
        ],
        outputs: &[RecipeIO {
            material: "noodles",
            mass_kg: 3.2,
        }],
    },
    Recipe {
        id: "reduce_tare",
        station: "kitchen",
        energy_kj: 20.0,
        time_s: 40.0,
        o2_kg: 0.2,
        co2_kg: 0.25,
        inputs: &[
            RecipeIO {
                material: "greens",
                mass_kg: 2.0,
            },
            RecipeIO {
                material: "ash",
                mass_kg: 0.3,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "tare",
                mass_kg: 1.8,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 0.5,
            },
        ],
    },
    Recipe {
        id: "bowl_ramen",
        station: "kitchen",
        energy_kj: 25.0,
        time_s: 15.0,
        o2_kg: 0.1,
        co2_kg: 0.1,
        inputs: &[
            RecipeIO {
                material: "noodles",
                mass_kg: 1.5,
            },
            RecipeIO {
                material: "broth",
                mass_kg: 2.0,
            },
            RecipeIO {
                material: "greens",
                mass_kg: 0.5,
            },
        ],
        outputs: &[RecipeIO {
            material: "ramen",
            mass_kg: 4.0,
        }],
    },
    Recipe {
        id: "bowl_rich_ramen",
        station: "kitchen",
        energy_kj: 35.0,
        time_s: 20.0,
        o2_kg: 0.15,
        co2_kg: 0.15,
        inputs: &[
            RecipeIO {
                material: "noodles",
                mass_kg: 1.5,
            },
            RecipeIO {
                material: "broth",
                mass_kg: 2.0,
            },
            RecipeIO {
                material: "greens",
                mass_kg: 0.8,
            },
            RecipeIO {
                material: "oil",
                mass_kg: 0.3,
            },
            RecipeIO {
                material: "tare",
                mass_kg: 0.4,
            },
        ],
        outputs: &[RecipeIO {
            material: "rich ramen",
            mass_kg: 5.0,
        }],
    },
    // Roots make more broth per kilogram than leaves do — the reason to
    // keep a plot rather than forage the hedgerow.
    Recipe {
        id: "simmer_veg_broth",
        station: "kitchen",
        energy_kj: 40.0,
        time_s: 60.0,
        o2_kg: 0.30,
        co2_kg: 0.40,
        inputs: &[
            RecipeIO {
                material: "vegetables",
                mass_kg: 2.5,
            },
            RecipeIO {
                material: "bone meal",
                mass_kg: 1.0,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "broth",
                mass_kg: 3.0,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 0.5,
            },
        ],
    },
    // Fruit reduces further than greens: same step, better tare.
    Recipe {
        id: "fruit_tare",
        station: "kitchen",
        energy_kj: 20.0,
        time_s: 40.0,
        o2_kg: 0.20,
        co2_kg: 0.25,
        inputs: &[
            RecipeIO {
                material: "fruit",
                mass_kg: 2.0,
            },
            RecipeIO {
                material: "ash",
                mass_kg: 0.3,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "tare",
                mass_kg: 1.9,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 0.4,
            },
        ],
    },
    // A second route to the good bowl. `bowl_rich_ramen` wants greens,
    // which every plant gives; this one wants what you had to grow.
    Recipe {
        id: "bowl_veg_ramen",
        station: "kitchen",
        energy_kj: 35.0,
        time_s: 20.0,
        o2_kg: 0.15,
        co2_kg: 0.15,
        inputs: &[
            RecipeIO {
                material: "noodles",
                mass_kg: 1.5,
            },
            RecipeIO {
                material: "broth",
                mass_kg: 2.0,
            },
            RecipeIO {
                material: "vegetables",
                mass_kg: 0.8,
            },
            RecipeIO {
                material: "oil",
                mass_kg: 0.3,
            },
            RecipeIO {
                material: "tare",
                mass_kg: 0.4,
            },
        ],
        outputs: &[RecipeIO {
            material: "rich ramen",
            mass_kg: 5.0,
        }],
    },
    // Somewhere for a bloom to go that is not the compost heap.
    Recipe {
        id: "steep_tea",
        station: "kitchen",
        energy_kj: 12.0,
        time_s: 45.0,
        o2_kg: 0.00,
        co2_kg: 0.05,
        inputs: &[
            RecipeIO {
                material: "flowers",
                mass_kg: 0.6,
            },
            RecipeIO {
                material: "water",
                mass_kg: 2.0,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "tea",
                mass_kg: 2.4,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 0.2,
            },
        ],
    },
    // Grass is bulky and near-weightless, so it is the one harvest that
    // fills a pack on volume. Bundling is what makes it carryable.
    Recipe {
        id: "thatch_bundle",
        station: "mill",
        energy_kj: 8.0,
        time_s: 25.0,
        o2_kg: 0.00,
        co2_kg: 0.00,
        inputs: &[
            RecipeIO {
                material: "grass",
                mass_kg: 6.0,
            },
            RecipeIO {
                material: "fibre",
                mass_kg: 1.0,
            },
        ],
        outputs: &[RecipeIO {
            material: "thatch",
            mass_kg: 7.0,
        }],
    },
    // Dug coal beats made charcoal as a reductant — less of it, more iron,
    // and no wood spent. It costs more atmosphere, which is the trade.
    Recipe {
        id: "smelt_ferrous_coal",
        station: "smelter",
        energy_kj: 760.0,
        time_s: 110.0,
        o2_kg: 9.00,
        co2_kg: 13.00,
        inputs: &[
            RecipeIO {
                material: "ferrous ore",
                mass_kg: 20.0,
            },
            RecipeIO {
                material: "coal",
                mass_kg: 4.0,
            },
        ],
        outputs: &[
            RecipeIO {
                material: "iron",
                mass_kg: 8.6,
            },
            RecipeIO {
                material: "slag",
                mass_kg: 12.0,
            },
            RecipeIO {
                material: "gas_loss",
                mass_kg: 3.4,
            },
        ],
    },
    // Where molten rock comes from. Nothing in a built drum melts on its
    // own, so the smelter is the only honest source of it.
    Recipe {
        id: "melt_basalt",
        station: "smelter",
        energy_kj: 1400.0,
        time_s: 180.0,
        o2_kg: 1.00,
        co2_kg: 1.40,
        inputs: &[RecipeIO {
            material: "basalt",
            mass_kg: 8.0,
        }],
        outputs: &[RecipeIO {
            material: "molten rock",
            mass_kg: 8.0,
        }],
    },
    // Cools back to stone you can place. Turns ore-poor spoil into
    // building rock instead of another heap.
    Recipe {
        id: "cast_basalt",
        station: "kiln",
        energy_kj: 30.0,
        time_s: 240.0,
        o2_kg: 0.00,
        co2_kg: 0.00,
        inputs: &[RecipeIO {
            material: "molten rock",
            mass_kg: 8.0,
        }],
        outputs: &[RecipeIO {
            material: "basalt",
            mass_kg: 8.0,
        }],
    },
];

/// Mass in must equal mass out within tolerance (item 832).
pub fn recipe_mass_ok(r: &Recipe, eps: f32) -> bool {
    let inn: f32 = r.inputs.iter().map(|i| i.mass_kg).sum();
    let out: f32 = r.outputs.iter().map(|o| o.mass_kg).sum();
    (inn - out).abs() <= eps
}

pub fn recipe_by_id(id: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|r| r.id == id)
}

pub fn recipe_by_index(i: usize) -> Option<&'static Recipe> {
    RECIPES.get(i)
}

/// Run a scaled batch: consume pack inputs, emit products, debit atmosphere.
pub fn craft(
    inv: &mut Inventory,
    atmo: &mut Atmosphere,
    recipe: &Recipe,
    scale: f32,
) -> Result<CraftReport, CraftError> {
    let scale = if scale.is_finite() { scale } else { 0.0 };
    if scale <= 1e-6 || scale > 1.0 + 1e-4 {
        return Err(CraftError::InvalidScale);
    }
    // Verify inputs.
    for io in recipe.inputs {
        let Some(mid) = material_id_by_name(io.material) else {
            return Err(CraftError::UnknownRecipe);
        };
        if inv.mass_of(mid) + 1e-4 < io.mass_kg * scale {
            return Err(CraftError::MissingInputs);
        }
    }
    let o2 = recipe.o2_kg * scale;
    let co2 = recipe.co2_kg * scale;
    if o2 > atmo.o2_kg + 1e-3 {
        return Err(CraftError::InsufficientOxygen);
    }
    if o2 > 1e-6 || co2 > 1e-6 {
        let _ = atmo.apply_combustion(o2, co2);
    }

    // Consume inputs and track mean grade for cooking quality.
    let mut grade_acc = 0.0f32;
    let mut grade_w = 0.0f32;
    for io in recipe.inputs {
        let mid = material_id_by_name(io.material).unwrap();
        let need = io.mass_kg * scale;
        // Grade from remaining stack before take.
        if let Some(s) = inv.stacks.iter().find(|s| s.material_id == mid) {
            grade_acc += s.grade * need;
            grade_w += need;
        }
        let got = inv.take_mass(mid, need);
        debug_assert!((got - need).abs() < 0.05);
    }
    let out_grade = if grade_w > 1e-6 {
        (grade_acc / grade_w).clamp(0.0, 1.0)
    } else {
        0.0
    };
    // Kitchen recipes inherit ingredient grade; rich ramen gets a bump.
    let out_grade = if recipe.id.contains("ramen") {
        (out_grade * 0.85 + 0.15).min(1.0)
    } else {
        out_grade
    };

    let mut produced = DigYield::default();
    for io in recipe.outputs {
        if io.material == "gas_loss" {
            continue; // already counted via o2/co2 / recipe mass balance
        }
        let Some(mid) = material_id_by_name(io.material) else {
            continue;
        };
        let mass = io.mass_kg * scale;
        let ph = bio_phys(mid);
        let vol = mass / ph.bulk_kg_m3.max(1.0);
        produced.push(YieldPart {
            material_id: mid,
            volume_m3: vol,
            mass_kg: mass,
            loose_m3: vol * ph.bulking,
            grade: out_grade,
        });
    }
    let accepted = inv.try_add(&produced);
    let mut spilled = DigYield::default();
    if accepted < 0.999 {
        let f = 1.0 - accepted;
        for p in &produced.parts {
            spilled.push(YieldPart {
                material_id: p.material_id,
                volume_m3: p.volume_m3 * f,
                mass_kg: p.mass_kg * f,
                loose_m3: p.loose_m3 * f,
                grade: 0.0,
            });
        }
    }
    Ok(CraftReport {
        recipe_id: recipe.id,
        scale,
        produced,
        spilled,
        o2_used: o2,
        co2_made: co2,
    })
}

/// Largest scale ≤ 1 that the pack + atmosphere can support.
pub fn max_craft_scale(inv: &Inventory, atmo: &Atmosphere, recipe: &Recipe) -> f32 {
    let mut s = 1.0f32;
    for io in recipe.inputs {
        let Some(mid) = material_id_by_name(io.material) else {
            return 0.0;
        };
        if io.mass_kg <= 1e-9 {
            continue;
        }
        s = s.min(inv.mass_of(mid) / io.mass_kg);
    }
    if recipe.o2_kg > 1e-6 {
        s = s.min(atmo.o2_kg / recipe.o2_kg);
    }
    s.clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Soil amendments (items 855, 869–871)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct AmendEffect {
    pub n: f32,
    pub p: f32,
    pub k: f32,
    pub organic: f32,
    pub ph: f32,
}

pub fn amendment_effect(material_id: u8) -> Option<AmendEffect> {
    match material_id {
        craft_id::ASH => Some(AmendEffect {
            n: 0.0,
            p: 0.02,
            k: 0.18,
            organic: 0.0,
            ph: 0.08,
        }),
        craft_id::LIME => Some(AmendEffect {
            n: 0.0,
            p: 0.0,
            k: 0.02,
            organic: 0.0,
            ph: 0.22,
        }),
        craft_id::MANURE => Some(AmendEffect {
            n: 0.16,
            p: 0.04,
            k: 0.06,
            organic: 0.12,
            ph: 0.0,
        }),
        craft_id::BONE_MEAL => Some(AmendEffect {
            n: 0.02,
            p: 0.22,
            k: 0.0,
            organic: 0.04,
            ph: 0.02,
        }),
        _ => None,
    }
}

/// Greenhouse placement cost in kg of glass (item 854).
pub const GREENHOUSE_GLASS_KG: f32 = 12.0;

#[derive(Clone, Copy, Debug)]
pub struct Greenhouse {
    pub theta: f32,
    pub z: f32,
    pub radius: f32,
}

/// Embedded TOML twin of [`RECIPES`] — designers edit this; CI checks parity.
pub const RECIPES_TOML: &str = include_str!("../data/recipes.toml");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habitat::Habitat;
    use crate::terrain::Terrain;

    #[test]
    fn deposit_heap_merges_nearby_same_material() {
        let mut heaps = Vec::new();
        deposit_heap(&mut heaps, 900.0, 0.0, 0.0, 1, 10.0, 0.5, 0.0);
        deposit_heap(&mut heaps, 900.0, 0.001, 0.5, 1, 5.0, 0.25, 0.0);
        assert_eq!(heaps.len(), 1);
        assert!((heaps[0].mass_kg - 15.0).abs() < 1e-3);
        // Far away → new heap.
        deposit_heap(&mut heaps, 900.0, 1.0, 40.0, 1, 3.0, 0.1, 0.0);
        assert_eq!(heaps.len(), 2);
    }

    #[test]
    fn recipes_balance_mass() {
        for r in RECIPES {
            assert!(
                recipe_mass_ok(r, 0.05),
                "recipe '{}' mass imbalance in={:.3} out={:.3}",
                r.id,
                r.inputs.iter().map(|i| i.mass_kg).sum::<f32>(),
                r.outputs.iter().map(|o| o.mass_kg).sum::<f32>(),
            );
        }
    }

    #[test]
    fn recipes_toml_lists_every_id() {
        for r in RECIPES {
            assert!(
                RECIPES_TOML.contains(&format!("id = \"{}\"", r.id)),
                "recipes.toml missing {}",
                r.id
            );
        }
    }

    /// The recipe table exists twice on disk: `sim/data/` is the copy Rust
    /// embeds, `game/data/` is the one staged for the eventual hot-reload path.
    /// Nothing in `game/scripts/` reads its copy yet, so a change to one and not
    /// the other is invisible — which is exactly what happened while these
    /// recipes were being added. Byte equality is the cheapest guard.
    #[test]
    fn both_recipe_copies_agree() {
        let sim = concat!(env!("CARGO_MANIFEST_DIR"), "/data/recipes.toml");
        let game = concat!(env!("CARGO_MANIFEST_DIR"), "/../game/data/recipes.toml");
        let a = std::fs::read_to_string(sim).expect("sim/data/recipes.toml");
        let b = match std::fs::read_to_string(game) {
            Ok(b) => b,
            // A source tarball may ship the crate without the Godot project.
            Err(_) => return,
        };
        assert_eq!(
            a, b,
            "sim/data/recipes.toml and game/data/recipes.toml have drifted; \
             edit one and copy it to the other"
        );
    }

    #[test]
    fn dig_half_brush_yields_less_than_full() {
        let hab = Habitat::kepler_drum();
        let t = Terrain::generate(hab);
        // Surface point near equator.
        let theta = 0.4f32;
        let z = 0.0f32;
        let surf = t.surface_radius(theta, z);
        let c = t.hab.to_world(theta, z, surf - 0.5);
        let up = t.hab.up_at(c);
        let full = Stroke {
            c,
            radius: 2.5,
            dig: true,
            level: false,
            up,
        };
        // Offset so only half the sphere intersects solid ground.
        let mut half_c = c;
        half_c[0] += up[0] * 2.2;
        half_c[1] += up[1] * 2.2;
        half_c[2] += up[2] * 2.2;
        let half = Stroke {
            c: half_c,
            radius: 2.5,
            dig: true,
            level: false,
            up,
        };
        let y_full = integrate_dig_yield(&t, &full);
        let y_half = integrate_dig_yield(&t, &half);
        assert!(
            y_full.total_volume_m3 > 1.0,
            "expected meaningful full dig volume, got {}",
            y_full.total_volume_m3
        );
        assert!(
            y_half.total_volume_m3 < y_full.total_volume_m3 * 0.75,
            "half-brush should yield less: half={} full={}",
            y_half.total_volume_m3,
            y_full.total_volume_m3
        );
    }

    #[test]
    fn inventory_mass_and_volume_bind() {
        let mut inv = Inventory::default();
        let y = DigYield {
            parts: vec![YieldPart {
                material_id: mid::BASALT,
                volume_m3: 0.05,
                mass_kg: 150.0, // way over mass cap
                loose_m3: 0.065,
                grade: 0.0,
            }],
            total_mass_kg: 150.0,
            total_loose_m3: 0.065,
            total_volume_m3: 0.05,
        };
        let frac = inv.try_add(&y);
        // The bound is the configured cap, not a hardcoded fraction: the pack
        // grew to hold felled trunks, and a literal here just goes stale.
        let expect = (inv.max_mass_kg / 150.0).min(inv.max_volume_m3 / 0.065);
        assert!(
            (frac - expect).abs() < 0.02,
            "mass cap should bind at {expect:.3}, got frac={frac}"
        );
        assert!(frac < 1.0, "a 150 kg bite must not fit whole");
        assert!(inv.mass_kg() <= inv.max_mass_kg + 1e-3);
    }

    /// The forest plan's stage-1 gate: "chopping blocks and then pressing H
    /// cannot pay out the original whole-tree biomass again."
    #[test]
    fn chop_then_fell_conserves_material() {
        let mut p = Plant::default();
        p.stem = 1.6;
        p.leaf = 0.9;
        p.root = 0.5;
        p.repro = 0.2;
        let above = plant_above_ground_kg(&p);
        assert!(above > 1.0, "need a tree with woody mass, got {above}");

        let woody = |y: &DigYield| -> f32 {
            y.parts
                .iter()
                .filter(|x| matches!(x.material_id, bio_id::WOOD | bio_id::GREEN))
                .map(|x| x.mass_kg)
                .sum()
        };

        // Chop roughly a third of it into blocks, then fell the remains.
        let chopped = above * 0.34;
        let rest = harvest_plant_remaining(&p, chopped);
        let total = chopped + woody(&rest);
        assert!(
            (total - above).abs() < 0.05,
            "chop {chopped:.2} + fell {:.2} = {total:.2}, but the tree only had {above:.2}",
            woody(&rest)
        );

        // Felling an untouched tree still pays the whole thing.
        assert!((woody(&harvest_plant_remaining(&p, 0.0)) - above).abs() < 1e-3);

        // A tree already chopped away pays no more wood, however often asked.
        let empty = harvest_plant_remaining(&p, above);
        assert!(woody(&empty) < 1e-3, "a stripped tree still paid wood");
        let over = harvest_plant_remaining(&p, above * 3.0);
        assert!(woody(&over) < 1e-3, "over-billing went negative");
    }

    /// Roots and seed are not in the block grid, so chopping branches must not
    /// reduce them.
    #[test]
    fn chopping_branches_does_not_take_roots_or_seed() {
        let mut p = Plant::default();
        p.stem = 1.4;
        p.leaf = 0.8;
        p.root = 0.6;
        p.repro = 0.3;
        let of = |y: &DigYield, id: u8| -> f32 {
            y.parts
                .iter()
                .filter(|x| x.material_id == id)
                .map(|x| x.mass_kg)
                .sum()
        };
        let full = harvest_plant(&p);
        let half = harvest_plant_remaining(&p, plant_above_ground_kg(&p) * 0.5);
        for id in [bio_id::FIBRE, bio_id::SEED] {
            assert!(
                (of(&full, id) - of(&half, id)).abs() < 1e-3,
                "material {id} changed when only branches were chopped"
            );
        }
        assert!(of(&half, bio_id::WOOD) < of(&full, bio_id::WOOD));
    }

    #[test]
    fn harvest_uses_plant_pools() {
        let mut p = Plant::default();
        p.stem = 2.0;
        p.leaf = 1.0;
        p.root = 0.5;
        p.repro = 0.2;
        let y = harvest_plant(&p);
        assert!(y.parts.iter().any(|x| x.material_id == bio_id::WOOD));
        assert!(y.total_mass_kg > 10.0);
    }

    #[test]
    fn encumbrance_slows_under_load() {
        let mut inv = Inventory::default();
        let empty = inv.encumbrance(9.81);
        inv.add_stack(mid::CLAY, 40.0, 0.035, 0.0);
        let full = inv.encumbrance(9.81);
        assert!(
            full < empty * 0.85,
            "loaded {full} should be slower than {empty}"
        );
    }

    #[test]
    fn charcoal_then_smelt_costs_oxygen() {
        let mut inv = Inventory::default();
        inv.max_mass_kg = 200.0;
        inv.max_volume_m3 = 0.5;
        inv.add_stack(bio_id::WOOD, 40.0, 0.06, 0.0);
        inv.add_stack(mid::FERROUS, 40.0, 0.02, 0.4);
        let mut atmo = Atmosphere::default();
        let o2_0 = atmo.o2_kg;
        let charcoal = recipe_by_id("charcoal_kiln").unwrap();
        let r1 = craft(&mut inv, &mut atmo, charcoal, 1.0).expect("charcoal");
        assert!(r1.o2_used > 0.0);
        assert!(inv.mass_of(craft_id::CHARCOAL) > 2.0);
        let smelt = recipe_by_id("smelt_ferrous").unwrap();
        // Need more charcoal for a full smelt — run another kiln batch.
        let _ = craft(&mut inv, &mut atmo, charcoal, 1.0);
        let r2 = craft(&mut inv, &mut atmo, smelt, 0.5).expect("smelt");
        assert!(inv.mass_of(craft_id::IRON) > 3.0);
        assert!(atmo.o2_kg < o2_0 - r1.o2_used);
        assert!(atmo.co2_kg > 600.0);
        assert!(r2.co2_made > 0.0);
    }

    #[test]
    fn glass_recipe_balances_and_needs_sand() {
        let r = recipe_by_id("melt_glass").unwrap();
        assert!(recipe_mass_ok(r, 0.05));
        let mut inv = Inventory::default();
        inv.max_mass_kg = 80.0;
        inv.max_volume_m3 = 0.2;
        inv.add_stack(craft_id::SAND, 8.0, 0.006, 0.0);
        inv.add_stack(craft_id::LIME, 1.5, 0.002, 0.0);
        inv.add_stack(craft_id::CHARCOAL, 2.0, 0.008, 0.0);
        let mut atmo = Atmosphere::default();
        let out = craft(&mut inv, &mut atmo, r, 1.0).expect("glass");
        assert!(inv.mass_of(craft_id::GLASS) + out.spilled.total_mass_kg > 8.0);
    }

    #[test]
    fn garden_to_ramen_bowl() {
        let mut inv = Inventory::default();
        inv.max_mass_kg = 80.0;
        inv.max_volume_m3 = 0.25;
        // Lush-band harvest grade.
        inv.add_stack(bio_id::SEED, 20.0, 0.04, 0.7);
        inv.add_stack(bio_id::GREEN, 15.0, 0.04, 0.65);
        inv.add_stack(craft_id::ASH, 2.0, 0.005, 0.0);
        let mut atmo = Atmosphere::default();
        let _ = craft(
            &mut inv,
            &mut atmo,
            recipe_by_id("mill_flour").unwrap(),
            1.0,
        );
        let _ = craft(&mut inv, &mut atmo, recipe_by_id("press_oil").unwrap(), 0.5);
        let _ = craft(
            &mut inv,
            &mut atmo,
            recipe_by_id("bone_meal_mix").unwrap(),
            1.0,
        );
        let _ = craft(
            &mut inv,
            &mut atmo,
            recipe_by_id("simmer_broth").unwrap(),
            1.0,
        );
        let _ = craft(
            &mut inv,
            &mut atmo,
            recipe_by_id("roll_noodles").unwrap(),
            1.0,
        );
        let _ = craft(
            &mut inv,
            &mut atmo,
            recipe_by_id("reduce_tare").unwrap(),
            0.5,
        );
        let bowl = craft(
            &mut inv,
            &mut atmo,
            recipe_by_id("bowl_rich_ramen").unwrap(),
            0.5,
        )
        .expect("rich ramen");
        assert!(inv.mass_of(craft_id::RICH_RAMEN) + bowl.spilled.total_mass_kg > 2.0);
        let g = inv
            .stacks
            .iter()
            .find(|s| s.material_id == craft_id::RICH_RAMEN)
            .map(|s| s.grade)
            .unwrap_or(0.0);
        assert!(g > 0.3, "ramen should inherit garden grade, got {g}");
    }
    /// A stratum nobody can find is dead code with a palette entry. Bounds,
    /// not an exact figure: the seam is noise-driven, so pin the band it has to
    /// stay inside — dense enough to be worth prospecting, rare enough that
    /// coal is not simply the ground.
    #[test]
    fn coal_seams_are_findable_but_not_everywhere() {
        let hab = Habitat::kepler_drum();
        let t = Terrain::generate(hab);
        let mut coal = 0u32;
        let mut n = 0u32;
        for zi in 0..32 {
            for ti in 0..32 {
                let theta = ti as f32 / 32.0 * std::f32::consts::TAU;
                let z = (zi as f32 / 32.0 - 0.5) * hab.length * 0.9;
                let surf0 = hab.radius - t.elevation0(theta, z);
                for d in 1..24 {
                    // r grows toward the hull, so depth below surface adds to r.
                    let p = hab.to_world(theta, z, surf0 + d as f32 * 1.4);
                    if crate::material::material_at(&t, p) == mid::COAL {
                        coal += 1;
                    }
                    n += 1;
                }
            }
        }
        let pct = coal as f32 / n as f32 * 100.0;
        assert!(
            (0.5..8.0).contains(&pct),
            "coal is {pct:.2}% of sampled rock; expected 0.5-8%"
        );
    }

    /// The whole point of splitting `greens`: forms differ in what they give.
    #[test]
    fn forms_give_different_produce() {
        let mut p = Plant::default();
        p.leaf = 1.0;
        p.stem = 0.0;
        p.root = 0.0;
        p.repro = 0.0;
        let mass = |y: &DigYield, id: u8| {
            y.parts
                .iter()
                .filter(|x| x.material_id == id)
                .map(|x| x.mass_kg)
                .sum::<f32>()
        };
        let orchard = harvest_plant_as(&p, 5);
        let conifer = harvest_plant_as(&p, 0);
        let reed = harvest_plant_as(&p, 4);
        assert!(
            mass(&orchard, bio_id::FRUIT) > 0.0,
            "orchard must give fruit"
        );
        assert!(mass(&orchard, bio_id::VEG) > 0.0, "orchard must give veg");
        assert_eq!(mass(&conifer, bio_id::FRUIT), 0.0, "a pine has no fruit");
        assert!(
            mass(&reed, bio_id::GRASS) > mass(&reed, bio_id::GREEN),
            "a reed bed is mostly grass"
        );
    }

    /// Splitting the pool must not enlarge it. If a share table stops summing
    /// to one, foraging silently starts minting mass.
    #[test]
    fn harvest_splits_preserve_leaf_mass() {
        for form in 0..8u8 {
            let shares = soft_tissue_shares(form);
            let total: f32 = shares.iter().sum();
            assert!(
                (total - 1.0).abs() < 1e-3,
                "form {form} shares sum to {total:.4}, not 1.0"
            );
        }
        // And end to end, against a plant with a real leaf pool.
        let mut p = Plant::default();
        p.leaf = 0.8;
        p.stem = 0.0;
        p.root = 0.0;
        p.repro = 0.0;
        let want = 0.8 * 12.0 * 1.4;
        for form in 0..8u8 {
            let y = harvest_plant_as(&p, form);
            let got: f32 = y
                .parts
                .iter()
                .filter(|x| {
                    matches!(
                        x.material_id,
                        bio_id::GREEN
                            | bio_id::FLOWER
                            | bio_id::FRUIT
                            | bio_id::VEG
                            | bio_id::GRASS
                    )
                })
                .map(|x| x.mass_kg)
                .sum();
            assert!(
                (got - want).abs() < 0.2,
                "form {form} soft tissue {got:.2} kg from a {want:.2} kg pool"
            );
        }
    }

    /// A material with no `Phys` row gets whatever the fallback hands back, so
    /// a forgotten entry shows up as a stack with absurd volume rather than as
    /// an error. Every carryable needs a name in both directions too, or a
    /// recipe referring to it silently fails to resolve.
    #[test]
    fn every_new_material_has_physics_and_a_name() {
        let carryable = [
            bio_id::FLOWER,
            bio_id::FRUIT,
            bio_id::VEG,
            bio_id::GRASS,
            bio_id::LAVA,
            craft_id::TEA,
            craft_id::THATCH,
            mid::COAL,
        ];
        for id in carryable {
            let name = if id < 100 {
                crate::material::name(id)
            } else {
                bio_name(id)
            };
            assert!(!name.is_empty() && name != "?", "id {id} has no name");
            assert_eq!(
                material_id_by_name(name),
                Some(id),
                "name {name:?} does not round-trip to id {id}"
            );
            let ph = if id < 100 { phys(id) } else { bio_phys(id) };
            // A material with no colour of its own piles up looking like dirt.
            let col = display_albedo(id);
            assert_ne!(
                col,
                [0.46, 0.36, 0.26],
                "id {id} ({name}) has no display colour"
            );
            assert!(
                ph.bulk_kg_m3 > 1.0 && ph.bulking >= 1.0,
                "id {id} ({name}) has no sensible Phys: {:?} / {:?}",
                ph.bulk_kg_m3,
                ph.bulking
            );
        }
    }
}
