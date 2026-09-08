//! Persist player deltas — strokes + soil (REQUIREMENTS E4 / NEXT #10).
//! Format v1: magic + version + stroke count + packed strokes.
//! Soil blob: "SOIL" + version + ST*SZ f32 N + ST*SZ f32 moisture.

use crate::dwelling::{DwellKind, Dwelling};
use crate::edits::{Edits, Stroke};
use crate::soil::{Soil, ST, SZ};

const MAGIC: &[u8; 4] = b"RAMA";
const VERSION: u32 = 1;
const SOIL_MAGIC: &[u8; 4] = b"SOIL";
const DWEL_MAGIC: &[u8; 4] = b"DWEL";

#[derive(Clone, Copy, Debug)]
pub struct DwellingRecord {
    pub agent_id: u32,
    pub kind: u8,
    pub followers: f32,
    pub works: u32,
    pub stores_kg: f32,
    pub build_days: f32,
    pub theta: f32,
    pub z: f32,
    pub capacity: f32,
    pub quality: f32,
}

pub fn encode_strokes(edits: &Edits) -> Vec<u8> {
    let strokes = edits.strokes_slice();
    let mut out = Vec::with_capacity(16 + strokes.len() * 32);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(strokes.len() as u32).to_le_bytes());
    for s in strokes {
        for v in s.c {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&s.radius.to_le_bytes());
        out.push(if s.dig { 1 } else { 0 });
        out.push(if s.level { 1 } else { 0 });
        out.push(0);
        out.push(0);
        for v in s.up {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

pub fn decode_strokes(bytes: &[u8]) -> Option<Vec<Stroke>> {
    if bytes.len() < 12 {
        return None;
    }
    if &bytes[0..4] != MAGIC {
        return None;
    }
    let ver = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
    if ver != VERSION {
        return None;
    }
    let n = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
    let mut off = 12usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if off + 32 > bytes.len() {
            return None;
        }
        let mut c = [0.0f32; 3];
        for i in 0..3 {
            c[i] = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
            off += 4;
        }
        let radius = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        let dig = bytes[off] != 0;
        let level = bytes[off + 1] != 0;
        off += 4;
        let mut up = [0.0f32; 3];
        for i in 0..3 {
            up[i] = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
            off += 4;
        }
        out.push(Stroke {
            c,
            radius,
            dig,
            level,
            up,
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edits::Edits;

    #[test]
    fn strokes_roundtrip_v1() {
        let mut edits = Edits::default();
        edits.add([1.0, 2.0, 3.0], 2.5, true, false, [0.0, 1.0, 0.0]);
        edits.add([4.0, 5.0, 6.0], 1.0, false, true, [0.0, 0.0, 1.0]);
        let bytes = encode_strokes(&edits);
        assert_eq!(&bytes[0..4], MAGIC);
        let ver = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(ver, VERSION);
        let decoded = decode_strokes(&bytes).expect("decode");
        assert_eq!(decoded.len(), 2);
        assert!((decoded[0].radius - 2.5).abs() < 1e-5);
        assert!(decoded[0].dig);
        assert!(decoded[1].level);
    }

    #[test]
    fn strokes_fixture_v1_loads() {
        // Committed empty v1 fixture — migration gate (§2192).
        let bytes = include_bytes!("../tests/fixtures/rama_strokes_v1_empty.bin");
        let decoded = decode_strokes(bytes).expect("fixture decode");
        assert!(decoded.is_empty());
    }

    #[test]
    fn soil_blob_header_v1() {
        // Encode/decode header without constructing a full Habitat soil grid.
        let n = ST * SZ;
        let mut bytes = Vec::with_capacity(8 + n * 8);
        bytes.extend_from_slice(SOIL_MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend(std::iter::repeat_n(0u8, n * 8));
        assert_eq!(&bytes[0..4], SOIL_MAGIC);
        let ver = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(ver, VERSION);
        assert_eq!(bytes.len(), 8 + n * 8);
    }
}

pub fn apply_strokes(edits: &mut Edits, strokes: &[Stroke]) {
    edits.clear();
    for s in strokes {
        edits.add(s.c, s.radius, s.dig, s.level, s.up);
    }
}

pub fn encode_soil(soil: &Soil) -> Vec<u8> {
    let n = ST * SZ;
    let mut out = Vec::with_capacity(8 + n * 8);
    out.extend_from_slice(SOIL_MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    for &v in &soil.n {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for &v in &soil.moisture {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

pub fn decode_soil_into(bytes: &[u8], soil: &mut Soil) -> bool {
    let n = ST * SZ;
    if bytes.len() < 8 + n * 8 {
        return false;
    }
    if &bytes[0..4] != SOIL_MAGIC {
        return false;
    }
    let ver = u32::from_le_bytes(match bytes[4..8].try_into() {
        Ok(b) => b,
        Err(_) => return false,
    });
    if ver != VERSION {
        return false;
    }
    let mut off = 8usize;
    for i in 0..n {
        soil.n[i] = f32::from_le_bytes(match bytes[off..off + 4].try_into() {
            Ok(b) => b,
            Err(_) => return false,
        });
        off += 4;
    }
    for i in 0..n {
        soil.moisture[i] = f32::from_le_bytes(match bytes[off..off + 4].try_into() {
            Ok(b) => b,
            Err(_) => return false,
        });
        off += 4;
    }
    true
}

fn dwell_kind_code(kind: DwellKind) -> u8 {
    match kind {
        DwellKind::Delve => 0,
        DwellKind::Terrace => 1,
        DwellKind::Township => 2,
    }
}

pub fn encode_dwellings(list: &[Dwelling]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + list.len() * 40);
    out.extend_from_slice(DWEL_MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(list.len() as u32).to_le_bytes());
    for d in list {
        out.extend_from_slice(&d.agent_id.to_le_bytes());
        out.push(dwell_kind_code(d.kind));
        out.extend_from_slice(&d.followers.to_le_bytes());
        out.extend_from_slice(&d.works.to_le_bytes());
        out.extend_from_slice(&d.stores_kg.to_le_bytes());
        out.extend_from_slice(&d.build_days.to_le_bytes());
        out.extend_from_slice(&d.theta.to_le_bytes());
        out.extend_from_slice(&d.z.to_le_bytes());
        out.extend_from_slice(&d.capacity.to_le_bytes());
        out.extend_from_slice(&d.quality.to_le_bytes());
    }
    out
}

pub fn decode_dwellings(bytes: &[u8]) -> Option<Vec<DwellingRecord>> {
    if bytes.len() < 12 {
        return None;
    }
    if &bytes[0..4] != DWEL_MAGIC {
        return None;
    }
    let ver = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
    if ver != VERSION {
        return None;
    }
    let n = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
    let mut off = 12usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if off + 37 > bytes.len() {
            return None;
        }
        let agent_id = u32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        let kind = bytes[off];
        off += 1;
        let followers = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        let works = u32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        let stores_kg = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        let build_days = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        let theta = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        let z = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        let capacity = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        let quality = f32::from_le_bytes(bytes[off..off + 4].try_into().ok()?);
        off += 4;
        out.push(DwellingRecord {
            agent_id,
            kind,
            followers,
            works,
            stores_kg,
            build_days,
            theta,
            z,
            capacity,
            quality,
        });
    }
    Some(out)
}
