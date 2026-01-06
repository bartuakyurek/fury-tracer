use bevy_math::{NormedVectorSpace};

use crate::ray::{Ray, HitRecord};
use crate::image::{Textures, Interpolation};
use crate::sampler::{hemisphere_cosine_sample, hemisphere_uniform_sample};
use crate::interval::*;
use crate::json_structs::Transformations;
use crate::prelude::*;


pub enum LightKind {
    Point(PointLight),
    Area(AreaLight),
    Directional(DirectionalLight),
    Spot(SpotLight),
    Env(SphericalDirectionalLight)
}

impl LightKind {

    pub fn setup(&mut self, transforms: &Transformations) {
        match self {
            LightKind::Point(pl) => pl.setup(transforms),
            LightKind::Area(al) => al.setup(),
            LightKind::Directional(dl) => dl.setup(),
            LightKind::Spot(sl) => sl.setup(),
            LightKind::Env(envl) => envl.setup(),
        }
    }
    // TODO: env light todos remain because they dont use shadow rays, so maybe I should rename LightKind to ShadowRayableLightKind 
    // to remove a potential future confusion. or think about another solution. anyway these todo!( ) are never triggered in hw5 scenes
    pub fn get_shadow_direction_and_distance(&self, ray_origin: &Vector3) -> (Vector3, Float) {

        fn regular(light_pos: Vector3, ray_origin: &Vector3) -> (Vector3, Float) {
            let distance_vec = light_pos - ray_origin;
            let distance = distance_vec.norm();
            (distance_vec / distance, distance)
        }

        match self {
            LightKind::Point(pl) => {
                regular(pl.position, ray_origin)
            },
            LightKind::Area(al) => {
                regular(al.sample_position(), ray_origin)
            },
            LightKind::Directional(dl) => {
                // Direction was normalized at setup( ) already 
                debug_assert!(dl.direction.is_normalized());
                (-dl.direction, FloatConst::INF)
            },
            LightKind::Spot(sl) => {
                regular(sl.position, ray_origin)
            },
            LightKind::Env(envl) => {
                todo!()
            },
        }
    }
    pub fn get_irradiance(&self, shadow_ray: &Ray, interval: &Interval) -> Vector3 {
        match self {
            LightKind::Point(pl) => {
                 pl.rgb_intensity / shadow_ray.squared_distance_at(interval.max)
            },
            LightKind::Area(al) => {
                al.radiance * al.attenuation(&shadow_ray.direction) / shadow_ray.squared_distance_at(interval.max)
            },
            LightKind::Directional(dl) => {
                dl.radiance
            },
            LightKind::Spot(sl) => {
                // Net irradiance E(x) given in hw5 pdf, eqn.4
                // See slides 09, p.11 for falloff range
                debug_assert!(sl.direction.is_normalized());
                debug_assert!(shadow_ray.direction.is_normalized());
                let light_dir = sl.direction.normalize(); // TODO: Why even though I normalize it at setup it isnt working?
                let cos_alpha = light_dir.dot(-shadow_ray.direction.normalize()); // I assume shadow ray is directed at light, so taking the negative
                assert!(cos_alpha <= 1.0 && cos_alpha >= -1.0); // Normalization asserts

                if cos_alpha <= 0.0 {
                     return Vector3::ZERO; // behind the light
                }

                let cos_f2 = ((sl.falloff_degrees / 180. * Float::PI) / 2.).cos(); // Converted degrees to radians for .cos( )
                let cos_c2 = ((sl.coverage_degrees / 180. * Float::PI) / 2.).cos(); 
                let cos_diff = cos_f2 - cos_c2;
                
                if cos_alpha <= cos_c2 {
                   return Vector3::ZERO; // outside of coverage range
                }           

                let dist_squared = (shadow_ray.origin - sl.position).norm_squared();
                let mut irrad = sl.intensity / dist_squared; 
            
                if cos_alpha < cos_f2 { 
                    let s = ((cos_alpha - cos_c2) / cos_diff).powf(4.);
                    irrad *= s;
                }
                
                irrad
            },
            LightKind::Env(envl) => {
                todo!("Environment lights should be already handled inside renderer, we should refactor LightKind...")
            },
        }
    }
}



#[derive(Debug, Deserialize, Clone, Copy)]
enum EnvironmentMap {
    #[serde(rename="latlong")]
    LatLong,

    #[serde(rename="probe")]
    Spherical, 
}

impl Default for EnvironmentMap {
    fn default() -> Self {
        EnvironmentMap::LatLong
    }
}

impl EnvironmentMap {
    pub fn get_uv(&self, d: Vector3) -> [Float; 2] {
        // See HW5 pdf, eqns 5-10
        // d is the sampled direction

        match self {
            EnvironmentMap::LatLong => {
                let u = ( 1. + (d.x.atan2(-d.z) / Float::PI) ) / 2.;
                let v = d.y.clamp(-1., 1.).acos() / Float::PI; // Clamp to prevent acos returning Nan
                [u, v]
            }
            EnvironmentMap::Spherical => {
                let a = (-d.z).clamp(-1., 1.).acos();
                let b = (d.x.powf(2.) + d.y.powf(2.)).sqrt();
                let r = (1. / Float::PI) * (a / b);

                let u = ( (r * d.x) + 1. ) / 2.;
                let v = ( -(r * d.y) + 1. ) / 2.;

                [u, v]
            }
        }
        
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SphericalDirectionalLight {
    #[serde(rename = "_id", deserialize_with = "deser_usize")]
    _id: usize,
    
    #[serde(rename = "_type")]
    _type: EnvironmentMap,

    #[serde(rename = "ImageId", deserialize_with = "deser_usize")]
    image_id: usize,

    #[serde(rename = "Sampler", default)]
    sampler: String,

    #[serde(skip)]
    image_idx: usize,
}

impl SphericalDirectionalLight {
    pub fn setup(&mut self) {
        warn!("Assuming image_id starts from 1 and images given sorted by id");
        self.image_idx = self.image_id - 1; 
    }

    pub fn image_idx(&self) -> usize {
        self.image_idx
    }

    pub fn get_uv(&self, dir: Vector3) -> [Float; 2] {
        self._type.get_uv(dir)
    }

    pub fn sample_and_get_radiance(&self, hit_record: &HitRecord,textures: &Textures) 
    -> (Vector3, Vector3) {
        // Build ONB from surface normal 
        let (u, v) = get_onb(&hit_record.normal);
        let n = hit_record.normal;
        
        // Sample direction
        let sampled_dir = match self.sampler.to_ascii_lowercase().as_str() {
            "uniform" => {
                hemisphere_uniform_sample(&u, &v, &n)
            }
            "cosine" => {
                hemisphere_cosine_sample(&u, &v, &n)
            }
            _ => {
                hemisphere_cosine_sample(&u, &v, &n)
            }
        };
        
        let uv = self.get_uv(sampled_dir);        
        let mut radiance = textures.tex_from_img(self.image_idx(), uv, &Interpolation::Bilinear);
        
        match self.sampler.to_ascii_lowercase().as_str() {
            "cosine" => {
                let cos_theta = sampled_dir.dot(n);
                radiance *= Float::PI / cos_theta;
            } 
            _ => {
                radiance *= 2. * Float::PI
            }
        }

        (sampled_dir, radiance)
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SpotLight {
    #[serde(rename = "_id", deserialize_with = "deser_usize")]
    pub _id: usize,

    #[serde(rename = "Position", deserialize_with = "deser_vec3")]
    pub position: Vector3,

    #[serde(rename = "Direction", deserialize_with = "deser_vec3")]
    pub direction: Vector3,

    #[serde(rename = "Intensity", deserialize_with = "deser_vec3")]
    pub intensity: Vector3,

    #[serde(rename = "CoverageAngle", deserialize_with = "deser_float")]
    pub coverage_degrees: Float,

    #[serde(rename = "FalloffAngle", deserialize_with = "deser_float")]
    pub falloff_degrees: Float,

}

impl SpotLight {
    pub fn setup(&mut self) {
        debug!("Normalizing direction for spot light id:{}...", self._id);
        self.direction = self.direction.normalize();
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DirectionalLight {
    #[serde(rename = "_id", deserialize_with = "deser_usize")]
    pub _id: usize,

    #[serde(rename = "Direction", deserialize_with = "deser_vec3")]
    pub direction: Vector3,

    #[serde(rename = "Radiance", deserialize_with = "deser_vec3")]
    pub radiance: Vector3,
}

impl DirectionalLight {
    pub fn setup(&mut self) {
        debug!("Normalizing direction for directional light id:{}...", self._id);
        self.direction = self.direction.normalize();
    }
}


#[derive(Debug, Deserialize, Clone, Default)]
pub struct AreaLight {
    #[serde(rename = "_id", deserialize_with = "deser_int")]
    pub _id: Int, 

    #[serde(rename = "Position", deserialize_with = "deser_vec3")]
    pub position: Vector3,

    #[serde(rename = "Normal", deserialize_with = "deser_vec3")]
    pub normal: Vector3,

    #[serde(rename = "Size", deserialize_with = "deser_int")]
    pub size: Int, // Assume square area light

    #[serde(rename = "Radiance", deserialize_with = "deser_vec3")]
    pub radiance: Vector3, 

    #[serde(skip)] // For ONB, see slides 05, p.96
    u: Vector3,
    #[serde(skip)]
    v: Vector3,
}

//pub struct AreaLightBase {
//    u: Vector3,
//    v: Vector3,
//}

impl AreaLight {

    pub fn setup(&mut self) {
        debug!("WARNING: Assumes area lights have no transformation to setup!");
        self.setup_onb();
    }

     pub fn setup_onb(&mut self) {
        // See slides 05, p.96
        let (u, v) = get_onb(&self.normal);
        self.u = u;
        self.v = v;
        debug!("Area light ONB is setup successfully.");
    }

    pub fn sample_position(&self) -> Vector3 {
        // see slides 05, p.97
        // extent is arealight.size here
        // TODO: how to use rng with seed here? and precompute jittered sampling etc.?
        if self.u.eq(&Vector3::ZERO) {
            warn!("Found area light u as a zero vector! Make sure you called arealight.setup_onb() before sampling!");
        }
        if self.v.eq(&Vector3::ZERO) {
            warn!("Found area light v as a zero vector! Make sure you called arealight.setup_onb() before sampling!");
        }
        debug_assert!(approx_zero(self.u.dot(self.v)), "ONB failed! u dot v found nonzero: {}", self.u.dot(self.v));

        let (psi_1, psi_2) = (random_float(), random_float());
        let extent = self.size as Float;
        self.position + (extent * ((psi_1 - 0.5) * self.u) + ((psi_2 - 0.5) * self.v))
    }

    pub fn attenuation(&self, shadow_ray_dir: &Vector3) -> Float {
        // See slides 05, p.98
        debug_assert!(shadow_ray_dir.is_normalized());
        let area = (self.size * self.size) as Float;
        let emitted_light_dir = -shadow_ray_dir;

        let cos_alpha = emitted_light_dir.dot(self.normal);
        //if cos_alpha <= 0.0 {
        //    return 0.0; 
        //}

        area * cos_alpha.abs() // abs for double sided area light
    }

   
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PointLight {
    #[serde(rename = "_id", deserialize_with = "deser_int")]
    pub _id: Int, 

    #[serde(rename = "Position", deserialize_with = "deser_vec3")]
    pub position: Vector3,

    #[serde(rename = "Intensity", deserialize_with = "deser_vec3")]
    pub rgb_intensity: Vector3,

     #[serde(rename = "Transformations")]
    pub(crate) transformation_names: Option<String>,

    #[serde(skip)]
    pub(crate) composite_mat: Matrix4,
}

impl PointLight {

    pub fn setup(&mut self, transforms: &Transformations) {
        self.composite_mat = 
        if self.transformation_names.is_some() 
        {
            parse_transform_expression(
                self.transformation_names.as_deref().unwrap_or(""),
                transforms,  
            )
        } else {
            debug!("No transformation matrix found for point light '{}', defaulting to Identity...", self._id);
            Matrix4::IDENTITY
        };

        self.position = transform_point(&self.composite_mat, &self.position);
    }
}
