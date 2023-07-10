use crate::AshDevice;
use ash::vk;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct SwapchainImage {
    pub index: u32,
    pub handle: vk::Image,
    pub view: vk::ImageView,

    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub usage: vk::ImageUsageFlags,
}

struct SwapchainInstance {
    device: Arc<AshDevice>,
    handle: vk::SwapchainKHR,

    images: Vec<SwapchainImage>,

    pub image_format: vk::Format,
    pub image_color_space: vk::ColorSpaceKHR,
    pub image_extent: vk::Extent2D,
    pub image_usage: vk::ImageUsageFlags,
    pub pre_transform: vk::SurfaceTransformFlagsKHR,
    pub composite_alpha: vk::CompositeAlphaFlagsKHR,
    pub present_mode: vk::PresentModeKHR,
}

impl SwapchainInstance {
    fn new(
        device: Arc<AshDevice>,
        create_info: &vk::SwapchainCreateInfoKHR,
    ) -> ash::prelude::VkResult<Self> {
        let handle = unsafe { device.swapchain.create_swapchain(create_info, None) }?;

        let mut images = Vec::new();

        for (index, &handle) in unsafe { device.swapchain.get_swapchain_images(handle) }?
            .iter()
            .enumerate()
        {
            let view = unsafe {
                device.core.create_image_view(
                    &vk::ImageViewCreateInfo::builder()
                        .image(handle)
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(create_info.image_format)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .components(vk::ComponentMapping {
                            r: vk::ComponentSwizzle::R,
                            g: vk::ComponentSwizzle::G,
                            b: vk::ComponentSwizzle::B,
                            a: vk::ComponentSwizzle::A,
                        }),
                    None,
                )?
            };

            images.push(SwapchainImage {
                index: index as u32,
                handle,
                view,
                format: create_info.image_format,
                extent: create_info.image_extent,
                usage: create_info.image_usage,
            });
        }

        Ok(Self {
            device,
            handle,
            images,
            image_format: create_info.image_format,
            image_color_space: create_info.image_color_space,
            image_extent: create_info.image_extent,
            image_usage: create_info.image_usage,
            pre_transform: create_info.pre_transform,
            composite_alpha: create_info.composite_alpha,
            present_mode: create_info.present_mode,
        })
    }
}

impl Drop for SwapchainInstance {
    fn drop(&mut self) {
        unsafe {
            self.images
                .iter()
                .for_each(|image| self.device.core.destroy_image_view(image.view, None));
            self.device.swapchain.destroy_swapchain(self.handle, None);
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AshSwapchainSettings {
    /// Preferred number of swapchain images, actual number will vary
    pub image_count: u32,

    pub format: vk::SurfaceFormatKHR,
    pub usage: vk::ImageUsageFlags,
    pub present_mode: vk::PresentModeKHR,
}

pub struct AshSwapchain {
    device: Arc<AshDevice>,
    surface: vk::SurfaceKHR,
    settings: AshSwapchainSettings,

    current_swapchain: Option<SwapchainInstance>,
}

impl AshSwapchain {
    pub fn new(
        device: Arc<AshDevice>,
        surface: vk::SurfaceKHR,
        settings: AshSwapchainSettings,
    ) -> ash::prelude::VkResult<Self> {
        let mut new_self = Self {
            device,
            surface,
            settings,
            current_swapchain: None,
        };
        new_self.rebuild()?;
        Ok(new_self)
    }

    pub fn update_settings(
        &mut self,
        settings: AshSwapchainSettings,
    ) -> ash::prelude::VkResult<()> {
        self.settings = settings;
        self.rebuild()
    }

    pub fn rebuild(&mut self) -> ash::prelude::VkResult<()> {
        let (extent, transform, image_count) = get_swapchain_extent_transform_count(
            &self.device.instance.surface,
            self.device.physical,
            self.surface,
            self.settings.image_count,
        )?;

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface)
            .min_image_count(image_count)
            .image_format(self.settings.format.format)
            .image_color_space(self.settings.format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(self.settings.usage)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.settings.present_mode)
            .clipped(true)
            .old_swapchain(
                self.current_swapchain
                    .as_ref()
                    .map(|swapchain| swapchain.handle)
                    .unwrap_or(vk::SwapchainKHR::null()),
            );

        self.current_swapchain = Some(SwapchainInstance::new(
            self.device.clone(),
            &swapchain_create_info,
        )?);

        Ok(())
    }

    pub(crate) fn get_handle(&self) -> vk::SwapchainKHR {
        self.current_swapchain.as_ref().unwrap().handle
    }

    pub(crate) fn get_image(&self, index: u32) -> SwapchainImage {
        self.current_swapchain.as_ref().unwrap().images[index as usize].clone()
    }

    pub(crate) fn acquire_next_image(
        &self,
        image_ready_semaphore: vk::Semaphore,
    ) -> ash::prelude::VkResult<(u32, bool)> {
        unsafe {
            self.device.swapchain.acquire_next_image(
                self.get_handle(),
                u64::MAX,
                image_ready_semaphore,
                vk::Fence::null(),
            )
        }
    }
}

fn get_swapchain_extent_transform_count(
    surface_extension: &ash::extensions::khr::Surface,
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

#[derive(Default)]
pub struct SwapchainManager {
    pub swapchains: HashMap<vk::SurfaceKHR, AshSwapchain>,
}

impl SwapchainManager {
    pub fn add_swapchain(&mut self, swapchain: AshSwapchain) {
        let surface = swapchain.surface;
        assert!(
            self.swapchains.insert(surface, swapchain).is_none(),
            "Swapchain for surface {:?} already exists, this shouldn't happen",
            surface
        );
    }
}
