use crate::device::AshDevice;
use crate::image::AshImage;
use crate::instance::AshInstance;
use crate::SurfaceHandle;
use ash::vk;
use std::collections::HashMap;
use std::sync::Arc;

struct SwapchainInstance {
    device: Arc<AshDevice>,
    handle: vk::SwapchainKHR,

    images: Vec<AshImage>,

    #[allow(unused)]
    image_color_space: vk::ColorSpaceKHR,

    #[allow(unused)]
    pre_transform: vk::SurfaceTransformFlagsKHR,

    #[allow(unused)]
    composite_alpha: vk::CompositeAlphaFlagsKHR,

    #[allow(unused)]
    present_mode: vk::PresentModeKHR,
}

impl SwapchainInstance {
    fn new(
        device: Arc<AshDevice>,
        create_info: &vk::SwapchainCreateInfoKHR,
    ) -> ash::prelude::VkResult<Self> {
        let handle = unsafe { device.swapchain.create_swapchain(create_info, None) }?;

        let mut images = Vec::new();

        for &handle in unsafe { device.swapchain.get_swapchain_images(handle) }?.iter() {
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

            images.push(AshImage {
                handle,
                view,
                format: create_info.image_format,
                size: create_info.image_extent,
                usage: create_info.image_usage,
                location: gpu_allocator::MemoryLocation::GpuOnly,
                storage_binding: None,
                sampled_binding: None,
            });
        }

        Ok(Self {
            device,
            handle,
            images,
            image_color_space: create_info.image_color_space,
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
pub struct SurfaceSettings {
    /// Preferred number of swapchain images, actual number will vary
    pub image_count: u32,
    pub format: vk::SurfaceFormatKHR,
    pub size: [u32; 2],
    pub usage: vk::ImageUsageFlags,
    pub present_mode: vk::PresentModeKHR,
}

pub struct Swapchain {
    device: Arc<AshDevice>,
    surface: vk::SurfaceKHR,
    settings: SurfaceSettings,

    current_swapchain: Option<SwapchainInstance>,
}

impl Swapchain {
    pub fn new(
        device: Arc<AshDevice>,
        surface_handle: SurfaceHandle,
        settings: &SurfaceSettings,
    ) -> ash::prelude::VkResult<Self> {
        let surface = match device.instance.surface_list.get(surface_handle.0) {
            None => return Err(vk::Result::ERROR_SURFACE_LOST_KHR),
            Some(surface) => surface,
        };

        let mut new_self = Self {
            device,
            surface,
            settings: settings.clone(),
            current_swapchain: None,
        };
        new_self.rebuild()?;
        Ok(new_self)
    }

    pub fn update_settings(&mut self, settings: &SurfaceSettings) -> ash::prelude::VkResult<()> {
        self.settings = settings.clone();
        self.rebuild()
    }

    pub fn rebuild(&mut self) -> ash::prelude::VkResult<()> {
        let (extent, transform, image_count) = get_swapchain_extent_transform_count(
            &self.device.instance.surface,
            self.device.physical,
            self.surface,
            &self.settings,
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

    pub(crate) fn acquire_next_image(
        &self,
        image_ready_semaphore: vk::Semaphore,
    ) -> ash::prelude::VkResult<(AcquiredSwapchainImage, bool)> {
        let swapchain = self.current_swapchain.as_ref().unwrap();

        unsafe {
            self.device.swapchain.acquire_next_image(
                swapchain.handle,
                u64::MAX,
                image_ready_semaphore,
                vk::Fence::null(),
            )
        }
        .map(|(index, suboptimal)| {
            (
                AcquiredSwapchainImage {
                    swapchain_handle: swapchain.handle,
                    image_index: index,
                    image: swapchain.images[index as usize],
                },
                suboptimal,
            )
        })
    }
}

#[derive(Clone)]
pub struct AcquiredSwapchainImage {
    pub swapchain_handle: vk::SwapchainKHR,
    pub image_index: u32,
    pub image: AshImage,
}

fn get_swapchain_extent_transform_count(
    surface_extension: &ash::extensions::khr::Surface,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    settings: &SurfaceSettings,
) -> ash::prelude::VkResult<(vk::Extent2D, vk::SurfaceTransformFlagsKHR, u32)> {
    unsafe {
        let capabilities =
            surface_extension.get_physical_device_surface_capabilities(physical_device, surface)?;

        Ok((
            vk::Extent2D {
                width: settings.size[0].clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: settings.size[1].clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            },
            capabilities.current_transform,
            settings
                .image_count
                .clamp(capabilities.min_image_count, capabilities.max_image_count),
        ))
    }
}

pub struct SwapchainManager {
    instance: Arc<AshInstance>,
    pub swapchains: HashMap<vk::SurfaceKHR, Swapchain>,
}

impl SwapchainManager {
    pub fn new(instance: Arc<AshInstance>) -> Self {
        Self {
            instance,
            swapchains: HashMap::new(),
        }
    }

    pub fn add(&mut self, swapchain: Swapchain) {
        let surface = swapchain.surface;
        assert!(
            self.swapchains.insert(surface, swapchain).is_none(),
            "Swapchain for surface {:?} already exists, this shouldn't happen",
            surface
        );
    }

    pub fn get(&mut self, surface_handle: SurfaceHandle) -> Option<&mut Swapchain> {
        self.instance
            .surface_list
            .get(surface_handle.0)
            .and_then(|surface| self.swapchains.get_mut(&surface))
    }

    pub fn remove(&mut self, surface_handle: SurfaceHandle) {
        let _ = self
            .instance
            .surface_list
            .get(surface_handle.0)
            .map(|surface| self.swapchains.remove(&surface));
    }
}
