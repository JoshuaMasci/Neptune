use proc_macro::TokenStream;
use quote::quote;
use std::hash::{Hash, Hasher};
use syn::DeriveInput;
use syn::{parse_macro_input, Expr, Lit};

use darling::FromDeriveInput;
use darling::FromField;
use darling::FromMeta;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(binding), supports(struct_any))]
struct DescriptorSetLayoutReceiver {
    ident: syn::Ident,
    data: darling::ast::Data<(), DescriptorSetBindingReceiver>,
}

#[derive(Debug, FromField)]
#[darling(attributes(binding))]
#[darling(and_then = "Self::only_one")]
struct DescriptorSetBindingReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    sampler: darling::util::Flag,
    combined_image_sampler: darling::util::Flag,
    sampled_image: darling::util::Flag,
    storage_image: darling::util::Flag,
    uniform_buffer: darling::util::Flag,
    storage_buffer: darling::util::Flag,
    uniform_buffer_dynamic: darling::util::Flag,
    storage_buffer_dynamic: darling::util::Flag,
    acceleration_structure: darling::util::Flag,
}

impl DescriptorSetBindingReceiver {
    fn only_one(self) -> Result<Self, darling::Error> {
        let array = [
            self.sampler,
            self.combined_image_sampler,
            self.sampled_image,
            self.storage_image,
            self.uniform_buffer,
            self.storage_buffer,
            self.uniform_buffer_dynamic,
            self.storage_buffer_dynamic,
            self.acceleration_structure,
        ];

        match array.iter().filter(|flag| flag.is_present()).count() {
            0 => Err(darling::Error::custom(
                format!( "{} must be set binding to one of the following [sampler, combined_image_sampler, sampled_image, storage_image, uniform_buffer, storage_buffer, uniform_buffer_dynamic, storage_buffer_dynamic, acceleration_structure]", self.ident.unwrap()),
            )),
            1 => Ok(self),
            _ => Err(darling::Error::custom("Only one binding type allowed")),

        }
    }

    fn get_descriptor_type(&self) -> DescriptorType {
        if self.sampler.is_present() {
            DescriptorType::Sampler
        } else if self.combined_image_sampler.is_present() {
            DescriptorType::CombinedImageSampler
        } else if self.sampled_image.is_present() {
            DescriptorType::SampledImage
        } else if self.storage_image.is_present() {
            DescriptorType::StorageImage
        } else if self.uniform_buffer.is_present() {
            DescriptorType::UniformBuffer
        } else if self.storage_buffer.is_present() {
            DescriptorType::StorageBuffer
        } else if self.uniform_buffer_dynamic.is_present() {
            DescriptorType::UniformBufferDynamic
        } else if self.storage_buffer_dynamic.is_present() {
            DescriptorType::StorageBufferDynamic
        } else if self.acceleration_structure.is_present() {
            DescriptorType::AccelerationStructure
        } else {
            unreachable!()
        }
    }

    fn into_descriptor_binding(&self) -> DescriptorBinding {
        DescriptorBinding {
            descriptor_type: self.get_descriptor_type(),
            count: get_descriptor_count(&self.ty),
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

#[derive(Clone, Hash, Debug)]
struct DescriptorBinding {
    descriptor_type: DescriptorType,
    count: u32,
}

#[proc_macro_derive(DescriptorSet, attributes(binding))]
pub fn descriptor_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let receiver = DescriptorSetLayoutReceiver::from_derive_input(&input).unwrap();

    let struct_name = receiver.ident;

    let fields = receiver.data.take_struct().unwrap();

    let descriptor_bindings: Vec<DescriptorBinding> = fields
        .iter()
        .map(|binding| binding.into_descriptor_binding())
        .collect();

    let descriptor_name: Vec<String> = fields
        .iter()
        .map(|binding| {
            binding
                .ident
                .as_ref()
                .map(|ident| ident.to_string())
                .unwrap()
        })
        .collect();

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    descriptor_bindings.hash(&mut hasher);
    let hash_value = hasher.finish();

    let descriptor_set_layout_bindings =
        descriptor_bindings
            .iter()
            .enumerate()
            .map(|(index, binding)| {
                let descriptor_index = index as u32;
                let descriptor_count = binding.count;
                let descriptor_type = binding.descriptor_type.get_vk_name();
                quote! {
                neptune_vulkan::ash::vk::DescriptorSetLayoutBinding {
                    binding: #descriptor_index,
                    descriptor_type: #descriptor_type,
                    descriptor_count: #descriptor_count,
                    stage_flags: Default::default(),
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

            fn write_descriptor_set(&self, device: &std::sync::Arc<neptune_vulkan::AshDevice>, descriptor_set: neptune_vulkan::ash::vk::DescriptorSet) {}
        }
    })
}