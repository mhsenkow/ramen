//! Player edits to the terrain — the mining system.
//!
//! We never store the world. We store the DIFFERENCE from what generation
//! produced (REQUIREMENTS.md E4), as CSG brush strokes against the procedural
//! density field:
//!
//!     dig  =  A \ sphere  =>  d = min(d, dist - radius)
//!     fill =  A U sphere  =>  d = max(d, radius - dist)
//!
//! A stroke is 20 bytes. A player who tunnels for forty hours has a save made
//! of tunnels, not a save made of a habitat.

use std::collections::HashMap;

/// Coarse spatial bucket so density() consults only nearby strokes, never all.
const CELL: f32 = 24.0;

#[derive(Clone, Copy, Debug)]
pub struct Stroke {
    pub c: [f32; 3],
    pub radius: f32,
    /// true = removed material, false = added it.
    pub dig: bool,
    /// Sphere brush, or a levelling brush: a cylinder about `up` cut off at the
    /// plane through `c`. Levelling is what you actually want for building
    /// pads, terraces and floors — a sphere can never make a flat surface.
    pub level: bool,
    pub up: [f32; 3],
}

#[derive(Default)]
pub struct Edits {
    strokes: Vec<Stroke>,
    index: HashMap<(i32, i32, i32), Vec<u32>>,
}

#[inline]
fn key(p: [f32; 3]) -> (i32, i32, i32) {
    (
        (p[0] / CELL).floor() as i32,
        (p[1] / CELL).floor() as i32,
        (p[2] / CELL).floor() as i32,
    )
}

impl Edits {
    pub fn len(&self) -> usize {
        self.strokes.len()
    }

    pub fn strokes_slice(&self) -> &[Stroke] {
        &self.strokes
    }

    pub fn clear(&mut self) {
        self.strokes.clear();
        self.index.clear();
    }

    pub fn add(&mut self, c: [f32; 3], radius: f32, dig: bool, level: bool, up: [f32; 3]) {
        let id = self.strokes.len() as u32;
        self.strokes.push(Stroke {
            c,
            radius,
            dig,
            level,
            up,
        });
        // Register in every bucket the stroke can reach, so lookup is O(1).
        let r = if level {
            radius * 1.7 + 1.0
        } else {
            radius + 1.0
        };
        let lo = key([c[0] - r, c[1] - r, c[2] - r]);
        let hi = key([c[0] + r, c[1] + r, c[2] + r]);
        for x in lo.0..=hi.0 {
            for y in lo.1..=hi.1 {
                for z in lo.2..=hi.2 {
                    self.index.entry((x, y, z)).or_default().push(id);
                }
            }
        }
    }

    /// Remove the most recent stroke. The bucket index is rebuilt rather than
    /// surgically unpicked: undo is rare, strokes are few, and correctness
    /// beats cleverness here.
    pub fn pop(&mut self) -> Option<Stroke> {
        let s = self.strokes.pop()?;
        self.index.clear();
        let strokes = std::mem::take(&mut self.strokes);
        for (i, st) in strokes.iter().enumerate() {
            let r = if st.level {
                st.radius * 1.7 + 1.0
            } else {
                st.radius + 1.0
            };
            let lo = key([st.c[0] - r, st.c[1] - r, st.c[2] - r]);
            let hi = key([st.c[0] + r, st.c[1] + r, st.c[2] + r]);
            for x in lo.0..=hi.0 {
                for y in lo.1..=hi.1 {
                    for z in lo.2..=hi.2 {
                        self.index.entry((x, y, z)).or_default().push(i as u32);
                    }
                }
            }
        }
        self.strokes = strokes;
        Some(s)
    }

    /// Apply every stroke touching `p` to the procedural density `d`.
    #[inline]
    pub fn apply(&self, p: [f32; 3], mut d: f32) -> f32 {
        if self.strokes.is_empty() {
            return d;
        }
        if let Some(ids) = self.index.get(&key(p)) {
            for &i in ids {
                let s = self.strokes[i as usize];
                let rel = [p[0] - s.c[0], p[1] - s.c[1], p[2] - s.c[2]];
                if s.level {
                    // Height above the brush plane, and distance from its axis.
                    let h = rel[0] * s.up[0] + rel[1] * s.up[1] + rel[2] * s.up[2];
                    let ax = [
                        rel[0] - s.up[0] * h,
                        rel[1] - s.up[1] * h,
                        rel[2] - s.up[2] * h,
                    ];
                    let rad = (ax[0] * ax[0] + ax[1] * ax[1] + ax[2] * ax[2]).sqrt();
                    let disc = s.radius - rad;
                    if s.dig {
                        // Remove everything above the plane inside the disc.
                        let b = disc.min(h);
                        d = d.min(-b);
                    } else {
                        // Fill up to the plane, bounded one radius below it.
                        let b = disc.min(-h).min(h + s.radius);
                        d = d.max(b);
                    }
                } else {
                    let dist = (rel[0] * rel[0] + rel[1] * rel[1] + rel[2] * rel[2]).sqrt();
                    if s.dig {
                        d = d.min(dist - s.radius);
                    } else {
                        d = d.max(s.radius - dist);
                    }
                }
            }
        }
        d
    }
}
