

use serde::{self, Deserialize, de::{Deserializer}};
use std::path::{Path, PathBuf};
use std::io::BufWriter;
use std::fs::File;
use std::ffi::OsStr;
use image::{DynamicImage, GenericImageView, ImageBuffer}; // TODO: right now png crate is used to save the final image but as of hw4, this crate is added to read texture images, so mayb we can remove png crate and just use image crate?
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};


use crate::{json_structs::SingleOrVec, ray::HitRecord};
use crate::prelude::*;

#[derive(Debug, Clone, Deserialize)]
pub struct Textures {
    #[serde(rename = "Images")]
    pub images: Option<TextureImages>, // WARNING: I assume Image _id corresponds to its index in the Images vector

    #[serde(rename = "TextureMap", default)] // Some files do not come with TextureMap, e.g. environment lights don't need them 
    pub texture_maps: SingleOrVec<TextureMap>,

    //#[serde(skip)]
    //pub texture_maps: Vec<TextureMap>, // To avoid calling .all( ) on SingleOrVec deserialization
}


// ---------------------------------------------------------
// Perlin Gradients (slides 06, p.52)
// ---------------------------------------------------------
use std::sync::OnceLock;

static PERLIN_GRADIENTS: OnceLock<Vec<Vector3>> = OnceLock::new();

fn perlin_gradients() -> &'static Vec<Vector3> {
    PERLIN_GRADIENTS.get_or_init(|| {
        vec![
            Vector3::new( 1.,  1., 0.),
            Vector3::new(-1.,  1., 0.),
            Vector3::new( 1., -1., 0.),
            Vector3::new(-1., -1., 0.),
            Vector3::new( 1., 0., 1.),
            Vector3::new(-1., 0., 1.),
            Vector3::new( 1., 0., -1.),
            Vector3::new(-1., 0., -1.),
            Vector3::new( 0., 1., 1.),
            Vector3::new( 0., -1., 1.),
            Vector3::new( 0., 1., -1.),
            Vector3::new( 0., -1., -1.),
            Vector3::new( 1., 1., 0.),
            Vector3::new( -1., 1., 0.),
            Vector3::new( 0., -1., 1.),
            Vector3::new( 0., -1., -1.),
        ]
    })
}


/// See slides 06, p.53 
/// i, j, k represent the lattice cell corners (see p. 60)
fn perlin_table_idx(table: &Vec<Float>, i: Float, j: Float, k: Float) -> usize {
    
    let mut idx: Float = table[(k.abs() % 16.) as usize];
    idx = table[((j + idx).abs() % 16.) as usize];
    idx = table[((i + idx).abs() % 16.) as usize];

    idx as usize
} 

/// Helper function for Perlin noise interpolation,
/// i.e. f(x) in slides 06, p.48
fn perlin_interp(x: Float) -> Float {
    let x = x.abs();
    if x < 1. {
        (-6. * x.powf(5.)) + (15. * x.powf(4.)) - (10. * x.powf(3.)) + 1. 
    } else {
        0.
    }
}

// Scale determines frequency of perlin noise
fn perlin_noise(xyz: Vector3, scale: Float, noise_conversion: &NoiseConversion) -> Float {
        let mut n_prime: Float = 0.; // notation in p.49
        
        let x: Float = xyz[0] * scale; //* perlin_texmap.noise_scale;
        let y: Float = xyz[1] * scale; //* perlin_texmap.noise_scale;
        let z: Float = xyz[2] * scale; // TODO: I assumed we should use 3D noise but then realized we are in u, v space... should xyz be hitpoints directly?

        let i0 = x.floor(); // as Int;
        let j0 = y.floor();// as Int;
        let k0 = z.floor();// as Int;

        // Create table
        let mut rng = StdRng::seed_from_u64(42); // see https://rust-random.github.io/book/
        let mut table = vec![0., 1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12., 13., 14., 15.,];
        table.shuffle(&mut rng); // slides 06, p.53 "table can be shuffled prior to being used"

        // 8 corners for 3D lattice
        for di in 0..=1 {
            for dj in 0..=1 {
                for dk in 0..=1 {
                    let i = i0 + di as Float;
                    let j = j0 + dj as Float;
                    let k = k0 + dk as Float;   

                    let g = perlin_gradients()[perlin_table_idx(&table, i, j, k)]; // slides 06, p.54
                    let dx: Float = x - i;
                    let dy: Float = y - j;
                    let dz: Float = z - k;

                    let d = Vector3::new(dx, dy, dz);
                    let c: Float = perlin_interp(dx) * perlin_interp(dy) * perlin_interp(dz) * g.dot(d); // p.55 

                    n_prime += c;
                }
            }
        }

        // Return n given noise type (corresponds to options in slides 06, p.49)
        match noise_conversion {
            NoiseConversion::AbsoluteVal => {
                n_prime.abs()
            },
            NoiseConversion::Linear => {
                (n_prime + 1.) / 2.
            },
        }
}

fn perlin_octave(n_octaves: usize, xyz: Vector3, scale: Float, noise_conversion: &NoiseConversion) -> Float {
    // See hw 4 specifications, p.4 
    let mut s: Float = 0.;
    let base: Float = 2.;
    for k in 0..n_octaves {
        let fade: Float = base.powi(-(k as Int)); 
        let amplify: Float = base.powi(k as Int);   
        s += fade * perlin_noise(xyz * amplify, scale, noise_conversion); 
    }

    s
}
// ---------------------------------------------------------

impl Textures {
    /// Given texture map index (assuming json ids are sorted and starting from 1 because I directly push them in a vector),
    /// return the color of the corresponding texture pixel from the texture (image or procedural).
    /// TODO: should we use hashmaps instead of vecs to avoid the assumption described above?
    /// uv: texture coordinates (currently only uv is supported, I am not sure how to generalize it atm) 
    pub fn get_texture_color(&self, texmap_idx: usize, uv: [Float; 2], interpolation: &Interpolation, apply_normalization: bool, xyz: Vector3) -> Vector3 {
        
        let texmap = self.texture_maps.all_ref()[texmap_idx];
        match texmap {
            TextureMap::Image(image_texmap) => {
                let images = self.images
                    .as_ref()
                    .expect("Image texture is required but no Images section found");

                let image = &images.data[image_texmap.image_index];

                debug_assert!(uv[0] <= 1.0 && uv[1] <= 1.0, "Failed condition (u, v) <= 1, found uv : ({}, {})", uv[0], uv[1]);
                debug_assert!(uv[0] >= 0.0 && uv[1] >= 0.0, "Failed (u, v) >= 0, found uv : ({}, {})", uv[0], uv[1]);

                let (col, row) = (uv[0] * image.width as Float, uv[1] * image.height as Float); // image coordinate (see slides 06, p.8)
                let color = image.interpolate(row, col, interpolation);
                if apply_normalization {
                    //if image_texmap.normalizer > 1. { // TODO: This isn't a real solution but it avoids exploded radiance in some cases (but makes the image appear darker, so it's not really solving the real issue)
                        color / image_texmap.normalizer 
                    //} else {
                     //   color // 255.
                    //}
                    //color / 255.  // By default divide by 255
                } else {
                    color
                }
            },
            TextureMap::Perlin(perlin_texmap) => {
                
                let n = perlin_octave(perlin_texmap.num_octaves, xyz, perlin_texmap.noise_scale, &perlin_texmap.noise_conversion);
                Vector3::new(n, n, n)  // Turn n into color (grayscale I assume here)
            },
            TextureMap::Checkerboard(checker_texmap) => {
                // For notation, see hw4 specifications, p.4
                let scale = checker_texmap.scale;
                let offset = checker_texmap.offset;
                let x: bool = ((((xyz[0] + offset) * scale).floor() as Int) % 2) != 0;
                let y: bool = ((((xyz[1] + offset) * scale).floor() as Int) % 2) != 0;
                let z: bool = ((((xyz[2] + offset) * scale).floor() as Int) % 2) != 0;

                let xor_xy: bool = x != y;
                if xor_xy != z {
                    checker_texmap.black
                } else {
                    checker_texmap.white
                }
            },
            _ => {
                todo!("I am not ready to get texel color of this texmap type '{:?}' yet...", texmap);
            }
        }
        
    }

    

    pub fn get_bump_mapping(&self, texmap: &TextureMap, hit_record: &HitRecord) -> Vector3 {
        match texmap {
            TextureMap::Image(image_texmap) => {

                let mut dp_du = hit_record.tbn_matrix.unwrap().x_axis; // T and B vectors (see slides 07, p.13)
                let mut dp_dv = hit_record.tbn_matrix.unwrap().y_axis; 
                debug_assert!(!dp_du.is_nan());
                debug_assert!(!dp_dv.is_nan());

                let tb_dot = dp_du.dot(dp_dv);
                if !approx_zero(tb_dot){
                    //warn!("TBN matrix not orthonormal, correcting it...");
                    let n = hit_record.tbn_matrix.unwrap().z_axis;
                    dp_du = (dp_du - (n * n.dot(dp_du))).normalize();
                    dp_dv = dp_du.cross(n);
                }
                let tb_dot = dp_du.dot(dp_dv);
                assert!( approx_zero(tb_dot), "Received non-orthonormal TBN, dot product is non zero: {}", tb_dot);

                let nuv = dp_dv.cross(dp_du).normalize(); // slides 07, p.24
                debug_assert!(nuv.is_normalized());
                debug_assert!(dp_du.is_normalized());
                debug_assert!(dp_dv.is_normalized());
                debug_assert!(hit_record.normal.is_normalized());

                let images_ref = &self.images.as_ref().expect("Image texture is required but no Images found");
                let img = &images_ref.data[image_texmap.image_index];
                let delta_u = 1. / img.width as Float;
                let delta_v = 1. / img.height as Float; // slides 07, p.27

                let (u, v) = (hit_record.texture_uv.unwrap()[0], hit_record.texture_uv.unwrap()[1]);

                // Helper height function for images (as Perlin noise will differ... I am not sure if this needs to be relocated)
                fn height(u: Float, v: Float, img: &ImageData, interp: &Interpolation, normalizer: Float, bump_factor: Float) -> Float {
                    let mut row: f64 = v * img.height as Float;
                    let mut col = u * img.width as Float;
                    row = row.min((img.width - 1) as Float);
                    col = col.min((img.height - 1) as Float); // Clamping to be safe

                    let c = img.interpolate(row, col, interp);
                    let gray = (c.x + c.y + c.z) / 3.;
                    
                    (gray / 255.) * bump_factor / normalizer * 2. // TODO: Somehow multiplying by 2 here gets the expected render but idk how... 
                }   
                
                let interp_choice = &image_texmap.interpolation; //Interpolation::Bilinear; // TODO: is it ok?
                let nzr = image_texmap.normalizer;
                let bf = image_texmap.bump_factor; 
                
                let h_uv = height(u, v, img, &interp_choice, nzr, bf);
                debug_assert!(h_uv >= 0.0 && h_uv <= 1.0);

                debug_assert!(!approx_zero(delta_u), "Expected nonzero delta_u, found: {}", delta_u);
                debug_assert!(!approx_zero(delta_v), "Expected nonzero delta_v, found: {}", delta_v);
                let dh_du = (height(u + delta_u, v, img, &interp_choice, nzr, bf) - h_uv) / delta_u;
                let dh_dv = (height(u, v + delta_v, img, &interp_choice, nzr, bf) - h_uv) / delta_v; // slides 07, p.27

                let dq_du = dp_du + (dh_du * nuv); //+ (h_uv); // here ignore partial derivatives of surface normal (slides 07, p.28)
                let dq_dv = dp_dv + (dh_dv * nuv); //+ (h_uv); // slides 07, p.26

                let new_normal = dq_dv.cross(dq_du); // new surface normal (slides 07, p.25)
                assert!(!new_normal.is_nan(), "Found nan vector, new_normal = {}, dq_dv = {}, dq_du = {}", new_normal, dq_dv, dq_du);
                new_normal.normalize()
            },
            TextureMap::Perlin(perlin_texmap) => {

                let n = hit_record.normal;
                let p = hit_record.hit_point;

                // Get height function (slides 07, p.29)
                let scale = perlin_texmap.noise_scale;
                let conv = &perlin_texmap.noise_conversion;
                let h = perlin_octave(perlin_texmap.num_octaves, p, scale, conv); 

                //let (pu, pv) = (hit_record.texture_uv.unwrap()[0], hit_record.texture_uv.unwrap()[1]);
                //let dp_du = hit_record.tbn_matrix.unwrap().x_axis; // T and B vectors (see slides 07, p.13)
                //let dp_dv = hit_record.tbn_matrix.unwrap().y_axis; 
                // Compute gradients, last term in p.29 ignored (see p.30)
                // let dq_du = dp_du + (dh_du) * n; 
                //let dq_dv = dp_dv + (dh_dv) * n; 
                //let new_normal = dq_dv.cross(dq_du);

                // Gradient of Perlin noise (07, p.36)
                let epsilon: Float = 0.001;
                let nabla_h = {   
                    // Perturb along x, y, z individually
                    let (mut p_xnudge, mut p_ynudge, mut p_znudge) = (p, p, p);
                    p_xnudge[0] += epsilon;
                    p_ynudge[1] += epsilon;
                    p_znudge[2] += epsilon;
                    // Compute partials (p.36)
                    let dh_dx = (perlin_noise(p_xnudge, scale, conv) - h) / epsilon;
                    let dh_dy = (perlin_noise(p_ynudge, scale, conv) - h) / epsilon;
                    let dh_dz = (perlin_noise(p_znudge, scale, conv) - h) / epsilon;
                    Vector3::new(dh_dx, dh_dy, dh_dz)
                }; 

                // Surface gradient (07, p.35)
                // I assume gradient of height field is the same as gradient of perlin noise here
                let g = nabla_h;
                let g_parallel = (g.dot(n)) * n;
                let g_perp = g - g_parallel; // This is the surface gradient in p.34

                let new_normal = n - (g_perp * perlin_texmap.bump_factor); //  Following p.34
                new_normal.normalize()
            },
            _ => {
              todo!("I am not ready for the bump mapping of this texmap type '{:?}' yet...", texmap);
            },
        }
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
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "_type", rename_all = "lowercase")] // content = "content", 
pub enum TextureMap {
    Image(ImageTexmap),
    Perlin(PerlinTexmap),
    Checkerboard(CheckerTexmap),
    Empty,
}


impl Default for TextureMap {
    fn default() -> Self {
        debug!("Default for TextureMap called. Setting to Empty...");
        TextureMap::Empty
    }
}

impl TextureMap {
    pub fn decal_mode(&self) -> Option<&DecalMode> {
        match self {
            TextureMap::Image(img) => Some(&img.decal_mode),
            TextureMap::Perlin(perlin) => Some(&perlin.decal_mode),
            TextureMap::Checkerboard(checker) => Some(&checker.decal_mode),
            TextureMap::Empty => None,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            TextureMap::Image(img) => img._id - 1, // Assuming id starts from 1 and index assumes 0 start
            TextureMap::Perlin(perlin) => perlin.id - 1,
            TextureMap::Checkerboard(checker) => checker._id - 1,
            TextureMap::Empty => panic!("Empty TextureMap received, index unknown!"),
        }
    }

    pub fn interpolation(&self) -> Option<&Interpolation> {
        match self {
            TextureMap::Image(img) => Some(&img.interpolation), // Only image textures have interpolation I assume
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct CheckerTexmap {
    _id: usize,
    black: Vector3,
    white: Vector3,
    scale: Float,
    offset: Float,
    decal_mode: DecalMode,
}


impl<'de> Deserialize<'de> for CheckerTexmap {
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
            #[serde(deserialize_with = "deser_vec3")]
            black_color: Vector3,
            #[serde(deserialize_with = "deser_vec3")]
            white_color: Vector3,
            #[serde(deserialize_with = "deser_float")]
            #[default = 1.]
            scale: Float,            
            #[serde(deserialize_with = "deser_float")]
            offset: Float,
            decal_mode: String,
        }
        debug!("Calling helper deserializer for 'Checkerboard' type texture map...");
        let h = Helper::deserialize(deserializer)?;
        debug!("Deserialized checkerboard texture map.");
        Ok(CheckerTexmap {
            _id: h._id,
            black: h.black_color,
            white: h.white_color,
            scale:  h.scale,
            offset: h.offset,
            decal_mode: parse_decal(&h.decal_mode).unwrap(),
        })
    }
}



#[derive(Debug, Clone)]
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
            #[default = 255.] // WARNING: I assume default for normalizer is 255 ("by default divide the texture value by 255")
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



#[derive(Debug, Clone)]
struct PerlinTexmap {
    id: usize, 
    noise_conversion: NoiseConversion,
    decal_mode: DecalMode,
    noise_scale: Float,
    bump_factor: Float,
    num_octaves: usize,
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

            #[serde(deserialize_with = "deser_usize")]
            #[default = 1]
            num_octaves: usize,
        }

        let h = Helper::deserialize(deserializer)?;

        Ok(PerlinTexmap {
            id: h._id,
            decal_mode: parse_decal(&h.decal_mode).unwrap(),
            noise_conversion: parse_noise_conversion(&h.noise_conversion).unwrap(),
            noise_scale: h.noise_scale,
            bump_factor: h.bump_factor,
            num_octaves: h.num_octaves,
        })
    }
}


#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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


#[derive(Debug, Clone)]
pub(crate) enum Interpolation {
    Nearest,
    Bilinear,
    Trilinear,
}


//impl Default for Interpolation {
//    fn default() -> Self {
//        debug!("Default for Interpolation called. Setting to nearest...");
//        Interpolation::Nearest      
//    }
//}

impl Interpolation {
    pub const DEFAULT: Interpolation = Interpolation::Nearest;
}


/// ImageData is meant to be used while saving the final rendered image
#[derive(Debug, Clone, SmartDefault)]
pub struct ImageData {
    // WARNING: Currently width and height is assumed to represent number of pixels,
    // not accepting a measure like centimeters, that'd require DPI as well
    pixel_radiance : Vec<Vector3>, // Vector of RGB per pixel
    width : usize, 
    height: usize,
    name: String, // TODO: width, height, name info actually is stored under camera as well
                  // is it wise to copy those into ImageData? I thought it is more organized this way.
}

impl ImageData {
    // TODO: now that we use image crate, should we rename this module or even remove it?
    /// Read from .jpg or .png (for other supported file formats see https://docs.rs/image )
    pub fn new_from_file(path: &PathBuf) -> Self {
        
        let img = image::open(path)
            .unwrap_or_else(|e| panic!("Failed to read image '{}': {}", path.display(), e));
        info!("Reading image file from {:?}...", path);
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

        debug!("Loading ImageData from {}... with dimensions ({}, {})", path.display(), width, height);
        Self {
            pixel_radiance: pixel_colors,
            width,
            height,
            name,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn new(width: usize, height: usize, name: String, pixel_colors: Vec<Vector3>) -> Self {
        ImageData {
            pixel_radiance: pixel_colors,
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

    pub fn flatten_data(self) -> Vec<Float> {
        // Return [R1, G1, B1, R2, G2, B2, ...] vector
        // where each triplet is RGB color of a pixel.
        self.pixel_radiance.into_iter().flat_map(|v| [v.x, v.y, v.z]).collect()
    }

    pub fn fetch_color(&self, row: usize, col: usize) -> Vector3 {
        debug_assert!(row < self.height, "Row {} must be smaller than image height {}", row, self.height);
        debug_assert!(col < self.width, "Column {} must be smaller than image width {}", col, self.width);
        self.pixel_radiance[(row * self.width) + col]
    }

    /// Clamp colors and return a flattened array of R G B values per pixel 
    pub fn to_rgb(self) -> Vec<u8> {
        
        self.flatten_data().into_iter().map(|x| 
            {
            x.clamp(0.0, 255.0) as u8
            }
        ).collect()
    } 

    pub fn check_extension(&self, path: &Path, extension: &str) -> bool {
        path.extension().unwrap().to_str().unwrap() == extension
    }

     pub fn update_extension(&mut self, new_suffix_ext: &str) {
        let path = Path::new(&self.name);

        // Filename without extension
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.name);

        self.name = format!("{}{}", stem, new_suffix_ext);
    }

    pub fn get_fullpath(&self, path: &str) -> PathBuf {
        // Check if provided path is a folder 
        // if so, create a .png under this folder
        // otherwise use the provided path as is
        
        let path = Path::new(path);
        let mut finalpath: PathBuf = path.to_path_buf();
        if path.is_dir() {
            // create <imagename>.png under this directory 
            finalpath = path.join(self.name.clone());
        } 
    
        finalpath
    }

    pub fn export(self, path: &str) -> Result<(), Box<dyn std::error::Error>>{
        // Path is either a folder name or full path including <imagename>.png
        // If full path is not provided it will use  stored image name.
        let path: PathBuf = self.get_fullpath(path);
        let img_extension = path.extension().unwrap().to_str().unwrap();
        
        let im_buffer = match img_extension {
            "png" | "jpg" | "jpeg" => { self.get_ldr() }
            "exr" | "hdr"  => { self.get_hdr() }
            _ => { panic!("Invalid image extension {}", img_extension) }
        };

        im_buffer.save(&path)?;
        info!("Image saved to {}", path.display());

        Ok(())
    }

    fn get_ldr(&self) -> DynamicImage {
        // Note: No tone mapping here
        let mut im_buffer = image::RgbImage::new(self.width as u32, self.height as u32);
        for (i, pixel) in im_buffer.pixels_mut().enumerate() {
            let radiance = self.pixel_radiance[i];

            *pixel = image::Rgb([
                (radiance.x).clamp(0.0, 255.0) as u8,
                (radiance.y).clamp(0.0, 255.0) as u8,
                (radiance.z).clamp(0.0, 255.0) as u8,
            ]);
        }
        DynamicImage::ImageRgb8(im_buffer)
    }

    fn get_hdr(&self) -> DynamicImage {
    
        let mut img = image::Rgb32FImage::new(self.width as u32, self.height as u32);

        for (i, pixel) in img.pixels_mut().enumerate() {
            let c = self.pixel_radiance[i];
            *pixel = image::Rgb([c.x as f32, c.y as f32, c.z as f32]);
        }

        DynamicImage::ImageRgb32F(img)
    }

    /// Given image coordinates (i, j), and interpolation choice, 
    /// retrieve the image color (note that i, j can be fractional, see slides 06, p.8)
    pub fn interpolate(&self, row: Float, col: Float,  style: &Interpolation) -> Vector3 {

        debug_assert!(col < self.width as Float);
        debug_assert!(row < self.height as Float);
       
        match style {
            Interpolation::Nearest => {
                self.lerp(row, col)
            },
            Interpolation::Bilinear => {
                self.bilinear(row, col)
            },
            Interpolation::Trilinear => {
                self.trilinear(row, col)
            }
        }
    }

    fn lerp(&self, i: Float, j: Float) -> Vector3 {
        // see slides 06, p.9
        let mut row = i.round() as usize;
        let mut col = j.round() as usize;
        // TODO: Clamping resolves out of bounds error but I wonder I'm just silencing a bug here...
        row = row.min(self.height - 1); 
        col = col.min(self.width - 1);
        self.fetch_color(row, col)
    }

    fn bilinear(&self, i: Float, j: Float) -> Vector3 {

        debug_assert!(i > 0.0);
        debug_assert!(j > 0.0);

        // see slides 06, p.9
        let p = i.floor() as usize;
        let q = j.floor() as usize;
        let dx = i - p as Float;
        let dy = j - q as Float;

        let p_next = (p + 1).min(self.height - 1); // TODO: Clamping resolved index out of bounds error but is this even correct?
        let q_next = (q + 1).min(self.width - 1);

        self.fetch_color(p, q) * (1. - dx) * (1. - dy) +
        self.fetch_color(p_next, q) * (dx) * (1. - dy) +
        self.fetch_color(p, q_next) * (1. - dx) * (dy) +
        self.fetch_color(p_next, q_next) * (dx) * (dy) 
    }

    fn trilinear(&self, i: Float, j: Float) -> Vector3 {
        todo!("Trilinear interpolation to be implemented...");
    }

    /// See slides 07, p.9
    pub fn color_to_direction(rgb: Vector3) -> Vector3 {
        let dir = (rgb / 127.5) - Vector3::ONE;
        //dir.map(|x| x.min(1.0).max(-1.0)) // Clamping in range [-1, 1] (note: it didnt solve acne)
        dir.normalize() // Don't forget to normalize ...
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