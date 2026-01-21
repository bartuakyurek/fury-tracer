/*

    A simple ray tracer implemented for CENG 795 course.

    @date: Oct, 2025
    @author: Bartu

*/
use walkdir::WalkDir;
use std::{env, path::Path, path::PathBuf};

use fury_tracer::*; // lib.rs mods
use crate::prelude::*; 
use crate::scene::Scene;

fn main()  -> Result<(), Box<dyn std::error::Error>> {

    // Logging on console
    tracing_subscriber::fmt::init(); 

    // Parse args
    let args: Vec<String> = env::args().collect();
    let input_path: &String = if args.len() == 1 {
        warn!("No arguments were provided, setting default scene path...");
        &String::from("./inputs/hw2/mirror_room.json")
    } else if args.len() == 2 {
        &args[1]
    } else {
        error!("Usage: {} <filename>.json or <path/to/folder>", args[0]);
        std::process::exit(1);
    };
    
    let path = Path::new(&input_path);
    if path.is_file() {
        // Scenario 1: input contains JSON file
        read_json_and_render(&path.to_str().unwrap().to_string())?; // TODO: Perhaps I should make these functions accept path directly
    } else if path.is_dir() {
        // Scenario 2: input is a directory, explore all .jsons recursively
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            let entry_path = entry.path();
            let is_json = entry_path.extension().map(|s| s == "json").unwrap_or(false);
            if entry_path.is_file() && is_json {
                info!("Rendering JSON: {:?}", entry_path);
                read_json_and_render(&entry_path.to_str().unwrap().to_string())?;
            }
        }
    } else {
        error!("Expected input path to be a file or a directory, got: {:?}", path);
        std::process::exit(1);
    }

    info!("Finished execution.");
    Ok(())
}

/// Helper function for main() 
fn read_json_and_render(json_path: &String) -> Result<(), Box<dyn std::error::Error>>  {
    // Parse JSON
    debug!("Loading scene from {}...", json_path);
    let mut root = parse_json795(json_path).map_err(|e| {
        error!("Failed to load scene: {}", e);
        e
    })?;

    let json_path = Path::new(json_path).canonicalize()?;
    let scene = Scene::new_from(&mut root.scene, &json_path); 
    debug!("Scene is setup successfully.");
    
    // UPDATE: If environment variable is given, just load the json, print it and exit. ---------------------------------------------------------
    if std::env::var("JUST_LOAD").is_ok() {
        print_my_dummy_debug(&scene);
        std::process::exit(0);
    }
    // ------------------------------------------------------------------------------------------------------------------------------------------

    // Render images and return array of RGB
    let images = renderer::render(&scene)?;
    
    // Write images to .png files
    let imagefolder_pathbuf = get_output_dir(json_path, "inputs", "outputs")?;
    let imagefolder = imagefolder_pathbuf.to_str().unwrap();
    for im in images.into_iter() {
        if let Err(e) = im.export(imagefolder) {
            eprintln!("Failed to save {}: {}", imagefolder, e);
        }
    }

    Ok(())
}

fn print_my_dummy_debug(scene: &Scene) {
    //dbg!("-------------------");
    //dbg!("Texture Coords:");
    //dbg!(&scene.data.tex_coord_data);
    //dbg!(&scene.vertex_cache.uv_coords);
    //dbg!("-------------------");
    //dbg!(&scene.data.textures.as_ref().unwrap().texture_maps); // TODO https://github.com/casey/just see this one to have commands like "just print textures"
    //dbg!(&scene.data.lights);
    dbg!(&scene.data.objects.light_meshes);
    dbg!(&scene.data.objects.light_spheres);
    dbg!("-------------------");
}

/// Given the JSON file path, and its parent name ("inputs" in our case), return the output path to be used while saving .png image
/// (it doesn't include .png name, only up to its parent folder)
/// In the homeworks our input_folder = "inputs" and output_folder = "outputs"
fn get_output_dir(json_path: PathBuf, input_folder: &str, output_folder: &str) -> Result<PathBuf,  Box<dyn std::error::Error>> {
    let json_dir = json_path.parent().unwrap();  // folder of the json

    // Try to find "inputs" in the path
    let components: Vec<_> = json_dir.components().collect();
    let mut input_subpath: Option<PathBuf> = None;

    for (i, comp) in components.iter().enumerate() {
        if comp.as_os_str() == input_folder {
            // collect everything after "inputs"
            let mut p = PathBuf::new();
            for c in &components[i+1..] {
                p.push(c.as_os_str());
            }
            input_subpath = Some(p);
            break;
        }
    }
    // If no "inputs" in the path, then use whole path except json filename
    let relative_path = input_subpath.unwrap_or_else(|| {
        PathBuf::from(json_dir.file_name().unwrap())
    });

    // Check if ./outputs exists, else create it
    let outputs_root = Path::new(output_folder);
    if !outputs_root.exists() {
        std::fs::create_dir(outputs_root)?;
    }
    // Construct full directory: outputs/rest/of/the/directory
    let image_folder_path = outputs_root.join(&relative_path);
    std::fs::create_dir_all(&image_folder_path)?;
    Ok(image_folder_path)
}
