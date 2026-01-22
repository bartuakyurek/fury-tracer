





use bevy_math::{NormedVectorSpace, VectorSpace};
use rand::random;

use crate::prelude::*;


#[derive(Clone, Debug)]
pub struct Ray {
    pub origin: Vector3,
    pub direction: Vector3,
    pub(crate) time: Float, // set nonzero for motion blur
}

impl Ray {

    pub fn new(origin: Vector3, direction: Vector3, time: Float) -> Self {
        // WARNING: Direction does not need to be normalized actually, hw4 veach ajar scene
        // fails if we normalize it... I'm not sure why this happens
        debug_assert!(direction.is_normalized());
        Self {
            origin,
            direction,
            time,
        }
    }

    pub fn new_from(origin: Vector3, direction: Vector3) -> Self {
        debug_assert!(direction.is_normalized());
        Self {
            origin,
            direction,
            time: 0.,
        }
    }

    pub fn new_with_random_t(origin: Vector3, direction: Vector3) -> Self {
        debug_assert!(direction.is_normalized());
        let t = random_float();
        debug_assert!(t >= 0.0 && t <= 1.0);
        Self {
            origin,
            direction,
            time: random_float(),
        }
    }

    #[inline]
    pub fn translate(&mut self, u: Vector3) {
        self.origin += u;
    }

    #[inline]
    pub fn get_translated(&self, u: Vector3) -> Self {
        Self {
            origin: self.origin + u,
            ..self.clone()
        }
    }

    #[inline] // TODO: does it matter? could you benchmark?
    pub fn at(&self, t: Float) -> Vector3 {
        self.origin + self.direction * t // r(t) = o + dt
    }

    #[inline]
    pub fn squared_distance_at(&self, t: Float) -> Float {
        // Squared distance between ray origin and ray(t) point
        (self.at(t) - self.origin).norm_squared()
    }

    #[inline]
    pub fn distance_at(&self, t: Float) -> Float {
        (self.at(t) - self.origin).norm()
    }

    #[inline]
    pub fn is_front_face(&self, normal: Vector3) -> bool {
         self.direction.dot(normal) <= 0.0 
    }

    #[inline]
    pub fn inverse_transform(&self, inv_matrix: &Matrix4) -> Ray {
        // slides 04, p.50
        let local_origin = transform_point(inv_matrix, &self.origin);
        let mat3 = Matrix3::from_mat4(*inv_matrix); //.transpose();
        let local_direction = mat3 * self.direction;
        //local_direction = local_direction.normalize(); DO NOT NORMALIZE!
        Ray::new(local_origin, local_direction, self.time)
    }
}


// Question: Couldn't we just use point to see which point is closer?
// but this is relative to camera, and t is a single scalar that encaptures
// which HitRecord is closer to the - actually not to camera (for primary rays only it is camera)
// but ray origin so t=0 is at ray origin, smaller t is, closer the object is.  
//
// DISCLAIMER: This struct is based on the approach presented in Ray Tracing in One Weekend book.
#[derive(Debug, Default)]  
pub struct HitRecord {
    pub entry_point: Vector3,
    pub hit_point: Vector3,
    pub normal: Vector3,
    pub is_front_face: bool,
    pub ray_t: Float,  // To check which HitRecord has smaller t 
    
    pub material: usize, // TODO: Should we hold the index of material or actually Option<Rc<dyn Material>> as in here https://the-ray-tracing-road-to-rust.vercel.app/9-metal? Or Arc instead of Rc if we use rayon in future.
    pub textures: Vec<usize>,
    pub texture_uv: Option<[Float; 2]>,
    pub tbn_matrix: Option<Matrix3>, // Tangent space matric (TBN matrix in slides 07 pp.10-16)

    pub radiance: Option<Vector3>,
    pub emissive_ptr: Option<Arc<dyn crate::shapes::EmissiveShape>>,
    pub emissive_shape_id: Option<usize>, // ID of the emissive shape to identify it reliably
}

impl HitRecord {
    // TODO: would it be better to use refs here instead of cloning?
    pub fn new_from(entry_point: Vector3, 
                    hit_point: Vector3, 
                    normal: Vector3, 
                    ray_t: Float, 
                    material: usize, 
                    is_front_face: bool, 
                    texs: Vec<usize>, 
                    uv: Option<[Float;2]>,
                    tbn: Option<Matrix3>) -> Self {
        Self {
            entry_point,
            hit_point,
            normal,
            ray_t,
            material,
            is_front_face,
            textures: texs,
            texture_uv: uv,
            tbn_matrix: tbn,
            radiance: None,
            emissive_ptr: None,
            emissive_shape_id: None,
        }
    }

    
    #[inline]
    pub fn to_world(&mut self, mat4: &Matrix4) {
        // WARNING: What about entry point??? <-------------------------
        self.entry_point = transform_point(mat4, &self.entry_point);

        // Slides 04, p.51
        // Transform hit point to world space
        self.hit_point = transform_point(mat4, &self.hit_point);
        // Transform normal to world space
        // WARNING: for normal only use upper 3x3, see p.53 
        // TODO: Cache?
        let mat3 = Matrix3::from_mat4(*mat4); 
        let inv_transpose = mat3.inverse().transpose();
        self.normal = (inv_transpose * self.normal).normalize();
    }
}



