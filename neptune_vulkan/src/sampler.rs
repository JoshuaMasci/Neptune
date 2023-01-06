use crate::AshDevice;
use ash::vk;
use std::sync::Arc;

#[derive(Default, Debug, Copy, Clone)]
pub enum AddressMode {
    #[default]
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
}

impl AddressMode {
    fn to_vk(&self) -> vk::SamplerAddressMode {
        match self {
            AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            AddressMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            AddressMode::ClampToBorder => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub enum FilterMode {
    #[default]
    Nearest,
    Linear,
}

impl FilterMode {
    fn to_vk(&self) -> vk::Filter {
        match self {
            FilterMode::Nearest => vk::Filter::NEAREST,
            FilterMode::Linear => vk::Filter::LINEAR,
        }
    }

    fn to_mip_vk(&self) -> vk::SamplerMipmapMode {
        match self {
            FilterMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            FilterMode::Linear => vk::SamplerMipmapMode::LINEAR,
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub enum BorderColor {
    #[default]
    TransparentBlack,
    OpaqueBlack,
    OpaqueWhite,
}

impl BorderColor {
    fn to_vk(&self) -> vk::BorderColor {
        match self {
            BorderColor::TransparentBlack => vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
            BorderColor::OpaqueBlack => vk::BorderColor::FLOAT_OPAQUE_BLACK,
            BorderColor::OpaqueWhite => vk::BorderColor::FLOAT_OPAQUE_WHITE,
        }
    }
}

#[derive(Default, Debug)]
pub struct SamplerCreateInfo {
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mip_filter: FilterMode,
    pub lod_clamp_range: Option<std::ops::Range<f32>>,
    pub anisotropy_clamp: Option<f32>,
    pub border_color: BorderColor,
    pub unnormalized_coordinates: bool,
}

impl SamplerCreateInfo {
    fn to_vk(&self) -> vk::SamplerCreateInfo {
        let lod_clamp_range = self
            .lod_clamp_range
            .clone()
            .unwrap_or(0.0..vk::LOD_CLAMP_NONE);
        vk::SamplerCreateInfo::builder()
            .address_mode_u(self.address_mode_u.to_vk())
            .address_mode_v(self.address_mode_v.to_vk())
            .address_mode_w(self.address_mode_w.to_vk())
            .mag_filter(self.mag_filter.to_vk())
            .min_filter(self.min_filter.to_vk())
            .mipmap_mode(self.mip_filter.to_mip_vk())
            .min_lod(lod_clamp_range.start)
            .max_lod(lod_clamp_range.end)
            .anisotropy_enable(self.anisotropy_clamp.is_some())
            .max_anisotropy(self.anisotropy_clamp.unwrap_or_default())
            .border_color(self.border_color.to_vk())
            .unnormalized_coordinates(self.unnormalized_coordinates)
            .build()
    }
}

pub(crate) struct AshSampler {
    device: Arc<AshDevice>,
    pub(crate) handle: vk::Sampler,
}

impl AshSampler {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        sampler_create_info: &SamplerCreateInfo,
    ) -> crate::Result<Self> {
        unsafe {
            match device.create_sampler(&sampler_create_info.to_vk(), None) {
                Ok(handle) => Ok(Self { device, handle }),
                Err(e) => Err(crate::Error::VkError(e)),
            }
        }
    }
}

impl Drop for AshSampler {
    fn drop(&mut self) {
        unsafe { self.device.destroy_sampler(self.handle, None) };
        trace!("Destroy Sampler");
    }
}
