use crate::traits::*;
use crate::types::*;
use std::sync::Arc;

pub struct Instance {
    pub(crate) instance: Arc<dyn InstanceTrait>,
}

impl Instance {
    pub fn create_surface<
        T: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    >(
        &self,
        name: &str,
        window: &T,
    ) -> Result<Surface> {
        self.instance
            .create_surface(
                name,
                window.raw_display_handle(),
                window.raw_window_handle(),
            )
            .map(|handle| Surface(handle, self.instance.clone()))
    }

    pub fn select_and_create_device(
        &self,
        surface: Option<&Surface>,
        score_function: impl Fn(usize, &PhysicalDeviceInfo) -> Option<u32>,
    ) -> Result<Device> {
        let supported_devices = self
            .instance
            .get_supported_devices(surface.map(|surface| surface.0));

        let highest_scored_device: Option<usize> = supported_devices
            .iter()
            .map(|(index, physical_device_info)| {
                (index, score_function(*index, physical_device_info))
            })
            .max_by_key(|index_score| index_score.1)
            .and_then(|highest_scored_device| {
                if highest_scored_device.1.is_some() {
                    Some(*highest_scored_device.0)
                } else {
                    None
                }
            });

        if let Some(highest_scored_device) = highest_scored_device {
            self.instance.create_device(highest_scored_device, 3)
        } else {
            Err(crate::Error::NoSuitableDeviceFound)
        }
    }
}

pub struct Surface(SurfaceHandle, Arc<dyn InstanceTrait>);
impl Drop for Surface {
    fn drop(&mut self) {
        self.1.destroy_surface(self.0);
    }
}

pub struct Device {
    pub(crate) device: Arc<dyn DeviceTrait>,
}

impl Device {
    pub fn create_buffer(&self, name: &str, description: &BufferDescription) -> Result<Buffer> {
        self.device
            .create_buffer(name, description)
            .map(|handle| Buffer::Persistent(PersistentBuffer(handle, self.device.clone())))
    }

    pub fn create_texture(
        &self,
        name: &str,
        description: &TextureDescription,
    ) -> Result<PersistentTexture> {
        self.device
            .create_texture(name, description)
            .map(|handle| PersistentTexture(handle, self.device.clone()))
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

    pub fn render_frame(
        &self,
        render_fn: impl FnOnce(&mut dyn RenderGraphBuilderTrait),
    ) -> Result<()> {
        let mut render_graph = self.device.begin_frame();
        render_fn(render_graph.as_mut());
        self.device.end_frame(render_graph)
    }
}

pub struct PersistentBuffer(pub(crate) BufferHandle, Arc<dyn DeviceTrait>);
impl Drop for PersistentBuffer {
    fn drop(&mut self) {
        self.1.destroy_buffer(self.0);
    }
}

pub struct TransientBuffer(usize);

pub struct PersistentTexture(pub(crate) TextureHandle, Arc<dyn DeviceTrait>);
impl Drop for PersistentTexture {
    fn drop(&mut self) {
        self.1.destroy_texture(self.0);
    }
}

pub struct TransientTexture(usize);

pub enum Buffer {
    Persistent(PersistentBuffer),
    Transient(TransientBuffer),
}

impl Buffer {
    pub fn is_persistent(&self) -> bool {
        match self {
            Buffer::Persistent(_) => true,
            Buffer::Transient(_) => false,
        }
    }

    pub fn is_transient(&self) -> bool {
        match self {
            Buffer::Persistent(_) => false,
            Buffer::Transient(_) => true,
        }
    }
}

pub enum Texture {
    Persistent(PersistentTexture),
    Transient(TransientTexture),
}

impl Texture {
    pub fn is_persistent(&self) -> bool {
        match self {
            Texture::Persistent(_) => true,
            Texture::Transient(_) => false,
        }
    }

    pub fn is_transient(&self) -> bool {
        match self {
            Texture::Persistent(_) => false,
            Texture::Transient(_) => true,
        }
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
