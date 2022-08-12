use bytemuck::{Pod, Zeroable};
use std::ops::Range;
use std::path::Path;
use wgpu::util::DeviceExt;
use wgpu::Device;

#[repr(packed)]
#[derive(Clone, Copy, Default)]
pub(crate) struct Vertex {
    pub(crate) _position: glam::Vec3,
    pub(crate) _normal: glam::Vec3,
    pub(crate) _uv: glam::Vec2,
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: usize,
}

impl Mesh {
    pub(crate) fn new(device: &Device, vertices: &[Vertex], indices: &[u32]) -> Self {
        assert_ne!(vertices.len(), 0, "Mesh Vertices cannot be empty");
        assert_ne!(indices.len(), 0, "Mesh Indices cannot be empty");

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len(),
        }
    }

    pub(crate) fn draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: Range<u32>,
    ) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count as u32, 0, instances);
    }

    pub(crate) fn load_obj(device: &Device, file_path: &Path) -> Option<Self> {
        let options = tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ignore_points: true,
            ignore_lines: true,
        };

        tobj::load_obj(&file_path, &options)
            .map(|(models, _materials)| {
                //Get the first model for now
                let mesh = &models[0].mesh;

                let vertex_count = mesh.positions.len() / 3;

                let vertices: Vec<Vertex> = (0..vertex_count)
                    .map(|i| {
                        let i_2 = i * 2;
                        let i_3 = i * 3;
                        let range_2 = i_2..(i_2 + 2);
                        let range_3 = i_3..(i_3 + 3);

                        let position = glam::Vec3::from_slice(&mesh.positions[range_3.clone()]);
                        let normal =
                            glam::Vec3::from_slice(&mesh.normals[range_3.clone()]).normalize();
                        let uv = glam::Vec2::from_slice(&mesh.texcoords[range_2]);

                        Vertex {
                            _position: position,
                            _normal: normal,
                            _uv: uv,
                        }
                    })
                    .collect();

                Self::new(device, &vertices, &mesh.indices)
            })
            .ok()
    }
}
