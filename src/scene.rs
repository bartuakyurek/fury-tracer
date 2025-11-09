/*

    Declare Scene consisting of all cameras, lights,
    materials, vertex data, and objects to be rendered.

    This declaration is meant to be compatible with 
    CENG 795's JSON file formats.

    WARNING: This Scene description is coupled with JSON file descriptions
    and it assumes JSON file fields are named in PascalCase (not camelCase or snake_case)
    TODO: Provide structs to (de)serialize JSON files, and communicate with a separate
    Scene struct that is hopefully decoupled from JSON file descriptions, i.e. to support
    such workflow:
        
        let s Scene::EMPTY
        s.add_some_object()
        s.add_some_light()
        s.center_camera() 
        let js = JSONScene::new_from(s)
        js.serialize(path/to/json)
    
    or
        let js = JSONSCene::new(path/to/json)
        let s = Scene::new_from(js)
        s.do_something_if_you_like()
        render(s)

    @date: 2 Oct, 2025
    @author: Bartu
*/
use std::{path::Path, io::BufReader, error::Error, fs::File};

use crate::material::{*};
use crate::shapes::{*};
use crate::mesh::Mesh;
use crate::json_structs::{*};
use crate::geometry::{VertexCache, HeapAllocatedVerts};
use crate::camera::{Cameras};
use crate::prelude::*;


pub struct Scene {
    pub max_recursion_depth: usize,
    pub background_color: Vector3,
    pub shadow_ray_epsilon: Float,
    pub intersection_test_epsilon: Float,

    pub vertex_data: VertexData, 
    pub vertex_cache: HeapAllocatedVerts,

    pub cameras: Cameras,
    pub lights: SceneLights,
    pub materials: SceneMaterials,
    pub objects: SceneObjects,
}

impl Scene {

    
    pub fn new_from(sj: &SceneJSON, jsonpath: &Path) -> Result<Self, Box<dyn Error>> { 

        let vertex_data: VertexData = sj.get_fixed_vertex_data()?; 
        let objs: SceneObjects = SceneObjects::new_from(&sj.objects); 
        let cache: VertexCache = VertexCache::build(&vertex_data, &all_triangles); 

        let mut scn = Self { 
            max_recursion_depth: sj.max_recursion_depth, 
            background_color: sj.background_color, 
            shadow_ray_epsilon: sj.shadow_ray_epsilon, 
            intersection_test_epsilon: sj.intersection_test_epsilon, 
            vertex_data: vertex_data, 
            vertex_cache: Arc::new(cache), 
            cameras: sj.cameras, 
            lights: SceneLights::new_from(&sj.lights), 
            materials: {sj.materials.finalize(); sj.materials}, 
            objects: objs, 
        }; 

        Ok(scn) 
    }

}


pub struct SceneLights { 
    pub ambient_light: Vector3, 
    pub point_lights: Vec<PointLight>, 
} 

impl SceneLights { 
    pub fn new_from(sl: &SceneLightsJSON) -> Self { 
        Self { 
            ambient_light: sl.ambient_light, 
            point_lights: sl.point_lights.all(), 
        } 
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PointLight {
    #[serde(rename = "_id", deserialize_with = "deser_int")]
    pub _id: Int, // or String if you prefer

    #[serde(rename = "Position", deserialize_with = "deser_vec3")]
    pub position: Vector3,

    #[serde(rename = "Intensity", deserialize_with = "deser_vec3")]
    pub rgb_intensity: Vector3,
}


#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct SceneMaterials {
    #[serde(rename = "Material")]
    raw_materials: SingleOrVec<serde_json::Value>, // Parse the json value later separately

    #[serde(skip)]
    pub materials: Vec<HeapAllocMaterial>,
}

impl SceneMaterials {
    pub fn finalize(&mut self) {
        self.materials = self.raw_materials
                        .all()
                        .into_iter()
                        .flat_map(parse_material)
                        .collect();
    }

    pub fn all(&mut self) -> &Vec<HeapAllocMaterial> {
        if self.materials.is_empty() && !self.raw_materials.all().is_empty() {
            warn!("Calling SceneMaterials.finalize() to fully deserialize materials from JSON file...");
            self.finalize(); 
        }
        &self.materials
    }
}






pub struct SceneObjects {
    
    triangles: Vec<Triangle>,
    spheres: Vec<Sphere>,
    planes: Vec<Plane>,
    meshes: Vec<Mesh>,
}

impl SceneObjects {

    pub fn new_from(json_struct: &SceneObjectsJSON) -> Self {
        // WARNING: all( ) clones data but I couldn't find a neat solution to deserialize SingleOrVec
        // without clone. Perhaps implementing iterator could make SingleOrVec more useful (and potentially cryptic)
        Self {
            triangles: json_struct.triangles.all(),
            spheres: json_struct.spheres.all(),
            planes: json_struct.planes.all(),
            meshes: json_struct.meshes.all(),
        }
    }

    pub fn setup_meshes(&mut self, verts: &VertexData, jsonpath: &Path) -> Result<VertexCache, Box<dyn Error>> { 
        
        for mesh in self.meshes { 
            let mut mesh = mesh; 
            let offset = verts._data.len(); 
            let triangles: Vec<Triangle> = mesh.to_triangles(verts, offset); 
            all_triangles.extend(triangles.iter().cloned()); 
            // Convert meshes to triangles 
            shapes.extend(triangles.into_iter().map(|t| Arc::new(t) as HeapAllocatedShape)); 
        } 
        info!(">> There are {} vertices in the scene.", verts._data.len()); 
        self.all_shapes = shapes; 
        Ok(()) 
    }

    pub fn all(&self) -> ShapeList {

    }
    
   
}

