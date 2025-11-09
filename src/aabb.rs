/*

    Axis Aligned Bounding Box

    @author: bartu
    @date: 9 Nov, 2025
*/


use crate::prelude::*;

struct BBox {
    xmin: Float, xmax: Float,
    ymin: Float, ymax: Float,
    zmin: Float, zmax: Float,
    
    width: Float,
    height: Float,
    depth: Float,
}

trait BBoxable {
    fn get_bbox(&self) -> BBox;
}