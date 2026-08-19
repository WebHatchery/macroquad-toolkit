//! Small deterministic value-noise helpers for seeded maps.
//!
//! The interpolation and hashing live here so projects can share the
//! reproducibility guarantee without sharing their terrain vocabulary.

/// A smooth deterministic value field over integer coordinates.
pub fn seeded_value(seed: u64, x: i32, y: i32, scale: f32) -> f32 {
    let scale = scale.max(1.0);
    let fx = x as f32 / scale;
    let fy = y as f32 / scale;
    let x0 = fx.floor();
    let y0 = fy.floor();
    let tx = smooth(fx - x0);
    let ty = smooth(fy - y0);
    let (x0, y0) = (x0 as i64, y0 as i64);
    let top = lerp(corner(seed, x0, y0), corner(seed, x0 + 1, y0), tx);
    let bottom = lerp(corner(seed, x0, y0 + 1), corner(seed, x0 + 1, y0 + 1), tx);
    lerp(top, bottom, ty)
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn corner(seed: u64, x: i64, y: i64) -> f32 {
    let mut h = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h >> 40) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::seeded_value;

    #[test]
    fn field_is_deterministic_and_bounded() {
        let a = seeded_value(42, 7, -3, 4.0);
        assert_eq!(a, seeded_value(42, 7, -3, 4.0));
        assert!((0.0..=1.0).contains(&a));
    }
}
