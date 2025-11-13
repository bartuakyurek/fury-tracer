/*

    Declare primitives: Triangle, Sphere, Plane
    

    @date: Oct, 2025
    @author: bartu
*/

use bevy_math::NormedVectorSpace;
use std::{fmt::Debug};

use crate::geometry::{get_tri_normal, moller_trumbore_intersection};

use crate::bbox::{BBox, BBoxable};
use crate::ray::{Ray, HitRecord}; // TODO: Can we create a small crate for gathering shapes.rs, ray.rs?
use crate::interval::{Interval};
use crate::json_structs::{VertexData};
use crate::scene::HeapAllocatedVerts;
use crate::prelude::*;

pub type HeapAllocatedShape = Arc<dyn Shape>;
pub type ShapeList = Vec<HeapAllocatedShape>; 


// =======================================================================================================
// Shape Trait
// =======================================================================================================
pub trait Shape : Debug + Send + Sync + BBoxable {
    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord>;
}


// =======================================================================================================
// Triangle (impl Shape + BBoxable)
// =======================================================================================================

// Raw data deserialized from .JSON file
// WARNING: it assumes vertex indices start from 1
#[derive(Debug, Deserialize, Clone, SmartDefault)]
pub struct Triangle {
    #[serde(deserialize_with = "deser_usize")]
    pub _id: usize,
    #[serde(rename = "Indices", deserialize_with = "deser_usize_array")]
    pub indices: [usize; 3],
    #[serde(rename = "Material", deserialize_with = "deser_usize")]
    pub material_idx: usize,

    #[serde(rename = "Transformations", default)]
    pub transformations: Option<String>,

    #[serde(skip)]
    #[default = false]
    pub is_smooth: bool,

    #[serde(skip)]
    pub normal: Vector3,
}

impl Shape for Triangle {
    

    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
        
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


impl BBoxable for Triangle {
    fn get_bbox(&self, verts: &VertexData) -> BBox {
        let (mut xint, mut yint, mut zint) = (Interval::EMPTY, Interval::EMPTY, Interval::EMPTY);
        for &i in &self.indices { // using & to borrow instead of move
            let v = verts[i];

            xint.expand(v.x);
            yint.expand(v.y);
            zint.expand(v.z);
        }

        BBox::new_from(&xint, &yint, &zint)
    }
}

// =======================================================================================================
// Sphere (impl Shape + BBoxable)
// =======================================================================================================
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

    #[serde(rename = "Transformations", default)]
    pub transformations: Option<String>,

}

impl Shape for Sphere {

   

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

impl BBoxable for Sphere {
    fn get_bbox(&self, verts: &VertexData) -> BBox {
        
        let center = verts[self.center_idx];

        let xint = Interval::new(center.x - self.radius, center.x + self.radius);
        let yint = Interval::new(center.y - self.radius, center.y + self.radius);
        let zint = Interval::new(center.z - self.radius, center.z + self.radius);

        BBox::new_from(&xint, &yint, &zint)
    }
}


// =======================================================================================================
// Plane (impl Shape)
// =======================================================================================================

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

    #[serde(rename = "Transformations", default)]
    pub transformations: Option<String>,

}

impl Shape for Plane {

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

impl BBoxable for Plane {
    /// Dummy bbox with no volume
     fn get_bbox(&self, verts: &VertexData) -> BBox {
        let p = verts[self.point_idx];
        let xint = Interval::new(p.x, p.x);
        let yint = Interval::new(p.y, p.y);
        let zint = Interval::new(p.z, p.z);
        BBox::new_from(&xint, &yint, &zint)
    }
}
