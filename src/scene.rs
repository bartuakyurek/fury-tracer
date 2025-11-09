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

#[derive(Debug, Deserialize)]
pub struct RootScene {
    #[serde(rename = "Scene")]
    pub scene: Scene,
}

#[derive(Debug, Deserialize, SmartDefault)]
#[serde(rename_all = "PascalCase")]
#[serde(default)]
pub struct Scene {
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

    #[serde(skip)]
    pub vertex_cache: HeapAllocatedVerts,

    pub cameras: Cameras,
    pub lights: SceneLights,
    pub materials: SceneMaterials,
    pub objects: SceneObjects,
}

impl Scene {
    //pub fn new() {
    //}
    pub fn setup_after_json(&mut self, jsonpath: &Path) -> Result<(), Box<dyn Error>>{
        // Implement required adjustments after loading from a JSON file

        // 1- Convert materials serde_json values to actual structs
        self.materials.finalize();
        for m in &self.materials.materials { // TODO: refactor that ambigious call materials.materials( )
            debug!("Material: {:#?}", m);
        }

        // 2- Fix VertexData if _type is not "xyz" 
        let previous_type = self.vertex_data._type.clone();
        if self.vertex_data.normalize_to_xyz() { warn!("VertexData _type is changed from '{}' to '{}'", previous_type, self.vertex_data._type); }

        // 3- Add a dummy vertex at index 0 because JSON vertex ids start from 1
        self.vertex_data.insert_dummy_at_the_beginning();
        warn!("Inserted a dummy vertex at the beginning to use vertex IDs beginning from 1.");

        // 4- Build cache
        let cache = self.objects.setup(&mut self.vertex_data,  jsonpath)?; // Appends new vertices if mesh is from PLY
        self.vertex_cache = Arc::new(cache);

        Ok(())
    }

}


#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SceneLights {
    #[serde(rename = "AmbientLight", deserialize_with = "deser_vec3")]
    pub ambient_light: Vector3, // Refers to ambient radience in p.75

    #[serde(rename = "PointLight")]
    pub point_lights: SingleOrVec<PointLight>, 
}

impl Default for SceneLights {
    fn default() -> Self {
        Self {
            ambient_light: Vector3::ZERO, // No intensity
            point_lights: SingleOrVec::default(),
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




#[derive(Debug, Deserialize, Default)]
#[serde(default)] // If any of the fields below is missing in the JSON, use default (empty vector, hopefully)
// #[serde(rename_all = "PascalCase")] // Do NOT do that here, naming is different in json file
pub struct SceneObjects {
    #[serde(rename = "Triangle")]
    pub triangles: SingleOrVec<Triangle>,
    #[serde(rename = "Sphere")]
    pub spheres: SingleOrVec<Sphere>,
    #[serde(rename = "Plane")]
    pub planes: SingleOrVec<Plane>,
    #[serde(rename = "Mesh")]
    pub meshes: SingleOrVec<Mesh>,

    #[serde(skip)]
    pub all_shapes: ShapeList,
}

impl SceneObjects {

    pub fn setup(&mut self, verts: &mut VertexData, jsonpath: &Path) -> Result<VertexCache, Box<dyn Error>> {
        // Return a vector of all shapes in the scene
        warn!("SceneObjects.all( ) assumes there are only triangles, spheres, planes, and meshes. If there are other Shape trait implementations they are not added yet.");
        let mut shapes: ShapeList = Vec::new();
        let mut all_triangles: Vec<Triangle> = self.triangles.all();

        shapes.extend(self.triangles.all().into_iter().map(|t| Arc::new(t) as HeapAllocatedShape));
        shapes.extend(self.spheres.all().into_iter().map(|s| Arc::new(s) as HeapAllocatedShape));
        shapes.extend(self.planes.all().into_iter().map(|p| Arc::new(p) as HeapAllocatedShape));
        //shapes.extend(self.meshes.all().into_iter().map(|m| Rc::new(m) as Rc<dyn Shape>));

        // Convert meshes to triangles 
        for mesh in self.meshes.all() {
            let mut mesh = mesh;
            if !mesh.faces._ply_file.is_empty() { 
                
                // Get path containing the JSON (_plyFile in json is relative to that json)
                let json_dir = Path::new(jsonpath)
                    .parent()
                    .unwrap_or(Path::new("."));
                let ply_file = &mesh.faces._ply_file;
                let ply_path = json_dir.join(ply_file);

                if ply_path.exists() {
                    info!("PLY file exists: {:?}", ply_path);
                } else {
                    error!("PLY file NOT found at: {:?}", ply_path);
                }

                info!("Loading mesh {} from PLY file path: {:?}", mesh._id, ply_path);
                
                let file = File::open(ply_path)?;
                let reader = BufReader::new(file);
                let plymesh: PlyMesh = serde_ply::from_reader(reader)?;
                let old_vertex_count = verts._data.len();
                // Append loaded ply to vertexdata 
                for v in &plymesh.vertex {
                    verts._data.push(Vector3::new(v.x as Float, v.y as Float, v.z as Float));
                }
                // Shift faces._data by offset
                mesh.faces._type = String::from("triangle");
                if let Some(faces) = &plymesh.face {
                    mesh.faces._data = faces
                        .iter()
                        .flat_map(|f| f.vertex_indices.clone()) // each face is a list of 3 indices
                        .map(|idx| idx + old_vertex_count)      // shift by existing vertices
                        .collect();
                    info!(">> Mesh {} has {} faces.", mesh._id, mesh.faces._data.len());
                } 
                else {
                    warn!("PLY mesh {} has no face data!", mesh._id);
                }
            }
            let offset = verts._data.len();
            let triangles: Vec<Triangle> = mesh.to_triangles(verts, offset);
            all_triangles.extend(triangles.iter().cloned());
            shapes.extend(triangles.into_iter().map(|t| Arc::new(t) as HeapAllocatedShape));
        }
        info!(">> There are {} vertices in the scene.", verts._data.len());
        self.all_shapes = shapes;
        let cache = VertexCache::build(&verts, &all_triangles);   
        Ok(cache)
    }

}

