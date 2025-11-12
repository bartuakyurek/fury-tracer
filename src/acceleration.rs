


use crate::prelude::*;
use crate::shapes::Shape;
use crate::ray::{Ray, HitRecord};
use crate::bbox::BBox;


// ====================================================================================================
// Bounding Volume Hierarchy
// ====================================================================================================
// Binary tree creation is inspired by:
// https://google.github.io/comprehensive-rust/smart-pointers/exercise.html


struct Subtree<T>(Option<Arc<Node<T>>>); // Inspired by the binary tree exercise (see link in header)

struct Node<T> {
    value: T,
    left: Subtree<T>,
    right: Subtree<T>,
}

pub type HeapBBoxable = Arc<dyn BBoxable>;
pub type BVHSubtree = Subtree<HeapBBoxable>;
pub type BVHNode = Node<HeapBBoxable>;

impl BVHSubtree {

    /// Construct BVH binary tree given a list of bounding-boxable
    // NOTE: BBoxable has get_bbox( ) though I'm not sure why I wanted use
    // generics here, instead of plain Vec<BBox>. Perhaps it'd be useful for
    // other non-AABB bounding boxes yet it's just an overkill at the moment?
    // just doing it for the sake of Rust practice
    // Actually it is useful for Scene to not call .getbbox( ) rather it just
    // holds scene objects in which some are bboxable, making this function flexible.
    pub fn build<T>(bbox_list: &Vec<T>) -> Self 
    where 
        T: BBoxable,
    {
        todo!()
    }

    pub fn intersect(&self, ray: &Ray, rec: &mut HitRecord) -> bool {
        // Refers Slides 03_acceleration_structures p.64
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