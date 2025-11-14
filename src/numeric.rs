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

use bevy_math::{DMat3, DVec3, DMat4};
pub type Int = i32;
pub type Float = f64; // WARNING: If you want to change it to f32, don't forget to update Vector3 and Matrix3 types
pub type Vector3 = DVec3; 
pub type Matrix3 = DMat3;
pub type Matrix4 = DMat4;

//#[derive(Clone, Copy, Debug, Default)]
//pub struct Vector3(pub DVec3); // To declare a type and use impl traits on this type
// TODO: Use your own Vector3 to implement its deserialization 
// this requires trait bounds to be satisfied, so it breaks most of the code atm

pub fn approx_zero(x: Float) -> bool {
    x.abs() < 1e-8
}
