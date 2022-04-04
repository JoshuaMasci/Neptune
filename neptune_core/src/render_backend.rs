use crate::render_graph::pipeline_cache::PipelineCache;
use crate::render_graph::render_graph::{ImageAccessType, RenderGraph, RenderPassBuilder};
use crate::render_graph::Renderer;
use crate::transfer_queue::TransferQueue;
use crate::vulkan::debug_messenger::DebugMessenger;
use crate::vulkan::swapchain::Swapchain;
use crate::vulkan::{DescriptorSet, Image, ImageDescription};
use ash::vk;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::rc::Rc;

//TODO: this
#[allow(dead_code)]
pub struct ResourceDeleter {
    base: Rc<ash::Device>,
    allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
}

#[derive(Clone)]
pub struct RenderDevice {
    pub base: Rc<ash::Device>,
    pub allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
    pub surface: Rc<ash::extensions::khr::Surface>,
    pub swapchain: Rc<ash::extensions::khr::Swapchain>,
    pub dynamic_rendering: Rc<ash::extensions::khr::DynamicRendering>,
    pub synchronization2: Rc<ash::extensions::khr::Synchronization2>,
    pub push_descriptor: Rc<ash::extensions::khr::PushDescriptor>,
}
#[allow(dead_code)]
pub struct RenderBackend {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_messenger: DebugMessenger,

    physical_device: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
    pub device: RenderDevice,

    surface: vk::SurfaceKHR,
    swapchain: Swapchain,
    swapchain_image_index: u32,

    descriptor_set: DescriptorSet,
    pipeline_layout: vk::PipelineLayout,

    //Temp Device Frame Objects
    command_pool: vk::CommandPool,
    transfer_command_buffer: vk::CommandBuffer,
    graphics_command_buffer: vk::CommandBuffer,

    transfer_done_semaphore: vk::Semaphore,
    image_ready_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
    frame_done_fence: vk::Fence,

    pipeline_cache: PipelineCache,
    transfer_queue: TransferQueue,
    graph_renderer: crate::render_graph::Renderer,
}

impl RenderBackend {
    pub fn new(window: &winit::window::Window) -> Self {
        let app_name = CString::new("Neptune Editor").unwrap();
        let app_version = vk::make_api_version(0, 0, 0, 0);
        let engine_name: CString = CString::new("Neptune Engine").unwrap();
        let engine_version = vk::make_api_version(0, 0, 0, 0);

        let entry = unsafe { ash::Entry::load() }.expect("Failed to create Vulkan Entry!");

        let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let surface_extensions = ash_window::enumerate_required_extensions(window)
            .expect("Failed to get required surface extensions");
        let mut extension_names_raw = surface_extensions.iter().map(|ext| ext).collect::<Vec<_>>();
        extension_names_raw.push(ash::extensions::ext::DebugUtils::name());
        extension_names_raw.push(ash::extensions::khr::GetPhysicalDeviceProperties2::name());

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
        let debug_messenger =
            crate::vulkan::debug_messenger::DebugMessenger::new(&entry, &instance);

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

        println!(
            "Device: \n\tName: {}\n\tDriver: {:?}\n\tType: {:?}",
            device_name, device_properties.driver_version, device_properties.device_type,
        );

        let device_extension_names_raw = vec![
            ash::extensions::khr::Swapchain::name().as_ptr(),
            ash::extensions::khr::Synchronization2::name().as_ptr(),
            ash::extensions::khr::PushDescriptor::name().as_ptr(), //I am not sure if I want to keep this long term
            ash::extensions::khr::DynamicRendering::name().as_ptr(),
        ];

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
        let base = unsafe {
            instance.create_device(
                physical_device,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_info)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut synchronization2_features)
                    .push_next(&mut dynamic_rendering_features),
                None,
            )
        }
        .expect("Failed to initialize vulkan device");

        let allocator = Rc::new(RefCell::new(
            gpu_allocator::vulkan::Allocator::new(&gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: base.clone(),
                physical_device,
                debug_settings: Default::default(),
                buffer_device_address: false,
            })
            .expect("Failed to create device allocator"),
        ));

        let device = RenderDevice {
            base: Rc::new(base.clone()),
            allocator,
            surface: Rc::new(surface_loader),
            swapchain: Rc::new(ash::extensions::khr::Swapchain::new(&instance, &base)),
            dynamic_rendering: Rc::new(ash::extensions::khr::DynamicRendering::new(
                &instance, &base,
            )),
            synchronization2: Rc::new(ash::extensions::khr::Synchronization2::new(
                &instance, &base,
            )),
            push_descriptor: Rc::new(ash::extensions::khr::PushDescriptor::new(&instance, &base)),
        };

        let graphics_queue =
            unsafe { device.base.get_device_queue(graphics_queue_family_index, 0) };

        //Swapchain
        let swapchain = crate::vulkan::swapchain::Swapchain::new(&device, physical_device, surface);

        let descriptor_set = DescriptorSet::new(device.clone(), 2048, 2048, 2048, 128);
        let pipeline_layout = unsafe {
            device.base.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[descriptor_set.get_layout()])
                    .push_constant_ranges(&[vk::PushConstantRange::builder()
                        .size(128)
                        .stage_flags(vk::ShaderStageFlags::ALL)
                        .build()]),
                None,
            )
        }
        .expect("Failed to create pipeline layout");

        //TEMP Device frame stuff
        let command_pool = unsafe {
            device.base.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(graphics_queue_family_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None,
            )
        }
        .expect("Failed to create command pool");

        let command_buffers = unsafe {
            device.base.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .command_buffer_count(2)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .build(),
            )
        }
        .expect("Failed to allocate command_buffers");

        let frame_done_fence = unsafe {
            device.base.create_fence(
                &vk::FenceCreateInfo::builder()
                    .flags(vk::FenceCreateFlags::SIGNALED)
                    .build(),
                None,
            )
        }
        .expect("Failed to create fence");
        let transfer_done_semaphore = unsafe {
            device
                .base
                .create_semaphore(&vk::SemaphoreCreateInfo::builder().build(), None)
        }
        .expect("Failed to create semaphore");
        let image_ready_semaphore = unsafe {
            device
                .base
                .create_semaphore(&vk::SemaphoreCreateInfo::builder().build(), None)
        }
        .expect("Failed to create semaphore");
        let present_semaphore = unsafe {
            device
                .base
                .create_semaphore(&vk::SemaphoreCreateInfo::builder().build(), None)
        }
        .expect("Failed to create semaphore");

        let pipeline_cache = PipelineCache::new(device.clone(), pipeline_layout);
        let transfer_queue = TransferQueue::new(device.clone());
        let graph_renderer = Renderer::new();

        Self {
            entry,
            instance,
            debug_messenger,
            physical_device,
            device,
            graphics_queue,
            surface,
            swapchain,
            swapchain_image_index: 0,

            descriptor_set,
            pipeline_layout,

            command_pool,
            transfer_command_buffer: command_buffers[0],
            graphics_command_buffer: command_buffers[1],
            transfer_done_semaphore,
            image_ready_semaphore,
            present_semaphore,
            frame_done_fence,
            pipeline_cache,
            transfer_queue,
            graph_renderer,
        }
    }

    fn begin_frame(&mut self) -> Option<vk::CommandBuffer> {
        unsafe {
            self.device
                .base
                .wait_for_fences(&[self.frame_done_fence], true, u64::MAX)
                .expect("Failed to wait for fence")
        };

        self.descriptor_set.commit_changes();

        let image_index = self
            .swapchain
            .acquire_next_image(self.image_ready_semaphore);

        if image_index.is_none() {
            println!("No Image available, returning");
            return None;
        }
        let image_index = image_index.unwrap();

        unsafe {
            self.device
                .base
                .reset_fences(&[self.frame_done_fence])
                .expect("Failed to reset fence")
        };

        unsafe {
            self.device
                .base
                .begin_command_buffer(
                    self.graphics_command_buffer,
                    &vk::CommandBufferBeginInfo::builder().build(),
                )
                .expect("Failed to begin command buffer recording");
        }

        self.swapchain_image_index = image_index;

        Some(self.graphics_command_buffer)
    }

    fn end_frame(&mut self) {
        unsafe {
            self.device
                .base
                .end_command_buffer(self.graphics_command_buffer)
                .expect("Failed to end command buffer recording");

            //Build transfer command buffer
            {
                self.device
                    .base
                    .begin_command_buffer(
                        self.transfer_command_buffer,
                        &vk::CommandBufferBeginInfo::builder().build(),
                    )
                    .expect("Failed to begin command buffer recording");
                self.transfer_queue
                    .commit_transfers(self.transfer_command_buffer);
                self.device
                    .base
                    .end_command_buffer(self.transfer_command_buffer)
                    .expect("Failed to end command buffer recording");

                let command_buffer_infos = &[vk::CommandBufferSubmitInfoKHR::builder()
                    .command_buffer(self.transfer_command_buffer)
                    .device_mask(0)
                    .build()];

                let signal_semaphore_infos = &[vk::SemaphoreSubmitInfoKHR::builder()
                    .semaphore(self.transfer_done_semaphore)
                    .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                    .device_index(0)
                    .build()];

                let submit_info = vk::SubmitInfo2KHR::builder()
                    .command_buffer_infos(command_buffer_infos)
                    .signal_semaphore_infos(signal_semaphore_infos)
                    .build();
                self.device
                    .synchronization2
                    .queue_submit2(self.graphics_queue, &[submit_info], vk::Fence::null())
                    .expect("Failed to queue command buffer");
            }

            let wait_semaphore_infos = &[
                vk::SemaphoreSubmitInfoKHR::builder()
                    .semaphore(self.image_ready_semaphore)
                    .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                    .device_index(0)
                    .build(),
                vk::SemaphoreSubmitInfoKHR::builder()
                    .semaphore(self.transfer_done_semaphore)
                    .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
                    .device_index(0)
                    .build(),
            ];

            let command_buffer_infos = &[vk::CommandBufferSubmitInfoKHR::builder()
                .command_buffer(self.graphics_command_buffer)
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
            self.device
                .synchronization2
                .queue_submit2(self.graphics_queue, &[submit_info], self.frame_done_fence)
                .expect("Failed to queue command buffer");

            //Present Image
            let wait_semaphores = &[self.present_semaphore];
            let swapchains = &[self.swapchain.handle];
            let image_indices = &[self.swapchain_image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(wait_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            let _ = self
                .device
                .swapchain
                .queue_present(self.graphics_queue, &present_info);
        }
    }

    pub fn render(&mut self, render: impl FnOnce(&mut RenderGraph)) -> bool {
        if let Some(command_buffer) = self.begin_frame() {
            let mut render_graph = RenderGraph {
                passes: vec![],
                buffers: vec![],
                images: vec![],
            };

            //Add swapchain image
            let swapchain_image_handle = render_graph.import_image(
                self.swapchain.images[self.swapchain_image_index as usize].clone(),
                ImageAccessType::None,
            );

            render(&mut render_graph);

            //Transition swapchain image to present
            render_graph.add_render_pass(
                RenderPassBuilder::new("PresentTransition")
                    .image(swapchain_image_handle, ImageAccessType::Present),
            );

            //TODO: submit render_graph
            self.graph_renderer.render(
                &self.device,
                command_buffer,
                self.descriptor_set.get_set(),
                render_graph,
                &mut self.pipeline_cache,
                &mut self.transfer_queue,
            );

            self.end_frame();
            true
        } else {
            false
        }
    }
}

impl Drop for RenderBackend {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.base.device_wait_idle();

            let _ = self
                .device
                .base
                .free_command_buffers(self.command_pool, &[self.graphics_command_buffer]);
            let _ = self
                .device
                .base
                .destroy_command_pool(self.command_pool, None);

            self.device.base.destroy_fence(self.frame_done_fence, None);
            self.device
                .base
                .destroy_semaphore(self.image_ready_semaphore, None);
            self.device
                .base
                .destroy_semaphore(self.present_semaphore, None);

            self.device
                .base
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
