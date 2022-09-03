use crate::device::{
    Buffer, ComputeShader, Texture, TextureCreateInfo, TextureFormat, TextureUsage,
};
use crate::handle::Handle;
use crate::{BufferUsage, DeviceTrait, IndexSize, PipelineState};
use std::ops::Range;

pub trait RenderGraphBuilderTrait {
    fn add_compute_pass(&mut self, compute_pass: ComputePass);
    fn add_raster_pass(&mut self, raster_pass: RasterPass);
}

//     type Device: DeviceTrait;
//     type Buffer: Sync + Clone;
//     type Texture: Sync + Clone;
//     type Sampler: Sync + Clone;
//
//     fn add_compute_pass(builder: ComputePassBuilder<Self::Device>);
//     fn add_raster_pass(builder: RasterPassBuilder<Self::Device>);
// }

pub enum ResourceUsage<'a> {
    BufferRead(&'a Buffer),
    BufferWrite(&'a Buffer),
    TextureSample(&'a Texture),
    TextureSampler(&'a Texture),
    TextureStorageWrite(&'a Texture),
    TextureStorageRead(&'a Texture),
}

pub enum LoadOp<T> {
    None,
    Clear(T),
}

pub struct Attachment<'a, T: Clone> {
    texture: &'a Texture,
    clear_value: LoadOp<T>,
}

impl<'a, T: Clone> Attachment<'a, T> {
    pub fn new(texture: &'a Texture) -> Self {
        Self {
            texture,
            clear_value: LoadOp::None,
        }
    }

    pub fn new_with_clear(texture: &'a Texture, clear_value: &T) -> Self {
        Self {
            texture,
            clear_value: LoadOp::Clear(clear_value.clone()),
        }
    }
}

// struct RasterCommandBuffer {}
//
// impl RasterCommandBuffer {
//     fn bind_vertex_buffers(&mut self, buffers_offset: &[(&Buffer, u32)]) {
//         todo!()
//     }
//
//     fn bind_index_buffer(&mut self, buffer: &Buffer, offset: u32, size: IndexSize) {
//         todo!()
//     }
//
//     fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
//         todo!()
//     }
//
//     fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
//         todo!()
//     }
// }

pub struct ComputePass<'a> {
    name: &'a str,
    compute_shader: &'a ComputeShader,
    dispatch_size: [u32; 3],
    resources: Vec<ResourceUsage<'a>>,
}

impl<'a> ComputePass<'a> {
    pub fn new(name: &'a str, compute_shader: &'a ComputeShader, dispatch_size: &[u32]) -> Self {
        assert!(
            dispatch_size.len() <= 3,
            "dispatch_size should be at max 3 elements"
        );

        let dispatch_size = [
            dispatch_size.get(0).cloned().unwrap_or(1),
            dispatch_size.get(1).cloned().unwrap_or(1),
            dispatch_size.get(2).cloned().unwrap_or(1),
        ];

        Self {
            name,
            compute_shader,
            dispatch_size,
            resources: Vec::new(),
        }
    }

    pub fn buffer_read(mut self, buffer: &'a Buffer) -> Self {
        self.resources.push(ResourceUsage::BufferRead(buffer));
        self
    }

    pub fn buffer_write(mut self, buffer: &'a Buffer) -> Self {
        self.resources.push(ResourceUsage::BufferWrite(buffer));
        self
    }

    pub fn texture_sampled_read(mut self, texture: &'a Texture) -> Self {
        self.resources.push(ResourceUsage::TextureSample(texture));
        self
    }

    pub fn texture_read(mut self, texture: &'a Texture) -> Self {
        self.resources
            .push(ResourceUsage::TextureStorageRead(texture));
        self
    }

    pub fn texture_write(mut self, texture: &'a Texture) -> Self {
        self.resources
            .push(ResourceUsage::TextureStorageWrite(texture));
        self
    }
}

pub struct RasterPass<'a> {
    name: String,
    color_attachments: Vec<Attachment<'a, [f32; 4]>>,
    depth_stencil_attachment: Option<Attachment<'a, [f32; 2]>>,
}

impl<'a> RasterPass<'a> {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            color_attachments: Vec::new(),
            depth_stencil_attachment: None,
        }
    }

    fn color_attachment(mut self, attachment: Attachment<'a, [f32; 4]>) -> Self {
        self.color_attachments.push(attachment);
        self
    }

    fn depth_stencil_attachment(mut self, attachment: Attachment<'a, [f32; 2]>) -> Self {
        let _ = self.depth_stencil_attachment.insert(attachment);
        self
    }

    // fn pipeline(
    //     mut self,
    //     vertex_shader: &Device::VertexShader,
    //     fragment_shader: Option<&Device::FragmentShader>,
    //     pipeline_state: &PipelineState,
    //     vertex_layout: (),
    //     raster_fn: impl FnOnce(&mut RasterCommandBuffer<Device>),
    // ) -> Self {
    //     raster_fn(&mut RasterCommandBuffer {
    //         used_buffers: vec![],
    //     });
    //     self
    // }
}

fn test_function<Device: DeviceTrait>(
    device: &mut Device,
    render_pass_builder: &mut dyn RenderGraphBuilderTrait,
) {
    let size = [1920, 1080];

    let color_attachment = device
        .create_texture(&TextureCreateInfo {
            format: TextureFormat::Some,
            size,
            usage: TextureUsage::RENDER_ATTACHMENT,
            mip_levels: 1,
            sample_count: 1,
        })
        .unwrap();

    let depth_stencil_attachment = device
        .create_texture(&TextureCreateInfo {
            format: TextureFormat::Other,
            size,
            usage: TextureUsage::RENDER_ATTACHMENT,
            mip_levels: 1,
            sample_count: 1,
        })
        .unwrap();

    let compute_data_buffer = device
        .create_static_buffer(BufferUsage::STORAGE, &[0, 1, 2, 3])
        .unwrap();

    let compute_shader = ComputeShader(Handle::new_temp(0));

    let vertex_buffer = device.create_static_buffer(BufferUsage::VERTEX, &[]);
    let index_buffer = device.create_static_buffer(BufferUsage::VERTEX, &[]);

    render_pass_builder.add_raster_pass(
        RasterPass::new("RenderPass")
            .color_attachment(Attachment::new_with_clear(&color_attachment, &[0.0f32; 4]))
            .depth_stencil_attachment(Attachment::new_with_clear(
                &depth_stencil_attachment,
                &[1.0, 0.0],
            )),
    );

    render_pass_builder.add_compute_pass(
        ComputePass::new("Bad Post Process Pass", &compute_shader, &size)
            .buffer_read(&compute_data_buffer)
            .texture_write(&color_attachment)
            .texture_read(&depth_stencil_attachment),
    );
}
