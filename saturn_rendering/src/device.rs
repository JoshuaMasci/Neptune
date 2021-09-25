use ash::*;
use gpu_allocator::vulkan;
use gpu_allocator::*;

use crate::buffer::Buffer;
use crate::swapchain::Swapchain;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::ffi::CStr;

struct DeviceDrop(ash::Device);
impl Drop for DeviceDrop {
    fn drop(&mut self) {
        unsafe { self.0.destroy_device(None) };
    }
}

struct DeviceFrame {
    device: ash::Device,
    frame_done_fence: vk::Fence,

    image_ready_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,

    command_buffer: vk::CommandBuffer,
}

impl DeviceFrame {
    fn new(device: ash::Device, command_buffer: vk::CommandBuffer) -> Self {
        let frame_done_fence = unsafe {
            device.create_fence(
                &vk::FenceCreateInfo::builder()
                    .flags(vk::FenceCreateFlags::SIGNALED)
                    .build(),
                None,
            )
        }
        .expect("Failed to create fence");
        let image_ready_semaphore =
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::builder().build(), None) }
                .expect("Failed to create semaphore");
        let present_semaphore =
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::builder().build(), None) }
                .expect("Failed to create semaphore");
        Self {
            device,
            frame_done_fence,
            image_ready_semaphore,
            present_semaphore,
            command_buffer,
        }
    }
}

impl Drop for DeviceFrame {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_fence(self.frame_done_fence, None);
            self.device
                .destroy_semaphore(self.image_ready_semaphore, None);
            self.device.destroy_semaphore(self.present_semaphore, None);
        }
    }
}

pub struct Device {
    //Drop order items
    swapchain: Swapchain,
    frames: Vec<DeviceFrame>,
    resources: Resources,
    device: DeviceDrop,

    //Non-Dropping Items
    synchronization2: ash::extensions::khr::Synchronization2,
    command_pool: vk::CommandPool,
    pdevice: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
    frame_index: u32,
}

impl Device {
    pub(crate) fn new(
        instance: ash::Instance,
        pdevice: ash::vk::PhysicalDevice,
        graphics_queue_index: u32,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::extensions::khr::Surface,
    ) -> Self {
        let device_properties = unsafe { instance.get_physical_device_properties(pdevice) };
        let device_name = unsafe { CStr::from_ptr(device_properties.device_name.as_ptr()) }
            .to_str()
            .expect("Failed to convert CStr to string");
        println!("Using Device:\n{}", device_name);

        const FRAMES_IN_FLIGHT: u32 = 3;

        let device_extension_names_raw = vec![
            ash::extensions::khr::Swapchain::name().as_ptr(),
            ash::extensions::khr::Synchronization2::name().as_ptr(),
        ];

        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::builder()
                .synchronization2(true)
                .build();

        let mut descriptor_indexing = vk::PhysicalDeviceDescriptorIndexingFeatures::builder()
            .descriptor_binding_partially_bound(true)
            .shader_storage_buffer_array_non_uniform_indexing(true)
            .shader_storage_image_array_non_uniform_indexing(true)
            .shader_sampled_image_array_non_uniform_indexing(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .build();

        let priorities = &[1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_index)
            .queue_priorities(priorities)
            .build()];

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extension_names_raw)
            .push_next(&mut synchronization2_features)
            .push_next(&mut descriptor_indexing);

        let device: ash::Device = unsafe {
            instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap()
        };

        let resources = Resources::new(pdevice, &instance, &device);

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_index, 0) };

        let command_pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(graphics_queue_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None,
            )
        }
        .expect("Failed to create command pool");

        let command_buffers = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .command_buffer_count(FRAMES_IN_FLIGHT)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .build(),
            )
        }
        .expect("Failed to allocate command_buffers");

        let mut frames: Vec<DeviceFrame> = Vec::with_capacity(FRAMES_IN_FLIGHT as usize);
        for i in 0..FRAMES_IN_FLIGHT as usize {
            frames.push(DeviceFrame::new(device.clone(), command_buffers[i]));
        }

        let swapchain =
            Swapchain::new(&instance, &device, pdevice, surface, surface_loader.clone());

        let synchronization2 = ash::extensions::khr::Synchronization2::new(&instance, &device);

        Self {
            pdevice,
            device: DeviceDrop(device),
            resources,
            graphics_queue,
            swapchain,
            frames,
            command_pool,
            frame_index: 0,
            synchronization2,
        }
    }

    pub fn create_buffer() -> BufferId {
        BufferId(0)
    }

    pub fn destroy_buffer(_id: BufferId) {}

    pub fn draw(&mut self) {
        let device = self.device.0.clone();
        let frame = self
            .frames
            .get_mut(self.frame_index as usize)
            .expect("Failed to get current frame");

        unsafe {
            device
                .wait_for_fences(&[frame.frame_done_fence], true, u64::MAX)
                .expect("Failed to wait for fence");
            device
                .reset_fences(&[frame.frame_done_fence])
                .expect("Failed to reset fence");
        }

        let image_index = self
            .swapchain
            .acquire_next_image(frame.image_ready_semaphore);

        unsafe {
            device
                .begin_command_buffer(
                    frame.command_buffer,
                    &vk::CommandBufferBeginInfo::builder().build(),
                )
                .expect("Failed to begin command buffer recording");

            let iamge_barriers = &[vk::ImageMemoryBarrier2KHR::builder()
                .image(self.swapchain.images[image_index as usize])
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .src_access_mask(vk::AccessFlags2KHR::NONE)
                .src_stage_mask(vk::PipelineStageFlags2KHR::NONE)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_access_mask(vk::AccessFlags2KHR::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2KHR::NONE)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_array_layer(0)
                        .layer_count(1)
                        .base_mip_level(0)
                        .level_count(1)
                        .build(),
                )
                .build()];

            let dependency = vk::DependencyInfoKHR::builder()
                .image_memory_barriers(iamge_barriers)
                .build();
            self.synchronization2
                .cmd_pipeline_barrier2(frame.command_buffer, &dependency);

            device
                .end_command_buffer(frame.command_buffer)
                .expect("Failed to end command buffer recording");

            let wait_semaphore_infos = &[vk::SemaphoreSubmitInfoKHR::builder()
                .semaphore(frame.image_ready_semaphore)
                .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                .device_index(0)
                .build()];

            let command_buffer_infos = &[vk::CommandBufferSubmitInfoKHR::builder()
                .command_buffer(frame.command_buffer)
                .device_mask(0) //WTF is this?
                .build()];

            let signal_semaphore_infos = &[vk::SemaphoreSubmitInfoKHR::builder()
                .semaphore(frame.present_semaphore)
                .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                .device_index(0)
                .build()];

            let submit_info = vk::SubmitInfo2KHR::builder()
                .wait_semaphore_infos(wait_semaphore_infos)
                .command_buffer_infos(command_buffer_infos)
                .signal_semaphore_infos(signal_semaphore_infos)
                .build();
            self.synchronization2
                .queue_submit2(self.graphics_queue, &[submit_info], frame.frame_done_fence)
                .expect("Failed to queue command buffer");

            //Present Image
            let wait_semaphores = &[frame.present_semaphore];
            let swapchains = &[self.swapchain.handle];
            let image_indices = &[image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(wait_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            self.swapchain
                .loader
                .queue_present(self.graphics_queue, &present_info)
                .expect("Failed to queue present");
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.device.0.device_wait_idle().unwrap();
            self.device.0.destroy_command_pool(self.command_pool, None);
        }
    }
}

pub struct BufferId(u32);
pub struct TextureId(u32);

struct BufferResource {
    buffer: Buffer,
    binding_index: Option<u32>,
}

struct Resources {
    device: ash::Device,
    allocator: vulkan::Allocator,
    descriptor_set: DescriptorSet,

    buffers: HashMap<BufferId, BufferResource>,
}

impl Resources {
    pub(crate) fn new(
        pdevice: vk::PhysicalDevice,
        instance: &ash::Instance,
        device: &ash::Device,
    ) -> Self {
        let device = device.clone();

        let allocator = vulkan::Allocator::new(&vulkan::AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: pdevice,
            debug_settings: Default::default(),
            buffer_device_address: true,
        })
        .expect("Failed to create allocator");

        let descriptor_set = DescriptorSet::new(&device);

        Self {
            device,
            allocator,
            descriptor_set,
            buffers: HashMap::new(),
        }
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        //Don't bother unbinding descriptor when destroying device
        for (_index, mut buffer) in self.buffers.drain() {
            buffer.buffer.destroy(&self.device, &mut self.allocator);
        }
    }
}

struct DescriptorSet {
    device: ash::Device,
    layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set: vk::DescriptorSet,
}

impl DescriptorSet {
    fn new(device: &ash::Device) -> Self {
        let device = device.clone();

        //TODO: generate these from inputs
        let bindings = &[vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1024)
            .stage_flags(vk::ShaderStageFlags::all())
            .build()];

        let pool_sizes = &[vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1024)
            .build()];

        let layout = unsafe {
            device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                    .bindings(bindings)
                    .build(),
                None,
            )
        }
        .expect("Failed to create descriptor set layout");

        let pool = unsafe {
            device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
                    .max_sets(1)
                    .pool_sizes(pool_sizes)
                    .build(),
                None,
            )
        }
        .expect("Failed to create descriptor pool");

        let sets = unsafe {
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(pool)
                    .set_layouts(&[layout]),
            )
        }
        .expect("Failed to allocate descriptor sets");

        Self {
            device,
            layout,
            pool,
            set: sets[0],
        }
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.pool, None);
            self.device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}
