use crate::buffer::{BufferGraphResource, BufferHandle, BufferResource};
use crate::render_graph::GraphResource;
use crate::sampler::{Sampler, SamplerHandle};
use crate::texture::TextureResource;
use crate::IndexSize;

pub struct RasterCommandBuffer {
    pub(crate) commands: Vec<RasterCommand>,
}

impl RasterCommandBuffer {
    pub(crate) fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn bind_vertex_buffer<T: BufferResource>(&mut self, vertex_buffer: &T, offset: u32) {
        self.commands.push(RasterCommand::BindVertexBuffers(vec![(
            vertex_buffer.get_graph_resource(),
            offset,
        )]));
    }

    pub fn bind_index_buffer<T: BufferResource>(
        &mut self,
        index_buffer: &T,
        offset: u32,
        size: IndexSize,
    ) {
        self.commands.push(RasterCommand::BindIndexBuffer {
            index_buffer: index_buffer.get_graph_resource(),
            offset,
            size,
        });
    }

    pub fn bind_storage_buffer<T: BufferResource>(&mut self, slot: u32, buffer: &T, write: bool) {
        self.commands.push(RasterCommand::BindResource {
            slot,
            resource: GraphResource::StorageBuffer {
                buffer: buffer.get_graph_resource(),
                write,
            },
        });
    }
    pub fn bind_storage_texture<T: TextureResource>(
        &mut self,
        slot: u32,
        texture: &T,
        write: bool,
    ) {
        self.commands.push(RasterCommand::BindResource {
            slot,
            resource: GraphResource::StorageTexture {
                texture: texture.get_graph_resource(),
                write,
            },
        });
    }
    pub fn bind_sampler(&mut self, slot: u32, sampler: &Sampler) {
        self.commands.push(RasterCommand::BindResource {
            slot,
            resource: GraphResource::Sampler(sampler.get_handle()),
        });
    }
    pub fn bind_sampled_texture<T: TextureResource>(&mut self, slot: u32, texture: &T) {
        self.commands.push(RasterCommand::BindResource {
            slot,
            resource: GraphResource::SampledTexture(texture.get_graph_resource()),
        });
    }

    pub fn draw(
        &mut self,
        vertex_range: std::ops::Range<u32>,
        instance_range: std::ops::Range<u32>,
    ) {
        self.commands.push(RasterCommand::Draw {
            vertex_range,
            instance_range,
        });
    }

    pub fn draw_indexed(
        &mut self,
        index_range: std::ops::Range<u32>,
        base_vertex: i32,
        instance_range: std::ops::Range<u32>,
    ) {
        self.commands.push(RasterCommand::DrawIndexed {
            index_range,
            base_vertex,
            instance_range,
        });
    }
}

pub enum RasterCommand {
    BindVertexBuffers(Vec<(BufferGraphResource, u32)>),
    BindIndexBuffer {
        index_buffer: BufferGraphResource,
        offset: u32,
        size: IndexSize,
    },

    //TODO: PushConstants(VK) / RootConstants?(DX12)
    // Research needed to see if an api can be created to abstract PushConstants, RootConstants and (maybe?) Metal Push Constants.
    // The Api will probably need to allocate some data per stage (vertex, fragment, etc..), in might have to use dynamic uniform buffers instead
    // Will replace that once that api is created
    BindResource {
        slot: u32,
        resource: GraphResource,
    },

    Draw {
        vertex_range: std::ops::Range<u32>,
        instance_range: std::ops::Range<u32>,
    },
    DrawIndexed {
        index_range: std::ops::Range<u32>,
        base_vertex: i32,
        instance_range: std::ops::Range<u32>,
    },
    //TODO: Indirect draw
}
