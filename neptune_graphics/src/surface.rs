pub type SurfaceHandle = u32;

pub struct Surface {
    handle: SurfaceHandle,
    freed_list: std::sync::Mutex<Vec<SurfaceHandle>>,
}

impl Surface {
    pub fn new_temp(handle: SurfaceHandle) -> Self {
        Self {
            handle,
            freed_list: std::sync::Mutex::new(vec![]),
        }
    }

    pub fn get_handle(&self) -> SurfaceHandle {
        self.handle
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        if let Ok(mut freed_list) = self.freed_list.lock() {
            freed_list.push(self.handle);
        }
    }
}
