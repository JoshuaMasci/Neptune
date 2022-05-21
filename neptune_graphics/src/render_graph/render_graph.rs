use crate::render_graph::render_pass::{ColorAttachment, DepthStencilAttachment};
use crate::render_graph::{
    BufferAccess, BufferId, ImportedBuffer, ImportedTexture, PassId, RasterFn, RasterPassBuilder,
    ResourceAccess, ResourceAccessType, TextureAccess, TextureId,
};
use crate::{MemoryType, TextureDimensions, TextureFormat};

pub(crate) enum BufferResourceDescription {
    New {
        size: usize,
        memory_type: MemoryType,
    },
    Imported(ImportedBuffer),
}

pub(crate) enum TextureResourceDescription {
    New {
        format: TextureFormat,
        size: TextureDimensions,
        memory_type: MemoryType,
    },
    Imported(ImportedTexture),
}

pub(crate) struct BufferResource {
    pub(crate) id: BufferId,
    pub(crate) description: BufferResourceDescription,
    pub(crate) access_list: Vec<ResourceAccessType<BufferAccess>>,
}

pub(crate) struct TextureResource {
    pub(crate) id: TextureId,
    pub(crate) description: TextureResourceDescription,
    pub(crate) access_list: Vec<ResourceAccessType<TextureAccess>>,
}

pub enum RenderPassData {
    Raster {
        color_attachments: Vec<ColorAttachment>,
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        raster_fn: Option<Box<RasterFn>>,
    },
}

pub(crate) struct RenderPass {
    pub(crate) id: PassId,
    pub(crate) name: String,
    pub(crate) data: RenderPassData,
    pub(crate) buffer_accesses: Vec<(BufferId, BufferAccess)>,
    pub(crate) texture_accesses: Vec<(TextureId, TextureAccess)>,
}

pub struct RenderGraphBuilder {
    pub(crate) passes: Vec<RenderPass>,
    pub(crate) buffers: Vec<BufferResource>,
    pub(crate) textures: Vec<TextureResource>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            passes: vec![],
            buffers: vec![],
            textures: vec![],
        }
    }

    pub fn create_buffer(&mut self, size: usize, memory_type: MemoryType) -> BufferId {
        let new_id = self.buffers.len();
        self.buffers.push(BufferResource {
            id: new_id,
            description: BufferResourceDescription::New { size, memory_type },
            access_list: vec![],
        });
        new_id
    }

    pub fn create_texture(
        &mut self,
        format: TextureFormat,
        size: TextureDimensions,
        memory_type: MemoryType,
    ) -> TextureId {
        let new_id = self.textures.len();
        self.textures.push(TextureResource {
            id: new_id,
            description: TextureResourceDescription::New {
                format,
                size,
                memory_type,
            },
            access_list: vec![],
        });
        new_id
    }

    fn add_render_pass(
        &mut self,
        name: String,
        data: RenderPassData,
        buffer_accesses: Vec<(BufferId, BufferAccess)>,
        texture_accesses: Vec<(TextureId, TextureAccess)>,
    ) {
        let id = self.passes.len();

        for (buffer_id, access) in buffer_accesses.iter() {
            let resource_access = ResourceAccess {
                pass_id: id,
                access: *access,
            };

            let buffer_resource = &mut self.buffers[*buffer_id];
            if access.is_write() {
                buffer_resource
                    .access_list
                    .push(ResourceAccessType::Write(resource_access))
            } else {
                if buffer_resource.access_list.is_empty() {
                    buffer_resource
                        .access_list
                        .push(ResourceAccessType::Reads(vec![]));
                }
                let last_index = buffer_resource.access_list.len() - 1;
                if let ResourceAccessType::Reads(list) =
                    &mut buffer_resource.access_list[last_index]
                {
                    list.push(resource_access);
                } else {
                    buffer_resource
                        .access_list
                        .push(ResourceAccessType::Reads(vec![resource_access]));
                }
            }
        }

        for (texture_id, access) in texture_accesses.iter() {
            let resource_access = ResourceAccess {
                pass_id: id,
                access: *access,
            };

            let texture_resource = &mut self.textures[*texture_id];
            if access.is_write() {
                texture_resource
                    .access_list
                    .push(ResourceAccessType::Write(ResourceAccess {
                        pass_id: id,
                        access: *access,
                    }))
            } else {
                if texture_resource.access_list.is_empty() {
                    texture_resource
                        .access_list
                        .push(ResourceAccessType::Reads(vec![]));
                }
                let last_index = texture_resource.access_list.len() - 1;
                if let ResourceAccessType::Reads(list) =
                    &mut texture_resource.access_list[last_index]
                {
                    list.push(resource_access);
                } else {
                    texture_resource
                        .access_list
                        .push(ResourceAccessType::Reads(vec![resource_access]));
                }
            }
        }

        self.passes.push(RenderPass {
            id,
            name,
            data,
            buffer_accesses,
            texture_accesses,
        });
    }

    pub fn add_raster_pass(&mut self, mut pass_builder: RasterPassBuilder) {
        let name = pass_builder.name;
        let data = RenderPassData::Raster {
            color_attachments: pass_builder.color_attachments.clone(),
            depth_stencil_attachment: pass_builder.depth_stencil_attachment.clone(),
            raster_fn: pass_builder.raster_fn.take(),
        };

        let mut buffer_accesses: Vec<(BufferId, BufferAccess)> = pass_builder
            .shader_reads
            .0
            .iter()
            .map(|&buffer| (buffer, BufferAccess::ShaderRead))
            .collect();

        buffer_accesses.append(
            &mut pass_builder
                .vertex_buffers
                .iter()
                .map(|&buffer| (buffer, BufferAccess::VertexBufferRead))
                .collect(),
        );

        buffer_accesses.append(
            &mut pass_builder
                .index_buffers
                .iter()
                .map(|&buffer| (buffer, BufferAccess::IndexBufferRead))
                .collect(),
        );

        let mut texture_access: Vec<(TextureId, TextureAccess)> = pass_builder
            .shader_reads
            .1
            .iter()
            .map(|&texture| (texture, TextureAccess::ShaderSampledRead))
            .collect();

        texture_access.append(
            &mut pass_builder
                .color_attachments
                .iter()
                .map(|attachment| (attachment.id, TextureAccess::ColorAttachmentWrite))
                .collect(),
        );

        if let Some(depth_attachment) = pass_builder.depth_stencil_attachment {
            texture_access.push((
                depth_attachment.id,
                TextureAccess::DepthStencilAttachmentWrite,
            ));
        }

        self.add_render_pass(name, data, buffer_accesses, texture_access);
    }
}
