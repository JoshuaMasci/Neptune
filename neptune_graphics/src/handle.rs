use std::fmt::{Debug, Formatter};

pub type HandleType = u32;

pub struct Handle {
    pub(crate) id: HandleType,
    freed_list: std::sync::Mutex<Vec<HandleType>>,
}

impl Handle {
    pub fn new_temp(id: HandleType) -> Self {
        Self {
            id,
            freed_list: std::sync::Mutex::new(vec![]),
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        if let Ok(mut freed_list) = self.freed_list.lock() {
            freed_list.push(self.id);
        }
    }
}

impl Debug for Handle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").field("id", &self.id).finish()
    }
}
