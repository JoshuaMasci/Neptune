use bytemuck::{Pod, Zeroable};
pub use neptune_core::log::{debug, error, info, trace, warn};
use std::borrow::Cow;
use std::collections::HashMap;
use std::iter;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use wgpu::{BindingResource, BufferBinding, ColorTargetState};

use crate::rendering::camera::Camera;
use crate::rendering::material::Material;
use crate::rendering::mesh::{Mesh, Vertex};
use winit::window::Window;

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

    pipeline_layout: wgpu::PipelineLayout,
    depth_format: wgpu::TextureFormat,

    material_map: HashMap<PathBuf, Weak<Material>>,
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
            camera_buffer,
            camera_bind_group,
            transforms_buffer,
            transforms_bind_group,
            pipeline_layout,
            depth_format: wgpu::TextureFormat::Depth24Plus,
            material_map: HashMap::new(),
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

    pub(crate) fn render_game_world(
        &mut self,
        camera: &Camera,
        camera_transform: &crate::game::Transform,
        world: &crate::game::World,
    ) -> Result<(), wgpu::SurfaceError> {
        let camera_data = CameraData {
            view_matrix: camera_transform.get_centered_view_matrix(),
            projection_matrix: camera
                .get_infinite_reverse_perspective_matrix([self.size.width, self.size.height]),
        };
        self.camera_buffer.write(&self.queue, camera_data);

        let camera_position = camera_transform.position;

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

            for (index, object) in world.static_objects.iter().enumerate() {
                if let Some(model) = &object.0.model {
                    let model_matrix = object.0.transform.get_offset_model_matrix(camera_position);

                    let offset = std::mem::size_of::<glam::Mat4>() * index;

                    self.queue.write_buffer(
                        &self.transforms_buffer,
                        offset as wgpu::BufferAddress,
                        bytemuck::bytes_of(&model_matrix),
                    );

                    render_pass.set_pipeline(&model.material.pipeline);
                    render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

                    render_pass.set_bind_group(
                        1,
                        &self.transforms_bind_group,
                        &[offset as wgpu::DynamicOffset],
                    );

                    model.mesh.draw(&mut render_pass, 0..1);
                }
            }

            let mesh_offset = world.static_objects.len();
            for (index, object) in world.dynamic_objects.iter().enumerate() {
                if let Some(model) = &object.object.model {
                    let model_matrix = object
                        .object
                        .transform
                        .get_offset_model_matrix(camera_position);

                    let offset = std::mem::size_of::<glam::Mat4>() * (index + mesh_offset);

                    self.queue.write_buffer(
                        &self.transforms_buffer,
                        offset as wgpu::BufferAddress,
                        bytemuck::bytes_of(&model_matrix),
                    );

                    render_pass.set_pipeline(&model.material.pipeline);
                    render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

                    render_pass.set_bind_group(
                        1,
                        &self.transforms_bind_group,
                        &[offset as wgpu::DynamicOffset],
                    );

                    model.mesh.draw(&mut render_pass, 0..1);
                }
            }
        }
        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn get_material(&mut self, path: &str) -> Option<Arc<Material>> {
        let path_buf = PathBuf::from(path);

        //If material is not loaded or has been unloaded this will be none
        let mut loaded_material = self
            .material_map
            .get(&path_buf)
            .map(|material_ref| material_ref.upgrade())
            .unwrap_or_default();

        //If material is not loaded, try to load it
        if loaded_material.is_none() {
            if let Ok(code_string) = std::fs::read_to_string(path) {
                let shader = self
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(path),
                        source: wgpu::ShaderSource::Wgsl(Cow::from(code_string)),
                    });

                let vertex_buffer_layout = [wgpu::VertexBufferLayout {
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

                let pipeline =
                    self.device
                        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                            label: None,
                            layout: Some(&self.pipeline_layout),
                            vertex: wgpu::VertexState {
                                module: &shader,
                                entry_point: "vs_main",
                                buffers: &vertex_buffer_layout,
                            },
                            fragment: Some(wgpu::FragmentState {
                                module: &shader,
                                entry_point: "fs_main",
                                targets: &[Some(ColorTargetState::from(self.config.format))],
                            }),
                            primitive: wgpu::PrimitiveState {
                                ..Default::default()
                            },
                            depth_stencil: Some(wgpu::DepthStencilState {
                                format: self.depth_format,
                                depth_write_enabled: true,
                                depth_compare: wgpu::CompareFunction::Greater,
                                stencil: Default::default(),
                                bias: Default::default(),
                            }),
                            multisample: wgpu::MultisampleState::default(),
                            multiview: None,
                        });

                let material = Arc::new(Material { pipeline });
                let _ = self
                    .material_map
                    .insert(path_buf, Arc::downgrade(&material));
                loaded_material = Some(material)
            }
        }

        loaded_material
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
