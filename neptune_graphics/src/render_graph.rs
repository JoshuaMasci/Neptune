use crate::{
    BufferDescription, BufferUsage, ComputeDispatch, ComputePipeline, Queue, RasterPassDescription,
    ShaderResourceAccess, SurfaceHandle, TextureDescription, Transfer, TransientBuffer,
    TransientImageSize, TransientTexture,
};

#[derive(Default, Debug)]
pub struct RenderGraph {
    pub(crate) swapchain_usage: Vec<SurfaceHandle>,
}

impl RenderGraph {
    pub(crate) fn acquire_swapchain_image(&mut self, surface_handle: SurfaceHandle) -> usize {
        let index = self.swapchain_usage.len();
        self.swapchain_usage.push(surface_handle);
        index
    }

    pub(crate) fn create_transient_buffer(
        &mut self,
        name: &str,
        description: &BufferDescription,
    ) -> TransientBuffer {
        todo!()
    }

    pub(crate) fn create_transient_buffer_init(
        &mut self,
        name: &str,
        usage: BufferUsage,
        data: &[u8],
    ) -> TransientBuffer {
        todo!()
    }

    pub(crate) fn create_transient_texture(
        &mut self,
        name: &str,
        description: &TextureDescription<TransientImageSize>,
    ) -> TransientTexture {
        todo!()
    }

    pub(crate) fn add_transfer_pass(&mut self, name: &str, queue: Queue, transfers: &[Transfer]) {}
    pub(crate) fn add_compute_pass(
        &mut self,
        name: &str,
        queue: Queue,
        pipeline: ComputePipeline,
        dispatch_size: &ComputeDispatch,
        resources: &[ShaderResourceAccess],
    ) {
    }
    pub(crate) fn add_raster_pass(&mut self, name: &str, description: &RasterPassDescription) {}
}
