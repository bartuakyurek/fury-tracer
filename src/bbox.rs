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
    xmin: Float, xmax: Float,
    ymin: Float, ymax: Float,
    zmin: Float, zmax: Float,
    
    width: Float,
    height: Float,
    depth: Float,
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
        todo!()
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