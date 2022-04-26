use crate::resource::Resource;
use crate::{BufferDescription, TextureDescription};
use std::rc::Rc;

//TODO: Create Abstraction Types
type Buffer = Resource<crate::vulkan::Buffer>;
type Texture = Resource<crate::vulkan::Texture>;
type RenderFnVulkan = dyn FnOnce();

#[derive(Copy, Clone, Debug)]
pub struct BufferHandle(u32);

impl BufferHandle {
    pub(crate) fn new(i: u32) -> Self {
        Self(i)
    }
    pub(crate) fn get(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TextureHandle(u32);

impl TextureHandle {
    pub(crate) fn new(i: u32) -> Self {
        Self(i)
    }
    pub(crate) fn get(&self) -> u32 {
        self.0
    }
}

pub type RenderFn = RenderFnVulkan;

//TODO: RayTracing Accesses
//TODO: make flags?
#[derive(Copy, Clone, Debug)]
pub enum BufferAccessType {
    None,

    IndexBuffer,
    VertexBuffer,

    TransferRead,
    TransferWrite,

    ShaderRead,
    ShaderWrite,
}

impl BufferAccessType {
    pub fn is_write(&self) -> bool {
        match self {
            BufferAccessType::TransferWrite | BufferAccessType::ShaderWrite => true,
            _ => false,
        }
    }
}

//TODO: RayTracing Accesses
#[derive(Copy, Clone, Debug)]
pub enum TextureAccessType {
    None,

    TransferRead,
    TransferWrite,

    ColorAttachmentWrite,
    DepthStencilAttachmentWrite,

    ShaderSampledRead,
    ShaderStorageRead,
    ShaderStorageWrite,
    //TODO: Input Attachments
    //ColorAttachmentRead,
    //DepthStencilAttachmentRead,
}

pub(crate) enum BufferResourceDescription {
    New(BufferDescription),
    Import(Rc<Buffer>, BufferAccessType),
}

pub struct BufferResource {
    pub(crate) description: BufferResourceDescription,
    pub(crate) accesses: Vec<(usize, BufferAccessType)>,
}

pub enum TextureResourceDescription {
    New(TextureDescription),
    Import(Rc<Texture>, TextureAccessType),
    Swapchain(),
}

pub struct TextureResource {
    pub(crate) description: TextureResourceDescription,
    pub(crate) accesses: Vec<(usize, TextureAccessType)>,
}

pub(crate) struct BufferResourceAccess {
    pub(crate) handle: BufferHandle,
    pub(crate) access_type: BufferAccessType,
}

pub(crate) struct TextureResourceAccess {
    pub(crate) handle: TextureHandle,
    pub(crate) access_type: TextureAccessType,
}

#[derive(Debug, Copy, Clone)]
pub enum AttachmentLoadOp {
    Load,
    Clear([f32; 4]),
}

#[derive(Debug)]
pub struct FramebufferDescription {
    pub(crate) color_attachments: Vec<(TextureHandle, AttachmentLoadOp)>,
    pub(crate) depth_attachment: Option<(TextureHandle, AttachmentLoadOp)>,
}

#[derive(Default)]
pub struct RenderPass {
    pub(crate) name: String,
    pub(crate) buffers_dependencies: Vec<BufferResourceAccess>,
    pub(crate) texture_dependencies: Vec<TextureResourceAccess>,
    pub(crate) framebuffer: Option<FramebufferDescription>,
    pub(crate) render_fn: Option<Box<RenderFn>>,
}

pub struct RenderPassBuilder {
    description: RenderPass,
}

impl RenderPassBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            description: RenderPass {
                name: String::from(name),
                buffers_dependencies: vec![],
                texture_dependencies: vec![],
                framebuffer: None,
                render_fn: None,
            },
        }
    }

    pub fn buffer(mut self, resource: BufferHandle, access: BufferAccessType) -> Self {
        self.description
            .buffers_dependencies
            .push(BufferResourceAccess {
                handle: resource,
                access_type: access,
            });
        self
    }

    pub fn texture(mut self, resource: TextureHandle, access: TextureAccessType) -> Self {
        self.description
            .texture_dependencies
            .push(TextureResourceAccess {
                handle: resource,
                access_type: access,
            });
        self
    }

    pub fn raster(
        mut self,
        color_attachments: &[(TextureHandle, AttachmentLoadOp)],
        depth_attachment: Option<(TextureHandle, AttachmentLoadOp)>,
    ) -> Self {
        if color_attachments.is_empty() && depth_attachment.is_none() {
            panic!("Tried to create a framebuffer with no attachments");
        }

        //Setup image transitions
        for (handle, _) in color_attachments.iter() {
            self = self.texture(*handle, TextureAccessType::ColorAttachmentWrite);
        }

        if let Some((handle, _)) = &depth_attachment {
            self = self.texture(*handle, TextureAccessType::DepthStencilAttachmentWrite);
        }

        self.description.framebuffer = Some(FramebufferDescription {
            color_attachments: color_attachments.to_vec(),
            depth_attachment,
        });
        self
    }

    pub fn render(mut self, render: impl FnOnce() + 'static) -> Self {
        assert!(
            self.description
                .render_fn
                .replace(Box::new(render))
                .is_none(),
            "Already set render function"
        );
        self
    }
}

#[derive(Default)]
pub struct RenderGraph {
    pub(crate) passes: Vec<RenderPass>,
    pub(crate) buffers: Vec<BufferResource>,
    pub(crate) textures: Vec<TextureResource>,
}

impl RenderGraph {
    pub fn create_buffer(&mut self, buffer_description: BufferDescription) -> BufferHandle {
        let new_handle = BufferHandle(self.buffers.len() as u32);
        self.buffers.push(BufferResource {
            description: BufferResourceDescription::New(buffer_description),
            accesses: vec![],
        });
        new_handle
    }

    pub fn import_buffer(
        &mut self,
        buffer: Rc<Buffer>,
        last_access: BufferAccessType,
    ) -> BufferHandle {
        let new_handle = BufferHandle(self.buffers.len() as u32);
        self.buffers.push(BufferResource {
            description: BufferResourceDescription::Import(buffer, last_access),
            accesses: vec![],
        });
        new_handle
    }

    pub fn create_texture(&mut self, texture_description: TextureDescription) -> TextureHandle {
        let new_handle = TextureHandle(self.textures.len() as u32);
        self.textures.push(TextureResource {
            description: TextureResourceDescription::New(texture_description),
            accesses: vec![],
        });
        new_handle
    }

    pub fn import_texture(
        &mut self,
        texture: Rc<Texture>,
        last_access: TextureAccessType,
    ) -> TextureHandle {
        let new_handle = TextureHandle(self.textures.len() as u32);
        self.textures.push(TextureResource {
            description: TextureResourceDescription::Import(texture, last_access),
            accesses: vec![],
        });
        new_handle
    }

    pub fn add_render_pass(&mut self, mut render_pass_builder: RenderPassBuilder) {
        let render_pass = std::mem::take(&mut render_pass_builder.description);

        let pass_index = self.passes.len();

        for buffer_accesses in render_pass.buffers_dependencies.iter() {
            self.buffers[buffer_accesses.handle.0 as usize]
                .accesses
                .push((pass_index, buffer_accesses.access_type));
        }

        for texture_accesses in render_pass.texture_dependencies.iter() {
            self.textures[texture_accesses.handle.0 as usize]
                .accesses
                .push((pass_index, texture_accesses.access_type));
        }

        self.passes.push(render_pass);
    }
}
