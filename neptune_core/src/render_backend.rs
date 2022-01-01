use ash::vk;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::rc::Rc;

pub struct RenderBackend {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_messenger: crate::debug_messenger::DebugMessenger,

    physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    graphics_queue: vk::Queue,
    pub device_allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
    synchronization2: ash::extensions::khr::Synchronization2,

    push_descriptor: ash::extensions::khr::PushDescriptor,

    surface: vk::SurfaceKHR,
    swapchain: crate::swapchain::Swapchain,

    //Temp Device Frame Objects
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    image_ready_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
    frame_done_fence: vk::Fence,
}

impl RenderBackend {
    pub fn new(window: &winit::window::Window) -> Self {
        let app_name = CString::new("Neptune Editor").unwrap();
        let app_version = vk::make_api_version(0, 0, 0, 0);
        let engine_name: CString = CString::new("Neptune Engine").unwrap();
        let engine_version = vk::make_api_version(0, 0, 0, 0);

        let entry = unsafe { ash::Entry::new().unwrap() };

        let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let surface_extensions = ash_window::enumerate_required_extensions(window)
            .expect("Failed to get required surface extensions");
        let mut extension_names_raw = surface_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();
        extension_names_raw.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        extension_names_raw
            .push(ash::extensions::khr::GetPhysicalDeviceProperties2::name().as_ptr());

        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name.as_c_str())
            .application_version(app_version)
            .engine_name(engine_name.as_c_str())
            .engine_version(engine_version)
            .api_version(vk::API_VERSION_1_2);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let instance: ash::Instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Failed to create vulkan instance")
        };

        //Validation Messages
        let debug_messenger = crate::debug_messenger::DebugMessenger::new(&entry, &instance);

        //Surface creation
        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, window, None)
                .expect("Failed to create vulkan surface")
        };

        //Device Selection
        let devices = unsafe { instance.enumerate_physical_devices() }
            .expect("Failed to enumerate vulkan physical devices");

        //TODO: select device and queues!!!
        let (physical_device, graphics_queue_family_index) = (devices[0], 0u32);

        unsafe {
            if !surface_loader
                .get_physical_device_surface_support(
                    physical_device,
                    graphics_queue_family_index,
                    surface,
                )
                .expect("Failed to check device support")
            {
                panic!("Selected device doesn't support the surface");
            }
        }

        //Device creation
        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe { CStr::from_ptr(device_properties.device_name.as_ptr()) }
            .to_str()
            .expect("Failed to convert CStr to string");
        println!("Using Device:\n{}", device_name);

        let device_extension_names_raw = vec![
            ash::extensions::khr::Swapchain::name().as_ptr(),
            ash::extensions::khr::Synchronization2::name().as_ptr(),
            ash::extensions::khr::PushDescriptor::name().as_ptr(), //I am not sure if I want to keep this long term
        ];

        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::builder()
                .synchronization2(true)
                .build();

        let priorities = &[1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_family_index)
            .queue_priorities(priorities)
            .build()];

        let device: ash::Device = unsafe {
            instance
                .create_device(
                    physical_device,
                    &vk::DeviceCreateInfo::builder()
                        .queue_create_infos(&queue_info)
                        .enabled_extension_names(&device_extension_names_raw)
                        .push_next(&mut synchronization2_features),
                    None,
                )
                .expect("Failed to initialize vulkan device")
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

        let device_allocator =
            gpu_allocator::vulkan::Allocator::new(&gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device,
                debug_settings: Default::default(),
                buffer_device_address: false,
            })
            .expect("Failed to create device allocator");

        let synchronization2 = ash::extensions::khr::Synchronization2::new(&instance, &device);

        let push_descriptor = ash::extensions::khr::PushDescriptor::new(&instance, &device);

        //Swapchain
        let swapchain = crate::swapchain::Swapchain::new(
            &instance,
            &device,
            physical_device,
            surface,
            surface_loader,
        );

        //TEMP Device frame stuff
        let command_pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(graphics_queue_family_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None,
            )
        }
        .expect("Failed to create command pool");

        let command_buffer = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .command_buffer_count(1)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .build(),
            )
        }
        .expect("Failed to allocate command_buffers")[0];

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
            entry,
            instance,
            debug_messenger,
            physical_device,
            device,
            graphics_queue,
            device_allocator: Rc::new(RefCell::new(device_allocator)),
            synchronization2,
            push_descriptor,
            surface,
            swapchain,

            command_pool,
            command_buffer,
            image_ready_semaphore,
            present_semaphore,
            frame_done_fence,
        }
    }

    pub fn draw_black(&mut self) {
        unsafe {
            self.device
                .wait_for_fences(&[self.frame_done_fence], true, u64::MAX)
                .expect("Failed to wait for fence")
        };

        let image_index = self
            .swapchain
            .acquire_next_image(self.image_ready_semaphore);

        if image_index.is_none() {
            println!("No Image available, returning");
            return;
        }
        let image_index = image_index.unwrap();

        unsafe {
            self.device
                .reset_fences(&[self.frame_done_fence])
                .expect("Failed to reset fence")
        };

        unsafe {
            self.device
                .begin_command_buffer(
                    self.command_buffer,
                    &vk::CommandBufferBeginInfo::builder().build(),
                )
                .expect("Failed to begin command buffer recording");

            let image_barriers = &[vk::ImageMemoryBarrier2KHR::builder()
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
                .image_memory_barriers(image_barriers)
                .build();
            self.synchronization2
                .cmd_pipeline_barrier2(self.command_buffer, &dependency);

            self.device
                .end_command_buffer(self.command_buffer)
                .expect("Failed to end command buffer recording");

            let wait_semaphore_infos = &[vk::SemaphoreSubmitInfoKHR::builder()
                .semaphore(self.image_ready_semaphore)
                .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                .device_index(0)
                .build()];

            let command_buffer_infos = &[vk::CommandBufferSubmitInfoKHR::builder()
                .command_buffer(self.command_buffer)
                .device_mask(0)
                .build()];

            let signal_semaphore_infos = &[vk::SemaphoreSubmitInfoKHR::builder()
                .semaphore(self.present_semaphore)
                .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                .device_index(0)
                .build()];

            let submit_info = vk::SubmitInfo2KHR::builder()
                .wait_semaphore_infos(wait_semaphore_infos)
                .command_buffer_infos(command_buffer_infos)
                .signal_semaphore_infos(signal_semaphore_infos)
                .build();
            self.synchronization2
                .queue_submit2(self.graphics_queue, &[submit_info], self.frame_done_fence)
                .expect("Failed to queue command buffer");

            //Present Image
            let wait_semaphores = &[self.present_semaphore];
            let swapchains = &[self.swapchain.handle];
            let image_indices = &[image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(wait_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            let result = self
                .swapchain
                .loader
                .queue_present(self.graphics_queue, &present_info);
        }
    }
}

impl Drop for RenderBackend {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            let _ = self
                .device
                .free_command_buffers(self.command_pool, &[self.command_buffer]);
            let _ = self.device.destroy_command_pool(self.command_pool, None);

            self.device.destroy_fence(self.frame_done_fence, None);
            self.device
                .destroy_semaphore(self.image_ready_semaphore, None);
            self.device.destroy_semaphore(self.present_semaphore, None);
        }
    }
}
