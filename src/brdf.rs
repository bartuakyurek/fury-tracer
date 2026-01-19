

use crate::prelude::*;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct BRDFCommonData {
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

impl Default for BRDFCommonData {
    fn default() -> Self {
        debug!("Defaulting BRDF...");
        BRDFCommonData {
            ambient_rf: Vector3::new(0.0, 0.0, 0.0),
            diffuse_rf: Vector3::new(1.0, 1.0, 1.0),
            specular_rf: Vector3::new(0.0, 0.0, 0.0),
            phong_exponent: 1.0,
            degamma: false,
        }
    }
}

impl BRDFCommonData {

    pub fn ambient(&self) -> Vector3 {
        if self.degamma {
            self.ambient_rf.powf(2.2)
        } else {
            self.ambient_rf
        }   
    }

    pub fn diffuse(&self, w_i: Vector3, n: Vector3) -> Vector3 {
        // Returns outgoing radiance (see Slides 01_B, p.73)        
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let cos_theta = w_i.dot(n).max(0.0);

        let mut diffuse_rf = self.diffuse_rf;
        if self.degamma { diffuse_rf = diffuse_rf.powf(2.2); }
        diffuse_rf * cos_theta  
        
    }

    pub fn specular(&self, w_o: Vector3, w_i: Vector3, n: Vector3) -> Vector3 {
        // Returns outgoing radiance (see Slides 01_B, p.80)
        debug_assert!(w_o.is_normalized());
        debug_assert!(w_i.is_normalized());
        debug_assert!(n.is_normalized());

        let h = (w_i + w_o).normalize(); //(w_i + w_o) / (w_i + w_o).norm();
        debug_assert!(h.is_normalized());
        
        let p = self.phong_exponent;
        let cos_a = n.dot(h).max(0.0);
        
        
        let mut specular_rf = self.specular_rf;
        if self.degamma { specular_rf = specular_rf.powf(2.2); }
        
        specular_rf * cos_a.powf(p)  
    }   
}