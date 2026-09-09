//! Dwellings — where each principal lives, and what that place can feed.
//!
//! Rule (1401) still holds: agents act through the same world operations the
//! player does. A dwelling is not an agent-only affordance — it is an
//! *aggregate* readout of ground someone claimed, the way a stockpile is an
//! aggregate of mass someone actually moved. Followers are the same simulation
//! at lower resolution, never decoration: they eat, they work, they leave.
//!
//! Three ways to live on a drum, sited by trait and scored against the real
//! heightfield:
//!
//! | kind     | ground              | wants            | reads as        |
//! |----------|---------------------|------------------|-----------------|
//! | Delve    | steep, high         | risk, low social | alone in a cave |
//! | Terrace  | moderate slope, mid | patience, care   | benches, orchard|
//! | Township | flat, low, by water | social           | a place, growing|
//!
//! Carrying capacity is a query over arable cells, not a growth curve. A
//! township on good bottomland grows; a delve on rock does not. That is the
//! whole readout: **six followers means thriving, alone means it is not.**

use crate::habitat::Habitat;
use crate::soil::Soil;
use crate::terrain::{idx, NT, NZ};

/// Dry food a tended plot returns per square metre per year.
/// Wheat at 3–5 t/ha is 0.30–0.50 kg/m²/yr; take the middle for hand-worked
/// habitat plots. CALIBRATION: literature range, midpoint chosen.
pub const ARABLE_YIELD_KG_M2_YR: f32 = 0.45;
/// Dry ration per person per day. ~2200 kcal at ~3.7 kcal/g dry staple.
/// CALIBRATION: derived from energy requirement, not measured in-game.
pub const RATION_KG_DAY: f32 = 0.60;
/// Radius of the ground a dwelling draws on, in metres. INVENTED — chosen so a
/// township's disc is legible from the air without swallowing its neighbours.
pub const CLAIM_R: f32 = 90.0;
/// Slope above which ground is not worth ploughing (rise over run).
pub const ARABLE_SLOPE: f32 = 0.25;
/// Fed labour-days that raise one structure. Works are paid for in labour,
/// not in food: the builders' rations are already charged as consumption, so
/// charging grain again would be counting the same bread twice.
pub const WORK_DAYS: f32 = 16.0;
/// People per structure. Eight people live in roughly six roofs.
pub const PEOPLE_PER_WORK: f32 = 1.5;
/// People one worker feeds on good ground. A farmer feeds more than himself —
/// that surplus is the whole basis of anyone living together. Pre-industrial
/// intensive agriculture runs about 2-3. CALIBRATION: historical ratio,
/// order-of-magnitude only.
pub const LABOUR_RATIO: f32 = 2.4;
/// Days of food in hand before a place can spare anyone for building.
pub const BUILD_KEEP_DAYS: f32 = 15.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DwellKind {
    Delve,
    Terrace,
    Township,
}

impl DwellKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DwellKind::Delve => "delve",
            DwellKind::Terrace => "terrace",
            DwellKind::Township => "township",
        }
    }
    pub fn code(self) -> f32 {
        match self {
            DwellKind::Delve => 0.0,
            DwellKind::Terrace => 1.0,
            DwellKind::Township => 2.0,
        }
    }
}

/// How hard this person is to reach. Derived from siting and circumstance —
/// never assigned by hand, so it can change when the world does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// Wants a thing, takes the thing, affinity moves. Teaches the system.
    Legible,
    /// Will not take help directly. You change the conditions around him.
    Indirect,
    /// Holed up, against the colony project. A long chain of indirect acts.
    Opaque,
    /// Shares a watershed with someone else. You cannot supply both.
    Contested,
}

impl Reach {
    pub fn as_str(self) -> &'static str {
        match self {
            Reach::Legible => "legible",
            Reach::Indirect => "indirect",
            Reach::Opaque => "opaque",
            Reach::Contested => "contested",
        }
    }
    pub fn code(self) -> f32 {
        match self {
            Reach::Legible => 0.0,
            Reach::Indirect => 1.0,
            Reach::Opaque => 2.0,
            Reach::Contested => 3.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Dwelling {
    pub agent_id: u32,
    pub kind: DwellKind,
    pub reach: Reach,
    pub theta: f32,
    pub z: f32,
    pub elev: f32,
    /// People this ground can feed, from arable area and soil quality.
    pub capacity: f32,
    /// Statistical population beyond the principal. Fractional on purpose:
    /// it is a population, not a roster.
    pub followers: f32,
    /// Completed structures. Surplus buys these; they do not decay yet.
    pub works: u32,
    pub stores_kg: f32,
    /// Labour-days banked toward the next structure (or, negative, toward
    /// letting one fall in).
    pub build_days: f32,
    /// Site quality 0..~1.4 from soil. Drives surplus.
    pub quality: f32,
    /// The other dwelling this one shares water with, if any.
    pub rival: Option<u32>,
    pub founded_day: f32,
}

impl Dwelling {
    /// Rations in store as days of keeping, for the readout.
    pub fn days_of_food(&self) -> f32 {
        let mouths = (self.followers + 1.0).max(0.1);
        self.stores_kg / (mouths * RATION_KG_DAY)
    }

    /// Is this place growing, holding, or emptying?
    pub fn trend(&self) -> f32 {
        let target = (self.capacity - 1.0).max(0.0);
        target - self.followers
    }
}

/// Metres per grid cell, tangential and axial.
pub fn cell_spacing(hab: &Habitat) -> (f32, f32) {
    (
        std::f32::consts::TAU * hab.radius / NT as f32,
        hab.length / NZ as f32,
    )
}

/// Gradient magnitude of the heightfield at a cell (rise over run).
pub fn slope_at(elev: &[f32], ti: usize, zi: usize, hab: &Habitat) -> f32 {
    let (dt_m, dz_m) = cell_spacing(hab);
    let tp = (ti + 1) % NT;
    let tm = (ti + NT - 1) % NT;
    let zp = (zi + 1).min(NZ - 1);
    let zm = zi.saturating_sub(1);
    let gt = (elev[idx(tp, zi)] - elev[idx(tm, zi)]) / (2.0 * dt_m);
    let gz = (elev[idx(ti, zp)] - elev[idx(ti, zm)]) / (2.0 * dz_m);
    (gt * gt + gz * gz).sqrt()
}

/// Arable area in m² within the claim, and mean soil quality over it.
/// This is the honest part: capacity is a query over real ground, so a bad
/// site simply cannot grow, and fixing its drainage genuinely changes it.
pub fn survey(hab: &Habitat, elev: &[f32], soil: Option<&Soil>, theta: f32, z: f32) -> (f32, f32) {
    let (dt_m, dz_m) = cell_spacing(hab);
    let cell_area = dt_m * dz_m;
    let rt = (CLAIM_R / dt_m).ceil() as i32;
    let rz = (CLAIM_R / dz_m).ceil() as i32;
    let ti0 = (theta / std::f32::consts::TAU * NT as f32).round() as i32;
    let zi0 = ((z / hab.length + 0.5) * NZ as f32).round() as i32;

    let mut area = 0.0f32;
    let mut qsum = 0.0f32;
    let mut qn = 0.0f32;
    for dz in -rz..=rz {
        let zi = zi0 + dz;
        if zi < 0 || zi >= NZ as i32 {
            continue;
        }
        for dt in -rt..=rt {
            let ti = (ti0 + dt).rem_euclid(NT as i32);
            // Elliptical claim in metres, not a rectangle in cells.
            let mx = dt as f32 * dt_m;
            let my = dz as f32 * dz_m;
            if mx * mx + my * my > CLAIM_R * CLAIM_R {
                continue;
            }
            let (ti, zi) = (ti as usize, zi as usize);
            let e = elev[idx(ti, zi)];
            if e < hab.water_level + 1.5 {
                continue;
            }
            if slope_at(elev, ti, zi, hab) > ARABLE_SLOPE {
                continue;
            }
            area += cell_area;
            if let Some(s) = soil {
                let th = ti as f32 / NT as f32 * std::f32::consts::TAU;
                let zz = (zi as f32 / NZ as f32 - 0.5) * hab.length;
                let ss = s.sample(th, zz);
                // Nitrogen and organic carry the yield; moisture gates it.
                let q = (0.35 + ss.n * 0.7 + ss.organic * 0.5)
                    * (0.4 + ss.moisture.clamp(0.0, 1.0) * 0.8);
                qsum += q;
                qn += 1.0;
            }
        }
    }
    let quality = if qn > 0.0 { qsum / qn } else { 0.75 };
    let prov = crate::province::province_at(hab, theta, z);
    let farm_boost = 1.0 + 0.22 * prov.weight(crate::province::id::FARMLAND);
    let city_trim = 1.0 - 0.12 * prov.weight(crate::province::id::CITY);
    (area, (quality * farm_boost * city_trim).clamp(0.15, 1.4))
}

/// People a site can feed at its current quality.
pub fn capacity_of(area_m2: f32, quality: f32) -> f32 {
    let kg_per_year = area_m2 * ARABLE_YIELD_KG_M2_YR * quality;
    let kg_per_day = kg_per_year / 365.0;
    kg_per_day / RATION_KG_DAY
}

/// Score a candidate cell for each way of living. Higher is better.
fn site_scores(elev: &[f32], ti: usize, zi: usize, hab: &Habitat) -> (f32, f32, f32) {
    let e = elev[idx(ti, zi)];
    let s = slope_at(elev, ti, zi, hab);
    let band = ((e - hab.water_level) / (hab.max_elevation - hab.water_level)).clamp(0.0, 1.0);
    let theta = ti as f32 / NT as f32 * std::f32::consts::TAU;
    let z = (zi as f32 / NZ as f32 - 0.5) * hab.length;
    let prov = crate::province::province_at(hab, theta, z);
    let farm_w = prov.weight(crate::province::id::FARMLAND);
    let city_w = prov.weight(crate::province::id::CITY);

    // Delve wants height and steepness — ground nobody wants to plough.
    let delve =
        (band - 0.42).max(0.0) * 2.4 + (s - 0.22).max(0.0) * 3.0 - farm_w * 0.8 - city_w * 1.1;
    // Terrace wants workable slope in the middle band — thrives on farmland.
    let terrace = (1.0 - (band - 0.30).abs() * 3.6).max(0.0)
        + (1.0 - (s - 0.16).abs() * 6.0).max(0.0)
        + farm_w * 1.8;
    // Township wants flat bottomland just above the waterline — city pads.
    let township = (1.0 - (band - 0.10).abs() * 6.0).max(0.0)
        + (1.0 - s * 8.0).max(0.0)
        + city_w * 2.2
        + farm_w * 0.45;
    (delve, terrace, township)
}

/// Which way of living a set of traits reaches for.
pub fn kind_for(patience: f32, risk: f32, care: f32, social: f32) -> DwellKind {
    let delve = risk * 1.3 + (1.0 - social) * 0.9;
    let terrace = patience * 1.1 + care * 1.0;
    let township = social * 1.6 + care * 0.4;
    if township >= terrace && township >= delve {
        DwellKind::Township
    } else if terrace >= delve {
        DwellKind::Terrace
    } else {
        DwellKind::Delve
    }
}

/// Legibility from siting and circumstance. Contested is decided later, once
/// every dwelling is placed and watersheds can be compared.
fn reach_for(kind: DwellKind, social: f32, care: f32) -> Reach {
    match kind {
        DwellKind::Delve if social < 0.35 => Reach::Opaque,
        DwellKind::Delve => Reach::Indirect,
        DwellKind::Terrace if social < 0.45 => Reach::Indirect,
        DwellKind::Township if social > 0.6 || care > 0.7 => Reach::Legible,
        _ => Reach::Legible,
    }
}

#[derive(Default)]
pub struct Dwellings {
    pub list: Vec<Dwelling>,
}

impl Dwellings {
    pub fn get(&self, agent_id: u32) -> Option<&Dwelling> {
        self.list.iter().find(|d| d.agent_id == agent_id)
    }

    /// Site one principal: search outward from where they already stand for the
    /// best ground for the way they want to live.
    pub fn site(
        &mut self,
        hab: &Habitat,
        elev: &[f32],
        soil: Option<&Soil>,
        agent_id: u32,
        theta: f32,
        z: f32,
        patience: f32,
        risk: f32,
        care: f32,
        social: f32,
        day: f32,
    ) {
        let kind = kind_for(patience, risk, care, social);
        let (dt_m, dz_m) = cell_spacing(hab);
        let ti0 = (theta / std::f32::consts::TAU * NT as f32).round() as i32;
        let zi0 = ((z / hab.length + 0.5) * NZ as f32).round() as i32;
        // Search a 320 m neighbourhood, striding so this stays cheap.
        let rt = (320.0 / dt_m) as i32;
        let rz = (320.0 / dz_m) as i32;
        let stride = 3i32;

        let mut best = f32::NEG_INFINITY;
        let mut best_ti = ti0.rem_euclid(NT as i32) as usize;
        let mut best_zi = zi0.clamp(0, NZ as i32 - 1) as usize;
        let mut dz = -rz;
        while dz <= rz {
            let zi = zi0 + dz;
            if zi < 0 || zi >= NZ as i32 {
                dz += stride;
                continue;
            }
            let mut dt = -rt;
            while dt <= rt {
                let ti = (ti0 + dt).rem_euclid(NT as i32) as usize;
                let zi = zi as usize;
                let e = elev[idx(ti, zi)];
                if e > hab.water_level + 2.0 {
                    let (sd, st, sw) = site_scores(elev, ti, zi, hab);
                    let s = match kind {
                        DwellKind::Delve => sd,
                        DwellKind::Terrace => st,
                        DwellKind::Township => sw,
                    };
                    // Prefer not to drag someone across the drum for a marginal
                    // site — they live where they already are, roughly.
                    let mx = dt as f32 * dt_m;
                    let my = dz as f32 * dz_m;
                    let pull = (mx * mx + my * my).sqrt() / 320.0 * 0.35;
                    if s - pull > best {
                        best = s - pull;
                        best_ti = ti;
                        best_zi = zi;
                    }
                }
                dt += stride;
            }
            dz += stride;
        }

        let sth = best_ti as f32 / NT as f32 * std::f32::consts::TAU;
        let sz = (best_zi as f32 / NZ as f32 - 0.5) * hab.length;
        let (area, quality) = survey(hab, elev, soil, sth, sz);
        let mut capacity = capacity_of(area, quality);
        // A delve is cut into rock. Its people do not plough the hillside, so
        // the site cannot feed a crowd however much land is nominally in reach.
        if kind == DwellKind::Delve {
            capacity = capacity.min(3.0);
        }
        self.list.push(Dwelling {
            agent_id,
            kind,
            reach: reach_for(kind, social, care),
            theta: sth,
            z: sz,
            elev: elev[idx(best_ti, best_zi)],
            capacity,
            followers: 0.0,
            // A man puts a roof over himself before anything else.
            works: 1,
            // Everyone starts with a fortnight in hand.
            stores_kg: 14.0 * RATION_KG_DAY,
            build_days: 0.0,
            quality,
            rival: None,
            founded_day: day,
        });
    }

    /// Mark pairs that share ground. Two claims that overlap draw on one
    /// watershed, and the habitat is closed — you cannot supply both.
    pub fn mark_contested(&mut self, hab_radius: f32) {
        for i in 0..self.list.len() {
            self.list[i].rival = None;
        }
        let n = self.list.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let a = &self.list[i];
                let b = &self.list[j];
                let dth = arc(a.theta, b.theta) * hab_radius;
                let dz = a.z - b.z;
                let d = (dth * dth + dz * dz).sqrt();
                if d < CLAIM_R * 1.85 {
                    let (ai, bi) = (a.agent_id, b.agent_id);
                    self.list[i].rival = Some(bi);
                    self.list[j].rival = Some(ai);
                    self.list[i].reach = Reach::Contested;
                    self.list[j].reach = Reach::Contested;
                }
            }
        }
    }

    /// Re-read the ground. Call when drainage or soil has moved — this is what
    /// makes fixing a river a social act rather than a landscaping one.
    pub fn resurvey(&mut self, hab: &Habitat, elev: &[f32], soil: Option<&Soil>) {
        for d in &mut self.list {
            let (area, quality) = survey(hab, elev, soil, d.theta, d.z);
            let mut cap = capacity_of(area, quality);
            if d.kind == DwellKind::Delve {
                cap = cap.min(3.0);
            }
            d.capacity = cap;
            d.quality = quality;
        }
    }

    /// Population and stores follow the land. No growth curve — people arrive
    /// when there is food and leave when there is not.
    pub fn tick(&mut self, dt_days: f32) {
        let dt = dt_days.max(0.0);
        if dt <= 0.0 {
            return;
        }
        for d in &mut self.list {
            let mouths = d.followers + 1.0;
            // Land and labour both cap the harvest; whichever binds, binds.
            // `capacity` is already people-the-ground-can-feed, quality
            // included — do not charge quality twice.
            let fed = d.capacity.max(0.0).min(mouths * LABOUR_RATIO);
            let produce = fed * RATION_KG_DAY;
            let consume = mouths * RATION_KG_DAY;
            d.stores_kg = (d.stores_kg + (produce - consume) * dt).max(0.0);

            let target = (d.capacity - 1.0).max(0.0);
            if d.stores_kg > mouths * RATION_KG_DAY * 5.0 && d.followers < target {
                // Word gets round a habitat with a surplus.
                d.followers = (d.followers + 0.06 * dt).min(target);
            } else if d.stores_kg <= 0.01 {
                // Hunger empties a place faster than plenty fills it.
                d.followers = (d.followers - 0.14 * dt).max(0.0);
            } else if d.followers > target {
                d.followers = (d.followers - 0.05 * dt).max(target);
            }

            // Works follow the people, not the food surplus. A place that has
            // filled to its carrying capacity runs no surplus by definition,
            // and it is exactly then that it has the most hands to build with.
            let want = (mouths / PEOPLE_PER_WORK).ceil().clamp(1.0, 48.0);
            let have = d.works as f32;
            if have < want && d.days_of_food() > BUILD_KEEP_DAYS {
                d.build_days += dt;
                if d.build_days >= WORK_DAYS {
                    d.build_days -= WORK_DAYS;
                    d.works += 1;
                }
            } else if have > want + 1.0 && d.stores_kg <= 0.01 {
                // A place people left lets its roofs fall in.
                d.build_days -= dt;
                if d.build_days <= -WORK_DAYS {
                    d.build_days += WORK_DAYS;
                    d.works = d.works.saturating_sub(1);
                }
            } else {
                d.build_days *= 1.0 - (0.02 * dt).min(1.0);
            }
        }
    }
}

/// Shortest signed arc between two angles.
fn arc(a: f32, b: f32) -> f32 {
    let mut d = a - b;
    while d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    while d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d.abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hab() -> Habitat {
        Habitat::kepler_drum()
    }

    #[test]
    fn traits_pick_a_way_of_living() {
        // Risk-taking loner goes to ground.
        assert_eq!(kind_for(0.3, 0.9, 0.3, 0.15), DwellKind::Delve);
        // Patient, careful, private: benches on a hillside.
        assert_eq!(kind_for(0.9, 0.2, 0.9, 0.3), DwellKind::Terrace);
        // Sociable: founds a place.
        assert_eq!(kind_for(0.5, 0.3, 0.6, 0.9), DwellKind::Township);
    }

    #[test]
    fn capacity_matches_the_ration_arithmetic() {
        // One hectare of good ground at unit quality.
        let cap = capacity_of(10_000.0, 1.0);
        // 10000 m2 * 0.45 kg/m2/yr = 4500 kg/yr = 12.33 kg/day / 0.6 = 20.5
        assert!((cap - 20.54).abs() < 0.1, "got {cap}");
    }

    #[test]
    fn a_place_with_no_food_empties() {
        let mut d = Dwellings::default();
        d.list.push(Dwelling {
            agent_id: 1,
            kind: DwellKind::Township,
            reach: Reach::Legible,
            theta: 0.0,
            z: 0.0,
            elev: 40.0,
            capacity: 0.0,
            followers: 4.0,
            works: 2,
            stores_kg: 0.0,
            build_days: 0.0,
            quality: 0.2,
            rival: None,
            founded_day: 0.0,
        });
        d.tick(10.0);
        assert!(d.list[0].followers < 4.0, "hunger should drive people off");
    }

    #[test]
    fn good_ground_draws_people() {
        let mut d = Dwellings::default();
        d.list.push(Dwelling {
            agent_id: 1,
            kind: DwellKind::Township,
            reach: Reach::Legible,
            theta: 0.0,
            z: 0.0,
            elev: 40.0,
            capacity: 9.0,
            followers: 0.0,
            works: 1,
            stores_kg: 400.0,
            build_days: 0.0,
            quality: 1.2,
            rival: None,
            founded_day: 0.0,
        });
        d.tick(30.0);
        assert!(d.list[0].followers > 0.5, "surplus should draw followers");
    }

    #[test]
    fn a_place_settles_at_its_carrying_capacity() {
        let mut d = Dwellings::default();
        d.list.push(Dwelling {
            agent_id: 1,
            kind: DwellKind::Township,
            reach: Reach::Legible,
            theta: 0.0,
            z: 0.0,
            elev: 40.0,
            capacity: 6.0,
            followers: 0.0,
            works: 1,
            stores_kg: 60.0,
            build_days: 0.0,
            quality: 1.0,
            rival: None,
            founded_day: 0.0,
        });
        for _ in 0..400 {
            d.tick(1.0);
        }
        let f = d.list[0].followers;
        // Six mouths is what the ground feeds: the principal plus five.
        assert!(
            (f - 5.0).abs() < 0.2,
            "should settle near capacity-1, got {f}"
        );
    }

    #[test]
    fn one_worker_on_poor_ground_starves() {
        let mut d = Dwellings::default();
        d.list.push(Dwelling {
            agent_id: 1,
            kind: DwellKind::Delve,
            reach: Reach::Opaque,
            theta: 0.0,
            z: 0.0,
            elev: 180.0,
            capacity: 0.3,
            followers: 0.0,
            works: 0,
            stores_kg: 8.4,
            build_days: 0.0,
            quality: 0.2,
            rival: None,
            founded_day: 0.0,
        });
        d.tick(40.0);
        assert_eq!(d.list[0].stores_kg, 0.0, "ground this poor cannot keep him");
        assert!(d.list[0].days_of_food() < 0.01);
    }

    #[test]
    fn a_full_township_keeps_building() {
        // The bug this pins: works used to be bought out of food surplus, so a
        // place that reached carrying capacity — and therefore ran no surplus —
        // never built anything again.
        let mut d = Dwellings::default();
        d.list.push(Dwelling {
            agent_id: 1,
            kind: DwellKind::Township,
            reach: Reach::Legible,
            theta: 0.0,
            z: 0.0,
            elev: 40.0,
            capacity: 9.0,
            followers: 8.0,
            works: 1,
            stores_kg: 400.0,
            build_days: 0.0,
            quality: 1.0,
            rival: None,
            founded_day: 0.0,
        });
        for _ in 0..120 {
            d.tick(1.0);
        }
        assert!(
            d.list[0].works >= 6,
            "nine people want six roofs, got {}",
            d.list[0].works
        );
    }

    #[test]
    fn overlapping_claims_are_contested() {
        let mut d = Dwellings::default();
        for (id, th) in [(1u32, 0.0f32), (2, 0.05)] {
            d.list.push(Dwelling {
                agent_id: id,
                kind: DwellKind::Township,
                reach: Reach::Legible,
                theta: th,
                z: 0.0,
                elev: 40.0,
                capacity: 6.0,
                followers: 0.0,
                works: 0,
                stores_kg: 10.0,
                build_days: 0.0,
                quality: 1.0,
                rival: None,
                founded_day: 0.0,
            });
        }
        d.mark_contested(hab().radius);
        assert_eq!(d.list[0].reach, Reach::Contested);
        assert_eq!(d.list[0].rival, Some(2));
    }

    #[test]
    fn distant_claims_are_not_contested() {
        let mut d = Dwellings::default();
        for (id, th) in [(1u32, 0.0f32), (2, 1.2)] {
            d.list.push(Dwelling {
                agent_id: id,
                kind: DwellKind::Terrace,
                reach: Reach::Indirect,
                theta: th,
                z: 0.0,
                elev: 40.0,
                capacity: 6.0,
                followers: 0.0,
                works: 0,
                stores_kg: 10.0,
                build_days: 0.0,
                quality: 1.0,
                rival: None,
                founded_day: 0.0,
            });
        }
        d.mark_contested(hab().radius);
        assert_eq!(d.list[0].rival, None);
    }
}
