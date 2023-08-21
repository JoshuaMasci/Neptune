use crate::mesh::{
    BoundingBox, IndexBuffer, Mesh, Primitive, VertexAttributes, VertexSkinningAttributes,
};
use anyhow::anyhow;
use glam::{Vec3, Vec4};
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::vk;

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
        max: glam::Vec3::from_array(gltf_primitive.bounding_box().max),
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
                    weight: glam::Vec4::from_array(weights),
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

fn create_gltf_vertex_buffer<T: gltf::accessor::Item>(
    device: &mut neptune_vulkan::Device,
    iter: gltf::accessor::util::Iter<T>,
) -> anyhow::Result<neptune_vulkan::BufferHandle> {
    let data_vec: Vec<T> = iter.collect();
    create_vertex_buffer(device, &data_vec)
}

fn create_vertex_buffer<T>(
    device: &mut neptune_vulkan::Device,
    data: &[T],
) -> anyhow::Result<neptune_vulkan::BufferHandle> {
    let data_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
    };

    let buffer = device.create_buffer(
        "Vertex Buffer",
        &neptune_vulkan::BufferDescription {
            size: data_bytes.len() as vk::DeviceSize,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            memory_location: MemoryLocation::GpuOnly,
        },
    )?;
    device.update_data_to_buffer(buffer, data_bytes)?;
    Ok(buffer)
}

fn create_index_buffer(
    device: &mut neptune_vulkan::Device,
    data: &[u32],
) -> anyhow::Result<neptune_vulkan::BufferHandle> {
    let data_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
    };

    let buffer = device.create_buffer(
        "Index Buffer",
        &neptune_vulkan::BufferDescription {
            size: std::mem::size_of_val(data) as vk::DeviceSize,
            usage: vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            memory_location: MemoryLocation::GpuOnly,
        },
    )?;
    device.update_data_to_buffer(buffer, data_bytes)?;
    Ok(buffer)
}
