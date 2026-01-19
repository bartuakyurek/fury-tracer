

use crate::prelude::*;
use crate::json_structs::{SingleOrVec};
use crate::material::MaterialCommon;

pub trait BRDF {
    fn eval(
            &self,
            wi: Vector3,
            wo: Vector3,
            n: Vector3,
            params: &MaterialCommon,
    ) -> Vector3;
}





pub struct BRDFs {
    pub original_phong: SingleOrVec<Phong>,
    pub modified_phong: SingleOrVec<ModifiedPhong>,
    pub original_blinn_phong: SingleOrVec<BlinnPhong>,
    pub modified_blinn_phong: SingleOrVec<ModifiedBlinnPhong>,
    pub torrance_sparrow: SingleOrVec<TorranceSparrow>,
}


struct Phong {
    _id: usize,
    exponent: Float,

}

struct ModifiedPhong {
    _id: usize,
    _normalized: bool,
    exponent: Float,
}

struct BlinnPhong {
    _id: usize,
    exponent: Float,
}


struct ModifiedBlinnPhong {
    _id: usize,
    exponent: Float,

}

struct TorranceSparrow {
    _id: usize,
    _kdfresnel: bool,
    exponent: Float,
}

    //pub fn ambient(&self) -> Vector3 {
    //    if self.degamma {
    //        self.ambient_rf.powf(2.2)
    //    } else {
    //        self.ambient_rf
    //    }   
    //}
//
    //pub fn diffuse(&self, w_i: Vector3, n: Vector3) -> Vector3 {
    //    // Returns outgoing radiance (see Slides 01_B, p.73)        
    //    debug_assert!(w_i.is_normalized());
    //    debug_assert!(n.is_normalized());
//
    //    let cos_theta = w_i.dot(n).max(0.0);
//
    //    let mut diffuse_rf = self.diffuse_rf;
    //    if self.degamma { diffuse_rf = diffuse_rf.powf(2.2); }
    //    diffuse_rf * cos_theta  
    //    
    //}
//
    //pub fn specular(&self, w_o: Vector3, w_i: Vector3, n: Vector3) -> Vector3 {
    //    // Returns outgoing radiance (see Slides 01_B, p.80)
    //    debug_assert!(w_o.is_normalized());
    //    debug_assert!(w_i.is_normalized());
    //    debug_assert!(n.is_normalized());
//
    //    let h = (w_i + w_o).normalize(); //(w_i + w_o) / (w_i + w_o).norm();
    //    debug_assert!(h.is_normalized());
    //    
    //    let p = self.phong_exponent;
    //    let cos_a = n.dot(h).max(0.0);
    //    
    //    
    //    let mut specular_rf = self.specular_rf;
    //    if self.degamma { specular_rf = specular_rf.powf(2.2); }
    //    
    //    specular_rf * cos_a.powf(p)  
    //}   
