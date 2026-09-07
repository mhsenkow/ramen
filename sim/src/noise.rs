//! Deterministic value-noise. Own implementation: no external RNG, no float
//! ambiguity, reproducible across platforms. (REQUIREMENTS.md B8)

#[inline]
fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = seed
        .wrapping_add((x as u32).wrapping_mul(0x9E3779B1))
        .wrapping_add((y as u32).wrapping_mul(0x85EBCA77));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A2D39);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 16_777_215.0
}

#[inline]
fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = seed
        .wrapping_add((x as u32).wrapping_mul(0x9E3779B1))
        .wrapping_add((y as u32).wrapping_mul(0x85EBCA77))
        .wrapping_add((z as u32).wrapping_mul(0xC2B2AE3D));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B3C6D);
    h ^= h >> 13;
    h = h.wrapping_mul(0x297A2D39);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 16_777_215.0
}

#[inline]
fn smooth(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }

/// Value noise on a torus in x (period `px` cells) so terrain wraps seamlessly
/// around the drum. Returns 0..1.
pub fn value2_wrapped(x: f32, y: f32, px: i32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = smooth(x - xi as f32);
    let yf = smooth(y - yi as f32);
    let w = |i: i32| i.rem_euclid(px.max(1));
    let (x0, x1) = (w(xi), w(xi + 1));
    let a = hash2(x0, yi, seed);
    let b = hash2(x1, yi, seed);
    let c = hash2(x0, yi + 1, seed);
    let d = hash2(x1, yi + 1, seed);
    let top = a + (b - a) * xf;
    let bot = c + (d - c) * xf;
    top + (bot - top) * yf
}

pub fn value3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (xi, yi, zi) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let (xf, yf, zf) = (smooth(x - xi as f32), smooth(y - yi as f32), smooth(z - zi as f32));
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let c00 = l(hash3(xi, yi, zi, seed), hash3(xi + 1, yi, zi, seed), xf);
    let c10 = l(hash3(xi, yi + 1, zi, seed), hash3(xi + 1, yi + 1, zi, seed), xf);
    let c01 = l(hash3(xi, yi, zi + 1, seed), hash3(xi + 1, yi, zi + 1, seed), xf);
    let c11 = l(hash3(xi, yi + 1, zi + 1, seed), hash3(xi + 1, yi + 1, zi + 1, seed), xf);
    l(l(c00, c10, yf), l(c01, c11, yf), zf)
}

/// Fractal Brownian motion, wrapped in x.
pub fn fbm2(x: f32, y: f32, px: i32, oct: u32, seed: u32) -> f32 {
    let (mut f, mut a, mut sum, mut norm) = (1.0f32, 1.0f32, 0.0f32, 0.0f32);
    let mut p = px;
    for o in 0..oct {
        sum += value2_wrapped(x * f, y * f, p.max(1), seed.wrapping_add(o * 7919)) * a;
        norm += a;
        f *= 2.0; a *= 0.5; p *= 2;
    }
    sum / norm
}

pub fn fbm3(x: f32, y: f32, z: f32, oct: u32, seed: u32) -> f32 {
    let (mut f, mut a, mut sum, mut norm) = (1.0f32, 1.0f32, 0.0f32, 0.0f32);
    for o in 0..oct {
        sum += value3(x * f, y * f, z * f, seed.wrapping_add(o * 6271)) * a;
        norm += a;
        f *= 2.0; a *= 0.5;
    }
    sum / norm
}

/// Ridged multifractal — the mountain-maker. Wrapped in x.
pub fn ridged2(x: f32, y: f32, px: i32, oct: u32, seed: u32) -> f32 {
    let (mut f, mut a, mut sum, mut norm) = (1.0f32, 1.0f32, 0.0f32, 0.0f32);
    let mut p = px;
    for o in 0..oct {
        let n = value2_wrapped(x * f, y * f, p.max(1), seed.wrapping_add(o * 4241));
        let r = 1.0 - (n * 2.0 - 1.0).abs();
        sum += r * r * a;
        norm += a;
        f *= 2.0; a *= 0.5; p *= 2;
    }
    sum / norm
}
