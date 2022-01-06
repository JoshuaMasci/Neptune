use crate::image::Image;
use std::cell::RefCell;
use std::rc::Rc;

pub struct TransferQueue {
    device: ash::Device,
    device_allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
    synchronization2: ash::extensions::khr::Synchronization2,
}

impl TransferQueue {
    pub(crate) fn copy_to_image<T>(&mut self, image: &Image, data: &[T]) {}
}
