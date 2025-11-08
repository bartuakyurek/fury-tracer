/*

    Declare data structs needed to parse JSON. 

    - DataField: To be used in Mesh and Faces
    - SingleOrVec
    - VertexData: Type alias of DataField<Vector3>

    @date: 13 Oct, 2025
    @author: Bartu
*/

use serde::{Deserialize, de::{Deserializer}};
use std::{ops::Index, str::FromStr};
use tracing::{warn};
use void::Void;

use crate::json_parser::{deser_vertex_data, deser_usize_vec, parse_string_vecvec3};
use crate::numeric::{Vector3};

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
    pub fn all(&self) -> Vec<T> {
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
}

impl<T: Default> Default for SingleOrVec<T> {
    fn default() -> Self {
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
        return true;
    }
}


pub type FaceType = DataField<usize>;
impl FaceType {
    pub fn len(&self) -> usize {
        debug_assert!(self._type == "triangle"); // Only triangle meshes are supported
        (self._data.len() as f64 / 3.) as usize
    }

    pub fn get_indices(&self, i: usize) -> [usize; 3] {
        debug_assert!(self._type == "triangle");
        let start = i * 3;
        [self._data[start], self._data[start + 1], self._data[start + 2]]
    }
}
