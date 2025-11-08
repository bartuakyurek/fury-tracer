/*


@date: Oct-Nov 2025
@author: Bartu

*/

use crate::json_structs::{FaceType, VertexData};
use crate::geometry::{get_tri_normal};
use crate::shapes::{Triangle};
use crate::prelude::*;


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

}

impl Mesh {
    
    // Helper function to convert a Mesh into individual Triangles
    pub fn to_triangles(&self, verts: &VertexData, id_offset: usize) -> Vec<Triangle> {
        
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
