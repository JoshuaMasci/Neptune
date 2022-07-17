use bytemuck::{Pod, Zeroable};
pub use neptune_core::log::{debug, error, info, trace, warn};
use std::borrow::Cow;
use std::collections::HashMap;
use std::iter;
use std::marker::PhantomData;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use wgpu::util::DeviceExt;
use wgpu::{BindingResource, BufferBinding, Device};

use crate::camera::Camera;
use crate::transform::Transform;
use crate::world::World;
use winit::window::Window;

#[repr(packed)]
#[derive(Clone, Copy, Default)]
pub(crate) struct Vertex {
    _position: glam::Vec3,
    _normal: glam::Vec3,
    _uv: glam::Vec2,
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CameraData {
    view_matrix: glam::Mat4,
    projection_matrix: glam::Mat4,
}
unsafe impl bytemuck::Zeroable for CameraData {}
unsafe impl bytemuck::Pod for CameraData {}

pub(crate) struct Renderer {
    _instance: wgpu::Instance,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,

    camera_buffer: UniformBuffer<CameraData>,
    camera_bind_group: wgpu::BindGroup,

    transforms_buffer: wgpu::Buffer,
    transforms_bind_group: wgpu::BindGroup,

    mesh_pipeline: wgpu::RenderPipeline,

    mesh_map: HashMap<PathBuf, Weak<Mesh>>,
}

impl Renderer {
    const MAX_TRANSFORMS: usize = 4096;
    const TRANSFORMS_BUFFER_SIZE: wgpu::BufferAddress =
        (Self::MAX_TRANSFORMS * std::mem::size_of::<glam::Mat4>()) as wgpu::BufferAddress;

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
                features: wgpu::Features::default(),
                limits: wgpu::Limits {
                    min_uniform_buffer_offset_alignment: std::mem::size_of::<glam::Mat4>() as u32,
                    ..Default::default()
                },
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

        let transforms_buffer_size =
            Some(wgpu::BufferSize::new(std::mem::size_of::<glam::Mat4>() as u64).unwrap());

        let transforms_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Mesh Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: transforms_buffer_size,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&scene_bind_group_layout, &transforms_bind_group_layout],
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

        let camera_buffer = UniformBuffer::new(&device, &queue, CameraData::default());

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &scene_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.buffer().as_entire_binding(),
            }],
        });

        let transforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transforms Buffer"),
            size: Self::TRANSFORMS_BUFFER_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let transforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &transforms_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &transforms_buffer,
                    offset: 0,
                    size: transforms_buffer_size,
                }),
            }],
        });

        Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            size,
            mesh_pipeline,
            camera_buffer,
            camera_bind_group,
            transforms_buffer,
            transforms_bind_group,
            mesh_map: HashMap::new(),
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

    pub(crate) fn render(
        &mut self,
        camera: &Camera,
        camera_transform: &Transform,
        world: &World,
    ) -> Result<(), wgpu::SurfaceError> {
        let camera_data = CameraData {
            view_matrix: camera_transform.get_centered_view_matrix(),
            projection_matrix: camera
                .get_infinite_reverse_perspective_matrix([self.size.width, self.size.height]),
        };
        self.camera_buffer.write(&self.queue, camera_data);

        let camera_position = camera_transform.position;

        let entity_count = usize::min(world.entities.len(), Self::MAX_TRANSFORMS);

        for i in 0..entity_count {
            let offset = (std::mem::size_of::<glam::Mat4>() * i) as wgpu::BufferAddress;
            let model_matrix = world.entities[i]
                .transform
                .get_offset_model_matrix(camera_position);
            self.queue.write_buffer(
                &self.transforms_buffer,
                offset,
                bytemuck::bytes_of(&model_matrix),
            )
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
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            for i in 0..entity_count {
                let offset = (std::mem::size_of::<glam::Mat4>() * i) as wgpu::DynamicOffset;
                render_pass.set_bind_group(1, &self.transforms_bind_group, &[offset]);
                world.entities[i].mesh.draw(&mut render_pass, 0..1);
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn get_mesh(&mut self, path: &str) -> Option<Arc<Mesh>> {
        let path_buf = PathBuf::from(path);

        //If mesh is not loaded or has been unloaded this will be none
        let mut loaded_mesh = self
            .mesh_map
            .get(&path_buf)
            .map(|mesh_ref| mesh_ref.upgrade())
            .unwrap_or_default();

        //If mesh is not loaded, try to load it
        if loaded_mesh.is_none() {
            let is_loaded = Mesh::load_obj(&self.device, &path_buf);
            if let Some(mesh) = is_loaded {
                let mesh = Arc::new(mesh);
                let _ = self.mesh_map.insert(path_buf, Arc::downgrade(&mesh));
                loaded_mesh = Some(mesh)
            }
        }

        loaded_mesh
    }
}

pub(crate) struct Mesh {
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

pub struct UniformBuffer<T: bytemuck::Pod> {
    buffer: wgpu::Buffer,
    phantom: PhantomData<T>,
}

impl<T: bytemuck::Pod> UniformBuffer<T> {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, data: T) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UniformBuffer"),
            size: std::mem::size_of::<T>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let new_self = Self {
            buffer,
            phantom: Default::default(),
        };
        new_self.write(queue, data);
        new_self
    }

    pub fn write(&self, queue: &wgpu::Queue, data: T) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}
