mod types;

use std::sync::Arc;
pub use types::*;

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

    fn create_surface(
        &self,
        name: &str,
        display_handle: raw_window_handle::RawDisplayHandle,
        window_handle: raw_window_handle::RawWindowHandle,
    ) -> Result<SurfaceHandle>;
    fn destroy_surface(&self, handle: SurfaceHandle);

    fn create_swapchain(
        &self,
        name: &str,
        surface: SurfaceHandle,
        description: &SwapchainDescription,
    ) -> Result<SwapchainHandle>;
    fn destroy_swapchain(&self, handle: SwapchainHandle);
    fn update_swapchain(
        &self,
        handle: SwapchainHandle,
        description: &SwapchainDescription,
    ) -> Result<()>;

    fn begin_frame(&self) -> Box<dyn RenderGraphImpl>;
    fn end_frame(&self, render_graph: Box<dyn RenderGraphImpl>) -> Result<()>;
}

pub trait RenderGraphImpl {
    fn import_buffer(&mut self, handle: &Buffer) -> BufferGraphResource;
    fn import_texture(&mut self, handle: &Texture) -> TextureGraphResource;

    fn create_buffer(&mut self, name: &str, description: &BufferDescription)
        -> BufferGraphResource;
    fn create_texture(
        &mut self,
        name: &str,
        description: &TextureDescription,
    ) -> TextureGraphResource;

    fn acquire_swapchain_texture(&mut self, swapchain: &Swapchain) -> TextureGraphResource;

    fn add_transfer_pass(&mut self, name: &str, queue: Queue, transfers: &[Transfer]);

    fn add_compute_pass(
        &mut self,
        name: &str,
        queue: Queue,
        pipeline: ComputePipeline,
        dispatch_size: ComputeDispatch,
        resources: &[ShaderResourceAccess],
    );

    //TODO: Add method to register resource accesses
    fn add_raster_pass(
        &mut self,
        name: &str,
        description: &RasterPassDescription,
        build_fn: RasterCommandBufferBuildFunction,
    );
}

pub type RasterCommandBufferBuildFunction = Box<dyn FnOnce(&mut dyn RasterCommandBuffer)>;
pub struct RasterBuildFunction(pub RasterCommandBufferBuildFunction);
impl RasterBuildFunction {
    pub fn new(build_fn: impl FnOnce(&mut dyn RasterCommandBuffer) + 'static) -> Self {
        Self(Box::new(build_fn))
    }
}

pub trait RasterCommandBuffer {
    fn set_viewport(
        &mut self,
        origin: [f32; 2],
        extent: [f32; 2],
        depth_range: std::ops::Range<f32>,
    );
    fn set_scissor(&mut self, offset: [u32; 2], extent: [u32; 2]);

    fn set_pipeline(&mut self, pipeline: &RasterPipeline);
    fn set_vertex_buffer(&mut self, buffers: &[(BufferGraphResource, u64)]);
    fn set_index_buffer(&mut self, buffer: BufferGraphResource, format: IndexFormat);
    fn set_resources(&mut self, resources: &[ShaderResourceAccess]);

    fn draw(&mut self, vertex_range: std::ops::Range<u32>, instance_range: std::ops::Range<u32>);
    fn draw_indexed(
        &mut self,
        index_range: std::ops::Range<u32>,
        base_vertex: i32,
        instance_range: std::ops::Range<u32>,
    );

    fn draw_indirect(&mut self, buffer: BufferGraphResource, offset: u64);
    fn draw_indexed_indirect(&mut self, buffer: BufferGraphResource, offset: u64);
}

pub struct Device {
    device: Arc<dyn DeviceTrait>,
}

impl Device {
    pub fn create_buffer(&self, name: &str, description: &BufferDescription) -> Result<Buffer> {
        self.device
            .create_buffer(name, description)
            .map(|handle| Buffer(handle, self.device.clone()))
    }

    pub fn create_texture(&self, name: &str, description: &TextureDescription) -> Result<Texture> {
        self.device
            .create_texture(name, description)
            .map(|handle| Texture(handle, self.device.clone()))
    }

    pub fn create_sampler(&self, name: &str, description: &SamplerDescription) -> Result<Sampler> {
        self.device
            .create_sampler(name, description)
            .map(|handle| Sampler(handle, self.device.clone()))
    }

    pub fn create_compute_pipeline(
        &self,
        name: &str,
        description: &ComputePipelineDescription,
    ) -> Result<ComputePipeline> {
        self.device
            .create_compute_pipeline(name, description)
            .map(|handle| ComputePipeline(handle, self.device.clone()))
    }

    pub fn create_raster_pipeline(
        &self,
        name: &str,
        description: &RasterPipelineDescription,
    ) -> Result<RasterPipeline> {
        self.device
            .create_raster_pipeline(name, description)
            .map(|handle| RasterPipeline(handle, self.device.clone()))
    }

    pub fn create_surface<
        T: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    >(
        &self,
        name: &str,
        window: &T,
    ) -> Result<Surface> {
        self.device
            .create_surface(
                name,
                window.raw_display_handle(),
                window.raw_window_handle(),
            )
            .map(|handle| Surface(handle, self.device.clone()))
    }

    pub fn create_swapchain(
        &self,
        name: &str,
        surface: &Surface,
        description: &SwapchainDescription,
    ) -> Result<Swapchain> {
        self.device
            .create_swapchain(name, surface.0, description)
            .map(|handle| Swapchain(handle, self.device.clone()))
    }

    pub fn render_frame(&self, render_fn: impl FnOnce(&mut dyn RenderGraphImpl)) -> Result<()> {
        let mut render_graph = self.device.begin_frame();
        render_fn(render_graph.as_mut());
        self.device.end_frame(render_graph)
    }
}

pub struct Buffer(BufferHandle, Arc<dyn DeviceTrait>);
impl Drop for Buffer {
    fn drop(&mut self) {
        self.1.destroy_buffer(self.0);
    }
}

pub struct Texture(TextureHandle, Arc<dyn DeviceTrait>);
impl Drop for Texture {
    fn drop(&mut self) {
        self.1.destroy_texture(self.0);
    }
}

pub struct Sampler(SamplerHandle, Arc<dyn DeviceTrait>);
impl Drop for Sampler {
    fn drop(&mut self) {
        self.1.destroy_sampler(self.0);
    }
}

pub struct ComputePipeline(ComputePipelineHandle, Arc<dyn DeviceTrait>);
impl Drop for ComputePipeline {
    fn drop(&mut self) {
        self.1.destroy_compute_pipeline(self.0);
    }
}

pub struct RasterPipeline(RasterPipelineHandle, Arc<dyn DeviceTrait>);
impl Drop for RasterPipeline {
    fn drop(&mut self) {
        self.1.destroy_raster_pipeline(self.0);
    }
}

pub struct Surface(SurfaceHandle, Arc<dyn DeviceTrait>);
impl Drop for Surface {
    fn drop(&mut self) {
        self.1.destroy_surface(self.0);
    }
}

pub struct Swapchain(SwapchainHandle, Arc<dyn DeviceTrait>);
impl Swapchain {
    pub fn update(&self, description: &SwapchainDescription) -> Result<()> {
        self.1.update_swapchain(self.0, description)
    }
}
impl Drop for Swapchain {
    fn drop(&mut self) {
        self.1.destroy_swapchain(self.0);
    }
}
