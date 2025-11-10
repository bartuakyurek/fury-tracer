/*

    Axis Aligned Bounding Box and Bounding Volume Hierarchy.

    
    @author: bartu
    @date: 9 Nov, 2025
*/


use crate::prelude::*;

use crate::interval::{FloatConst, Interval};
use crate::json_structs::{VertexData};
use crate::ray::{Ray, HitRecord};
use crate::shapes::Shape;

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

// ====================================================================================================
// Bounding Volume Hierarchy
// ====================================================================================================
// Intersection logic follows Slides 03_acceleration_structures p.64
// and the binary tree creation is inspired by:
// https://google.github.io/comprehensive-rust/smart-pointers/exercise.html


struct Subtree<T>(Option<Arc<Node<T>>>); // Inspired by the binary tree exercise (see link in header)

struct Node<T> {
    value: T,
    left: Subtree<T>,
    right: Subtree<T>,
}

pub type BVHSubtree = Subtree<BBox>;
pub type BVHNode = Node<BBox>;

impl BVHSubtree {
    pub fn intersect(&self, ray: &Ray, rec: &mut HitRecord) -> bool {
        
        match &self.0 {
            None => { return false; }
            Some(node) => {
                
                if node.value.intersect(ray) {
                    
                    let mut rec1 = HitRecord::default();
                    let mut rec2 = HitRecord::default();

                    rec.ray_t = FloatConst::INF;

                    let hitleft: bool = node.left.intersect(ray, &mut rec1);
                    let hitright: bool = node.right.intersect(ray, &mut rec2);

                    if hitleft { *rec = rec1; }
                    if hitright { *rec = rec2; }
                    return hitleft || hitright;
                }
                else {
                    // The ray entirely misses this bounding box
                    return false;
                }
            }
        }

    }
}