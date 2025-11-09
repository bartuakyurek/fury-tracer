/*


@date: Oct-Nov 2025
@author: Bartu

*/

use crate::json_structs::{FaceType, VertexData};
use crate::geometry::{get_tri_normal};
use crate::shapes::{Triangle};
use crate::prelude::*;
use crate::shapes::PrimitiveShape;
use crate::geometry::HeapAllocatedVerts;
use crate::ray::{Ray, HitRecord};
use crate::interval::Interval;


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
    #[serde(skip)]
    pub triangles: Vec<Triangle>,

}

impl Mesh {
    
    /// Given global vertex data and id_offset, 
    /// Populate self.triangles with a vector, and
    /// return the vector of the created triangles.
    pub fn setup_triangles_vec(&mut self, verts: &VertexData, id_offset: usize) -> Vec<Triangle> {
        let triangles: Vec<Triangle> = self.to_triangles(verts, id_offset);
        self.triangles = triangles.clone();
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
            });
        }
        
        triangles
    }

}

impl PrimitiveShape for Mesh {
    fn indices(&self) -> Vec<usize> {
        self.faces._data.clone()
    }

    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
        // Delegate intersection test to per-mesh Triangle objects (avoids duplicating the
        // Möller–Trumbore code / normal interpolation logic which is implemented in Triangle).
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
