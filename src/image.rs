

use serde::{self, Deserialize, de::{Deserializer}};
use std::path::{Path, PathBuf};
use std::io::BufWriter;
use std::fs::File;
use image::{GenericImageView}; // TODO: right now png crate is used to save the final image but as of hw4, this crate is added to read texture images, so mayb we can remove png crate and just use image crate?


use crate::json_structs::SingleOrVec;
use crate::prelude::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
//#[serde(default)]
pub struct Textures {
    pub images: Option<TextureImages>, // WARNING: I assume Image _id corresponds to its index in the Images vector
    pub texture_map: SingleOrVec<TextureMap>,
}

impl Textures {
    /// Given texture map index (assuming json ids are sorted and starting from 1 because I directly push them in a vector),
    /// return the color of the corresponding texture pixel from the texture (image or procedural).
    /// TODO: should we use hashmaps instead of vecs to avoid the assumption described above?
    /// uv: texture coordinates (currently only uv is supported, I am not sure how to generalize it atm) 
    pub fn get_texel_color(&self, texmap_idx: usize, uv: [usize; 2], interpolation: &Interpolation) -> Vector3 {
        //let texmap = self.texture_map.all()... --> how to access it efficiently?
        todo!()
    }
}
#[derive(Debug, Clone, Deserialize)]
//#[serde(default)]
pub struct TextureImages {
    #[serde(rename = "Image")] 
    raw_images: SingleOrVec<TextureImageHelper>,
    #[serde(skip)]
    pub data: Vec<ImageData>, 
} // Currently trying to make it similar to SceneMaterials deserialization 

#[derive(Debug, Clone, Deserialize)]
struct TextureImageHelper {

    _data: String,

    #[serde(deserialize_with = "deser_usize")]
    _id: usize,
}

impl TextureImages {
    pub fn setup(&mut self, base_dir: &Path) {
        
        let mut helpers = self.raw_images.all();

        // Sort by _id (I assume they are given sorted in jsons though maybe I should've considered hashmaps instead of vec)
        helpers.sort_by_key(|h| h._id);
        self.data = Vec::with_capacity(helpers.len());

        for helper in helpers {
            let full_path = base_dir.join(&helper._data);
            debug!("Loading texture image id={} from '{}'", helper._id, full_path.display() );

            let img = ImageData::new_from_file(&full_path); 
            self.data.push(img);
        }
    }
}

// See https://serde.rs/enum-representations.html for internally tagged representation
#[derive(Debug, Deserialize)]
#[serde(tag = "_type", rename_all = "lowercase")] // content = "content", 
enum TextureMap {
    Image(ImageTexmap),
    Perlin(PerlinTexmap),
    Empty,
}


impl Default for TextureMap {
    fn default() -> Self {
        debug!("Default for TextureMap called. Setting to Empty...");
        TextureMap::Empty
    }
}

#[derive(Debug)]
struct ImageTexmap {
   
    _id: usize, 
    image_index: usize,
    interpolation: Interpolation,
    decal_mode: DecalMode,    
    normalizer: Float,
    bump_factor: Float, 
}

impl<'de> Deserialize<'de> for ImageTexmap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, SmartDefault)]
        #[serde(rename_all = "PascalCase")]
        #[serde(default)]
        struct Helper {
            #[serde(rename = "_id", deserialize_with = "deser_usize")]
            _id: usize,
            #[serde(deserialize_with = "deser_usize")]
            image_id: usize,
            decal_mode: String,
            interpolation: String,

            #[serde(deserialize_with = "deser_float")]
            #[default = 1.0] // WARNING: I assume default for normalizer is 1.
            normalizer: Float,

            #[serde(deserialize_with = "deser_float")]
            #[default = 1.0]
            bump_factor: Float, // TODO: is it safe to assume 1 here?
        }
        debug!("Calling helper deserializer for 'Image' type texture map...");
        let h = Helper::deserialize(deserializer)?;
        debug!("Deserialized image texture map.");
        debug!("Assumes ImageId starts from 1, and subtracts 1 to store ImageTexmap.image_index...");
        Ok(ImageTexmap {
            _id: h._id,
            image_index: h.image_id - 1,
            decal_mode: parse_decal(&h.decal_mode).unwrap(),
            interpolation: parse_interp(&h.interpolation).unwrap(),
            normalizer: h.normalizer,
            bump_factor: h.bump_factor,
        })
    }
}



#[derive(Debug)]
struct PerlinTexmap {
    id: usize, 
    noise_conversion: NoiseConversion,
    decal_mode: DecalMode,
    noise_scale: Float,
    bump_factor: Float,
}

impl<'de> Deserialize<'de> for PerlinTexmap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, SmartDefault)]
        #[serde(rename_all = "PascalCase")]
        #[serde(default)]
        struct Helper {
           
            #[serde(rename = "_id", deserialize_with = "deser_usize")]
            _id: usize,

            decal_mode: String,
            noise_conversion: String,

            #[serde(deserialize_with = "deser_float")]
            #[default = 1.0]
            noise_scale: Float,

            #[serde(deserialize_with = "deser_float")]
            #[default = 1.0]
            bump_factor: Float, // TODO: is it safe to assume 1 here?
        }

        let h = Helper::deserialize(deserializer)?;

        Ok(PerlinTexmap {
            id: h._id,
            decal_mode: parse_decal(&h.decal_mode).unwrap(),
            noise_conversion: parse_noise_conversion(&h.noise_conversion).unwrap(),
            noise_scale: h.noise_scale,
            bump_factor: h.bump_factor,
        })
    }
}


#[derive(Debug)]
pub(crate) enum NoiseConversion {
    AbsoluteVal,
    Linear,
}

impl Default for NoiseConversion {
    fn default() -> Self {
        debug!("Default for NoiseConversion called. Setting to linear [0,1] range...");
        NoiseConversion::Linear      
    }
}

#[derive(Debug)]
pub(crate) enum DecalMode {
    ReplaceKd,
    BlendKd,
    ReplaceKs,
    ReplaceBackground,
    ReplaceNormal,
    BumpNormal,
    ReplaceAll,
}


impl Default for DecalMode {
    fn default() -> Self {
        debug!("Default for DecalMode called. Setting to replace_kd...");
        DecalMode::ReplaceKd      
    }
}


#[derive(Debug)]
pub(crate) enum Interpolation {
    Nearest,
    Bilinear,
    Trilinear,
}


impl Default for Interpolation {
    fn default() -> Self {
        debug!("Default for Interpolation called. Setting to nearest...");
        Interpolation::Nearest      
    }
}

/// ImageData is meant to be used while saving the final rendered image
#[derive(Debug, Clone, SmartDefault)]
pub struct ImageData {
    // WARNING: Currently width and height is assumed to represent number of pixels,
    // not accepting a measure like centimeters, that'd require DPI as well
    pixel_colors : Vec<Vector3>, // Vector of RGB per pixel
    width : usize, 
    height: usize,
    name: String, // TODO: width, height, name info actually is stored under camera as well
                  // is it wise to copy those into ImageData? I thought it is more organized this way.
}


//impl<'de> Deserialize<'de> for ImageData {
//    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//    where
//        D: Deserializer<'de>,
//    {
//        #[derive(Deserialize)]
//        struct Helper {
//            _data: String,
//            #[serde(deserialize_with = "deser_usize")]
//            _id: usize, // TODO: WARNING here I assume _id aligns with order of these images in the json file, unused field here.
//        }
//
//        let helper = Helper::deserialize(deserializer)?;
//        Ok(Self::new_from_file(helper._data))
//    }
//}



impl ImageData {
    // TODO: now that we use image crate, should we rename this module or even remove it?
    /// Read from .jpg or .png (for other supported file formats see https://docs.rs/image )
    pub fn new_from_file(path: &PathBuf) -> Self {
        
        let img = image::open(path)
            .unwrap_or_else(|e| panic!("Failed to read image '{}': {}", path.display(), e));

        let (width, height) = img.dimensions();
        let width = width as usize;
        let height = height as usize;

        // WARNING: Asusmes 8 bit RGB
        let rgb = img.to_rgb8();

        let mut pixel_colors = Vec::with_capacity(width * height);
        for chunk in rgb.as_raw().chunks_exact(3) {
            pixel_colors.push(Vector3::new(
                chunk[0] as Float,
                chunk[1] as Float,
                chunk[2] as Float,
            ));
        }

        // WARNING: Assumes the name is not the path of the image! 
        let name = std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image")
            .to_string();

        Self {
            pixel_colors,
            width,
            height,
            name,
        }
    }


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

    pub fn check_extension(&self, path: &Path, extension: &str) -> bool {
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
        let w = &mut BufWriter::new(file);
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

//////////////////////////// Sampling Pixels ///////////////////////////////////////////////////
/// TODO: can we put these functions below into a module? 
/// 


/// Uniformly get pixel centers on the nearplane (no sampling, hw1 and hw2 used this function)
pub fn get_pixel_centers(width: usize, height: usize, near_plane_corners: &[Vector3; 4]) -> Vec<Vector3> {
    // Assuming nearplane corners are:
    // [0]=top-left, [1]=top-right, [2]=bottom-left, [3]=bottom-right
    let mut pixel_centers = Vec::with_capacity(width * height);
    
    for row in 0..height {
        for col in 0..width {
            let u = (col as Float + 0.5) / width as Float; // pixel width in range [0,1] for lerp
            let v = (row as Float + 0.5) / height as Float; 
            
            let top = near_plane_corners[0] * (1.0 - u) + near_plane_corners[1] * u; // top-center
            let bottom = near_plane_corners[2] * (1.0 - u) + near_plane_corners[3] * u; // bottom-center
            let center = top * (1.0 - v) + bottom * v;
            
            pixel_centers.push(center);
        }
    }
    
    pixel_centers
}


/// Given top corner pixel coordinate and number of rows and columns,
/// push sample on the pixel to samples.
fn _jittered_sample_pixel(top_left: Vector3, pixel_w: Float, pixel_h: Float, n_rows: usize, n_cols: usize, samples: &mut Vec<Vector3>) {

    for y in 0..n_rows {
        for x in 0..n_cols {
            let psi_1: Float = random_float();
            let psi_2: Float = random_float();

            let mut sample = top_left;
            sample.x = top_left.x + ((x as Float + psi_1) / (n_cols as Float)) * pixel_w;
            sample.y = top_left.y + ((y as Float + psi_2) / (n_rows as Float)) * pixel_h;
            samples.push(sample);
        }
    }

}

/// Stratified random sampling, given nearplane corners and image resoluion (width, height)
/// See slides 05, p.40
pub fn jittered_sampling(n_samples: usize, width: usize, height: usize, nearplane_corners: &[Vector3; 4]) -> Vec<Vector3> {
    if n_samples <= 1 {
        warn!("Something is wrong! n_samples is expected to be > 1, got {}", n_samples);
    }

    // Check also this https://users.rust-lang.org/t/integer-square-root/96/10
    warn!("Assumes number of samples = {} is a perfect square.", n_samples);
    let n_rows = (n_samples as Float).sqrt() as usize;
    let n_cols = n_rows.clone();
    let mut samples: Vec<Vector3> = Vec::with_capacity(width * height * n_samples);

    for im_row in 0..height {
        for im_col in 0..width {
            
            let u = im_col as Float / width as Float; // Not adding 0.5 here because we'll use top left corner in sampling
            let v = im_row as Float / height as Float;
            let top = nearplane_corners[0] * (1.0 - u) + nearplane_corners[1] * u;
            let bottom = nearplane_corners[2] * (1.0 - u) + nearplane_corners[3] * u;
            let top_left = top * (1.0 - v) + bottom * v;

            let right_edge = nearplane_corners[1] - nearplane_corners[0];
            let bottom_edge = nearplane_corners[2] - nearplane_corners[0];
            let pixel_w = right_edge.length() / width as Float;
            let pixel_h = bottom_edge.length() / height as Float;
            _jittered_sample_pixel(top_left, pixel_w, pixel_h, n_rows, n_cols, &mut samples);
        }
    }
    
    samples
}