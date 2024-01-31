use crate::material::{Material, MaterialTexture};
use crate::mesh::{
    BoundingBox, IndexBuffer, Mesh, Primitive, VertexAttributes, VertexSkinningAttributes,
};
use anyhow::anyhow;
use glam::{Vec2, Vec3, Vec4};
use gltf::image::Format;
use gltf::material::NormalTexture;
use gltf::texture::{MagFilter, MinFilter, WrappingMode};
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::{vk, AddressMode, FilterMode, ImageHandle, SamplerHandle};

fn neptune_address_mode(mode: WrappingMode) -> AddressMode {
    match mode {
        WrappingMode::ClampToEdge => AddressMode::ClampToEdge,
        WrappingMode::MirroredRepeat => AddressMode::MirroredRepeat,
        WrappingMode::Repeat => AddressMode::Repeat,
    }
}

pub struct GltfSamplers {
    pub default: SamplerHandle,
    pub samplers: Vec<SamplerHandle>,
}

pub fn load_samplers(
    device: &mut neptune_vulkan::Device,
    gltf_doc: &gltf::Document,
) -> anyhow::Result<GltfSamplers> {
    let default_sampler = device.create_sampler(
        "Gltf Default Sampler",
        &neptune_vulkan::SamplerDescription {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            lod_clamp_range: None,
            anisotropy_clamp: None,
            border_color: Default::default(),
            unnormalized_coordinates: false,
        },
    )?;

    let mut samplers = Vec::with_capacity(gltf_doc.samplers().len());
    for gltf_sampler in gltf_doc.samplers() {
        let name = gltf_sampler.name().unwrap_or("Unnamed Sampler");

        let address_mode_u = neptune_address_mode(gltf_sampler.wrap_s());
        let address_mode_v = neptune_address_mode(gltf_sampler.wrap_t());

        let mag_filter = match gltf_sampler.mag_filter().unwrap_or(MagFilter::Linear) {
            MagFilter::Nearest => FilterMode::Nearest,
            MagFilter::Linear => FilterMode::Linear,
        };

        let (min_filter, mip_filter) = match gltf_sampler.min_filter().unwrap_or(MinFilter::Linear)
        {
            MinFilter::Nearest | MinFilter::NearestMipmapNearest => {
                (FilterMode::Nearest, FilterMode::Nearest)
            }
            MinFilter::Linear | MinFilter::LinearMipmapLinear => {
                (FilterMode::Linear, FilterMode::Linear)
            }
            MinFilter::NearestMipmapLinear => (FilterMode::Nearest, FilterMode::Linear),
            MinFilter::LinearMipmapNearest => (FilterMode::Linear, FilterMode::Nearest),
        };

        let description = neptune_vulkan::SamplerDescription {
            address_mode_u,
            address_mode_v,
            address_mode_w: AddressMode::Repeat,
            mag_filter,
            min_filter,
            mip_filter,
            lod_clamp_range: None,
            anisotropy_clamp: None,
            border_color: Default::default(),
            unnormalized_coordinates: false,
        };

        samplers.push(device.create_sampler(name, &description)?);
    }

    Ok(GltfSamplers {
        default: default_sampler,
        samplers,
    })
}

pub fn load_images(
    device: &mut neptune_vulkan::Device,
    gltf_doc: &gltf::Document,
    gltf_images: &[gltf::image::Data],
) -> anyhow::Result<Vec<ImageHandle>> {
    let mut images = Vec::with_capacity(gltf_doc.images().len());
    for gltf_image in gltf_doc.images() {
        let name = gltf_image.name().unwrap_or("Unnamed Image");

        let gltf_image_data = &gltf_images[gltf_image.index()];

        let mut format = match gltf_image_data.format {
            Format::R8 => vk::Format::R8_UNORM,
            Format::R8G8 => vk::Format::R8G8_UNORM,
            Format::R8G8B8 => vk::Format::R8G8B8_UNORM,
            Format::R8G8B8A8 => vk::Format::R8G8B8A8_UNORM,
            Format::R16 => vk::Format::R16_UNORM,
            Format::R16G16 => vk::Format::R16G16_UNORM,
            Format::R16G16B16 => vk::Format::R16G16B16_UNORM,
            Format::R16G16B16A16 => vk::Format::R16G16B16A16_UNORM,
            Format::R32G32B32FLOAT => vk::Format::R32G32B32_SFLOAT,
            Format::R32G32B32A32FLOAT => vk::Format::R32G32B32A32_SFLOAT,
        };

        let mut image_data_slice: &[u8] = &gltf_image_data.pixels;

        //Because Nvidia doesn't like non-32 aligned RGB image formats
        let image_data_slice_new: Vec<u8>;
        if format == vk::Format::R8G8B8_UNORM {
            format = vk::Format::R8G8B8A8_UNORM;
            image_data_slice_new = image_data_slice
                .chunks_exact(3)
                .flat_map(|chunk| [chunk[0], chunk[1], chunk[2], 255])
                .collect();
            image_data_slice = &image_data_slice_new;
        } else if format == vk::Format::R16G16B16_UNORM {
            format = vk::Format::R16G16B16A16_UNORM;
            image_data_slice_new = image_data_slice
                .chunks_exact(6)
                .flat_map(|chunk| {
                    [
                        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], 255, 255,
                    ]
                })
                .collect();
            image_data_slice = &image_data_slice_new;
        }
        let description = neptune_vulkan::ImageDescription2D {
            size: [gltf_image_data.width, gltf_image_data.height],
            format,
            usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            mip_levels: 1,
            location: MemoryLocation::GpuOnly,
        };

        images.push(device.create_image_init(name, &description, image_data_slice)?);
    }

    Ok(images)
}

pub fn load_meshes(
    device: &mut neptune_vulkan::Device,
    gltf_doc: &gltf::Document,
    gltf_buffers: &[gltf::buffer::Data],
) -> anyhow::Result<Vec<Mesh>> {
    let mut meshes = Vec::with_capacity(gltf_doc.meshes().len());

    for gltf_mesh in gltf_doc.meshes() {
        let mut mesh = Mesh {
            name: gltf_mesh
                .name()
                .map(|str| str.to_string())
                .unwrap_or(String::from("Unnamed Mesh")),
            primitives: Vec::new(),
        };

        for gltf_primitive in gltf_mesh.primitives() {
            mesh.primitives
                .push(load_primitive(device, gltf_buffers, &gltf_primitive)?);
        }
        meshes.push(mesh);
    }

    Ok(meshes)
}

pub fn load_primitive(
    device: &mut neptune_vulkan::Device,
    gltf_buffers: &[gltf::buffer::Data],
    gltf_primitive: &gltf::Primitive,
) -> anyhow::Result<Primitive> {
    let reader = gltf_primitive.reader(|buffer| Some(&gltf_buffers[buffer.index()]));

    let bounding_box = BoundingBox {
        min: Vec3::from_array(gltf_primitive.bounding_box().min),
        max: Vec3::from_array(gltf_primitive.bounding_box().max),
    };

    let (position_buffer, vertex_count) = {
        let positions: Vec<Vec3> = match reader.read_positions() {
            None => return Err(anyhow!("Mesh contains no vertex positions")),
            Some(positions) => positions,
        }
        .map(Vec3::from_array)
        .collect();
        (create_vertex_buffer(device, &positions)?, positions.len())
    };

    let attributes_buffer = {
        let mut attributes: Vec<VertexAttributes> = if let Some(normals) = reader.read_normals() {
            if let Some(tangents) = reader.read_tangents() {
                if let Some(tex_coords) = reader.read_tex_coords(0) {
                    normals
                        .zip(tangents)
                        .zip(tex_coords.into_f32())
                        .map(|((normal, tangent), tex_coord)| VertexAttributes {
                            normal: Vec3::from_array(normal),
                            tangent: Vec4::from_array(tangent),
                            tex_coords: Vec4::new(tex_coord[0], tex_coord[1], 0.0, 0.0),
                            color: Vec4::splat(1.0),
                        })
                        .collect()
                } else {
                    return Err(anyhow!("Mesh primitive doesn't contain uv0"));
                }
            } else {
                return Err(anyhow!("Mesh primitive doesn't contain tangents"));
            }
        } else {
            return Err(anyhow!("Mesh primitive doesn't contain normals"));
        };

        //Uv1
        if let Some(tex_coords) = reader.read_tex_coords(1) {
            for (attribute, tex_coord) in attributes.iter_mut().zip(tex_coords.into_f32()) {
                attribute.tex_coords[2] = tex_coord[0];
                attribute.tex_coords[3] = tex_coord[1];
            }
        }

        //Color
        if let Some(colors) = reader.read_colors(1) {
            for (attribute, color) in attributes.iter_mut().zip(colors.into_rgba_f32()) {
                attribute.color = Vec4::from_array(color);
            }
        }
        create_vertex_buffer(device, &attributes)?
    };

    let skinning_buffer = if let Some(joints) = reader.read_joints(0) {
        if let Some(weights) = reader.read_weights(0) {
            let array: Vec<VertexSkinningAttributes> = joints
                .into_u16()
                .zip(weights.into_f32())
                .map(|(joint, weights)| VertexSkinningAttributes {
                    joint: glam::UVec4::new(
                        joint[0] as u32,
                        joint[1] as u32,
                        joint[2] as u32,
                        joint[3] as u32,
                    ),
                    weight: Vec4::from_array(weights),
                })
                .collect();
            Some(create_vertex_buffer(device, &array)?)
        } else {
            None
        }
    } else {
        None
    };

    let index_buffer = match reader.read_indices() {
        None => None,
        Some(indices) => {
            let indices_vec: Vec<u32> = indices.into_u32().collect();
            Some(IndexBuffer {
                count: indices_vec.len() as u32,
                buffer: create_index_buffer(device, &indices_vec)?,
            })
        }
    };

    Ok(Primitive {
        bounding_box,
        vertex_count,
        position_buffer,
        attributes_buffer,
        skinning_buffer,
        index_buffer,
    })
}

fn create_vertex_buffer<T>(
    device: &mut neptune_vulkan::Device,
    data: &[T],
) -> anyhow::Result<neptune_vulkan::BufferHandle> {
    let data_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
    };

    Ok(device.create_buffer_init(
        "Vertex Buffer",
        neptune_vulkan::BufferUsage::VERTEX | neptune_vulkan::BufferUsage::TRANSFER,
        MemoryLocation::GpuOnly,
        data_bytes,
    )?)
}

fn create_index_buffer(
    device: &mut neptune_vulkan::Device,
    data: &[u32],
) -> anyhow::Result<neptune_vulkan::BufferHandle> {
    let data_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
    };

    Ok(device.create_buffer_init(
        "Index Buffer",
        neptune_vulkan::BufferUsage::INDEX | neptune_vulkan::BufferUsage::TRANSFER,
        MemoryLocation::GpuOnly,
        data_bytes,
    )?)
}

pub fn load_materials(
    gltf_doc: &gltf::Document,
    images: &[ImageHandle],
    samplers: &GltfSamplers,
) -> Vec<Material> {
    gltf_doc
        .materials()
        .map(|gltf_material| Material {
            name: gltf_material
                .name()
                .unwrap_or("Unnamed Material")
                .to_string(),
            base_color: gltf_material
                .pbr_metallic_roughness()
                .base_color_factor()
                .into(),
            metallic_roughness_factor: Vec2::new(
                gltf_material.pbr_metallic_roughness().metallic_factor(),
                gltf_material.pbr_metallic_roughness().roughness_factor(),
            ),
            emissive_color: gltf_material.emissive_factor().into(),
            base_color_texture: gltf_material
                .pbr_metallic_roughness()
                .base_color_texture()
                .map(|info| {
                    load_material_texture(&info.texture(), info.tex_coord(), images, samplers)
                }),
            metallic_roughness_texture: gltf_material
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
                .map(|info| {
                    load_material_texture(&info.texture(), info.tex_coord(), images, samplers)
                }),
            normal_texture: gltf_material.normal_texture().map(|info| {
                // assert_eq!(
                //     info.scale(),
                //     1.0,
                //     "Normal Texture has a non-one scale, unsure of what todo here"
                // );
                load_material_texture(&info.texture(), info.tex_coord(), images, samplers)
            }),
            occlusion_texture: gltf_material.occlusion_texture().map(|info| {
                (
                    load_material_texture(&info.texture(), info.tex_coord(), images, samplers),
                    info.strength(),
                )
            }),
            emissive_texture: gltf_material.emissive_texture().map(|info| {
                load_material_texture(&info.texture(), info.tex_coord(), images, samplers)
            }),
        })
        .collect()
}

fn load_material_texture(
    texture: &gltf::Texture,
    uv_index: u32,
    images: &[ImageHandle],
    samplers: &GltfSamplers,
) -> MaterialTexture {
    let image = images[texture.source().index()];
    let sampler = if let Some(sampler_index) = texture.sampler().index() {
        samplers.samplers[sampler_index]
    } else {
        samplers.default
    };

    MaterialTexture {
        image,
        sampler,
        uv_index,
    }
}
