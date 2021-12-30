use ash::vk;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::sync::Arc;

pub struct RenderBackend {
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,

    device: ash::Device,
    graphics_queue: vk::Queue,
    device_allocator: Arc<RefCell<gpu_allocator::vulkan::Allocator>>,

    surface: vk::SurfaceKHR,
    swapchain: crate::swapchain::Swapchain,
}

impl RenderBackend {
    pub fn new(window: &winit::window::Window) -> Self {
        let app_name = CString::new("APP NAME HERE").unwrap();
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
        //TODO: abstract to struct with drop
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
        let debug_call_back = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap()
        };

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
            if surface_loader
                .get_physical_device_surface_support(
                    physical_device,
                    graphics_queue_family_index,
                    surface,
                )
                .expect("Failed to check device support")
                == false
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

        let swapchain = crate::swapchain::Swapchain::new(
            &instance,
            &device,
            physical_device,
            surface,
            surface_loader,
        );

        Self {
            instance,
            physical_device,
            device,
            graphics_queue,
            device_allocator: Arc::new(RefCell::new(device_allocator)),
            surface,
            swapchain,
        }
    }
}

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

    if message_severity != vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        println!("Vulkan {:?}: {}", message_severity, message,);
    }

    vk::FALSE
}
