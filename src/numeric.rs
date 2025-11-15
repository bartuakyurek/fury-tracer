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

use bevy_math::{DMat3, DMat4, DVec3, DVec4};
pub type Int = i32;
pub type Float = f64; // WARNING: If you want to change it to f32, don't forget to update Vector3 and Matrix3 types
pub type Vector3 = DVec3; 
pub type Matrix3 = DMat3;
pub type Matrix4 = DMat4;
pub type Vector4 = DVec4;

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