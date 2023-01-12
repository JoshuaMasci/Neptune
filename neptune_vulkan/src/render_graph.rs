use crate::resource_manager::{BufferHandle, ComputePipelineHandle, SamplerHandle, TextureHandle};
use crate::{BufferUsage, Sampler, Swapchain, SwapchainHandle, Texture, TextureUsage};
use ash::vk;
use bitflags::bitflags;
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct BufferDescription {
    name: String,
    size: u64,
}

#[derive(Debug, Clone)]
pub enum TextureSize {
    Absolute([u32; 2]),
    Relative(TextureResource, [f32; 2]),
}

#[derive(Debug, Clone)]
pub struct TextureDescription {
    name: String,
    format: vk::Format,
    size: TextureSize,
    sampler: Option<SamplerHandle>,
}

#[derive(Debug, Copy, Clone, Hash)]
pub enum Queue {
    Primary,
    AsyncCompute,
    AsyncTransfer,
}

#[derive(Debug, Copy, Clone)]
pub struct BufferResource(usize);

#[derive(Debug, Copy, Clone)]
pub struct TextureResource(usize);

//TODO: Store resource Arcs rather than handles?
#[derive(Debug)]
pub enum BufferType {
    Transient(BufferDescription),
    Import(BufferHandle),
}

//TODO: Store resource Arcs rather than handles?
#[derive(Debug)]
pub enum TextureType {
    Transient(TextureDescription),
    Import(TextureHandle),
    Swapchain(SwapchainHandle),
}

bitflags! {
    pub struct BufferAccess: u32 {
        const INDEX_BUFFER_READ = 1 << 0;
        const VERTEX_BUFFER_READ = 1 << 1;
        const TRANSFER_READ = 1 << 2;
        const TRANSFER_WRITE = 1 << 3;
        const UNIFORM_READ = 1 << 4;
        const STORAGE_READ = 1 << 5;
        const STORAGE_WRITE = 1 << 6;
    }
}

bitflags! {
    pub struct TextureAccess: u32 {
        const  ATTACHMENT_WRITE  = 1 << 0;
        const  TRANSFER_READ  = 1 << 1;
        const  TRANSFER_WRITE  = 1 << 2;
        const  SAMPLED_READ  = 1 << 3;
        const  STORAGE_READ  = 1 << 4;
        const  STORAGE_WRITE  = 1 << 5;
    }
}

#[derive(Debug, Clone)]
pub struct ColorAttachment {
    texture: TextureResource,
    clear: Option<[f32; 4]>,
}

impl ColorAttachment {
    pub fn new(texture: TextureResource) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: TextureResource, clear: [f32; 4]) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DepthStencilAttachment {
    texture: TextureResource,
    clear: Option<(f32, u32)>,
}

impl DepthStencilAttachment {
    pub fn new(texture: TextureResource) -> Self {
        Self {
            texture,
            clear: None,
        }
    }

    pub fn new_clear(texture: TextureResource, clear: (f32, u32)) -> Self {
        Self {
            texture,
            clear: Some(clear),
        }
    }
}

pub enum IndexSize {
    Int16,
    Int32,
}

pub enum RenderPass {
    Transfer {
        //TODO: this
    },
    Compute {
        pipeline: ComputePipelineHandle,
        dispatch: [u32; 3],
    },
    Raster {
        input_attachments: Vec<TextureResource>,
        color_attachments: Vec<ColorAttachment>,
        depth_stencil_attachment: Option<DepthStencilAttachment>,
    },
}

pub struct RenderPassDescription {
    name: String,
    queue: Queue,
    pass: RenderPass,
}

#[derive(Default)]
pub struct RenderGraphBuilder {
    swapchain_textures: Vec<TextureResource>,
    texture_resources: Vec<TextureType>,
    passes: Vec<RenderPassDescription>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn import_texture(&mut self, texture: &Texture) -> TextureResource {
        let resource = TextureResource(self.texture_resources.len());
        self.texture_resources
            .push(TextureType::Import(texture.handle));
        resource
    }

    pub fn create_texture(
        &mut self,
        name: &str,
        format: vk::Format,
        size: TextureSize,
        sampler: Option<&Sampler>,
    ) -> TextureResource {
        let resource = TextureResource(self.texture_resources.len());
        self.texture_resources
            .push(TextureType::Transient(TextureDescription {
                name: name.to_string(),
                format,
                size,
                sampler: sampler.map(|sampler| sampler.handle),
            }));
        resource
    }

    pub fn acquire_swapchain_texture(&mut self, swapchain: &Swapchain) -> TextureResource {
        let resource = TextureResource(self.texture_resources.len());
        self.texture_resources
            .push(TextureType::Swapchain(swapchain.0.handle));
        self.swapchain_textures.push(resource);
        resource
    }

    pub fn add_raster_pass(
        &mut self,
        name: &str,
        color_attachments: &[ColorAttachment],
        depth_stencil_attachment: Option<DepthStencilAttachment>,
    ) {
        self.passes.push(RenderPassDescription {
            name: name.to_string(),
            queue: Queue::Primary,
            pass: RenderPass::Raster {
                input_attachments: Vec::new(),
                color_attachments: color_attachments.to_vec(),
                depth_stencil_attachment,
            },
        });
    }
}

use crate::AshDevice;
use std::sync::Arc;

/// A limited render graph executor that is quick to implement.
/// Planned to not have the following features
/// 1. Async queues
/// 2. Render Pass Reordering
/// 3. Optimal Pipeline Barriers
/// 4. Multiple frames in flight
/// 5. Multithreading command buffer recording
/// A More complete render graph executor will be built once the api is proven in
pub(crate) struct BasicLinearRenderGraphExecutor {
    device: Arc<AshDevice>,
    swapchain_ext: Arc<ash::extensions::khr::Swapchain>,

    graphics_queue: (vk::Queue, u32),

    graphics_command_pool: vk::CommandPool,

    graphics_command_buffer: vk::CommandBuffer,
    image_ready_semaphore: vk::Semaphore,
    frame_done_semaphore: vk::Semaphore,
    frame_done_fence: vk::Fence,
}

impl BasicLinearRenderGraphExecutor {
    pub(crate) fn new(
        device: Arc<AshDevice>,
        swapchain_ext: Arc<ash::extensions::khr::Swapchain>,
        graphics_queue: (vk::Queue, u32),
    ) -> Self {
        let graphics_command_pool = unsafe {
            device
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::builder()
                        .queue_family_index(graphics_queue.1)
                        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                    None,
                )
                .unwrap()
        };

        let graphics_command_buffer = unsafe {
            device
                .allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::builder()
                        .command_pool(graphics_command_pool)
                        .command_buffer_count(1),
                )
                .unwrap()[0]
        };

        let image_ready_semaphore = unsafe {
            device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap()
        };

        let frame_done_semaphore = unsafe {
            device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap()
        };

        let frame_done_fence = unsafe {
            device
                .create_fence(
                    &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
                .unwrap()
        };

        Self {
            device,
            swapchain_ext,
            graphics_queue,
            graphics_command_pool,
            graphics_command_buffer,
            image_ready_semaphore,
            frame_done_semaphore,
            frame_done_fence,
        }
    }

    pub(crate) fn execute_graph(&mut self, render_graph_builder: RenderGraphBuilder) {
        const TIMEOUT: u64 = std::time::Duration::from_secs(2).as_nanos() as u64;

        let _ = render_graph_builder;

        unsafe {
            self.device
                .wait_for_fences(&[self.frame_done_fence], true, TIMEOUT)
                .unwrap();
            self.device.reset_fences(&[self.frame_done_fence]).unwrap();

            //TODO: get actual swapchain
            let swapchain_images: Vec<&SwapchainHandle> = render_graph_builder
                .swapchain_textures
                .iter()
                .map(
                    |index| match &render_graph_builder.texture_resources[index.0] {
                        TextureType::Swapchain(swapchain) => swapchain,
                        _ => unreachable!("TextureType must be swapchain"),
                    },
                )
                .collect();

            self.device
                .begin_command_buffer(
                    self.graphics_command_buffer,
                    &vk::CommandBufferBeginInfo::builder(),
                )
                .unwrap();

            self.device
                .end_command_buffer(self.graphics_command_buffer)
                .unwrap();

            // let wait_semaphore_infos = &[vk::SemaphoreSubmitInfoKHR::builder()
            //     .semaphore(self.image_ready_semaphore)
            //     .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
            //     .build()];

            let command_buffer_infos = &[vk::CommandBufferSubmitInfoKHR::builder()
                .command_buffer(self.graphics_command_buffer)
                .build()];

            // let signal_semaphore_infos = &[vk::SemaphoreSubmitInfoKHR::builder()
            //     .semaphore(self.frame_done_semaphore)
            //     .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
            //     .build()];

            self.device
                .queue_submit2(
                    self.graphics_queue.0,
                    &[vk::SubmitInfo2::builder()
                        //.wait_semaphore_infos(wait_semaphore_infos)
                        .command_buffer_infos(command_buffer_infos)
                        //.signal_semaphore_infos(signal_semaphore_infos)
                        .build()],
                    self.frame_done_fence,
                )
                .unwrap();
        }
    }
}

impl Drop for BasicLinearRenderGraphExecutor {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device
                .destroy_command_pool(self.graphics_command_pool, None);
            self.device
                .destroy_semaphore(self.image_ready_semaphore, None);
            self.device
                .destroy_semaphore(self.frame_done_semaphore, None);
            self.device.destroy_fence(self.frame_done_fence, None);
        }
    }
}
