use crate::traits::RenderGraphBuilderTrait;
use crate::vulkan::device::{AshDevice, AshQueue};
use crate::vulkan::instance::AshSurfaceSwapchains;
use crate::vulkan::AshSurfaceHandle;
use crate::{
    BufferDescription, ComputeDispatch, ComputePipeline, Queue, RasterCommand,
    RasterPassDescription, ShaderResourceAccess, SurfaceHandle, TextureDescription, Transfer,
    TransientBuffer, TransientTexture,
};
use slotmap::SlotMap;
use std::sync::{Arc, Mutex};

pub(crate) struct AshRenderGraphBuilder {
    device: Arc<AshDevice>,
    surfaces_swapchains: Arc<Mutex<SlotMap<AshSurfaceHandle, AshSurfaceSwapchains>>>,

    used_swapchains: Vec<SurfaceHandle>,
}

impl AshRenderGraphBuilder {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        surfaces_swapchains: Arc<Mutex<SlotMap<AshSurfaceHandle, AshSurfaceSwapchains>>>,
    ) -> Self {
        Self {
            device,
            surfaces_swapchains,
            used_swapchains: Vec::new(),
        }
    }
}

impl RenderGraphBuilderTrait for AshRenderGraphBuilder {
    fn create_buffer(&mut self, name: &str, description: &BufferDescription) -> TransientBuffer {
        TransientBuffer(0)
    }

    fn create_texture(&mut self, name: &str, description: &TextureDescription) -> TransientTexture {
        TransientTexture(0)
    }

    fn acquire_swapchain_texture(&mut self, surface: SurfaceHandle) -> TransientTexture {
        self.used_swapchains.push(surface);
        TransientTexture(1)
    }

    fn add_transfer_pass(&mut self, name: &str, queue: Queue, transfers: &[Transfer]) {
        todo!()
    }

    fn add_compute_pass(
        &mut self,
        name: &str,
        queue: Queue,
        pipeline: ComputePipeline,
        dispatch_size: &ComputeDispatch,
        resources: &[ShaderResourceAccess],
    ) {
        todo!()
    }

    fn add_raster_pass(
        &mut self,
        name: &str,
        description: &RasterPassDescription,
        raster_commands: &[RasterCommand],
    ) {
        todo!()
    }

    fn execute_graph(&mut self) -> crate::Result<()> {
        Ok(())
    }
}
