use crate::render_graph::graph::RenderGraph;
use crate::render_graph::{
    Attachment, RasterPassPipeline, RenderPass, RenderPassFramebuffer, RenderPassType,
};
use crate::shader::{FragmentShader, VertexShader};
use crate::{PipelineState, VertexElement};

pub struct RenderGraphBuilder<'a> {
    render_graph: &'a mut RenderGraph,
}

impl<'a> RenderGraphBuilder<'a> {
    pub fn new(render_graph: &'a mut RenderGraph) -> Self {
        Self { render_graph }
    }

    pub fn add_raster_pass(&mut self, mut raster_pass: RasterPass) {
        self.render_graph.render_passes.push(RenderPass {
            name: raster_pass.name,
            buffer_read: vec![],
            buffer_write: vec![],
            texture_read: vec![],
            texture_write: vec![],
            pass_type: RenderPassType::Raster {
                framebuffer: raster_pass.framebuffer,
                pipelines: raster_pass.pipelines,
            },
        });
    }
}

pub struct RasterPass {
    name: String,
    framebuffer: RenderPassFramebuffer,
    pipelines: Vec<RasterPassPipeline>,
}

impl RasterPass {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            framebuffer: RenderPassFramebuffer::default(),
            pipelines: Vec::new(),
        }
    }

    pub fn color_attachment(&mut self, attachment: Attachment<[f32; 4]>) {
        self.framebuffer.color_attachment.push(attachment);
    }

    pub fn depth_stencil_attachment(&mut self, attachment: Attachment<(f32, u8)>) {
        let _ = self.framebuffer.depth_stencil_attachment.insert(attachment);
    }

    pub fn pipeline(
        &mut self,
        vertex_shader: &VertexShader,
        fragment_shader: Option<&FragmentShader>,
        pipeline_state: &PipelineState,
        vertex_layout: &[VertexElement],
        raster_fn: impl FnOnce(),
    ) {
        self.pipelines.push(RasterPassPipeline {
            vertex_shader: vertex_shader.get_handle(),
            fragment_shader: fragment_shader.map(|fragment_shader| fragment_shader.get_handle()),
            pipeline_state: (*pipeline_state).clone(),
            vertex_layout: vertex_layout.to_vec(),
            commands: vec![],
        });
    }
}
