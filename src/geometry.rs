/*

    Aggregate geometry utilities on Shapes
    and cache them in a struct. WARNING: None
    of the structs are actually used at the moment.
    
    TODO:This module is intended to operate on libigl-like data
    expecting matrices V, F for verts and faces to compute
    normals and other useful cache for ray tracing.
    
    @date: 9 Oct, 2025
    @author: bartu
*/


use crate::json_structs::{VertexData};
use crate::{ray::Ray, interval::Interval};
use crate::prelude::*;

/// Return true if any of the two verts at the same position
pub fn is_degenerate_triangle(verts: &VertexData, faces: [usize; 3]) -> bool {

    for i in 0..3 {
        for j in 0..3 {
    
            if i == j {
                continue;
            }

            let outer = faces[i];
            let inner = faces[j];
            if outer == inner {
                debug!("Found degenarate triangle where face indices correspond to same vertex. {:?}", faces);
                return true;
            }
    
            if verts[outer].distance_squared(verts[inner]) < 1e-10 {
                debug!("Found degenarate triangle with vertices v1: {:?} = v2: {:?} ", verts[inner], verts[outer]);
                return true;
            }
        }
    }
    
    return false;
}

// NOTE: There is an article on how to rotate-align without trigonometry
// https://iquilezles.org/articles/noacos/ 
// it does not directly apply in our case but might be handy in future.
pub fn rodrigues_rotation(axis: &Vector3, angle: Float) -> Matrix3 {
    
    let angle = -angle; // TODO: WARNING: This is not how handedness problem was supposed to be solved...
    let k = axis.normalize();
    let x = k.x;
    let y = k.y;
    let z = k.z;

    let si = angle.sin();
    let co = angle.cos();
    let ic = 1.0 - co;

    Matrix3::from_cols(
        Vector3::new(x*x*ic + co,    y*x*ic - si*z,  z*x*ic + si*y),
        Vector3::new(x*y*ic + si*z,  y*y*ic + co,   z*y*ic - si*x),
        Vector3::new(x*z*ic - si*y,  y*z*ic + si*x, z*z*ic + co),
    )
}

pub fn get_tri_normal(v1: &Vector3, v2: &Vector3, v3: &Vector3) -> Vector3{
    // WARNING: Assumes triangle indices are given in counter clockwise order 
    //
    //    v1
    //  /    \
    // v2 —— v3
    //
    let left = v1 - v2;
    let right = v3 - v2;
    let mut normal = right.cross(left); 
    normal = normal.normalize();
    
    debug_assert!(normal.is_normalized());
    normal
}


pub fn moller_trumbore_intersection(ray: &Ray, t_interval: &Interval, tri_indices: [usize; 3], verts: &VertexData) -> Option<(Float, Float, Float)> {
    // Based on Möller-Trumbore algorithm
    //
    //     a (pivot)
    //    / \
    //  b  -  c
    // 
    // WARNING: Assumes given interval has incorporated relevant epsilon e.g.
    // instead of [0.0, inf], [0.0001, inf] is given otherwise there might be
    // floating point errors.
    // TODO: Is there something wrong in this function?
    let tri_coords = tri_indices.map(|i| verts[i]);
    let [tri_pivot, tri_left, tri_right] = tri_coords;        
    let edge_ab = tri_left - tri_pivot;
    let edge_ac = tri_right - tri_pivot;
    // Scalar triple product https://youtu.be/fK1RPmF_zjQ
    debug_assert!(ray.direction.is_normalized());
    let perp = ray.direction.cross(edge_ac);
    let determinant: Float = perp.dot(edge_ab);
    if (determinant > -t_interval.min) && (determinant < t_interval.min) { // TODO: shouldn't this be ray epsilon? t_interval.min could be zero here?
        return None;
    }
    let inverse_determinant = 1.0 as Float / determinant;
    let dist = ray.origin - tri_pivot;
    let barycentric_u = dist.dot(perp) * inverse_determinant;
    if !(0.0..=1.0).contains(&barycentric_u) {
        return None;
    }
    let another_perp = dist.cross(edge_ab);
    let barycentric_v = ray.direction.dot(another_perp) * inverse_determinant;
    if (barycentric_v < 0.0) || ((barycentric_u + barycentric_v) > 1.0) {
        return None;
    }
    // Get ray t
    let t = edge_ac.dot(another_perp) * inverse_determinant;
    if !t_interval.contains(t) {
        return None;
    }
    Some((barycentric_u, barycentric_v, t))
}

