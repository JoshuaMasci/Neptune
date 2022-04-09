use crate::buffer::BufferDescription;
use crate::resource::{Resource, ResourceDeleter};
use crate::texture::TextureDescription;
use crate::vulkan::buffer::Buffer;
use crate::vulkan::descriptor_set::DescriptorSet;
use crate::vulkan::texture::Texture;
use crate::{BufferUsages, TextureUsages};
use ash::vk;
use std::cell::RefCell;
use std::ffi::CStr;
use std::rc::Rc;

struct DeviceDrop(Rc<ash::Device>);
impl DeviceDrop {
    fn new(device: &Rc<ash::Device>) -> Self {
        Self { 0: device.clone() }
    }
}

impl Drop for DeviceDrop {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
        }
    }
}

pub struct Device {
    device: Rc<ash::Device>,
    resource_deleter: Rc<RefCell<ResourceDeleter>>,
    descriptor_set: DescriptorSet,

    allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
    device_drop: DeviceDrop,

    //TODO: find better place for this stuff?????
    graphics_queue: vk::Queue,
}

impl Device {
    pub(crate) fn new(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        graphics_queue_family_index: u32,
        frame_in_flight_count: u32,
    ) -> Self {
        //Device creation
        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe { CStr::from_ptr(device_properties.device_name.as_ptr()) }
            .to_str()
            .expect("Failed to convert CStr to string");

        println!(
            "Device: \n\tName: {}\n\tDriver: {:?}\n\tType: {:?}",
            device_name, device_properties.driver_version, device_properties.device_type,
        );

        let device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

        let mut features_vulkan_12 = vk::PhysicalDeviceVulkan12Features::builder()
            .descriptor_indexing(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .build();

        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::builder()
                .synchronization2(true)
                .build();

        let mut dynamic_rendering_features =
            vk::PhysicalDeviceDynamicRenderingFeaturesKHR::builder()
                .dynamic_rendering(true)
                .build();

        let priorities = &[1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_family_index)
            .queue_priorities(priorities)
            .build()];

        //Load all device functions
        let device = Rc::new(
            unsafe {
                instance.create_device(
                    physical_device,
                    &vk::DeviceCreateInfo::builder()
                        .queue_create_infos(&queue_info)
                        .enabled_extension_names(&device_extension_names_raw)
                        .push_next(&mut features_vulkan_12)
                        .push_next(&mut synchronization2_features)
                        .push_next(&mut dynamic_rendering_features),
                    None,
                )
            }
            .expect("Failed to initialize vulkan device"),
        );

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

        let allocator = Rc::new(RefCell::new(
            gpu_allocator::vulkan::Allocator::new(&gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: (*device).clone(),
                physical_device,
                debug_settings: gpu_allocator::AllocatorDebugSettings {
                    log_memory_information: false,
                    log_leaks_on_shutdown: true,
                    store_stack_traces: false,
                    log_allocations: false,
                    log_frees: false,
                    log_stack_traces: false,
                },
                buffer_device_address: false,
            })
            .expect("Failed to create device allocator"),
        ));

        const RESOURCE_DESCRIPTOR_COUNT: u32 = 2048;
        const SAMPLER_DESCRIPTOR_COUNT: u32 = 128;
        let descriptor_set = DescriptorSet::new(
            device.clone(),
            RESOURCE_DESCRIPTOR_COUNT,
            RESOURCE_DESCRIPTOR_COUNT,
            RESOURCE_DESCRIPTOR_COUNT,
            SAMPLER_DESCRIPTOR_COUNT,
        );

        let resource_deleter = ResourceDeleter::new(frame_in_flight_count as usize);

        let device_drop = DeviceDrop::new(&device);
        Self {
            device,
            allocator,
            resource_deleter,
            graphics_queue,
            device_drop,
            descriptor_set,
        }
    }

    pub fn create_buffer(
        &mut self,
        description: BufferDescription,
        name: &'static str,
    ) -> Resource<Buffer> {
        let is_storage = description.usage.contains(BufferUsages::STORAGE);

        let mut buffer = Buffer::new(
            self.device.clone(),
            self.allocator.clone(),
            description,
            name,
        );

        if is_storage {
            buffer.binding = Some(self.descriptor_set.bind_storage_buffer(&buffer));
        }

        Resource::new(buffer, self.resource_deleter.clone())
    }

    pub fn create_texture(
        &mut self,
        description: TextureDescription,
        name: &'static str,
    ) -> Resource<Texture> {
        let is_storage = description.usage.contains(TextureUsages::STORAGE);
        let is_sampled = description.usage.contains(TextureUsages::SAMPLED);

        let mut texture = Texture::new(
            self.device.clone(),
            self.allocator.clone(),
            description,
            name,
        );

        if is_storage {
            texture.storage_binding = Some(self.descriptor_set.bind_storage_image(&texture));
        }

        if is_sampled {
            texture.sampled_binding = Some(self.descriptor_set.bind_sampled_image(&texture));
        }

        Resource::new(texture, self.resource_deleter.clone())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle();
        }
    }
}
