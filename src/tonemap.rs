
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

        // 1 - Compute luminance from RGB 
        let lumi: Vec<Float> = im.get_luminances();

        // 2 (and 3) - Apply tone mapping algorithm
        let compressed_lumi= self.operator.compress_luminance(&lumi, &self.options);

        // 4 - Apply eqn.1 in HW5 pdf (RGB colors from luminances)
        let num_pixels = im.num_pixels();
        for i in 0..num_pixels {
            let y_o = compressed_lumi[i];
            let y_i = lumi[i];
            let new_color = y_o * (im.colors[i] / y_i).powf(self.saturation);
            im.colors[i] = new_color;
        }

        // 5 - Apply eqn.2 in HW5 pdf (Gamma correction) 
        for i in 0..num_pixels {
            im.colors[i] = 255. * im.colors[i].powf(1. / self.gamma);
        }
        im
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


impl ToneMapOperator {
    pub fn compress_luminance(&self, lumi: &Vec<Float>, options: &[Float; 2]) -> Vec<Float> {
        
        let (alpha, percentile) = (options[0], options[1]);

        match self {
            ToneMapOperator::ACES => {
                self.aces_tmo(lumi, alpha, percentile)
            }
            ToneMapOperator::Filmic => {
                self.filmic_tmo(lumi, alpha, percentile)
            }
            ToneMapOperator::Photographic => {
                self.photographic_tmo(lumi, alpha, percentile)
            }
        }
    }

    fn photographic_tmo(&self, lumi: &Vec<Float>, alpha: Float, percentile: Float) -> Vec<Float> {
        // See slides 08, p.42 Photographic TMO consists of two stages
        
        // Stage one - a global operator simulating key mapping
        // initial luminance mapping (slides 08, p.43)
        let n = lumi.len() as Float;
        let eps = 1e-10;
        let middle_gray = ((1. / n) * lumi.iter().map(|lw| (lw + eps).ln()).sum::<Float>()).exp();
        let mut comp_lumi: Vec<Float> = lumi.iter().map(|lw| (alpha / middle_gray) * lw).collect();
        comp_lumi.iter_mut().for_each(|lumi| *lumi = *lumi / (1. + *lumi));

        // Stage two - a local operator simulating dodging-and-burning
        if percentile > 0.0 {
            let mut sorted_lumi = comp_lumi.clone();
            sorted_lumi.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let idx = ((percentile / 100.) * sorted_lumi.len() as Float) as usize;
            assert!(idx <= sorted_lumi.len() - 1);

            let l_white_squared = sorted_lumi[idx].powf(2.);
            comp_lumi.iter_mut().for_each(|lumi| {
                *lumi = (*lumi * (1. + (*lumi / l_white_squared))) / (1. + *lumi)
            });
        }

        comp_lumi
        
    }

    fn aces_tmo(&self, lumi: &[Float], alpha: Float, percentile: Float) -> Vec<Float> {
        todo!()
    }

    fn filmic_tmo(&self, lumi: &[Float], alpha: Float, percentile: Float) -> Vec<Float> {
        todo!()
    }

}