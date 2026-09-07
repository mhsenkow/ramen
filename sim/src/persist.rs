//! Persist player deltas — strokes + soil (REQUIREMENTS E4 / NEXT #10).
//! Format v1: magic + version + stroke count + packed strokes.
//! Soil blob: "SOIL" + version + ST*SZ f32 N + ST*SZ f32 moisture.

use crate::edits::{Edits, Stroke};
use crate::soil::{Soil, ST, SZ};

const MAGIC: &[u8; 4] = b"RAMA";
const VERSION: u32 = 1;
const SOIL_MAGIC: &[u8; 4] = b"SOIL";

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
    if bytes.len() < 12 { return None; }
    if &bytes[0..4] != MAGIC { return None; }
    let ver = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
    if ver != VERSION { return None; }
    let n = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
    let mut off = 12usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if off + 32 > bytes.len() { return None; }
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
        out.push(Stroke { c, radius, dig, level, up });
    }
    Some(out)
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
    if bytes.len() < 8 + n * 8 { return false; }
    if &bytes[0..4] != SOIL_MAGIC { return false; }
    let ver = u32::from_le_bytes(match bytes[4..8].try_into() {
        Ok(b) => b,
        Err(_) => return false,
    });
    if ver != VERSION { return false; }
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
