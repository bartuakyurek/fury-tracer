/*

UPDATE: Acceleration structure added Mesh::bvh

@date: Oct-Nov 2025
@author: Bartu

*/


use crate::json_structs::{FaceType, SingleOrVec, VertexData, TexCoordData};
use crate::geometry::{get_tri_normal, is_degenerate_triangle};
use crate::shapes::{CommonPrimitiveData, Shape, Triangle};
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

    #[serde(rename = "Textures", deserialize_with = "deser_usize_vec", default)]
    pub texture_idxs: Vec<usize>,
    
    #[serde(rename = "Transformations")]
    pub(crate) transformation_names: String,

    #[serde(rename = "MotionBlur", deserialize_with = "deser_vec3", default)]
    pub(crate) motionblur: Vector3, // translational

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

    #[serde(rename = "Textures", deserialize_with = "deser_usize_vec")]
    pub texture_idxs: Vec<usize>,

    #[serde(rename = "Faces")]
    pub faces: FaceType,

    #[serde(rename = "_shadingMode")]
    #[default = "flat"]
    pub _shading_mode: String,

    #[serde(rename = "Transformations", default)]
    pub transformation_names: Option<String>,

    #[serde(rename = "MotionBlur", deserialize_with = "deser_vec3", default)]
    pub(crate) motionblur: Vector3, // translational

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

        // Apply vertex offset to faces._data
        // subsequent uses of faces._data will have correct indices
        if let Some(offset) = self.faces._vertex_offset {
            info!("Found vertex_offset: {} ", offset);
            for idx in &mut self.faces._data {
                *idx = (*idx as isize + offset) as usize;
            }
        }

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
            warn!(">> Expected triangle faces in mesh_to_triangles, got face._type '{}'.", self.faces._type);
        }
        
        let n_faces = self.faces.len_tris();
        let mut triangles = Vec::with_capacity(n_faces);
        
        for i in 0..n_faces {
            let face_indices = self.faces.get_tri_indices(i);
            
            // Vertex offset should be baked into faces._data during setup()
            // so we can use face_indices directly
            // TODO: updated the offset code so setting it directly face_indices, maybe we
            // can clean this up later
            let vert_offseted_face_indices = face_indices;

            if is_degenerate_triangle(verts, vert_offseted_face_indices) {
                continue;
            }
            
            
            let [v1, v2, v3] = vert_offseted_face_indices.map(|i| verts[i]);

            let cpd = CommonPrimitiveData{
                _id: id_offset + i, 
                material_idx: self.material_idx,
                transformation_names: None, 
                texture_idxs: self.texture_idxs.clone(),
            };


            let tex_offset: isize = {
                if let Some(tex_offset) = self.faces._texture_offset {
                    tex_offset
                } else {
                    0
                }
            };
            
            // Texture indices need to un-offset the vertex offset first, then apply texture offset
            // TODO: any better way than reversing vertex offset here? clones are causing some problems, so currently hw6/brdf and hw4/galactica scenes can only work under this setting...
            let vertex_offset = self.faces._vertex_offset.unwrap_or(0);
            let mut texture_indices = vert_offseted_face_indices.clone();
            texture_indices[0] = ((texture_indices[0] as isize - vertex_offset) + tex_offset) as usize;
            texture_indices[1] = ((texture_indices[1] as isize - vertex_offset) + tex_offset) as usize;
            texture_indices[2] = ((texture_indices[2] as isize - vertex_offset) + tex_offset) as usize;


            triangles.push(Triangle {
                _data: cpd,
                vert_indices: vert_offseted_face_indices,
                is_smooth: self._shading_mode.eq_ignore_ascii_case("smooth"),
                normal: get_tri_normal(&v1, &v2, &v3),
                matrix: None, //Some(Arc::new(self.matrix)), // NOTE: here it is ok to .clone( ) because it just increases Arc's counter, not cloning the whole data
                texture_indices, // WARNING: DO NOT CONFUSE IT WITH TEXTUREMAP IDS which is also named texture_idxs in CommonPrimitiveData
                //_texture_offset: tex_offset,
                //_vertex_offset: vert_offset,
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
                if bvh.intersect(ray, t_interval, vertex_cache, &mut closest, false) { // Early break: false for BLAS (adding it to BLAS didn't improve results, only cluttered my intersect( ) functions in impl Shape trait)
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
        
        // Motion blur (Note: normally we inverse transform the ray along translation but here I add it first, it is transformed to inverse in the next step tgogether with object transformation since they have the same logic)
        let mut ray = ray.clone();
        ray.origin += self.motionblur * ray.time;

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
        let mut local_box = BBox::new_from(&xint, &yint, &zint);
        local_box = local_box.expand_by_motion(self.motionblur);
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
        
        // Motion blur (Note: normally we inverse transform the ray along translation but here I add it first, it is transformed to inverse in the next step tgogether with object transformation since they have the same logic)
        let mut ray = ray.clone();
        ray.origin += self.motionblur * ray.time;

        let base_mesh = self.base_mesh.as_deref().unwrap();
        
        if self.reset_transform {
            let inv_instance = self.matrix.inverse();
            let local_ray = ray.inverse_transform(&inv_instance);
            
            // Intersect without applying base mesh's transform
            if let Some(mut hit) = base_mesh.intersect_bvh(&local_ray, t_interval, vertex_cache) {
                hit.material = self.material_id.unwrap_or(self.base_mesh.clone().unwrap().material_idx);
                hit.textures = self.texture_idxs.clone();
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
                hit.textures = self.texture_idxs.clone();
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
            
            let mut local_box = base_mesh.get_bbox(verts, false);
            local_box = local_box.expand_by_motion(self.motionblur);
            let mut composite = self.matrix;

            if !self.reset_transform{
                composite *= base_mesh.matrix;
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