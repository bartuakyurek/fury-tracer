/*

    Declare Camera and its related structs like NearPlane
    
    @date: Oct, 2025
    @author: bartu
*/


use crate::prelude::*;
use crate::{image, ray::Ray};
use crate::json_structs::{SingleOrVec, Transformations};

#[derive(Debug, Deserialize, Default)]
pub struct Cameras {
    #[serde(rename = "Camera")]
    camera: SingleOrVec<Camera>, // Allow either single cam (as in test.json) or multiple cams
}

impl Cameras {
    /// Always returns a Vec<Camera> regardless of JSON being a single object or array
    pub fn all(&self) -> Vec<Camera> {
        self.camera.all()
    }
}

#[derive(Debug, Deserialize, Clone)]
#[derive(SmartDefault)]
#[serde(default)]
pub struct Camera {
    #[serde(rename = "_id", deserialize_with = "deser_int")]
    _id: Int,
    
    #[default = ""]
    _type: String,

    #[serde(rename = "Position", deserialize_with = "deser_vec3")]
    position: Vector3,

    #[serde(rename = "Gaze", deserialize_with = "deser_vec3")]
    gaze_dir: Vector3,

    #[serde(rename = "GazePoint", deserialize_with = "deser_vec3")]
    gaze_point: Vector3, // To be used if _type = "lookAt"

    #[serde(rename = "Up", deserialize_with = "deser_vec3")]
    up: Vector3,

    #[serde(rename = "FovY", deserialize_with = "deser_float")]
    fovy: Float,

    #[serde(rename = "NearPlane", deserialize_with = "deser_nearplane")]
    pub(crate) nearplane: NearPlane,

    #[serde(rename = "NearDistance", deserialize_with = "deser_float")]
    near_distance: Float,

    #[serde(rename = "ImageResolution", deserialize_with = "deser_pair")]
    pub image_resolution: [usize; 2],  

    #[serde(rename = "ImageName")]
    pub image_name: String,

    #[default = 1]
    #[serde(rename = "NumSamples", deserialize_with = "deser_int")]
    pub num_samples: Int,

    #[serde(rename = "Transformations")]
    pub(crate) transformation_names: Option<String>,

    #[serde(skip)]
    pub(crate) composite_mat: Matrix4,

    #[serde(skip)]
    w : Vector3,

    #[serde(skip)]
    v : Vector3,

    #[serde(skip)]
    u : Vector3,

}

impl Camera {
    //pub fn new(id: Int, position: Vector3, gaze: Vector3, up: Vector3, nearplane: NearPlane, near_distance: Float, image_resolution: [usize; 2], image_name: String, num_samples: Int) -> Self {
    //    let mut cam = Camera {
    //        _id: id,
    //        position,
    //        gaze_dir: gaze,
    //        up,
    //        nearplane,
    //        near_distance,
    //        image_resolution,
    //        image_name,
    //        num_samples,
    //        w : Vector3::NAN,
    //        v : Vector3::NAN,
    //        u : Vector3::NAN,
    //    };
    //    cam.setup();
    //    cam
    //}
    pub fn setup(&mut self, transforms: &Transformations) {
        // Compute w, v, u vectors
        // assumes Gaze and Up is already provided during creation
        // corrects Up vector if given Up was not perpendicular to
        // Gaze vector.

         self.composite_mat = if self.transformation_names.is_some() {
                parse_transform_expression(
                    self.transformation_names.as_deref().unwrap_or(""),
                    transforms,  
                )
        } else {
            debug!("No transformation matrix found for camera, defaulting to Identity...");
            Matrix4::IDENTITY
        };

        if self._type == "lookAt" {
            info!("Found camera _type = lookAt, constructing nearplane...");
            // (From h1.pdf) You can fnd the gaze direction by subtracting the camera position from this gaze point
            self.gaze_dir = self.gaze_point - self.position;
           
            // (From hw1.pdf) FovY parameter specifies the field of view in **degrees** that the image plane 
            // covers in its vertical direction. The aspect ratio is implicitly defined by the resolution of the image plane.
            let fovy_rad = self.fovy.to_radians();
            let aspect = self.image_resolution[0] as Float / self.image_resolution[1] as Float;
            let top = self.near_distance * (fovy_rad / 2.0).tan();
            let bottom = -top;
            let right = top * aspect as Float;
            let left = -right;
            self.nearplane = NearPlane::new(left, right, bottom, top);
        }
        
        self.w = -self.gaze_dir.normalize();
        self.u = self.up.cross(self.w).normalize(); 
        self.v = self.w.cross(self.u).normalize();  // directly use corrected up
        
        // Apply transformations
        self.position = transform_point(&self.composite_mat, &self.position);
        self.w = transform_dir(&self.composite_mat, &self.w); //.normalize(); -- this doesn't let camera to zoom in under scaling
        self.u = transform_dir(&self.composite_mat, &self.u); //.normalize();
        self.v = transform_dir(&self.composite_mat, &self.v);//.normalize();
        
        debug_assert!(approx_zero(self.u.dot(self.w))); 
        debug_assert!(approx_zero(self.v.dot(self.w))); 
        debug_assert!(approx_zero(self.v.dot(self.u))); 
        debug!("Camera position {:#?}", self.position);
        //debug!("Nearplane corners are {:#?}", &self.get_nearplane_corners());
    }

    pub fn get_resolution(&self) -> (usize, usize) {
        (self.image_resolution[0], self.image_resolution[1])
    }

    pub fn get_nearplane_corners(&self) -> [Vector3; 4] {
        self.nearplane.corners(self.position, self.u, self.v, self.w, self.near_distance)
    }

    pub fn get_position(&self) -> Vector3 {
        self.position
    }

    pub fn generate_primary_rays(&self) -> Vec<Ray> {
        let (width, height) = self.get_resolution();
        let pixel_centers = image::get_pixel_centers(width, height, &self.get_nearplane_corners()); 
        let ray_origin = self.position;
        let mut rays = Vec::<Ray>::with_capacity(pixel_centers.len());
        for pixel_center in pixel_centers.iter() {            
            let direction = (pixel_center - ray_origin).normalize(); 
            rays.push(Ray::new(ray_origin, direction));
        }
        rays
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct NearPlane {
    #[serde(deserialize_with = "deser_float")]
    pub(crate) left: Float,
    #[serde(deserialize_with = "deser_float")]
    pub(crate) right: Float,
    #[serde(deserialize_with = "deser_float")]
    pub(crate) bottom: Float,
    #[serde(deserialize_with = "deser_float")]
    pub(crate) top: Float,
}

impl NearPlane {
    pub fn new(left: Float, right: Float, bottom: Float, top: Float) -> Self {
        NearPlane { 
            left,
            right,
            bottom,
            top,
        }
    }

    /// Returns the four corners in world space using camera basis vectors
    /// Order: [top-left, top-right, bottom-left, bottom-right]
    pub fn corners(
        &self,
        camera_position: Vector3,
        u: Vector3,  // camera's right vector
        v: Vector3,  // camera's up vector
        w: Vector3,  // camera's backward vector (-gaze)
        near_distance: Float
    ) -> [Vector3; 4] {
        // Center of near plane in world space
        let plane_center = camera_position - w * near_distance; // subtract because w points backward
        
        [
            plane_center + u * self.left + v * self.top,      // top-left
            plane_center + u * self.right + v * self.top,     // top-right
            plane_center + u * self.left + v * self.bottom,   // bottom-left
            plane_center + u * self.right + v * self.bottom,  // bottom-right
        ]
    }
}


//#[cfg(test)]
//mod tests {
//    use super::*; // access to the outer scope
//
//    #[test]
//    fn test_setup() {
//
//        let cam = Camera::new(
//            1,
//            Vector3::new(0., 0., 0.),
//            Vector3::new(0., 0.2, -10.), // Not perpendicular to up
//            Vector3::new(0., 1., 0.),
//            NearPlane::new(-1., 1., -1., 1.),
//            10.0,
//            [720, 720],
//            "test.png".to_string(),
//            1,
//        );
//        assert!(approx_zero(cam.u.dot(cam.v))); 
//        assert!(approx_zero(cam.v.dot(cam.w))); 
//        assert!(approx_zero(cam.w.dot(cam.u))); 
//        // These asserts are redundant with debug_asserts in new( )
//        // but keeping them here just for sanity checks.
//
//    }
//}