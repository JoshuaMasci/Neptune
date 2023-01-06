use crate::AshDevice;
use ash::vk;
use std::ffi::{CStr, CString};
use std::sync::Arc;

pub(crate) struct AshComputePipeline {
    device: Arc<AshDevice>,
    pub(crate) handle: vk::Pipeline,
}

impl AshComputePipeline {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        cache: vk::PipelineCache,
        layout: vk::PipelineLayout,
        code: &[u32],
    ) -> crate::Result<Self> {
        let module = match unsafe {
            device.create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(code), None)
        } {
            Ok(module) => module,
            Err(e) => return Err(crate::Error::VkError(e)),
        };

        let entry_point_name = CString::new("main").unwrap();

        let handle = match unsafe {
            device.create_compute_pipelines(
                cache,
                &[vk::ComputePipelineCreateInfo::builder()
                    .layout(layout)
                    .stage(
                        vk::PipelineShaderStageCreateInfo::builder()
                            .module(module)
                            .stage(vk::ShaderStageFlags::COMPUTE)
                            .name(&entry_point_name)
                            .build(),
                    )
                    .build()],
                None,
            )
        } {
            Ok(handles) => handles[0],
            Err((_, e)) => unsafe {
                device.destroy_shader_module(module, None);
                return Err(crate::Error::VkError(e));
            },
        };

        unsafe {
            device.destroy_shader_module(module, None);
        }

        Ok(Self { device, handle })
    }
}

impl Drop for AshComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.handle, None);
        }
    }
}
