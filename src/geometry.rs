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

use std::{ops::Index};
use crate::shapes::{Triangle};
use crate::numeric::{Float, Vector3, approx_zero};
use crate::{ray::Ray, interval::Interval, dataforms::VertexData};

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
    
    debug_assert!(approx_zero(normal.length() - 1.0));
    normal
}


//fn get_beta_gamma_t(a: Vector3, b: Vector3, c: Vector3, o: Vector3, d: Vector3) -> (Float, Float, Float) { 
//    // Helper function for computations in Slides 01_B, p.30
//    // a, b, c are triangle corners
//    // o, d are ray's origin and direction r(t) = o + d * t
//    //
//    // TODO: reduce verbosity and unnecessary operations
//    // I've written in naive way for the beginning
//    let (ax, ay, az) = (a[0], a[1], a[2]);
//    let (bx, by, bz) = (b[0], b[1], b[2]);
//    let (cx, cy, cz) = (c[0], c[1], c[2]);
//    let (ox, oy, oz) = (o[0], o[1], o[2]);
//    let (dx, dy, dz) = (d[0], d[1], d[2]);
//
//    // Construct A
//    let A_x = Vector3::new(ax - bx, ay - by, az - bz);
//    let A_y = Vector3::new(ax - cx, ay - cy, az - cz);
//    let A_z = Vector3::new(dx, dy, dz);
//    let A = Matrix3::from_cols(A_x, A_y, A_z);
//    let A_determinant = A.determinant();
//
//    // Construct beta 
//    let beta_x = Vector3::new(ax - ox, ay - oy, az - oz);
//    let beta_y = Vector3::new(ax - cx, ay - cy, az - cz);
//    let beta_z = Vector3::new(dx, dy, dx);
//    let beta_matrix = Matrix3::from_cols(beta_x, beta_y, beta_z);
//    let beta = beta_matrix.determinant() / A_determinant;
//
//    // Construct gamma
//    let gamma_x = Vector3::new(ax - bx, ay - by, az - bz);
//    let gamma_y = Vector3::new(ax - ox, ay - oy, az - oz);
//    let gamma_z = Vector3::new(dx, dy, dz);
//    let gamma_matrix = Matrix3::from_cols(gamma_x, gamma_y, gamma_z);
//    let gamma = gamma_matrix.determinant() / A_determinant;
//
//    let t_x = Vector3::new(ax - bx, ay - by, az - bz);
//    let t_y = Vector3::new(ax - cx, ay - cy, az - cz);
//    let t_z = Vector3::new(ax - ox, ay - oy, az - oz);
//    let t_matrix = Matrix3::from_cols(t_x, t_y, t_z);
//    let t = t_matrix.determinant() / A_determinant;
//
//    (beta, gamma, t)
//}

//fn lengthy_but_simple_intersection(ray: &Ray, t_interval: &Interval, tri_indices: [usize; 3], verts: &VertexData) -> Option<(Vector3, Float)> {
//    // Slides 01_B, p.14
//    //
//    //  n    a  
//    //   \  / \
//    //     /   \
//    //   b ----- c
//    let [a, b, c] = tri_indices.map(|i| verts[i]);
//    let (beta, gamma, t) = get_beta_gamma_t(a, b, c, ray.origin, ray.direction);
//
//    // Conditions at p.32
//    if !t_interval.contains(t) {
//        return None;
//    }
//    if (beta + gamma) > 1. {
//        return None;
//    }
//    if (0. > beta) || (0. > gamma) {
//        return None;
//    }
//
//    // Construct p from barycentric coords
//    let p = ray.at(t);
//    //let p = a + (beta * (b - a)) + (gamma * (c - a)); // p.27
//    //assert!(approx_zero((p-pt).norm())); TODO: Why does it fail?
//
//    // Check for edge BA 
//    let edge_ba = a - b;
//    let edge_bc = c - b;
//    let n = (edge_bc).cross(edge_ba); // vc in p.16
//    let vp = (p - b).cross(edge_ba); // TODO: we can use the same vp for other checks, right?
//    if vp.dot(n) <= 0.0 {
//        return None;
//    }
//
//    // Check for AC
//    let edge_ca = a - c;
//    let edge_ac = c - a;
//    let vb = (edge_bc).cross(edge_ac);
//    if vb.dot(n) <= 0.0 {
//        return None;
//    }
//
//    // Check for CB
//    let edge_cb = b - c;
//    let va = (edge_ca).cross(edge_cb);
//    if va.dot(n) <= 0.0 {
//        return None;
//    }
//
//    Some((p, t))
//}

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
    if (determinant > -t_interval.min) && (determinant < t_interval.min) {
        return None;
    }
    let inverse_determinant = 1.0 as Float / determinant;
    let dist = ray.origin - tri_pivot;
    let barycentric_u = dist.dot(perp) * inverse_determinant;
    if (barycentric_u < 0.0) || (barycentric_u > 1.0) {
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


trait StructofArrays {
    type Item;

    fn vectorize(&self) -> Vec<Self::Item>; // Convert to AoS
    fn len(&self) -> usize;
}

pub struct CoordLike {
    // Struct of Arrays for 3D coordinates-like data
    // Useful for holding vertex coordinates or face normals etc.
    // WARNING: Assumes all fields have equal length
    xs: Vec<Float>,
    ys: Vec<Float>,
    zs: Vec<Float>,
}

impl CoordLike {
    pub fn new_from(coords: &Vec<Vector3>) -> Self {
        let xs = (0..coords.len()).map(|i| coords[i][0]).collect();
        let ys = (0..coords.len()).map(|i| coords[i][1]).collect();
        let zs = (0..coords.len()).map(|i| coords[i][2]).collect();
        Self {
            xs,
            ys,
            zs,
        }
    }
}

impl StructofArrays for CoordLike {
    type Item = Vector3;

    fn vectorize(&self) -> Vec<Self::Item> {
        (0..self.len()).map(|i| Vector3::new(self.xs[i], self.ys[i], self.zs[i])).collect()
    }

    fn len(&self) -> usize{
        // Check if all vectors have same size
        // Return length of the struct
        assert_eq!(self.xs.len(), self.ys.len());
        assert_eq!(self.xs.len(), self.zs.len());
        assert_eq!(self.ys.len(), self.zs.len());

        self.xs.len()
    }
}

impl CoordLike {
   
    fn tri_normals(triangles: &Vec<Triangle>, vertices: &Vec<Vector3>) -> CoordLike {
        
        let len = triangles.len();
        let mut xs: Vec<Float> = vec![0.; len];
        let mut ys: Vec<Float> = vec![0.; len];
        let mut zs: Vec<Float> = vec![0.; len];

        for (i, tri) in triangles.iter().enumerate()  {
            let v1 = vertices[tri.indices[0]];
            let v2 =  vertices[tri.indices[1]];
            let v3 = vertices[tri.indices[2]];

            let n = get_tri_normal(&v1, &v2, &v3);
            (xs[i], ys[i], zs[i]) = (n[0], n[1], n[2]);
        }
       
        CoordLike { xs, ys, zs }
    }
    
}


#[cfg(test)]
mod tests {
    use super::*; // access to the outer scope

    //#[test]
    //fn test_normals() {
    //    // WARNING: A simple test is provided, does not
    //    // check degenerate cases at this point.
    //    let verts: Vec<Vector3> = vec![
    //            Vector3::new(0., 0., 0.),
    //            Vector3::new(1., 0., 0.),
    //            Vector3::new(0.5, 0.5, 0.),
    //    ];
    //    let tri = Triangle { _id: 0, 
    //                indices: [0, 1, 2], 
    //                material_idx: 0, 
    //                
    //            };
    //
    //    let n_tri: usize = 20;
    //    let triangles = vec![tri; n_tri];
    //    let tri_normals_soa = CoordLike::tri_normals(&triangles, &verts);
    //    let tri_normals_aos = tri_normals_soa.vectorize();
    //    assert_eq!(tri_normals_aos, vec![Vector3::new(0.,0.,1.); n_tri]);
    //}
}