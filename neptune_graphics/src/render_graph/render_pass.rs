use crate::render_graph::{BufferId, RasterFn, TextureId};

#[derive(Clone, Debug)]
pub struct ColorAttachment {
    pub id: TextureId,
    pub clear: Option<[f32; 4]>,
}

#[derive(Clone, Debug)]
pub struct DepthStencilAttachment {
    pub id: TextureId,
    pub clear: Option<[f32; 2]>,
}

pub struct RasterPassBuilder {
    pub(crate) name: String,
    pub(crate) raster_fn: Option<Box<RasterFn>>,
    pub(crate) color_attachments: Vec<ColorAttachment>,
    pub(crate) depth_stencil_attachment: Option<DepthStencilAttachment>,

    pub(crate) vertex_buffers: Vec<BufferId>,
    pub(crate) index_buffers: Vec<BufferId>,
    pub(crate) shader_reads: (Vec<BufferId>, Vec<TextureId>),
}

impl RasterPassBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            raster_fn: None,
            color_attachments: vec![],
            depth_stencil_attachment: None,
            vertex_buffers: vec![],
            index_buffers: vec![],
            shader_reads: (vec![], vec![]),
        }
    }

    pub fn attachments(
        mut self,
        color_attachments: &[ColorAttachment],
        depth_stencil_attachment: Option<DepthStencilAttachment>,
    ) -> Self {
        self.color_attachments = color_attachments.to_vec();
        self.depth_stencil_attachment = depth_stencil_attachment;
        self
    }

    pub fn vertex_buffer(mut self, buffer_id: BufferId) -> Self {
        self.vertex_buffers.push(buffer_id);
        self
    }

    pub fn index_buffer(mut self, buffer_id: BufferId) -> Self {
        self.index_buffers.push(buffer_id);
        self
    }

    pub fn shader_read_buffer(mut self, buffer_id: BufferId) -> Self {
        self.shader_reads.0.push(buffer_id);
        self
    }

    pub fn shader_read_texture(mut self, texture_id: TextureId) -> Self {
        self.shader_reads.1.push(texture_id);
        self
    }

    pub fn raster_fn(
        mut self,
        raster_fn: impl FnOnce(std::rc::Rc<ash::Device>, ash::vk::CommandBuffer) + 'static,
    ) -> Self {
        assert!(
            self.raster_fn.replace(Box::new(raster_fn)).is_none(),
            "Already set raster function"
        );
        self
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
