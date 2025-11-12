


use crate::prelude::*;
use crate::json_structs::VertexData;
use crate::shapes::{Shape, HeapAllocatedShape};
use crate::ray::{Ray, HitRecord};
use crate::bbox::{BBox, BBoxable};


// ====================================================================================================
// Bounding Volume Hierarchy
// ====================================================================================================
// Binary tree creation is inspired by:
// https://google.github.io/comprehensive-rust/smart-pointers/exercise.html


/// BVH node storing a bounding box, optional children, and a list of objects for leaves.
#[derive(Debug)]
pub struct BVHNode {
    pub bbox: BBox,
    pub left: Option<Arc<BVHNode>>,
    pub right: Option<Arc<BVHNode>>,
    pub objects: Vec<HeapAllocatedShape>,
}

/// BVHSubtree is a wrapper around an optional root node.
#[derive(Debug, Clone)]
pub struct BVHSubtree(pub Option<Arc<BVHNode>>);

impl BVHSubtree {
    /// Build a BVH from a list of shapes using their bounding boxes.
    /// verts needed for get_bbox( ) called inside, since shapes only store indices, 
    /// not the actual verts. 
    pub fn build<T>(shapes: &Vec<T>, verts: &VertexData) -> Self 
        where 
            T: Shape + BBoxable,
    {
        
        if shapes.is_empty() {
            return BVHSubtree(None);
        }

    }

    /// Intersect a ray with the BVH. Returns true if any hit was found and fills `rec` with the closest hit.
    pub fn intersect(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &crate::scene::HeapAllocatedVerts, rec: &mut HitRecord) -> bool {
        
        false
    }
}