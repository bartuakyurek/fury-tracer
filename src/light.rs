use bevy_math::{NormedVectorSpace};

use crate::ray::Ray;
use crate::interval::*;
use crate::json_structs::Transformations;
use crate::prelude::*;


pub enum LightKind {
    Point(PointLight),
    Area(AreaLight),
    Directional(DirectionalLight),
    Spot(SpotLight),
    Env(EnvironmentLight)
}


impl LightKind {

    pub fn get_shadow_direction_and_distance(&self, ray_origin: &Vector3) -> (Vector3, Float) {
        match self {
            LightKind::Point(pl) => {
                let distance_vec = pl.position - ray_origin;
                let distance = distance_vec.norm();
                (distance_vec / distance, distance)
            },
            LightKind::Area(al) => {
                let distance_vec = al.sample_position() - ray_origin;
                let distance = distance_vec.norm();
                (distance_vec / distance, distance)
            },
            LightKind::Directional(dl) => {
                // Direction was normalized at setup( ) already 
                debug_assert!(dl.direction.is_normalized());
                (-dl.direction, FloatConst::INF)
            },
            LightKind::Spot(sl) => {
                todo!()
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
                todo!()
            },
            LightKind::Env(envl) => {
                todo!()
            },
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct EnvironmentLight {
    // TODO
}

impl EnvironmentLight {
    pub fn setup(&mut self) {
        todo!()
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

    pub fn setup_onb(&mut self) {
        // See slides 05, p.96
        let (u, v) = get_onb(&self.normal);
        self.u = u;
        self.v = v;
        debug!("Area light ONB is setup successfully.");
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
