use crate::resource_managers::{
    BufferBarrierFlags, BufferResourceAccess, ImageBarrierFlags, ImageResourceAccess,
};
use crate::{
    BufferDescription, BufferKey, ComputePipelineHandle, ImageKey, RasterPipelineHandle,
    SamplerHandle, SurfaceHandle, TransientImageDesc,
};
use ash::vk;
use std::ops::Range;

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub enum QueueType {
    #[default]
    Graphics,
    PreferAsyncCompute,
    PreferAsyncTransfer,
}

pub type BufferIndex = usize;

#[derive(Debug)]
pub enum BufferResourceDescription {
    Persistent(BufferKey),
    Transient(BufferDescription),
}

impl BufferResourceDescription {
    pub fn is_persistent(&self) -> bool {
        match self {
            BufferResourceDescription::Persistent(_) => true,
            BufferResourceDescription::Transient(_) => false,
        }
    }

    pub fn as_persistent(&self) -> Option<BufferKey> {
        match self {
            BufferResourceDescription::Persistent(key) => Some(*key),
            BufferResourceDescription::Transient(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct BufferGraphResource {
    pub description: BufferResourceDescription,
    pub last_access: BufferResourceAccess,
}

pub type ImageIndex = usize;

#[derive(Debug)]
pub enum ImageResourceDescription {
    Persistent(ImageKey),
    Transient(TransientImageDesc),
    Swapchain(usize),
}

#[derive(Debug)]
pub struct ImageGraphResource {
    pub description: ImageResourceDescription,
    pub first_access: Option<ImageResourceAccess>,
    pub last_access: Option<ImageResourceAccess>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BufferOffset {
    pub buffer: BufferIndex,
    pub offset: u64,
}

#[derive(Debug)]
pub enum ShaderResourceUsage {
    StorageBuffer { buffer: BufferIndex, write: bool },
    StorageImage { image: ImageIndex, write: bool },
    SampledImage(ImageIndex),
    Sampler(SamplerHandle),
}

//Transfer
#[derive(Debug, Eq, PartialEq, Copy, Clone)]

pub struct ImageCopyBuffer {
    pub buffer: BufferIndex,
    pub offset: u64,
    pub row_length: Option<u32>,
    pub row_height: Option<u32>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]

pub struct ImageCopyImage {
    pub image: ImageIndex,
    pub offset: [u32; 2],
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Transfer {
    BufferToBuffer {
        src: BufferOffset,
        dst: BufferOffset,
        copy_size: u64,
    },
    BufferToImage {
        src: ImageCopyBuffer,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    },
    ImageToBuffer {
        src: ImageCopyImage,
        dst: ImageCopyBuffer,
        copy_size: [u32; 2],
    },
    ImageToImage {
        src: ImageCopyImage,
        dst: ImageCopyImage,
        copy_size: [u32; 2],
    },
}

//Compute
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ComputeDispatch {
    Size([u32; 3]),
    Indirect(BufferOffset),
}

//Raster
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum IndexType {
    U16,
    U32,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct ColorAttachment {
    pub image: ImageIndex,
    pub clear: Option<[f32; 4]>,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct DepthStencilAttachment {
    pub image: ImageIndex,
    pub clear: Option<(f32, u32)>,
}

#[derive(Default, Debug)]
pub struct Framebuffer {
    pub color_attachments: Vec<ColorAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum DrawCommandDispatch {
    Draw {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
    DrawIndexed {
        base_vertex: i32,
        indices: Range<u32>,
        instances: Range<u32>,
        index_buffer: BufferOffset,
        index_type: IndexType,
    },
    DrawIndirect {
        indirect_buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
    },
    DrawIndirectIndexed {
        indirect_buffer: BufferOffset,
        draw_count: u32,
        stride: u32,
        index_buffer: BufferOffset,
        index_type: IndexType,
    },
}

#[derive(Debug)]
pub struct RasterDrawCommand {
    pub pipeline: RasterPipelineHandle,
    pub vertex_buffers: Vec<BufferOffset>,
    pub resources: Vec<ShaderResourceUsage>,
    pub dispatch: DrawCommandDispatch,
}

#[derive(Debug)]
pub enum RenderPassCommand {
    Transfer {
        transfers: Vec<Transfer>,
    },
    Compute {
        pipeline: ComputePipelineHandle,
        resources: Vec<ShaderResourceUsage>,
        dispatch: ComputeDispatch,
    },
    Raster {
        framebuffer: Framebuffer,
        draw_commands: Vec<RasterDrawCommand>,
    },
}

#[derive(Debug)]
pub struct RenderPass {
    pub label_name: String,
    pub label_color: [f32; 4],
    pub queue: QueueType,
    pub buffer_access: Vec<(BufferIndex, BufferResourceAccess)>,
    pub image_access: Vec<(ImageIndex, ImageResourceAccess)>,
    pub command: Option<RenderPassCommand>,
}

#[derive(Debug, Default)]
pub struct RenderGraph {
    pub buffer_resources: Vec<BufferGraphResource>,
    pub image_resources: Vec<ImageGraphResource>,
    pub swapchain_images: Vec<(SurfaceHandle, ImageIndex)>,
    pub render_passes: Vec<RenderPass>,
}

// Compiled Graph Struct
// Will be the result of the RenderGraphBuilder, all sync requirements and command buffer lists are precalculate
// Frame executor will only have to resolve resource, sync primitives, command buffer recording, submission, present.

// TODO: Determine the best pre and/or post frame ownership barriers

// Graph Builders:
// 1. Debug = Single Queue + Serial + Image Transitions + Global Memory Barriers
// 2. Basic = Single Queue + Pass Promoting + Image/Buffer Transitions
// 3. Graph = Single Queue + Topological Sort + Image/Buffer Transitions
// 4. GraphMultiQueue = Multiple Queues + Topological Sort + Image/Buffer Transitions

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub enum Queue {
    #[default]
    Graphics,
    Compute,
    Transfer,
}

#[derive(Debug)]
pub struct RenderPass2 {
    pub label_name: String,
    pub label_color: [f32; 4],
    pub command: Option<RenderPassCommand>,
}

#[derive(Debug, Default)]
pub enum BufferBarrierSource {
    #[default]
    /// Retrieve usage from a previous frame
    FirstUsage,

    /// Precalculated usage from the graph
    Precalculated(BufferResourceAccess),
}

#[derive(Debug, Default)]
pub struct BufferBarrier {
    pub index: BufferIndex,
    pub src: BufferBarrierSource,
    pub dst: BufferResourceAccess,
}

#[derive(Debug, Default)]
pub enum ImageBarrierSource {
    #[default]
    /// Retrieve usage from a previous frame
    FirstUsage,

    /// Precalculated usage from the graph
    Precalculated(ImageResourceAccess),
}

#[derive(Debug, Default)]
pub struct ImageBarrier {
    pub index: ImageIndex,
    pub src: ImageBarrierSource,
    pub dst: ImageResourceAccess,
}

#[derive(Debug, Default)]
pub struct RenderPassSet {
    pub memory_barriers: Vec<vk::MemoryBarrier2>,
    pub buffer_barriers: Vec<BufferBarrier>,
    pub image_barriers: Vec<ImageBarrier>,

    pub render_passes: Vec<RenderPass2>,
}

#[derive(Debug, Default)]
pub struct BufferOwnershipTransfer {
    pub index: BufferIndex,
    pub access_flags: vk::AccessFlags2,
}

#[derive(Debug, Default)]
pub struct ImageOwnershipTransfer {
    pub index: BufferIndex,
    pub access_flags: vk::AccessFlags2,
}

#[derive(Debug)]
pub enum CommandBufferDependency {
    CommandBuffer {
        /// The index of the command buffer
        command_buffer_index: usize,

        /// The index of the dependency, used for sync primitive lookup
        dependency_index: usize,

        /// The wait or signal stage of the semaphore
        stage_mask: vk::PipelineStageFlags2,

        /// The buffers to send or receive
        buffer_ownership_transfer: Vec<BufferOwnershipTransfer>,

        /// The images to send or receive
        image_ownership_transfer: Vec<ImageOwnershipTransfer>,
    },
    Swapchain {
        index: usize,
        access: ImageResourceAccess,
    },
}

#[derive(Debug, Default)]
pub struct CommandBuffer {
    /// Queue that the command buffer is submitted to
    pub queue: Queue,

    /// List of command buffers / swapchains that this command buffer is dependant on
    pub command_buffer_wait_dependencies: Vec<CommandBufferDependency>,

    pub render_pass_sets: Vec<RenderPassSet>,

    /// List of command buffers / swapchains that depend on this command buffer
    pub command_buffer_signal_dependencies: Vec<CommandBufferDependency>,
}

#[derive(Debug, Default)]
pub struct CompiledRenderGraph {
    //TODO: Update this to contain first and last usages with queue
    /// List of buffers used by this graph
    pub buffer_resources: Vec<BufferGraphResource>,

    //TODO: Update this to contain first and last usages with queue
    /// List of images used by this graph
    pub image_resources: Vec<ImageGraphResource>,

    /// List of swapchains and swapchain images used by this graph
    pub swapchain_images: Vec<(SurfaceHandle, ImageIndex)>,

    pub command_buffers: Vec<CommandBuffer>,
}
