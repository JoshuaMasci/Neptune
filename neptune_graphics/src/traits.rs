use crate::interfaces::*;
use crate::render_graph::RenderGraph;
use crate::types::*;

pub trait InstanceTrait {
    fn create_surface(
        &self,
        name: &str,
        display_handle: raw_window_handle::RawDisplayHandle,
        window_handle: raw_window_handle::RawWindowHandle,
    ) -> Result<SurfaceHandle>;
    fn destroy_surface(&self, handle: SurfaceHandle);

    fn get_supported_devices(
        &self,
        surface: Option<SurfaceHandle>,
    ) -> Vec<(usize, PhysicalDeviceInfo)>;

    fn create_device(&self, device_index: usize, create_info: &DeviceCreateInfo) -> Result<Device>;

    fn get_surface_support(
        &self,
        device_index: usize,
        surface_handle: SurfaceHandle,
    ) -> Option<SwapchainSupportInfo>;
}

pub trait DeviceTrait {
    fn create_buffer(
        &mut self,
        name: &str,
        description: &BufferDescription,
    ) -> Result<BufferHandle>;
    fn destroy_buffer(&mut self, handle: BufferHandle);

    fn create_texture(
        &mut self,
        name: &str,
        description: &TextureDescription<[u32; 2]>,
    ) -> Result<TextureHandle>;
    fn destroy_texture(&mut self, handle: TextureHandle);

    fn create_sampler(
        &mut self,
        name: &str,
        description: &SamplerDescription,
    ) -> Result<SamplerHandle>;
    fn destroy_sampler(&mut self, handle: SamplerHandle);

    fn create_compute_pipeline(
        &mut self,
        name: &str,
        description: &ComputePipelineDescription,
    ) -> Result<ComputePipelineHandle>;
    fn destroy_compute_pipeline(&mut self, handle: ComputePipelineHandle);

    fn create_raster_pipeline(
        &mut self,
        name: &str,
        description: &RasterPipelineDescription,
    ) -> Result<RasterPipelineHandle>;
    fn destroy_raster_pipeline(&mut self, handle: RasterPipelineHandle);

    fn configure_surface(
        &mut self,
        surface_handle: SurfaceHandle,
        description: &SwapchainDescription,
    ) -> Result<()>;
    fn release_surface(&mut self, surface_handle: SurfaceHandle);

    //Render Graph Function
    fn submit_frame(&mut self, render_graph: &RenderGraph) -> Result<()>;
}
