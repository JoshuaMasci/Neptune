use crate::render_graph::{BufferId, TextureId};
use std::rc::Rc;

//TODO: use abstract types
use crate::pipeline::{PipelineState, VertexElement};
use crate::vulkan::ShaderModule;
use crate::vulkan::VulkanRasterCommandBuffer;

use crate::render_graph::render_graph::RasterPipeline;

#[derive(Clone, Debug)]
pub struct ColorAttachment {
    pub id: TextureId,
    pub clear: Option<[f32; 4]>,
}

#[derive(Clone, Debug)]
pub struct DepthStencilAttachment {
    pub id: TextureId,
    pub clear: Option<(f32, u32)>,
}

pub struct RasterPassBuilder {
    pub(crate) name: String,

    pub(crate) color_attachments: Vec<ColorAttachment>,
    pub(crate) depth_stencil_attachment: Option<DepthStencilAttachment>,

    pub(crate) vertex_buffers: Vec<BufferId>,
    pub(crate) index_buffers: Vec<BufferId>,
    pub(crate) shader_reads: (Vec<BufferId>, Vec<TextureId>),

    pub(crate) pipelines: Vec<RasterPipeline>,
}

impl RasterPassBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            color_attachments: vec![],
            depth_stencil_attachment: None,
            vertex_buffers: vec![],
            index_buffers: vec![],
            shader_reads: (vec![], vec![]),
            pipelines: vec![],
        }
    }

    pub fn attachments(
        &mut self,
        color_attachments: &[ColorAttachment],
        depth_stencil_attachment: Option<DepthStencilAttachment>,
    ) {
        self.color_attachments = color_attachments.to_vec();
        self.depth_stencil_attachment = depth_stencil_attachment;
    }

    pub fn vertex_buffer(&mut self, buffer_id: BufferId) {
        self.vertex_buffers.push(buffer_id);
    }

    pub fn index_buffer(&mut self, buffer_id: BufferId) {
        self.index_buffers.push(buffer_id);
    }

    pub fn shader_read_buffer(&mut self, buffer_id: BufferId) {
        self.shader_reads.0.push(buffer_id);
    }

    pub fn shader_read_texture(&mut self, texture_id: TextureId) {
        self.shader_reads.1.push(texture_id);
    }

    pub fn pipeline(
        &mut self,
        vertex_module: Rc<ShaderModule>,
        fragment_module: Option<Rc<ShaderModule>>,
        vertex_elements: Vec<VertexElement>,
        pipeline_state: PipelineState,
        raster_fn: impl FnOnce(&mut VulkanRasterCommandBuffer) + 'static,
    ) {
        self.pipelines.push(RasterPipeline {
            vertex_module,
            fragment_module,
            vertex_elements,
            pipeline_state,
            raster_fn: Box::new(raster_fn),
        });
    }
}

pub struct ComputePassBuilder {
    pub(crate) shader: String,
    pub(crate) dispatch_size: [u32; 3],
    pub(crate) buffer_reads: Vec<BufferId>,
    pub(crate) buffer_writes: Vec<BufferId>,
    pub(crate) texture_reads: Vec<TextureId>,
    pub(crate) texture_writes: Vec<TextureId>,
}
