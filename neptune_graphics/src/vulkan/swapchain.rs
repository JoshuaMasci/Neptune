use crate::vulkan::instance::AshSurface;
use ash::vk;
use log::warn;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct SwapchainConfig {
    pub(crate) image_count: u32,
    pub(crate) format: vk::SurfaceFormatKHR,
    pub(crate) present_mode: vk::PresentModeKHR,
    pub(crate) usage: vk::ImageUsageFlags,
    pub(crate) composite_alpha: vk::CompositeAlphaFlagsKHR,
}

pub(crate) struct AshSwapchainImage {
    device: Arc<ash::Device>,
    #[allow(unused)]
    pub(crate) handle: vk::Image,
    pub(crate) view: vk::ImageView,
}

impl AshSwapchainImage {
    pub(crate) fn new(
        device: Arc<ash::Device>,
        handle: vk::Image,
        format: vk::Format,
    ) -> ash::prelude::VkResult<Self> {
        let view = unsafe {
            device.create_image_view(
                &vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .image(handle)
                    .components(vk::ComponentMapping::default())
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    }),
                None,
            )?
        };

        Ok(Self {
            device,
            handle,
            view,
        })
    }
}

impl Drop for AshSwapchainImage {
    fn drop(&mut self) {
        unsafe { self.device.destroy_image_view(self.view, None) }
    }
}

pub(crate) struct AshSwapchainInstance {
    swapchain_extension: Arc<ash::extensions::khr::Swapchain>,
    pub(crate) handle: vk::SwapchainKHR,
    pub(crate) images: Vec<AshSwapchainImage>,
}

impl AshSwapchainInstance {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        device: Arc<ash::Device>,
        swapchain_extension: Arc<ash::extensions::khr::Swapchain>,
        surface: vk::SurfaceKHR,
        swapchain_config: &SwapchainConfig,
        swapchain_extent: vk::Extent2D,
        transform: vk::SurfaceTransformFlagsKHR,
        image_count: u32,
        old_swapchain: vk::SwapchainKHR,
    ) -> ash::prelude::VkResult<Self> {
        let handle = unsafe {
            swapchain_extension.create_swapchain(
                &vk::SwapchainCreateInfoKHR::builder()
                    .surface(surface)
                    .min_image_count(image_count)
                    .image_color_space(swapchain_config.format.color_space)
                    .image_format(swapchain_config.format.format)
                    .image_extent(swapchain_extent)
                    .image_array_layers(1)
                    .image_usage(swapchain_config.usage)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .pre_transform(transform)
                    .composite_alpha(swapchain_config.composite_alpha)
                    .present_mode(swapchain_config.present_mode)
                    .clipped(true)
                    .old_swapchain(old_swapchain),
                None,
            )?
        };

        let images: Vec<AshSwapchainImage> =
            unsafe { swapchain_extension.get_swapchain_images(handle) }
                .unwrap()
                .drain(..)
                .map(|image| {
                    AshSwapchainImage::new(device.clone(), image, swapchain_config.format.format)
                        .unwrap()
                })
                .collect();

        Ok(Self {
            swapchain_extension,
            handle,
            images,
        })
    }
}

impl Drop for AshSwapchainInstance {
    fn drop(&mut self) {
        warn!("Dropping Swapchain");

        self.images.clear();
        unsafe {
            self.swapchain_extension
                .destroy_swapchain(self.handle, None);
        }
    }
}

pub(crate) struct AshSwapchain {
    device: Arc<ash::Device>,
    swapchain_extension: Arc<ash::extensions::khr::Swapchain>,
    surface_extension: Arc<ash::extensions::khr::Surface>,
    physical_device: vk::PhysicalDevice,

    current_config: SwapchainConfig,
    current_swapchain: AshSwapchainInstance,

    suboptimal: bool,

    //Should drop last
    surface: Arc<AshSurface>,
}

impl AshSwapchain {
    pub(crate) fn new(
        device: Arc<ash::Device>,
        swapchain_extension: Arc<ash::extensions::khr::Swapchain>,
        surface_extension: Arc<ash::extensions::khr::Surface>,
        physical_device: vk::PhysicalDevice,
        surface: Arc<AshSurface>,
        swapchain_config: SwapchainConfig,
    ) -> ash::prelude::VkResult<Self> {
        let (current_extent, current_transform, image_count) =
            get_swapchain_extent_transform_count(
                &surface_extension,
                physical_device,
                surface.get_handle(),
                swapchain_config.image_count,
            )?;

        let current_swapchain = AshSwapchainInstance::new(
            device.clone(),
            swapchain_extension.clone(),
            surface.get_handle(),
            &swapchain_config,
            current_extent,
            current_transform,
            image_count,
            vk::SwapchainKHR::null(),
        )?;

        Ok(Self {
            device,
            swapchain_extension,
            surface_extension,
            physical_device,
            surface,
            current_config: swapchain_config,
            current_swapchain,
            suboptimal: false,
        })
    }

    pub(crate) fn is_suboptimal(&self) -> bool {
        self.suboptimal
    }

    pub(crate) fn update_config(
        &mut self,
        new_config: SwapchainConfig,
    ) -> ash::prelude::VkResult<()> {
        self.current_config = new_config;
        self.rebuild()
    }

    pub(crate) fn rebuild(&mut self) -> ash::prelude::VkResult<()> {
        let (current_extent, current_transform, image_count) =
            get_swapchain_extent_transform_count(
                &self.surface_extension,
                self.physical_device,
                self.surface.get_handle(),
                self.current_config.image_count,
            )?;

        self.current_swapchain = AshSwapchainInstance::new(
            self.device.clone(),
            self.swapchain_extension.clone(),
            self.surface.get_handle(),
            &self.current_config,
            current_extent,
            current_transform,
            image_count,
            self.current_swapchain.handle,
        )?;

        self.suboptimal = false;

        Ok(())
    }

    pub(crate) fn acquire_next_image(
        &mut self,
        timeout: u64,
        image_ready_semaphore: vk::Semaphore,
        image_ready_fence: vk::Fence,
    ) -> ash::prelude::VkResult<u32> {
        let (index, suboptimal) = unsafe {
            self.swapchain_extension.acquire_next_image(
                self.current_swapchain.handle,
                timeout,
                image_ready_semaphore,
                image_ready_fence,
            )
        }?;

        if suboptimal {
            self.suboptimal = true
        }

        Ok(index)
    }

    pub(crate) fn present_image(
        &mut self,
        queue: vk::Queue,
        image_index: u32,
        wait_semaphore: vk::Semaphore,
    ) -> ash::prelude::VkResult<()> {
        let suboptimal = unsafe {
            let mut present_info = vk::PresentInfoKHR::builder()
                .swapchains(&[self.current_swapchain.handle])
                .image_indices(&[image_index])
                .wait_semaphores(&[wait_semaphore])
                .build();

            if wait_semaphore == vk::Semaphore::null() {
                present_info.wait_semaphore_count = 0;
            }

            self.swapchain_extension
                .queue_present(queue, &present_info)?
        };

        if suboptimal {
            self.suboptimal = true;
        }

        Ok(())
    }
}

fn get_swapchain_extent_transform_count(
    surface_extension: &Arc<ash::extensions::khr::Surface>,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    image_count: u32,
) -> ash::prelude::VkResult<(vk::Extent2D, vk::SurfaceTransformFlagsKHR, u32)> {
    unsafe {
        let capabilities =
            surface_extension.get_physical_device_surface_capabilities(physical_device, surface)?;

        Ok((
            vk::Extent2D {
                width: capabilities.current_extent.width.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: capabilities.current_extent.height.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            },
            capabilities.current_transform,
            image_count.clamp(capabilities.min_image_count, capabilities.max_image_count),
        ))
    }
}
