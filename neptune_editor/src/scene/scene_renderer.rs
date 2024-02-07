use crate::camera::Camera;
use crate::material::{Material, MaterialTexture};
use crate::mesh;
use crate::mesh::{Mesh, Primitive};
use crate::transform::Transform;
use anyhow::Context;
use glam::{Mat4, Vec3};
use neptune_core::id_pool::IdPool;
use neptune_vulkan::gpu_allocator::MemoryLocation;
use neptune_vulkan::render_graph_builder::{BufferWriteCallback, RenderGraphBuilderTrait};
use neptune_vulkan::{
    vk, BufferUsage, Device, ImageDescription2D, ImageHandle, RasterPipelineHandle,
    SamplerDescription, TransientImageDesc, TransientImageSize,
};
use slotmap::SlotMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) unsafe fn slice_to_bytes_unsafe<T>(slice: &[T]) -> &[u8] {
    std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice))
}

pub struct SceneRenderer {
    depth_format: vk::Format,
    raster_pipeline: RasterPipelineHandle,
    default_texture: MaterialTexture,
}

impl SceneRenderer {
    pub fn new(device: &mut Device, depth_format: vk::Format) -> anyhow::Result<Self> {
        let raster_pipeline = {
            let vertex_shader_code = crate::shader::MESH_STATIC_VERT;
            let fragment_shader_code = crate::shader::MESH_FRAG;

            let vertex_state = neptune_vulkan::VertexState {
                shader: neptune_vulkan::ShaderStage {
                    code: vertex_shader_code,
                    entry: "main",
                },
                layouts: &[
                    mesh::VertexPosition::VERTEX_BUFFER_LAYOUT,
                    mesh::VertexAttributes::VERTEX_BUFFER_LAYOUT,
                ],
            };

            device.create_raster_pipeline(&neptune_vulkan::RasterPipelineDescription {
                vertex: vertex_state,
                primitive: neptune_vulkan::PrimitiveState {
                    front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                    cull_mode: vk::CullModeFlags::BACK,
                },
                depth_state: Some(neptune_vulkan::DepthState {
                    format: depth_format,
                    depth_enabled: true,
                    write_depth: true,
                    depth_op: vk::CompareOp::LESS,
                }),
                fragment: Some(neptune_vulkan::FragmentState {
                    shader: neptune_vulkan::ShaderStage {
                        code: fragment_shader_code,
                        entry: "main",
                    },
                    targets: &[neptune_vulkan::ColorTargetState {
                        format: vk::Format::B8G8R8A8_UNORM,
                        blend: None,
                        write_mask: vk::ColorComponentFlags::RGBA,
                    }],
                }),
            })?
        };

        let default_texture = MaterialTexture {
            image: device.create_image_init(
                "Default Image",
                &ImageDescription2D {
                    size: [1; 2],
                    format: vk::Format::R8G8B8A8_UNORM,
                    usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                    mip_levels: 1,
                    location: MemoryLocation::GpuOnly,
                },
                &[255u8; 4],
            )?,
            sampler: device.create_sampler("Default Sampler", &SamplerDescription::default())?,
            uv_index: 0,
        };

        Ok(Self {
            depth_format,
            raster_pipeline,
            default_texture,
        })
    }
    pub fn write_render_passes<T: RenderGraphBuilderTrait>(
        &mut self,
        target_image: ImageHandle,
        camera: &SceneCamera,
        scene: &Scene,
        render_graph_builder: &mut T,
    ) {
        let depth_image = render_graph_builder.create_transient_image(TransientImageDesc {
            size: TransientImageSize::Relative([1.0; 2], target_image),
            format: self.depth_format,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            mip_levels: 1,
            memory_location: MemoryLocation::GpuOnly,
        });

        let mut raster_pass_builder =
            neptune_vulkan::render_graph_builder::RasterPassBuilder::new("Swapchain Pass");
        raster_pass_builder.add_color_attachment(target_image, Some([0.0, 0.0, 0.0, 1.0]));
        raster_pass_builder.add_depth_stencil_attachment(depth_image, Some((1.0, 0)));

        for (_key, instance) in scene.instance_map.iter() {
            for model_primitive in instance.model.primitives.iter() {
                let texture = model_primitive
                    .material
                    .as_ref()
                    .and_then(|material| material.base_color_texture.clone())
                    .unwrap_or_else(|| self.default_texture.clone());

                let mut draw_command_builder =
                    neptune_vulkan::render_graph_builder::RasterDrawCommandBuilder::new(
                        self.raster_pipeline,
                    );

                draw_command_builder.add_vertex_buffer(
                    neptune_vulkan::render_graph_builder::BufferOffset {
                        buffer: model_primitive.primitive.position_buffer,
                        offset: 0,
                    },
                );
                draw_command_builder.add_vertex_buffer(
                    neptune_vulkan::render_graph_builder::BufferOffset {
                        buffer: model_primitive.primitive.attributes_buffer,
                        offset: 0,
                    },
                );
                draw_command_builder.read_buffer(camera.camera_buffer);
                draw_command_builder.read_buffer(scene.model_matrix_buffer);
                draw_command_builder.read_sampler(texture.sampler);
                draw_command_builder.read_sampled_image(texture.image);

                let instance_range = (instance.index as u32)..(instance.index as u32 + 1);

                if let Some(index_buffer_ref) = &model_primitive.primitive.index_buffer {
                    draw_command_builder.draw_indexed(
                        0,
                        0..index_buffer_ref.count,
                        instance_range,
                        neptune_vulkan::render_graph_builder::BufferOffset {
                            buffer: index_buffer_ref.buffer,
                            offset: 0,
                        },
                        neptune_vulkan::render_graph::IndexType::U32,
                    );
                } else {
                    draw_command_builder.draw(
                        0..model_primitive.primitive.vertex_count as u32,
                        instance_range,
                    );
                }

                draw_command_builder.build(&mut raster_pass_builder);
            }
        }

        raster_pass_builder.build(render_graph_builder);
    }
}

#[derive(Clone)]
pub struct Model {
    pub name: String,
    pub primitives: Vec<ModelPrimitive>,
}

#[derive(Clone)]
pub struct ModelPrimitive {
    pub primitive: Arc<Primitive>,
    pub material: Option<Arc<Material>>,
}

#[derive(Default, Copy, Clone)]
pub struct SceneInstanceHandle(slotmap::DefaultKey);

struct SceneInstance {
    index: usize,
    transform: Transform,
    model: Model,
}

pub struct Scene {
    instance_map: SlotMap<slotmap::DefaultKey, SceneInstance>,

    model_matrix_index_pool: IdPool,
    model_matrix_buffer: neptune_vulkan::BufferHandle,
    model_matrix_buffer_size: usize,
    model_matrix_data: Rc<RefCell<Vec<Mat4>>>,
}

impl Scene {
    pub fn new(device: &mut Device, instance_count: usize) -> anyhow::Result<Self> {
        let model_matrix_data = vec![Mat4::ZERO; instance_count];
        let model_matrix_buffer = device
            .create_buffer_init(
                "ModelMatrixBuffer",
                BufferUsage::STORAGE | BufferUsage::TRANSFER,
                MemoryLocation::GpuOnly,
                unsafe { slice_to_bytes_unsafe(&model_matrix_data) },
            )
            .context("Failed to create camera buffer")?;
        let model_matrix_index_pool = IdPool::new(0..instance_count);
        let model_matrix_buffer_size = instance_count * std::mem::size_of::<Mat4>();

        let instance_map = SlotMap::default();

        Ok(Self {
            instance_map,
            model_matrix_index_pool,
            model_matrix_buffer,
            model_matrix_buffer_size,
            model_matrix_data: Rc::new(RefCell::new(model_matrix_data)),
        })
    }

    pub fn add_instance(
        &mut self,
        transform: Transform,
        model: Model,
    ) -> Option<SceneInstanceHandle> {
        if let Some(index) = self.model_matrix_index_pool.get() {
            let mut data_mut = self.model_matrix_data.borrow_mut();
            data_mut[index] = transform.model_matrix();
            Some(SceneInstanceHandle(self.instance_map.insert(
                SceneInstance {
                    index,
                    transform,
                    model,
                },
            )))
        } else {
            //Out of space
            None
        }
    }
    pub fn remove_instance(&mut self, instance_handle: SceneInstanceHandle) {
        if let Some(instance) = self.instance_map.remove(instance_handle.0) {
            //Clear the old matrix,
            let mut data_mut = self.model_matrix_data.borrow_mut();
            data_mut[instance.index] = Mat4::ZERO;

            self.model_matrix_index_pool.free(instance.index);
        } else {
            warn!("SceneInstance({:?}) doesn't exist", instance_handle.0)
        }
    }

    pub fn update_instance(&mut self, instance_handle: SceneInstanceHandle, transform: Transform) {
        if let Some(instance) = self.instance_map.get_mut(instance_handle.0) {
            let mut data_mut = self.model_matrix_data.borrow_mut();
            data_mut[instance.index] = transform.model_matrix();

            instance.transform = transform;
        } else {
            warn!("SceneInstance({:?}) doesn't exist", instance_handle.0)
        }
    }

    //TODO: optimize this for shared memory platform
    pub fn write_render_passes<T: RenderGraphBuilderTrait>(
        &mut self,
        render_graph_builder: &mut T,
    ) {
        let transient_model_matrix_buffer = render_graph_builder.create_transient_buffer(
            self.model_matrix_buffer_size,
            BufferUsage::TRANSFER,
            MemoryLocation::CpuToGpu,
        );

        let model_matrix_data_clone = self.model_matrix_data.clone();
        render_graph_builder.add_mapped_buffer_write(
            transient_model_matrix_buffer,
            BufferWriteCallback::new(move |slice| {
                let model_matrix_data = model_matrix_data_clone.borrow();
                slice.copy_from_slice(unsafe { slice_to_bytes_unsafe(&model_matrix_data) });
            }),
        );

        let mut data_upload_pass = neptune_vulkan::render_graph_builder::TransferPassBuilder::new(
            "Camera Buffer Upload Pass",
            neptune_vulkan::render_graph::QueueType::Graphics,
        );
        data_upload_pass.copy_buffer_to_buffer(
            neptune_vulkan::render_graph_builder::BufferOffset {
                buffer: transient_model_matrix_buffer,
                offset: 0,
            },
            neptune_vulkan::render_graph_builder::BufferOffset {
                buffer: self.model_matrix_buffer,
                offset: 0,
            },
            self.model_matrix_buffer_size,
        );
        data_upload_pass.build(render_graph_builder);
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone)]
struct SceneCameraData {
    view_projection_matrix: Mat4,
    camera_position: Vec3,
}

impl SceneCameraData {
    fn new(camera: &Camera, camera_transform: &Transform, aspect_ratio: f32) -> Self {
        let projection_matrix = camera.projection_matrix(aspect_ratio);
        let view_matrix = camera_transform.view_matrix();
        let view_projection_matrix = projection_matrix * view_matrix;
        Self {
            view_projection_matrix,
            camera_position: camera_transform.position,
        }
    }
}

pub struct SceneCamera {
    camera_buffer: neptune_vulkan::BufferHandle,
    camera_data: Rc<RefCell<SceneCameraData>>,
}

impl SceneCamera {
    pub fn new(device: &mut Device) -> anyhow::Result<Self> {
        let camera_data = SceneCameraData::default();
        let camera_buffer = device
            .create_buffer_init(
                "SceneCamera",
                BufferUsage::STORAGE | BufferUsage::TRANSFER,
                MemoryLocation::GpuOnly,
                unsafe { slice_to_bytes_unsafe(&[camera_data.clone()]) },
            )
            .context("Failed to create camera buffer")?;
        Ok(Self {
            camera_buffer,
            camera_data: Rc::new(RefCell::new(camera_data)),
        })
    }

    pub fn update(&mut self, camera: &Camera, camera_transform: &Transform, aspect_ratio: f32) {
        let mut data_mut = self.camera_data.borrow_mut();
        *data_mut = SceneCameraData::new(camera, camera_transform, aspect_ratio);
    }

    //TODO: optimize this for shared memory platform
    pub fn write_render_passes<T: RenderGraphBuilderTrait>(
        &mut self,
        render_graph_builder: &mut T,
    ) {
        let buffer_size = std::mem::size_of::<SceneCameraData>();
        let transient_camera_buffer = render_graph_builder.create_transient_buffer(
            buffer_size,
            BufferUsage::TRANSFER,
            MemoryLocation::CpuToGpu,
        );

        let camera_data_clone = self.camera_data.clone();
        render_graph_builder.add_mapped_buffer_write(
            transient_camera_buffer,
            BufferWriteCallback::new(move |slice| {
                let camera_data = camera_data_clone.borrow().clone();
                slice.copy_from_slice(unsafe { slice_to_bytes_unsafe(&[camera_data]) });
            }),
        );

        let mut data_upload_pass = neptune_vulkan::render_graph_builder::TransferPassBuilder::new(
            "Camera Buffer Upload Pass",
            neptune_vulkan::render_graph::QueueType::Graphics,
        );
        data_upload_pass.copy_buffer_to_buffer(
            neptune_vulkan::render_graph_builder::BufferOffset {
                buffer: transient_camera_buffer,
                offset: 0,
            },
            neptune_vulkan::render_graph_builder::BufferOffset {
                buffer: self.camera_buffer,
                offset: 0,
            },
            buffer_size,
        );
        data_upload_pass.build(render_graph_builder);
    }
}
