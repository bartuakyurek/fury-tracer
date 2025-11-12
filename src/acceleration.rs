


use crate::prelude::*;
use crate::json_structs::VertexData;
use crate::shapes::{Shape, HeapAllocatedShape};
use crate::ray::{Ray, HitRecord};
use crate::bbox::{BBox, BBoxable};
use crate::interval::{Interval, FloatConst};
use crate::scene::{HeapAllocatedVerts};

// ====================================================================================================
// Bounding Volume Hierarchy
// ====================================================================================================
// Binary tree creation is inspired by:
// https://google.github.io/comprehensive-rust/smart-pointers/exercise.html


/// BVH node storing a bounding box, optional children, and a list of objects for leaves.
#[derive(Debug)]
pub struct BVHNode {
    pub bbox: BBox,
    pub left: Option<BVHNodePtr>,
    pub right: Option<BVHNodePtr>,
    pub objects: Vec<HeapAllocatedShape>,
}

/// BVHSubtree is a wrapper around an optional root node.
#[derive(Debug, Clone)]
pub struct BVHSubtree(pub Option<BVHNodePtr>);

pub type BVHNodePtr = Arc<BVHNode>;

impl BVHSubtree {

    /// Recursively builds nodes in BVH tree
    /// TODO: it was meant to be inside build( ) function but inner functions cannot use generics from the outer
    /// as rustc told, so I'm moving it here.
    fn build_nodes<T>(mut items: Vec<(Arc<T>, BBox, Vector3)>) -> Option<BVHNodePtr> {
            
        if items.is_empty() { return None; } // Base case

        let mut unified_bbox = items[0].1.clone(); // Get the first bounding box, skip it in the following iter, clone because cannot muve this index out of our input Vec 
        for (_, other_bbox, _) in items.iter().skip(1) {
            unified_bbox = unified_bbox.merge(other_bbox);
        }

        const LEAF_SIZE: usize = 4; // TODO: should we move it inside subtree struct so that it's not a magic constant to set here?
        if items.len() <= LEAF_SIZE {
            let node_objects: Vec<Arc<T>> = items.into_iter().map(|(s, _, _)| s).collect(); // NOTE: This is called *consuming*, ownership of items is moved to node_objects but this is fine because we are about to return
            //return OptionalBVHNodePtr::Some() ok this is where declaring a new type is not really useful... I still need to use Arc::new or declare another type and nest the type declarations which is... ugly.
            return Some(BVHNodePtr::new(BVHNode { bbox: unified_bbox, left: None, right: None, objects: node_objects }));

        }
    
        todo!()
    }

    /// Build a BVH from a list of shapes using their bounding boxes.
    /// verts needed for get_bbox( ) called inside, since shapes only store indices, 
    /// not the actual verts. 
    pub fn build<T>(shapes: &Vec<Arc<T>>, verts: &VertexData) -> Self // shapes is a vector of pointers because cloning the whole shape would be costly, it's like HeapAllocatedShape type in shapes.rs but now with generics 
        where 
            T: Shape + BBoxable,
    {
        
        if shapes.is_empty() {
            return BVHSubtree(None);
        }

        // Precompute for sorting: (shape pointer, its bbox, bbox centroid)
        let mut items: Vec<(Arc<T>, BBox, Vector3)> = Vec::with_capacity(shapes.len());
        for s in shapes.iter() {
            let bbox = s.get_bbox(verts);
            let center = bbox.get_center();
            items.push((s.clone(), bbox, center)); // clone the pointer, *s doesn't work because "s is behind a shared reference" as rustc states
        }
        
        // Recursively create nodes 
        BVHSubtree(Self::build_nodes::<T>(items))
    }

    /// Intersect a ray with the BVH. 
    /// Returns true if any hit was found and mutates hitrecord to closest hit.
    /// TODO: Now this is literally the same as Shape, BVHSubtree itself could impl Shape 
    pub fn intersect(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts, rec: &mut HitRecord) -> bool {
        
        // NOTE: See the following link for this match &self.0 pattern here
        // https://google.github.io/comprehensive-rust/smart-pointers/solution.html
        match &self.0 {
            None => false,
            Some(root) => { 
                
                rec.ray_t = FloatConst::INF; // For BVH, we shoot to infinity, right? Well yes that's also true for bbox intersections
                
                // Introduce helper function to recursively traverse the tree 
                // Because calling intersect( ) directly 
                fn walk(node: &Arc<BVHNode>, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts, closest: &mut Option<HitRecord>) {
                    if !node.bbox.intersect(ray) { return; }  // This is the base case return for recursive helper, not the outer intersect( )!
                                                              
                    if node.objects.is_empty() {
                        if let Some(l) = &node.left { walk(l, ray, t_interval, vertex_cache, closest); }
                        if let Some(r) = &node.right { walk(r, ray, t_interval, vertex_cache, closest); }
                    } else {
                        // Reached to leaf node (remember only leaf nodes have objects) 
                        // TODO: This is the same as what we did in HW1, iterating all the shapes, it could have been called hit_naive()
                        // or with a better name, perhaps inside Shape trait with this default implementation. 
                        for obj in &node.objects {
                            if let Some(hit) = obj.intersects_with(ray, t_interval, vertex_cache) {
                                if let Some(existing) = &closest {
                                    if hit.ray_t < existing.ray_t {
                                        *closest = Some(hit);
                                    }
                                } else {
                                    *closest = Some(hit);
                                }
                            }
                        }
                    } 
                }

                // TODO: Could we avoid this deeply nested statements if intersect( ) allowed mut HitRecord inside instead of returning Option<HitRecord>?
                // Because currently it is totally unreadable with all the if let if let if let expressions.
                let mut closest: Option<HitRecord> = None;
                walk(root, ray, t_interval, vertex_cache, &mut closest);
                if let Some(h) = closest {
                    *rec = h;
                    true
                } else {
                    false
                }   
            }
        }
        
    }
}