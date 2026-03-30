use core::cmp::Ordering;
use core::ops::{Add, Mul, Neg, Sub};
use serde::{Deserialize, Serialize};

/// Fixed-point number: 1000 units = 1 pixel.
/// Range: roughly ±2,147,483 pixels — more than enough.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FixedPoint(pub i32);

impl FixedPoint {
    pub const SCALE: i32 = 1000;
    pub const ZERO: Self = Self(0);

    #[inline]
    pub const fn from_pixels(px: i32) -> Self {
        Self(px * Self::SCALE)
    }

    #[inline]
    pub const fn raw(v: i32) -> Self {
        Self(v)
    }

    #[inline]
    pub const fn to_raw(self) -> i32 {
        self.0
    }

    #[inline]
    pub const fn to_pixels(self) -> i32 {
        self.0 / Self::SCALE
    }
}

impl From<i32> for FixedPoint {
    #[inline]
    fn from(px: i32) -> Self {
        Self::from_pixels(px)
    }
}

impl Add for FixedPoint {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for FixedPoint {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Mul for FixedPoint {
    type Output = Self;
    /// Fixed × Fixed: result is (a*b)/SCALE to keep scale correct.
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self(((self.0 as i64 * rhs.0 as i64) / Self::SCALE as i64) as i32)
    }
}

impl Neg for FixedPoint {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl PartialOrd for FixedPoint {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FixedPoint {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_roundtrip() {
        let fp = FixedPoint::from_pixels(42);
        assert_eq!(fp.to_pixels(), 42);
        assert_eq!(fp.to_raw(), 42_000);
    }

    #[test]
    fn arithmetic() {
        let a = FixedPoint::from_pixels(10);
        let b = FixedPoint::from_pixels(3);
        assert_eq!((a + b).to_pixels(), 13);
        assert_eq!((a - b).to_pixels(), 7);
        assert_eq!((-a).to_pixels(), -10);
    }

    #[test]
    fn mul_fixed() {
        let a = FixedPoint::from_pixels(6);
        let b = FixedPoint::raw(500); // 0.5 pixels
        let result = a * b;
        assert_eq!(result.to_raw(), 3000); // 3 pixels
    }

    #[test]
    fn ordering() {
        let a = FixedPoint::from_pixels(5);
        let b = FixedPoint::from_pixels(10);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, a);
    }
}
