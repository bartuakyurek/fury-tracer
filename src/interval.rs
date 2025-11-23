/*

    Responsible for creating a struct that represents
    ranges from a to b and functionality to check if
    x is in range [a,b] or (a,b).

    See also associated constants of Interval class:
    - EMPTY: (inf, -inf)
    - UNIVERSE: (-inf, inf)
    - NONNEGATIVE: (0*, inf)
    - UNIT: (0*, 1*) 
    
    *with epsilon

    @author: Bartu
    @date: Sept 2025

*/

use crate::numeric::{Float};

#[derive(Debug, Clone, Copy)]
pub struct Interval {
    pub min: Float,
    pub max: Float,
}

impl Interval {

    pub const EMPTY: Self = Self {
        min: FloatConst::INF,
        max: FloatConst::NEG_INF,
    };

    pub const UNIVERSE: Self = Self {
        min: FloatConst::NEG_INF,
        max: FloatConst::INF,
    };

    pub const NONNEGATIVE: Self = Self {
        min: 0.0,
        max: FloatConst::INF,
    };

    pub fn validate(&self) -> bool {
        self.max >= self.min
    }

    pub fn new(min: Float, max: Float) -> Self {
        Self { 
            min,
            max,
        }
    }

    pub fn new_with_mineps(min: Float, max: Float, epsilon: Float) -> Self {
        Self { 
            min: {min + epsilon}, 
            max,
        }
    }

    pub fn positive(epsilon: Float) -> Self {
        // [epsilon, inf)
        Self { 
            min: epsilon, 
            max: FloatConst::INF,
        }
    }

    pub fn new_with_minmaxeps(min: Float, max: Float, epsilon: Float) -> Self {
        // [min + epsilon, max - epsilon]
        Self { 
            min: {min + epsilon}, 
            max: {max - epsilon},
        }
    }

    pub fn unit(epsilon: Float) -> Self {
        Self {
            min: epsilon,
            max: {1.0 - epsilon},
        }
    }
   
    pub fn size(&self) -> Float {
        self.max - self.min
    }

    pub fn contains(&self, x: Float) -> bool {
        self.min <= x && x <= self.max
    }

    pub fn surrounds(&self, x: Float) -> bool {
        self.min < x && x < self.max
    }

    pub fn clamp(&self, x: Float) -> Float {
        if x < self.min { self. min }
        else if x > self.max { self.max }
        else { x }
    }

    pub fn expand(&mut self, x: Float) {
        if x < self.min { self.min = x; }
        if x > self.max { self.max = x; }
    }

}



// TODO: Allow epsilons to be set by outside of the crate
// Perhaps with use of enums or remove 0* 1* from consts and 
// add functions to construct such epsilon intervals
pub trait FloatConst: Copy {
    const PI: Self;
    const INF: Self;
    const NEG_INF: Self;
}

impl FloatConst for f32 {
    const PI: Self = std::f32::consts::PI;
    const INF: Self = f32::INFINITY;
    const NEG_INF: Self = f32::NEG_INFINITY;
}

impl FloatConst for f64 {
    const PI: Self = std::f64::consts::PI;
    const INF: Self = f64::INFINITY;
    const NEG_INF: Self = f64::NEG_INFINITY;
}