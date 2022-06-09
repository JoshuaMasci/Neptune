use crate::pipeline::{PipelineState, VertexElement};
use crate::render_graph::render_pass::{ColorAttachment, DepthStencilAttachment};
use crate::render_graph::{
    BufferAccess, BufferId, ImportedBuffer, ImportedTexture, PassId, RasterFn, RasterPassBuilder,
    ResourceAccess, ResourceAccessType, TextureAccess, TextureId,
};
use crate::vulkan::ShaderModule;
use crate::{BufferDescription, BufferUsages, MemoryType, Resource, TextureDescription};
use std::rc::Rc;

pub(crate) enum BufferResourceDescription {
    New(BufferDescription),
    Imported(ImportedBuffer),
}

impl BufferResourceDescription {
    fn get_size(&self) -> usize {
        match self {
            BufferResourceDescription::New(description) => description.size,
            BufferResourceDescription::Imported(buffer) => buffer.description.size,
        }
    }
}

pub(crate) enum TextureResourceDescription {
    Swapchain(u32),
    New(TextureDescription),
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

pub enum UploadData {
    U8(Vec<u8>),
    U32(Vec<u32>),
    F32(Vec<f32>),
}

impl UploadData {
    pub fn get_size(&self) -> usize {
        match self {
            UploadData::U8(data) => std::mem::size_of::<u8>() * data.len(),
            UploadData::U32(data) => std::mem::size_of::<u32>() * data.len(),
            UploadData::F32(data) => std::mem::size_of::<f32>() * data.len(),
        }
    }
}

pub struct RasterPipeline {
    pub(crate) vertex_module: Rc<ShaderModule>,
    pub(crate) fragment_module: Option<Rc<ShaderModule>>,
    pub(crate) vertex_elements: Vec<VertexElement>,
    pub(crate) pipeline_state: PipelineState,
    pub(crate) raster_fn: Box<RasterFn>,
}

pub enum RenderPassData {
    BufferUpload {
        src_buffer: BufferId,
        src_data: UploadData,
        dst_buffer: BufferId,
        dst_offset: usize,
    },
    TextureUpload {
        src_buffer: BufferId,
        src_data: UploadData,
        dst_texture: TextureId,
    },
    Raster {
        color_attachments: Vec<ColorAttachment>,
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        pipelines: Vec<RasterPipeline>,
    },
}

pub(crate) struct RenderPass {
    pub(crate) id: PassId,

    #[allow(dead_code)]
    pub(crate) name: String,

    pub(crate) data: RenderPassData,
    pub(crate) buffer_accesses: Vec<(BufferId, BufferAccess)>,
    pub(crate) texture_accesses: Vec<(TextureId, TextureAccess)>,
}

pub struct RenderGraphBuilder {
    pub(crate) swapchain_texture: (TextureId, [u32; 2]),
    pub(crate) passes: Vec<RenderPass>,
    pub(crate) buffers: Vec<BufferResource>,
    pub(crate) textures: Vec<TextureResource>,
}

impl RenderGraphBuilder {
    pub fn new(swapchain_size: [u32; 2]) -> Self {
        Self {
            swapchain_texture: (0, swapchain_size),
            passes: vec![],
            buffers: vec![],
            textures: vec![TextureResource {
                id: 0,
                description: TextureResourceDescription::Swapchain(0),
                access_list: vec![],
            }],
        }
    }

    pub fn get_swapchain_image(&self) -> (TextureId, [u32; 2]) {
        self.swapchain_texture
    }

    pub fn create_buffer(&mut self, description: BufferDescription) -> BufferId {
        let new_id = self.buffers.len();
        self.buffers.push(BufferResource {
            id: new_id,
            description: BufferResourceDescription::New(description),
            access_list: vec![],
        });
        new_id
    }

    pub fn create_texture(&mut self, description: TextureDescription) -> TextureId {
        let new_id = self.textures.len();
        self.textures.push(TextureResource {
            id: new_id,
            description: TextureResourceDescription::New(description),
            access_list: vec![],
        });
        new_id
    }

    pub fn import_texture(&mut self, texture: ImportedTexture) -> TextureId {
        let new_id = self.textures.len();
        self.textures.push(TextureResource {
            id: new_id,
            description: TextureResourceDescription::Imported(texture),
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

    pub fn add_buffer_upload_pass(
        &mut self,
        dst_buffer: BufferId,
        dst_offset: usize,
        src_data: UploadData,
    ) {
        let src_buffer = self.create_buffer(BufferDescription {
            size: src_data.get_size(),
            usage: BufferUsages::TRANSFER_SRC,
            memory_type: MemoryType::CpuToGpu,
        });

        self.add_render_pass(
            format!("Buffer Upload Id: {}", dst_buffer),
            RenderPassData::BufferUpload {
                src_buffer,
                src_data,
                dst_buffer,
                dst_offset,
            },
            vec![
                (src_buffer, BufferAccess::TransferRead),
                (dst_buffer, BufferAccess::TransferWrite),
            ],
            Vec::new(),
        );
    }

    pub(crate) fn add_texture_upload_pass(&mut self, dst_texture: TextureId, src_data: UploadData) {
        let src_buffer = self.create_buffer(BufferDescription {
            size: src_data.get_size(),
            usage: BufferUsages::TRANSFER_SRC,
            memory_type: MemoryType::CpuToGpu,
        });

        self.add_render_pass(
            format!("Texture Upload Id: {}", dst_texture),
            RenderPassData::TextureUpload {
                src_buffer,
                src_data,
                dst_texture,
            },
            vec![(src_buffer, BufferAccess::TransferRead)],
            vec![(dst_texture, TextureAccess::TransferWrite)],
        );
    }

    pub fn add_raster_pass(&mut self, pass_builder: RasterPassBuilder) {
        let name = pass_builder.name;

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

        if let Some(depth_attachment) = &pass_builder.depth_stencil_attachment {
            texture_access.push((
                depth_attachment.id,
                TextureAccess::DepthStencilAttachmentWrite,
            ));
        }

        let data = RenderPassData::Raster {
            color_attachments: pass_builder.color_attachments,
            depth_stencil_attachment: pass_builder.depth_stencil_attachment,
            pipelines: pass_builder.pipelines,
        };

        self.add_render_pass(name, data, buffer_accesses, texture_access);
    }
}
