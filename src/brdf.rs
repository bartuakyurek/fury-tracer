

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
#[serde(default)] // Skip the missing fields
pub struct BRDFs {
    #[serde(rename = "OriginalPhong")]
    original_phong: SingleOrVec<Phong>,

    #[serde(rename = "ModifiedPhong")]
    modified_phong: SingleOrVec<ModifiedPhong>,

    #[serde(rename = "OriginalBlinnPhong")]
    original_blinn_phong: SingleOrVec<BlinnPhong>,
    
    #[serde(rename = "ModifiedBlinnPhong")]
    modified_blinn_phong: SingleOrVec<ModifiedBlinnPhong>,
    
    #[serde(rename = "TorranceSparrow")]
    torrance_sparrow: SingleOrVec<TorranceSparrow>,
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
                    debug!("Found BRDF with id {}", id);
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

//////////////////////////////////////////////////////////////////////////////////////////
// Static functions to be called in renderer
//////////////////////////////////////////////////////////////////////////////////////////
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
            material_common.phong_exponent,
            material_common.diffuse_rf,
            material_common.specular_rf,
        )
}

// Declaring it as function to be called by eval_brdf( ) if BRDF not specified, and re-used by original blinn phong
fn blinn_phong_eval( 
        wi: Vector3,
        wo: Vector3,
        n: Vector3,
        exponent: Float,
        kd: Vector3,
        ks: Vector3,
) -> Vector3 {
    
    assert!(wi.is_normalized());
    assert!(wo.is_normalized());
    assert!(n.is_normalized());

    let cos_theta = wi.dot(n);
    if cos_theta < 0. {
        return Vector3::ZERO;
    }

    let h = (wi + wo).normalize();
    let cos_a = n.dot(h).max(0.0);
    let specular_weight = cos_a.powf(exponent) / cos_theta;

    kd + (ks * specular_weight)
}
/////////////////////////////////////////////////////////////////////////////////////////////////

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
        
        blinn_phong_eval(wi, wo, n, self.exponent, params.diffuse_rf, params.specular_rf)
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
