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
use bevy_math::NormedVectorSpace;
use rand::random; // traits needed for norm_squared( ) 

use crate::brdf::BRDFs;
use crate::image::{ImageData, Textures};
use crate::material::{*};
use crate::shapes::{*};
use crate::mesh::{LightMesh, Mesh, MeshInstanceField};
use crate::json_structs::{*};
use crate::camera::{Cameras};
use crate::interval::{Interval, FloatConst};
use crate::ray::{Ray, HitRecord};
use crate::acceleration::BVHSubtree;
use crate::{light::*, numeric};
use crate::prelude::*; // TODO: Excuse me but what's the point of prelude if there are so many use crate::yet_another_mod above?

pub type HeapAllocatedVerts = Arc<VertexCache>;

pub trait Scene {

    fn print_my_dummy_debug(&self) {
        dbg!("No debug message found for the scene. Make sure to implement Scene trait print function.");
    }

    fn render(&self) -> Result<Vec<crate::image::ImageData>, Box<dyn std::error::Error>>;
}


#[derive(Debug, Deserialize)]
pub struct RootScene {
    #[serde(rename = "Scene")]
    pub scene_3d: Option<Scene3DJSON>,

    #[serde(rename = "Scene2D")]
    pub scene_2d: Option<Scene2D>,
}

#[derive(Debug, Deserialize)]
pub struct Scene2D {
    #[serde(rename = "BackgroundColor", deserialize_with = "deser_vec3")]
    pub background_color: Vector3,

    #[serde(rename = "Lights")]
    pub lights: SceneLights2D,

    #[serde(rename = "Layers")]
    pub layers: SingleOrVec<Layer2D>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Layer2D {
    #[serde(rename = "_id", deserialize_with = "deser_usize")]
    _id: usize,

    #[serde(rename = "Image")]
    image_relative_path: String,

    #[serde(skip)]
    data: Vec<crate::pixel::PixelData>,
}

impl Layer2D {
    pub fn setup(&mut self, jsonpath: &Path) {
        let path = jsonpath.parent().unwrap_or(jsonpath).join(self.image_relative_path.clone());
        info!("Reading layer image from {:?} ", path);
        let img = image::open(path).unwrap();

        let img_rgba = img.as_rgba8().unwrap();
        todo!("Turn img into actual data holding PixelData");
    }
}


#[derive(Debug, Deserialize)]
pub struct SceneLights2D {
    #[serde(rename = "PointLight2D")]
    pub point_lights: SingleOrVec<PointLight2D>,

}

impl Scene2D {
    pub fn setup(&mut self, jsonpath: &Path) {
        for layer in self.layers.iter_mut() {
            layer.setup(jsonpath);
        }
    }
}

#[derive(Debug, Deserialize, SmartDefault)]
#[serde(rename_all = "PascalCase")]
#[serde(default)]
pub struct Scene3DJSON {
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

    pub tex_coord_data: Option<TexCoordData>,
    pub textures: Option<Textures>,

    pub transformations: Transformations,
    pub cameras: Cameras,
    pub lights: SceneLights,
    pub materials: SceneMaterials,
    pub objects: SceneObjects,

    #[serde(rename = "BRDFs")]
    pub brdfs: BRDFs,
    
}

impl Scene3DJSON {
    pub fn setup_and_get_cache(&mut self, jsonpath: &Path) -> Result<VertexCache, Box<dyn Error>>{
        // Implement required adjustments after loading from a JSON file
        debug!(">> Scene transformations: {:?}", self.transformations);

        // 1- Convert materials serde_json values to actual structs
        self.materials.finalize();
        //for m in &self.materials.materials { // TODO: refactor that ambigious call materials.materials( )
        //    debug!("Material {}: {:#?}", m.get_type(), m);
        //}

        // 2- Fix VertexData if _type is not "xyz" 
        let previous_type = self.vertex_data._type.clone();
        if self.vertex_data.normalize_to_xyz() { warn!("VertexData _type is changed from '{}' to '{}'", previous_type, self.vertex_data._type); }

        // 3- Add a dummy vertex at index 0 because JSON vertex ids start from 1
        self.vertex_data.insert_dummy_at_the_beginning();
        debug!("Inserted a dummy vertex at the beginning to use vertex IDs beginning from 1.");

        // 4 - Setup object transformations WARNING: Order of this is important unfortunately..
        self.objects.setup_transforms(&self.transformations);

        // 5 - Setup texture images (read from image files and store)
        if let Some(tex_coords) = self.tex_coord_data.as_mut() {
            tex_coords.insert_dummy_at(0);
            tex_coords.insert_dummy_at(0); // two dummies as (u_dummy, v_dummy) pair (DataField<Float> has flattened _data field)
        }
        if let Some(textures) = self.textures.as_mut() {
            if let Some(texture_images) = textures.images.as_mut() {
                let base_dir = jsonpath
                                        .parent()
                                        .ok_or("WARNING: JSON path has no parent directory")?;
                texture_images.setup(base_dir);
            }
        }
        
        // 6 - Get cache per vertex (objects.setup appends PLY data to vertex_data)
        let cache = self.objects.setup_and_get_cache(&mut self.vertex_data, &self.tex_coord_data, jsonpath)?;

        // 7 - Setup scene lights transforms
        self.lights.setup(&self.transformations);

        Ok(cache)
    }
}

#[derive(Debug)]
pub struct Scene3D 
//where 
//   T: Shape + BBoxable + 'static,
{
    pub data: Box<Scene3DJSON>, // Now owns the data instead of borrowing

    pub vertex_cache: HeapAllocatedVerts,
    pub bvh: Option<BVHSubtree>,
}


impl Scene3D 
//where 
//    T: Shape + BBoxable + 'static,
    { 
    pub fn new_from(scene_json: Scene3DJSON, jsonpath: &Path) -> Self {

        let mut scene_json = scene_json;
        let cache = scene_json.setup_and_get_cache(jsonpath).unwrap(); 

        let mut scene = Self {
            data: Box::new(scene_json),
            vertex_cache: Arc::new(cache),
            bvh: None,
        };
        scene.build_bvh();
        scene
    }

    /// Build top-tevel BVH for scene
    pub fn build_bvh(&mut self) {
        let shapes = &self.data.objects.bboxable_shapes;
        let verts = &self.vertex_cache.vertex_data;
        self.bvh = Some(BVHSubtree::build(shapes, verts, true)); // Apply object's transformation for top-level BVH
    }

    /// Iterate over all shapes to find the closest hit
    pub fn hit_naive(&self, ray: &Ray, t_interval: &Interval, early_break: bool) -> Option<HitRecord>{
        // Refers to p.91 of slide 01_b, lines 3-7
        let mut rec: Option<HitRecord> = None;
        let mut t_min: Float = FloatConst::INF;
        for shape in self.data.objects.bboxable_shapes.iter() { 
            if let Some(hit_record) = shape.intersects_with(ray, t_interval, &self.vertex_cache){

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


    // TODO: Is it better hitrecord a mutable input parameter rather than returning Option<HitRecord>?  
    pub fn hit_bvh(&self, ray: &Ray, t_interval: &Interval, early_break: bool)
    -> Option<HitRecord> 
    {
        // 1. BVH hit first with bounding boxable shapes
        let mut best = None;
        let mut best_t = FloatConst::INF;

        if let Some(bvh) = &self.bvh {
            let mut rec = HitRecord::default();
            if bvh.intersect(ray, t_interval, &self.vertex_cache, &mut rec, early_break) {
                best_t = rec.ray_t;
                best = Some(rec);
            }
        }
        else {
            best = self.hit_naive(ray, t_interval, early_break);
        }

        // 2. Test planes (looping over all planes)
        for plane in &self.data.objects.unbboxable_shapes {
            if let Some(hit) = plane.intersects_with(ray, t_interval, &self.vertex_cache) 
                && hit.ray_t < best_t {
                    if early_break {
                        return Some(hit);
                    }
                    best_t = hit.ray_t;
                    best = Some(hit);
                
            }
        }

        best
    }
}


#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "PascalCase")]
#[serde(default)] // If any of the fields below is missing in the JSON, use default
pub struct SceneLights {
    #[serde(rename = "AmbientLight", deserialize_with = "deser_vec3")]
    pub ambient_light: Vector3, // Refers to ambient radience in p.75

    #[serde(rename = "PointLight")]
    pub point_lights: SingleOrVec<PointLight>, 

    #[serde(rename = "AreaLight")]
    pub area_lights: SingleOrVec<AreaLight>,

    #[serde(rename = "DirectionalLight")]
    pub dir_lights: SingleOrVec<DirectionalLight>,

    #[serde(rename = "SpotLight")]
    pub spot_lights: SingleOrVec<SpotLight>,

    #[serde(rename = "SphericalDirectionalLight")]
    pub env_lights: SingleOrVec<SphericalDirectionalLight>,

    #[serde(skip)]
    cached_shadow_rayable: Vec<LightKind>,
}

impl SceneLights {
    pub fn setup(&mut self, transforms: &Transformations) {

        debug!("Setting up scene lights...\n{:#?}", self);

        // TODO: Learn how to use macros to avoid repetition (it seems like macros are the
        // way to go in this case)
        for light in self.point_lights.iter_mut() {
            light.setup(transforms);
        }
        for light in self.area_lights.iter_mut() {
            light.setup();
        }
        for light in self.dir_lights.iter_mut() {
            light.setup();
        }
        for light in self.spot_lights.iter_mut() {
            light.setup();
        }
        for light in self.env_lights.iter_mut() {
            light.setup();
        }

        self.cached_shadow_rayable = self.build_shadow_rayable();

        for env_light in self.env_lights.iter_mut() {
            env_light.setup();
        }

        debug!("Scene lights setup done!");
    }

    // TODO: DONT FORGET TO ADD YOUR NEW LIGHTKIND HERE, well, this is easy to forget and not functional...
    pub fn build_shadow_rayable(&self) -> Vec<LightKind> {
        self.point_lights.iter()
        .map(|p| LightKind::Point(p.clone()))
        .chain(self.area_lights.iter().map(|a| LightKind::Area(a.clone())))
        .chain(self.dir_lights.iter().map(|dl| LightKind::Directional(dl.clone())))
        .chain(self.spot_lights.iter().map(|sl| LightKind::Spot(sl.clone())))
        //.chain(self.env_lights.iter().map(|el| LightKind::Env(el.clone())))
        .collect()
    }

    pub fn all_shadow_rayable(&self) -> &Vec<LightKind> {
        &self.cached_shadow_rayable
    }
}



#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct SceneMaterials {
    #[serde(rename = "Material")]
    raw_materials: SingleOrVec<serde_json::Value>, // Parse the json value later separately

    #[serde(skip)]
    pub data: Vec<HeapAllocMaterial>,
}

impl SceneMaterials {
    pub fn finalize(&mut self) {
        self.data = self.raw_materials
                        .all()
                        .into_iter()
                        .flat_map(parse_material)
                        .map(|mut m| {
                            m.setup();
                            m
                        })
                        .collect();
    }

    pub fn all(&mut self) -> &Vec<HeapAllocMaterial> {
        if self.data.is_empty() && !self.raw_materials.all().is_empty() {
            warn!("Calling SceneMaterials.finalize() to fully deserialize materials from JSON file...");
            self.finalize(); 
        }
        &self.data
    }
}


#[derive(Clone, Debug, Deserialize, Default)]
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
    
    #[serde(rename = "LightMesh")]
    pub light_meshes: SingleOrVec<LightMesh>,
    #[serde(rename = "LightSphere")]
    pub light_spheres: SingleOrVec<LightSphere>,

    #[serde(rename = "MeshInstance")]
    pub mesh_instances: SingleOrVec<MeshInstanceField>,

    #[serde(skip)]
    pub bboxable_shapes: ShapeList, 
    #[serde(skip)]
    pub unbboxable_shapes: ShapeList,
    #[serde(skip)]
    pub emissive_shapes: EmissiveShapeList,  
}

fn resolve_all_mesh_instances(
    mesh_instances: &mut SingleOrVec<MeshInstanceField>,
    meshes: &SingleOrVec<Mesh>,
) {
    let slice = mesh_instances.as_mut_slice();
    let n = slice.len();

    for i in 0..n {
        // Split slice into the current element and the rest
        let (left, right) = slice.split_at_mut(i);
        let (mint, rest) = right.split_first_mut().unwrap();

        // First check "Mesh" objects
        if let Some(mesh) = meshes.iter().find(|m| m._id == mint.base_mesh_id) {
            mint.base_mesh = Some(Arc::new(mesh.clone()));
            debug!("Mesh instance {} refers base mesh {} ", mint._id, mint.base_mesh.clone().unwrap()._id);
            continue;
        }

        // Then try other "MeshInstance" objects
        for other in left.iter().chain(rest.iter()) {
            if other._id == mint.base_mesh_id {
                mint.base_mesh = other.base_mesh.clone();
                mint.matrix *= other.matrix; // TODO: is this the correct order?
                debug!("Mesh instance {} refers base mesh instance {} ", mint._id, mint.base_mesh.clone().unwrap()._id);
                break;
            }
        }

        // If still None, panic
        if mint.base_mesh.is_none() {
            panic!("Could not resolve base mesh id {}", mint.base_mesh_id);
        }
    }
}


fn setup_single_mesh_transform(mesh: &mut Mesh,  transforms: &Transformations) {
    mesh.matrix = if mesh.transformation_names.is_some() {
        parse_transform_expression(
            mesh.transformation_names.as_deref().unwrap_or(""),
            transforms,  
        )
    } else {
        debug!("Mesh '{}'s transformation is not given, defaulting to Identity.", mesh._id);
        Matrix4::IDENTITY  // Default to identity if no transform is given
    };
    debug!("Composite transform for mesh '{}' is {}", mesh._id, mesh.matrix);
}

fn unnecessarily_long_setup_function_for_scene_meshes(
    mesh: &mut Mesh, 
    json_dir: &Path,
    verts: &mut VertexData,
    all_triangles: &mut Vec<Triangle>,
    uv_coords: &mut Vec<Option<[Float; 2]>>,
    tot_mesh_faces: &mut usize,
) -> Result<(), Box<dyn Error>> 
{
    if !mesh.faces._ply_file.is_empty() {

                
                let ply_file = &mesh.faces._ply_file;
                let ply_path = json_dir.join(ply_file);

                if ply_path.exists() {
                    debug!("PLY file exists: {:?}", ply_path);
                } else {
                    error!("PLY file NOT found at: {:?}", ply_path);
                }

                debug!("Loading mesh {} from PLY file path: {:?}", mesh._id, ply_path);

                let file = File::open(ply_path)?;
                let reader = BufReader::new(file);
                let plymesh: PlyMesh = serde_ply::from_reader(reader)?;
                let old_vertex_count = verts._data.len();
                // Append loaded ply to vertexdata
                for vert in &plymesh.vertex {
                    verts._data.push(Vector3::new(vert.x as Float, vert.y as Float, vert.z as Float));
                
                    let uv = match (vert.u, vert.v) {
                        (Some(u), Some(v)) => {
                            Some([u as Float , v as Float])
                        },
                        _ => None,
                    };
                    uv_coords.push(uv);
                }
                // Shift faces._data by offset
                mesh.faces._type = String::from("triangle");
                if let Some(faces) = &plymesh.face {
                    mesh.faces._data = faces
                        .iter()
                        .flat_map(|f| f.vertex_indices.clone()) // each face is a list of 3 indices
                        .map(|idx| idx + old_vertex_count)      // shift by existing vertices
                        .collect();
                    //info!(">> Mesh {} has {} faces.", mesh._id, mesh.faces._data.len());
                    *tot_mesh_faces += mesh.faces._data.len();
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

            Ok(())
}

impl SceneObjects {

    fn setup_transforms(&mut self, transforms: &Transformations) { // TODO: What's the deal with setting matrices within scene? these could be impl in shapes.rs 

        for mesh in self.meshes.iter_mut() {
            setup_single_mesh_transform(mesh, transforms);
        }
        for lightmesh in self.light_meshes.iter_mut() {
            setup_single_mesh_transform(&mut lightmesh.data, transforms);
        }

        for mint in self.mesh_instances.iter_mut() {
            mint.matrix = parse_transform_expression(
                    mint.transformation_names.as_str(),
                    transforms,  
            );
            debug!("Composite transform for mesh '{}' is {}", mint._id, mint.matrix);
        }

        for tri in self.triangles.iter_mut() {
            debug!("Setting up transforms for mesh._id '{}'", tri._data._id.clone());
            tri.matrix = Some(Arc::new(parse_transform_expression(
                    tri._data.transformation_names.as_deref().unwrap_or(""),
                    transforms,  
            )));
        }

        for sphere in self.spheres.iter_mut() {
            sphere.matrix = Some(Arc::new(parse_transform_expression(
                sphere._data.transformation_names.as_deref().unwrap_or(""), 
                transforms)));
        }

        
        for light_sphere in self.light_spheres.iter_mut() {
            light_sphere.data.matrix = Some(Arc::new(parse_transform_expression(
                light_sphere.data._data.transformation_names.as_deref().unwrap_or(""), 
                transforms)));
        }

        for plane in self.planes.iter_mut() {
            debug!("Setting up transforms for mesh._id '{}'", plane._data._id.clone());
            plane.matrix = Some(Arc::new(parse_transform_expression(
                    plane._data.transformation_names.as_deref().unwrap_or(""),
                    transforms,  
            )));
        }
    }

    pub fn setup_and_get_cache(&mut self, verts: &mut VertexData, texture_coords: &Option<TexCoordData>, jsonpath: &Path) -> Result<VertexCache, Box<dyn Error>> {
        // NOTE: Vec::extend( ) pushes a collection of data all at once, 
        // if you have a single object to push, then use Vec::push( )

        let mut bboxable_shapes: ShapeList = Vec::new();
        let mut unbboxable_shapes: ShapeList = Vec::new();
        let mut emissive_shapes: EmissiveShapeList = Vec::new();
        let mut all_triangles: Vec<Triangle> = self.triangles.all();
        
        // Initiate uv_coords from given texture coords or if not available with a new vector
        let mut uv_coords: Vec<Option<[Float; 2]>> = if let Some(tc) = texture_coords {
            let raw = &tc._data;
            let mut out = Vec::with_capacity(verts._data.len());
            for chunk in raw.chunks_exact(2) {
                out.push(Some([chunk[0], chunk[1]]));
            }
            out
        } else {
            Vec::new()
        };

        // Step 2: Fill missing slots for any dummy vertices at the start (note that verts will be mutated as we push more verts read from ply files)
        while uv_coords.len() < verts._data.len() {
            uv_coords.push(None);
        }

        bboxable_shapes.extend(self.triangles.all().into_iter().map(|t| Arc::new(t) as HeapAllocatedShape));
        bboxable_shapes.extend(self.spheres.all().into_iter().map(|s| Arc::new(s) as HeapAllocatedShape));
        
        // Assign nonces to light spheres and add them to emissive shapes
        for light_sphere in self.light_spheres.iter_mut() {
            light_sphere.nonce = numeric::next_uuid(); //rand::random::<u64>();
            bboxable_shapes.push(Arc::new(light_sphere.clone()) as HeapAllocatedShape);
            emissive_shapes.push(Arc::new(light_sphere.clone()) as Arc<dyn EmissiveShape>);
        }

        unbboxable_shapes.extend(self.planes.all().into_iter().map(|p| Arc::new(p) as HeapAllocatedShape));

        // Get path containing the JSON (_plyFile in json is relative to that json)
        let json_dir = Path::new(jsonpath)
                    .parent()
                    .unwrap_or(Path::new("."));
        // Convert meshes: UPDATE: do not convert it into individual triangles
        let mut tot_mesh_faces: usize = 0;
        for mesh in self.meshes.iter_mut() {
            unnecessarily_long_setup_function_for_scene_meshes(mesh, json_dir, verts, &mut all_triangles, &mut uv_coords, &mut tot_mesh_faces)?;            
            bboxable_shapes.push(Arc::new(mesh.clone()) as HeapAllocatedShape);
        }

        for lightmesh in self.light_meshes.iter_mut() {
            unnecessarily_long_setup_function_for_scene_meshes(&mut lightmesh.data, json_dir, verts, &mut all_triangles, &mut uv_coords, &mut tot_mesh_faces)?;
            
            // Assign random nonce
            lightmesh.nonce = numeric::next_uuid(); // rand::random::<u64>();
            
            bboxable_shapes.push(Arc::new(lightmesh.clone()) as HeapAllocatedShape);
            emissive_shapes.push(Arc::new(lightmesh.clone()) as Arc<dyn EmissiveShape>);
        }

        // Find which meshes the mesh refers to
        let mesh_instances = &mut self.mesh_instances;
        let meshes = &self.meshes;
        resolve_all_mesh_instances(mesh_instances, meshes);

        // Push all mesh instances to scene shapes -----------------
        for mint in self.mesh_instances.iter() { 
            debug!("Before pushing into all_shapes, Mesh instance {} referes base mesh {} ", mint._id, mint.base_mesh.clone().unwrap()._id);
            bboxable_shapes.push(Arc::new(mint.clone()) as HeapAllocatedShape);
        }

        info!(">> There are {} vertices in the scene (excluding {} instance mesh). Meshes have {} faces in total.", verts._data.len(), self.mesh_instances.len(), tot_mesh_faces);
        self.bboxable_shapes = bboxable_shapes;
        self.unbboxable_shapes = unbboxable_shapes;
        self.emissive_shapes = emissive_shapes;
        let normals_cache = VertexCache::build_normals(verts, &all_triangles);
        
        let cache = VertexCache { vertex_data: verts.clone(), vertex_normals: normals_cache, uv_coords }; 
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
    pub uv_coords: Vec<Option<[Float;2]>>,
}

impl Default for VertexCache {
    fn default() -> Self {
        debug!("Defaulting VertexCache...");
        Self {
            vertex_data: VertexData::default(),
            vertex_normals: Vec::new(),
            uv_coords: Vec::new(),
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
    
    pub fn build_uv(n_verts: usize, 
                    tex_coords: &Option<TexCoordData>,
                    ply_uv_coords: &[Option<[Float; 2]>]
                ) -> Vec<Option<[Float; 2]>> {
        // Assumes:
        // - dummy texcoord already inserted
        // - texcoords are aligned with vertex_data indices

        if let Some(tc) = tex_coords {
                
            let raw = &tc._data;
            debug_assert!(tc._type == "uv" || tc._type == ""); // Only uv supported currently
            debug_assert!(
                raw.len() % 2 == 0,
                "TexCoordData length must be even (because we assume u,v pairs), got length {}", raw.len()
            );

            let mut out: Vec<Option<[Float; 2]>> = Vec::with_capacity(n_verts);

            // Group the texture coordinates into pairs (assuming uv type)
            for chunk in raw.chunks_exact(2) {
                if let Some(offset) = tc._texture_offset {
                    info!("TextureCoords Got offset: {} however it is not utilized as TextureCoordinates should not have offset itself (Face field has it, and since they both impl DataField, it is easy to confuse so I better separate them into two different structs)", offset);
                }
                out.push(Some([chunk[0], chunk[1]]));
            }

            // Add uv coordinates coming from PLY
            out.extend_from_slice(ply_uv_coords);

            // Fill the remaining fields with none, (e.g. if ply file will be used to insert more coords)
            while out.len() < n_verts {
                //debug!("Pushing none to texcoords...");
                out.push(None);
            }
            assert!(out.len() == n_verts);
            
            out
        } 
        else {
            vec![None; n_verts] // If no texture coordinates are given, just fill all with None
        }
    }

    pub fn build_normals(verts: &VertexData, triangles: &[Triangle]) -> Vec<Vector3> {
        // Compute per-vertex normal from neighbouring triangles
        let vertex_data = verts;//.clone();
        let mut vertex_normals: Vec<Vector3> = vec![Vector3::ZERO; vertex_data._data.len()];
        for tri in triangles.iter() {
            let indices = tri.vert_indices;
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
        vertex_normals
    }
}
