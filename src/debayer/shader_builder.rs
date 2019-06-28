use crate::{
    error,
    graphical::ui_lib::{Cache, DrawParams, Drawable, ShaderBox, SpatialProperties, Vec2},
    throw,
    util::error::{Error, Res},
};
use glium::{
    backend::glutin::headless::Headless,
    texture::{self, MipmapsOption, Texture2d, UncompressedFloatFormat},
    uniform,
};
use glutin::{ContextBuilder, EventsLoop};
use std::{borrow::Cow, collections::btree_map::BTreeMap, error, result::Result::Ok};

use crate::util::formatting_helpers::format_hash_map_option;
use glium::{
    buffer::BufferType::ShaderStorageBuffer,
    texture::RawImage2d,
    uniforms::{AsUniformValue, EmptyUniforms, UniformValue, Uniforms, UniformsStorage},
};
use glutin::dpi::PhysicalSize;
use include_dir::{Dir, *};
use itertools::Itertools;
use regex::Regex;
use std::{collections::HashMap, hash::Hash, panic::set_hook};


type Implications = HashMap<String, Option<String>>;

// this is only a newtype because rusts prohibition of implementing foreign
// traits for foreign Types sucks
#[derive(Clone)]
pub struct F32Uniforms(pub HashMap<String, Option<f32>>);

impl Uniforms for F32Uniforms {
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut callback: F) {
        for (k, v) in &self.0 {
            callback(k, UniformValue::Float(v.unwrap()));
        }
    }
}

// statically pull some shaders into the binary
static SHADERS: Dir = include_dir!("src/debayer/shader");

pub struct ShaderBuilder {
    shader_parts: Vec<ShaderBuilderPart>,
}

impl ShaderBuilder {
    pub fn from_descr_str(descr_str: &str) -> Res<Self> {
        let re = Regex::new("(\\.?/?[a-z_]*)\\((.*?)\\)").unwrap();
        let mut shader_parts = Vec::new();
        for cap in re.captures_iter(descr_str.as_ref()) {
            let part_name = String::from(format!("{}.glsl", cap.get(1).unwrap().as_str()));
            let part_params = String::from(cap.get(2).unwrap().as_str());

            let shader_code = if part_name.contains("/") {
                // Shader should be read from fs
                unimplemented!()
            } else {
                // A builtin Shader should be used
                SHADERS
                    .get_file(part_name.clone())
                    .ok_or(Error::new(format!(
                        "shader '{}' is not builtin. Did you mean './{}'? \nBuiltin Shaders are: \n{}",
                        part_name.clone(), part_name.clone(), ShaderBuilder::get_available()?.iter().map(|(name, (uniforms, implications))| {
                            format!(
                                "\t* {}({}) [{}]",
                                name,
                                format_hash_map_option(&uniforms.0),
                                format_hash_map_option(implications),
                            )
                        }).collect::<Vec<String>>().join("\n")
                    )))?
                    .contents_utf8()
                    .unwrap()
            };

            shader_parts.push(ShaderBuilderPart::new_with_str_params(
                String::from(shader_code),
                part_params,
                part_name,
            )?);
        }

        Ok(Self { shader_parts })
    }

    pub fn get_available() -> Res<HashMap<String, (F32Uniforms, Implications)>> {
        let mut result = HashMap::new();
        for file in SHADERS.files() {
            let filepath = file.path().to_str().unwrap();
            if !filepath.ends_with(".glsl") {
                continue;
            };

            let part = ShaderBuilderPart::new(
                String::from(file.contents_utf8().unwrap()),
                None,
                String::from(filepath),
            )?;
            result.insert(String::from(filepath), (part.get_uniforms(), part.get_implications()));
        }

        Ok(result)
    }

    pub fn get_implications(&self) -> Implications {
        let mut to_return = HashMap::new();
        for part in &self.shader_parts {
            for (k, v) in part.get_implications() {
                to_return.insert(k, v);
            }
        }
        to_return
    }

    pub fn get_uniforms(&self) -> F32Uniforms {
        let mut to_return = HashMap::new();
        for part in &self.shader_parts {
            for (k, v) in part.get_uniforms().0 {
                to_return.insert(k, v);
            }
        }
        F32Uniforms(to_return)
    }

    pub fn get_code(&self) -> String {
        let mut to_return = String::new();

        to_return += r#"
            #version 450
            uniform sampler2D raw_image;
            out vec4 color;
        "#;

        for part in &self.shader_parts {
            to_return += &format!("\n\n/////////////////////// {} /////////////////\n", part.name);
            to_return += &part.get_code();
        }

        to_return += &format!("\n\n///////////// main /////////////////\n");
        to_return += r#"
            void main(void) {
                ivec2 size = textureSize(raw_image, 0);
                ivec2 icord = ivec2(gl_FragCoord);
                ivec2 rotcord = ivec2(icord.x, size.y - icord.y);

                vec3 debayered = get_color_value(rotcord);

                color = vec4(debayered, 1.0);
            }
        "#;

        String::from(to_return)
    }
}

pub struct ShaderBuilderPart {
    code: String,
    uniforms: F32Uniforms,
    name: String,
}

impl ShaderBuilderPart {
    fn new(code: String, non_default_uniforms: Option<F32Uniforms>, name: String) -> Res<Self> {
        let re =
            Regex::new("uniform\\s+float\\s+(\\w+)\\s*;\\s*//\\s*=\\s*(\\d*\\.?\\d*)").unwrap();
        let mut uniforms = F32Uniforms(HashMap::new()).0;

        let mut taken = 0;
        for cap in re.captures_iter(code.as_str()) {
            let uniform_name = cap.get(1).unwrap().as_str();
            let default_value: Option<f32> = cap.get(2).map(|v| v.as_str().parse().unwrap());

            let value = match &non_default_uniforms {
                Some(ndu) => match ndu.0.get(uniform_name) {
                    Some(v) => {
                        taken += 1;
                        Some(v.unwrap_or(1.0))
                    }
                    None => default_value,
                },
                None => default_value,
            };

            if non_default_uniforms.is_some() && value.is_none() {
                throw!(
                    "uniform '{}' of part '{}' has no default and is not set.",
                    uniform_name,
                    name
                )
            }

            uniforms.insert(String::from(uniform_name), value);
        }

        if non_default_uniforms.is_some() {
            if taken != non_default_uniforms.unwrap().0.len() {
                throw!("some uniform values were not consumed by that shader. maybe you set nonexistent uniforms?")
            }
        }

        Ok(ShaderBuilderPart { code, uniforms: F32Uniforms(uniforms), name })
    }

    fn new_with_str_params(code: String, params: String, name: String) -> Res<Self> {
        let re = Regex::new("(\\w+):\\s*(\\d*\\.?\\d*)").unwrap();
        let mut non_default_uniforms: HashMap<String, Option<f32>> = HashMap::new();
        for cap in re.captures_iter(params.as_str()) {
            non_default_uniforms.insert(
                String::from(cap.get(1).unwrap().as_str()),
                cap.get(2).map(|v| v.as_str().parse().unwrap()),
            );
        }

        Self::new(code, Some(F32Uniforms(non_default_uniforms)), name)
    }

    fn get_uniforms(&self) -> F32Uniforms { self.uniforms.clone() }

    fn get_implications(&self) -> Implications {
        let re = Regex::new("! (.*)(\\s?=\\s?(.*))").unwrap();
        let mut result = HashMap::new();
        for cap in re.captures_iter(&self.code) {
            result.insert(
                String::from(cap.get(1).unwrap().as_str()),
                cap.get(3).map(|x| String::from(x.as_str())),
            );
        }

        result
    }

    fn get_code(&self) -> String { self.code.clone() }
}
