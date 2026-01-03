
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
        let world_lumi: Vec<Float> = im.get_luminances();

        // 2 (and 3) - Apply tone mapping algorithm
        let display_lumi= self.operator.luminance(&world_lumi, &self.options);

        // 4 - Apply eqn.1 in HW5 pdf (RGB colors from luminances)
        let num_pixels = im.num_pixels();
        for i in 0..num_pixels {
            let new_color = display_lumi[i] * (im.colors[i] / world_lumi[i]).powf(self.saturation);
            im.colors[i] = new_color;
        }

        // 5 - Apply eqn.2 in HW5 pdf (Gamma correction) 
        for i in 0..num_pixels {
            // TODO: What's the idiomatic way to compute bleow?
            im.colors[i].x = 255. * (im.colors[i].x.powf(1. / self.gamma)).clamp(0.0, 1.0);
            im.colors[i].y = 255. * (im.colors[i].y.powf(1. / self.gamma)).clamp(0.0, 1.0);
            im.colors[i].z = 255. * (im.colors[i].z.powf(1. / self.gamma)).clamp(0.0, 1.0);
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
    pub fn luminance(&self, lumi: &Vec<Float>, options: &[Float; 2]) -> Vec<Float> {
        
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

    fn get_white(&self, lumi: &Vec<Float>, percentile: Float) -> Float {
        let mut sorted_lumi = lumi.clone();
        sorted_lumi.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let portion = (100. - percentile) / 100.;
        let idx = (portion * sorted_lumi.len() as Float) as usize;
        let idx = idx.min(sorted_lumi.len() - 1);
        sorted_lumi[idx]
    }

    fn prescale(&self, lumi: &Vec<Float>, alpha: Float) -> Vec<Float> {
        let n = lumi.len() as Float;
        let eps = 1e-10;
        let middle_gray = ((1. / n) * lumi.iter().map(|lw| (lw + eps).ln()).sum::<Float>()).exp();
        lumi.iter().map(|lw| (alpha / middle_gray) * lw).collect()
    }

    fn photographic_tmo(&self, lumi: &Vec<Float>, alpha: Float, percentile: Float) -> Vec<Float> {
        // See slides 08, p.42 Photographic TMO consists of two stages
        
        // Stage one - a global operator simulating key mapping
        // initial luminance mapping (slides 08, p.43)
        let mut comp_lumi: Vec<Float> = self.prescale(lumi, alpha);
        
        // Stage two - a local operator simulating dodging-and-burning
        if percentile > 0.0 {
            let l_white = self.get_white(lumi, percentile);
            let l_white_squared = l_white.powf(2.);
            comp_lumi.iter_mut().for_each(|lumi| {
                *lumi = (*lumi * (1. + (*lumi / l_white_squared))) / (1. + *lumi)
            });
        } else {
            comp_lumi.iter_mut().for_each(|lumi| *lumi = *lumi / (1. + *lumi));
        }

        comp_lumi
        
    }

    fn aces_tmo(&self, lumi: &Vec<Float>, alpha: Float, percentile: Float) -> Vec<Float> {

        fn map_lumi(l: Float) -> Float {
            let (a, b, c, d, e) = (2.51, 0.03, 2.43, 0.59, 0.14);
            (l * ((l*a) + b)) / ((l*((l*c) + d)) + e)
        }

        let l_white = self.get_white(lumi, percentile);
        let mapped_white = map_lumi(l_white);
        let mut comp_lumi: Vec<Float> = self.prescale(lumi, alpha);
        comp_lumi.iter_mut().for_each(|l| *l = map_lumi(*l) / mapped_white );
        comp_lumi
    }

    fn filmic_tmo(&self, lumi: &Vec<Float>, alpha: Float, percentile: Float) -> Vec<Float> {
        
        fn map_lumi(l: Float) -> Float {
            let (a, b, c, d, e, f) = (0.22 as Float, 0.3 as Float, 0.1 as Float, 0.2 as Float, 0.01 as Float, 0.3 as Float);
            (( (l * ( (a*l) + (c*b) )) + (d*e)  ) / ( (l * ((a*l) + b)) + (d*f) )) - (e / f) 
        }

        let l_white = self.get_white(lumi, percentile);
        let mapped_white = map_lumi(l_white);
        let mut comp_lumi: Vec<Float> = self.prescale(lumi, alpha);
        comp_lumi.iter_mut().for_each(|l| *l = map_lumi(*l) / mapped_white );
        comp_lumi
    }

}