pub type ShaderHandle = u32;

pub struct Shader {
    handle: ShaderHandle,
    freed_list: std::sync::Mutex<Vec<ShaderHandle>>,
}

impl Shader {
    pub fn new_temp(handle: ShaderHandle) -> Self {
        Self {
            handle,
            freed_list: std::sync::Mutex::new(vec![]),
        }
    }

    pub fn get_handle(&self) -> ShaderHandle {
        self.handle
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        if let Ok(mut freed_list) = self.freed_list.lock() {
            freed_list.push(self.handle);
        }
    }
}

pub type VertexShader = Shader;
pub type FragmentShader = Shader;
pub type ComputeShader = Shader;
