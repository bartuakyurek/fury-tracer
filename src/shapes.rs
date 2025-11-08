/*

    Declare primitives: Triangle, Sphere, Plane
    

    @date: Oct, 2025
    @author: bartu
*/

use bevy_math::NormedVectorSpace;
use std::{fmt::Debug};

use crate::geometry::{get_tri_normal, moller_trumbore_intersection, HeapAllocatedVerts};

use crate::ray::{Ray, HitRecord}; // TODO: Can we create a small crate for gathering shapes.rs, ray.rs?
use crate::interval::{Interval};
use crate::prelude::*;

pub type HeapAllocatedShape = Arc<dyn PrimitiveShape>;
pub type ShapeList = Vec<HeapAllocatedShape>; 

pub trait PrimitiveShape : Debug + Send + Sync  {
    //fn normal(&self, _: &VertexData) -> Option<Vector3> {
    //    None
    //}
    fn indices(&self) -> Vec<usize>;
    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord>;
}

// Raw data deserialized from .JSON file
// WARNING: it assumes vertex indices start from 1
// TODO: How to convert this struct into V, F matrices, for both array of triangles and Mesh objects in the scene?
#[derive(Debug, Deserialize, Clone, SmartDefault)]
pub struct Triangle {
    #[serde(deserialize_with = "deser_usize")]
    pub _id: usize,
    #[serde(rename = "Indices", deserialize_with = "deser_usize_array")]
    pub indices: [usize; 3],
    #[serde(rename = "Material", deserialize_with = "deser_usize")]
    pub material_idx: usize,

    #[serde(skip)]
    #[default = false]
    pub is_smooth: bool,

    #[serde(skip)]
    pub normal: Vector3,
}

impl PrimitiveShape for Triangle {
    fn indices(&self) -> Vec<usize> {
        self.indices.to_vec()
    }

    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {

        // TODO: cache vertex / face normals
        // WARNING: vertex normals are tricky because if the same vertex was used by multiple 
        // meshes, that means there are more vertex normals than the length of vertexdata because
        // connectivities are different. Perhaps it is safe to assume no vertex is used in multiple
        // objects, but there needs to be function to actually check the scene if a vertex in VertexData
        // only referred by a single scene object. 
        // Furthermore, what if there were multiple VertexData to load multiple meshes in the Scene? 
        // this is not handled yet and our assumption is VertexData is the only source of vertices, every
        // shape refers to this data for their coordinates. 
        
        let verts = &vertex_cache.vertex_data;
        if let Some((u, v, t)) = moller_trumbore_intersection(ray, t_interval, self.indices, verts) {
            
            let p = ray.at(t); // Construct hit point p // TODO: would it be faster to use barycentric u,v here? 
            let tri_normal = {
                
                if self.is_smooth {
                    let v1_n = vertex_cache.vertex_normals[self.indices[0]];
                    let v2_n = vertex_cache.vertex_normals[self.indices[1]];
                    let v3_n = vertex_cache.vertex_normals[self.indices[2]];
                    let w = 1. - u - v;
                    (v1_n * w + v2_n * u + v3_n * v).normalize() // WARNING: Be careful with interpolation order!
                }
                else {
                    if self.normal.norm_squared() > 0.0 {
                        self.normal
                    } else {
                        // info!("I hope this never occurs"); --> WARNING: Occurs when triangle is not constructed from Mesh data
                        let verts = &vertex_cache.vertex_data;
                        let [a, b, c] = self.indices.map(|i| verts[i]);
                        get_tri_normal(&a, &b, &c)
                    }
                }
            };
           
            let front_face = ray.is_front_face(tri_normal);
            let normal = if front_face { tri_normal } else { -tri_normal };
            Some(HitRecord::new(ray.origin, p, normal, t, self.material_idx, front_face)) 
        }
        else {
            None
        }
        
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Sphere {
    #[serde(deserialize_with = "deser_usize")]
    pub _id: usize,
    #[serde(rename = "Center", deserialize_with = "deser_usize")]
    pub center_idx: usize, // Refers to VertexData
    #[serde(rename = "Radius", deserialize_with = "deser_float")]
    pub radius: Float,
    #[serde(rename = "Material", deserialize_with = "deser_usize")]
    pub material_idx: usize,
}

impl PrimitiveShape for Sphere {

    fn indices(&self) -> Vec<usize> {
        [self.center_idx].to_vec()
    }

    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
        
        // Based on Slides 01_B, p.11, Ray-Sphere Intersection 
        let verts = &vertex_cache.vertex_data;
        let center = verts[self.center_idx];
        let o_minus_c = ray.origin - center;
        let d_dot_d: Float = ray.direction.dot(ray.direction);
        let oc_dot_oc: Float = o_minus_c.dot(o_minus_c);
        let d_dot_oc: Float = ray.direction.dot(o_minus_c);
        let discriminant_left: Float = d_dot_oc.powi(2) as Float;
        let discriminant_right: Float = d_dot_d * (oc_dot_oc - self.radius.powi(2)) as Float; // TODO: cache radius squared?
        let discriminant: Float = discriminant_left - discriminant_right;
        if discriminant < 0. { // Negative square root
            None
        }
        else {
            
            let discriminant = discriminant.sqrt();
            let t1 = (-d_dot_oc + discriminant) / d_dot_d;
            let t2 = (-d_dot_oc - discriminant) / d_dot_d; // t2 < t1 
            
            let t= if t2 > 0.0 {t2} else {t1}; // Pick smaller first
            if !t_interval.contains(t) {
                return None;  // Invalid intersection
            };
            
            let point = ray.at(t); // Note that this computation is done inside new_from as well
            let normal = (point - center).normalize(); // TODO: is this correct?
            
            let is_front_face = ray.is_front_face(normal);
            let normal = if is_front_face { normal } else { -normal };
            Some(HitRecord::new(ray.origin, point, normal, t, self.material_idx, is_front_face))
            
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Plane {
    #[serde(deserialize_with = "deser_usize")]
    pub _id: usize,
    #[serde(rename = "Point", deserialize_with = "deser_usize")]
    pub point_idx: usize,
    #[serde(rename = "Normal", deserialize_with = "deser_vec3")]
    pub normal: Vector3,
    #[serde(rename = "Material", deserialize_with = "deser_usize")]
    pub material_idx: usize,
}

impl PrimitiveShape for Plane {

    fn indices(&self) -> Vec<usize> {
        [self.point_idx].to_vec()
    }
    
    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
       // Based on Slides 01_B, p.9, Ray-Plane Intersection 
        let verts = &vertex_cache.vertex_data;
        let a_point_on_plane = verts[self.point_idx];
        let dist = a_point_on_plane - ray.origin;
        let  t = dist.dot(self.normal) / ray.direction.dot(self.normal);

        if t_interval.contains(t) {
            // Construct Hit Record
            let front_face = ray.is_front_face(self.normal);
            let normal = if front_face { self.normal } else { -self.normal };
            Some(HitRecord::new(ray.origin, ray.at(t), normal, t, self.material_idx, front_face))
        }
        else {
            None // t is not within the limits
        }
    }
}
