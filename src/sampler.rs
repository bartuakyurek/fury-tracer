

use crate::{interval::FloatConst, numeric::{Float, Vector3, debug_assert_orthonormality}};
use bevy_math::NormedVectorSpace; // Adding this resolves error when using Float::PI "associated item not found in `f64`", idk which trait bounds are satisfied with this

//////////////////////////////////////////////////////////////////////////
/// SAMPLING UTILS
//////////////////////////////////////////////////////////////////////////


pub fn random_float() -> Float {
    // Return a random real in [0.0, 1.0)
    // NOTE: (See https://docs.rs/rand/latest/rand/fn.random.html)
    // If you’re calling random() repeatedly, consider using a local rng handle 
    // to save an initialization-check on each usage
    rand::random::<Float>()
}


pub fn hemisphere_uniform_sample(u: &Vector3, v: &Vector3, n: &Vector3) -> Vector3 {
    // Assuming input vectors are orthonormal
    debug_assert_orthonormality(u, v, n);
    let psi_1 = random_float();
    let psi_2 = random_float();

    // Slides 09, p.51 uniform smapling a hemisphere (formula under "simplifies to")
    let some_coeff  = (1. - psi_1.powf(2.)).sqrt();
    let some_angle = psi_2 * 2. * Float::PI;
    
    let u_coeff: Float = some_coeff * (some_angle).cos(); 
    let v_coeff: Float = some_coeff * (some_angle).sin();

    (u * u_coeff) + (v * v_coeff) + (n * psi_1)
}


pub fn hemisphere_cosine_sample(u: &Vector3, v: &Vector3, n: &Vector3) -> Vector3 {
    debug_assert_orthonormality(u, v, n);

    todo!()
}
