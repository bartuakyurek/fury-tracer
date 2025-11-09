/*


@date: Oct-Nov 2025
@author: Bartu

*/
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{io::BufReader, fs::File};

use crate::json_structs::{FaceType, VertexData, PlyMesh};
use crate::geometry::{get_tri_normal};
use crate::shapes::{Triangle};
use crate::prelude::*;

fn get_ply_path(jsonpath: &Path, ply_filename: String) -> PathBuf {

    let json_dir = Path::new(jsonpath)
                            .parent()
                            .unwrap_or(Path::new("."));

    let ply_path = json_dir.join(ply_filename);
    
    if ply_path.exists() {
        info!("PLY file exists: {:?}", ply_path);
    } else {
        error!("PLY file NOT found at: {:?}", ply_path);
    }
    ply_path
}

fn load_ply(ply_path: &PathBuf) -> Result<PlyMesh, Box<dyn Error>> {
    let file = File::open(ply_path)?;
    let reader = BufReader::new(file);
    let plymesh: PlyMesh = serde_ply::from_reader(reader)?;
    Ok(plymesh)
}

pub struct Mesh {
    pub _id: usize,
    pub material_idx: usize,
    pub tri_faces: Vec<usize>,
    pub _shading_mode: String,
}


impl Mesh { // TODO: all these JSON and their actual structs could derive a trait and implement it.
    pub fn new_from(mj: &MeshJSON, jsonpath: &Path) -> Self {

        mj.setup(jsonpath); 

        Self {
            _id: mj._id,
            material_idx: mj.material_idx,
            tri_faces: mj.faces._data.clone(),
            _shading_mode: mj._shading_mode.clone(),
        }
    }

    // Helper function to convert a Mesh into individual Triangles
    pub fn get_triangles(&self, verts: &VertexData, id_offset: usize) -> Vec<Triangle> {
        
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

