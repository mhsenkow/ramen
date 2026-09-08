//! Biomes as readout from fields, not stored labels. LANDSCAPE_200.md §G 119–124.
//! Extended LANDSCAPE_4200 §BS/BU: swamp, meadow, desert, dune, shore.

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
    pub const SWAMP: u8 = 9;
    pub const MEADOW: u8 = 10;
    pub const DESERT: u8 = 11;
    pub const DUNE: u8 = 12;
    pub const SHORE: u8 = 13;
}

const COUNT: usize = 14;

/// Derive biome from continuous fields. `aridity` 0..1 (0 = wet climate).
/// Priority order encodes rare/special cases first.
pub fn classify(
    moisture: f32,
    temp_c: f32,
    elev: f32,
    slope: f32,
    flux: f32,
    soil_depth: f32,
) -> u8 {
    classify_ex(moisture, temp_c, elev, slope, flux, soil_depth, 0.35, 0.0)
}

pub fn classify_ex(
    moisture: f32,
    temp_c: f32,
    elev: f32,
    slope: f32,
    flux: f32,
    soil_depth: f32,
    aridity: f32,
    water_level: f32,
) -> u8 {
    let m = moisture.clamp(0.0, 1.0);
    let t = temp_c;
    let s = slope.clamp(0.0, 1.0);
    let f = flux.clamp(0.0, 1.0);
    let soil = soil_depth.max(0.0);
    let arid = aridity.clamp(0.0, 1.0);

    // Standing water / saturated flats / below waterline.
    if (water_level > 0.0 && elev < water_level - 0.25) || (m > 0.92 && s < 0.12) {
        return id::WATER;
    }
    // Beach / shore band just above waterline.
    if water_level > 0.0 && elev >= water_level - 0.25 && elev < water_level + 2.2 && s < 0.35 {
        return id::SHORE;
    }
    // Steep faces or skin-thin soil → bare rock.
    if s > 0.55 || soil < 0.25 {
        return id::BARE_ROCK;
    }
    // Active dunes: arid + modest relief texture signal via low soil + arid.
    if arid > 0.62 && m < 0.28 && s < 0.45 && soil < 1.4 {
        return id::DUNE;
    }
    // Desert: arid + dry, not dune.
    if arid > 0.58 && m < 0.30 && t >= 12.0 {
        return id::DESERT;
    }
    // Cold / high → alpine.
    if t < 2.0 || elev > 280.0 {
        return id::ALPINE;
    }
    // High-flux corridor → riparian.
    if f > 0.58 && m > 0.35 {
        return id::RIPARIAN;
    }
    // Swamp: wet, gentle, ponded-ish (warm/temperate peat).
    if m > 0.70 && s < 0.18 && t >= 8.0 && elev < water_level.max(22.0) + 12.0 {
        return id::SWAMP;
    }
    // Wet + cool → wetland.
    if m > 0.62 && t < 12.0 {
        return id::WETLAND;
    }
    // Wet + warm + enough soil → forest.
    if m > 0.55 && t >= 12.0 && soil > 0.8 && arid < 0.45 {
        return id::FOREST;
    }
    // Meadow: mesic, gentle, drained (not swamp).
    if m > 0.38 && m < 0.72 && s < 0.28 && soil > 0.9 && arid < 0.45 && t > 6.0 && t < 28.0 {
        return id::MEADOW;
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
        id::SWAMP => "swamp",
        id::MEADOW => "meadow",
        id::DESERT => "desert",
        id::DUNE => "dune",
        id::SHORE => "shore",
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
        id::SWAMP => [0.16, 0.32, 0.24],
        id::MEADOW => [0.30, 0.54, 0.22],
        id::DESERT => [0.62, 0.52, 0.34],
        id::DUNE => [0.72, 0.60, 0.38],
        id::SHORE => [0.70, 0.64, 0.48],
        _ => [0.5, 0.5, 0.5],
    }
}

/// Count biome occurrences over `samples` query sites.
/// `query(i) -> (moisture, temp_c, elev, slope, flux, soil_depth)`.
pub fn census(
    samples: usize,
    mut query: impl FnMut(usize) -> (f32, f32, f32, f32, f32, f32),
) -> [u32; COUNT] {
    let mut counts = [0u32; COUNT];
    for i in 0..samples {
        let (m, t, e, s, f, soil) = query(i);
        let b = classify(m, t, e, s, f, soil) as usize;
        counts[b.min(COUNT - 1)] += 1;
    }
    counts
}

pub fn census_ex(
    samples: usize,
    mut query: impl FnMut(usize) -> (f32, f32, f32, f32, f32, f32, f32, f32),
) -> [u32; COUNT] {
    let mut counts = [0u32; COUNT];
    for i in 0..samples {
        let (m, t, e, s, f, soil, arid, wl) = query(i);
        let b = classify_ex(m, t, e, s, f, soil, arid, wl) as usize;
        counts[b.min(COUNT - 1)] += 1;
    }
    counts
}
