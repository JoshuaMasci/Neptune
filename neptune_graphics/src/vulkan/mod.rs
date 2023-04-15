mod buffer;
mod debug_utils;
mod device;
mod image;
mod instance;
mod render_graph_builder;
mod sampler;
mod swapchain;

slotmap::new_key_type! {
    pub struct AshSurfaceHandle;
    pub struct AshBufferHandle;
    pub struct AshTextureHandle;
    pub struct AshSamplerHandle;
    pub struct AshComputePipelineHandle;
    pub struct AshRasterPipelineHandle;
    pub struct AshSwapchainHandle;
}

use crate::{
    AddressMode, BorderColor, ColorSpace, CompositeAlphaMode, FilterMode, PresentMode,
    TextureFormat,
};
use ash::vk;
pub use device::Device;
pub use instance::Instance;

impl crate::BufferUsage {
    pub(crate) fn to_vk(&self) -> vk::BufferUsageFlags {
        let mut flags = vk::BufferUsageFlags::empty();

        if self.contains(Self::VERTEX) {
            flags |= vk::BufferUsageFlags::VERTEX_BUFFER;
        }

        if self.contains(Self::INDEX) {
            flags |= vk::BufferUsageFlags::INDEX_BUFFER;
        }

        if self.contains(Self::UNIFORM) {
            flags |= vk::BufferUsageFlags::UNIFORM_BUFFER;
        }

        if self.contains(Self::STORAGE) {
            flags |= vk::BufferUsageFlags::STORAGE_BUFFER;
        }

        if self.contains(Self::INDIRECT) {
            flags |= vk::BufferUsageFlags::INDIRECT_BUFFER;
        }

        if self.contains(Self::TRANSFER_SRC) {
            flags |= vk::BufferUsageFlags::TRANSFER_SRC;
        }

        if self.contains(Self::TRANSFER_DST) {
            flags |= vk::BufferUsageFlags::TRANSFER_DST;
        }

        flags
    }
}

impl crate::TextureUsage {
    pub(crate) fn to_vk(&self, is_color: bool) -> vk::ImageUsageFlags {
        let mut flags = vk::ImageUsageFlags::empty();

        if self.contains(Self::RENDER_ATTACHMENT) {
            flags |= if is_color {
                vk::ImageUsageFlags::COLOR_ATTACHMENT
            } else {
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
            };
        }

        if self.contains(Self::INPUT_ATTACHMENT) {
            flags |= vk::ImageUsageFlags::INPUT_ATTACHMENT;
        }
        if self.contains(Self::SAMPLED) {
            flags |= vk::ImageUsageFlags::SAMPLED;
        }

        if self.contains(Self::STORAGE) {
            flags |= vk::ImageUsageFlags::STORAGE;
        }

        if self.contains(Self::TRANSFER_SRC) {
            flags |= vk::ImageUsageFlags::TRANSFER_SRC;
        }

        if self.contains(Self::TRANSFER_DST) {
            flags |= vk::ImageUsageFlags::TRANSFER_DST;
        }

        flags
    }

    pub(crate) fn from_vk(image_usage: vk::ImageUsageFlags) -> Self {
        let mut flags = Self::empty();

        if image_usage.contains(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            || image_usage.contains(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        {
            flags |= Self::RENDER_ATTACHMENT;
        }

        if image_usage.contains(vk::ImageUsageFlags::INPUT_ATTACHMENT) {
            flags |= Self::INPUT_ATTACHMENT;
        }

        if image_usage.contains(vk::ImageUsageFlags::SAMPLED) {
            flags |= Self::SAMPLED;
        }

        if image_usage.contains(vk::ImageUsageFlags::STORAGE) {
            flags |= Self::STORAGE;
        }

        if image_usage.contains(vk::ImageUsageFlags::TRANSFER_SRC) {
            flags |= Self::TRANSFER_SRC;
        }

        if image_usage.contains(vk::ImageUsageFlags::TRANSFER_DST) {
            flags |= Self::TRANSFER_DST;
        }

        flags
    }
}

impl crate::TextureFormat {
    pub(crate) fn to_vk(&self) -> vk::Format {
        match self {
            TextureFormat::R8Unorm => vk::Format::R8_UNORM,
            TextureFormat::Rg8Unorm => vk::Format::R8G8_UNORM,
            TextureFormat::Rgb8Unorm => vk::Format::R8G8B8_UNORM,
            TextureFormat::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,

            TextureFormat::R8Snorm => vk::Format::R8_SNORM,
            TextureFormat::Rg8Snorm => vk::Format::R8G8_SNORM,
            TextureFormat::Rgb8Snorm => vk::Format::R8G8B8_SNORM,
            TextureFormat::Rgba8Snorm => vk::Format::R8G8B8A8_SNORM,

            TextureFormat::R8Uint => vk::Format::R8_UINT,
            TextureFormat::Rg8Uint => vk::Format::R8G8_UINT,
            TextureFormat::Rgb8Uint => vk::Format::R8G8B8_UINT,
            TextureFormat::Rgba8Uint => vk::Format::R8G8B8A8_UINT,

            TextureFormat::R8Sint => vk::Format::R8_SINT,
            TextureFormat::Rg8Sint => vk::Format::R8G8_SINT,
            TextureFormat::Rgb8Sint => vk::Format::R8G8B8_SINT,
            TextureFormat::Rgba8Sint => vk::Format::R8G8B8A8_SINT,

            TextureFormat::R16Unorm => vk::Format::R16_UNORM,
            TextureFormat::Rg16Unorm => vk::Format::R16G16_UNORM,
            TextureFormat::Rgb16Unorm => vk::Format::R16G16B16_UNORM,
            TextureFormat::Rgba16Unorm => vk::Format::R16G16B16A16_UNORM,

            TextureFormat::R16Snorm => vk::Format::R16_SNORM,
            TextureFormat::Rg16Snorm => vk::Format::R16G16_SNORM,
            TextureFormat::Rgb16Snorm => vk::Format::R16G16B16_SNORM,
            TextureFormat::Rgba16Snorm => vk::Format::R16G16B16A16_SNORM,

            TextureFormat::R16Uint => vk::Format::R16_UINT,
            TextureFormat::Rg16Uint => vk::Format::R16G16_UINT,
            TextureFormat::Rgb16Uint => vk::Format::R16G16B16_UINT,
            TextureFormat::Rgba16Uint => vk::Format::R16G16B16A16_UINT,

            TextureFormat::R16Sint => vk::Format::R16_SINT,
            TextureFormat::Rg16Sint => vk::Format::R16G16_SINT,
            TextureFormat::Rgb16Sint => vk::Format::R16G16B16_SINT,
            TextureFormat::Rgba16Sint => vk::Format::R16G16B16A16_SINT,

            TextureFormat::Bgra8Unorm => vk::Format::B8G8R8A8_UNORM,
            TextureFormat::Bgra8Srgb => vk::Format::B8G8R8A8_SRGB,
            TextureFormat::A2Bgr10Unorm => vk::Format::A2B10G10R10_UNORM_PACK32,

            TextureFormat::D16Unorm => vk::Format::D16_UNORM,
            TextureFormat::D24UnormS8Uint => vk::Format::D24_UNORM_S8_UINT,
            TextureFormat::D32Float => vk::Format::D32_SFLOAT,
            TextureFormat::D32FloatS8Uint => vk::Format::D32_SFLOAT_S8_UINT,
        }
    }

    pub(crate) fn from_vk(vk_format: vk::Format) -> Option<Self> {
        Some(match vk_format {
            vk::Format::R8_UNORM => TextureFormat::R8Unorm,
            vk::Format::R8G8_UNORM => TextureFormat::Rg8Unorm,
            vk::Format::R8G8B8_UNORM => TextureFormat::Rgb8Unorm,
            vk::Format::R8G8B8A8_UNORM => TextureFormat::Rgba8Unorm,

            vk::Format::R8_SNORM => TextureFormat::R8Snorm,
            vk::Format::R8G8_SNORM => TextureFormat::Rg8Snorm,
            vk::Format::R8G8B8_SNORM => TextureFormat::Rgb8Snorm,
            vk::Format::R8G8B8A8_SNORM => TextureFormat::Rgba8Snorm,

            vk::Format::R8_UINT => TextureFormat::R8Uint,
            vk::Format::R8G8_UINT => TextureFormat::Rg8Uint,
            vk::Format::R8G8B8_UINT => TextureFormat::Rgb8Uint,
            vk::Format::R8G8B8A8_UINT => TextureFormat::Rgba8Uint,

            vk::Format::R8_SINT => TextureFormat::R8Sint,
            vk::Format::R8G8_SINT => TextureFormat::Rg8Sint,
            vk::Format::R8G8B8_SINT => TextureFormat::Rgb8Sint,
            vk::Format::R8G8B8A8_SINT => TextureFormat::Rgba8Sint,

            vk::Format::R16_UNORM => TextureFormat::R16Unorm,
            vk::Format::R16G16_UNORM => TextureFormat::Rg16Unorm,
            vk::Format::R16G16B16_UNORM => TextureFormat::Rgb16Unorm,
            vk::Format::R16G16B16A16_UNORM => TextureFormat::Rgba16Unorm,

            vk::Format::R16_SNORM => TextureFormat::R16Snorm,
            vk::Format::R16G16_SNORM => TextureFormat::Rg16Snorm,
            vk::Format::R16G16B16_SNORM => TextureFormat::Rgb16Snorm,
            vk::Format::R16G16B16A16_SNORM => TextureFormat::Rgba16Snorm,

            vk::Format::R16_UINT => TextureFormat::R16Uint,
            vk::Format::R16G16_UINT => TextureFormat::Rg16Uint,
            vk::Format::R16G16B16_UINT => TextureFormat::Rgb16Uint,
            vk::Format::R16G16B16A16_UINT => TextureFormat::Rgba16Uint,

            vk::Format::R16_SINT => TextureFormat::R16Sint,
            vk::Format::R16G16_SINT => TextureFormat::Rg16Sint,
            vk::Format::R16G16B16_SINT => TextureFormat::Rgb16Sint,
            vk::Format::R16G16B16A16_SINT => TextureFormat::Rgba16Sint,

            vk::Format::B8G8R8A8_UNORM => TextureFormat::Bgra8Unorm,
            vk::Format::B8G8R8A8_SRGB => TextureFormat::Bgra8Srgb,
            vk::Format::A2B10G10R10_UNORM_PACK32 => TextureFormat::A2Bgr10Unorm,

            vk::Format::D16_UNORM => TextureFormat::D16Unorm,
            vk::Format::D24_UNORM_S8_UINT => TextureFormat::D24UnormS8Uint,
            vk::Format::D32_SFLOAT => TextureFormat::D32Float,
            vk::Format::D32_SFLOAT_S8_UINT => TextureFormat::D32FloatS8Uint,
            _ => return None,
        })
    }
}

impl crate::ColorSpace {
    pub(crate) fn to_vk(&self) -> vk::ColorSpaceKHR {
        match self {
            ColorSpace::SrgbNonlinear => vk::ColorSpaceKHR::SRGB_NONLINEAR,
        }
    }

    pub(crate) fn from_vk(color_space: vk::ColorSpaceKHR) -> Option<Self> {
        Some(match color_space {
            vk::ColorSpaceKHR::SRGB_NONLINEAR => ColorSpace::SrgbNonlinear,
            _ => return None,
        })
    }
}

impl crate::PresentMode {
    pub(crate) fn to_vk(&self) -> vk::PresentModeKHR {
        match self {
            PresentMode::Fifo => vk::PresentModeKHR::FIFO,
            PresentMode::FifoRelaxed => vk::PresentModeKHR::FIFO_RELAXED,
            PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
            PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
        }
    }

    pub(crate) fn from_vk(present_mode: &vk::PresentModeKHR) -> Option<Self> {
        Some(match *present_mode {
            vk::PresentModeKHR::FIFO => PresentMode::Fifo,
            vk::PresentModeKHR::FIFO_RELAXED => PresentMode::FifoRelaxed,
            vk::PresentModeKHR::IMMEDIATE => PresentMode::Immediate,
            vk::PresentModeKHR::MAILBOX => PresentMode::Mailbox,
            _ => return None,
        })
    }
}

impl crate::CompositeAlphaMode {
    pub(crate) fn to_vk(&self) -> vk::CompositeAlphaFlagsKHR {
        match self {
            CompositeAlphaMode::Opaque => vk::CompositeAlphaFlagsKHR::OPAQUE,
            CompositeAlphaMode::PreMultiplied => vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
            CompositeAlphaMode::PostMultiplied => vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
            CompositeAlphaMode::Inherit => vk::CompositeAlphaFlagsKHR::INHERIT,
        }
    }

    pub(crate) fn from_vk(
        composite_alpha_mode: &vk::CompositeAlphaFlagsKHR,
    ) -> Vec<CompositeAlphaMode> {
        let mut modes = vec![];

        if composite_alpha_mode.contains(vk::CompositeAlphaFlagsKHR::OPAQUE) {
            modes.push(CompositeAlphaMode::Opaque);
        }

        if composite_alpha_mode.contains(vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED) {
            modes.push(CompositeAlphaMode::PreMultiplied);
        }

        if composite_alpha_mode.contains(vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED) {
            modes.push(CompositeAlphaMode::PostMultiplied);
        }

        if composite_alpha_mode.contains(vk::CompositeAlphaFlagsKHR::INHERIT) {
            modes.push(CompositeAlphaMode::Inherit);
        }

        modes
    }
}

impl crate::AddressMode {
    pub(crate) fn to_vk(&self) -> vk::SamplerAddressMode {
        match self {
            AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            AddressMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            AddressMode::ClampToBorder => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        }
    }
}

impl crate::FilterMode {
    pub(crate) fn to_vk_filter(&self) -> vk::Filter {
        match self {
            FilterMode::Nearest => vk::Filter::NEAREST,
            FilterMode::Linear => vk::Filter::LINEAR,
        }
    }

    pub(crate) fn to_vk_mip(&self) -> vk::SamplerMipmapMode {
        match self {
            FilterMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            FilterMode::Linear => vk::SamplerMipmapMode::LINEAR,
        }
    }
}

impl crate::BorderColor {
    pub(crate) fn to_vk(&self) -> vk::BorderColor {
        match self {
            BorderColor::TransparentBlack => vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
            BorderColor::OpaqueBlack => vk::BorderColor::FLOAT_OPAQUE_BLACK,
            BorderColor::OpaqueWhite => vk::BorderColor::FLOAT_OPAQUE_WHITE,
        }
    }
}

impl crate::SamplerDescription {
    pub(crate) fn to_vk(&self) -> vk::SamplerCreateInfo {
        vk::SamplerCreateInfo::builder()
            .address_mode_u(self.address_mode_u.to_vk())
            .address_mode_v(self.address_mode_v.to_vk())
            .address_mode_w(self.address_mode_w.to_vk())
            .mag_filter(self.mag_filter.to_vk_filter())
            .min_filter(self.min_filter.to_vk_filter())
            .mipmap_mode(self.mip_filter.to_vk_mip())
            .min_lod(
                self.lod_clamp_range
                    .clone()
                    .map(|range| range.start)
                    .unwrap_or(0.0),
            )
            .max_lod(
                self.lod_clamp_range
                    .clone()
                    .map(|range| range.end)
                    .unwrap_or(vk::LOD_CLAMP_NONE),
            )
            .anisotropy_enable(self.anisotropy_clamp.is_some())
            .max_anisotropy(self.anisotropy_clamp.unwrap_or(0.0))
            .border_color(self.border_color.to_vk())
            .unnormalized_coordinates(self.unnormalized_coordinates)
            .build()
    }
}
