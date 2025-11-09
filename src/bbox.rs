/*

    Axis Aligned Bounding Box

    @author: bartu
    @date: 9 Nov, 2025
*/


use crate::prelude::*;

use crate::interval::{Interval};
use crate::json_structs::{VertexData};

pub struct BBox {
    xmin: Float, xmax: Float,
    ymin: Float, ymax: Float,
    zmin: Float, zmax: Float,
    
    width: Float,
    height: Float,
    depth: Float,
}

impl BBox {
    pub fn new_from(xint: &Interval, yint: &Interval, zint: &Interval) -> Self {
        
        assert!(xint.validate() && yint.validate() && zint.validate(), "Invalid interval, found max < min");
        Self {
            xmin: xint.min,
            xmax: xint.max,
            ymin: yint.min,
            ymax: yint.max,
            zmin: zint.min,
            zmax: zint.max,
            width: xint.max - xint.min,
            height: yint.max - yint.min,
            depth: zint.max - zint.min,
        }
    }
}

pub trait BBoxable {
    fn get_bbox(&self, verts: &VertexData) -> BBox;
}