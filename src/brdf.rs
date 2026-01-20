use crate::interval::FloatConst;
/// brdf.rs
/// 
/// Declare different kinds of BRDFs as given in HW6
/// they are implemented as BRDF trait, to ease the 
/// renderer access BRDF::eval( ) for material's associated
/// brdf.
/// 

use crate::prelude::*;
use crate::json_structs::{SingleOrVec, HasId};
use crate::material::{self, HeapAllocMaterial, ReflectanceParams};

pub trait BRDF {
    fn eval(
            &self,
            wi: Vector3,
            wo: Vector3,
            n: Vector3,
            material: &HeapAllocMaterial,
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
        mat: &HeapAllocMaterial, 
        scene_brdfs: &BRDFs,
        wi: Vector3,
        wo: Vector3,
        n: Vector3,
    ) -> Vector3 {
        
        // 1 - If brdf._id is given in JSON, use it 
        if let Some(brdf_ref) = brdf_id {
            let brdf = scene_brdfs.get(brdf_ref).unwrap();
            return brdf.eval(wi, wo, n, mat);
        }

        // 2 - Otherwise use our Blinn–Phong shading as in previous homeworks
        let material_common = mat.reflectance_data();
        blinn_phong_eval(
            wi,
            wo,
            n,
            material_common.phong_exponent,
            material_common.diffuse_rf,
            material_common.specular_rf,
            false,
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
        modified: bool,
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
    let mut specular_weight = cos_a.powf(exponent);
    if !modified {
     specular_weight /= cos_theta;        
    }

    kd + (ks * specular_weight)
}

fn torrance_sparrow_eval(
    wi: Vector3,
    wo: Vector3,
    n: Vector3,
    params: &ReflectanceParams,
    fresnel: (Float, Float),
    exponent: Float,
) -> Vector3 {

    let cos_theta = wi.dot(n);
    if cos_theta < 0. {
        return Vector3::ZERO;
    }

    // See brdf.pdf given alongside with hw6 (section 8)
    // 1 - Compute half vector
    let wh = (wi + wo).normalize();
    
    // 2 - Compute the angle
    let cos_a = n.dot(wh).max(0.0);

    // 3 - Compute Blinn distribution (D)
    let blinn_dist = ((exponent + 2.) / (2. * Float::PI)) * cos_a.powf(exponent);

    // 4- Compute the geometry term (G)
    let n_dot_wh = n.dot(wh); 
    let n_dot_wo = n.dot(wo);
    let n_dot_wi = cos_theta; // n.dot(wi);
    let wo_dot_wh = wo.dot(wh);

    let first_term = (2. * n_dot_wh * n_dot_wo) / wo_dot_wh;
    let second_term = (2. * n_dot_wh * n_dot_wi) / wo_dot_wh;
    let geometry_term = first_term.min(second_term).min(1.);

    // 5 - Compute the Fresnel reflectance using Schlick's approximation (F)
    let absorption_index = fresnel.0; // TODO: I thought this was necessary but seems like it is not, a refactor might be needed on Fresnel data
    let refractive_index = fresnel.1;

    let r_zero = (refractive_index - 1.).powf(2.) / (refractive_index + 1.).powf(2.);
    let cos_beta = wo_dot_wh;
    let fresnel_reflectance = r_zero + (1. - r_zero) * (1. - cos_beta).powf(5.);

    // 6 - Compute final BRDF (eqn. 10)
    let fresnel_stuff = blinn_dist * fresnel_reflectance * geometry_term;
    let cos_phi = n_dot_wo;
    let cosine_denominator = 4. * cos_theta * cos_phi;

    let f1 = params.diffuse_rf / Float::PI;
    let f2 = params.specular_rf * fresnel_stuff / cosine_denominator;
    
    f1 + f2
}


/////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct Phong {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,
}


impl BRDF for Phong {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                mat: &HeapAllocMaterial,
        ) -> Vector3 {
        let material_common = mat.reflectance_data();
        
        todo!()
    }
}

/////////////////////////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct ModifiedPhong {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(deserialize_with = "deser_bool")]
    _normalized: bool,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,
}


impl BRDF for ModifiedPhong {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                mat: &HeapAllocMaterial,
        ) -> Vector3 {
        
        let material_common = mat.reflectance_data();
        
        if self._normalized {
            todo!("Please implement normalization for Modified Blinn Phong")
        }

        
        todo!()
    }
}


////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct BlinnPhong {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,
}


impl BRDF for BlinnPhong {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                mat: &HeapAllocMaterial,
        ) -> Vector3 {
        let params = mat.reflectance_data();
        
        blinn_phong_eval(wi, wo, n, self.exponent, params.diffuse_rf, params.specular_rf, false)
    }
}


////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct ModifiedBlinnPhong {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(deserialize_with = "deser_bool")]
    _normalized: bool,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,
}


impl BRDF for ModifiedBlinnPhong {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                mat: &HeapAllocMaterial,
        ) -> Vector3 {
        
        let params = mat.reflectance_data();

        if self._normalized {
            todo!("Please implement normalization for Modified Blinn Phong")
        }
        blinn_phong_eval(wi, wo, n, self.exponent, params.diffuse_rf, params.specular_rf, true)
    }
}

////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize, SmartDefault)]
struct TorranceSparrow {
    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
    #[serde(deserialize_with = "deser_bool")]
    _kdfresnel: bool,
    #[serde(rename = "Exponent", deserialize_with = "deser_float")]
    exponent: Float,
}


impl BRDF for TorranceSparrow {
    fn eval(
                &self,
                wi: Vector3,
                wo: Vector3,
                n: Vector3,
                mat: &HeapAllocMaterial,
        ) -> Vector3 {
        
        let params = mat.reflectance_data();
        let fresnel = mat.get_fresnel_indices().unwrap();
        
        torrance_sparrow_eval(wi, wo, n, params, fresnel, self.exponent)
        
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
