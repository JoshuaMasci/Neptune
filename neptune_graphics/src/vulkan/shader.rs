use std::rc::Rc;

pub struct ShaderModule {
    device: Rc<ash::Device>,
    module: ash::vk::ShaderModule,
}

impl ShaderModule {
    pub(crate) fn new(device: Rc<ash::Device>, code: &[u32]) -> Self {
        let module = unsafe {
            device.create_shader_module(
                &ash::vk::ShaderModuleCreateInfo::builder()
                    .code(code)
                    .build(),
                None,
            )
        }
        .expect("Failed to create shader module");

        Self { device, module }
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.module, None);
        }
    }
}
