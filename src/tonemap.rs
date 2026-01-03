
use std::fmt;

use image::ImageBuffer;

use crate::{image::ImageData, prelude::*};


#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct ToneMap {

    #[serde(rename = "TMO")]
    operator: ToneMapOperator,

    #[serde(rename = "TMOOptions", deserialize_with = "deser_pair")]
    options: [Float; 2], 

    #[serde(rename = "Saturation", deserialize_with = "deser_float")]
    saturation: Float,

    #[serde(rename = "Gamma", deserialize_with = "deser_float")]
    gamma: Float,

    #[serde(rename = "Extension")]
    extension: String,
}

impl ToneMap {
    pub fn apply(&self, im: &ImageData) -> ImageData {

        let mut im = im.clone();
        im.update_extension(&self.extension);
        info!("Updated image extension. New image name: {}", im.name());

        let lumis = im.get_luminances();
        match self.operator {
            ToneMapOperator::ACES => {
                todo!()
            }
            ToneMapOperator::Filmic => {
                todo!()
            }
            ToneMapOperator::Photographic => {
                todo!()
            }
        }
    }
}

// Just for decorative purposes in info message
impl fmt::Display for ToneMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.operator {
            ToneMapOperator::ACES => "ACES",
            ToneMapOperator::Filmic => "Filmic",
            ToneMapOperator::Photographic => "Photographic",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "PascalCase")]
enum ToneMapOperator {
    ACES,
    Filmic,
    Photographic,
}

impl Default for ToneMapOperator {
    fn default() -> Self {
        ToneMapOperator::Photographic
    }
}
