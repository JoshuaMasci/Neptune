use crate::{DeviceTrait, IndexSize};
use std::ops::Range;

//TODO: Should RenderGraphBuilder just be a trait? letting is manage internally best for that backend

pub struct RenderGraphBuilderImpl<Device: DeviceTrait> {
    //Just used so the compiler doesn't complain about unused generic type
    used_buffers: Vec<Device::Buffer>,
}

impl<Device: DeviceTrait> Default for RenderGraphBuilderImpl<Device> {
    fn default() -> Self {
        Self {
            used_buffers: vec![],
        }
    }
}

impl<Device: DeviceTrait> RenderGraphBuilderImpl<Device> {
    pub fn transfer_buffer_to_buffer(
        &mut self,
        src: Device::Buffer,
        src_offset: usize,
        dst: Device::Buffer,
        dst_offset: usize,
        size: usize,
    ) {
        self.used_buffers.push(src);
        self.used_buffers.push(dst);
        let _ = src_offset;
        let _ = dst_offset;
        let _ = size;
    }
}

pub trait RenderGraphBuilderTrait {
    type Device: DeviceTrait;
    type Buffer: Sync + Clone;
    type Texture: Sync + Clone;
    type Sampler: Sync + Clone;

    fn add_compute_pass(builder: ComputePassBuilder<Self::Device>);
    fn add_raster_pass(builder: RasterPassBuilder<Self::Device>);
}

pub enum ResourceUsage<Device: DeviceTrait> {
    BufferRead(Device::Buffer),
    BufferWrite(Device::Buffer),
    TextureSample(Device::Texture),
    TextureSampler(Device::Texture),
    TextureStorageWrite(Device::Texture),
    TextureStorageRead(Device::Texture),
}

pub struct ComputePassBuilder<Device: DeviceTrait> {
    shader: Device::ComputeShader,
    dispatch: [u32; 3],
    resources: Vec<ResourceUsage<Device>>,
}

pub struct Attachment<Device: DeviceTrait, T> {
    texture: Device::Texture,
    clear_value: Option<T>,
}

impl<Device: DeviceTrait, T> Attachment<Device, T> {
    pub fn new(texture: Device::Texture) -> Self {
        Self {
            texture,
            clear_value: None,
        }
    }

    pub fn new_with_clear(texture: Device::Texture, clear_value: T) -> Self {
        Self {
            texture,
            clear_value: Some(clear_value),
        }
    }
}

pub struct RasterPassBuilder<Device: DeviceTrait> {
    color_attachments: Vec<Attachment<Device, [f32; 4]>>,
    depth_stencil_attachment: Option<Attachment<Device, (f32, f32)>>,
    render_function: Option<Box<dyn FnOnce()>>,
}

// pub struct RasterPipeline<Device: DeviceTrait> {
//     vertex_shader: Device::VertexShader,
//     fragment_shader: Option<Device::FragmentShader>,
//     //vertex_elements: Vec<VertexElement>,
//     //pipeline_state: PipelineState,
//     render_function: Option<Box<dyn FnOnce()>>,
// }

pub trait RasterCommandBuffer {
    type Device: DeviceTrait;

    type VertexShader: Sync + Clone;
    type FragmentShader: Sync + Clone;

    type Buffer: Sync + Clone;

    fn bind_pipeline(
        &mut self,
        vertex_shader: Self::VertexShader,
        fragment_shader: Option<Self::FragmentShader>,
        vertex_elements: &[crate::VertexElement],
        pipeline_state: crate::PipelineState,
    );

    fn set_scissor(&mut self, position: [u32; 2], size: [u32; 2]);
    fn set_viewport(&mut self, position: [f32; 2], size: [f32; 2], depth: [f32; 2]);

    fn bind_vertex_buffers(&mut self, vertex_buffers: &[(Self::Buffer, u32)]);
    fn bind_index_buffer(&mut self, index_buffer: (), index_offset: u32, index_size: IndexSize);

    fn bind_resources(&mut self);

    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>);
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>);
}
