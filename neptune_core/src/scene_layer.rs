use crate::render_backend::RenderDevice;
use crate::render_graph::pipeline_cache::{
    BlendFactor, BlendOp, CullMode, DepthTestMode, DepthTestOp, GraphicsPipelineDescription,
    VertexElement,
};
use crate::render_graph::render_graph::{RenderGraph, RenderPassBuilder};
use crate::render_graph::{render_graph, ImageHandle};
use crate::vulkan::{Buffer, BufferDescription};
use ash::vk;
use cgmath::Vector3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct ColorVertex {
    position: Vector3<f32>,
    color: Vector3<f32>,
}

pub struct SceneLayer {
    device: RenderDevice,

    triangle_index_count: u32,
    triangle_vertex_buffer: Buffer,
    triangle_index_buffer: Buffer,
    triangle_transfer: Option<(Vec<ColorVertex>, Vec<u32>)>,

    vertex_module: vk::ShaderModule,
    fragment_module: vk::ShaderModule,
}

fn read_shader_from_source(source: &[u8]) -> Vec<u32> {
    use std::io::Cursor;
    let mut cursor = Cursor::new(source);
    ash::util::read_spv(&mut cursor).expect("Failed to read spv")
}

impl SceneLayer {
    pub fn new(device: RenderDevice) -> Self {
        let triangle_vertex_data = vec![
            ColorVertex {
                position: Vector3::new(0.0, -0.75, 0.0),
                color: Vector3::new(1.0, 0.0, 0.0),
            },
            ColorVertex {
                position: Vector3::new(-0.75, 0.75, 0.0),
                color: Vector3::new(0.0, 1.0, 0.0),
            },
            ColorVertex {
                position: Vector3::new(0.75, 0.75, 0.0),
                color: Vector3::new(0.0, 0.0, 1.0),
            },
        ];

        let triangle_index_data: Vec<u32> = vec![0, 1, 2];

        let triangle_index_count: u32 = triangle_index_data.len() as u32;
        let triangle_vertex_buffer: Buffer = Buffer::new(
            &device,
            BufferDescription {
                size: std::mem::size_of::<ColorVertex>() * triangle_vertex_data.len(),
                usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                memory_location: gpu_allocator::MemoryLocation::GpuOnly,
            },
        );
        let triangle_index_buffer: Buffer = Buffer::new(
            &device,
            BufferDescription {
                size: std::mem::size_of::<u32>() * triangle_index_data.len(),
                usage: vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                memory_location: gpu_allocator::MemoryLocation::GpuOnly,
            },
        );

        let triangle_transfer = Some((triangle_vertex_data, triangle_index_data));

        let vertex_shader_code =
            read_shader_from_source(std::include_bytes!("../shaders/tri.vert.spv"));
        let fragment_shader_code =
            read_shader_from_source(std::include_bytes!("../shaders/tri.frag.spv"));

        let vertex_module = unsafe {
            device.base.create_shader_module(
                &vk::ShaderModuleCreateInfo::builder().code(&vertex_shader_code),
                None,
            )
        }
        .expect("Failed to create vertex module");

        let fragment_module = unsafe {
            device.base.create_shader_module(
                &vk::ShaderModuleCreateInfo::builder().code(&fragment_shader_code),
                None,
            )
        }
        .expect("Failed to create fragment module");

        Self {
            device,
            triangle_index_count,
            triangle_vertex_buffer,
            triangle_index_buffer,
            triangle_transfer,
            vertex_module,
            fragment_module,
        }
    }

    pub fn build_render_pass(&mut self, render_graph: &mut RenderGraph, target_image: ImageHandle) {
        let triangle_vertex_buffer = self.triangle_vertex_buffer.clone_no_drop();
        let triangle_index_buffer = self.triangle_index_buffer.clone_no_drop();
        let triangle_index_count = self.triangle_index_count;

        let triangle_transfer = self.triangle_transfer.take();

        let vertex_module = self.vertex_module;
        let fragment_module = self.fragment_module;

        render_graph.add_render_pass(
            RenderPassBuilder::new("ScenePass")
                .raster(vec![(target_image, [0.75, 0.5, 0.25, 0.0])], None)
                .render(
                    move |render_api, pipeline_cache, transfer_queue, pass_info, _resources| {
                        if let Some(triangle_transfer) = triangle_transfer {
                            transfer_queue.copy_to_buffer(
                                &triangle_vertex_buffer,
                                triangle_transfer.0.as_slice(),
                            );
                            transfer_queue.copy_to_buffer(
                                &triangle_index_buffer,
                                triangle_transfer.1.as_slice(),
                            );
                        }

                        let framebuffer_info = pass_info.framebuffer.as_ref().unwrap();
                        let framebuffer_layout = &framebuffer_info.layout;

                        let pipeline = pipeline_cache.get_graphics(
                            &GraphicsPipelineDescription {
                                vertex_module,
                                fragment_shader: Some(fragment_module),
                                cull_mode: CullMode::None,
                                depth_mode: DepthTestMode::None,
                                depth_op: DepthTestOp::Never,
                                src_factor: BlendFactor::One,
                                dst_factor: BlendFactor::Zero,
                                blend_op: BlendOp::None,
                                vertex_elements: vec![VertexElement::float3, VertexElement::float3],
                            },
                            framebuffer_layout,
                        );

                        unsafe {
                            render_api.device.base.cmd_bind_descriptor_sets(
                                render_api.command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                render_api.pipeline_layout,
                                0,
                                &[render_api.descriptor_set],
                                &[],
                            );

                            render_api.device.base.cmd_bind_pipeline(
                                render_api.command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                pipeline,
                            );

                            render_api.device.base.cmd_set_viewport(
                                render_api.command_buffer,
                                0,
                                &[vk::Viewport {
                                    x: 0.0,
                                    y: 0.0,
                                    width: framebuffer_info.size.width as f32,
                                    height: framebuffer_info.size.height as f32,
                                    min_depth: 0.0,
                                    max_depth: 1.0,
                                }],
                            );

                            render_api.device.base.cmd_set_scissor(
                                render_api.command_buffer,
                                0,
                                &[vk::Rect2D {
                                    offset: vk::Offset2D { x: 0, y: 0 },
                                    extent: vk::Extent2D {
                                        width: framebuffer_info.size.width,
                                        height: framebuffer_info.size.height,
                                    },
                                }],
                            );

                            render_api.device.base.cmd_bind_vertex_buffers(
                                render_api.command_buffer,
                                0,
                                &[triangle_vertex_buffer.handle],
                                &[0],
                            );
                            render_api.device.base.cmd_bind_index_buffer(
                                render_api.command_buffer,
                                triangle_index_buffer.handle,
                                0,
                                vk::IndexType::UINT32,
                            );

                            render_api.device.base.cmd_draw_indexed(
                                render_api.command_buffer,
                                triangle_index_count,
                                1,
                                0,
                                0,
                                0,
                            );
                        }
                    },
                ),
        );
    }
}

impl Drop for SceneLayer {
    fn drop(&mut self) {}
}
