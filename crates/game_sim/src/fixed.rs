/// Fixed-point number type: i32 with 1/1000 pixel precision.
/// 1.0 pixel = 1000 units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedPoint(pub i32);

pub const SCALE: i32 = 1000;

impl FixedPoint {
    pub const ZERO: FixedPoint = FixedPoint(0);
    pub const ONE: FixedPoint = FixedPoint(SCALE);
    /// 0.5 pixels/frame^2 = 500 units
    pub const GRAVITY: FixedPoint = FixedPoint(500);

    pub fn from_int(n: i32) -> FixedPoint {
        FixedPoint(n * SCALE)
    }

    /// For initialization only, not gameplay.
    pub fn from_f32(f: f32) -> FixedPoint {
        FixedPoint((f * SCALE as f32) as i32)
    }

    /// For rendering only.
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / SCALE as f32
    }

    pub fn raw(self) -> i32 {
        self.0
    }
}

impl core::ops::Add for FixedPoint {
    type Output = FixedPoint;
    fn add(self, rhs: FixedPoint) -> FixedPoint {
        FixedPoint(self.0 + rhs.0)
    }
}

impl core::ops::AddAssign for FixedPoint {
    fn add_assign(&mut self, rhs: FixedPoint) {
        self.0 += rhs.0;
    }
}

impl core::ops::Sub for FixedPoint {
    type Output = FixedPoint;
    fn sub(self, rhs: FixedPoint) -> FixedPoint {
        FixedPoint(self.0 - rhs.0)
    }
}

impl core::ops::SubAssign for FixedPoint {
    fn sub_assign(&mut self, rhs: FixedPoint) {
        self.0 -= rhs.0;
    }
}

impl core::ops::Mul for FixedPoint {
    type Output = FixedPoint;
    fn mul(self, rhs: FixedPoint) -> FixedPoint {
        FixedPoint(((self.0 as i64 * rhs.0 as i64) / SCALE as i64) as i32)
    }
}

impl core::ops::Div for FixedPoint {
    type Output = FixedPoint;
    fn div(self, rhs: FixedPoint) -> FixedPoint {
        FixedPoint(((self.0 as i64 * SCALE as i64) / rhs.0 as i64) as i32)
    }
}

impl core::ops::Neg for FixedPoint {
    type Output = FixedPoint;
    fn neg(self) -> FixedPoint {
        FixedPoint(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_point_arithmetic() {
        let a = FixedPoint::from_int(3);
        let b = FixedPoint::from_int(2);

        assert_eq!((a + b).0, 5000);
        assert_eq!((a - b).0, 1000);
        assert_eq!((a * b).0, 6000);
        assert_eq!((a / b).0, 1500);
        assert_eq!((-a).0, -3000);

        // Edge cases
        assert_eq!((FixedPoint::ZERO + a).0, a.0);
        assert_eq!((a * FixedPoint::ONE).0, a.0);
        assert_eq!((a / FixedPoint::ONE).0, a.0);

        // Fractional
        let half = FixedPoint(500);
        assert_eq!((half * FixedPoint::from_int(4)).0, 2000);
    }
}
