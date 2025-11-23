/*

    Provide utilities to parse JSON file in CENG 795 format.

    This format currently assumes:
        - Every field is String (even integers are encapsulated in quotes e.g. "6")
        - Vector3 data fields are in format "<a> <a> <a>" where <a> is integer or float


    The parser is somewhat robust, let <a> be integer or float type,
    in JSON file <a> can be given both in quotes (string) or as is.

    e.g. In JSON file both
    "MaxRecursionDepth": "6" and "MaxRecursionDepth": 6
    works as MaxRecursionDepth: int in source code

    WARNING: It is not robust for handling vec3 types given in brackets 
    e.g. providing [0, 0, 0] for "BackgroundColor" will fail. It is assumed to be
    "BackgroundColor": "0 0 0" for the time being.

    @date: 2 Oct, 2025
    @author: bartu 
*/

use std::fmt::{self};
use std::marker::PhantomData;
use std::str::FromStr;
use std::fs::File;
use std::io::BufReader;

use void::Void;
use serde_json::{self, Value};
use serde::{Deserialize, Deserializer};
use serde::de::{self, Visitor, SeqAccess, MapAccess};

use crate::prelude::*;
use crate::scene::{RootScene};
use crate::camera::{NearPlane};
use crate::material::*;
use crate::numeric::{Int, Float, Vector3};
use crate::json_structs::{Transformations, TransformKind};

pub fn parse_json795(path: &str) -> Result<RootScene, Box<dyn std::error::Error>> {
    /*
        Parse JSON files in CENG 795 format.
    */

    let span = tracing::span!(tracing::Level::INFO, "load_scene");
    let _enter = span.enter();

    // Open file
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    debug!("Reading file from {}", path);
    
    // Parse JSON into Scene
    let root: RootScene = serde_json::from_reader(reader)?;
    Ok(root) 


}

pub(crate) fn deser_opt_usize<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;

    match opt {
        Some(s) if !s.is_empty() => {
            let parsed = s.parse::<usize>().map_err(serde::de::Error::custom)?;
            Ok(Some(parsed))
        }
        _ => Ok(None),
    }
}


pub(crate) fn deser_usize<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    /*
        Deserialize usize type given as either string or number in JSON
        TODO: This is code duplication, use generics to combine
        deser_float, deser_int, deser_usize
    */
    let s: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match s {
        serde_json::Value::Number(n) => n.as_i64()
            .map(|v| v as usize)
            .ok_or_else(|| de::Error::custom("Invalid integer")),
        serde_json::Value::String(s) => s.parse::<usize>()
            .map_err(|_| de::Error::custom("Failed to parse integer from string")),
        t => Err(de::Error::custom(format!("Expected int or string, found {:#?}", t))),
    }
}

pub(crate) fn deser_int<'de, D>(deserializer: D) -> Result<Int, D::Error>
where
    D: Deserializer<'de>,
{
    /*
        Deserialize integer type given as either string or number in JSON
    */
    let s: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match s {
        serde_json::Value::Number(n) => n.as_i64()
            .map(|v| v as Int)
            .ok_or_else(|| de::Error::custom("Invalid integer")),
        serde_json::Value::String(s) => s.parse::<Int>()
            .map_err(|_| de::Error::custom("Failed to parse integer from string")),
        t => Err(de::Error::custom(format!("Expected int or string, found {t}"))),
    }
}

// Handles floats as string or number
pub(crate) fn deser_float<'de, D>(deserializer: D) -> Result<Float, D::Error>
where
    D: Deserializer<'de>,
{
    /*
        Deserialize float type given as either string or number in JSON
    */
    let s: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match s {
        serde_json::Value::Number(n) => n.as_f64()
            .map(|v| v as Float)
            .ok_or_else(|| de::Error::custom("Invalid float")),
        serde_json::Value::String(s) => s.parse::<Float>()
            .map_err(|_| de::Error::custom("Failed to parse float from string")),
        t => Err(de::Error::custom(format!("Expected float or string, found {t}"))),
    }
}

pub(crate) fn deser_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    struct BoolVisitor;

    impl<'de> serde::de::Visitor<'de> for BoolVisitor {
        type Value = bool;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a boolean or a string containing true/false")
        }

        fn visit_bool<E>(self, v: bool) -> Result<bool, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }

        fn visit_str<E>(self, v: &str) -> Result<bool, E>
        where
            E: serde::de::Error,
        {
            match v.to_lowercase().as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                "1" => Ok(true),
                "0" => Ok(false),
                _ => Err(E::custom(format!("invalid bool '{}'", v))),
            }
        }

        fn visit_string<E>(self, v: String) -> Result<bool, E>
        where
            E: serde::de::Error,
        {
            self.visit_str(&v)
        }

        fn visit_u64<E>(self, v: u64) -> Result<bool, E>
        where
            E: serde::de::Error,
        {
            Ok(v != 0)
        }
    }

    deserializer.deserialize_any(BoolVisitor)
}

pub trait From3<T>: Sized {
    fn new(x: T, y: T, z: T) -> Self;
}

impl From3<f32> for bevy_math::Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self::new(x, y, z)
    }
}

impl From3<f64> for bevy_math::DVec3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self::new(x, y, z)
    }
}

pub(crate) fn deser_vec3<'de, D, V, F>(deserializer: D) -> Result<V, D::Error>
where
    D: Deserializer<'de>,
    F: Deserialize<'de> + FromStr,
    F::Err: fmt::Display,
    V: From3<F>,
{
    struct Vec3Visitor<V, F>(PhantomData<(V, F)>);

    impl<'de, V, F> Visitor<'de> for Vec3Visitor<V, F>
    where
        F: Deserialize<'de> + FromStr,
        F::Err: fmt::Display,
        V: From3<F>,
    {
        type Value = V;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a Vec3 as a string 'x y z' or an array [x, y, z]")
        }

        // Given "X Y Z"
        fn visit_str<E>(self, value: &str) -> Result<V, E>
        where
            E: de::Error,
        {
            parse_vec3_str(value).map_err(de::Error::custom)
        }

        // Given [X, Y, Z]
        fn visit_seq<A>(self, mut seq: A) -> Result<V, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let x: F = seq
                .next_element()?
                .ok_or_else(|| de::Error::custom("Expected 3 elements in Vec3 array"))?;
            let y: F = seq
                .next_element()?
                .ok_or_else(|| de::Error::custom("Expected 3 elements in Vec3 array"))?;
            let z: F = seq
                .next_element()?
                .ok_or_else(|| de::Error::custom("Expected 3 elements in Vec3 array"))?;
            if seq.next_element::<F>()?.is_some() {
                return Err(de::Error::custom("Expected only 3 elements in Vec3 array"));
            }
            Ok(V::new(x, y, z))
        }
    }

    deserializer.deserialize_any(Vec3Visitor(PhantomData))
}


pub(crate) fn deser_pair<'de, D, T>(deserializer: D) -> Result<[T; 2], D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + FromStr,
    T::Err: fmt::Display,
{
    struct Vec2Visitor<T>(PhantomData<T>);

    impl<'de, T> Visitor<'de> for Vec2Visitor<T>
    where
        T: Deserialize<'de> + FromStr,
        T::Err: fmt::Display,
    {
        type Value = [T; 2];

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an array of 2 numbers or a string e.g. 'width height'")
        }

        fn visit_str<E>(self, value: &str) -> Result<[T; 2], E>
        where
            E: de::Error,
        {
            let parts: Vec<&str> = value.split_whitespace().collect();
            if parts.len() != 2 {
                return Err(E::custom("Expected 2 components for Vec2 string"));
            }
            let x = parts[0]
                .parse::<T>()
                .map_err(|_| E::custom("Failed parsing first component"))?;
            let y = parts[1]
                .parse::<T>()
                .map_err(|_| E::custom("Failed parsing second component"))?;
            Ok([x, y])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<[T; 2], A::Error>
        where
            A: SeqAccess<'de>,
        {
            let x: T = seq.next_element()?.ok_or_else(|| de::Error::custom("expected 2 elements"))?;
            let y: T = seq.next_element()?.ok_or_else(|| de::Error::custom("expected 2 elements"))?;
            if seq.next_element::<T>()?.is_some() {
                return Err(de::Error::custom("expected only 2 elements"));
            }
            Ok([x, y])
        }
    }

    deserializer.deserialize_any(Vec2Visitor::<T>(PhantomData))
}

pub(crate) fn deser_numeric_vec<'de, D, N>(deserializer: D) -> Result<Vec<N>, D::Error>
where
    D: serde::Deserializer<'de>,
    N: FromStr, 
    N::Err: fmt::Display,
{
    // Deserialize string of numbers separated by whitespace
    // into a vector of numbers, e.g. "0 2 3" in .json is deserialized
    // to Vec<N> where N is number-like (see deser_usize_vec and deser_int_vec 
    // wrappers for usize and Int (which is defined in numeric.rs) types.
    let s: String = Deserialize::deserialize(deserializer)?;
    let numbers = s
        .split_whitespace()
        .map(|x| x.parse::<N>().map_err(serde::de::Error::custom))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(numbers)
}


// Wrapper for deser_numeric_vec<Float>
pub(crate) fn deser_float_vec<'de, D>(deserializer: D) -> Result<Vec<Float>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deser_numeric_vec::<D, Float>(deserializer)
}


// Wrapper for deser_numeric_vec<usize>
pub(crate) fn deser_usize_vec<'de, D>(deserializer: D) -> Result<Vec<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deser_numeric_vec::<D, usize>(deserializer)
}


// Wrapper for deser_numeric_vec<Int>
//pub(crate) fn deser_int_vec<'de, D>(deserializer: D) -> Result<Vec<Int>, D::Error>
//where
//    D: serde::Deserializer<'de>,
//{
//    deser_numeric_vec::<D, Int>(deserializer)
//}

pub(crate) fn deser_usize_array<'de, D, const N: usize>(deserializer: D) -> Result<[usize; N], D::Error>
where
    D: Deserializer<'de>,
{
    let v = deser_usize_vec(deserializer)?;
    if v.len() != N {
        return Err(serde::de::Error::custom(format!(
            "expected {} elements, got {}",
            N, v.len()
        )));
    }

    // Convert Vec<usize> to [usize; N] array 
    v.try_into()
        .map_err(|_| serde::de::Error::custom("failed to convert Vec to array"))
}

pub(crate) fn deser_nearplane<'de, D>(deserializer: D) -> Result<NearPlane, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() > 4 {
        warn!("Expected 4 floats for nearplane definition found extra elements in size {} (most likely 5 floats are received, ignoring parts after 4th value)", parts.len())
        // return Err(de::Error::custom("Expected 5 elements for NearPlane"));
    }
    Ok(NearPlane {
        left: parts[0].parse().map_err(|_| de::Error::custom("Failed parsing left"))?,
        right: parts[1].parse().map_err(|_| de::Error::custom("Failed parsing right"))?,
        bottom: parts[2].parse().map_err(|_| de::Error::custom("Failed parsing bottom"))?,
        top: parts[3].parse().map_err(|_| de::Error::custom("Failed parsing top"))?,
    })
}

pub fn deser_vecvec3<'de, D>(deserializer: D) -> Result<Vec<Vector3>, D::Error>
where
    D: Deserializer<'de>,
{
    // Deserialize a vector of Vector3
    // given either a single string of "X Y Z" or 
    // array of strings ["X1 Y1 Z1", "X2 Y2 Z2", ...]
    struct VecVec3Visitor;

    impl<'de> Visitor<'de> for VecVec3Visitor {
        type Value = Vec<Vector3>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string 'X Y Z' or an array of such strings")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![parse_vec3_str(v).map_err(de::Error::custom)?])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(elem) = seq.next_element::<String>()? {
                vec.push(parse_vec3_str(&elem).map_err(de::Error::custom)?);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(VecVec3Visitor)
}

/// Helper function: parse a string like "25 25 25" into Vector3
fn parse_vec3_str<V, F>(s: &str) -> Result<V, String> 
where 
    F: FromStr,
    F::Err: fmt::Display,
    V: From3<F>,
{
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(format!("Expected 3 values, got {}", parts.len()));
    }
    let x = parts[0].parse::<F>().map_err(|e| e.to_string())?;
    let y = parts[1].parse::<F>().map_err(|e| e.to_string())?;
    let z = parts[2].parse::<F>().map_err(|e| e.to_string())?;
    Ok(V::new(x, y, z))
}


pub fn deser_vertex_data<'de, D>(deserializer: D) -> Result<Vec<Vector3>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    parse_string_vecvec3(&s).map_err(serde::de::Error::custom)
}


pub fn parse_string_vecvec3(s: &str) -> Result<Vec<Vector3>, String> {
    parse_string_vec(s, 3, |chunk| Ok(Vector3::new(chunk[0], chunk[1], chunk[2])))
}


fn parse_string_vec<T, F>(s: &str, chunk_len: usize, mut f: F) -> Result<Vec<T>, String>
where
    F: FnMut(&[Float]) -> Result<T, String>,
{
    let nums: Vec<Float> = s
        .split_whitespace()
        .map(|x| x.parse::<Float>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;

    if nums.len() % chunk_len != 0 {
        return Err(format!("Input length not divisible by {}", chunk_len));
    }

    nums.chunks(chunk_len)
        .map(|chunk| f(chunk))
        .collect::<Result<Vec<_>, _>>()
}


// DISCLAIMER: This function is taken from
// https://serde.rs/string-or-struct.html
pub fn deser_string_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Void>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Void>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

fn parse_single_material(value: serde_json::Value) -> HeapAllocMaterial {
    
    debug!("Parsing material JSON: {:#?}", value);

    // Check _type field
    let mat_type = value.get("_type").and_then(|v| v.as_str()).unwrap_or("diffuse");

    match mat_type {
        // TODO: This box will break if you change HeapAllocatedMaterial type! Update: wait it didn't... I understand both are smart pointers but why this function is stil valid? Shouldn't it be updated to Arc? 
        "diffuse" => Box::new(DiffuseMaterial::new_from(&value)),
        "mirror" => Box::new(MirrorMaterial::new_from(&value)),
        "dielectric" => Box::new(DielectricMaterial::new_from(&value)),
        "conductor" => Box::new(ConductorMaterial::new_from(&value)),
        // Add more materials here

        other => {
            error!("Unknown material type '{other}', defaulting to DiffuseMaterial");
            Box::new(DiffuseMaterial::new_from(&value))
        }
    }
}

pub fn parse_material(value: serde_json::Value) -> Vec<HeapAllocMaterial> {
    match value {
        Value::Array(arr) => arr.into_iter().map(parse_single_material).collect(),
        Value::Object(_) => vec![parse_single_material(value)],
        _ => {
            error!("Invalid material JSON, expected object or array: {value:?}");
            vec![]
        }
    }
}
pub fn parse_transform_expression(
    expr: &str,
    global_transforms: &Transformations
) -> Matrix4 {

    let mut out = Matrix4::IDENTITY;

    for token in expr.split_whitespace() {
        debug!("Parsing token {}", token);
        if token.len() < 2 {
            warn!("Found token.len() < 2, skipping...");
            continue;
        }
        
        let (kind, id_str) = token.split_at(1);
        let id: usize = match id_str.parse() {
            Ok(n) => n,
            Err(_) => {
                warn!("Invalid transformation id in '{}'", token);
                continue;
            }
        };
        debug!("Searching transformation of id '{}'...", id);
        match kind {
            "t" | "T" => {
                if let Some(tf) = global_transforms.find_translation(id) {
                    debug!("Found translation: {:?}", tf);
                    out = tf.get_mat4(TransformKind::Translation) * out; 
                    debug!("After translation: {}", out);
                }
            }
            "s" | "S" => {
                if let Some(sf) = global_transforms.find_scaling(id) {
                    debug!("Found scaling: {:?}", sf);
                    out = sf.get_mat4(TransformKind::Scaling) * out;
                    debug!("After scaling: {}", out);
                }
            }
            "r" | "R" => {
                if let Some(rf) = global_transforms.find_rotation(id) {
                    debug!("Found rotation: {:?}", rf);
                    out = rf.get_mat4(TransformKind::Rotation) * out;
                    debug!("After rotation: {}", out);
                }
            }
            "c" | "C" => {
                if let Some(cf) = global_transforms.find_composite(id) {
                    debug!("Found composite: {:?}", cf);
                    out = cf.get_mat4(TransformKind::Composite) * out;
                    debug!("After composite: {}", out);
                }
            }
            _ => warn!("Unknown transform token '{}'", kind),
        }
    }

    out
}
