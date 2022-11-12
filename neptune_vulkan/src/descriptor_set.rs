use crate::{AshDevice, Error};
use ash::vk;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub trait DescriptorSetLayout {
    const LAYOUT_HASH: u64;
    fn create_layout(device: &Arc<AshDevice>) -> ash::prelude::VkResult<vk::DescriptorSetLayout>;
    fn create_pool_sizes() -> Vec<vk::DescriptorPoolSize>;
    fn write_descriptor_set(&self, device: &Arc<AshDevice>, descriptor_set: vk::DescriptorSet);
}

pub struct DescriptorSet<T: DescriptorSetLayout> {
    _data: T,
    descriptor_set: vk::DescriptorSet,
    _descriptor_layout: vk::DescriptorSetLayout,
    pool_inner: Arc<Mutex<DescriptorPoolInner>>,
}

impl<T: DescriptorSetLayout> Drop for DescriptorSet<T> {
    fn drop(&mut self) {
        self.pool_inner
            .lock()
            .unwrap()
            .available_sets
            .push(self.descriptor_set);
        trace!("Drop DescriptorSet");
    }
}

//DescriptorPool calls must be externally synchronized so it will be wrapped in mutex
struct DescriptorPoolInner {
    device: Arc<AshDevice>,
    layout: vk::DescriptorSetLayout,
    handle: vk::DescriptorPool,
    available_sets: Vec<vk::DescriptorSet>,
}

impl Drop for DescriptorPoolInner {
    fn drop(&mut self) {
        unsafe { self.device.destroy_descriptor_pool(self.handle, None) }
        trace!("Drop DescriptorPool");
    }
}

#[derive(Clone)]
pub struct DescriptorPool<T: DescriptorSetLayout> {
    _phantom_type: std::marker::PhantomData<T>,
    inner: Arc<Mutex<DescriptorPoolInner>>,
}

impl<T: DescriptorSetLayout> DescriptorPool<T> {
    pub(crate) fn new(
        device: &Arc<AshDevice>,
        descriptor_set_layout_pool: &Arc<DescriptorSetLayoutPool>,
        max_sets: u32,
    ) -> crate::Result<Self> {
        let mut sizes = T::create_pool_sizes();
        sizes
            .iter_mut()
            .for_each(|size| size.descriptor_count *= max_sets);
        let handle = match unsafe {
            device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(max_sets)
                    .pool_sizes(&sizes),
                None,
            )
        } {
            Ok(handle) => handle,
            Err(e) => return Err(Error::VkError(e)),
        };

        let layout = descriptor_set_layout_pool.get_layout::<T>();

        Ok(Self {
            _phantom_type: Default::default(),

            inner: Arc::new(Mutex::new(DescriptorPoolInner {
                device: device.clone(),
                layout,
                handle,
                available_sets: vec![],
            })),
        })
    }

    pub fn create(
        &self,
        name: &str,
        descriptor_set_data: T,
    ) -> ash::prelude::VkResult<Arc<DescriptorSet<T>>> {
        let inner_clone = self.inner.clone();
        let mut inner_lock = self.inner.lock().unwrap();
        if let Some(descriptor_set) = inner_lock.available_sets.pop() {
            Ok(descriptor_set)
        } else {
            unsafe {
                inner_lock
                    .device
                    .allocate_descriptor_sets(
                        &vk::DescriptorSetAllocateInfo::builder()
                            .descriptor_pool(inner_lock.handle)
                            .set_layouts(&[inner_lock.layout])
                            .build(),
                    )
                    .map(|allocated_set| allocated_set[0])
            }
        }
        .map(|descriptor_set| {
            descriptor_set_data.write_descriptor_set(&inner_lock.device, descriptor_set);
            Arc::new(DescriptorSet {
                _data: descriptor_set_data,
                descriptor_set,
                _descriptor_layout: inner_lock.layout,
                pool_inner: inner_clone,
            })
        })
    }
}

pub struct DescriptorSetLayoutPool {
    device: Arc<AshDevice>,
    layouts: std::sync::Mutex<HashMap<u64, vk::DescriptorSetLayout>>,
}

impl DescriptorSetLayoutPool {
    pub(crate) fn new(device: &Arc<AshDevice>) -> Self {
        Self {
            device: device.clone(),
            layouts: Default::default(),
        }
    }

    pub(crate) fn get_layout<T: DescriptorSetLayout>(&self) -> vk::DescriptorSetLayout {
        let mut layouts = self.layouts.lock().unwrap();

        if let Some(layout) = layouts.get(&T::LAYOUT_HASH) {
            *layout
        } else {
            let layout =
                T::create_layout(&self.device).expect("Failed to create vk::DescriptorSetLayout");
            if let Some(layout) = layouts.insert(T::LAYOUT_HASH, layout) {
                warn!(
                    "DescriptorSetLayout for hash({}) already exists, DescriptorSetLayout {:?} will be leaked",
                    T::LAYOUT_HASH, layout
                );
            }
            layout
        }
    }
}

impl Drop for DescriptorSetLayoutPool {
    fn drop(&mut self) {
        let write = self.layouts.get_mut().unwrap();
        for (_, layout) in write.drain() {
            unsafe { self.device.destroy_descriptor_set_layout(layout, None) }
        }
        trace!("Drop DescriptorSetLayoutPool");
    }
}
