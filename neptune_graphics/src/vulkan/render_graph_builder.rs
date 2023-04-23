use crate::traits::RenderGraphBuilderTrait;
use crate::vulkan::device::{AshDevice, AshQueue};
use crate::vulkan::instance::AshSurfaceSwapchains;
use crate::vulkan::AshSurfaceHandle;
use crate::{
    BufferDescription, ComputeDispatch, ComputePipeline, Queue, RasterCommand,
    RasterPassDescription, ShaderResourceAccess, SurfaceHandle, TextureDescription, Transfer,
    TransientBuffer, TransientTexture,
};
use ash::vk;
use slotmap::{KeyData, SlotMap};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

pub(crate) struct AshRenderGraphBuilder {
    device: Arc<AshDevice>,
    surfaces_swapchains: Arc<Mutex<SlotMap<AshSurfaceHandle, AshSurfaceSwapchains>>>,

    used_swapchains: Vec<AshSurfaceHandle>,
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
        self.used_swapchains
            .push(AshSurfaceHandle(KeyData::from_ffi(surface as u64)));
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
        // unsafe {
        //     for swapchain_handle in &self.used_swapchains {
        //         let mut lock = self.surfaces_swapchains.lock().unwrap();
        //
        //         let surface_swapchain = lock.get_mut(*swapchain_handle).unwrap();
        //         let swapchain = surface_swapchain
        //             .swapchains
        //             .get_mut(&self.device.physical_device)
        //             .unwrap();
        //
        //         let semaphore = self
        //             .device
        //             .handle
        //             .create_semaphore(&vk::SemaphoreCreateInfo::builder().build(), None)
        //             .unwrap();
        //
        //         let fence = self
        //             .device
        //             .handle
        //             .create_fence(&vk::FenceCreateInfo::builder().build(), None)
        //             .unwrap();
        //
        //         let index = swapchain
        //             .acquire_next_image(2000, semaphore, fence)
        //             .unwrap();
        //         swapchain
        //             .present_image(self.device.primary_queue.queue, index, semaphore)
        //             .unwrap();
        //
        //         self.device
        //             .handle
        //             .wait_for_fences(&[fence], true, 2000)
        //             .unwrap();
        //
        //         self.device.handle.destroy_semaphore(semaphore, None);
        //         self.device.handle.destroy_fence(fence, None);
        //     }
        // }

        Ok(())
    }
}
