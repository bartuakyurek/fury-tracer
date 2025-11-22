/*

    Given Scene description and Camera,
    render an image.

    Currently supports:
        - Recursive ray tracing 


    @date: Oct 11, 2025
    @author: Bartu
*/

use rayon::prelude::*;
use bevy_math::{NormedVectorSpace};
use std::{self, time::Instant, sync::Arc, sync::Mutex};

use crate::material::{HeapAllocMaterial};
use crate::ray::{HitRecord, Ray};
use crate::scene::{PointLight, Scene};
use crate::image::{ImageData};
use crate::interval::{Interval};
use crate::prelude::*;


/// Returns a tuple of ray from hit point (epsilon shifted) towards the point light
/// and interval [0, distance]. 
pub fn get_shadow_ray(point_light: &PointLight, hit_record: &HitRecord, epsilon: Float) -> (Ray, Interval) { 
        
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

pub fn shade_diffuse(scene: &Scene, hit_record: &HitRecord, ray_in: &Ray, mat: &HeapAllocMaterial) -> Vector3 {
    let mut color = mat.ambient() * scene.data.lights.ambient_light; 
    for point_light in scene.data.lights.point_lights.all() {
            
            let (shadow_ray, interval) = get_shadow_ray(&point_light, hit_record, scene.data.shadow_ray_epsilon);
            if scene.hit_bvh(&shadow_ray, &interval, true).is_none() {
                
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

struct HitTrace(HitRecord, Vector3); // Hitrecord and radiance tuple

pub fn get_color(ray_in: &Ray, scene: &Scene, depth: usize, hitpool: Arc<Mutex<Vec<HitTrace>>>) -> Vector3 { 
  
   if depth >= scene.data.max_recursion_depth {
        return scene.data.background_color;
   }
   
   let t_interval = Interval::positive(scene.data.intersection_test_epsilon);
   if let Some(hit_record) = scene.hit_bvh(ray_in, &t_interval, false) {
        

        let mat: &HeapAllocMaterial = &scene.data.materials.materials[hit_record.material - 1];
        let mut color = Vector3::ZERO;
        let mat_type = mat.get_type();
        let epsilon = scene.data.intersection_test_epsilon;  
        let radiance = match mat_type{ 
            "diffuse" => {
                shade_diffuse(scene, &hit_record, &ray_in, mat)
            },
            "mirror" | "conductor" => { 
                    if let Some((reflected_ray, attenuation)) = mat.interact(ray_in, &hit_record, epsilon, true) {
                        shade_diffuse(scene,  &hit_record, &ray_in, mat) + attenuation * get_color(&reflected_ray, scene, depth + 1, hitpool.clone()) 
                    }
                    else {
                        warn!("Material not reflecting...");
                        Vector3::ZERO // Perfect mirror always reflects so this hopefully is not triggered
                    }
            }, 
           "dielectric" => {
                let mut tot_radiance = Vector3::ZERO;
                
                // Only add diffuse, specular, and ambient components if front face (see slides 02, p.29)
                if hit_record.is_front_face { 
                    tot_radiance += shade_diffuse(scene, &hit_record, &ray_in, mat);
                }
 
                // Reflected 
                if let Some((reflected_ray, attenuation)) = mat.interact(ray_in, &hit_record, epsilon, true) {
                        tot_radiance += attenuation * get_color(&reflected_ray, scene, depth + 1, hitpool.clone());
                }
        
                // Refracted 
                if let Some((refracted_ray, attenuation)) = mat.interact(ray_in, &hit_record, epsilon, false) {
                        tot_radiance += attenuation * get_color(&refracted_ray, scene, depth + 1, hitpool.clone());
                }
                tot_radiance
            },
            _ => {
                // WARNING: Below does not panic when json has unknown material because parser defaults it to Diffuse (however it does panic if you make a typo or not implement shading function)
                panic!(">> Unknown material type '{}'! Shading function for this material is missing.", mat_type); 
            },
        };
        color += radiance;
        hitpool.lock().unwrap().push(HitTrace(hit_record.clone(), radiance));
        color
   }
   else {
        scene.data.background_color // no hit
   }
}

/// Given hitpool, return pixel colors
pub fn postprocess(hitpool: &Vec<HitTrace>) -> Vec<Vector3> {
    todo!() 
}

pub fn render(scene: &Scene) -> Result<Vec<ImageData>, Box<dyn std::error::Error>>
{
    let mut images: Vec<ImageData> = Vec::new();

    for mut cam in scene.data.cameras.all() {
        // TODO: Could setup() be integrated to deserialization? Because it's easy to forget calling it
        // but for that to be done in Scene creation (or in setup() of scene), cameras need to be
        // vectorized via .all( ) call, however we don't hold the vec versions (currently they are SingleOrVec) 
        //  in actual scene structs, that needs to be changed maybe.
        cam.setup(&scene.data.transformations); 
        if cam.num_samples != 1 { warn!("Found num_samples = '{}' > 1, sampling is not implemented yet...", cam.num_samples); }
        
        // --- Rayon Multithreading ---
        info!("Starting rayon multithreading...");
        let start = Instant::now();
        let eye_rays: Vec<Ray> = cam.generate_primary_rays();
        let hitpool = Arc::new(Mutex::new(Vec::new()));
       
        let pixel_colors: Vec<_> = eye_rays
            .par_iter()
            .map(|ray| get_color(ray, scene, 0, hitpool.clone()))
            .collect();
        info!("Rendering of {} took: {:?}", cam.image_name, start.elapsed()); 
        // --- Post processing Hitpool ----
        info!("Hitpool has {} entries.", hitpool.lock().unwrap().len());
        let postproc_colors = postprocess(&hitpool.lock().unwrap());
        // ------ Push final images (both original and postprocessed) -----
        let raytraced_image = ImageData::new_from_colors(cam.image_resolution, cam.image_name.clone(), pixel_colors);
        let postproc_image = ImageData::new_from_colors(cam.image_resolution, format!("{}{}", String::from("post_"), cam.image_name.clone()), postproc_colors);
        images.push(raytraced_image);
        images.push(postproc_image);
    }
    
    Ok(images)
}

