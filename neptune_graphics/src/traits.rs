use crate::interfaces::*;
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
    fn create_buffer(&self, name: &str, description: &BufferDescription) -> Result<BufferHandle>;
    fn destroy_buffer(&self, handle: BufferHandle);

    fn create_texture(&self, name: &str, description: &TextureDescription)
        -> Result<TextureHandle>;
    fn destroy_texture(&self, handle: TextureHandle);

    fn create_sampler(&self, name: &str, description: &SamplerDescription)
        -> Result<SamplerHandle>;
    fn destroy_sampler(&self, handle: SamplerHandle);

    fn create_compute_pipeline(
        &self,
        name: &str,
        description: &ComputePipelineDescription,
    ) -> Result<ComputePipelineHandle>;
    fn destroy_compute_pipeline(&self, handle: ComputePipelineHandle);

    fn create_raster_pipeline(
        &self,
        name: &str,
        description: &RasterPipelineDescription,
    ) -> Result<RasterPipelineHandle>;
    fn destroy_raster_pipeline(&self, handle: RasterPipelineHandle);

    fn configure_swapchain(
        &self,
        surface_handle: SurfaceHandle,
        description: &SwapchainDescription,
    ) -> Result<()>;

    fn begin_frame(&self) -> Box<dyn RenderGraphBuilderTrait>;
    fn end_frame(&self, render_graph: Box<dyn RenderGraphBuilderTrait>) -> Result<()>;
}

pub trait RenderGraphBuilderTrait {
    fn create_buffer(&mut self, name: &str, description: &BufferDescription) -> Buffer;
    fn create_texture(&mut self, name: &str, description: &TextureDescription) -> Texture;
    fn acquire_swapchain_texture(&mut self, surface: &Surface) -> Texture;

    fn add_transfer_pass(&mut self, name: &str, queue: Queue, transfers: &[Transfer]);

    fn add_compute_pass(
        &mut self,
        name: &str,
        queue: Queue,
        pipeline: ComputePipeline,
        dispatch_size: &ComputeDispatch,
        resources: &[ShaderResourceAccess],
    );

    fn add_raster_pass(
        &mut self,
        name: &str,
        description: &RasterPassDescription,
        raster_commands: &[RasterCommand],
    );
}
