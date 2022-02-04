use crate::render_backend::RenderDevice;
use crate::render_graph::render_graph::PipelineDescription;
use crate::render_graph::{render_graph, ImageHandle};
use crate::vulkan::{Buffer, BufferDescription};
use ash::vk;
use ash::vk::Pipeline;
use cgmath::Vector3;

#[repr(C)]
#[derive(Copy, Clone)]
struct ColorVertex {
    position: Vector3<f32>,
    color: Vector3<f32>,
}

#[allow(dead_code)]
impl ColorVertex {
    //TODO: make trait?
    fn get_vertex_layout() {}
}

pub struct SceneLayer {
    device: RenderDevice,

    triangle_index_count: u32,
    triangle_vertex_buffer: Buffer,
    triangle_index_buffer: Buffer,
    triangle_transfer: Option<(Vec<ColorVertex>, Vec<u32>)>,

    //TODO: use render graph pipeline
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
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

        Self {
            device,
            triangle_index_count,
            triangle_vertex_buffer,
            triangle_index_buffer,
            triangle_transfer,
            pipeline_layout: Default::default(),
            pipeline: Default::default(),
        }
    }

    pub fn build_render_pass(
        &mut self,
        rgb: &mut render_graph::RenderGraphBuilder,
        target_image: ImageHandle,
    ) {
        let mut scene_pass = rgb.create_pass("ScenePass");
        scene_pass.raster(vec![(target_image, [0.75, 0.5, 0.25, 0.0])], None);

        let triangle_vertex_buffer = self.triangle_vertex_buffer.clone_no_drop();
        let triangle_index_buffer = self.triangle_index_buffer.clone_no_drop();
        let triangle_transfer = self.triangle_transfer.take();

        scene_pass.render(move |render_api, transfer_queue, pass_info, resources| {
            if let Some((vertex_data, index_data)) = triangle_transfer {
                transfer_queue.copy_to_buffer(&triangle_vertex_buffer, &vertex_data);
                transfer_queue.copy_to_buffer(&triangle_index_buffer, &index_data);
            }
        });
    }
}

impl Drop for SceneLayer {
    fn drop(&mut self) {}
}
