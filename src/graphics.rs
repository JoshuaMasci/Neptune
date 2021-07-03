use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Swapchain},
};
use ash::*;

use ash::version::DeviceV1_0;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;

use gpu_allocator::*;

use winit;

use std::ffi::CStr;
use std::ffi::CString;

pub struct Graphics {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub pdevice: vk::PhysicalDevice,

    pub debug_utils_loader: DebugUtils,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,

    pub surface_loader: Surface,
    pub surface: vk::SurfaceKHR,

    pub device: ash::Device,
    pub present_queue: vk::Queue,
    pub allocator: VulkanAllocator,

    descriptor: BindlessDescriptor,

    //TODO: seprate with surface into object
    pub swapchain_loader: Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,

    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,
    pub command_fence: vk::Fence,

    pub image_ready_semaphore: vk::Semaphore,
    pub command_buffer_done_semaphore: vk::Semaphore,
}

pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl AppVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

pub struct AppInfo {
    pub name: String,
    pub version: AppVersion,
}

const SATURN_VERSION: u32 = vk::make_version(0, 0, 0);

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    use std::borrow::Cow;
    let callback_data = *p_callback_data;
    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!("Vulkan {:?}: {}", message_severity, message,);

    vk::FALSE
}

impl Graphics {
    pub fn new(
        app: &AppInfo,
    ) -> (
        Self,
        (winit::event_loop::EventLoop<()>, winit::window::Window),
    ) {
        let event_loop = winit::event_loop::EventLoop::new();
        let window = winit::window::WindowBuilder::new()
            .with_title(app.name.as_str())
            .with_resizable(true)
            .with_maximized(true)
            .build(&event_loop)
            .unwrap();
        let engine_name: CString = CString::new("Saturn Engine").unwrap();

        let entry = unsafe { Entry::new().unwrap() };

        let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let surface_extensions = ash_window::enumerate_required_extensions(&window).unwrap();
        let mut extension_names_raw = surface_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();
        extension_names_raw.push(DebugUtils::name().as_ptr());

        let temp_name = CString::new(app.name.as_str()).unwrap();
        let appinfo = vk::ApplicationInfo::builder()
            .application_name(temp_name.as_c_str())
            .application_version(SATURN_VERSION)
            .engine_name(engine_name.as_c_str())
            .engine_version(SATURN_VERSION)
            .api_version(vk::make_version(
                app.version.major,
                app.version.minor,
                app.version.patch,
            ));

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let ash_instance: ash::Instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("Instance creation error")
        };

        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils_loader = DebugUtils::new(&entry, &ash_instance);
        let debug_call_back = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap()
        };

        let surface =
            unsafe { ash_window::create_surface(&entry, &ash_instance, &window, None).unwrap() };
        let surface_loader = Surface::new(&entry, &ash_instance);

        let pdevices = unsafe {
            ash_instance
                .enumerate_physical_devices()
                .expect("Failed to enumerate devices")
        };

        println!("Vulkan Devices\n------------");
        for (i, pdevice) in pdevices.iter().enumerate() {
            let prop = unsafe { ash_instance.get_physical_device_properties(*pdevice) };
            let device_name =
                unsafe { CStr::from_ptr(prop.device_name.as_ptr()).to_str().unwrap() };
            println!("{}:{}", i, device_name);
        }
        println!("");

        let (pdevice, queue_family_index) = pdevices
            .iter()
            .map(|pdevice| unsafe {
                ash_instance
                    .get_physical_device_queue_family_properties(*pdevice)
                    .iter()
                    .enumerate()
                    .filter_map(|(index, ref info)| {
                        let supports_graphic_and_surface =
                            info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                && surface_loader
                                    .get_physical_device_surface_support(
                                        *pdevice,
                                        index as u32,
                                        surface,
                                    )
                                    .unwrap();
                        if supports_graphic_and_surface {
                            Some((*pdevice, index))
                        } else {
                            None
                        }
                    })
                    .next()
            })
            .flatten()
            .next()
            .expect("Couldn't find suitable device.");

        {
            let prop = unsafe { ash_instance.get_physical_device_properties(pdevice) };
            let device_name =
                unsafe { CStr::from_ptr(prop.device_name.as_ptr()).to_str().unwrap() };
            println!("Selected Device: {}", device_name);
        }

        let queue_family_index = queue_family_index as u32;
        let device_extension_names_raw = [Swapchain::name().as_ptr()];
        let features = vk::PhysicalDeviceFeatures {
            ..Default::default()
        };
        let priorities = [1.0];

        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&priorities)
            .build()];

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device: ash::Device = unsafe {
            ash_instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap()
        };

        let present_queue = unsafe { device.get_device_queue(queue_family_index as u32, 0) };

        let allocator = VulkanAllocator::new(&VulkanAllocatorCreateDesc {
            instance: ash_instance.clone(),
            device: device.clone(),
            physical_device: pdevice,
            debug_settings: Default::default(),
        });

        //TODO: read limits from device properties
        let push_size: u32 = 128;
        let bindings = vec![
            (vk::DescriptorType::STORAGE_BUFFER, 2048),
            (vk::DescriptorType::SAMPLED_IMAGE, 2048),
            (vk::DescriptorType::SAMPLER, 512),
        ];

        let descriptor = BindlessDescriptor::new(device.clone(), push_size, bindings);

        let swapchain_loader = Swapchain::new(&ash_instance, &device);

        let surface_size = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(pdevice, surface)
                .unwrap()
                .current_extent
        };

        let command_pool = unsafe {
            device
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::builder()
                        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                        .queue_family_index(queue_family_index),
                    None,
                )
                .unwrap()
        };

        let command_buffer = unsafe {
            device
                .allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::builder()
                        .command_buffer_count(2)
                        .command_pool(command_pool)
                        .level(vk::CommandBufferLevel::PRIMARY),
                )
                .unwrap()[0]
        };

        let command_fence = unsafe {
            device
                .create_fence(
                    &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
                .unwrap()
        };

        let image_ready_semaphore = unsafe {
            device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap()
        };

        let command_buffer_done_semaphore = unsafe {
            device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap()
        };

        let mut new_graphics = Self {
            entry,
            instance: ash_instance,
            pdevice: pdevice,

            debug_utils_loader,
            debug_call_back,

            surface_loader,
            surface,

            device,
            present_queue,
            allocator,
            descriptor,

            swapchain_loader,
            swapchain: vk::SwapchainKHR::null(),
            swapchain_images: Vec::new(),

            command_pool,
            command_buffer,
            command_fence,
            image_ready_semaphore,
            command_buffer_done_semaphore,
        };

        new_graphics.recreate_swapchain(surface_size);

        (new_graphics, (event_loop, window))
    }

    fn recreate_swapchain(&mut self, surface_size: vk::Extent2D) {
        unsafe {
            let surface_capabilities = self
                .surface_loader
                .get_physical_device_surface_capabilities(self.pdevice, self.surface)
                .unwrap();

            let desired_image_count = u32::min(
                surface_capabilities.min_image_count + 1,
                surface_capabilities.max_image_count,
            );

            let surface_format = self
                .surface_loader
                .get_physical_device_surface_formats(self.pdevice, self.surface)
                .unwrap()[0];

            let surface_resolution = match surface_capabilities.current_extent.width {
                std::u32::MAX => vk::Extent2D {
                    width: u32::min(
                        u32::max(
                            surface_size.width,
                            surface_capabilities.min_image_extent.width,
                        ),
                        surface_capabilities.max_image_extent.width,
                    ),
                    height: u32::min(
                        u32::max(
                            surface_size.height,
                            surface_capabilities.min_image_extent.height,
                        ),
                        surface_capabilities.max_image_extent.height,
                    ),
                },
                _ => surface_capabilities.current_extent,
            };

            let present_mode = self
                .surface_loader
                .get_physical_device_surface_present_modes(self.pdevice, self.surface)
                .unwrap()
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);

            let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
                .old_swapchain(self.swapchain)
                .surface(self.surface)
                .min_image_count(desired_image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_resolution)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(surface_capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            self.swapchain = self
                .swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap();

            self.swapchain_images = self
                .swapchain_loader
                .get_swapchain_images(self.swapchain)
                .unwrap();
        }
    }

    pub fn draw(&mut self) {
        //Get next image index
        let image_index: u32;
        loop {
            let (index, suboptimal) = unsafe {
                //TODO actually look at the vkResult
                self.swapchain_loader
                    .acquire_next_image(
                        self.swapchain,
                        std::u64::MAX,
                        self.image_ready_semaphore,
                        vk::Fence::null(),
                    )
                    .unwrap_or((0, true))
            };
            if !suboptimal {
                image_index = index;
                break;
            }

            //Needs to rebuild swapchain
            let surface_size = unsafe {
                self.surface_loader
                    .get_physical_device_surface_capabilities(self.pdevice, self.surface)
                    .unwrap()
                    .current_extent
            };
            self.recreate_swapchain(surface_size);
        }

        //~~Draw Frame~~
        unsafe {
            //Wait for fence
            self.device
                .wait_for_fences(&[self.command_fence], true, std::u64::MAX)
                .expect("Failed to wait for fence");
            self.device
                .reset_fences(&[self.command_fence])
                .expect("Failed to reset fence");

            self.device
                .begin_command_buffer(
                    self.command_buffer,
                    &vk::CommandBufferBeginInfo::builder()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .expect("Failed to start command buffer");

            //Transition Image
            let image_barrier = vk::ImageMemoryBarrier::builder()
                .image(self.swapchain_images[image_index as usize])
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

            self.device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier],
            );

            self.device
                .end_command_buffer(self.command_buffer)
                .expect("Failed to end command buffer");

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&[self.image_ready_semaphore])
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::TOP_OF_PIPE])
                .command_buffers(&[self.command_buffer])
                .signal_semaphores(&[self.command_buffer_done_semaphore])
                .build();
            self.device
                .queue_submit(self.present_queue, &[submit_info], self.command_fence)
                .expect("Failed to submit command buffer");
        }

        //Present Image
        let wait_semaphors = [self.command_buffer_done_semaphore];
        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphors) // &base.rendering_complete_semaphore)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            let _ = self
                .swapchain_loader
                .queue_present(self.present_queue, &present_info);
        }
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.device
                .destroy_semaphore(self.image_ready_semaphore, None);
            self.device
                .destroy_semaphore(self.command_buffer_done_semaphore, None);
            self.device.destroy_fence(self.command_fence, None);
            self.device.destroy_command_pool(self.command_pool, None);

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);

            self.descriptor.destroy();

            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }
}

struct BindlessDescriptor {
    device: ash::Device,

    descriptor_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,

    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
}

impl BindlessDescriptor {
    fn new(device: ash::Device, push_size: u32, bindings: Vec<(vk::DescriptorType, u32)>) -> Self {
        let descriptor_bindings: Vec<vk::DescriptorSetLayoutBinding> = bindings
            .iter()
            .enumerate()
            .map(|(i, (d_type, d_count))| {
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(i as u32)
                    .descriptor_type(*d_type)
                    .descriptor_count(*d_count)
                    .stage_flags(vk::ShaderStageFlags::ALL)
                    .build()
            })
            .collect();
        let pool_sizes: Vec<vk::DescriptorPoolSize> = bindings
            .iter()
            .map(|(d_type, d_count)| {
                vk::DescriptorPoolSize::builder()
                    .ty(*d_type)
                    .descriptor_count(*d_count)
                    .build()
            })
            .collect();
        let create_info =
            vk::DescriptorSetLayoutCreateInfo::builder().bindings(&descriptor_bindings);
        let descriptor_layout = unsafe {
            device
                .create_descriptor_set_layout(&create_info.build(), None)
                .unwrap()
        };

        let create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&[descriptor_layout])
            .push_constant_ranges(&[vk::PushConstantRange::builder()
                .offset(0)
                .size(push_size)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .build()])
            .build();
        let pipeline_layout = unsafe { device.create_pipeline_layout(&create_info, None).unwrap() };

        let create_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(1)
            .pool_sizes(&pool_sizes)
            .build();

        let descriptor_pool = unsafe { device.create_descriptor_pool(&create_info, None).unwrap() };

        let create_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&[descriptor_layout])
            .build();
        let descriptor_set = unsafe { device.allocate_descriptor_sets(&create_info).unwrap()[0] };

        Self {
            device,
            descriptor_layout,
            pipeline_layout,
            descriptor_pool,
            descriptor_set,
        }
    }

    //TODO: replace with drop at some point
    fn destroy(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);

            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);

            self.device
                .destroy_descriptor_set_layout(self.descriptor_layout, None);
        }
    }
}
