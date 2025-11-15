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
    pub transformation_names: Option<String>,

    #[serde(skip)]
    pub matrix: Option<Arc<Matrix4>>, // Arc here to share Transformations with Mesh, I didn't want to clone the same transform while creating triangles for mesh

    #[serde(skip)]
    #[default = false]
    pub is_smooth: bool,

    #[serde(skip)]
    pub normal: Vector3,
}

impl Shape for Triangle {
    

    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord> {
        
        // ---- Apply transformation --------
        //TODO: how not to copy paste the same logic for other shapes?
        let viewmat = self.matrix.clone().unwrap_or(Arc::new(Matrix4::IDENTITY));
        let inv_matrix = viewmat.inverse(); // TODO: better to cache inverse because of the borrow rules we keep .clone( ) and Arc::new( )
        let ray = &ray.inverse_transform(&inv_matrix);
        // ----------------------------------

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

            // ------ Create hitrecord wrt transform ------------------
            let mut rec = HitRecord::new(ray.origin, p, normal, t, self.material_idx, front_face);
            rec.to_world(&viewmat);
            Some(rec) 
            // --------------------------------------------------------
        }
        else {
            None
        }
        
    }
}


impl BBoxable for Triangle {
    fn get_bbox(&self, verts: &VertexData, apply_t: bool) -> BBox {
        let (mut xint, mut yint, mut zint) = (Interval::EMPTY, Interval::EMPTY, Interval::EMPTY);
        for &i in &self.indices { // using & to borrow instead of move
            let v = verts[i];

            xint.expand(v.x);
            yint.expand(v.y);
            zint.expand(v.z);
        }

        let local_box = BBox::new_from(&xint, &yint, &zint);
        if apply_t {
            if let Some(matrix) = &self.matrix {
                local_box.transform(matrix)
            } else {
                warn!("No transformation matrix found for Triangle. Returning local bounding box.");
                local_box
            }
        } else {
            local_box
        }
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
    pub transformation_names: Option<String>,

    #[serde(skip)]
    pub matrix: Option<Arc<Matrix4>>, // Arc here to share Transformations with Mesh, I didn't want to clone the same transform while creating triangles for mesh

}
impl Shape for Sphere {
    fn intersects_with(&self, ray: &Ray, t_interval: &Interval, vertex_cache: &HeapAllocatedVerts)
        -> Option<HitRecord>
    {
        // --- Transform ray into local space ---
        let viewmat = self.matrix.clone().unwrap_or(Arc::new(Matrix4::IDENTITY));
        let inv_matrix = viewmat.inverse();
        let local_ray = &ray.inverse_transform(&inv_matrix);

        // --- Sphere intersection in local space ---
        let verts = &vertex_cache.vertex_data;
        let center = verts[self.center_idx];

        let o_minus_c = local_ray.origin - center;
        let a: Float = local_ray.direction.dot(local_ray.direction);
        let b: Float = 2.0 * local_ray.direction.dot(o_minus_c);
        let c: Float = o_minus_c.dot(o_minus_c) - self.radius * self.radius;

        let discriminant = b*b - 4.0*a*c;
        if discriminant < 0.0 {
            return None;
        }
        let sqrt_d = discriminant.sqrt();
        let t1 = (-b - sqrt_d) / (2.0*a); 
        let t2 = (-b + sqrt_d) / (2.0*a); 

        // Pick the closer t
        let t_local = if t1 > 0.0 { t1 } else if t2 > 0.0 { t2 } else { return None; };

        // Compute hit in local space and then transform back  to world
        let p_local = local_ray.at(t_local);
        let p_world = transform_point(&*viewmat, &p_local); 

        // Update ray t to worlds space
        let ray_dir_lensqrd = ray.direction.dot(ray.direction);
        if ray_dir_lensqrd == 0.0 { // Avoid division by zero
            return None; 
        }
        let t_world = (p_world - ray.origin).dot(ray.direction) / ray_dir_lensqrd;
        if !t_interval.contains(t_world) || t_world <= 0.0 {
            return None;
        }

        // World space normal
        let local_normal = (p_local - center).normalize();
        let mut world_normal = transform_normal(&*viewmat, &local_normal); 
        if world_normal.norm_squared() > 0.0 { world_normal = world_normal.normalize(); }

        // Check front face and build hitrecord (I was transforming hitrecord::to_world( ) but here it is already transformed.)
        let front_face = ray.is_front_face(world_normal);
        let final_normal = if front_face { world_normal } else { -world_normal };
        let rec = HitRecord::new(ray.origin, p_world, final_normal, t_world, self.material_idx, front_face);
        Some(rec)
    }
}

impl BBoxable for Sphere {
    fn get_bbox(&self, verts: &VertexData, apply_t: bool) -> BBox {
        
        let center = verts[self.center_idx];

        let xint = Interval::new(center.x - self.radius, center.x + self.radius);
        let yint = Interval::new(center.y - self.radius, center.y + self.radius);
        let zint = Interval::new(center.z - self.radius, center.z + self.radius);

        let local_box = BBox::new_from(&xint, &yint, &zint);
        if apply_t {
            if let Some(matrix) = &self.matrix {
                let transformed_bbox = local_box.transform(matrix);
                transformed_bbox
            } else {
                warn!("No transformation matrix found for Sphere. Returning local bounding box.");
                local_box
            }
        }
        else {
            local_box
        }
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
    pub transformation_names: Option<String>,

    #[serde(skip)]
    pub matrix: Option<Arc<Matrix4>>, // Arc here to share Transformations with Mesh, I didn't want to clone the same transform while creating triangles for mesh


}

impl Shape for Plane {
    fn intersects_with(
        &self,
        ray: &Ray,
        t_interval: &Interval,
        vertex_cache: &HeapAllocatedVerts
    ) -> Option<HitRecord> {

        // --- Transform ray ---
        let viewmat = self.matrix.clone().unwrap_or(Arc::new(Matrix4::IDENTITY));
        let inv_matrix = viewmat.inverse();
        let ray = &ray.inverse_transform(&inv_matrix);
        // ---------------------
        let verts = &vertex_cache.vertex_data;
        let p0 = verts[self.point_idx];
        let n = self.normal;

        let denom = ray.direction.dot(n);

        // ray is parallel to plane ----
        if denom.abs() < 1e-12 {
            return None;
        }

        let t = (p0 - ray.origin).dot(n) / denom;

        // plane is behind the ray origin ----
        if t <= 0.0 {
            return None;
        }

        //  t must be within interval ----
        if !t_interval.contains(t) {
            return None;
        }

        // Construct Hit Record
        let front_face = ray.is_front_face(n);
        let normal = if front_face { n } else { -n };
        let mut rec = HitRecord::new(ray.origin, ray.at(t), normal, t, self.material_idx, front_face);

        // transform hitpoint and normal (04, p.53) -----
        rec.to_world(&viewmat);
        Some(rec)
    }
}


impl BBoxable for Plane {
    /// Dummy bbox with no volume -- WARNING: Not to be used in BVH! BBoxable was meant to be separated from Shapes trait
    /// but I couldn't figure out how to set trait bounds without using trait objects in the scene object
    /// vectors yet... 
     fn get_bbox(&self, verts: &VertexData, apply_t: bool) -> BBox {
        todo!();
        let p = verts[self.point_idx];
        let xint = Interval::new(p.x, p.x);
        let yint = Interval::new(p.y, p.y);
        let zint = Interval::new(p.z, p.z);

        let local_box = BBox::new_from(&xint, &yint, &zint);
        if apply_t {
            if let Some(matrix) = &self.matrix {
                let transformed_bbox = local_box.transform(matrix);
                transformed_bbox
            } else {
                warn!("No transformation matrix found for Plane. Returning local bounding box.");
                BBox::new_from(&xint, &yint, &zint)
            }
        }
        else {
            local_box
        }
    }
}
