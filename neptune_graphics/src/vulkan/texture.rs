use crate::texture::{TextureDescription, TextureDimensions, TextureFormat, TextureUsages};
use crate::vulkan::descriptor_set::Binding;
use ash::vk;
use gpu_allocator::vulkan;
use std::cell::RefCell;
use std::rc::Rc;

impl TextureFormat {
    pub fn to_vk(&self) -> vk::Format {
        match self {
            TextureFormat::Unknown => vk::Format::UNDEFINED,
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

            TextureFormat::D16Unorm => vk::Format::D16_UNORM,
            TextureFormat::D24UnormS8Uint => vk::Format::D24_UNORM_S8_UINT,
            TextureFormat::D32Float => vk::Format::D32_SFLOAT,
            TextureFormat::D32FloatS8Uint => vk::Format::D32_SFLOAT_S8_UINT,
        }
    }

    pub fn from_vk(format: vk::Format) -> Self {
        match format {
            vk::Format::UNDEFINED => TextureFormat::Unknown,

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
            vk::Format::D16_UNORM => TextureFormat::D16Unorm,

            vk::Format::D24_UNORM_S8_UINT => TextureFormat::D24UnormS8Uint,
            vk::Format::D32_SFLOAT => TextureFormat::D32Float,
            vk::Format::D32_SFLOAT_S8_UINT => TextureFormat::D32FloatS8Uint,

            _ => panic!("Unknown Texture Format"),
        }
    }
}

impl TextureUsages {
    fn to_vk(&self) -> vk::ImageUsageFlags {
        let mut result = vk::ImageUsageFlags::empty();
        if self.contains(TextureUsages::TRANSFER_SRC) {
            result |= vk::ImageUsageFlags::TRANSFER_SRC;
        }
        if self.contains(TextureUsages::TRANSFER_DST) {
            result |= vk::ImageUsageFlags::TRANSFER_DST;
        }
        if self.contains(TextureUsages::STORAGE) {
            result |= vk::ImageUsageFlags::STORAGE;
        }
        if self.contains(TextureUsages::SAMPLED) {
            result |= vk::ImageUsageFlags::SAMPLED;
        }
        if self.contains(TextureUsages::COLOR_ATTACHMENT) {
            result |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
        }
        if self.contains(TextureUsages::DEPTH_STENCIL_ATTACHMENT) {
            result |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        }
        if self.contains(TextureUsages::INPUT_ATTACHMENT) {
            result |= vk::ImageUsageFlags::INPUT_ATTACHMENT;
        }
        if self.contains(TextureUsages::TRANSIENT_ATTACHMENT) {
            result |= vk::ImageUsageFlags::TRANSIENT_ATTACHMENT;
        }
        result
    }
}

pub struct Texture {
    device: Rc<ash::Device>,
    allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,

    pub description: TextureDescription,
    pub allocation: gpu_allocator::vulkan::Allocation,
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub storage_binding: Option<Binding>,
    pub sampled_binding: Option<Binding>,
    pub format: vk::Format,
    pub subresource_range: vk::ImageSubresourceRange,
}

impl TextureDimensions {
    pub fn to_vk(self) -> (vk::Extent3D, vk::ImageType, vk::ImageViewType) {
        match self {
            TextureDimensions::D1(width) => (
                vk::Extent3D {
                    width,
                    height: 1,
                    depth: 1,
                },
                vk::ImageType::TYPE_1D,
                vk::ImageViewType::TYPE_1D,
            ),
            TextureDimensions::D2(width, height) => (
                vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                vk::ImageType::TYPE_2D,
                vk::ImageViewType::TYPE_2D,
            ),
            TextureDimensions::D3(width, height, depth) => (
                vk::Extent3D {
                    width,
                    height,
                    depth,
                },
                vk::ImageType::TYPE_3D,
                vk::ImageViewType::TYPE_3D,
            ),
        }
    }
}

impl Texture {
    pub(crate) fn new(
        device: Rc<ash::Device>,
        allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
        description: TextureDescription,
    ) -> Self {
        assert_ne!(
            description.format,
            TextureFormat::Unknown,
            "Texture format must not be TextureFormat::Unknown in create_texture"
        );

        let usage = description.usage.to_vk();
        let format = description.format.to_vk();
        let (extent, image_type, view_type) = description.size.to_vk();

        let handle = unsafe {
            device.create_image(
                &vk::ImageCreateInfo::builder()
                    .usage(usage)
                    .format(format)
                    .extent(extent)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .mip_levels(1)
                    .array_layers(1)
                    .image_type(image_type)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .build(),
                None,
            )
        }
        .expect("Failed to create image");

        let requirements = unsafe { device.get_image_memory_requirements(handle) };
        let allocation = allocator
            .borrow_mut()
            .allocate(&vulkan::AllocationCreateDesc {
                name: "Texture Allocation",
                requirements,
                location: description.memory_type.to_gpu_alloc(),
                linear: true,
            })
            .expect("Failed to allocate image memory");

        unsafe {
            device
                .bind_image_memory(handle, allocation.memory(), allocation.offset())
                .expect("Failed to bind image memory");
        }

        let aspect_mask = if description.format.is_color() {
            vk::ImageAspectFlags::COLOR
        } else {
            vk::ImageAspectFlags::DEPTH
        };

        let sub_resource_range = vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let view = unsafe {
            device.create_image_view(
                &vk::ImageViewCreateInfo::builder()
                    .format(format)
                    .image(handle)
                    .view_type(view_type)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(sub_resource_range),
                None,
            )
        }
        .expect("Failed to create image view");

        Self {
            device,
            allocator,
            description,
            allocation,
            handle,
            view,
            storage_binding: None,
            sampled_binding: None,
            format,
            subresource_range: sub_resource_range,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.handle, None);
        }

        let allocation = std::mem::take(&mut self.allocation);
        self.allocator
            .borrow_mut()
            .free(allocation)
            .expect("Failed to free texture memory");
    }
}
