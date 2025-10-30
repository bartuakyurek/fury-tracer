/*

    Given Scene description and Camera,
    render an image.

    Currently supports:
        - <TODO: type of raytracing here e.g. recursive>


    @date: Oct 11, 2025
    @author: Bartu
*/

use std::rc::Rc;
use std::sync::Arc;
use rayon::prelude::*;
use std::io::{self, Write};
use bevy_math::{NormedVectorSpace, VectorSpace};
use tracing::span::Record;
use tracing::{debug, info, warn, error};
use std::{self, time::Instant};

use crate::camera::Camera;
use crate::material::{HeapAllocMaterial};
use crate::ray::{HitRecord, Ray};
use crate::scene::{PointLight, Scene};
use crate::numeric::{Float, Vector3};
use crate::image::{ImageData};
use crate::interval::{Interval, FloatConst};
use crate::shapes::{ShapeList, HeapAllocatedVerts};



pub fn closest_hit(ray: &Ray, t_interval: &Interval, shapes: &ShapeList, vertex_cache: &HeapAllocatedVerts) -> Option<HitRecord>{
    // Refers to p.91 of slide 01_b, lines 3-7
    let mut rec = None;
    let mut t_min = FloatConst::INF;
    for shape in shapes.iter() { // TODO: later we'll use acceleration structures instead of checking *all* objects like this
       if let Some(hit_record) = shape.intersects_with(ray, &t_interval, vertex_cache){
           // Update if new hit is closer 
           if t_min > hit_record.ray_t { 
               t_min = hit_record.ray_t;
               rec = Some(hit_record);
           }
       }
   }
   rec
}

pub fn any_hit(ray: &Ray, t_interval: &Interval, shapes: &ShapeList, vertex_cache: &HeapAllocatedVerts) -> bool {
    // Check if ray intersects with any shape in the scene
    for shape in shapes.iter() { // TODO: later we'll use acceleration structures instead of checking *all* objects like this
       if let Some(_) = shape.intersects_with(ray, &t_interval, vertex_cache){
           return true;
       }
   }
   false
}

pub fn get_shadow_ray(point_light: &PointLight, hit_record: &HitRecord, epsilon: Float) -> (Ray, Interval) { // TODO: Should we box hitrecord here?
    
    debug_assert!(hit_record.normal.is_normalized());
    let ray_origin = hit_record.hit_point + (hit_record.normal * epsilon);
    let distance_vec = point_light.position - ray_origin;
    let distance_squared = distance_vec.norm_squared(); // TODO: Cache?
    let distance = distance_squared.sqrt();
    let dir = distance_vec / distance;
    debug_assert!(dir.is_normalized());
    let shadow_ray = Ray::new(ray_origin, dir);
    let interval = Interval::new(0.0, distance); 
    (shadow_ray, interval)
}

// TODO: Wait why there is both scene and shapes where scene already should contain shapes?
pub fn shade_diffuse(scene: &Scene, shapes: &ShapeList, vertex_cache: &HeapAllocatedVerts, hit_record: &HitRecord, ray_in: &Ray, mat: &HeapAllocMaterial) -> Vector3 {
    let mut color = Vector3::ZERO;
    for point_light in scene.lights.point_lights.all() {
            
            let (shadow_ray, interval) = get_shadow_ray(&point_light, hit_record, scene.shadow_ray_epsilon);
            if !any_hit(&shadow_ray, &interval, shapes, vertex_cache) {
                // TODO: We can implement attenuate( ) for diffuse by taking 
                // denominator part out of irradiance and use it in attenuate( )
                // that way get_shadow_ray( ) can return ray_t: Float, instead of interval
                let irradiance = point_light.rgb_intensity / shadow_ray.squared_distance_at(interval.max); // TODO interval is confusing here
                let n = hit_record.normal;
                let w_i = shadow_ray.direction;
                let w_o = -ray_in.direction;
                color += mat.diffuse(w_i, n) * irradiance;
                color += mat.specular(w_o, w_i, n) * irradiance; 
            }
    }
    color
}

pub fn get_color(ray_in: &Ray, scene: &Scene, shapes: &ShapeList, vertex_cache: &HeapAllocatedVerts, depth: usize) -> Vector3 { // TODO: add depth & check depth > scene.max_recursion_depth
   // TODO: Shouldn't we box the scene or even Rc<scene> here? otherwise it lives on the stack
   // and it's a huge struct, isn't it?
   if depth >= scene.max_recursion_depth {
        return scene.background_color;
   }
   
   let t_interval = Interval::positive(scene.intersection_test_epsilon);
   if let Some(hit_record) = closest_hit(ray_in, &t_interval, shapes, vertex_cache) {
        
        let mat: &HeapAllocMaterial = &scene.materials.materials[hit_record.material - 1];
        let mut color = mat.ambient() * scene.lights.ambient_light;
        let mat_type = mat.get_type();
        let epsilon = scene.intersection_test_epsilon; // TODO: Is this the correct epsilon? Seems like yes, visually checked with other epsilon vs. given output image 
        color += match mat_type{ // WARNING: Expecting lowercase material
            "diffuse" => {
                shade_diffuse(scene, shapes, vertex_cache, &hit_record, &ray_in, mat)
            },
            "mirror" => {
                    //let attenuation = mat.attenuate_reflect(ray_in, hit_record.ray_t); 
                    if let Some((reflected_ray, attenuation)) = mat.reflect(ray_in, &hit_record, epsilon) {
                        shade_diffuse(scene, shapes, vertex_cache, &hit_record, &ray_in, mat) + attenuation * get_color(&reflected_ray, scene, shapes, vertex_cache, depth + 1) 
                    }
                    else {
                        warn!("Mirror reflection is missing in 'mirror' arm in renderer.rs .");
                        Vector3::ZERO // Perfect mirror always reflects so this hopefully is not triggered
                    }
            }, 
           "dielectric" | "conductor" => {
                let mut tot_radiance = Vector3::ZERO;
                
                // Only add diffuse, specular, and ambient components if front face (see slides 02, p.29)
                // TODO: Below should have been without "!" but I'm not sure why this looks better 
                if !hit_record.is_front_face { 
                    tot_radiance += shade_diffuse(scene, shapes, vertex_cache, &hit_record, &ray_in, mat);
                }
 
                // Reflected 
                //TODO: there could be a single scatter( ) taking parameter is_reflect to decide which one to call...
                if let Some((reflected_ray, attenuation)) = mat.reflect(ray_in, &hit_record, epsilon) {
                        tot_radiance += attenuation * get_color(&reflected_ray, scene, shapes, vertex_cache, depth + 1);
                }
        
                // Refracted 
                // TODO: Should we check !is_front_face here? 
                if let Some((refracted_ray, attenuation)) = mat.refract(ray_in, &hit_record, epsilon) {
                        tot_radiance += attenuation * get_color(&refracted_ray, scene, shapes, vertex_cache, depth + 1);
                }
                tot_radiance
            }
            _ => {
                // WARNING: Below does not panic when json has unknown material because parser defaults it to Diffuse (however it does panic if you make a typo or not implement shading function)
                panic!(">> Unknown material type '{}'! Shading function for this material is missing.", mat_type); 
            },
        };
        color
   }
   else {
        scene.background_color // no hit
   }
}

pub fn render(scene: &Scene) -> Result<Vec<ImageData>, Box<dyn std::error::Error>>
{
    let mut images: Vec<ImageData> = Vec::new();

    for mut cam in scene.cameras.all() {
        cam.setup(); // TODO: Could this be integrated to deserialization? Because it's easy to forget calling it
        if cam.num_samples != 1 { warn!("Found num_samples = '{}' > 1, sampling is not implemented yet...", cam.num_samples); }
        
        let start = Instant::now();
       
        let eye_rays = cam.generate_primary_rays();
        let shapes: &ShapeList = &scene.objects.all_shapes;
        info!(">> There are {} shapes in the scene.", shapes.len());
        
        let vcache: &HeapAllocatedVerts = &scene.vertex_cache;

        // --- Rayon Multithreading ---
        let pixel_colors: Vec<_> = eye_rays
            .par_iter()
            .map(|ray| get_color(ray, scene, shapes, vcache, 0))
            .collect();
        // -----------------------------
            
        let im = ImageData::new_from_colors(cam.image_resolution, cam.image_name, pixel_colors);
        images.push(im);
        info!("Rendering of image took: {:?}", start.elapsed()); 
    }
    
    Ok(images)
}

