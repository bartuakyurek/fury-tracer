


use std::path::{Path, PathBuf};
use std::io::BufWriter;
use std::fs::File;

use crate::prelude::*;


#[derive(Clone)]
pub struct ImageData {
    // WARNING: Currently width and height is assumed to represent number of pixels,
    // not accepting a measure like centimeters, that'd require DPI as well
    pixel_colors : Vec<Vector3>, // Vector of RGB per pixel
    width : usize, 
    height: usize,
    name: String, // TODO: width, height, name info actually is stored under camera as well
                  // is it wise to copy those into ImageData? I thought it is more organized this way.
}


impl ImageData {

    pub fn new(width: usize, height: usize, name: String, pixel_colors: Vec<Vector3>) -> Self {
        ImageData {
            pixel_colors,
            width,
            height,
            name,
        }
    }

    pub fn new_from_colors(resolution: [usize; 2], name: String, colors: Vec<Vector3>) -> Self {
        // Create a new image of specified background color
        // Set background to Vector3::ZERO for black background
        let (width, height) = (resolution[0], resolution[1]);
        Self::new(width, height, name, colors)
    }

    pub fn new_from_background(resolution: [usize; 2], name: String, background: Vector3) -> Self {
        // Create a new image of specified background color
        // Set background to Vector3::ZERO for black background
        let (width, height) = (resolution[0], resolution[1]);
        let pixel_colors = vec![background; width * height];
        Self::new(width, height, name, pixel_colors)
    }

    pub fn flatten_color(self) -> Vec<Float> {
        // Return [R1, G1, B1, R2, G2, B2, ...] vector
        // where each triplet is RGB color of a pixel.
        self.pixel_colors.into_iter().flat_map(|v| [v.x, v.y, v.z]).collect()
    }

    /// Clamp colors and return a flattened array of R G B values per pixel 
    pub fn to_rgb(self) -> Vec<u8> {
        
        self.flatten_color().into_iter().map(|x| 
            {
            x.clamp(0.0, 255.0) as u8
            }
        ).collect()
    } 

    pub fn check_extension(&self, path: &PathBuf, extension: &str) -> bool {
        path.extension().unwrap().to_str().unwrap() == extension
    }

    pub fn get_png_fullpath(&self, path: &str) -> PathBuf {
        // Check if provided path is a folder 
        // if so, create a .png under this folder
        // otherwise use the provided path as is
        let extension = "png";
        {
            let path = Path::new(path);
            let mut finalpath: PathBuf = path.to_path_buf();
            if path.is_dir() {
                // create <imagename>.png under this directory 
                finalpath = path.join(self.name.clone());
            } 
            
            if !self.check_extension(&finalpath, extension){
                finalpath.set_extension(extension);
                warn!(">> Extension changed to .{}, final path is {}", extension, finalpath.to_str().unwrap_or("<invalid UTF-8 path>")); 
            }
            finalpath
        }
    }

    pub fn save_png(self, path: &str) -> Result<(), Box<dyn std::error::Error>>{
        // Path is either a folder name or
        // full path including <imagename>.png
        // If full path is not provided it will use 
        // stored image name.
        //
        // WARNING: Assumes RGB is used (no transparency available atm)
        // WARNING: Only png accepted for now, if specified image name has another
        // extension it will be silently converted to .png
        //
        // DISCLAIMER: This function is based on https://docs.rs/png/0.18.0/png/
        let path: PathBuf = self.get_png_fullpath(path);

        let file = File::create(path.clone()).unwrap();
        let ref mut w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, self.width as u32, self.height as u32); // Width is 2 pixels and height is 1.
    
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        // TODO / WARNING: You may need to set gamma as in this link https://docs.rs/png/0.18.0/png/
        let mut writer = encoder.write_header().unwrap();

        let data = self.to_rgb();
        writer.write_image_data(&data)?; // Save
        info!("Image saved to {}", path.to_str().unwrap());
        Ok(())
    }
}


pub fn get_pixel_centers(width: usize, height: usize, near_plane_corners: &[Vector3; 4]) -> Vec<Vector3> {
    // Assuming nearplane corners are:
    // [0]=top-left, [1]=top-right, [2]=bottom-left, [3]=bottom-right
    let mut pixel_centers = Vec::with_capacity(width * height);
    
    for row in 0..height {
        for col in 0..width {
            let u = (col as Float + 0.5) / width as Float; // pixel width
            let v = (row as Float + 0.5) / height as Float; // pixel height
            
            let top = near_plane_corners[0] * (1.0 - u) + near_plane_corners[1] * u;
            let bottom = near_plane_corners[2] * (1.0 - u) + near_plane_corners[3] * u;
            let center = top * (1.0 - v) + bottom * v;
            
            pixel_centers.push(center);
        }
    }
    
    pixel_centers
}

