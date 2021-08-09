use narui::*;
use std::{
    sync::mpsc::RecvTimeoutError,
};
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};
use recorder::pipeline_processing::create_node_from_name;
use recorder::pipeline_processing::parametrizable::{Parameters, ParameterValue};
use recorder::gui::image::*;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::array::IntoIter;
use recorder::pipeline_processing::processing_node::ProcessingNode;
use recorder::pipeline_processing::payload::Payload;
use std::sync::{MutexGuard, Arc};
use recorder::frame::rgb_frame::RgbFrame;
use recorder::pipeline_processing::execute::execute_pipeline;

#[derive(Clone)]
enum Message {
    Stop,
}

pub struct PlayerSink<T: Fn(Arc<RgbFrame>) + Send + Sync> {
    callback: T,
}
impl<T: Fn(Arc<RgbFrame>) + Send + Sync> ProcessingNode for PlayerSink<T> {
    fn process(&self, input: &mut Payload, frame_lock: MutexGuard<u64>) -> anyhow::Result<Option<Payload>> {
        let frame = input.downcast::<RgbFrame>().expect("Wrong input format");
        (self.callback)(frame);
        Ok(Some(Payload::empty()))
    }
}


#[widget]
pub fn player(context: Context) -> Fragment {
    let current_frame = context.listenable(None);
    context.thread(
        move |context, rx| {
            let callback = move |frame: Arc<RgbFrame>| {
                context.shout(current_frame, Some(frame));
            };

            let nodes = vec![
                create_node_from_name("RawDirectoryReader", &Parameters(HashMap::<_, _>::from_iter(IntoIter::new([
                    ("fps".to_string(), ParameterValue::FloatRange(24.0)),
                    ("file-pattern".to_string(), ParameterValue::StringParameter("/home/anuejn/code/apertus/axiom-recorder/test/Darkbox-Timelapse-Clock-Sequence/*".to_string())),
                    ("first-red-x".to_string(), ParameterValue::BoolParameter(false)),
                    ("first-red-y".to_string(), ParameterValue::BoolParameter(false)),
                    ("bit-depth".to_string(), ParameterValue::IntRange(12)),
                    ("width".to_string(), ParameterValue::IntRange(4096)),
                    ("height".to_string(), ParameterValue::IntRange(3072)),
                    ("loop".to_string(), ParameterValue::BoolParameter(true)),
                    ("sleep".to_string(), ParameterValue::FloatRange(0.0)),
                ])))).unwrap(),
                create_node_from_name("BitDepthConverter", &Parameters(HashMap::new())).unwrap(),
                create_node_from_name("Debayer", &Parameters(HashMap::new())).unwrap(),
                Arc::new(PlayerSink { callback }) as Arc<dyn ProcessingNode>
            ];
            execute_pipeline(nodes).unwrap();
        },
        Message::Stop,
        (),
    );

    let frame = context.listen(current_frame);
    if let Some(frame) = frame {
        rsx! { <image image={frame}/> }
    } else {
        rsx! { }
    }
}

fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("ara player")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <player />
        },
    );
}
