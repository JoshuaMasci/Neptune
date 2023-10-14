use crate::{
    BufferDescription, BufferHandle, ComputePipelineHandle, ImageHandle, RasterPipelineHandle,
    SamplerHandle, SurfaceHandle, TransientImageDesc,
};
use std::ops::Range;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum QueueType {
    Graphics,
    PreferAsyncCompute,
    ForceAsyncCompute,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BufferOffset {
    pub buffer: BufferHandle,
    pub offset: usize,
}

#[derive(Debug)]
pub enum ShaderResourceUsage {
    StorageBuffer { buffer: BufferHandle, write: bool },
    StorageImage { image: ImageHandle, write: bool },
    SampledImage(ImageHandle),
    Sampler(SamplerHandle),
}

#[derive(Debug, Eq, PartialEq)]
pub enum ComputeDispatch {
    Size([u32; 3]),
    Indirect(BufferOffset),
}

#[derive(Debug)]
pub struct ComputePassBuilder {
    name: String,
    queue: QueueType,
    pipeline: ComputePipelineHandle,
    resources: Vec<ShaderResourceUsage>,
    dispatch: ComputeDispatch,
}

impl ComputePassBuilder {
    pub fn new(name: &str, queue: QueueType, pipeline: ComputePipelineHandle) -> Self {
        Self {
            name: name.to_string(),
            queue,
            pipeline,
            resources: Vec::new(),
            dispatch: ComputeDispatch::Size([1; 3]),
        }
    }

    pub fn dispatch_size(mut self, size: [u32; 3]) -> Self {
        self.dispatch = ComputeDispatch::Size(size);
        self
    }

    pub fn dispatch_indirect(mut self, buffer: BufferHandle, offset: usize) -> Self {
        self.dispatch = ComputeDispatch::Indirect(BufferOffset { buffer, offset });
        self
    }

    pub fn read_buffer(mut self, buffer: BufferHandle) -> Self {
        self.resources.push(ShaderResourceUsage::StorageBuffer {
            buffer,
            write: false,
        });
        self
    }

    pub fn write_buffer(mut self, buffer: BufferHandle) -> Self {
        self.resources.push(ShaderResourceUsage::StorageBuffer {
            buffer,
            write: true,
        });
        self
    }

    pub fn read_storage_image(mut self, image: ImageHandle) -> Self {
        self.resources.push(ShaderResourceUsage::StorageImage {
            image,
            write: false,
        });
        self
    }

    pub fn write_storage_image(mut self, image: ImageHandle) -> Self {
        self.resources
            .push(ShaderResourceUsage::StorageImage { image, write: true });
        self
    }

    pub fn read_sampled_image(mut self, image: ImageHandle) -> Self {
        self.resources
            .push(ShaderResourceUsage::SampledImage(image));
        self
    }

    pub fn read_sampler(mut self, sampler: SamplerHandle) -> Self {
        self.resources.push(ShaderResourceUsage::Sampler(sampler));
        self
    }

    pub fn build(self) -> RenderPass {
        RenderPass {
            lable_name: self.name,
            lable_color: [0.0, 1.0, 0.0, 1.0],
            queue: self.queue,
            pass_type: RenderPassType::Compute {
                pipeline: self.pipeline,
                resources: self.resources,
                dispatch: self.dispatch,
            },
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]

pub enum IndexType {
    U16,
    U32,
}

#[derive(Debug, PartialEq, Copy, Clone)]

pub struct ColorAttachment {
    pub image: ImageHandle,
    pub clear: Option<[f32; 4]>,
}

impl ColorAttachment {
    pub fn new(image: ImageHandle) -> Self {
        Self { image, clear: None }
    }

    pub fn new_clear(image: ImageHandle, clear: [f32; 4]) -> Self {
        Self {
            image,
            clear: Some(clear),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct DepthStencilAttachment {
    pub image: ImageHandle,
    pub clear: Option<(f32, u32)>,
}

impl DepthStencilAttachment {
    pub fn new(image: ImageHandle) -> Self {
        Self { image, clear: None }
    }

    pub fn new_clear(image: ImageHandle, clear: (f32, u32)) -> Self {
        Self {
            image,
            clear: Some(clear),
        }
    }
}

#[derive(Default, Debug)]
pub struct Framebuffer {
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RasterDispatch {
    Draw {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
    DrawIndexed {
        base_vertex: i32,
        indices: Range<u32>,
        instances: Range<u32>,
    },
    DrawIndirect {
        buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
    },
    DrawIndirectIndexed {
        buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
    },
}

#[derive(Debug)]
pub struct RasterDrawCommand {
    pub pipeline: RasterPipelineHandle,
    pub vertex_buffers: Vec<BufferOffset>,
    pub index_buffer: Option<(BufferOffset, IndexType)>,
    pub resources: Vec<ShaderResourceUsage>,
    pub dispatch: RasterDispatch,
}

#[derive(Debug)]
pub struct RasterPassBuilder {
    name: String,
    framebuffer: Framebuffer,
    draw_commands: Vec<RasterDrawCommand>,
}

impl RasterPassBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            framebuffer: Framebuffer::default(),
            draw_commands: Vec::new(),
        }
    }

    pub fn add_color_attachment(mut self, attachment: ColorAttachment) -> Self {
        self.framebuffer.color_attachments.push(attachment);
        self
    }

    pub fn add_depth_stencil_attachment(mut self, attachment: DepthStencilAttachment) -> Self {
        self.framebuffer.depth_stencil_attachment = Some(attachment);
        self
    }

    pub fn add_draw_command(mut self, draw_command: RasterDrawCommand) -> Self {
        self.draw_commands.push(draw_command);
        self
    }

    pub fn build(self) -> RenderPass {
        RenderPass {
            lable_name: self.name,
            lable_color: [1.0, 0.0, 0.0, 1.0],
            queue: QueueType::Graphics,
            pass_type: RenderPassType::Raster {
                framebuffer: self.framebuffer,
                draw_commands: self.draw_commands,
            },
        }
    }
}

#[derive(Debug)]
pub(crate) enum RenderPassType {
    Compute {
        pipeline: ComputePipelineHandle,
        resources: Vec<ShaderResourceUsage>,
        dispatch: ComputeDispatch,
    },
    Raster {
        framebuffer: Framebuffer,
        draw_commands: Vec<RasterDrawCommand>,
    },
}

#[derive(Debug)]
pub struct RenderPass {
    pub(crate) lable_name: String,
    pub(crate) lable_color: [f32; 4],
    pub(crate) queue: QueueType,
    pub(crate) pass_type: RenderPassType,
}

#[derive(Default, Debug)]
pub struct RenderGraphBuilder {
    pub(crate) transient_buffers: Vec<BufferDescription>,
    pub(crate) transient_images: Vec<TransientImageDesc>,
    pub(crate) swapchain_images: Vec<SurfaceHandle>,
    pub(crate) passes: Vec<RenderPass>,
}

impl RenderGraphBuilder {
    pub fn create_transient_buffer(&mut self, desc: BufferDescription) -> BufferHandle {
        let index = self.transient_buffers.len();
        self.transient_buffers.push(desc);
        BufferHandle::Transient(index)
    }

    pub fn create_transient_image(&mut self, desc: TransientImageDesc) -> ImageHandle {
        let index = self.transient_images.len();
        self.transient_images.push(desc);
        ImageHandle::Transient(index)
    }

    pub fn acquire_swapchain_image(&mut self, surface_handle: SurfaceHandle) -> ImageHandle {
        let index = self.swapchain_images.len();
        self.swapchain_images.push(surface_handle);
        ImageHandle::Swapchain(index)
    }

    pub fn add_pass(&mut self, pass: RenderPass) {
        self.passes.push(pass);
    }
}
