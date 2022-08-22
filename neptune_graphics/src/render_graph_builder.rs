use crate::device::{TextureCreateInfo, TextureFormat, TextureUsage};
use crate::{DeviceTrait, IndexSize, PipelineState, RenderGraphBuilder};
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

    vertex_buffers: Vec<Device::Buffer>,
    index_buffers: Vec<Device::Buffer>,
    resources: Vec<ResourceUsage<Device>>,
}

// pub struct RasterPipeline<Device: DeviceTrait> {
//     vertex_shader: Device::VertexShader,
//     fragment_shader: Option<Device::FragmentShader>,
//     //vertex_elements: Vec<VertexElement>,
//     //pipeline_state: PipelineState,
//     render_function: Option<Box<dyn FnOnce()>>,
// }

// pub trait RasterCommandBuffer {
//     type Device: DeviceTrait;
//
//     type VertexShader: Sync + Clone;
//     type FragmentShader: Sync + Clone;
//
//     type Buffer: Sync + Clone;
//
//     fn bind_pipeline(
//         &mut self,
//         vertex_shader: Self::VertexShader,
//         fragment_shader: Option<Self::FragmentShader>,
//         vertex_elements: &[crate::VertexElement],
//         pipeline_state: crate::PipelineState,
//     );
//
//     fn set_scissor(&mut self, position: [u32; 2], size: [u32; 2]);
//     fn set_viewport(&mut self, position: [f32; 2], size: [f32; 2], depth: [f32; 2]);
//
//     fn bind_vertex_buffers(&mut self, vertex_buffers: &[(Self::Buffer, u32)]);
//     fn bind_index_buffer(&mut self, index_buffer: (), index_offset: u32, index_size: IndexSize);
//
//     fn bind_resources(&mut self);
//
//     fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>);
//     fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>);
// }

struct RasterCommandBuffer<Device: DeviceTrait> {
    used_buffers: Vec<Device::Buffer>,
}

impl<Device: DeviceTrait> RasterCommandBuffer<Device> {
    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        todo!()
    }
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        todo!()
    }
}

enum LoadOp<T> {
    None,
    Clear(T),
}

struct RasterPass<'a, Device: DeviceTrait> {
    name: String,
    color_attachments: Vec<(Device::Texture, LoadOp<[f32; 4]>)>,
    depth_stencil_attachment: Option<(Device::Texture, LoadOp<(f32, f32)>)>,

    render_graph_builder: &'a mut RenderGraphBuilderImpl<Device>,
}

impl<'a, Device: DeviceTrait> RasterPass<'a, Device> {
    fn new(name: &str, render_graph_builder: &'a mut RenderGraphBuilderImpl<Device>) -> Self {
        Self {
            name: name.to_string(),
            color_attachments: Vec::new(),
            depth_stencil_attachment: None,
            render_graph_builder,
        }
    }

    fn color_attachment(mut self, attachment: Device::Texture, load_op: LoadOp<[f32; 4]>) -> Self {
        self.color_attachments.push((attachment, load_op));
        self
    }

    fn depth_stencil_attachment(
        mut self,
        attachment: Device::Texture,
        load_op: LoadOp<(f32, f32)>,
    ) -> Self {
        let _ = self.depth_stencil_attachment.insert((attachment, load_op));
        self
    }

    fn pipeline(
        mut self,
        vertex_shader: Device::VertexShader,
        fragment_shader: Option<Device::FragmentShader>,
        pipeline_state: &PipelineState,
        vertex_layout: (),
        raster_fn: impl FnOnce(&mut RasterCommandBuffer<Device>),
    ) -> Self {
        raster_fn(&mut RasterCommandBuffer {
            used_buffers: vec![],
        });
        self
    }
}

impl<Device: DeviceTrait> Drop for RasterPassBuilder<Device> {
    fn drop(&mut self) {
        todo!()
    }
}

fn test_function<Device: DeviceTrait>(
    device: &mut Device,
    render_pass_builder: &mut RenderGraphBuilderImpl<Device>,
) {
    let color_attachment = device
        .create_texture(&TextureCreateInfo {
            format: TextureFormat::Some,
            size: [1920, 1080],
            usage: TextureUsage::RENDER_ATTACHMENT,
            mip_levels: 1,
            sample_count: 1,
        })
        .unwrap();

    let depth_stencil_attachment = device
        .create_texture(&TextureCreateInfo {
            format: TextureFormat::Other,
            size: [1920, 1080],
            usage: TextureUsage::RENDER_ATTACHMENT,
            mip_levels: 1,
            sample_count: 1,
        })
        .unwrap();

    let vertex_shader1 = device.create_vertex_shader(&[]).unwrap();
    let vertex_shader2 = device.create_vertex_shader(&[]).unwrap();
    let fragment_shader1 = device.create_fragment_shader(&[]);
    let fragment_shader2 = device.create_fragment_shader(&[]);
    let pipeline_state = PipelineState::default();

    RasterPass::new("Test Pass", render_pass_builder)
        .color_attachment(color_attachment, LoadOp::Clear([0.0f32; 4]))
        .depth_stencil_attachment(depth_stencil_attachment, LoadOp::Clear((1.0, 0.0)))
        .pipeline(
            vertex_shader1,
            fragment_shader1,
            &pipeline_state,
            (),
            |raster_command_buffer| {
                raster_command_buffer.draw(0..3, 0..1);
            },
        )
        .pipeline(
            vertex_shader2,
            fragment_shader2,
            &pipeline_state,
            (),
            |raster_command_buffer| {
                raster_command_buffer.draw(0..24, 0..1);
            },
        );
}
