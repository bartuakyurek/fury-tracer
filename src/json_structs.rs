/*

    Declare data structs needed to parse JSON. 

    - DataField: To be used in Mesh and Faces
    - SingleOrVec
    - VertexData: Type alias of DataField<Vector3>

    @date: 13 Oct, 2025
    @author: Bartu
*/

use serde::{self, Deserialize, de::{Deserializer}};
use tracing_subscriber::registry::Data;
use std::{ops::Index, str::FromStr};
use tracing::{warn};
use void::Void;

use crate::json_parser::{deser_vertex_data, deser_usize_vec, parse_string_vecvec3};
use crate::geometry::rodrigues_rotation;
use crate::prelude::*;

#[derive(Copy, Clone)]
pub enum TransformKind {
    Translation,
    Rotation,
    Scaling,
    Composite,
}


#[derive(Debug, Clone)]
pub struct Transformations { // To store global transformation in the scene
    pub(crate) translation: SingleOrVec<TransformField>,
    pub(crate) rotation: SingleOrVec<TransformField>,
    pub(crate) scaling: SingleOrVec<TransformField>,
    pub(crate) composite: SingleOrVec<TransformField>,
}

impl Default for Transformations {
    fn default() -> Self {
        Self {
            translation: SingleOrVec::Empty,
            rotation: SingleOrVec::Empty,
            scaling: SingleOrVec::Empty,
            composite: SingleOrVec::Empty,
        }
    }
}

impl Transformations {

    pub fn find_translation(&self, id: usize) -> Option<&TransformField> {
        debug!("Searching for translation with id: {}", id);
        debug!("Available translations: {:?}", self.translation);
        self.translation.iter().find(|t| t._id == id)
    }

    pub fn find_scaling(&self, id: usize) -> Option<&TransformField> {
        debug!("Searching for scaling with id: {}", id);
        debug!("Available scalings: {:?}", self.scaling);
        self.scaling.iter().find(|s| s._id == id)
    }

    pub fn find_rotation(&self, id: usize) -> Option<&TransformField> {
        debug!("Searching for rotation with id: {}", id);
        debug!("Available rotations: {:?}", self.rotation);
        self.rotation.iter().find(|r| r._id == id)
    }

    pub fn find_composite(&self, id: usize) -> Option<&TransformField> {
        debug!("Searching for composite with id: {}", id);
        debug!("Available composites: {:?}", self.composite);
        self.composite.iter().find(|c| c._id == id)
    }
}


// To be used by Translation, Rotation, Scaling
#[derive(Debug, Clone)]
pub struct TransformField {
    pub(crate) _data: Vec<Float>,
    pub(crate) _id: usize,
}


impl<'de> Deserialize<'de> for TransformField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(deserialize_with = "deser_float_vec")]
            _data: Vec<Float>,
            #[serde(deserialize_with = "deser_usize")]
            _id: usize,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(TransformField {
            _data: helper._data,
            _id: helper._id,
        })
    }
}

impl TransformField {
    pub fn get_mat4(&self, kind: TransformKind) -> Matrix4 {
        match kind {
            TransformKind::Translation => {
                let v = &self._data;
                Matrix4::from_translation(Vector3::new(v[0], v[1], v[2]))
            }

            TransformKind::Scaling => {
                let v = &self._data;
                Matrix4::from_scale(Vector3::new(v[0], v[1], v[2]))
            }

            TransformKind::Rotation => {
                let d = &self._data;
                let angle = d[0].to_radians(); // WARNING: Assumes first item in Rotation was in degrees
                let axis = Vector3::new(d[1], d[2], d[3]);
                let rot3 = rodrigues_rotation(&axis, angle);
                Matrix4::from_cols(
                    rot3.col(0).extend(0.0),
                    rot3.col(1).extend(0.0),
                    rot3.col(2).extend(0.0),
                    Vector3::ZERO.extend(1.0),
                )
            }

            TransformKind::Composite => {
                let d = &self._data;
                Matrix4::from_cols_array(&[
                    d[0], d[4], d[8],  d[12],
                    d[1], d[5], d[9],  d[13],
                    d[2], d[6], d[10], d[14],
                    d[3], d[7], d[11], d[15],
                ])
            }
        }
    }
}


// To be used for VertexData and Faces in JSON files
#[derive(Debug, Clone, Default)]
pub struct DataField<T> {
    
    pub(crate) _data: Vec<T>,
    pub(crate) _type: String,
    pub(crate) _ply_file: String,
}

impl<T> Index<usize> for DataField<T> {
    // To access data through indexing like
    // let some_field = DataField::default()
    // some_field[i] = ...
    // instead of some_field._data[i]
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self._data[index]
    }
}
impl<'de> Deserialize<'de> for DataField<Vector3> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(rename = "_data", default, deserialize_with = "deser_vertex_data")]
            _data: Vec<Vector3>,
            #[serde(rename = "_type", default)]
            _type: String,
            #[serde(rename = "_plyFile", default)]
            _ply_file: String,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(DataField {
            _data: helper._data,
            _type: helper._type,
            _ply_file: helper._ply_file,
        })
    }
}

impl<'de> Deserialize<'de> for DataField<usize> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(rename = "_data", default, deserialize_with = "deser_usize_vec")]
            _data: Vec<usize>,
            #[serde(rename = "_type", default)]
            _type: String,
            #[serde(rename = "_plyFile", default)]
            _ply_file: String,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(DataField {
            _data: helper._data,
            _type: helper._type,
            _ply_file: helper._ply_file,
        })
    }
}

impl<'de> Deserialize<'de> for DataField<Float> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(rename = "_data", default, deserialize_with = "deser_float_vec")]
            _data: Vec<Float>,
            #[serde(rename = "_type", default)]
            _type: String,
            #[serde(rename = "_plyFile", default)]
            _ply_file: String,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(DataField {
            _data: helper._data,
            _type: helper._type,
            _ply_file: helper._ply_file,
        })
    }
} // TODO: These deserializations are boilerplate (and _plyFile is not even used for e.g. TextureCoords, but I'm not sure how to re-use DataField properly atm)


// To handle JSON file having a single <object>
// or an array of <object>s 
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum SingleOrVec<T> {
    Empty,
    Single(T),
    Multiple(Vec<T>),
}

impl<T: Clone> SingleOrVec<T>  {

    pub fn all(&self) -> Vec<T> { // WARNING: It clones data to create vecs
        match &self {
            SingleOrVec::Empty => vec![],
            SingleOrVec::Single(t) => vec![t.clone()],
            SingleOrVec::Multiple(vec) => vec.clone(),
        }
    }

    pub fn all_mut(&mut self) -> Vec<&mut T> {
        match self {
            SingleOrVec::Empty => vec![],
            SingleOrVec::Single(t) => vec![t],
            SingleOrVec::Multiple(vec) => vec.iter_mut().collect(),
        }
    }

    pub fn all_ref(&self) -> Vec<&T> {
        match self {
            SingleOrVec::Empty => vec![],
            SingleOrVec::Single(t) => vec![t],
            SingleOrVec::Multiple(vs) => vs.iter().collect(),
        }
    }

    pub fn push(&mut self, item: T) {
        match self {
            SingleOrVec::Empty => {
                *self = SingleOrVec::Single(item);
            }
            SingleOrVec::Single(v) => {
                let old = std::mem::replace(v, item);
                *self = SingleOrVec::Multiple(vec![old, v.clone()]);
            }
            SingleOrVec::Multiple(vs) => vs.push(item),
        }
    }
    
     pub fn as_slice(&self) -> &[T] {
        match self {
            SingleOrVec::Empty => &[],
            SingleOrVec::Single(v) => std::slice::from_ref(v),
            SingleOrVec::Multiple(vec) => vec.as_slice(),
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self {
            SingleOrVec::Empty => &mut [],
            SingleOrVec::Single(v) => std::slice::from_mut(v),
            SingleOrVec::Multiple(vs) => vs.as_mut_slice(),
        }
    }
    
    /// create iterator (borrows, read-only access)
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_slice().iter()
    }

    /// create mutable iterator
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        match self {
            SingleOrVec::Empty => [].iter_mut(),
            SingleOrVec::Single(v) => std::slice::from_mut(v).iter_mut(),
            SingleOrVec::Multiple(vec) => vec.iter_mut(),
        }
    }

     pub fn len(&self) -> usize {
        match self {
            SingleOrVec::Empty => 0,
            SingleOrVec::Single(_) => 1,
            SingleOrVec::Multiple(vs) => vs.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    
}

impl<T> Default for SingleOrVec<T> {
    fn default() -> Self {
        debug!("Implementing default for SingleOrVec...");
        SingleOrVec::Empty
    }
}

 
#[derive(Deserialize)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Deserialize)]
pub struct Face {
    #[serde(rename = "vertex_index", alias = "vertex_indices")]
    pub vertex_indices: Vec<usize>, 
}

#[derive(Deserialize)]
pub struct PlyMesh {
    pub vertex: Vec<Vertex>,
    pub face: Option<Vec<Face>>, 
}


pub type VertexData = DataField<Vector3>; // TODO: use CoordLike in geometry_processing.rs?

// DISCLAIMER: This function is taken from
// https://serde.rs/string-or-struct.html
impl FromStr for VertexData {
    type Err = Void;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DataField::<Vector3>{
            _data: parse_string_vecvec3(s).unwrap(),
            _type: String::from("xyz"), // Default for VertexData (Note: it would be different from other DataFields)
            _ply_file: String::from(""),
        })
    }
}


impl VertexData{
    pub fn insert_dummy_at_the_beginning(&mut self) {
        self._data.insert(0, Vector3::ZERO);
    }

    pub fn normalize_to_xyz(&mut self) -> bool {
        // If given vertex data has type other than xyz,
        // (specifically a permutation of xyz) convert data 
        // layout to xyz to be used in computations. Returns 
        // false if no change is applied. 
        if self._type == "xyz" {
            return false; // already as expected
        }

        let mut new_data = Vec::with_capacity(self._data.len());

        for chunk in self._data.chunks_exact(3) {
            if chunk.len() < 3 {
                warn!("DataField had incomplete triplet, skipping remainder");
                break;
            }

            let (x, y, z) = match self._type.as_str() {
                "xyz" => (chunk[0], chunk[1], chunk[2]),
                "xzy" => (chunk[0], chunk[2], chunk[1]),
                "yxz" => (chunk[1], chunk[0], chunk[2]),
                "yzx" => (chunk[2], chunk[0], chunk[1]),
                "zxy" => (chunk[1], chunk[2], chunk[0]),
                "zyx" => (chunk[2], chunk[1], chunk[0]),
                other => {
                    warn!("Unknown vertex data type '{other}', assuming xyz");
                    (chunk[0], chunk[1], chunk[2])
                }
            };

            new_data.extend_from_slice(&[x, y, z]);
        }

        self._data = new_data;
        self._type = "xyz".to_string();
        true
    }
}

pub type TexCoordData = DataField<Float>; // Similar to VertexData and FaceType
impl TexCoordData {
    // length of uv field (since FaceType is also DataField<usize> duplicate defns error was occuring for len() function so I renamed it as a quick fix..)
    pub fn len_uv(&self) -> usize {
        debug_assert!(self._type == "uv"); // || self._type == ""); // Only uv supported
        (self._data.len() as f64 / 2.) as usize
    }

    pub fn get_uv_coords(&self, i: usize) -> [Float; 2] {
        debug_assert!(self._type == "uv"); // || self._type == "");
        let start = i * 2;
        [self._data[start], self._data[start + 1]]
    }
    
}

pub type FaceType = DataField<usize>;
impl FaceType {
    pub fn len_tris(&self) -> usize {
        debug_assert!(self._type == "triangle" || self._type == ""); // Only triangle meshes are supported
        (self._data.len() as f64 / 3.) as usize
    }

    pub fn is_empty(&self) -> bool {
        //debug_assert!(self._type == "triangle"); // Only triangle meshes are supported   
        self._data.is_empty()
    }

    pub fn get_tri_indices(&self, i: usize) -> [usize; 3] {
        debug_assert!(self._type == "triangle" || self._type == "");
        let start = i * 3;
        [self._data[start], self._data[start + 1], self._data[start + 2]]
    }
}


// TODO: Debug logs for Transformations deserialization
impl<'de> Deserialize<'de> for Transformations {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        struct Helper {
            #[serde(rename = "Translation")]
            translation: Option<SingleOrVec<TransformField>>,
            #[serde(rename = "Rotation")]
            rotation: Option<SingleOrVec<TransformField>>,
            #[serde(rename = "Scaling")]
            scaling: Option<SingleOrVec<TransformField>>,
            #[serde(rename = "Composite")]
            composite: Option<SingleOrVec<TransformField>>,
        }

        let helper = Helper::deserialize(deserializer)?;
        debug!("Deserialized Transformations Helper: {:?}", helper);

        Ok(Transformations {
            translation: helper.translation.unwrap_or(SingleOrVec::Empty),
            rotation: helper.rotation.unwrap_or(SingleOrVec::Empty),
            scaling: helper.scaling.unwrap_or(SingleOrVec::Empty),
            composite: helper.composite.unwrap_or(SingleOrVec::Empty),
        })
    }
}
