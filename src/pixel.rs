



use image::{Rgba};
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct PixelData {
    pub color: Option<Vector3>, // None = transparent
    pub is_emissive: bool,  // e.g. cyan color set as _indicator in json file will make the pixel emissive
}

const EPSILON: Float = 1e-6;
const TRANSPARENT_THRESHOLD: Float = 10.;

impl PixelData {


    pub fn from_rgba(rgba: Rgba<u8>, emission_indicator: Vector3, emission_color: Vector3) -> Self {
        let r = rgba[0] as Float;
        let g = rgba[1] as Float;
        let b = rgba[2] as Float;
        let a = rgba[3] as Float;

        if a < TRANSPARENT_THRESHOLD { // If alpha channel is very small, treat it as transparent
            return PixelData {
                color: None,
                is_emissive: false,
            };
        }

        let pixel_color = Vector3::new(r, g, b);
        let is_emissive = (pixel_color - emission_indicator).length() < EPSILON;

        let color = if is_emissive {
            Some(emission_color)
        } else {
            Some(pixel_color)
        };

        PixelData {
            color,
            is_emissive,
        }
    }

}

