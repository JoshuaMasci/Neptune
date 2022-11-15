use proc_macro::TokenStream;
use quote::quote;
use std::hash::{Hash, Hasher};
use syn::DeriveInput;
use syn::{parse_macro_input, Expr, Lit};

use darling::FromDeriveInput;
use darling::FromField;
use darling::FromMeta;

#[derive(Copy, Clone, Hash, Debug, FromMeta)]
#[darling(rename_all = "snake_case")]
enum DescriptorType {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformBuffer,
    StorageBuffer,
    UniformBufferDynamic,
    StorageBufferDynamic,
    AccelerationStructure,
}

impl DescriptorType {
    pub(crate) fn get_vk_name(&self) -> proc_macro2::TokenStream {
        match self {
            DescriptorType::Sampler => quote! {neptune_vulkan::ash::vk::DescriptorType::SAMPLER},
            DescriptorType::CombinedImageSampler => {
                quote! {neptune_vulkan::ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER}
            }
            DescriptorType::SampledImage => {
                quote! {neptune_vulkan::ash::vk::DescriptorType::SAMPLED_IMAGE}
            }
            DescriptorType::StorageImage => {
                quote! {neptune_vulkan::ash::vk::DescriptorType::STORAGE_IMAGE}
            }
            DescriptorType::UniformBuffer => {
                quote! {neptune_vulkan::ash::vk::DescriptorType::UNIFORM_BUFFER}
            }
            DescriptorType::StorageBuffer => {
                quote! {neptune_vulkan::ash::vk::DescriptorType::STORAGE_BUFFER}
            }
            DescriptorType::UniformBufferDynamic => {
                quote! {neptune_vulkan::ash::vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC}
            }
            DescriptorType::StorageBufferDynamic => {
                quote! {neptune_vulkan::ash::vk::DescriptorType::STORAGE_BUFFER_DYNAMIC}
            }
            DescriptorType::AccelerationStructure => {
                quote! {neptune_vulkan::ash::vk::DescriptorType::ACCELERATION_STRUCTURE_KHR}
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Hash, Debug, FromMeta)]
#[darling(rename_all = "snake_case")]
enum StageFlags {
    Compute,
    Vertex,
    Fragment,
    AllGraphics,
    Raygen,
    AnyHit,
    ClosestHit,
    Miss,
    Intersection,
    Callable,
    All,
}

#[allow(dead_code)]
impl StageFlags {
    pub(crate) fn get_vk_name(&self) -> proc_macro2::TokenStream {
        match self {
            StageFlags::Compute => quote! {neptune_vulkan::ash::vk::ShaderStageFlags::COMPUTE},
            StageFlags::Vertex => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::VERTEX}
            }
            StageFlags::Fragment => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::FRAGMENT}
            }
            StageFlags::AllGraphics => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::ALL_GRAPHICS}
            }
            StageFlags::Raygen => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::RAYGEN_KHR}
            }
            StageFlags::AnyHit => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::ANY_HIT_KHR}
            }
            StageFlags::ClosestHit => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::CLOSEST_HIT_KHR}
            }
            StageFlags::Miss => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::MISS_KHR}
            }
            StageFlags::Intersection => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::INTERSECTION_KHR}
            }
            StageFlags::Callable => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::CALLABLE_KHR}
            }
            StageFlags::All => {
                quote! {neptune_vulkan::ash::vk::ShaderStageFlags::ALL}
            }
        }
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(binding), supports(struct_any))]
struct DescriptorSetLayoutReceiver {
    ident: syn::Ident,
    data: darling::ast::Data<(), DescriptorSetBindingReceiver>,
}

#[derive(Debug, FromField)]
#[darling(attributes(binding))]
struct DescriptorSetBindingReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    descriptor_type: DescriptorType,
    // #[darling(multiple)]
    // stage_flags: Vec<StageFlags>,
}

impl DescriptorSetBindingReceiver {
    fn as_descriptor_binding(&self) -> DescriptorBinding {
        DescriptorBinding {
            ident: self.ident.clone().unwrap(),
            descriptor_type: self.descriptor_type,
            count: get_descriptor_count(&self.ty),
            //stage_flags: self.stage_flags.clone(),
        }
    }
}

fn get_descriptor_count(ty: &syn::Type) -> u32 {
    if let syn::Type::Array(array_type) = ty {
        let count = match &array_type.len {
            Expr::Lit(lit) => match &lit.lit {
                Lit::Int(int) => int
                    .base10_parse::<u32>()
                    .expect("Failed to parse  integer literal"),
                _ => panic!("Array Len must be integer literal"),
            },
            _ => panic!("Array Len must be integer literal"),
        };
        count
    } else {
        1
    }
}

#[derive(Clone, Hash, Debug)]
struct DescriptorBinding {
    ident: syn::Ident,
    descriptor_type: DescriptorType,
    count: u32,
    //stage_flags: Vec<StageFlags>,
}

#[proc_macro_derive(DescriptorSet, attributes(binding))]
pub fn descriptor_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let receiver = DescriptorSetLayoutReceiver::from_derive_input(&input).unwrap();

    let struct_name = receiver.ident;

    let fields = receiver.data.take_struct().unwrap();

    let descriptor_bindings: Vec<DescriptorBinding> = fields
        .iter()
        .map(|binding| binding.as_descriptor_binding())
        .collect();

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    descriptor_bindings.hash(&mut hasher);
    let hash_value = hasher.finish();

    let descriptor_set_layout_bindings =
        descriptor_bindings
            .iter()
            .enumerate()
            .map(|(index, binding)| {
                //TODO: ACTUALLY USE STAGE FLAGS!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                let descriptor_index = index as u32;
                let descriptor_count = binding.count;
                let descriptor_type = binding.descriptor_type.get_vk_name();
                quote! {
                neptune_vulkan::ash::vk::DescriptorSetLayoutBinding {
                    binding: #descriptor_index,
                    descriptor_type: #descriptor_type,
                    descriptor_count: #descriptor_count,
                    stage_flags: neptune_vulkan::ash::vk::ShaderStageFlags::ALL,
                    p_immutable_samplers: std::ptr::null(),
                },
                }
            });

    let descriptor_set_pool_sizes = descriptor_bindings.iter().map(|binding| {
        let descriptor_count = binding.count;
        let descriptor_type = binding.descriptor_type.get_vk_name();
        quote! {
            neptune_vulkan::ash::vk::DescriptorPoolSize { ty: #descriptor_type, descriptor_count: #descriptor_count },
        }
    });

    let descriptor_writes = descriptor_bindings
        .iter()
        .enumerate()
        .map(|(i, binding)| descriptor_write_struct(i as u32, binding));

    TokenStream::from(quote! {
        impl neptune_vulkan::descriptor_set::DescriptorSetLayout for #struct_name {
            const LAYOUT_HASH: u64 = #hash_value;

            fn create_layout(device: &std::sync::Arc<neptune_vulkan::AshDevice>) -> neptune_vulkan::ash::prelude::VkResult<neptune_vulkan::ash::vk::DescriptorSetLayout>
            {
                unsafe { device.create_descriptor_set_layout(&neptune_vulkan::ash::vk::DescriptorSetLayoutCreateInfo::builder().bindings(&[#(#descriptor_set_layout_bindings)*]).build(), None) }
            }

            fn create_pool_sizes() -> Vec<neptune_vulkan::ash::vk::DescriptorPoolSize>
            {
                vec![#(#descriptor_set_pool_sizes)*]
            }

            fn write_descriptor_set(&self, device: &std::sync::Arc<neptune_vulkan::AshDevice>, descriptor_set: neptune_vulkan::ash::vk::DescriptorSet) {
                unsafe  {
                    device.update_descriptor_sets(&[#(#descriptor_writes)*], &[]);
                }
            }
        }
    })
}

fn descriptor_write_struct(
    binding_index: u32,
    binding_info: &DescriptorBinding,
) -> proc_macro2::TokenStream {
    let descriptor_write_data = match binding_info.descriptor_type {
        DescriptorType::UniformBuffer
        | DescriptorType::StorageBuffer
        | DescriptorType::UniformBufferDynamic
        | DescriptorType::StorageBufferDynamic => get_buffer_write_data(binding_info),
        DescriptorType::Sampler => get_sampler_write_date(binding_info),
        DescriptorType::CombinedImageSampler => get_combined_image_sampler_write_date(binding_info),
        DescriptorType::SampledImage => get_image_write_date(
            binding_info,
            quote!(neptune_vulkan::ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
        ),
        DescriptorType::StorageImage => get_image_write_date(
            binding_info,
            quote!(neptune_vulkan::ash::vk::ImageLayout::GENERAL),
        ),
        DescriptorType::AccelerationStructure => todo!("AccelerationStructure descriptor writes"),
    };

    let p_next = descriptor_write_data.p_next;

    let descriptor_count = binding_info.count;
    let descriptor_type = binding_info.descriptor_type.get_vk_name();

    let p_image_info = descriptor_write_data.p_image_info;
    let p_buffer_info = descriptor_write_data.p_buffer_info;

    quote! {neptune_vulkan::ash::vk::WriteDescriptorSet {
        s_type: neptune_vulkan::ash::vk::StructureType::WRITE_DESCRIPTOR_SET,
        p_next: #p_next,
        dst_set: descriptor_set,
        dst_binding: #binding_index,
        dst_array_element: 0,
        descriptor_count: #descriptor_count,
        descriptor_type: #descriptor_type,
        p_image_info: #p_image_info,
        p_buffer_info: #p_buffer_info,
        p_texel_buffer_view: std::ptr::null(),
    },}
}

struct DescriptorSetWriteData {
    p_next: proc_macro2::TokenStream,
    p_image_info: proc_macro2::TokenStream,
    p_buffer_info: proc_macro2::TokenStream,
}

fn get_sampler_write_date(binding_info: &DescriptorBinding) -> DescriptorSetWriteData {
    let ident = &binding_info.ident;

    let image_info_list = if binding_info.count == 1 {
        quote!(neptune_vulkan::ash::vk::DescriptorImageInfo {
                    sampler: self.#ident.handle,
                    image_view: neptune_vulkan::ash::vk::ImageView::null(),
                    image_layout: neptune_vulkan::ash::vk::ImageLayout::UNDEFINED,
        },)
    } else {
        let list = (0..binding_info.count).map(|i| {
            quote!(neptune_vulkan::ash::vk::DescriptorImageInfo {
                    sampler: self.#ident[#i].handle,
                    image_view: neptune_vulkan::ash::vk::ImageView::null(),
                    image_layout: neptune_vulkan::ash::vk::ImageLayout::UNDEFINED,
        },)
        });
        quote!(#(#list)*)
    };

    DescriptorSetWriteData {
        p_next: quote!(std::ptr::null()),
        p_image_info: quote!([#image_info_list].as_ptr()),
        p_buffer_info: quote!(std::ptr::null()),
    }
}

fn get_combined_image_sampler_write_date(
    binding_info: &DescriptorBinding,
) -> DescriptorSetWriteData {
    let ident = &binding_info.ident;

    let image_info_list = if binding_info.count == 1 {
        quote!(neptune_vulkan::ash::vk::DescriptorImageInfo {
                    sampler: self.#ident.1.handle,
                    image_view: self.#ident.0.handle,
                    image_layout: neptune_vulkan::ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        },)
    } else {
        let list = (0..binding_info.count).map(|i| {
            quote!(neptune_vulkan::ash::vk::DescriptorImageInfo {
                    sampler: self.#ident[#i].1.handle,
                    image_view: self.#ident[#i].0.handle,
                    image_layout: neptune_vulkan::ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        },)
        });
        quote!(#(#list)*)
    };

    DescriptorSetWriteData {
        p_next: quote!(std::ptr::null()),
        p_image_info: quote!([#image_info_list].as_ptr()),
        p_buffer_info: quote!(std::ptr::null()),
    }
}

fn get_image_write_date(
    binding_info: &DescriptorBinding,
    image_layout: proc_macro2::TokenStream,
) -> DescriptorSetWriteData {
    let ident = &binding_info.ident;

    let image_info_list = if binding_info.count == 1 {
        quote!(neptune_vulkan::ash::vk::DescriptorImageInfo {
                    sampler: neptune_vulkan::ash::vk::Sampler::null(),
                    image_view: self.#ident.handle,
                    image_layout: #image_layout,
        },)
    } else {
        let list = (0..binding_info.count).map(|i| {
            quote!(neptune_vulkan::ash::vk::DescriptorImageInfo {
                    sampler: neptune_vulkan::ash::vk::Sampler::null(),
                    image_view: self.#ident[#i].handle,
                    image_layout: #image_layout,
        },)
        });
        quote!(#(#list)*)
    };

    DescriptorSetWriteData {
        p_next: quote!(std::ptr::null()),
        p_image_info: quote!([#image_info_list].as_ptr()),
        p_buffer_info: quote!(std::ptr::null()),
    }
}

fn get_buffer_write_data(binding_info: &DescriptorBinding) -> DescriptorSetWriteData {
    let ident = &binding_info.ident;

    let buffer_info_list = if binding_info.count == 1 {
        quote!(neptune_vulkan::ash::vk::DescriptorBufferInfo {
                    buffer: self.#ident.handle,
                    offset: 0,
                    range: neptune_vulkan::ash::vk::WHOLE_SIZE,
        },)
    } else {
        let list = (0..binding_info.count).map(|i| {
            quote!(neptune_vulkan::ash::vk::DescriptorBufferInfo {
                    buffer: self.#ident[#i].handle,
                    offset: 0,
                    range: neptune_vulkan::ash::vk::WHOLE_SIZE,
        },)
        });
        quote!(#(#list)*)
    };

    DescriptorSetWriteData {
        p_next: quote!(std::ptr::null()),
        p_image_info: quote!(std::ptr::null()),
        p_buffer_info: quote!([#buffer_info_list].as_ptr()),
    }
}
