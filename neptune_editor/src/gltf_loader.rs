use crate::mesh::{BoundingBox, IndexBuffer, Mesh, Primitive, SkinningBuffers};
use anyhow::anyhow;
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::vk;

const MAX_BUFFER_ARRAYS: u32 = 8;

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
        min: glam::Vec3::from_array(gltf_primitive.bounding_box().min),
        max: glam::Vec3::from_array(gltf_primitive.bounding_box().max),
    };

    let (position_buffer, vertex_count) = match reader.read_positions() {
        None => return Err(anyhow!("Mesh contains no vertex positions")),
        Some(positions) => {
            let vertex_count = positions.len();
            (create_gltf_vertex_buffer(device, positions)?, vertex_count)
        }
    };

    let normal_buffer = match reader.read_normals() {
        None => None,
        Some(normals) => {
            assert_eq!(
                vertex_count,
                normals.len(),
                "All buffers must have the same vertex count"
            );
            Some(create_gltf_vertex_buffer(device, normals)?)
        }
    };

    let tangent_buffer = match reader.read_tangents() {
        None => None,
        Some(tangents) => {
            assert_eq!(
                vertex_count,
                tangents.len(),
                "All buffers must have the same vertex count"
            );
            Some(create_gltf_vertex_buffer(device, tangents)?)
        }
    };

    let mut tex_coord_buffers = Vec::new();
    for i in 0..MAX_BUFFER_ARRAYS {
        if let Some(tex_coords) = reader.read_tex_coords(i) {
            let tex_coord_vec: Vec<[f32; 2]> = tex_coords.into_f32().collect();
            assert_eq!(
                vertex_count,
                tex_coord_vec.len(),
                "All buffers must have the same vertex count"
            );
            tex_coord_buffers.push(create_vertex_buffer(device, &tex_coord_vec)?);
        } else {
            break;
        }
    }

    let mut color_buffers = Vec::new();
    for i in 0..MAX_BUFFER_ARRAYS {
        if let Some(colors) = reader.read_colors(i) {
            let color_vec: Vec<[f32; 4]> = colors.into_rgba_f32().collect();
            assert_eq!(
                vertex_count,
                color_vec.len(),
                "All buffers must have the same vertex count"
            );
            color_buffers.push(create_vertex_buffer(device, &color_vec)?);
        } else {
            break;
        }
    }

    let mut skinning_buffers = Vec::new();
    for i in 0..MAX_BUFFER_ARRAYS {
        let joint_accessor = reader.read_joints(i);
        let weight_accessor = reader.read_weights(i);

        if joint_accessor.is_some() && weight_accessor.is_some() {
            let joints = joint_accessor.unwrap().into_u16();
            let weights = weight_accessor.unwrap().into_f32();

            assert_eq!(
                vertex_count,
                joints.len(),
                "All buffers must have the same vertex count"
            );
            assert_eq!(
                vertex_count,
                weights.len(),
                "All buffers must have the same vertex count"
            );

            let joint_vec: Vec<[u16; 4]> = joints.collect();
            let weight_vec: Vec<[f32; 4]> = weights.collect();

            skinning_buffers.push(SkinningBuffers {
                joint_buffer: create_vertex_buffer(device, &joint_vec)?,
                weight_buffer: create_vertex_buffer(device, &weight_vec)?,
            });
        } else if joint_accessor.is_none() && weight_accessor.is_none() {
            break;
        } else {
            return Err(anyhow!("Mismatched Skinning Buffers"));
        }
    }

    let index_buffer = match reader.read_indices() {
        None => None,
        Some(indices) => {
            let indices_vec: Vec<u32> = indices.into_u32().collect();
            Some(IndexBuffer {
                count: indices_vec.len() as u32,
                buffer: create_vertex_buffer(device, &indices_vec)?,
            })
        }
    };

    Ok(Primitive {
        bounding_box,
        vertex_count,
        position_buffer,
        normal_buffer,
        tangent_buffer,
        tex_coord_buffers,
        color_buffers,
        skinning_buffers,
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
        &neptune_vulkan::BufferDesc {
            size: data_bytes.len() as vk::DeviceSize,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            memory_location: MemoryLocation::GpuOnly,
        },
    )?;
    device.update_data_to_buffer(buffer, &data_bytes)?;
    Ok(buffer)
}

fn create_index_buffer(
    device: &mut neptune_vulkan::Device,
    data: &[u8],
) -> anyhow::Result<neptune_vulkan::BufferHandle> {
    let buffer = device.create_buffer(
        "Index Buffer",
        &neptune_vulkan::BufferDesc {
            size: data.len() as vk::DeviceSize,
            usage: vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            memory_location: MemoryLocation::GpuOnly,
        },
    )?;
    device.update_data_to_buffer(buffer, &data)?;
    Ok(buffer)
}
