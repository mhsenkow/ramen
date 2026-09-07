//! Biomes as readout from fields, not stored labels. LANDSCAPE_200.md §G 119–124.
//!
//! Moisture + temperature (Whittaker axes), then soil depth, slope, elev, flux.
//! Same derivation trick as materials: query-time, never a painted map.

/// Stable biome ids for HUD, census, and map colouring.
pub mod id {
    pub const WATER: u8 = 0;
    pub const WETLAND: u8 = 1;
    pub const RIPARIAN: u8 = 2;
    pub const GRASSLAND: u8 = 3;
    pub const SCRUB: u8 = 4;
    pub const FOREST: u8 = 5;
    pub const ALPINE: u8 = 6;
    pub const BARE_ROCK: u8 = 7;
    pub const FARM: u8 = 8;
}

const COUNT: usize = 9;

/// Derive biome from continuous fields (item 121). Priority order encodes the
/// rare/special cases first so corridors and extremes are not swallowed by
/// grassland.
pub fn classify(
    moisture: f32,
    temp_c: f32,
    elev: f32,
    slope: f32,
    flux: f32,
    soil_depth: f32,
) -> u8 {
    let m = moisture.clamp(0.0, 1.0);
    let t = temp_c;
    let s = slope.clamp(0.0, 1.0);
    let f = flux.clamp(0.0, 1.0);
    let soil = soil_depth.max(0.0);

    // Standing water / saturated flats.
    if m > 0.92 && s < 0.12 {
        return id::WATER;
    }
    // Steep faces or skin-thin soil → bare rock (item 120).
    if s > 0.55 || soil < 0.25 {
        return id::BARE_ROCK;
    }
    // Cold / high → alpine.
    if t < 2.0 || elev > 160.0 {
        return id::ALPINE;
    }
    // High-flux corridor → riparian (item 127 / user brief).
    if f > 0.58 && m > 0.35 {
        return id::RIPARIAN;
    }
    // Wet + cool → wetland.
    if m > 0.62 && t < 12.0 {
        return id::WETLAND;
    }
    // Wet + warm + enough soil → forest.
    if m > 0.55 && t >= 12.0 && soil > 0.8 {
        return id::FOREST;
    }
    // Dry + warm → scrub.
    if m < 0.32 && t >= 14.0 {
        return id::SCRUB;
    }
    // Optional farm: deep soil, gentle, temperate moisture.
    if soil > 2.2 && s < 0.22 && m > 0.35 && m < 0.75 && t > 8.0 && t < 28.0 {
        return id::FARM;
    }
    id::GRASSLAND
}

pub fn name(id: u8) -> &'static str {
    match id {
        id::WATER => "water",
        id::WETLAND => "wetland",
        id::RIPARIAN => "riparian",
        id::GRASSLAND => "grassland",
        id::SCRUB => "scrub",
        id::FOREST => "forest",
        id::ALPINE => "alpine",
        id::BARE_ROCK => "bare rock",
        id::FARM => "farm",
        _ => "unknown",
    }
}

/// Map / legend colour (linear RGB).
pub fn color(id: u8) -> [f32; 3] {
    match id {
        id::WATER => [0.13, 0.30, 0.44],
        id::WETLAND => [0.22, 0.40, 0.32],
        id::RIPARIAN => [0.18, 0.42, 0.38],
        id::GRASSLAND => [0.26, 0.50, 0.21],
        id::SCRUB => [0.48, 0.42, 0.24],
        id::FOREST => [0.12, 0.34, 0.16],
        id::ALPINE => [0.62, 0.60, 0.55],
        id::BARE_ROCK => [0.46, 0.44, 0.40],
        id::FARM => [0.42, 0.48, 0.22],
        _ => [0.5, 0.5, 0.5],
    }
}

/// Count biome occurrences over `samples` query sites.
/// `query(i) -> (moisture, temp_c, elev, slope, flux, soil_depth)`.
pub fn census(samples: usize, mut query: impl FnMut(usize) -> (f32, f32, f32, f32, f32, f32)) -> [u32; COUNT] {
    let mut counts = [0u32; COUNT];
    for i in 0..samples {
        let (m, t, e, s, f, soil) = query(i);
        let b = classify(m, t, e, s, f, soil) as usize;
        counts[b.min(COUNT - 1)] += 1;
    }
    counts
}
