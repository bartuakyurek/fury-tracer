





use bevy_math::NormedVectorSpace;
use rand::random;

use crate::prelude::*;


#[derive(Debug)]
pub struct Ray {
    pub origin: Vector3,
    pub direction: Vector3,
    pub(crate) time: Float, // set nonzero for motion blur
}

impl Ray {

    pub fn new(origin: Vector3, direction: Vector3, time: Float) -> Self {
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
        Ray::new(local_origin, local_direction)
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
    pub ray_t: Float,  // To check which HitRecord has smaller t 
    pub material: usize, // TODO: Should we hold the index of material or actually Option<Rc<dyn Material>> as in here https://the-ray-tracing-road-to-rust.vercel.app/9-metal? Or Arc instead of Rc if we use rayon in future.
    pub is_front_face: bool,
}

impl HitRecord {
    pub fn new(entry_point: Vector3, hit_point: Vector3, normal: Vector3, ray_t: Float, material: usize, is_front_face: bool) -> Self {
        Self {
            entry_point,
            hit_point,
            normal,
            ray_t,
            material,
            is_front_face,
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



