/*

UPDATE: Acceleration structure added Mesh::bvh

@date: Oct-Nov 2025
@author: Bartu

*/

use crate::json_structs::{FaceType, VertexData};
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
    pub(crate) _base_mesh_id: usize,

    #[serde(rename = "_resetTransform", deserialize_with = "deser_bool")]
    pub(crate) _reset_transform: bool,

    #[serde(rename = "Material", deserialize_with = "deser_usize")]
    pub(crate) material_id: usize,
    
    #[serde(rename = "Transformations")]
    pub(crate) transformation_names: String,

    #[serde(skip)]
    pub(crate) matrix: Arc<Matrix4>, // WARNING: This should apply its M_instance on M_base

    //#[serde(skip)]
    //pub inv_matrix: Arc<Matrix4>,
}


#[derive(Debug, Deserialize, Clone)]
#[derive(SmartDefault)]
#[serde(default)]
pub struct Mesh {
    #[serde(deserialize_with = "deser_usize")]
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
    pub matrix: Arc<Matrix4>,

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
                is_smooth: self._shading_mode.to_ascii_lowercase() == "smooth",
                normal: get_tri_normal(&v1, &v2, &v3),

                transformation_names: self.transformation_names.clone(),
                matrix: Some(self.matrix.clone()), // NOTE: here it is ok to .clone( ) because it just increases Arc's counter, not cloning the whole data
            });
        }
        
        triangles
    }

    fn intersect_naive(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
        // Delegate intersection test to per-mesh Triangle objects 
        // by iterating over all the triangles (hence naive, accelerated intersection function is to be added soon)
        let mut closest: Option<HitRecord> = None;
        let mut t_min = std::f64::INFINITY as crate::numeric::Float;

        for tri in self.triangles.iter() {
            if let Some(hit) = tri.intersects_with(ray, t_interval, vertex_cache) {
                if hit.ray_t < t_min {
                    t_min = hit.ray_t;
                    closest = Some(hit);
                }
            }
        }
        closest
    }

}

impl Shape for Mesh {
    
    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
        let rec = {
            if let Some(bvh) = &self.bvh {
                let mut closest = HitRecord::default();    
                if bvh.intersect(ray, t_interval, &vertex_cache, &mut closest) {
                    Some(closest)
                }
                else {
                    None
                }
            } 
            else {
                self.intersect_naive(ray, t_interval, vertex_cache)
            }
        };

        // Transform the local intersection point and normal
        // See slides 04, p.51
        if let Some(mut hit) = rec {
            hit.hit_point = transform_point(&self.matrix, &hit.hit_point);

            // WARNING: only use upper 3x3, see p.53 
            // TODO: cache it?
            let mat3 = Matrix3::from_mat4(*self.matrix);
            let inv = mat3.inverse().transpose();
            hit.normal = inv * hit.normal; 
            Some(hit)
             // WARNING: How about entry_point, should we inverse transform it? it's used for dielectric
        } 
        else {
            None
        }
        
       
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
            info!("Applying transform for TLAS {}", self.matrix);
           local_box.transform(&self.matrix)
        } else {
            local_box
        }
    }
}

