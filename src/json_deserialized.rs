
use std::error::Error;

use crate::json_structs::{*};
use crate::scene::{SceneMaterials};
use crate::camera::Cameras;
use crate::prelude::*;

#[derive(Debug, Deserialize)]
pub struct RootScene {
    #[serde(rename = "Scene")]
    pub scene: SceneJSON,
}

#[derive(Debug, Deserialize, SmartDefault)]
#[serde(rename_all = "PascalCase")]
#[serde(default)]
pub struct SceneJSON {
    #[default = 5]
    #[serde(deserialize_with = "deser_usize")]
    pub max_recursion_depth: usize,

    #[serde(deserialize_with = "deser_vec3")]
    pub background_color: Vector3,

    #[default = 1e-10]
    #[serde(deserialize_with = "deser_float")]
    pub shadow_ray_epsilon: Float,

    #[default = 1e-10]
    #[serde(deserialize_with = "deser_float")]
    pub intersection_test_epsilon: Float,

    #[serde(deserialize_with = "deser_string_or_struct")]
    pub vertex_data: VertexData, 

    pub cameras: Cameras,
    pub lights: SceneLightsJSON,
    pub materials: SceneMaterials,
    pub objects: SceneObjectsJSON,
}


#[derive(Debug, Deserialize, Clone)]
#[derive(SmartDefault)]
#[serde(default)]
pub struct MeshJSON {
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



impl MeshJSON {
    
    
    fn setup(&mut self, jsonpath: &Path) -> Result<(), Box<dyn Error>> {

        if !self.faces._ply_file.is_empty() { 
                
                let ply_path = get_ply_path(jsonpath, self.faces._ply_file.clone());
                info!("Loading mesh {} from PLY file path: {:?}", self._id, ply_path);
                
                let file = File::open(ply_path)?;
                let reader = BufReader::new(file);
                let plymesh: PlyMesh = serde_ply::from_reader(reader)?;
              
               
                if let Some(faces) = &plymesh.face {
                    self.faces._data = faces
                        .iter()
                        //.map(|idx| idx)
                        .flat_map(|f| f.vertex_indices.clone())      
                        .collect();
                    info!(">> Mesh {} has {} faces.", self._id, self.faces._data.len());
                } 
                else {
                    warn!("PLY mesh {} has no face data!", self._id);
                }
        }
        else {
            info!("Mesh face data is given in JSON (no PLY read).");   
        }
        Ok(())
    }

}



#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SceneLightsJSON {
    #[serde(rename = "AmbientLight", deserialize_with = "deser_vec3")]
    pub ambient_light: Vector3, // Refers to ambient radience in p.75

    #[serde(rename = "PointLight")]
    pub point_lights: SingleOrVec<PointLight>, 
}

impl Default for SceneLightsJSON {
    fn default() -> Self {
        Self {
            ambient_light: Vector3::ZERO, // No intensity
            point_lights: SingleOrVec::default(),
            }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)] // If any of the fields below is missing in the JSON, use default (empty vector, hopefully)
// #[serde(rename_all = "PascalCase")] // Do NOT do that here, naming is different in json file
pub struct SceneObjectsJSON {
    #[serde(rename = "Triangle")]
    pub triangles: SingleOrVec<Triangle>,
    #[serde(rename = "Sphere")]
    pub spheres: SingleOrVec<Sphere>,
    #[serde(rename = "Plane")]
    pub planes: SingleOrVec<Plane>,
    #[serde(rename = "Mesh")]
    pub meshes: SingleOrVec<Mesh>,

}