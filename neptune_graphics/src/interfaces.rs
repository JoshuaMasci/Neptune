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
    ) -> Option<PhysicalDevice> {
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

        highest_scored_device.map(|highest_scored_device| PhysicalDevice {
            instance: self.instance.clone(),
            device_index: highest_scored_device,
            device_info: supported_devices[highest_scored_device].1.clone(),
        })
    }
}

pub struct PhysicalDevice {
    instance: Arc<dyn InstanceTrait>,
    device_index: usize,
    device_info: PhysicalDeviceInfo,
}

impl PhysicalDevice {
    pub fn index(&self) -> usize {
        self.device_index
    }

    pub fn get_device_info(&self) -> PhysicalDeviceInfo {
        self.device_info.clone()
    }

    pub fn get_surface_support(&self, surface: &Surface) -> Option<SwapchainSupportInfo> {
        self.instance
            .get_surface_support(self.device_index, surface.0)
    }

    pub fn create(self, create_info: &DeviceCreateInfo) -> Result<Device> {
        self.instance.create_device(self.device_index, create_info)
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

    pub fn create_texture(&self, name: &str, description: &TextureDescription) -> Result<Texture> {
        self.device
            .create_texture(name, description)
            .map(|handle| Texture::Persistent(PersistentTexture(handle, self.device.clone())))
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

    pub fn configure_swapchain(
        &self,
        surface: &Surface,
        description: &SwapchainDescription,
    ) -> Result<()> {
        self.device.configure_swapchain(surface.0, description)
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

pub struct RenderGraphBuilder(Box<dyn RenderGraphBuilderTrait>);
impl RenderGraphBuilder {}
