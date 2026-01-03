

use crate::prelude::*;


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