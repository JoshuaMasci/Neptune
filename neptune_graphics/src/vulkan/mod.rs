mod debug_utils;
mod device;
mod instance;

slotmap::new_key_type! {
    pub struct AshSurfaceHandle;
    pub struct AshBufferHandle;
    pub struct AshTextureHandle;
    pub struct AshSamplerHandle;
    pub struct AshComputePipelineHandle;
    pub struct AshRasterPipelineHandle;
    pub struct AshSwapchainHandle;
}

pub use instance::Instance;
pub use device::Device;
