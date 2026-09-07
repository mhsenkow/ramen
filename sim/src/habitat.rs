//! Habitat geometry and the parameters that define it.
//! Every number the world depends on lives here, not scattered as constants.
//! (REQUIREMENTS.md G — "settings exist and were just decided")

#[derive(Clone, Copy, Debug)]
pub struct Habitat {
    /// Hull radius, metres. Ground sits at or below this, toward the axis.
    pub radius: f32,
    /// Axial length, metres.
    pub length: f32,
    /// Spin rate, rad/s. Chosen so that omega^2 * radius = target gravity.
    pub omega: f32,
    /// Maximum terrain elevation above the hull floor, metres.
    pub max_elevation: f32,
    /// Water plane, metres above hull floor.
    pub water_level: f32,
    pub seed: u32,
}

impl Habitat {
    /// Kepler Drum — the tutorial habitat (PD_BRIEF.md §5).
    pub fn kepler_drum() -> Self {
        let radius = 900.0;
        let target_g = 9.81 * 0.95; // colony standard: 0.95g at the hull
        Self {
            radius,
            length: 6000.0,
            omega: (target_g / radius).sqrt(),
            max_elevation: 235.0,
            water_level: 22.0,
            seed: 0x5A1D_0C0A,
        }
    }

    /// Gravity magnitude at radius r (m/s^2). Zero at the axis, max at the hull.
    #[inline]
    pub fn gravity_at(&self, r: f32) -> f32 { self.omega * self.omega * r }

    /// Surface gravity — what the player feels standing on the ground.
    pub fn surface_gravity(&self) -> f32 { self.gravity_at(self.radius) }

    /// One rotation, seconds.
    pub fn spin_period(&self) -> f32 { std::f32::consts::TAU / self.omega }

    /// Cylindrical (theta, z, r) -> world (x, y, z). Axis is world Z.
    #[inline]
    pub fn to_world(&self, theta: f32, z: f32, r: f32) -> [f32; 3] {
        [r * theta.cos(), r * theta.sin(), z]
    }

    /// World -> (theta, z, r).
    #[inline]
    pub fn to_cyl(&self, p: [f32; 3]) -> (f32, f32, f32) {
        (p[1].atan2(p[0]), p[2], (p[0] * p[0] + p[1] * p[1]).sqrt())
    }

    /// "Up" for an occupant — toward the axis, i.e. inward.
    #[inline]
    pub fn up_at(&self, p: [f32; 3]) -> [f32; 3] {
        let m = (p[0] * p[0] + p[1] * p[1]).sqrt().max(1e-6);
        [-p[0] / m, -p[1] / m, 0.0]
    }

    /// Sagitta across a chunk of arc-length `c` — the flat-grid justification
    /// from EM_BRIEF.md §4 / REQUIREMENTS.md A3.
    pub fn chunk_sagitta(&self, chord: f32) -> f32 {
        chord * chord / (8.0 * self.radius)
    }
}
