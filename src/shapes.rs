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
use crate::interval::{FloatConst, Interval};
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

#[derive(Debug, Deserialize, Clone, SmartDefault)]
pub(crate) struct CommonPrimitiveData {
    #[serde(deserialize_with = "deser_usize")]
    pub _id: usize,

    #[serde(rename = "Material", deserialize_with = "deser_usize")]
    pub material_idx: usize,

    #[serde(rename = "Transformations", default)]
    pub transformation_names: Option<String>,

    #[serde(rename = "Textures", deserialize_with = "deser_usize_vec", default)]
    pub texture_idxs: Vec<usize>,
}

// =======================================================================================================
// Triangle (impl Shape + BBoxable)
// =======================================================================================================

// Raw data deserialized from .JSON file
// WARNING: it assumes vertex indices start from 1
#[derive(Debug, Deserialize, Clone, SmartDefault)]
pub struct Triangle {
    #[serde(flatten)]
    pub(crate) _data: CommonPrimitiveData, 

    #[serde(rename = "Indices", deserialize_with = "deser_usize_array")]
    pub vert_indices: [usize; 3],
    
    #[serde(skip)]
    pub matrix: Option<Arc<Matrix4>>, // Arc here to share Transformations with Mesh, I didn't want to clone the same transform while creating triangles for mesh

    #[serde(skip)]
    #[default = false]
    pub is_smooth: bool,

    #[serde(skip)]
    pub normal: Vector3,

    #[serde(skip)]
    pub texture_indices: [usize; 3],

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
        if let Some((bary_beta, bary_gamma, t)) = moller_trumbore_intersection(ray, t_interval, self.vert_indices, verts) {
            
            let p = ray.at(t); // Construct hit point p // TODO: would it be faster to use barycentric u,v here? 
            let mut tri_normal = {
                
                if self.is_smooth {
                    let v1_n = vertex_cache.vertex_normals[self.vert_indices[0]];
                    let v2_n = vertex_cache.vertex_normals[self.vert_indices[1]];
                    let v3_n = vertex_cache.vertex_normals[self.vert_indices[2]];
                    let bary_w = 1. - bary_beta - bary_gamma;
                    (v1_n * bary_w + v2_n * bary_beta + v3_n * bary_gamma).normalize() // WARNING: Be careful with interpolation order!
                } 
                else if self.normal.norm_squared() > 0.0 {
                    self.normal
                } 
                else {
                    // Occurs when triangle is not constructed from Mesh data
                    let [a, b, c] = self.vert_indices.map(|i| verts[i]);
                    get_tri_normal(&a, &b, &c)
                }
            };
           
            let front_face = ray.is_front_face(tri_normal);
            tri_normal = if front_face { tri_normal } else { -tri_normal };

            // ------ Create hitrecord wrt transform ------------------

            let mut texture_uv = None; 
            let mut tbn = None;
            let texs = self._data.texture_idxs.clone(); // TODO: any better ideas to avoid clone?
            if !texs.is_empty() {
                // See slides 06, p.20
                let (a, b, c) = (self.texture_indices[0], self.texture_indices[1], self.texture_indices[2]);
                
                debug_assert!(a > 0 && b > 0 && c > 0, "Assumption of vertex indices starting from 1 failed!");
                let uv_a: [Float; 2] = vertex_cache.uv_coords[a].unwrap_or_default();
                let uv_b: [Float; 2] = vertex_cache.uv_coords[b].unwrap_or_default();
                let uv_c: [Float; 2] = vertex_cache.uv_coords[c].unwrap_or_default(); // TODO: this isn't a good solution but in case of perlin noise u, v is not needed so _or_default avoids kernel panic for meshes without uv given ... I'd better check the texture type but I dont want to infer it here
                //debug_assert!(uv_a[0] <= 1.0 && uv_a[1] <= 1.0, "Failed uv_a > 1: ({}, {})", uv_a[0], uv_a[1]);
                //debug_assert!(uv_b[0] <= 1.0 && uv_b[1] <= 1.0, "Failed uv_b > 1: ({}, {})", uv_b[0], uv_b[1]);
                //debug_assert!(uv_c[0] <= 1.0 && uv_c[1] <= 1.0, "Failed uv_c > 1: ({}, {})", uv_c[0], uv_c[1]);
                // Above is not necessary if tiling is allowed
                const NEG_ZERO: Float = -1e-2; 
                debug_assert!(uv_a[0] >= NEG_ZERO && uv_a[1] >= NEG_ZERO, "Failed uv_a < 0: ({}, {})", uv_a[0], uv_a[1]);
                debug_assert!(uv_b[0] >= NEG_ZERO && uv_b[1] >= NEG_ZERO, "Failed uv_b < 0: ({}, {})", uv_b[0], uv_b[1]);
                debug_assert!(uv_c[0] >= NEG_ZERO && uv_c[1] >= NEG_ZERO, "Failed uv_c < 0: ({}, {})", uv_c[0], uv_c[1]);
        
                let tex_u: Float = uv_a[0] + (bary_beta * (uv_b[0] - uv_a[0])) + (bary_gamma * (uv_c[0] - uv_a[0]));
                let tex_v: Float = uv_a[1] + (bary_beta * (uv_b[1] - uv_a[1])) + (bary_gamma * (uv_c[1] - uv_a[1]));

                let tex_u = tex_u - tex_u.floor(); // support tiling
                let tex_v = tex_v - tex_v.floor(); // slides 06, p.30 

                debug_assert!(tex_u <= 1.0 && tex_u >= 0.0);
                debug_assert!(tex_v <= 1.0 && tex_v >= 0.0);
                texture_uv = Some([tex_u, tex_v]);

                // Compute TBN matrix for triangle (see slides 07, pp.10-16) ---------------------------------------------
                let u_col = Vector2::new(uv_b[0] - uv_a[0], uv_c[0] - uv_a[0]);
                let v_col = Vector2::new(uv_b[1] - uv_a[1], uv_c[1] - uv_a[1]);
                let first_mat2 = Matrix2::from_cols(u_col, v_col); // p.13
                
                // Check if matrix is invertible (determinant != 0)
                let det = first_mat2.determinant();
                if det.abs() > 1e-6 {
                    let inverse_mat2 = first_mat2.inverse();

                    let x_axis = Vector2::new(verts[b].x - verts[a].x, verts[c].x - verts[a].x);
                    let y_axis = Vector2::new(verts[b].y - verts[a].y, verts[c].y - verts[a].y);
                    let z_axis = Vector2::new(verts[b].z - verts[a].z, verts[c].z - verts[a].z);
                    // TODO: Since bevy does not support 2x3 matrices and I am lazy to convert all math to ndarray, here is a quick solution...
                    let tx_bx = inverse_mat2 * x_axis;
                    let ty_by = inverse_mat2 * y_axis;
                    let tz_bz = inverse_mat2 * z_axis;
                    let t_vec = Vector3::new(tx_bx.x, ty_by.x, tz_bz.x).normalize();
                    let b_vec = Vector3::new(tx_bx.y, ty_by.y, tz_bz.y).normalize();
                    //debug_assert!(approx_zero(t_vec.dot(b_vec)), "Found non-orthogonal vectors t_vec: {}, b_vec: {}, t dot b: {}", t_vec, b_vec, t_vec.dot(b_vec)); // Orthogonality
                    debug_assert!(approx_zero(t_vec.dot(tri_normal)));
                    tbn = Some(Matrix3::from_cols(t_vec, b_vec, tri_normal));
                } else {
                    // UPDATE after HW4: Fix galactica scene not rendering properly 
                    debug!("Degenerate UV coordinates for triangle (det={}), using fallback TBN", det);
                    let reference = if tri_normal.x.abs() < 0.9 {
                        Vector3::X
                    } else {
                        Vector3::Y
                    };
                    let t_vec = tri_normal.cross(reference).normalize();
                    let b_vec = tri_normal.cross(t_vec).normalize();
                    tbn = Some(Matrix3::from_cols(t_vec, b_vec, tri_normal));
                }
                // -------------------------------------------------------------------------------------------------------
            }
            let mut rec = HitRecord::new_from(ray.origin, p, tri_normal, t, self._data.material_idx, front_face, texs, texture_uv, tbn);
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
        for &i in &self.vert_indices { // using & to borrow instead of move
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
    #[serde(flatten)]
    pub(crate) _data: CommonPrimitiveData, 

    #[serde(rename = "Center", deserialize_with = "deser_usize")]
    pub center_idx: usize, // Refers to VertexData
    #[serde(rename = "Radius", deserialize_with = "deser_float")]
    pub radius: Float,

    #[serde(rename = "MotionBlur", deserialize_with = "deser_vec3", default)]
    pub(crate) motionblur: Vector3, 

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
        let p_world = transform_point(&viewmat, &p_local); 

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
        let mut world_normal = transform_normal(&viewmat, &local_normal); 
        if world_normal.norm_squared() > 0.0 { world_normal = world_normal.normalize(); }

        // Check front face and build hitrecord (I was transforming hitrecord::to_world( ) but here it is already transformed.)
        let front_face = ray.is_front_face(world_normal);
        let final_normal = if front_face { world_normal } else { -world_normal };
        
        // Check texture uv coords
        let mut tbn = None;
        let mut uv = None;
        if !self._data.texture_idxs.is_empty() { 
            // See slides 06, p.6-7
            // (assumes sphere center is at origin, so we translate hitpoint by the center)
            let p = p_local - center; 
            
            let theta = ( p.y / self.radius ).acos();
            let phi = p.z.atan2(p.x);
            let u = (-phi + Float::PI) / (2. * Float::PI);
            let v = theta / Float::PI;
            uv = Some([u, v]);

            // TODO: avoid trigonometry calls here
            let cos_phi = phi.cos();
            let sin_theta = theta.sin();
            let sin_phi = phi.sin();

            let t_vec = Vector3::new(2. * Float::PI * p.z, 0., - 2. * Float::PI * p.x).normalize();
            let b_vec = Vector3::new(Float::PI * p.y * cos_phi, - Float::PI * self.radius * sin_theta, Float::PI * p.y * sin_phi).normalize();
            let n_vec = b_vec.cross(t_vec); // see slides 07, p.18
            // Compute TBN matrix for sphere (see slides 07, pp.10-16)
            tbn = Some(Matrix3::from_cols(t_vec, b_vec, n_vec));
        }
        
        let texs = self._data.texture_idxs.clone(); // TODO: I keep cloning texture indices "because Vec<usize> does not implement copy trait" but I dont want to impl Copy and let clone occur under the hood, any better solution?
        let rec = HitRecord::new_from(ray.origin, p_world, final_normal, t_world, self._data.material_idx, front_face, texs, uv, tbn);
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
                local_box.transform(matrix) // return transformed bbox
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
    #[serde(flatten)]
    pub(crate) _data: CommonPrimitiveData, 

    #[serde(rename = "Point", deserialize_with = "deser_usize")]
    pub point_idx: usize,
    #[serde(rename = "Normal", deserialize_with = "deser_vec3")]
    pub normal: Vector3,
    #[serde(rename = "MotionBlur", deserialize_with = "deser_vec3", default)]
    pub(crate) motionblur: Vector3, 

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

        // Check texture uv coords
        let uv = Some([-999999., -99999.]); // Dummy uv is set //todo!("Create uv for Plane hits (i guess we leave it None for planes, no?)!");
        
        // Compute TBN for plane
        let n = self.normal; 
        debug_assert!(n.is_normalized());
        let reference = if n.x.abs() < 0.9 {
            Vector3::X
        } else {
            Vector3::Y
        };
        let t_vec = n.cross(reference).normalize();
        let b_vec = n.cross(t_vec).normalize();
        let tbn = Matrix3::from_cols(t_vec, b_vec, n);
        let tbn = Some(tbn);
        //let tbn = None; // todo!("Construct TBN for plane!");
        
        
        
        let texs = self._data.texture_idxs.clone();
        let mut rec = HitRecord::new_from(ray.origin, ray.at(t), normal, t, self._data.material_idx, front_face, texs, uv, tbn);

        // transform hitpoint and normal (04, p.53) -----
        rec.to_world(&viewmat);
        Some(rec)
    }
}


impl BBoxable for Plane {
    /// Dummy bbox with no volume -- WARNING: Not to be used in BVH! BBoxable was meant to be separated from Shapes trait
    /// but I couldn't figure out how to set trait bounds without using trait objects in the scene object
    /// vectors yet... 
     fn get_bbox(&self, _: &VertexData, _: bool) -> BBox {
        todo!();
    }
}
