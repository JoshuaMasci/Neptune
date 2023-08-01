use crate::{
    BufferDescription, BufferHandle, BufferUsage, ComputePipeline, Queue, SurfaceHandle,
    TextureDescription, TextureHandle, TransientBuffer, TransientImageSize, TransientTexture,
};

pub(crate) enum BufferResource {
    Persistent(BufferHandle),
    Transient(TransientBuffer),
}

pub(crate) enum TextureResource {
    Persistent(TextureHandle),
    Transient(TransientTexture),
    Swapchain(usize),
}

pub(crate) enum ComputeDispatch {
    Size([u32; 3]),
    Indirect { buffer: BufferResource, offset: u64 },
}

pub(crate) struct FramebufferDescription {
    pub(crate) color_attachments: Vec<()>,
    pub(crate) depth_stencil_attachment: Option<()>,
    pub(crate) input_attachments: Vec<()>,
}

#[derive(Default, Debug)]
pub struct RenderGraph {
    pub(crate) swapchain_usage: Vec<SurfaceHandle>,
}

impl RenderGraph {
    pub(crate) fn acquire_swapchain_image(
        &mut self,
        surface_handle: SurfaceHandle,
    ) -> TextureResource {
        let index = self.swapchain_usage.len();
        self.swapchain_usage.push(surface_handle);
        TextureResource::Swapchain(index)
    }

    pub(crate) fn create_transient_buffer(
        &mut self,
        name: &str,
        description: &BufferDescription,
    ) -> BufferResource {
        todo!()
    }

    pub(crate) fn create_transient_buffer_init(
        &mut self,
        name: &str,
        usage: BufferUsage,
        data: &[u8],
    ) -> BufferResource {
        todo!()
    }

    pub(crate) fn create_transient_texture(
        &mut self,
        name: &str,
        description: &TextureDescription<TransientImageSize>,
    ) -> TextureResource {
        todo!()
    }

    pub(crate) fn add_transfer_pass(&mut self, name: &str, queue: Queue, transfers: &[()]) {}
    pub(crate) fn add_compute_pass(
        &mut self,
        name: &str,
        queue: Queue,
        pipeline: ComputePipeline,
        dispatch_size: &ComputeDispatch,
        resources: &[()],
    ) {
    }
    pub(crate) fn add_raster_pass(&mut self, name: &str, framebuffer: FramebufferDescription) {}
}
