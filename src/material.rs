/*
    material.rs 

    Declare Material trait, and store data related to
    different types of materials. Currently supporting:
        - Diffuse
        - Mirror
        - Conductor (TBI)
        - Dielectric (TBI)

    @date: Oct, 2025
    @author: Bartu

*/

use std::fmt::Debug;
use bevy_math::NormedVectorSpace;
use serde::{Deserialize, de::DeserializeOwned};

use crate::interval::FloatConst;
use crate::ray::{Ray, HitRecord}; 
use crate::prelude::*;
use crate::brdf;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ReflectanceParams {
    #[serde(rename = "AmbientReflectance", deserialize_with = "deser_vec3")]
    pub ambient_rf: Vector3,
    #[serde(rename = "DiffuseReflectance", deserialize_with = "deser_vec3")]
    pub diffuse_rf: Vector3,
    #[serde(rename = "SpecularReflectance", deserialize_with = "deser_vec3")]
    pub specular_rf: Vector3,
    #[serde(rename = "PhongExponent", deserialize_with = "deser_float")]
    pub phong_exponent: Float,

    #[serde(rename = "_degamma", deserialize_with = "deser_bool")]
    pub degamma: bool,
}

impl Default for ReflectanceParams {
    fn default() -> Self {
        debug!("Defaulting common reflectance params...");
        ReflectanceParams {
            ambient_rf: Vector3::new(0.0, 0.0, 0.0),
            diffuse_rf: Vector3::new(1.0, 1.0, 1.0),
            specular_rf: Vector3::new(0.0, 0.0, 0.0),
            phong_exponent: 1.0,
            degamma: false,
        }
    }
}

impl ReflectanceParams {

    pub fn apply_degamma(&mut self) {
        assert!(self.degamma, "Degamma flag found false but apply_degamma() is called.");
        self.ambient_rf = self.ambient_rf.powf(2.2);
        self.diffuse_rf = self.diffuse_rf.powf(2.2);
        self.specular_rf = self.specular_rf.powf(2.2);
    }

    pub fn ambient(&self) -> Vector3 {
        assert!(!self.degamma, "Found degamma = true, please call apply_degamma() and set degamma = False before calling ambient( ). Note: Material::setup( ) implementations should have handled it already.");
        self.ambient_rf
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// MATERIAL TRAIT
/// 
////////////////////////////////////////////////////////////////////////////////////////////////////////////////

// I thought this could be used as a trait bound but it wasnt compatible with dyn BRDF
//pub trait FresnelIndex: Debug {
//    fn absorption(&self) -> Float; 
//    fn refraction(&self) -> Float;
//}


pub trait Material : Debug + Send + Sync  {
    // TODO: could’ve implemet shade_diffuse(shadow_ray: &Ray, …) inside Material for cleaner logic ?
    fn setup(&mut self) { debug!("Empty setup called for a material, ignoring..."); }

    fn new_from(value: &serde_json::Value) -> Self 
    where
        Self: Sized + DeserializeOwned + Default,
    {
        match serde_json::from_value::<Self>(value.clone()) {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to parse Material: {e}. JSON: {value}\nSetting material to default...");
                Self::default()
            }
        }
    }

    fn get_type(&self) -> &str; 
    fn brdf(&self) -> Option<usize>; // Returns BRDF _id if specified
    fn reflectance_data(&self) -> &ReflectanceParams;
    fn get_fresnel_indices(&self) -> Option<(Float, Float)>; 
    
    fn interact(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, does_reflect: bool) -> Option<(Ray, Vector3)>; //(Ray, attenuation) --> Used for Ray Tracing requiring Shadow Rays, deterministically reflecting refracting rays if needed (e.g. in Mirror materials) and Diffuse material does not spawn a ray
    fn scatter(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, importance_sampling: bool) -> Option<(Ray, Vector3)>; // Intended to be used for Path Tracing, spawning rays randomly
    // TODO: Should we merge interact( ) and scatter( )?
}

pub type HeapAllocMaterial = Box<dyn Material>; // Box, Rc, Arc -> Probably will be Arc when we use rayon


////////////////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// DIFFUSE
/// 
////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DiffuseMaterial {
    #[serde(deserialize_with = "deser_usize")]
    pub _id: usize,
    
    #[serde(rename = "_BRDF", deserialize_with = "deser_opt_usize")]
    _brdf: Option<usize>,

    #[serde(flatten)]
    pub brdf_common: ReflectanceParams,

}


impl Default for DiffuseMaterial {
    fn default() -> Self {
        DiffuseMaterial {
            _id: 0,
            _brdf: None,
            brdf_common: ReflectanceParams {
                ambient_rf: Vector3::new(0.0, 0.0, 0.0),
                diffuse_rf: Vector3::new(1.0, 1.0, 1.0),
                specular_rf: Vector3::new(0.0, 0.0, 0.0),
                phong_exponent: 1.0,
                degamma: false,
                },
        }
    }
}

impl DiffuseMaterial {

    
}

impl Material for DiffuseMaterial{

    fn setup(&mut self) {
        debug!("Applying degamma for diffuse...");
        if self.brdf_common.degamma {
            self.brdf_common.apply_degamma();
            self.brdf_common.degamma = false;
        }
    }

    fn get_type(&self) -> &str {
        "diffuse"
    }

    fn reflectance_data(&self) -> &ReflectanceParams {
        &self.brdf_common
    }

    fn brdf(&self) -> Option<usize> {
        self._brdf
    }

    fn interact(&self, _: &Ray, _: &HitRecord, _: Float, _: bool) -> Option<(Ray, Vector3)> {
        warn!("Diffuse material assumed to only use shadow rays, rays are not meant to be scattered here.");
        None
    }

    fn get_fresnel_indices(&self) -> Option<(Float, Float)> {
        info!("Diffuse material returning fresnel indices as none... Consider adding type field to Material in JSON file if this behaviour is unexpected.");
        None
    }

    fn scatter(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, importance_sampling: bool) -> Option<(Ray, Vector3)> {
        
        //if importance_sampling {
        //    todo!()
        //}
        //else { // UNIFORM
        //    todo!()
        //}

        let n = hit_record.normal;
        let (u, v) = get_onb(&n);
        
        let psi1 = random_float();
        let psi2 = random_float();
        let cos_theta = psi1.sqrt();
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let phi: Float = 2.0 * Float::PI * psi2;
        
        let local_dir = Vector3::new(
            sin_theta * phi.cos(),
            sin_theta * phi.sin(),
            cos_theta,
        );
        
        let scattered_dir = local_dir.x * u + local_dir.y * v + local_dir.z * n;
        let ray_origin = hit_record.hit_point + (n * epsilon);
        let scattered_ray = Ray::new(ray_origin, scattered_dir.normalize(), ray_in.time);
        
        let pdf: Float = cos_theta / Float::PI;
        let attenuation = self.brdf_common.diffuse_rf / pdf;
        
        Some((scattered_ray, attenuation))
    }


}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// MIRROR
/// 
////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct MirrorMaterial {
    #[serde(deserialize_with = "deser_usize")]
    pub _id: usize,

    
    #[serde(rename = "_BRDF", deserialize_with = "deser_opt_usize")]
    _brdf: Option<usize>,

    #[serde(flatten)]
    pub brdf_common: ReflectanceParams,

    #[serde(rename = "MirrorReflectance", deserialize_with = "deser_vec3")]
    pub mirror_rf: Vector3,
    
    #[serde(rename = "Roughness", deserialize_with = "deser_float")]
    pub roughness: Float,

}

impl Default for MirrorMaterial {
    fn default() -> Self {
        Self {
            _id: 0,
            _brdf: None, // Default BRDF is BlinnPhong
            brdf_common: ReflectanceParams {
                    ambient_rf: Vector3::new(0.0, 0.0, 0.0),
                    diffuse_rf: Vector3::new(0.5, 0.5, 0.5),
                    specular_rf: Vector3::new(0.0, 0.0, 0.0),
                    phong_exponent: 1.0,
                    degamma: false,
                },
            mirror_rf: Vector3::new(1.0, 1.0, 1.0),
            roughness: 0.0, // Perfect mirror
        }
    }
}

impl MirrorMaterial {
    fn reflect(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float) -> Option<(Ray, Vector3)> {
        // Reflected ray from Slides 02, p.4 (Perfect Mirror)
        // wr ​= - wo ​+ 2 n (n . wo)
        // WARNING: Assume ray_in.direction = wi = - wo
        let n = hit_record.normal;
        let w_i = ray_in.direction;
        let w_r = w_i - 2. * n * (n.dot(w_i));
        debug_assert!(w_r.is_normalized());        

        // Glossy reflections (slides 05, p.108)
        let ray_origin = hit_record.hit_point + (n * epsilon);
        let r = w_r;
        let ray_dir = if self.roughness > 0.0 {
            let (u, v) = get_onb(&r);
            let (psi_1, psi_2) = (random_float(), random_float());
            let r_prime = r + self.roughness * (((psi_1 - 0.5) * u) + ((psi_2 - 0.5) * v));
            r_prime.normalize()
        } else {
            r
        };
        let ray = Ray::new(ray_origin, ray_dir, ray_in.time);

        let attenuation = self.mirror_rf;
        Some((ray, attenuation)) // Always reflects
    }
}

impl Material for MirrorMaterial {

    fn setup(&mut self) {
        if self.brdf_common.degamma {
            debug!("Applying degamma for mirror...");
            self.brdf_common.apply_degamma();
            self.brdf_common.degamma = false;
        }
    }
    
    fn brdf(&self) -> Option<usize> {
        self._brdf
    }

    fn get_type(&self) -> &str {
        "mirror"
    }

    fn reflectance_data(&self) -> &ReflectanceParams {
        &self.brdf_common
    }

    fn interact(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, does_reflect: bool) -> Option<(Ray, Vector3)> {
        if !does_reflect {
            warn!("Mirror material assumed to always reflect but interact( ) got does_reflect=False. Ignoring...");
        }
        self.reflect(ray_in, hit_record, epsilon)
    }

    fn get_fresnel_indices(&self) -> Option<(Float, Float)> {
        info!("Mirror material returning fresnel indices as none");
        None
    }

    fn scatter(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, _: bool) -> Option<(Ray, Vector3)> {
        // TODO: is it safe to ignore importance sampling here?
        self.reflect(ray_in, hit_record, epsilon)
    }
    
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// DIELECTRIC (GLASS)
/// 
////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug)]
struct FresnelData {
    cos_theta: Float,
    cos_phi: Float,
    f_r: Float,
    f_t: Float,
    n_ratio: Float,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DielectricMaterial {
    #[serde(deserialize_with = "deser_usize")]
    pub _id: usize,

    #[serde(rename = "_BRDF", deserialize_with = "deser_opt_usize")]
    _brdf: Option<usize>,
    
    #[serde(flatten)]
    pub brdf_common: ReflectanceParams,

    #[serde(rename = "MirrorReflectance", deserialize_with = "deser_vec3")]
    pub mirror_rf: Vector3,
   
    #[serde(rename = "AbsorptionCoefficient", deserialize_with = "deser_vec3")]
    pub absorption_coeff: Vector3,
    #[serde(rename = "RefractionIndex", deserialize_with = "deser_float")]
    pub refraction_index: Float,
    #[serde(rename = "Roughness", deserialize_with = "deser_float")]
    pub roughness: Float,
}

impl Default for DielectricMaterial {
    fn default() -> Self {
        Self {
            _id: 0,
            _brdf: None,
            brdf_common: ReflectanceParams{
                    ambient_rf: Vector3::new(0.0, 0.0, 0.0),
                    diffuse_rf: Vector3::new(0.5, 0.5, 0.5),
                    specular_rf: Vector3::new(0.0, 0.0, 0.0),
                    phong_exponent: 1.0,
                    degamma: false,
                },
            mirror_rf: Vector3::new(0.5, 0.5, 0.5),
            absorption_coeff: Vector3::new(0.01, 0.01, 0.01),
            refraction_index: 1.5,
            roughness: 0.0,
        }
    }
}

impl DielectricMaterial {

    fn setup(&mut self) {
        if self.brdf_common.degamma {
            debug!("Applying degamma for dielectric...");
            self.brdf_common.apply_degamma();
            self.brdf_common.degamma = false;
        }
    }

    fn get_beers_law_attenuation(&self, distance: Float) -> Vector3 {
        // Slides 02, p.27, only e^(-Cx) part
        // where C is the absorption coefficient
        // WARNING: ray_in.origin is assumed to be the location of the last hit point
        // i.e. point in p.28 with arrow to L(x)
        (- self.absorption_coeff * distance).exp() 
    }

    fn fresnel(&self, ray_in: &Ray, hit_record: &HitRecord, fresnel: &mut FresnelData) ->  bool {
        // returns reflection ratio F_r
        // (for transmissoion use 1 - F_r )
        // see slides 02, p.20 for notation
        // Update: now it should fill FresnelData
        // return false if total reflection occurs

        // d: incoming normalized ray
        // n: surface normal
        let d = ray_in.direction;
        let n = hit_record.normal;
        debug_assert!(d.is_normalized());
        debug_assert!(n.is_normalized());
        let cos_theta = n.dot(-d);
        
        // TODO: Would it be more flexible if we read it from FresnelData?
        let mut n1 = 1.0 as Float; // Assuming Vacuum in slides 02, p.22
        let mut n2 = self.refraction_index;
        if !hit_record.is_front_face {
            std::mem::swap(&mut n1, &mut n2);
        }
        
        let ratio_squared: Float = (n1 / n2).powi(2);
        let one_minus_cossqrd: Float = 1. - (cos_theta.powi(2));
        let inside_of_sqrt: Float = 1. - (ratio_squared * one_minus_cossqrd);

        let cos_phi: Float = if inside_of_sqrt < 0. {
            //info!("Total internal reflection occured!");
            fresnel.f_r = 1.0;
            return false; // TODO No need to compute, right? I assume it is total internal reflection (p.16)
        }
        else {
            inside_of_sqrt.sqrt() // TODO: do we need sqrt here or could we use sin^2 = 1 - cos^2?
        };

        let n1cos_p = n1 * cos_phi;
        let n2cos_p = n2 * cos_phi;
        let n1cos_t = n1 * cos_theta;
        let n2cos_t = n2 * cos_theta;

        let r_parallel: Float = (n2cos_t - n1cos_p) / (n2cos_t + n1cos_p);
        let r_perp: Float = (n1cos_t - n2cos_p) / (n1cos_t + n2cos_p);
        // TODO: in slides 02, p.20 this ratio has - in the denominator but that makes the ratio = 1
        // I assumed this is a typo and checked Fresnel from wikipedia... but I gotta ask for confirmation

        let f_r = 0.5 * (r_parallel.powi(2) + r_perp.powi(2));
        debug_assert!( (f_r > 1e-20) && (f_r < 1.+1e-20)); // in range [0,1]

        fresnel.n_ratio = n1 / n2;
        fresnel.cos_theta = cos_theta; 
        fresnel.cos_phi = cos_phi;
        fresnel.f_r = f_r;
        fresnel.f_t = 1. - f_r;
        true
    }

    fn reflect(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float) -> Option<(Ray, Vector3)> {
        
        let mut fresnel = FresnelData::default();
        self.fresnel(ray_in, hit_record, &mut fresnel);
        
        if fresnel.f_r > 1e-16 {
            let n = hit_record.normal;
            let w_i = ray_in.direction;
            let w_r = w_i - 2.0 * n * (n.dot(w_i));
            debug_assert!(w_r.is_normalized());

            // Glossy reflections (slides 05, p.108)
            let ray_origin = hit_record.hit_point + (n * epsilon);
            let r = w_r;
            let ray_dir = if self.roughness > 0.0 {
                let (u, v) = get_onb(&r);
                let (psi_1, psi_2) = (random_float(), random_float());
                let r_prime = r + self.roughness * (((psi_1 - 0.5) * u) + ((psi_2 - 0.5) * v));
                r_prime.normalize()
            } else {
                r
            };
            let ray = Ray::new(ray_origin, ray_dir, ray_in.time);

            let attenuation = fresnel.f_r * self.mirror_rf; 
            Some((ray, attenuation))
        } else {
            None
        }
    }

    fn refract(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float) -> Option<(Ray, Vector3)> {
        
        let mut frd = FresnelData::default();

        if !self.fresnel(ray_in, hit_record, &mut frd) {
        // Total internal reflection
            return None;
        }

        let d = ray_in.direction;
        let n = hit_record.normal;
        let mut refracted_direction = ((d + (n * frd.cos_theta)) * frd.n_ratio) - (n * frd.cos_phi); // p.15
        debug_assert!(refracted_direction.is_normalized());
        
         if self.roughness > 0.0 {
            let (u, v) = get_onb(&refracted_direction); // build tangent around refracted dir
            let (psi1, psi2) = (random_float(), random_float());
            let jitter = ((psi1 - 0.5) * u) + ((psi2 - 0.5) * v);
            refracted_direction = (refracted_direction + self.roughness * jitter).normalize();
        }
        
        let ray = Ray::new(hit_record.hit_point - n * epsilon, refracted_direction, ray_in.time); // Apply epsilon in negative normal direction!
        let mut attenuation = frd.f_t * Vector3::ONE;
        if !hit_record.is_front_face {
            // Attenuate as it goes out of object 
            // assumes glass object is empty
            let distance = (hit_record.entry_point - hit_record.hit_point).norm(); 
            //let distance2 = ray_in.distance_at(hit_record.ray_t);
            //debug_assert_eq!(distance, distance2);
            attenuation *= self.get_beers_law_attenuation(distance);
        } 
        
        Some((ray, attenuation))
    
    }
}


impl Material for DielectricMaterial {

    fn get_type(&self) -> &str {
        "dielectric"
    }
    
    fn brdf(&self) -> Option<usize> {
        self._brdf
    }

    fn reflectance_data(&self) -> &ReflectanceParams {
        &self.brdf_common
    }
    
    fn interact(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, does_reflect: bool) -> Option<(Ray, Vector3)> {
        if does_reflect {
            self.reflect(ray_in, hit_record, epsilon)
        } 
        else {
            self.refract(ray_in, hit_record, epsilon)
        }
    }
    
    fn get_fresnel_indices(&self) -> Option<(Float, Float)> {
        // TODO: I should remove absorption coeff 
        // (absorption coefficient is not used in the same way for dielectrics)
        Some((0.0, self.refraction_index))
    }

    fn scatter(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, _: bool) -> Option<(Ray, Vector3)> {
        
        let mut fresnel = FresnelData::default();
        if !self.fresnel(ray_in, hit_record, &mut fresnel) {
            // Total internal reflection
            return self.reflect(ray_in, hit_record, epsilon);
        }
        
        // TODO: is this correct way to handle?
        if random_float() < fresnel.f_r {
            self.reflect(ray_in, hit_record, epsilon)
        } else {
            self.refract(ray_in, hit_record, epsilon)
        }
    }
}


////////////////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// CONDUCTOR
/// 
////////////////////////////////////////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ConductorMaterial {
    #[serde(deserialize_with = "deser_usize")]
    pub _id: usize,

    #[serde(rename = "_BRDF", deserialize_with = "deser_opt_usize")]
    _brdf: Option<usize>,

    #[serde(flatten)]
    pub brdf_common: ReflectanceParams,
    
    #[serde(rename = "MirrorReflectance", deserialize_with = "deser_vec3")]
    pub mirror_rf: Vector3,
    #[serde(rename = "AbsorptionIndex", deserialize_with = "deser_float")]
    pub absorption_index: Float,
    #[serde(rename = "RefractionIndex", deserialize_with = "deser_float")]
    pub refraction_index: Float,
    #[serde(rename = "Roughness", deserialize_with = "deser_float")]
    pub roughness: Float,
}

impl Default for ConductorMaterial {
    fn default() -> Self {
        Self {
            _id: 0,
            _brdf: None,
            brdf_common: ReflectanceParams {
                    ambient_rf: Vector3::new(0., 0., 0.),
                    diffuse_rf: Vector3::new(0., 0., 0.),
                    specular_rf: Vector3::new(0., 0., 0.),
                    phong_exponent: 1., // TODO: Is that a good default? WARNING: cornellbox_recursive missing phong 
                    degamma: false,
                },
            mirror_rf: Vector3::new(1., 1., 1.),
            absorption_index: 2.82,
            refraction_index: 0.37,
            roughness: 0.0,
        }
    }
}


impl ConductorMaterial {

    fn fresnel(&self, ray_in: &Ray, hit_record: &HitRecord, fresnel: &mut FresnelData) {
        // Refer to slides 02, p.21 for notation
        // d: incoming normalized ray
        // n: surface normal
        let d = ray_in.direction;
        let n = hit_record.normal;
        debug_assert!(d.is_normalized());
        debug_assert!(n.is_normalized());
        let cos_theta = n.dot(-d);
        
        let n2 = self.refraction_index;
        let k2 = self.absorption_index; // TODO: Why this is named as _index but not _coefficient as in p.21?
        
        let sum_nk = n2.powi(2) + k2.powi(2); 
        let two_n_cos = 2. * n2 * cos_theta;
        let cos_squared = cos_theta.powi(2);
        let sum_nk_cos = sum_nk * cos_squared;

        let r_s = (sum_nk - two_n_cos + cos_squared) / (sum_nk + two_n_cos + cos_squared);
        let r_p = (sum_nk_cos - two_n_cos + 1.) / (sum_nk_cos + two_n_cos + 1.);

        fresnel.cos_theta = cos_theta; 
        fresnel.f_r =  0.5 * (r_s + r_p); // Reflection ratio
        fresnel.f_t = 0.;
    }

    fn reflect(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float) -> Option<(Ray, Vector3)> {
        // TODO: This should be the same reflection logic with dielectric, right? Only fresnel is different?
        // Also it seems like we don't need FresnelData at all for conductor, since we only need F_r?
        let mut fresnel = FresnelData::default();
        self.fresnel(ray_in, hit_record, &mut fresnel);
        
        if fresnel.f_r > 1e-6 {
            let n = hit_record.normal;
            let w_i = ray_in.direction;
            let w_r = w_i - 2.0 * n * (n.dot(w_i));
            debug_assert!(w_r.is_normalized());
            
            // Glossy reflections (slides 05, p.108)
            let ray_origin = hit_record.hit_point + (n * epsilon);
            let r = w_r;
            let ray_dir = if self.roughness > 0.0 {
                let (u, v) = get_onb(&r);
                let (psi_1, psi_2) = (random_float(), random_float());
                let r_prime = r + self.roughness * (((psi_1 - 0.5) * u) + ((psi_2 - 0.5) * v));
                r_prime.normalize()
            } else {
                r
            };
            let ray = Ray::new(ray_origin, ray_dir, ray_in.time);
            
            let attenuation = fresnel.f_r * self.mirror_rf; 
            Some((ray, attenuation))
        } else {
            info!("I expect this message never occurs...");
            None
        }
    }

    //fn refract(&self, _: &Ray, _: &HitRecord, _: Float) -> Option<(Ray, Vector3)> {
    //    None // F_t = 0 (see slides 02, p.21)
    //}
}

//impl FresnelIndex for ConductorMaterial {
//    fn absorption(&self) -> Float {
//        self.absorption_index
//    }
//    fn refraction(&self) -> Float {
//        self.refraction_index
//    }
//}

impl Material for ConductorMaterial {

    fn setup(&mut self) {
        if self.brdf_common.degamma {
            debug!("Applying degamma for conductor...");
            self.brdf_common.apply_degamma();
            self.brdf_common.degamma = false;
        }
    }

    fn get_type(&self) -> &str {
        "conductor"
    }

    fn brdf(&self) -> Option<usize> {
        self._brdf
    }

    fn reflectance_data(&self) -> &ReflectanceParams {
        &self.brdf_common
    }
    
    fn interact(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, does_reflect: bool) -> Option<(Ray, Vector3)> {
        if does_reflect {
            self.reflect(ray_in, hit_record, epsilon)
        } 
        else {
            None // No refraction for conductor
        }
        
    }    

    fn get_fresnel_indices(&self) -> Option<(Float, Float)> {
        Some((self.absorption_index, self.refraction_index))
    }

    fn scatter(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, _: bool) -> Option<(Ray, Vector3)> {
        // TODO: just like mirror here I assume
        self.reflect(ray_in, hit_record, epsilon)
    }
}
