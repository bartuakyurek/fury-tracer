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

use std::f64::consts::PI;
use std::{self, time::Instant};

use crate::material::{BRDFData, HeapAllocMaterial};
use crate::ray::{HitRecord, Ray};
use crate::light::{LightKind};
use crate::scene::{Scene};
use crate::camera::Camera;
use crate::image::{DecalMode, ImageData, Interpolation, TextureMap};
use crate::interval::{Interval};
use crate::prelude::*;


/// Returns a tuple of ray from hit point (epsilon shifted) towards the point light
/// and interval [0, distance]. 
pub fn get_shadow_ray(light: &LightKind, hit_record: &HitRecord, ray_in: &Ray, epsilon: Float) -> (Ray, Interval) { 
        
    debug_assert!(hit_record.normal.is_normalized());
    let ray_origin = hit_record.hit_point + (hit_record.normal * epsilon);

    let (dir, distance) = light.get_shadow_direction_and_distance(&ray_origin);
    debug_assert!(dir.is_normalized());
    let shadow_ray = Ray::new(ray_origin, dir, ray_in.time); // TODO: is it important for shadow ray's time parameter? could it be just set zero?
    let interval = Interval::new(0.0, distance); 
    (shadow_ray, interval)
}

pub fn shade_diffuse(scene: &Scene, hit_record: &HitRecord, ray_in: &Ray, brdf: &BRDFData) -> Vector3 {
    let mut color = brdf.ambient() * scene.data.lights.ambient_light; 
    for light in scene.data.lights.all_shadow_rayable().iter() {
            
            let (shadow_ray, interval) = get_shadow_ray(&light, hit_record, ray_in, scene.data.shadow_ray_epsilon);
            if scene.hit_bvh(&shadow_ray, &interval, true).is_none() {

                // Note: below assert might fail in bump or normal mapping case once the normals are updated:
                debug_assert!( (hit_record.is_front_face && hit_record.normal.dot(ray_in.direction) < 1e-6) || (!hit_record.is_front_face && hit_record.normal.dot(ray_in.direction) > -1e-6), "Found front_face = {} and normal dot ray_in direction = {}", hit_record.is_front_face, hit_record.normal.dot(ray_in.direction) );
                // Note that we don't attenuate the light as we assume rays are travelling in vacuum
                // but area lights will scale intensity wrt ray's direction and for point lights attenuation is simply one
                let irradiance = light.get_irradiance(&shadow_ray, &interval); //light.get_intensity() * light.attenuation(&shadow_ray.direction) / shadow_ray.squared_distance_at(interval.max); // TODO interval is confusing here
                let n = hit_record.normal;
                let w_i = shadow_ray.direction;
                let w_o = -ray_in.direction;
                color += brdf.diffuse(w_i, n) * irradiance;
                color += brdf.specular(w_o, w_i, n) * irradiance; 
            }
    }

    // HW5 Update: add color from environment lights (I kept it separate from shadow ray logic)
    // TODO: refactor this huge function
    for env_light in scene.data.lights.env_lights.iter() {
        if let Some(textures) = &scene.data.textures {
            //let (sampled_dir, radiance) 
            let (sampled_dir, radiance) = env_light.sample_and_get_radiance(
                &hit_record,
                textures,
            );
            
            let w_i = sampled_dir;
            let w_o = -ray_in.direction;
            let n = hit_record.normal;
            color += radiance  * brdf.diffuse(w_i, n);
            color += brdf.specular(w_o, w_i, n) * radiance; 

        }
    }

    color
}

pub fn get_color(ray_in: &Ray, scene: &Scene, cam: &Camera, depth: usize) -> Vector3 { 
  
   if depth >= scene.data.max_recursion_depth {
        //return Vector3::X * 255.;
        return sample_background(ray_in, scene, cam);
        //return scene.data.background_color;
   }
   
   let t_interval = Interval::positive(scene.data.intersection_test_epsilon);
   if let Some(mut hit_record) = scene.hit_bvh(ray_in, &t_interval, false) {
        
        let mat: &HeapAllocMaterial = &scene.data.materials.data[hit_record.material - 1];
        let mut brdf = mat.brdf().clone(); // Clone needed for mutability but if no texture is present this is very unefficient I assume    
        
        // HW4 Update: apply textures if provided to change brdf -----------
        if let Some(textures) = &scene.data.textures {
            for texmap_id in &hit_record.textures {
                let texmap = &textures.texture_maps.as_slice()[*texmap_id - 1]; // TODO: I am not sure if as_slice( ) is still relevant here, it resolved a rustc error before I change the implementation though            
                
                let uv = &hit_record.texture_uv.expect("Texture coordinates (u, v) is not written to hitrecord.");
                let interpolation = texmap.interpolation().unwrap_or(&Interpolation::DEFAULT); //
                let tex_color = textures.tex_from_map(texmap_id - 1, *uv, interpolation, true, hit_record.hit_point);
                if let Some(decal_mode) = texmap.decal_mode() {
                    match decal_mode {
                        // Update BRDF ----------------------------------------------------------
                        DecalMode::BlendKd => { brdf.diffuse_rf = (0.5 * brdf.diffuse_rf) + (0.5 * tex_color); }, // in blendKd do we mix by 0.5 weights or just add them together? could there be multilpe blendkd?
                        DecalMode::ReplaceKd => { brdf.diffuse_rf = tex_color;  },
                        DecalMode::ReplaceKs => { brdf.specular_rf = tex_color; },
                        DecalMode::ReplaceAll => { 
                                                    brdf.diffuse_rf = tex_color;   
                                                    brdf.specular_rf = tex_color;
                                                    brdf.ambient_rf = tex_color;
                                                },
                        // Update hitrecord normal ----------------------------------------------
                        DecalMode::ReplaceNormal => { 
                                                     // TODO: better solution than "apply_normalization" parameter in retrieving colors...? 
                                                     let tex_color = textures.tex_from_map(texmap_id - 1, hit_record.texture_uv.unwrap(), texmap.interpolation().unwrap(), false, hit_record.hit_point);
                                                     let dir = ImageData::color_to_direction(tex_color);
                                                     hit_record.normal = hit_record.tbn_matrix.unwrap() * dir;
                                                     debug_assert!(hit_record.normal.is_normalized());
                                                    },
                        DecalMode::BumpNormal => {
                                let perturbed_normal = textures.get_bump_mapping(texmap, &hit_record);
                                hit_record.normal = perturbed_normal; // Update normals for bump mapping (see the goal in slides 07, p.23)
                                debug_assert!(!perturbed_normal.is_nan(), "Found perturbed normal: {}", perturbed_normal);
                                debug_assert!(hit_record.normal.is_normalized(), "Found hit record normal: {}", hit_record.normal);
                        },
                        DecalMode::ReplaceBackground => {todo!("Unexpected decalibration mode! This is implemented elsewhere in the renderer...");},
                        _ => { debug!("Unexpeced decalibration mode {:?}...", decal_mode); }
                    }
                }
            }
        }; 
        // -----------------------------------------------------------------

        let mut color = Vector3::ZERO;
        let mat_type = mat.get_type();
        let epsilon = scene.data.intersection_test_epsilon;  
        color += match mat_type{ 
            "diffuse" => {
                shade_diffuse(scene, &hit_record, ray_in, &brdf)
            },
            "mirror" | "conductor" => { 
                    if let Some((reflected_ray, attenuation)) = mat.interact(ray_in, &hit_record, epsilon, true) {
                        shade_diffuse(scene,  &hit_record, ray_in, &brdf) + attenuation * get_color(&reflected_ray, scene, cam, depth + 1) 
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
                    tot_radiance += shade_diffuse(scene, &hit_record, ray_in, &brdf);
                }
 
                // Reflected 
                if let Some((reflected_ray, attenuation)) = mat.interact(ray_in, &hit_record, epsilon, true) {
                        tot_radiance += attenuation * get_color(&reflected_ray, scene, cam, depth + 1);
                }
        
                // Refracted 
                if let Some((refracted_ray, attenuation)) = mat.interact(ray_in, &hit_record, epsilon, false) {
                        tot_radiance += attenuation * get_color(&refracted_ray, scene, cam, depth + 1);
                }
                tot_radiance
            },
            _ => {
                // WARNING: Below does not panic when json has unknown material because parser defaults it to Diffuse (however it does panic if you make a typo or not implement shading function)
                panic!(">> Unknown material type '{}'! Shading function for this material is missing.", mat_type); 
            },
        };

        color
   }
   else {
        sample_background(ray_in, scene, cam)
   }
}


fn sample_background(ray_in: &Ray, scene: &Scene, cam: &Camera) -> Vector3 {
    // TODO: avoid iterating over textures, cache if background texture is
    // provided in json file

    if !scene.data.lights.env_lights.is_empty() {
        let env_light = &scene.data.lights.env_lights.all()[0]; // Use first env light
        
        if let Some(textures) = &scene.data.textures {
           debug_assert!(ray_in.direction.is_normalized());
           
           let dir = ray_in.direction; 
           let mut uv = env_light.get_uv(dir);
           //let mut uv = cam.calculate_nearplane_uv(ray_in);
           uv[0] = (uv[0] - 0.5); // 2.;
           uv[1] = (uv[1] - 0.5); // 2.;
           
           let radiance = textures.tex_from_img(
               env_light.image_idx(),  
               uv,
               &Interpolation::Bilinear,
           );
           
           return radiance; // * 2. * PI;
        }
    }
    
    if let Some(textures) = &scene.data.textures {
            for texmap in textures.texture_maps.iter() {
                if let Some(decal_mode) = texmap.decal_mode() {
                    match decal_mode {
                        DecalMode::ReplaceBackground => {
                            let uv = cam.calculate_nearplane_uv(ray_in);
                            let interpolation = texmap.interpolation().unwrap_or(&Interpolation::DEFAULT);
                            let bg_color = textures.tex_from_map(
                                texmap.index(),
                                uv,
                                interpolation,
                                true, 
                                Vector3::ZERO, 
                            );
                            //let bg_color = Vector3::Y * 255.;
                            return bg_color * 255.; // TODO: WARNING THIS IS ERROR PRONE. Background image was returned in range [0, 1] but that appears black, so scale it back
                        },
                        _ => { debug!("ignoring decal mode {:?}...", decal_mode); }
                    }
                }
            }
        }; 
        //return Vector3::Z * 255.;
        scene.data.background_color // no hit
}


/// Average samples per pixel (WARNING: it does not incorporate neighbouring 
/// pixels, simply partitions given colors into chunks with each chunk having
/// n_samples length, then sums each component and takes the average)
fn box_filter(colors: &Vec<Vector3>, n_samples: usize) -> Vec<Vector3> {
    info!("Applying box filter to obtain final pixel colors...");
    colors
            .chunks_exact(n_samples)
            .map(|chunk| {
                let mut sum = Vector3::ZERO;
                for c in chunk { sum += *c; }
                sum / (chunk.len() as Float)
            })
            .collect()
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
        let n_samples = cam.num_samples as usize;
        //if cam.num_samples != 1 { warn!("Found num_samples = '{}' > 1, sampling is not implemented yet...", cam.num_samples); }
        
        // --- Rayon Multithreading ---
        let start = Instant::now();
        let eye_rays: Vec<Ray> = cam.generate_primary_rays(n_samples);
        info!("Starting ray tracing...");
        let colors: Vec<_> = eye_rays
            .par_iter()
            .map(|ray| get_color(ray, scene, &cam, 0))
            .collect();
        info!("Ray tracing completed.");
        // -----------------------------
        
        let pixel_colors = if n_samples > 1 {
            box_filter(&colors, n_samples)
        } else {
            colors
        };

        let im_raw = ImageData::new_from_colors(cam.image_resolution, cam.image_name.clone(), pixel_colors);
        
        for tonemap in cam.tone_maps.all().iter() {
            info!("Applying tone map {}", tonemap);
            let tonemapped_im = tonemap.apply(&im_raw);
            images.push(tonemapped_im);
        }

        images.push(im_raw); 
        info!("Rendering of {} took: {:?}", cam.image_name, start.elapsed()); 
    }
    
    Ok(images)
}

