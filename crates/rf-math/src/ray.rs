use crate::Aabb;
use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }

    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    pub fn intersect_aabb(&self, aabb: &Aabb) -> Option<f32> {
        let inv_dir = self.direction.recip();
        let t0 = (aabb.min - self.origin) * inv_dir;
        let t1 = (aabb.max - self.origin) * inv_dir;

        let tmin = t0.min(t1);
        let tmax = t0.max(t1);

        let t_enter = tmin.max_element();
        let t_exit = tmax.min_element();

        if t_exit >= t_enter.max(0.0) {
            Some(t_enter.max(0.0))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_box_in_front() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::Z);
        let aabb = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        let hit = ray.intersect_aabb(&aabb);
        assert!(hit.is_some());
        assert!((hit.unwrap() - 4.0).abs() < 1e-4);
    }

    #[test]
    fn ray_misses_box_behind() {
        let ray = Ray::new(Vec3::new(10.0, 10.0, 10.0), Vec3::Z);
        let aabb = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        assert!(ray.intersect_aabb(&aabb).is_none());
    }
}
