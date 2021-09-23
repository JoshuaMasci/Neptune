use ash::*;
use gpu_allocator::*;

use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;

use crate::swapchain::Swapchain;
use crate::BufferId;
use std::borrow::BorrowMut;

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
    allocator: VulkanAllocator,
    device: DeviceDrop,

    //Non-Dropping Items
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
        const FRAMES_IN_FLIGHT: u32 = 3;

        let device_extension_names_raw = [extensions::khr::Swapchain::name().as_ptr()];
        let features = vk::PhysicalDeviceFeatures {
            ..Default::default()
        };
        let priorities = [1.0];

        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_index)
            .queue_priorities(&priorities)
            .build()];

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device: ash::Device = unsafe {
            instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap()
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_index, 0) };

        let allocator = VulkanAllocator::new(&VulkanAllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: pdevice,
            debug_settings: Default::default(),
        });

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

        Self {
            pdevice,
            device: DeviceDrop(device),
            allocator,
            graphics_queue,
            swapchain,
            frames,
            command_pool,
            frame_index: 0,
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

            //Transition Image
            let image_barrier = vk::ImageMemoryBarrier::builder()
                .image(self.swapchain.images[image_index as usize])
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_array_layer(0)
                        .layer_count(1)
                        .base_mip_level(0)
                        .level_count(1)
                        .build(),
                )
                .build();

            device.cmd_pipeline_barrier(
                frame.command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier],
            );

            device
                .end_command_buffer(frame.command_buffer)
                .expect("Failed to end command buffer recording");

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&[frame.image_ready_semaphore])
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::TOP_OF_PIPE])
                .command_buffers(&[frame.command_buffer])
                .signal_semaphores(&[frame.present_semaphore])
                .build();
            device
                .queue_submit(self.graphics_queue, &[submit_info], frame.frame_done_fence)
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
