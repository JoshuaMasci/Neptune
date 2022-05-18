use crate::render_graph::render_pass::{ColorAttachment, DepthStencilAttachment};
use crate::render_graph::{
    BufferAccess, BufferId, ImportedBuffer, ImportedTexture, PassId, RasterFn, RasterPassBuilder,
    ResourceAccess, ResourceAccessType, TextureAccess, TextureId,
};
use crate::{MemoryType, TextureDimensions, TextureFormat};

pub enum RenderPassData {
    Import,
    Raster {
        color_attachments: Vec<ColorAttachment>,
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        raster_fn: Option<Box<RasterFn>>,
    },
}

pub(crate) struct RenderPass {
    id: PassId,
    name: String,
    data: RenderPassData,
    buffer_accesses: Vec<(BufferId, BufferAccess)>,
    texture_accesses: Vec<(TextureId, TextureAccess)>,
}

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

pub struct BufferResource {
    description: BufferResourceDescription,
    access_list: Vec<ResourceAccessType<BufferAccess>>,
}
pub struct TextureResource {
    description: TextureResourceDescription,
    access_list: Vec<ResourceAccessType<TextureAccess>>,
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
        let new_handle = self.buffers.len();
        self.buffers.push(BufferResource {
            description: BufferResourceDescription::New { size, memory_type },
            access_list: vec![],
        });
        new_handle
    }

    pub fn create_texture(
        &mut self,
        format: TextureFormat,
        size: TextureDimensions,
        memory_type: MemoryType,
    ) -> TextureId {
        let new_handle = self.textures.len();
        self.textures.push(TextureResource {
            description: TextureResourceDescription::New {
                format,
                size,
                memory_type,
            },
            access_list: vec![],
        });
        new_handle
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
