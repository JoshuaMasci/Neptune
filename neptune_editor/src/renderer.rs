use bytemuck::{Pod, Zeroable};
pub use neptune_core::log::{debug, error, info, trace, warn};
use std::borrow::Cow;
use std::iter;
use std::ops::Range;
use std::path::Path;
use wgpu::util::DeviceExt;
use wgpu::Device;

use crate::world::World;
use winit::window::Window;

pub(crate) struct Renderer {
    _instance: wgpu::Instance,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,

    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,

    mesh_pipeline: wgpu::RenderPipeline,
    cube_mesh: Mesh,
}

impl Renderer {
    pub(crate) fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
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
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&device, &config);

        let scene_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Scene Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&scene_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
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
                    offset: memoffset::offset_of!(Vertex, _position) as wgpu::BufferAddress,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: memoffset::offset_of!(Vertex, _normal) as wgpu::BufferAddress,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: memoffset::offset_of!(Vertex, _uv) as wgpu::BufferAddress,
                    shader_location: 2,
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
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let scene_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SceneBuffer"),
            size: std::mem::size_of::<SceneData>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &scene_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene_buffer.as_entire_binding(),
            }],
        });

        let cube_mesh = Mesh::load_obj(&device, Path::new("resource/cube.obj")).unwrap();

        Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            size,
            mesh_pipeline,
            scene_buffer,
            scene_bind_group,
            cube_mesh,
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
        {
            let scene_data = SceneData::from_world(world, [self.config.width, self.config.height]);
            self.queue
                .write_buffer(&self.scene_buffer, 0, bytemuck::cast_slice(&[scene_data]));
        }

        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

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
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
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
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.mesh_pipeline);
            render_pass.set_bind_group(0, &self.scene_bind_group, &[]);
            self.cube_mesh
                .draw(&mut render_pass, 0..world.entities.len() as u32);
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
    fn new(device: &Device, vertices: &[Vertex], indices: &[u32]) -> Self {
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

    fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, instances: Range<u32>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count as u32, 0, instances);
    }

    fn load_obj(device: &Device, file_path: &Path) -> Option<Self> {
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

pub const MAX_ENTITY_COUNT: usize = 512;

#[repr(C)]
#[derive(Clone, Copy)]
struct SceneData {
    view_matrix: glam::Mat4,
    projection_matrix: glam::Mat4,
    model_matrices: [glam::Mat4; MAX_ENTITY_COUNT],
}

unsafe impl bytemuck::Zeroable for SceneData {}
unsafe impl bytemuck::Pod for SceneData {}

impl SceneData {
    fn from_world(world: &World, size: [u32; 2]) -> Self {
        let view_matrix = world.camera_transform.get_centered_view_matrix();
        let projection_matrix = world.camera.get_infinite_reverse_perspective_matrix(size);
        let mut model_matrices = [Default::default(); MAX_ENTITY_COUNT];

        let camera_position = world.camera_transform.position;

        for (i, transform) in world.entities.iter().enumerate() {
            if i >= model_matrices.len() {
                break;
            }
            model_matrices[i] = transform.get_offset_model_matrix(camera_position);
        }

        Self {
            view_matrix,
            projection_matrix,
            model_matrices,
        }
    }
}

#[repr(packed)]
#[derive(Clone, Copy, Default)]
struct Vertex {
    _position: glam::Vec3,
    _normal: glam::Vec3,
    _uv: glam::Vec2,
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}
