use crate::buffer::BufferResource;
use crate::render_graph::command_buffer::{RasterCommand, RasterCommandBuffer};
use crate::render_graph::graph::RenderGraph;
use crate::render_graph::{
    Attachment, GraphResource, RasterPassPipeline, RenderPass, RenderPassFramebuffer,
    RenderPassType,
};
use crate::sampler::Sampler;
use crate::shader::{ComputeShader, FragmentShader, ShaderHandle, VertexShader};
use crate::texture::TextureResource;
use crate::{PipelineState, VertexElement};

pub struct RenderGraphBuilder<'a> {
    render_graph: &'a mut RenderGraph,
}

impl<'a> RenderGraphBuilder<'a> {
    pub fn new(render_graph: &'a mut RenderGraph) -> Self {
        Self { render_graph }
    }

    pub fn add_raster_pass(&mut self, raster_pass: RasterPass) {
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

    pub fn add_compute_pass(&mut self, compute_pass: ComputePass) {
        self.render_graph.render_passes.push(RenderPass {
            name: compute_pass.name,
            buffer_read: vec![],
            buffer_write: vec![],
            texture_read: vec![],
            texture_write: vec![],
            pass_type: RenderPassType::Compute {
                shader: compute_pass.shader,
                dispatch: compute_pass.dispatch,
                resources: compute_pass.resources,
            },
        })
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
        raster_fn: impl FnOnce(&mut RasterCommandBuffer),
    ) {
        let mut raster_command_buffer = RasterCommandBuffer::new();

        raster_fn(&mut raster_command_buffer);

        self.pipelines.push(RasterPassPipeline {
            vertex_shader: vertex_shader.get_handle(),
            fragment_shader: fragment_shader.map(|fragment_shader| fragment_shader.get_handle()),
            pipeline_state: (*pipeline_state).clone(),
            vertex_layout: vertex_layout.to_vec(),
            commands: raster_command_buffer.commands,
        });
    }
}

pub struct ComputePass {
    name: String,
    shader: ShaderHandle,
    dispatch: [u32; 3],
    resources: Vec<(u32, GraphResource)>,
}

impl ComputePass {
    pub fn new(name: &str, shader: &ComputeShader, dispatch: &[u32]) -> Self {
        let dispatch = [
            dispatch.first().cloned().unwrap_or(1),
            dispatch.get(1).cloned().unwrap_or(1),
            dispatch.get(2).cloned().unwrap_or(1),
        ];

        Self {
            name: String::from(name),
            shader: shader.get_handle(),
            dispatch,
            resources: vec![],
        }
    }

    pub fn bind_storage_buffer<T: BufferResource>(
        mut self,
        slot: u32,
        buffer: &T,
        write: bool,
    ) -> Self {
        self.resources.push((
            slot,
            GraphResource::StorageBuffer {
                buffer: buffer.get_graph_resource(),
                write,
            },
        ));
        self
    }

    pub fn bind_storage_texture<T: TextureResource>(
        mut self,
        slot: u32,
        texture: &T,
        write: bool,
    ) -> Self {
        self.resources.push((
            slot,
            GraphResource::StorageTexture {
                texture: texture.get_graph_resource(),
                write,
            },
        ));
        self
    }

    pub fn bind_sampler(mut self, slot: u32, sampler: &Sampler) -> Self {
        self.resources
            .push((slot, GraphResource::Sampler(sampler.get_handle())));
        self
    }

    pub fn bind_sampled_texture<T: TextureResource>(mut self, slot: u32, texture: &T) -> Self {
        self.resources.push((
            slot,
            GraphResource::SampledTexture(texture.get_graph_resource()),
        ));
        self
    }
}
