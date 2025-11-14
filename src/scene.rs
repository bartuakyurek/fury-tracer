/*

    Declare Scene consisting of all cameras, lights,
    materials, vertex data, and objects to be rendered.

    This declaration is meant to be compatible with 
    CENG 795's JSON file formats.

    UPDATE: Acceleration structure added as Scene::bvh

    @date: 2 Oct, 2025
    @author: Bartu
*/
use std::{path::Path, io::BufReader, error::Error, fs::File};
use bevy_math::NormedVectorSpace; // traits needed for norm_squared( ) 

use crate::material::{*};
use crate::shapes::{*};
use crate::mesh::{Mesh, MeshInstanceField};
use crate::json_structs::{*};
use crate::camera::{Cameras};
use crate::interval::{Interval, FloatConst};
use crate::ray::{Ray, HitRecord};
use crate::acceleration::BVHSubtree;
use crate::prelude::*; // TODO: Excuse me but what's the point of prelude if there are so many use crate::yet_another_mod above?

pub type HeapAllocatedVerts = Arc<VertexCache>;




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


    #[serde(skip)]
    pub transformations: Transformations,

    pub cameras: Cameras,
    pub lights: SceneLights,
    pub materials: SceneMaterials,
    pub objects: SceneObjects,
}

impl SceneJSON {
    pub fn setup_and_get_cache(&mut self, jsonpath: &Path) -> Result<VertexCache, Box<dyn Error>>{
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

        // 4 - Get cache per vertex (objects.setup appends PLY data to vertex_data)
        let cache = self.objects.setup_and_get_cache(&mut self.vertex_data,  jsonpath)?; 

        // 5 - Setup object transformations
        self.objects.setup_transforms(&self.transformations);

        Ok(cache)
    }
}

#[derive(Debug)]
pub struct Scene <'a> 
//where 
//   T: Shape + BBoxable + 'static,
{
    pub data: &'a SceneJSON, // I'm figuring out data composition in Rust here
                             // in order not to clutter deserialized Scene with additional data.
                             // Otherwise it requires serde[skip] annotations for each addition.

    pub vertex_cache: HeapAllocatedVerts,
    pub bvh: Option<BVHSubtree>,
}


impl<'a> Scene <'a>  // Lifetime annotation 'a looks scary but it was needed for storing a pointer to deserialized data
//where 
//    T: Shape + BBoxable + 'static,
    { 
    pub fn new_from(scene_json: &'a mut SceneJSON, jsonpath: &Path) -> Self {

        let cache = scene_json.setup_and_get_cache(jsonpath).unwrap(); 

        let mut scene = Self {
            data: scene_json,
            vertex_cache: Arc::new(cache),
            bvh: None,
        };
        scene.build_bvh();
        scene
    }

    /// Build top-tevel BVH for scene
    pub fn build_bvh(&mut self) {
        let shapes = &self.data.objects.all_shapes;
        let verts = &self.vertex_cache.vertex_data;
        self.bvh = Some(BVHSubtree::build(shapes, verts));
    }

    /// Iterate over all shapes to find the closest hit
    pub fn hit_naive(&self, ray: &Ray, t_interval: &Interval, early_break: bool) -> Option<HitRecord>{
        // Refers to p.91 of slide 01_b, lines 3-7
        let mut rec: Option<HitRecord> = None;
        let mut t_min: Float = FloatConst::INF;
        for shape in self.data.objects.all_shapes.iter() { 
            if let Some(hit_record) = shape.intersects_with(ray, &t_interval, &self.vertex_cache){

                if early_break { 
                    return Some(hit_record);
                }

                // Update if new hit is closer 
                if t_min > hit_record.ray_t { 
                    t_min = hit_record.ray_t;
                    rec = Some(hit_record);
                }
            }
        }
        rec
    }

    // TODO: Maybe a Trait like Acceleration could be useful to extend other acceleration structs
    // like KD-Trees, not just BVH. So we'd use a generic T, where T: Acceleration
    // For now let's assume it is BVH. I'm suggesting it especially if the function signature will
    // stay the same as hit_naive, we could even impl Acceleration for Vec<Shapes> that simply iterates
    // shapes. Then hit_naive( ) and hit_bvh( ) and any potential future functions could be reduced to 
    // hit<T>( ) where T: Acceleration { }
    // TODO: Is it better hitrecord a mutable input parameter rather than returning Option<HitRecord>?  
    pub fn hit_bvh(&self, ray: &Ray, t_interval: &Interval, early_break: bool) -> Option<HitRecord> {
        let mut rec: HitRecord = HitRecord::default();
        if let Some(bvh) = &self.bvh {
            if bvh.intersect(ray, t_interval, &self.vertex_cache, &mut rec) {
                return Some(rec);
            }
        }
        //warn!("No BVH found, using naive hit( ). You shouldn't be seeing this message though.");
        self.hit_naive(ray, t_interval, early_break)
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
    
    #[serde(rename = "MeshInstance")]
    pub mesh_instances: SingleOrVec<MeshInstanceField>,

    #[serde(skip)]
    pub all_shapes: ShapeList, 
}

impl SceneObjects {

    fn setup_transforms(&mut self, transforms: &Transformations) {

        
        for mesh in self.meshes.iter_mut() {
            info!("Setting up transforms for mesh._id '{}'", mesh._id.clone());
            mesh.transform = parse_transform_expression(
                    mesh.transformation_names.as_deref().unwrap_or(""),
                    &transforms,  
            );
        }

         for mint in self.mesh_instances.iter_mut() {
            info!("Setting up transforms for mesh_instance._id '{}'", mint._id.clone());
            mint.transform = parse_transform_expression(
                    mint.transformation_names.as_str(),
                    &transforms,  
            );
        }


        for tri in self.triangles.iter_mut() {
            info!("Setting up transforms for mesh._id '{}'", tri._id.clone());
            tri.transform = Some(parse_transform_expression(
                    tri.transformation_names.as_deref().unwrap_or(""),
                    &transforms,  
            ));
        }

        for sphere in self.spheres.iter_mut() {
            sphere.transform = Some(parse_transform_expression(
                sphere.transformation_names.as_deref().unwrap_or(""), 
                &transforms));
        }

        
        for plane in self.planes.iter_mut() {
            info!("Setting up transforms for mesh._id '{}'", plane._id.clone());
            plane.transform = Some(parse_transform_expression(
                    plane.transformation_names.as_deref().unwrap_or(""),
                    &transforms,  
            ));
        }

        //if self.all_shapes.len() == 0 {
        //    error!("You are trying to call setup_transforms( ) before filling SceneObjects::all_shapes, please call setup_and_get_cache( ) first or refactor me.");
        //}
        //for shape in self.all_shapes.iter_mut() {
        //    shape.transform // --> Oh this is where my traits intuition fail, I cannot access data of a trait object! 
        //  but how do I make sure dyn Trait has certain data? jus tby declaring a struct and using it? like struct ShapeData { transform }
        //  and then struct Sphere { shape_data: ShapeData .. } but still how do we access ShapeData for Sphere, Mesh, Plane if 
        // they can also store other fields? 
        //}

    }

    pub fn setup_and_get_cache(&mut self, verts: &mut VertexData, jsonpath: &Path) -> Result<VertexCache, Box<dyn Error>> {
        // NOTE: Vec::extend( ) pushes a collection of data all at once, 
        // if you have a single object to push, then use Vec::push( )

        let mut shapes: ShapeList = Vec::new();
        let mut all_triangles: Vec<Triangle> = self.triangles.all();

        shapes.extend(self.triangles.all().into_iter().map(|t| Arc::new(t) as HeapAllocatedShape));
        shapes.extend(self.spheres.all().into_iter().map(|s| Arc::new(s) as HeapAllocatedShape));
        shapes.extend(self.planes.all().into_iter().map(|p| Arc::new(p) as HeapAllocatedShape));
        
        // Convert meshes: UPDATE: do not convert it into individual triangles
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

            // For vertex cache, get the triangles in a single mesh 
            // TODO: this is done because we have global vertex_data
            let offset = verts._data.len();
            let triangles: Vec<Triangle> = mesh.setup(verts, offset);
            all_triangles.extend(triangles.into_iter());

            // Push mesh to shapes (previously I was deconstructing it into individual triangles)
            shapes.push(Arc::new(mesh) as HeapAllocatedShape);
        }
        info!(">> There are {} vertices in the scene.", verts._data.len());
        self.all_shapes = shapes;
        info!(">> There are {} shapes in the scene.", self.all_shapes.len());
        let cache = VertexCache::build(&verts, &all_triangles);   
        Ok(cache)
    }

}


// ======================================================
// Vertex Cache 
// TODO: where to actually declare these structs? I couldn't find a proper place.
// perhaps under common.rs or something, we also have numerics.rs that is 
// commonly used numerical types... 
// ======================================================

#[derive(Debug, Clone)]
pub struct VertexCache {
    pub vertex_data: VertexData,
    pub vertex_normals: Vec<Vector3>,
}

impl Default for VertexCache {
    fn default() -> Self {
        Self {
            vertex_data: VertexData::default(),
            vertex_normals: Vec::new(),
        }
    }
}

// WARNING: caching vertex normals are tricky because if the same vertex was used by multiple 
// meshes, that means there are more vertex normals than the length of vertexdata because
// connectivities are different. Perhaps it is safe to assume no vertex is used in multiple
// objects, but there needs to be function to actually check the scene if a vertex in VertexData
// only referred by a single scene object. 
// Furthermore, what if there were multiple VertexData to load multiple meshes in the Scene? 
// this is not handled yet and our assumption is VertexData is the only source of vertices, every
// shape refers to this data for their coordinates. 
impl VertexCache {
    
    pub fn build(verts: &VertexData, triangles: &Vec<Triangle>) -> VertexCache {
        // Compute per-vertex normal from neighbouring triangles
        let vertex_data = verts.clone();
        let mut vertex_normals: Vec<Vector3> = vec![Vector3::ZERO; vertex_data._data.len()];
        for tri in triangles.iter() {
            let indices = tri.indices;
            // Check if indices are in bounds of vertex_data
            if indices.iter().any(|&i| i >= vertex_data._data.len()) {
                continue;
            }
            let v1 = vertex_data._data[indices[0]];
            let v2 = vertex_data._data[indices[1]];
            let v3 = vertex_data._data[indices[2]];
            let edge_ab = v2 - v1;
            let edge_ac = v3 - v1;
            let face_n = edge_ab.cross(edge_ac); // Be careful, not normalized yet, to preserve area contribution from each face

            // Sum the area-weighted face normals 
            for &idx in &indices {
                if idx < vertex_normals.len() {
                    vertex_normals[idx] += face_n;
                }
            }
        }

        // Normalize accumulated normals
        for n in vertex_normals.iter_mut() {
            if n.norm_squared() > 0.0 { 
                *n = n.normalize();
            }
        }

        VertexCache {
            vertex_data,
            vertex_normals,
        }
    }
}
