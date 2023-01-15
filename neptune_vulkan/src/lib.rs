mod buffer;
mod compute_pipeline;
mod debug_utils;
mod descriptor_set;
mod device;
mod instance;
mod render_graph;
mod resource_manager;
mod sampler;
mod surface;
mod swapchain;
mod texture;
mod transfer_queue;

pub use buffer::*;
pub use device::*;
pub use instance::*;
pub use render_graph::*;
pub use sampler::*;
pub use swapchain::*;
pub use texture::*;

use std::sync::{Arc, LockResult, Mutex, MutexGuard};

pub use ash;
use slotmap::SlotMap;

#[macro_use]
extern crate log;

pub type MemoryLocation = gpu_allocator::MemoryLocation;

//TODO: Replace this with bespoke error types for each operation
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("PlaceHolder Error")]
    Unknown,

    #[error("Vk error: {0}")]
    VkError(ash::vk::Result),

    #[error("Gpu alloc error: {0}")]
    GpuAllocError(gpu_allocator::AllocationError),

    #[error("Error: {0}")]
    StringError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

slotmap::new_key_type! {
    pub struct SurfaceHandle;
    pub struct SwapchainHandle;

    pub struct BufferHandle;
    pub struct TextureHandle;
    pub struct SamplerHandle;

    pub struct ComputePipelineHandle;
    pub struct RasterPipelineHandle;
}

pub struct GpuResource<K: slotmap::Key, T> {
    pub(crate) handle: K,
    pub(crate) pool: GpuResourcePool<K, T>,
}

impl<K: slotmap::Key, T> GpuResource<K, T> {
    pub(crate) fn new(handle: K, pool: GpuResourcePool<K, T>) -> Self {
        Self { handle, pool }
    }
}

impl<K: slotmap::Key, T> Drop for GpuResource<K, T> {
    fn drop(&mut self) {
        let _ = self
            .pool
            .lock()
            .unwrap()
            .remove(self.handle)
            .expect("Failed to find key in slotmap");
    }
}

#[derive(Clone)]
pub struct GpuResourcePool<K: slotmap::Key, T>(Arc<Mutex<SlotMap<K, T>>>);

impl<K: slotmap::Key, T> GpuResourcePool<K, T> {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(SlotMap::with_key())))
    }

    pub(crate) fn lock(&self) -> LockResult<MutexGuard<SlotMap<K, T>>> {
        self.0.lock()
    }
}

// Test for abstraction
pub type PublicBufferHandle = u64;
pub trait DeviceImpl {
    fn create_buffer(&self, size: u64) -> Option<PublicBufferHandle>;
    fn destroy_buffer(&self, handle: PublicBufferHandle);

    fn begin_frame(&self) -> Box<dyn RenderGraphImpl>;
    fn end_frame(&self, render_graph: Box<dyn RenderGraphImpl>);
}
pub trait RenderGraphImpl {
    fn use_buffer(&mut self, handle: &PublicBuffer);
}

pub struct TestDevice {}
impl DeviceImpl for TestDevice {
    fn create_buffer(&self, size: u64) -> Option<PublicBufferHandle> {
        Some(0)
    }
    fn destroy_buffer(&self, handle: PublicBufferHandle) {}

    fn begin_frame(&self) -> Box<dyn RenderGraphImpl> {
        Box::new(TestRenderGraph {
            function_callbacks: vec![],
        })
    }

    fn end_frame(&self, render_graph: Box<dyn RenderGraphImpl>) {
        let _ = render_graph;
    }
}

type RasterCommandCallback = dyn FnOnce(&mut dyn RenderGraphImpl);

pub struct TestRenderGraph {
    function_callbacks: Vec<Box<RasterCommandCallback>>,
}
impl RenderGraphImpl for TestRenderGraph {
    fn use_buffer(&mut self, handle: &PublicBuffer) {
        warn!("Use Buffer: {}", handle.0);
    }
}

pub type TestDeviceType = TestDevice;

pub struct PublicBuffer(PublicBufferHandle, Arc<TestDeviceType>);
impl Drop for PublicBuffer {
    fn drop(&mut self) {
        self.1.destroy_buffer(self.0);
    }
}

pub struct PublicDevice {
    device_impl: Arc<TestDeviceType>,
}

pub struct PublicDevice2 {
    device_impl: Arc<dyn DeviceImpl>,
}

impl PublicDevice {
    pub fn new() -> Self {
        Self {
            device_impl: Arc::new(TestDevice {}),
        }
    }

    pub fn create_buffer(&self, size: u64) -> Option<PublicBuffer> {
        self.device_impl
            .create_buffer(size)
            .map(|handle| PublicBuffer(handle, self.device_impl.clone()))
    }

    fn render_frame(&self, render_fn: impl FnOnce(&mut dyn RenderGraphImpl)) {
        let mut render_graph = self.device_impl.begin_frame();
        render_fn(render_graph.as_mut());
        self.device_impl.end_frame(render_graph);
    }
}

pub fn test_abstract_api() {
    let device = PublicDevice::new();
    let buffer = device.create_buffer(16).unwrap();

    for _ in 0..10 {
        device.render_frame(|render_graph| {
            render_graph.use_buffer(&buffer);
        });
    }
}
