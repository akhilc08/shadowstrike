use crate::fixed::FixedPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AABB {
    pub x: FixedPoint,
    pub y: FixedPoint,
    pub w: FixedPoint,
    pub h: FixedPoint,
}

impl AABB {
    pub fn new(x: FixedPoint, y: FixedPoint, w: FixedPoint, h: FixedPoint) -> Self {
        AABB { x, y, w, h }
    }
}

pub fn overlaps(a: &AABB, b: &AABB) -> bool {
    let a_right = a.x + a.w;
    let b_right = b.x + b.w;
    let a_bottom = a.y + a.h;
    let b_bottom = b.y + b.h;

    a.x < b_right && a_right > b.x && a.y < b_bottom && a_bottom > b.y
}

pub fn check_hit(
    attacker_hitboxes: &[Option<AABB>],
    defender_hurtboxes: &[Option<AABB>],
) -> bool {
    for hit in attacker_hitboxes {
        if let Some(h) = hit {
            for hurt in defender_hurtboxes {
                if let Some(d) = hurt {
                    if overlaps(h, d) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_overlap() {
        let a = AABB::new(
            FixedPoint::from_int(0),
            FixedPoint::from_int(0),
            FixedPoint::from_int(10),
            FixedPoint::from_int(10),
        );
        let b = AABB::new(
            FixedPoint::from_int(5),
            FixedPoint::from_int(5),
            FixedPoint::from_int(10),
            FixedPoint::from_int(10),
        );
        assert!(overlaps(&a, &b));

        // Non-overlapping
        let c = AABB::new(
            FixedPoint::from_int(20),
            FixedPoint::from_int(20),
            FixedPoint::from_int(5),
            FixedPoint::from_int(5),
        );
        assert!(!overlaps(&a, &c));

        // Edge-touching (not overlapping)
        let d = AABB::new(
            FixedPoint::from_int(10),
            FixedPoint::from_int(0),
            FixedPoint::from_int(5),
            FixedPoint::from_int(5),
        );
        assert!(!overlaps(&a, &d));
    }

    #[test]
    fn test_check_hit() {
        let hit = AABB::new(
            FixedPoint::from_int(5),
            FixedPoint::from_int(5),
            FixedPoint::from_int(10),
            FixedPoint::from_int(10),
        );
        let hurt = AABB::new(
            FixedPoint::from_int(8),
            FixedPoint::from_int(8),
            FixedPoint::from_int(10),
            FixedPoint::from_int(10),
        );
        assert!(check_hit(&[Some(hit)], &[Some(hurt)]));
        assert!(!check_hit(&[None], &[Some(hurt)]));
        assert!(!check_hit(&[Some(hit)], &[None]));
    }
}
