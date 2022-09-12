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

pub type SamplerHandle = u32;

pub struct Sampler {
    handle: SamplerHandle,
    freed_list: std::sync::Mutex<Vec<SamplerHandle>>,
}

impl Sampler {
    pub fn new_temp(handle: SamplerHandle) -> Self {
        Self {
            handle,
            freed_list: std::sync::Mutex::new(vec![]),
        }
    }

    pub fn get_handle(&self) -> SamplerHandle {
        self.handle
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        if let Ok(mut freed_list) = self.freed_list.lock() {
            freed_list.push(self.handle);
        }
    }
}
