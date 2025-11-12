/*

    Axis Aligned Bounding Box and Bounding Volume Hierarchy.

    
    @author: bartu
    @date: 9 Nov, 2025
*/


use crate::prelude::*;
use crate::ray::{Ray};
use crate::interval::{Interval};
use crate::json_structs::{VertexData};

#[derive(Debug)]
pub struct BBox {
    pub xmin: Float, 
    pub xmax: Float,
    pub ymin: Float, 
    pub ymax: Float,
    pub zmin: Float, 
    pub zmax: Float,
    
    pub width: Float, // TODO: Are these actually needed? 
    pub height: Float,
    pub depth: Float,
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
            width: xmax - xmin,
            height: ymax - ymin,
            depth: zmax - zmin,
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
}

pub trait BBoxable {
    fn get_bbox(&self, verts: &VertexData) -> BBox;
}
