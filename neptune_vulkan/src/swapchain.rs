use crate::{AshDevice, TextureUsage};
use ash::prelude::VkResult;
use ash::vk;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub enum PresentMode {
    #[default]
    Fifo,
    Immediate,
    Mailbox,
}

impl PresentMode {
    pub(crate) fn to_vk(&self) -> vk::PresentModeKHR {
        match self {
            PresentMode::Fifo => vk::PresentModeKHR::FIFO,
            PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
            PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
        }
    }
}

#[derive(Default)]
pub enum CompositeAlphaMode {
    #[default]
    Auto,
    Opaque,
    PreMultiplied,
    PostMultiplied,
    Inherit,
}

pub struct SwapchainConfig {
    pub format: vk::Format,
    pub present_mode: PresentMode,
    pub usage: TextureUsage,
    pub composite_alpha: CompositeAlphaMode,
}

pub(crate) struct AcquiredSwapchainTexture {
    pub(crate) index: u32,
    pub(crate) suboptimal: bool,
}

#[derive(Debug)]
struct SwapchainCapabilities {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainCapabilities {
    pub fn new(
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_ext: &Arc<ash::extensions::khr::Surface>,
    ) -> crate::Result<Self> {
        unsafe {
            let capabilities = match surface_ext
                .get_physical_device_surface_capabilities(physical_device, surface)
            {
                Ok(capabilities) => capabilities,
                Err(e) => return Err(crate::Error::VkError(e)),
            };
            let formats =
                match surface_ext.get_physical_device_surface_formats(physical_device, surface) {
                    Ok(formats) => formats,
                    Err(e) => return Err(crate::Error::VkError(e)),
                };
            let present_modes = match surface_ext
                .get_physical_device_surface_present_modes(physical_device, surface)
            {
                Ok(present_modes) => present_modes,
                Err(e) => return Err(crate::Error::VkError(e)),
            };

            Ok(Self {
                capabilities,
                formats,
                present_modes,
            })
        }
    }

    pub fn get_size(&self, desired_size: Option<vk::Extent2D>) -> vk::Extent2D {
        let desired_size = desired_size.unwrap_or(self.capabilities.current_extent);
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

    pub fn get_format(&self, desired_format: vk::Format) -> Option<vk::SurfaceFormatKHR> {
        self.formats
            .iter()
            .find(|surface_format| surface_format.format == desired_format)
            .copied()
    }

    pub fn get_present_mode(&self, desired_mode: vk::PresentModeKHR) -> Option<vk::PresentModeKHR> {
        self.present_modes
            .iter()
            .find(|&&present_mode| present_mode == desired_mode)
            .copied()
    }

    pub fn get_image_count(&self, desired_count: u32) -> u32 {
        u32::clamp(
            desired_count,
            self.capabilities.min_image_count,
            self.capabilities.max_image_count,
        )
    }
}

pub(crate) struct AshSwapchain {
    physical_device: vk::PhysicalDevice,
    device: Arc<AshDevice>,
    surface: vk::SurfaceKHR,
    surface_ext: Arc<ash::extensions::khr::Surface>,
    swapchain_ext: Arc<ash::extensions::khr::Swapchain>,

    current_config: SwapchainConfig,
    current_swapchain: AshSwapchainInstance,
}

impl AshSwapchain {
    const ACQUIRE_IMAGE_TIMEOUT: u64 = std::time::Duration::from_secs(2).as_nanos() as u64;

    pub(crate) fn new(
        physical_device: vk::PhysicalDevice,
        device: Arc<AshDevice>,
        surface: vk::SurfaceKHR,
        surface_ext: Arc<ash::extensions::khr::Surface>,
        swapchain_ext: Arc<ash::extensions::khr::Swapchain>,
        swapchain_config: SwapchainConfig,
    ) -> crate::Result<Self> {
        let current_swapchain = AshSwapchainInstance::new(
            physical_device,
            &device,
            surface,
            &surface_ext,
            &swapchain_ext,
            &swapchain_config,
            vk::SwapchainKHR::null(),
        )?;

        Ok(Self {
            physical_device,
            device,
            surface,
            surface_ext,
            swapchain_ext,
            current_config: swapchain_config,
            current_swapchain,
        })
    }

    pub(crate) fn get_handle(&self) -> vk::SwapchainKHR {
        self.current_swapchain.handle
    }

    pub(crate) fn acquire_next_image(
        &self,
        image_ready_semaphore: vk::Semaphore,
    ) -> crate::Result<AcquiredSwapchainTexture> {
        match unsafe {
            self.swapchain_ext.acquire_next_image(
                self.current_swapchain.handle,
                Self::ACQUIRE_IMAGE_TIMEOUT,
                image_ready_semaphore,
                vk::Fence::null(),
            )
        } {
            Ok((index, suboptimal)) => Ok(AcquiredSwapchainTexture { index, suboptimal }),
            Err(e) => Err(crate::Error::VkError(e)),
        }
    }

    pub(crate) fn update(&mut self, swapchain_config: SwapchainConfig) -> crate::Result<()> {
        self.current_config = swapchain_config;
        self.rebuild()
    }

    pub(crate) fn rebuild(&mut self) -> crate::Result<()> {
        self.current_swapchain = AshSwapchainInstance::new(
            self.physical_device,
            &self.device,
            self.surface,
            &self.surface_ext,
            &self.swapchain_ext,
            &self.current_config,
            self.current_swapchain.handle,
        )?;
        Ok(())
    }
}

pub(crate) struct AshSwapchainInstance {
    device: Arc<AshDevice>,
    swapchain_ext: Arc<ash::extensions::khr::Swapchain>,

    pub(crate) handle: vk::SwapchainKHR,
    pub(crate) textures: Vec<AshSwapchainTexture>,
}

impl AshSwapchainInstance {
    fn new(
        physical_device: vk::PhysicalDevice,
        device: &Arc<AshDevice>,
        surface: vk::SurfaceKHR,
        surface_ext: &Arc<ash::extensions::khr::Surface>,
        swapchain_ext: &Arc<ash::extensions::khr::Swapchain>,
        swapchain_config: &SwapchainConfig,
        old_swapchain: vk::SwapchainKHR,
    ) -> crate::Result<Self> {
        let capabilities = SwapchainCapabilities::new(physical_device, surface, surface_ext)?;

        let surface_size = capabilities.get_size(None);
        let surface_format = capabilities.get_format(swapchain_config.format).unwrap();
        let present_mode = capabilities
            .get_present_mode(swapchain_config.present_mode.to_vk())
            .unwrap();
        let image_count = capabilities.get_image_count(0);

        let mut image_usage = vk::ImageUsageFlags::TRANSFER_DST;

        if swapchain_config.usage.contains(TextureUsage::ATTACHMENT) {
            image_usage |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
        }

        let composite_alpha_mode = match swapchain_config.composite_alpha {
            CompositeAlphaMode::Auto => capabilities.capabilities.supported_composite_alpha, //TODO: is this correct for auto
            _ => todo!("Need to support more than Auto"),
        };

        let handle = match unsafe {
            swapchain_ext.create_swapchain(
                &vk::SwapchainCreateInfoKHR::builder()
                    .surface(surface)
                    .min_image_count(image_count)
                    .image_color_space(surface_format.color_space)
                    .image_format(surface_format.format)
                    .image_extent(surface_size)
                    .image_array_layers(1)
                    .image_usage(image_usage)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .pre_transform(capabilities.capabilities.current_transform)
                    .composite_alpha(composite_alpha_mode)
                    .present_mode(present_mode)
                    .clipped(true)
                    .old_swapchain(old_swapchain),
                None,
            )
        } {
            Ok(swapchain) => swapchain,
            Err(e) => return Err(crate::Error::VkError(e)),
        };

        let textures: Vec<AshSwapchainTexture> =
            unsafe { swapchain_ext.get_swapchain_images(handle) }
                .unwrap()
                .drain(..)
                .map(|image| {
                    AshSwapchainTexture::new(device.clone(), image, surface_format.format).unwrap()
                })
                .collect();

        Ok(Self {
            device: device.clone(),
            swapchain_ext: swapchain_ext.clone(),
            handle,
            textures,
        })
    }
}

impl Drop for AshSwapchainInstance {
    fn drop(&mut self) {
        self.textures.clear();
        unsafe {
            self.swapchain_ext.destroy_swapchain(self.handle, None);
        }
    }
}

pub struct AshSwapchainTexture {
    device: Arc<AshDevice>,
    pub(crate) handle: vk::Image,
    pub(crate) view: vk::ImageView,
}

impl AshSwapchainTexture {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        handle: vk::Image,
        format: vk::Format,
    ) -> crate::Result<Self> {
        let view = match unsafe {
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
            )
        } {
            Ok(view) => view,
            Err(e) => return Err(crate::Error::VkError(e)),
        };

        Ok(Self {
            device,
            handle,
            view,
        })
    }
}

impl Drop for AshSwapchainTexture {
    fn drop(&mut self) {
        unsafe { self.device.destroy_image_view(self.view, None) }
    }
}
