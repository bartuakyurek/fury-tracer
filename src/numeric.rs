/*

    Declare numeric types used throughout this repo.

    WARNING: If you like to use f32 instead of f64
    during computations, you need to change both of these:
    pub type Float = f32;
    pub type Vector3 = Vec3;

    TODO: maybe provide Vector3 struct to avoid this
    explicit coupling rather than depending on bevy_math.

    @date: 2 Oct, 2025
    @author: Bartu
*/

use bevy_math::{DMat2, DMat3, DMat4, DVec2, DVec3, DVec4};
use crate::prelude::*;

pub type Int = i32;
pub type Float = f64; // WARNING: If you want to change it to f32, don't forget to update Vector3 and Matrix3 types
pub type Vector3 = DVec3; 
pub type Matrix3 = DMat3;
pub type Matrix4 = DMat4;
pub type Vector4 = DVec4;
pub type Matrix2 = DMat2;
pub type Vector2 = DVec2;

//#[derive(Clone, Copy, Debug, Default)]
//pub struct Vector3(pub DVec3); // To declare a type and use impl traits on this type
// TODO: Use your own Vector3 to implement its deserialization 
// this requires trait bounds to be satisfied, so it breaks most of the code atm

pub fn approx_zero(x: Float) -> bool {
    x.abs() < 1e-8
}

pub fn transform_point(mat: &Matrix4, v: &Vector3) -> Vector3 {
    let v4 = Vector4::new(v.x, v.y, v.z, 1.0);
    let r = *mat * v4;
    Vector3::new(r.x, r.y, r.z)
}

pub fn transform_dir(mat: &Matrix4, v: &Vector3) -> Vector3 {
    // Only difference from transform_point is that last component
    // w = 0 
    let v4 = Vector4::new(v.x, v.y, v.z, 0.0);
    let r = *mat * v4;
    Vector3::new(r.x, r.y, r.z)
}

pub fn transform_normal(mat: &Matrix4, n: &Vector3) -> Vector3 {
    // Compute inverse transpose matrix 
    let inv = mat.inverse();
    let inv_t = inv.transpose();

    // Normal is a direction, so set w = 0.0
    let n4 = Vector4::new(n.x, n.y, n.z, 0.0);
    let r = inv_t * n4;

    Vector3::new(r.x, r.y, r.z).normalize()
}

pub fn get_onb(normal: &Vector3) -> (Vector3, Vector3) {
    // See slides 05, p.96
    debug_assert!(normal.is_normalized(), "normal is not normalized: normal = {}", normal); 
    let mut u = normal.clone();
    let min_idx = u.abs().min_position(); // FIND THE ABSOLUTE MINIMUM
    u[min_idx] = 0.;
    debug_assert!(!u.is_nan(), "u found Nan: {}", u);
    
    let (i, j) = if min_idx == 2 {(0, 1)} else if min_idx == 1 {(0, 2)} else if min_idx == 0 {(1, 2)} else {panic!("Expected min_idx to be either 0, 1, 2. Got {}.", min_idx)};
    let tmp = u[i]; // swap other two components(std::mem::swap didn't work here due to second mutation not allowed)
    u[i] = - u[j]; // negating one
    u[j] = tmp;
    u = u.normalize();    
    debug_assert!(!u.is_nan(), "u found Nan: {}", u);
    
    let v = u.cross(*normal);
    debug_assert!(v.is_normalized());
    debug_assert!(approx_zero(u.dot(v)), "u and v not orthogonal: dot = {}", u.dot(v));
    debug_assert!(approx_zero(u.dot(*normal)), "u and n not orthogonal: dot = {}", u.dot(*normal));
    debug_assert!(approx_zero(v.dot(*normal)), "v and n not orthogonal: dot = {}", v.dot(*normal));

    (u, v)
}

//////////////////////////////////////////////////////////////////////////
/// Assert utils
//////////////////////////////////////////////////////////////////////////
pub fn debug_assert_orthonormality(u: &Vector3, v: &Vector3, n: &Vector3) {
    debug_assert!(approx_zero(u.dot(*v)), "Expected u, v orthogonality. Found u = {:?} and v = {:?}", u, v);
    debug_assert!(approx_zero(n.dot(*v)), "Expected n, v orthogonality. Found n = {:?} and v = {:?}", n, v);
    debug_assert!(approx_zero(u.dot(*n)), "Expected u, n orthogonality. Found u = {:?} and n = {:?}", u, n);
    debug_assert!(u.is_normalized());
    debug_assert!(v.is_normalized());
    debug_assert!(n.is_normalized());
}
