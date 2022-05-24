use ash::*;
use std::rc::Rc;

pub struct SwapchainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupportDetails {
    pub fn new(
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::extensions::khr::Surface,
    ) -> Self {
        let capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .unwrap()
        };

        let formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .unwrap()
        };

        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface)
                .unwrap()
        };

        Self {
            capabilities,
            formats,
            present_modes,
        }
    }

    pub fn get_size(&self, desired_size: vk::Extent2D) -> vk::Extent2D {
        if self.capabilities.current_extent.width != u32::MAX {
            return self.capabilities.current_extent;
        }

        vk::Extent2D::builder()
            .width(u32::clamp(
                desired_size.width,
                self.capabilities.min_image_extent.width,
                self.capabilities.max_image_extent.width,
            ))
            .height(u32::clamp(
                desired_size.height,
                self.capabilities.min_image_extent.height,
                self.capabilities.max_image_extent.height,
            ))
            .build()
    }

    pub fn get_format(&self, desired_format: vk::Format) -> vk::SurfaceFormatKHR {
        *self
            .formats
            .iter()
            .find(|surface_format| surface_format.format == desired_format)
            .unwrap_or(&self.formats[0])
    }

    pub fn get_present_mode(&self, desired_mode: vk::PresentModeKHR) -> vk::PresentModeKHR {
        *self
            .present_modes
            .iter()
            .find(|&&present_mode| present_mode == desired_mode)
            .unwrap_or(&vk::PresentModeKHR::FIFO)
    }

    pub fn get_image_count(&self, desired_count: u32) -> u32 {
        u32::clamp(
            desired_count,
            self.capabilities.min_image_count,
            self.capabilities.max_image_count,
        )
    }
}

pub(crate) struct SwapchainImage {
    pub(crate) format: vk::Format,
    pub(crate) size: [u32; 2],
    pub(crate) handle: vk::Image,
    pub(crate) view: vk::ImageView,
}

pub(crate) struct Swapchain {
    invalid: bool,
    physical_device: vk::PhysicalDevice,
    device: Rc<ash::Device>,

    surface: vk::SurfaceKHR,
    surface_ext: Rc<ash::extensions::khr::Surface>,

    swapchain_ext: Rc<ash::extensions::khr::Swapchain>,
    pub(crate) handle: vk::SwapchainKHR,

    pub(crate) mode: vk::PresentModeKHR,
    pub(crate) images: Vec<SwapchainImage>,
}

impl Swapchain {
    pub fn new(
        physical_device: vk::PhysicalDevice,
        device: Rc<ash::Device>,
        surface_ext: Rc<ash::extensions::khr::Surface>,
        surface: vk::SurfaceKHR,
        swapchain_ext: Rc<ash::extensions::khr::Swapchain>,
    ) -> Self {
        //Temp values
        let handle = vk::SwapchainKHR::null();
        let mode = vk::PresentModeKHR::FIFO;
        let images = Vec::new();

        let mut new = Self {
            invalid: true,
            physical_device,
            device,
            surface_ext,
            swapchain_ext,
            surface,
            handle,
            mode,
            images,
        };
        new.rebuild();
        new
    }

    fn rebuild(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            for image in self.images.drain(..) {
                self.device.destroy_image_view(image.view, None)
            }
        }

        let swapchain_support =
            SwapchainSupportDetails::new(self.physical_device, self.surface, &self.surface_ext);

        self.mode = swapchain_support.get_present_mode(vk::PresentModeKHR::MAILBOX);
        let surface_format = swapchain_support.get_format(vk::Format::B8G8R8A8_UNORM);
        let image_count = swapchain_support.get_image_count(3);

        //TODO: get size
        let surface_size = swapchain_support.get_size(vk::Extent2D::builder().build());

        let old_swapchain = self.handle;

        let image_usage = vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT;

        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface)
            .min_image_count(image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_size)
            .image_array_layers(1)
            .image_usage(image_usage)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(swapchain_support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.mode)
            .clipped(true)
            .old_swapchain(old_swapchain)
            .build();

        self.handle = unsafe { self.swapchain_ext.create_swapchain(&create_info, None) }
            .expect("Failed to create swapchain!");

        let images: Vec<vk::Image> =
            unsafe { self.swapchain_ext.get_swapchain_images(self.handle) }
                .expect("Failed to get swapchain images");

        let views: Vec<vk::ImageView> = images
            .iter()
            .map(|&image| unsafe {
                self.device
                    .create_image_view(
                        &vk::ImageViewCreateInfo::builder()
                            .format(surface_format.format)
                            .image(image)
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .components(vk::ComponentMapping {
                                r: vk::ComponentSwizzle::IDENTITY,
                                g: vk::ComponentSwizzle::IDENTITY,
                                b: vk::ComponentSwizzle::IDENTITY,
                                a: vk::ComponentSwizzle::IDENTITY,
                            })
                            .subresource_range(vk::ImageSubresourceRange {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                base_mip_level: 0,
                                level_count: 1,
                                base_array_layer: 0,
                                layer_count: 1,
                            }),
                        None,
                    )
                    .expect("Failed to create swapchain image views")
            })
            .collect();

        self.images = images
            .iter()
            .zip(views.iter())
            .map(|(&handle, &view)| SwapchainImage {
                format: surface_format.format,
                size: [surface_size.width, surface_size.height],
                handle,
                view,
            })
            .collect();

        unsafe {
            self.swapchain_ext.destroy_swapchain(old_swapchain, None);
        }

        println!(
            "Swapchain Rebuild: \n\tCount: {} \n\tMode: {:?}\n\tFormat: {:?}\n\tExtend: {:?}",
            self.images.len(),
            self.mode,
            surface_format,
            surface_size,
        );
        self.invalid = false;
    }

    pub fn acquire_next_image(&mut self, image_ready_semaphore: vk::Semaphore) -> Option<u32> {
        if !self.invalid {
            let result = unsafe {
                self.swapchain_ext.acquire_next_image(
                    self.handle,
                    u64::MAX,
                    image_ready_semaphore,
                    vk::Fence::null(),
                )
            };

            if let Ok((index, suboptimal)) = result {
                if !suboptimal {
                    return Some(index);
                }
            }

            self.invalid = true;
        }

        //Rebuild if the size is valid
        let capabilities = unsafe {
            self.surface_ext
                .get_physical_device_surface_capabilities(self.physical_device, self.surface)
                .unwrap()
        };

        if capabilities.min_image_extent.width >= 1 || capabilities.min_image_extent.height >= 1 {
            self.rebuild();
        }

        None
    }

    pub fn present_image(&mut self, queue: vk::Queue, image_index: u32, semaphore: vk::Semaphore) {
        unsafe {
            let wait_semaphores = &[semaphore];
            let swapchains = &[self.handle];
            let image_indices = &[image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(wait_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);
            let _ = self.swapchain_ext.queue_present(queue, &present_info);
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for image in self.images.drain(..) {
                self.device.destroy_image_view(image.view, None)
            }
        }

        unsafe {
            self.swapchain_ext.destroy_swapchain(self.handle, None);
        }
    }
}
