mod interfaces;
mod traits;
mod types;
mod vulkan;

pub use interfaces::*;
use std::sync::Arc;
pub use types::*;

pub fn create_vulkan_instance(engine_info: &AppInfo, app_info: &AppInfo) -> Instance {
    let instance = Arc::new(crate::vulkan::Instance::new(engine_info, app_info).unwrap());
    Instance { instance }
}
