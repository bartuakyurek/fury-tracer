

use crate::prelude::*;
use crate::json_structs::{SingleOrVec, HasId};
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


#[derive(Debug, Clone, Deserialize, SmartDefault)]
pub struct BRDFs {
    pub original_phong: SingleOrVec<Phong>,
    pub modified_phong: SingleOrVec<ModifiedPhong>,
    pub original_blinn_phong: SingleOrVec<BlinnPhong>,
    pub modified_blinn_phong: SingleOrVec<ModifiedBlinnPhong>,
    pub torrance_sparrow: SingleOrVec<TorranceSparrow>,
}

impl BRDFs {
   pub fn get(&self, id: usize) -> Option<&dyn BRDF> {
        fn find<'a, T>(
            items: &'a SingleOrVec<T>,
            id: usize,
        ) -> Option<&'a dyn BRDF>
        where
            T: BRDF + HasId + Clone, // Clone is the bound for SingleOrVec functions
        {
            for brdf in items.as_slice() {
                if brdf.id() == id {
                    return Some(brdf as &dyn BRDF);
                }
            }
            None
        }

        find(&self.original_phong, id)
            .or_else(|| find(&self.modified_phong, id))
            .or_else(|| find(&self.original_blinn_phong, id))
            .or_else(|| find(&self.modified_blinn_phong, id))
            .or_else(|| find(&self.torrance_sparrow, id))
    }
}


pub fn eval_brdf(
        brdf_id: Option<usize>,
        material_common: &MaterialCommon,
        scene_brdfs: &BRDFs,
        wi: Vector3,
        wo: Vector3,
        n: Vector3,
    ) -> Vector3 {
        
        // 1 - If brdf._id is given in JSON, use it 
        if let Some(brdf_ref) = brdf_id {
            let brdf = scene_brdfs.get(brdf_ref).unwrap();
            return brdf.eval(wi, wo, n, material_common);
        }

        // 2 - Otherwise use our Blinn–Phong shading as in previous homeworks
        blinn_phong_eval(
            wi,
            wo,
            n,
            material_common,
        )
}

fn blinn_phong_eval( 
        wi: Vector3,
        wo: Vector3,
        n: Vector3,
        material_common: &MaterialCommon,
) -> Vector3 {
    todo!("evaluate blinn phong as usual");
}


#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct Phong {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,
}

#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct ModifiedPhong {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(deserialize_with = "deser_bool")]
    _normalized: bool,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,
}

#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct BlinnPhong {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,
}


#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct ModifiedBlinnPhong {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,

}

#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct TorranceSparrow {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(deserialize_with = "deser_bool")]
    _kdfresnel: bool,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,
}

///////////////////////////////////////////////////////
/// BRDF Trait implementations for each concrete type 
///////////////////////////////////////////////////////

impl BRDF for Phong {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                params: &MaterialCommon,
        ) -> Vector3 {
        todo!()
    }
}

impl BRDF for ModifiedPhong {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                params: &MaterialCommon,
        ) -> Vector3 {
        todo!()
    }
}

impl BRDF for BlinnPhong {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                params: &MaterialCommon,
        ) -> Vector3 {
        todo!()
    }
}

impl BRDF for ModifiedBlinnPhong {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                params: &MaterialCommon,
        ) -> Vector3 {
        todo!()
    }
}

impl BRDF for TorranceSparrow {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                params: &MaterialCommon,
        ) -> Vector3 {
        todo!()
    }
}

/////////////////////////////////////////////////
/// HadId Trait implementations for BRDFs
/////////////////////////////////////////////////

impl HasId for Phong {
    fn id(&self) -> usize { self._id }
}

impl HasId for ModifiedPhong {
    fn id(&self) -> usize { self._id }
}

impl HasId for BlinnPhong {
    fn id(&self) -> usize { self._id }
}

impl HasId for ModifiedBlinnPhong {
    fn id(&self) -> usize { self._id }
}

impl HasId for TorranceSparrow {
    fn id(&self) -> usize { self._id }
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
