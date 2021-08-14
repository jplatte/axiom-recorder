use crate::{PositionedRenderObject, RenderObject};
use std::sync::Arc;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
    render_pass::RenderPass,
};

pub struct RawRenderer {
    render_pass: Arc<RenderPass>,
}
impl RawRenderer {
    pub fn new(render_pass: Arc<RenderPass>) -> Self { Self { render_pass } }
    pub fn render(
        &mut self,
        buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        dynamic_state: &DynamicState,
        _dimensions: &[u32; 2],
        render_objects: Arc<Vec<PositionedRenderObject>>,
    ) {
        for render_object in render_objects.iter() {
            if let RenderObject::Raw { render_fn } = &render_object.render_object {
                render_fn(
                    self.render_pass.clone(),
                    buffer_builder,
                    dynamic_state,
                    render_object.rect,
                );
            }
        }
    }
}
