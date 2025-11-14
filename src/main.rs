/*

    A simple ray tracer implemented for CENG 795 course.

    @date: Oct, 2025
    @author: Bartu

*/

use std::{env, path::Path};
use tracing_subscriber;

use fury_tracer::*; // lib.rs mods
use crate::prelude::*; 
use crate::scene::Scene;

fn main()  -> Result<(), Box<dyn std::error::Error>> {

    // Logging on console
    tracing_subscriber::fmt::init(); 

    // Parse args
    let args: Vec<String> = env::args().collect();
    let json_path: &String = if args.len() == 1 {
        warn!("No arguments were provided, setting default scene path...");
        //&String::from("./inputs/hw1/scienceTree_glass.json")
        //&String::from("./inputs/hw1/akif_uslu/berserker_smooth.json")
        &String::from("./inputs/hw2/dragon_metal.json")
    } else if args.len() == 2 {
        &args[1]
    } else {
        error!("Usage: {} <filename>.json", args[0]);
        std::process::exit(1);
    };
    
    // Parse JSON
    info!("Loading scene from {}...", json_path);
    let mut root = parse_json795(json_path).map_err(|e| {
        error!("Failed to load scene: {}", e);
        Box::<dyn std::error::Error>::from(e)
    })?;

    let json_path = Path::new(json_path).canonicalize()?;

    let scene = Scene::new_from(&mut root.scene, &json_path); // TODO: This should be done in a different way
    debug!("Scene is setup successfully.\n {:#?}", scene);

    // Render images and return array of RGB
    let images = renderer::render(&scene)?;
    
    // Write images to .png files
    for im in images.into_iter() {
        let imagefolder = "./"; // Save to this folder TODO: add outputs/subfolder/... 
        if let Err(e) = im.save_png(&imagefolder) {
            eprintln!("Failed to save {}: {}", imagefolder, e);
        }
    }
    info!("Finished execution.");
    Ok(())
}
