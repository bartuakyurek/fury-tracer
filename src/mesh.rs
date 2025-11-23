/*

UPDATE: Acceleration structure added Mesh::bvh

@date: Oct-Nov 2025
@author: Bartu

*/


use crate::json_structs::{FaceType, SingleOrVec, VertexData};
use crate::geometry::{get_tri_normal};
use crate::shapes::{Shape, Triangle};
use crate::ray::{Ray, HitRecord};
use crate::interval::Interval;
use crate::bbox::{BBoxable, BBox};
use crate::scene::{HeapAllocatedVerts};
use crate::acceleration::BVHSubtree;
use crate::shapes::ShapeList;

use crate::prelude::*;


#[derive(Debug, Deserialize, Clone)]
#[derive(SmartDefault)]
pub struct MeshInstanceField {
    
    #[serde(deserialize_with = "deser_usize")]
    pub(crate) _id: usize,
    
    #[serde(rename = "_baseMeshId", deserialize_with = "deser_usize")]
    pub(crate) base_mesh_id: usize,

    #[serde(rename = "_resetTransform", deserialize_with = "deser_bool", default)]
    #[default = false]
    pub(crate) reset_transform: bool,

    #[serde(rename = "Material", deserialize_with = "deser_opt_usize", default)]
    pub(crate) material_id: Option<usize>,
    
    #[serde(rename = "Transformations")]
    pub(crate) transformation_names: String,

    #[serde(skip)]
    pub(crate) matrix: Matrix4, // WARNING: This should apply its M_instance on M_base

    #[serde(skip)]
    pub base_mesh: Option<Arc<Mesh>>, // wrapped around Option to prevent default mesh construction
    // pointer to base mesh because trait impls need to access the actual mesh
    //pub inv_matrix: Arc<Matrix4>,
}

impl MeshInstanceField {
    pub fn setup_mesh_pointers(&mut self, base_meshes: &SingleOrVec<Mesh>) {
    
        let mut flag = false;
        debug!(">> Searching for mesh with id {} for instancing...", self.base_mesh_id);
        for mesh in base_meshes.iter() { // TODO: this for loop could be converted to iter().map() 
            debug!("Mesh of id {}", mesh._id);
            if mesh._id == self.base_mesh_id {
                flag = true;
                self.base_mesh = Some(Arc::new(mesh.clone())); //TODO: iirc Arc is smart enough to not clone the whole Mesh but not sure??
                debug!("Set base_mesh {} ", self.base_mesh.clone().unwrap()._id);
                break;
            }
        }

        if !flag {
            warn!("Couldn't find base mesh id {} ", self.base_mesh_id);
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[derive(SmartDefault)]
#[serde(default)]
pub struct Mesh {
    #[serde(deserialize_with = "deser_usize")]
    #[default = 999999] // To see if I'm defaulting Mesh 
    pub _id: usize,
    
    #[serde(rename = "Material", deserialize_with = "deser_usize")]
    pub material_idx: usize,

    #[serde(rename = "Faces")]
    pub faces: FaceType,

    #[serde(rename = "_shadingMode")]
    #[default = "flat"]
    pub _shading_mode: String,

    #[serde(rename = "Transformations", default)]
    pub transformation_names: Option<String>,

    #[serde(skip)]
    pub matrix: Matrix4,

    //#[serde(skip)]
    //pub inv_matrix: Arc<Matrix4>,

    #[serde(skip)]
    pub triangles: ShapeList,
    #[serde(skip)]
    pub bvh: Option<BVHSubtree>,
}

impl Mesh {
    
    /// Given global vertex data and id_offset, 
    /// Populate self.triangles with a vector, and
    /// return the vector of the created triangles.
    pub fn setup(&mut self, verts: &VertexData, id_offset: usize) -> Vec<Triangle> {

        let triangles: Vec<Triangle> = self.to_triangles(verts, id_offset);
        
        self.triangles = triangles.clone()
                                  .into_iter()
                                  .map(|tri| Arc::new(tri) as Arc<dyn Shape>)
                                  .collect();

        // Build BVH for acceleration
        self.bvh = Some(BVHSubtree::build(&self.triangles, verts,false));

        triangles
    }


    /// Helper function to convert a Mesh into individual Triangles
    fn to_triangles(&self, verts: &VertexData, id_offset: usize) -> Vec<Triangle> {
        
        if self.faces._type != "triangle" {
            panic!(">> Expected triangle faces in mesh_to_triangles, got '{}'.", self.faces._type);
        }
        
        let n_faces = self.faces.len();
        let mut triangles = Vec::with_capacity(n_faces);
        
        for i in 0..n_faces {
            let indices = self.faces.get_indices(i);
            let [v1, v2, v3] = indices.map(|i| verts[i]);
            triangles.push(Triangle {
                _id: id_offset + i, 
                indices,
                material_idx: self.material_idx,
                is_smooth: self._shading_mode.eq_ignore_ascii_case("smooth"),
                normal: get_tri_normal(&v1, &v2, &v3),

                transformation_names: None,//self.transformation_names.clone(),
                matrix: None, //Some(Arc::new(self.matrix)), // NOTE: here it is ok to .clone( ) because it just increases Arc's counter, not cloning the whole data
            });
        }
        
        triangles
    }

    fn intersect_naive(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
        // Delegate intersection test to per-mesh Triangle objects 
        // by iterating over all the triangles (hence naive, accelerated intersection function is to be added soon)
        let mut closest: Option<HitRecord> = None;
        let mut t_min = Float::INFINITY;

        for tri in self.triangles.iter() {
            if let Some(hit) = tri.intersects_with(ray, t_interval, vertex_cache) 
                && hit.ray_t < t_min {
                    t_min = hit.ray_t;
                    closest = Some(hit);
                
            }
        }
        closest
    }

    fn intersect_bvh(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
         if let Some(bvh) = &self.bvh {
                let mut closest = HitRecord::default();    
                if bvh.intersect(ray, t_interval, vertex_cache, &mut closest) {
                    Some(closest)
                }
                else {
                    None
                }
            } 
            else {
                warn!("Intersecting naively.... this shouldn't happen.");
                self.intersect_naive(ray, t_interval, vertex_cache)
            }
    }

}


impl Shape for Mesh {
    
    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
        
        // Transform ray to local space
        let inv_matrix = self.matrix.inverse();
        let local_ray = ray.inverse_transform(&inv_matrix);

        // Intersect in local space
        let rec = self.intersect_bvh(&local_ray, t_interval, vertex_cache);

        rec.map(|mut r| {
            r.to_world(&self.matrix);
            r.ray_t = (r.hit_point - ray.origin).length(); //TODO: it's so easy to forget it, how to refactor?
            r
        }) // Added to reduce if let verbosity but it didn't reduce nesting above...
    }
}

impl BBoxable for Mesh {
    fn get_bbox(&self, verts: &VertexData, apply_t: bool) -> BBox {
        let (mut xint, mut yint, mut zint) = (Interval::EMPTY, Interval::EMPTY, Interval::EMPTY);

        // TODO: This is not an optimal way to get bbox, as it uses faces
        // to access vertices of a mesh but ideally we'd like to iterate 
        // vertices one-shot. As a solution Mesh struct could contain where its data begins
        // in global vertexdata, and number of verts, such that we can use this info
        // to access relevant vertices. This solution assumes vertexdata has one continuous
        // segment of data per scene object, which is a reasonable assumption. However in order
        // to implement it, if given Mesh in JSON has directly face data, one should save the min
        // and max indices occuring in face._data. On the downside, introducing extra variables in
        // deserialized struct requires additional function implementation to fill those variables,
        // e.g. setup( ) functions I've been using for these purposes. Since this has been a pattern
        // perhaps we could have a fromJSON trait with setup( ) and new_from( ) and impl it for 
        // Scene, Mesh etc.
        for &i in &self.faces._data { // FaceType does not impl Copy, so using & to borrow
            let v = verts[i];

            xint.expand(v.x);
            yint.expand(v.y);
            zint.expand(v.z);
        }

        // Transform bounding box for top-level BVH
        let local_box = BBox::new_from(&xint, &yint, &zint);
        if apply_t {
            debug!("Applying transform for TLAS {}", self.matrix);
           local_box.transform(&self.matrix)
        } else {
            local_box
        }
    }
}


// =======================================================================================================
// MeshInstanceField: Shape + BBoxable
// =======================================================================================================


impl Shape for MeshInstanceField {
    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
        
        let base_mesh = self.base_mesh.as_deref().unwrap();
        
        if self.reset_transform {
            let inv_instance = self.matrix.inverse();
            let local_ray = ray.inverse_transform(&inv_instance);
            
            // Intersect without applying base mesh's transform
            if let Some(mut hit) = base_mesh.intersect_bvh(&local_ray, t_interval, vertex_cache) {
                hit.material = self.material_id.unwrap_or(self.base_mesh.clone().unwrap().material_idx);
                hit.to_world(&self.matrix);  // this transforms normals and hitpoints p.53
                hit.ray_t = (hit.hit_point - ray.origin).length(); //TODO: it's so easy to forget it, how to refactor?

                Some(hit)
            } else {
                None
            }
        } 
        else {
            // Compose transforms: inv(M_instance * M_base)
            let composite_matrix = self.matrix * base_mesh.matrix;
            let inv_composite = composite_matrix.inverse();
            let local_ray = ray.inverse_transform(&inv_composite);
            
            // Intersect with BVH 
            if let Some(mut hit) = base_mesh.intersect_bvh(&local_ray, t_interval, vertex_cache) {
                hit.material = self.material_id.unwrap_or(self.base_mesh.clone().unwrap().material_idx);
                hit.to_world(&composite_matrix);  // this transforms normals and hitpoints p.53
                hit.ray_t = (hit.hit_point - ray.origin).length(); //TODO: it's so easy to forget it, how to refactor?

                Some(hit)
            } else {
                None
            }
        }
    }
}


impl BBoxable for MeshInstanceField {
    fn get_bbox(&self, verts: &VertexData, apply_t: bool) -> BBox {
        
        if let Some(base_mesh) = self.base_mesh.as_deref() {
            debug!("Retrieving bounding box for base mesh '{}' of instance '{}'", base_mesh._id, self._id);
            
            let local_box = base_mesh.get_bbox(verts, false);
            let mut composite = self.matrix;

            if !self.reset_transform{
                composite = composite * base_mesh.matrix;
            }
            
            if apply_t {
                    local_box.transform(&composite)
            } 
            else {
                    local_box
            }
        }
        else {
            panic!("Mesh instance {} is missing base mesh (Option set to None)", self._id);
        }
    }
}