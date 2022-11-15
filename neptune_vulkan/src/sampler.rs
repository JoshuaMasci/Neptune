use crate::AshDevice;
use ash::vk;
use std::sync::Arc;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum FilterMode {
    Nearest,
    Linear,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AddressMode {
    Repeat,
    MirrorRepeat,
    ClampToEdge,
    ClampToBorder,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AnisotropicFilter {
    X1,
    X2,
    X4,
    X8,
    X16,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum BorderColor {
    TransparentBlack,
    OpaqueBlack,
    OpaqueWhite,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct SamplerCreateInfo {
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mip_filter: FilterMode,
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub min_lod: f32,
    pub max_lod: f32,
    pub max_anisotropy: Option<AnisotropicFilter>,
    pub boarder_color: BorderColor,
}

impl Default for SamplerCreateInfo {
    fn default() -> Self {
        Self {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            min_lod: 0.0,
            max_lod: 0.0,
            max_anisotropy: None,
            boarder_color: BorderColor::TransparentBlack,
        }
    }
}

struct Sampler {
    device: Arc<AshDevice>,
    pub handle: vk::Sampler,
}

impl Sampler {
    pub(crate) fn new(device: Arc<AshDevice>) -> crate::Result<Self> {
        let handle =
            match unsafe { device.create_sampler(&vk::SamplerCreateInfo::builder().build(), None) }
            {
                Ok(handle) => handle,
                Err(e) => return Err(crate::Error::VkError(e)),
            };

        Ok(Self { device, handle })
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe { self.device.destroy_sampler(self.handle, None) }
    }
}
