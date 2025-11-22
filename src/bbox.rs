/*

    Axis Aligned Bounding Box (AABB)
    
    @author: bartu
    @date: 9 Nov, 2025
*/


use crate::prelude::*;
use crate::ray::{Ray};
use crate::interval::{Interval};
use crate::json_structs::{VertexData};

#[derive(Debug, Clone)]
pub struct BBox {
    pub xmin: Float, 
    pub xmax: Float,
    pub ymin: Float, 
    pub ymax: Float,
    pub zmin: Float, 
    pub zmax: Float,
}

// TODO: Ideally, BBox should impl Shape but since intersect( ) signatures are different
// perhaps Shape trait coud have two function does_intersect( ) and intersect( ) to provide
// support to both boolean and hitrecord returns.
impl BBox {

    pub fn new(xmin: Float, xmax: Float, ymin: Float, ymax: Float, zmin: Float, zmax: Float) -> Self {
        debug_assert!(xmax >= xmin);
        debug_assert!(ymax >= ymin);
        debug_assert!(zmax >= zmin);
        Self {
            xmin,
            xmax,
            ymin,
            ymax,
            zmin,
            zmax,
        }
    }

    pub fn new_from(xint: &Interval, yint: &Interval, zint: &Interval) -> Self {
        
        assert!(xint.validate() && yint.validate() && zint.validate(), "Invalid interval, found max < min");
        Self::new(
            xint.min,
            xint.max,
            yint.min,
            yint.max,
            zint.min,
            zint.max,
        )
    }

    pub fn empty() -> Self {
        Self {
            xmin: Float::INFINITY,
            xmax: Float::NEG_INFINITY,
            ymin: Float::INFINITY,
            ymax: Float::NEG_INFINITY,
            zmin: Float::INFINITY,
            zmax: Float::NEG_INFINITY,
        }
    }

    /// Merge two bboxes into a single one by
    /// comparing their extents
    pub fn merge(&self, other: &Self) -> Self {
        Self::new(
            self.xmin.min(other.xmin),
            self.xmax.max(other.xmax),
            self.ymin.min(other.ymin),
            self.ymax.max(other.ymax),
            self.zmin.min(other.zmin),
            self.zmax.max(other.zmax),
        )
    }

    pub fn get_largest_extents(bboxes: &Vec<&Self>) -> (Float, Float, Float) {

        let merged: Self = bboxes.iter().fold(Self::empty(), |acc, b| acc.merge(&b));
        (
            merged.xmax - merged.xmin,
            merged.ymax - merged.ymin,
            merged.zmax - merged.zmin,
        )
    }

    pub fn get_center(&self) -> Vector3 {
         Vector3::new((self.xmin + self.xmax) * 0.5, (self.ymin + self.ymax) * 0.5, (self.zmin + self.zmax) * 0.5)
    }

    pub fn intersect(&self, ray: &Ray) -> bool {
        // See slides 03, p.5-6
        
        debug_assert!(ray.direction.is_normalized());
        
        // Helper function for p.5
        let slab_intersect = |min: Float, max: Float, o: Float, d: Float| -> (Float , Float) {
            let mut t1 = (min - o) / d;
            let mut t2 = (max - o) / d;
            if t2 < t1 {
                std::mem::swap(&mut t1, &mut t2);
            }
            (t1, t2)
        };

        // Intersections for X Y Z slabs
        let (t1x, t2x) = slab_intersect(self.xmin, self.xmax, ray.origin.x, ray.direction.x);
        let (t1y, t2y) = slab_intersect(self.ymin, self.ymax, ray.origin.y, ray.direction.y);
        let (t1z, t2z) = slab_intersect(self.zmin, self.zmax, ray.origin.z, ray.direction.z);

        // p.6
        let t1: Float = t1x.max(t1y).max(t1z);
        let t2: Float = t2x.min(t2y).min(t2z);

        // WARNING: It does not save enterance and exit points atm, just bool returned
        t1 <= t2 
    }

    /// Transform the bounding box given a 4x4 matrix
    pub fn transform(&self, matrix: &Matrix4) -> Self {
        // TODO: why not directly apply transform to points? do we really need to compute new extents?
        let corners = [
            Vector3::new(self.xmin, self.ymin, self.zmin),
            Vector3::new(self.xmin, self.ymin, self.zmax),
            Vector3::new(self.xmin, self.ymax, self.zmin),
            Vector3::new(self.xmin, self.ymax, self.zmax),
            Vector3::new(self.xmax, self.ymin, self.zmin),
            Vector3::new(self.xmax, self.ymin, self.zmax),
            Vector3::new(self.xmax, self.ymax, self.zmin),
            Vector3::new(self.xmax, self.ymax, self.zmax),
        ];

        let transformed_corners: Vec<Vector3> = corners
            .iter()
            .map(|corner| matrix.transform_point3(*corner))
            .collect();

        // Find transformed extents and new corners
        let (mut xmin, mut xmax) = (Float::INFINITY, Float::NEG_INFINITY);
        let (mut ymin, mut ymax) = (Float::INFINITY, Float::NEG_INFINITY);
        let (mut zmin, mut zmax) = (Float::INFINITY, Float::NEG_INFINITY);

        for corner in transformed_corners {
            xmin = xmin.min(corner.x);
            xmax = xmax.max(corner.x);
            ymin = ymin.min(corner.y);
            ymax = ymax.max(corner.y);
            zmin = zmin.min(corner.z);
            zmax = zmax.max(corner.z);
        }

        Self::new(xmin, xmax, ymin, ymax, zmin, zmax)
    }
}

pub trait BBoxable {
    fn get_bbox(&self, verts: &VertexData, apply_t: bool) -> BBox;
}
