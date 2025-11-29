/*

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

use crate::ray::{Ray, HitRecord}; 
use crate::prelude::*;


////////////////////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// MATERIAL TRAIT
/// 
////////////////////////////////////////////////////////////////////////////////////////////////////////////////
pub trait Material : Debug + Send + Sync  {
    // TODO: could’ve implemet shade_diffuse(shadow_ray: &Ray, …) inside Material for cleaner logic ?
    fn new_from(value: &serde_json::Value) -> Self 
    where
        Self: Sized + DeserializeOwned + Default,
    {
        match serde_json::from_value::<Self>(value.clone()) {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to parse DiffuseMaterial: {e}. JSON: {value}");
                Self::default()
            }
        }
    }
    fn get_type(&self) -> &str;
    fn diffuse(&self, w_i: Vector3, n: Vector3) -> Vector3;
    fn specular(&self, w_o: Vector3, w_i: Vector3, n: Vector3) -> Vector3;
    fn ambient(&self) -> Vector3; 

    
    fn interact(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, does_reflect: bool) -> Option<(Ray, Vector3)>; //(Ray, attenuation)
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
    #[serde(rename = "AmbientReflectance", deserialize_with = "deser_vec3")]
    pub ambient_rf: Vector3,
    #[serde(rename = "DiffuseReflectance", deserialize_with = "deser_vec3")]
    pub diffuse_rf: Vector3,
    #[serde(rename = "SpecularReflectance", deserialize_with = "deser_vec3")]
    pub specular_rf: Vector3,
    #[serde(rename = "PhongExponent", deserialize_with = "deser_float")]
    pub phong_exponent: Float,
}


impl Default for DiffuseMaterial {
    fn default() -> Self {
        DiffuseMaterial {
            _id: 0,
            ambient_rf: Vector3::new(0.0, 0.0, 0.0),
            diffuse_rf: Vector3::new(1.0, 1.0, 1.0),
            specular_rf: Vector3::new(0.0, 0.0, 0.0),
            phong_exponent: 1.0,
        }
    }
}

impl DiffuseMaterial {

    
}

impl Material for DiffuseMaterial{


    fn get_type(&self) -> &str {
        "diffuse"
    }

    fn interact(&self, _: &Ray, _: &HitRecord, _: Float, _: bool) -> Option<(Ray, Vector3)> {
        warn!("Diffuse material assumed to only use shadow rays, rays are not meant to be scattered here.");
        None
    }

    fn ambient(&self) -> Vector3 {
        // Returns outgoing radiance (see Slides 01_B, p.75)
        // e.g. for test.json it is [25, 25, 25]
        self.ambient_rf 
    }

    fn diffuse(&self, w_i: Vector3, n: Vector3) -> Vector3 {
        // Returns outgoing radiance (see Slides 01_B, p.73)        
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let cos_theta = w_i.dot(n).max(0.0);
        self.diffuse_rf * cos_theta 
    }

    fn specular(&self, w_o: Vector3, w_i: Vector3, n: Vector3) -> Vector3 {
        // Returns outgoing radiance (see Slides 01_B, p.80)
        debug_assert!(w_o.is_normalized());
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let h = (w_i + w_o).normalize(); //(w_i + w_o) / (w_i + w_o).norm();
        debug_assert!(h.is_normalized());
        
        let p = self.phong_exponent;
        let cos_a = n.dot(h).max(0.0);
        self.specular_rf * cos_a.powf(p)
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
    #[serde(rename = "AmbientReflectance", deserialize_with = "deser_vec3")]
    pub ambient_rf: Vector3,
    #[serde(rename = "DiffuseReflectance", deserialize_with = "deser_vec3")]
    pub diffuse_rf: Vector3,
    #[serde(rename = "SpecularReflectance", deserialize_with = "deser_vec3")]
    pub specular_rf: Vector3,
    #[serde(rename = "MirrorReflectance", deserialize_with = "deser_vec3")]
    pub mirror_rf: Vector3,
    #[serde(rename = "PhongExponent", deserialize_with = "deser_float")]
    pub phong_exponent: Float,
    #[serde(rename = "Roughness", deserialize_with = "deser_float")]
    pub roughness: Float,
}

impl Default for MirrorMaterial {
    fn default() -> Self {
        Self {
            _id: 0,
            ambient_rf: Vector3::new(0.0, 0.0, 0.0),
            diffuse_rf: Vector3::new(0.5, 0.5, 0.5),
            specular_rf: Vector3::new(0.0, 0.0, 0.0),
            mirror_rf: Vector3::new(0.5, 0.5, 0.5),
            phong_exponent: 1.0,
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
        
        let ray = Ray::new(hit_record.hit_point + (n * epsilon), w_r);
        let attenuation = self.mirror_rf;
        Some((ray, attenuation)) // Always reflects
    }

}

impl Material for MirrorMaterial {

    fn get_type(&self) -> &str {
        "mirror"
    }

    
    fn interact(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, does_reflect: bool) -> Option<(Ray, Vector3)> {
        if !does_reflect {
            warn!("Mirror material assumed to always reflect but interact( ) got does_reflect=False. Ignoring...");
        }
        self.reflect(ray_in, hit_record, epsilon)
    }

    
    fn ambient(&self) -> Vector3 {
        self.ambient_rf  
    }

    fn diffuse(&self, w_i: Vector3, n: Vector3) -> Vector3 {
        // Returns outgoing radiance (see Slides 01_B, p.73)
        // TODO: reduce the verbosity here
        
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let cos_theta = w_i.dot(n).max(0.0);
        self.diffuse_rf * cos_theta  
    }

    fn specular(&self, w_o: Vector3, w_i: Vector3, n: Vector3) -> Vector3 {
        // Returns outgoing radiance (see Slides 01_B, p.80)
        debug_assert!(w_o.is_normalized());
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let h = (w_i + w_o).normalize(); //(w_i + w_o) / (w_i + w_o).norm();
        debug_assert!(h.is_normalized());
        
        let p = self.phong_exponent;
        let cos_a = n.dot(h).max(0.0);
        self.specular_rf * cos_a.powf(p)  
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
    #[serde(rename = "AmbientReflectance", deserialize_with = "deser_vec3")]
    pub ambient_rf: Vector3,
    #[serde(rename = "DiffuseReflectance", deserialize_with = "deser_vec3")]
    pub diffuse_rf: Vector3,
    #[serde(rename = "SpecularReflectance", deserialize_with = "deser_vec3")]
    pub specular_rf: Vector3,
    #[serde(rename = "MirrorReflectance", deserialize_with = "deser_vec3")]
    pub mirror_rf: Vector3,
    #[serde(rename = "PhongExponent", deserialize_with = "deser_float")]
    pub phong_exponent: Float,
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
            ambient_rf: Vector3::new(0.0, 0.0, 0.0),
            diffuse_rf: Vector3::new(0.5, 0.5, 0.5),
            specular_rf: Vector3::new(0.0, 0.0, 0.0),
            mirror_rf: Vector3::new(0.5, 0.5, 0.5),
            phong_exponent: 1.0,
            absorption_coeff: Vector3::new(0.01, 0.01, 0.01),
            refraction_index: 1.5,
            roughness: 0.0,
        }
    }
}

impl DielectricMaterial {

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
        
        if fresnel.f_r > 1e-6 {
            let n = hit_record.normal;
            let w_i = ray_in.direction;
            let w_r = w_i - 2.0 * n * (n.dot(w_i));
            debug_assert!(w_r.is_normalized());
            
            let ray = Ray::new(hit_record.hit_point + (n * epsilon), w_r);
            let attenuation = fresnel.f_r * self.mirror_rf; // TODO: Am I doing it right?? scalar times a vector, is that really the attenuation from glass reflectance?
            Some((ray, attenuation))
        } else {
            None
        }
    }

    fn refract(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float) -> Option<(Ray, Vector3)> {
        
        let mut frd = FresnelData::default();
        if self.fresnel(ray_in, hit_record, &mut frd) {
            
            let d = ray_in.direction;
            let n = hit_record.normal;
            let refracted_direction = ((d + (n * frd.cos_theta)) * frd.n_ratio) - (n * frd.cos_phi); // p.15
            debug_assert!(refracted_direction.is_normalized());

            let ray = Ray::new(hit_record.hit_point - n * epsilon, refracted_direction); // Apply epsilon in negative normal direction!
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
        else {
            None // Total internal reflection
        }
       
        
    }
}


impl Material for DielectricMaterial {

    fn get_type(&self) -> &str {
        "dielectric"
    }
    
    fn interact(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, does_reflect: bool) -> Option<(Ray, Vector3)> {
        if does_reflect {
            self.reflect(ray_in, hit_record, epsilon)
        } 
        else {
            self.refract(ray_in, hit_record, epsilon)
        }
    }
    
    fn ambient(&self) -> Vector3 {
        self.ambient_rf  
    }

    fn diffuse(&self, w_i: Vector3, n: Vector3) -> Vector3 {
        // TODO: these are copy paste from Diffuse material,
        // should we refactor them into a single function within
        // this crate?
        // Actually a better implementation would be to create a struct
        // for diffuse, specular, ambient, and phong as these four are common
        // and then just store them in material, that way you can move diffuse
        // and other common functions inside Material trait! I believe this is 
        // a Rusty way to implement it but before that I better decouple json
        // parser from these material structs...
        
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let cos_theta = w_i.dot(n).max(0.0);
        self.diffuse_rf * cos_theta  
    }

    fn specular(&self, w_o: Vector3, w_i: Vector3, n: Vector3) -> Vector3 {
        // Returns outgoing radiance (see Slides 01_B, p.80)
        debug_assert!(w_o.is_normalized());
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let h = (w_i + w_o).normalize(); //(w_i + w_o) / (w_i + w_o).norm();
        debug_assert!(h.is_normalized());
        
        let p = self.phong_exponent;
        let cos_a = n.dot(h).max(0.0);
        self.specular_rf * cos_a.powf(p)  
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
    #[serde(rename = "AmbientReflectance", deserialize_with = "deser_vec3")]
    pub ambient_rf: Vector3,
    #[serde(rename = "DiffuseReflectance", deserialize_with = "deser_vec3")]
    pub diffuse_rf: Vector3,
    #[serde(rename = "SpecularReflectance", deserialize_with = "deser_vec3")]
    pub specular_rf: Vector3,
    #[serde(rename = "MirrorReflectance", deserialize_with = "deser_vec3")]
    pub mirror_rf: Vector3,
    #[serde(rename = "PhongExponent", deserialize_with = "deser_float")]
    pub phong_exponent: Float,
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
            ambient_rf: Vector3::new(0., 0., 0.),
            diffuse_rf: Vector3::new(0., 0., 0.),
            specular_rf: Vector3::new(0., 0., 0.),
            mirror_rf: Vector3::new(1., 1., 1.),
            phong_exponent: 1., // TODO: Is that a good default? WARNING: cornellbox_recursive missing phong 
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
            
            let ray = Ray::new(hit_record.hit_point + (n * epsilon), w_r);
            let attenuation = fresnel.f_r * self.mirror_rf; // TODO: Am I doing it right?? scalar times a vector, is that really the attenuation from glass reflectance?
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

impl Material for ConductorMaterial {

    fn get_type(&self) -> &str {
        "conductor"
    }
    
    fn interact(&self, ray_in: &Ray, hit_record: &HitRecord, epsilon: Float, does_reflect: bool) -> Option<(Ray, Vector3)> {
        if does_reflect {
            self.reflect(ray_in, hit_record, epsilon)
        } 
        else {
            None // No refraction for conductor
        }
        
    }    

    fn ambient(&self) -> Vector3 {
        self.ambient_rf  
    }

    fn diffuse(&self, w_i: Vector3, n: Vector3) -> Vector3 {
        // TODO: these are copy paste from Diffuse material,
        // should we refactor them into a single function within
        // this crate?
        // Actually a better implementation would be to create a struct
        // for diffuse, specular, ambient, and phong as these four are common
        // and then just store them in material, that way you can move diffuse
        // and other common functions inside Material trait! I believe this is 
        // a Rusty way to implement it but before that I better decouple json
        // parser from these material structs...
        
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let cos_theta = w_i.dot(n).max(0.0);
        self.diffuse_rf * cos_theta  
    }

    fn specular(&self, w_o: Vector3, w_i: Vector3, n: Vector3) -> Vector3 {
        // Returns outgoing radiance (see Slides 01_B, p.80)
        debug_assert!(w_o.is_normalized());
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let h = (w_i + w_o).normalize(); //(w_i + w_o) / (w_i + w_o).norm();
        debug_assert!(h.is_normalized());
        
        let p = self.phong_exponent;
        let cos_a = n.dot(h).max(0.0);
        self.specular_rf * cos_a.powf(p)  
    }   
}
