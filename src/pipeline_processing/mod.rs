use crate::{
    frame::bit_depth_converter::BitDepthConverter,
    gpu::{debayer::Debayer, display::Display},
    pipeline_processing::parametrizable::{Parameterizable, ParameterizableDescriptor, Parameters},
    video_io::{
        reader_raw::{RawBlobReader, RawDirectoryReader},
        writer_cinema_dng::CinemaDngWriter,
        writer_ffmpeg::FfmpegWriter,
        writer_raw::{RawBlobWriter, RawDirectoryWriter},
    },
};
use anyhow::{anyhow, Result};
use processing_node::ProcessingNode;
use std::{collections::HashMap, sync::Arc};

#[cfg(feature = "gst")]
use crate::video_io::writer_gstreamer::GstWriter;

pub mod execute;
pub mod parametrizable;
pub mod payload;
pub mod processing_node;

macro_rules! generate_dynamic_node_creation_functions {
    ($($x:ty),+ $(,)?) => {
        pub fn list_available_nodes() -> HashMap<String, ParameterizableDescriptor> {
            let mut to_return = HashMap::new();
            $(
                to_return.insert(<$x>::get_name(), <$x>::describe());
            )+
            to_return
        }

        pub fn create_node_from_name(name: &str, parameters: &Parameters) -> Result<Arc<dyn ProcessingNode>> {
            $(
                if name == <$x>::get_name() {
                    return Ok(Arc::new(<$x>::from_parameters(parameters)?))
                };
            )+

            Err(anyhow!("no node named {} found", name))
        }
    };
}


// TODO(robin): this is stupid
#[cfg(feature = "gst")]
generate_dynamic_node_creation_functions![
    RawBlobReader,
    RawDirectoryReader,
    BitDepthConverter,
    Debayer,
    RawBlobWriter,
    RawDirectoryWriter,
    CinemaDngWriter,
    FfmpegWriter,
    GstWriter,
    Display,
];

#[cfg(not(feature = "gst"))]
generate_dynamic_node_creation_functions![
    RawBlobReader,
    RawDirectoryReader,
    BitDepthConverter,
    Debayer,
    RawBlobWriter,
    RawDirectoryWriter,
    CinemaDngWriter,
    FfmpegWriter,
    Display,
];
