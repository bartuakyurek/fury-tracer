/*

    Axis Aligned Bounding Box and Bounding Volume Hierarchy.

    
    @author: bartu
    @date: 9 Nov, 2025
*/


use crate::prelude::*;
use crate::ray::{Ray};
use crate::interval::{Interval};
use crate::json_structs::{VertexData};

pub struct BBox {
    pub xmin: Float, 
    pub xmax: Float,
    pub ymin: Float, 
    pub ymax: Float,
    pub zmin: Float, 
    pub zmax: Float,
    
    pub width: Float,
    pub height: Float,
    pub depth: Float,
}

// TODO: Ideally, BBox should impl Shape but since intersect( ) signatures are different
// perhaps Shape trait coud have two function does_intersect( ) and intersect( ) to provide
// support to both boolean and hitrecord returns.
impl BBox {
    pub fn new_from(xint: &Interval, yint: &Interval, zint: &Interval) -> Self {
        
        assert!(xint.validate() && yint.validate() && zint.validate(), "Invalid interval, found max < min");
        Self {
            xmin: xint.min,
            xmax: xint.max,
            ymin: yint.min,
            ymax: yint.max,
            zmin: zint.min,
            zmax: zint.max,
            width: xint.max - xint.min,
            height: yint.max - yint.min,
            depth: zint.max - zint.min,
        }
    }

    fn intersect(&self, ray: &Ray) -> bool {
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
