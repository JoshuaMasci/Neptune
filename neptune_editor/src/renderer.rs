use bytemuck::{Pod, Zeroable};
pub use neptune_core::log::{debug, error, info, trace, warn};
use std::borrow::Cow;
use std::iter;
use wgpu::util::DeviceExt;
use wgpu::Device;

use crate::world::World;
use winit::window::Window;

pub(crate) struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,

    mesh_pipeline: wgpu::RenderPipeline,
    cube_mesh: Mesh,
    tri_mesh: Mesh,
}

impl Renderer {
    pub(crate) fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::DX12);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ))
        .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&device, &config);

        let (vertex_data, index_data) = create_vertices();
        let cube_mesh = Mesh::new(&device, &vertex_data, &index_data);

        let tri_mesh = Mesh::new(
            &device,
            &[
                vertex([0.0, 0.5, 0.0], [1.0, 0.0, 0.0, 0.0]),
                vertex([-0.5, -0.5, 0.0], [0.0, 0.0, 1.0, 0.0]),
                vertex([0.5, -0.5, 0.0], [0.0, 1.0, 0.0, 0.0]),
            ],
            &[0, 1, 2],
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../shader/triangle.wgsl"
            ))),
        });

        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: (std::mem::size_of::<f32>() * 4) as wgpu::BufferAddress,
                    shader_location: 1,
                },
            ],
        }];

        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[config.format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            mesh_pipeline,
            cube_mesh,
            tri_mesh,
        }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub(crate) fn update(&mut self) {}

    pub(crate) fn render(&mut self, world: &World) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.01,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.mesh_pipeline);
            render_pass.set_vertex_buffer(0, self.tri_mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.tri_mesh.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..self.tri_mesh.index_count as u32, 0, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: usize,
}

impl Mesh {
    fn new(device: &Device, vertices: &[Vertex], indices: &[u16]) -> Self {
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
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 3],
    _color: [f32; 4],
}

fn vertex(pos: [f32; 3], color: [f32; 4]) -> Vertex {
    Vertex {
        _pos: pos,
        _color: color,
    }
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        // top (0, 0, 1)
        vertex([-1.0, -1.0, 1.0], [0.0, 0.0, 0.0, 0.0]),
        vertex([1.0, -1.0, 1.0], [1.0, 0.0, 0.0, 0.0]),
        vertex([1.0, 1.0, 1.0], [1.0, 1.0, 0.0, 0.0]),
        vertex([-1.0, 1.0, 1.0], [0.0, 1.0, 0.0, 0.0]),
        // bottom (0.0, 0.0, -1.0)
        vertex([-1.0, 1.0, -1.0], [1.0, 0.0, 0.0, 0.0]),
        vertex([1.0, 1.0, -1.0], [0.0, 0.0, 0.0, 0.0]),
        vertex([1.0, -1.0, -1.0], [0.0, 1.0, 0.0, 0.0]),
        vertex([-1.0, -1.0, -1.0], [1.0, 1.0, 0.0, 0.0]),
        // right (1.0, 0.0, 0.0)
        vertex([1.0, -1.0, -1.0], [0.0, 0.0, 0.0, 0.0]),
        vertex([1.0, 1.0, -1.0], [1.0, 0.0, 0.0, 0.0]),
        vertex([1.0, 1.0, 1.0], [1.0, 1.0, 0.0, 0.0]),
        vertex([1.0, -1.0, 1.0], [0.0, 1.0, 0.0, 0.0]),
        // left (-1.0, 0.0, 0.0)
        vertex([-1.0, -1.0, 1.0], [1.0, 0.0, 0.0, 0.0]),
        vertex([-1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 0.0]),
        vertex([-1.0, 1.0, -1.0], [0.0, 1.0, 0.0, 0.0]),
        vertex([-1.0, -1.0, -1.0], [1.0, 1.0, 0.0, 0.0]),
        // front (0.0, 1.0, 0.0)
        vertex([1.0, 1.0, -1.0], [1.0, 0.0, 0.0, 0.0]),
        vertex([-1.0, 1.0, -1.0], [0.0, 0.0, 0.0, 0.0]),
        vertex([-1.0, 1.0, 1.0], [0.0, 1.0, 0.0, 0.0]),
        vertex([1.0, 1.0, 1.0], [1.0, 1.0, 0.0, 0.0]),
        // back (0.0, -1.0, 0.0)
        vertex([1.0, -1.0, 1.0], [0.0, 0.0, 0.0, 0.0]),
        vertex([-1.0, -1.0, 1.0], [1.0, 0.0, 0.0, 0.0]),
        vertex([-1.0, -1.0, -1.0], [1.0, 1.0, 0.0, 0.0]),
        vertex([1.0, -1.0, -1.0], [0.0, 1.0, 0.0, 0.0]),
    ];

    let index_data: &[u16] = &[
        0, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    (vertex_data.to_vec(), index_data.to_vec())
}
