use crate::render_graph_builder::{BufferReadCallback, BufferWriteCallback};
use crate::resource_managers::{BufferResourceAccess, BufferTempResource, ImageResourceAccess};
use crate::{
    BufferKey, BufferUsage, ComputePipelineHandle, ImageKey, RasterPipelineHandle, SamplerHandle,
    SurfaceHandle, TransientImageDesc,
};
use ash::vk;
use log::info;
use std::fmt::{Debug, Formatter};
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
    Transient {
        size: usize,
        usage: BufferUsage,
        location: gpu_allocator::MemoryLocation,
    },
}

impl BufferResourceDescription {
    pub fn is_persistent(&self) -> bool {
        match self {
            BufferResourceDescription::Persistent(_) => true,
            BufferResourceDescription::Transient { .. } => false,
        }
    }

    pub fn as_persistent(&self) -> Option<BufferKey> {
        match self {
            BufferResourceDescription::Persistent(key) => Some(*key),
            BufferResourceDescription::Transient { .. } => None,
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

// TODO: Determine the best pre and/or post frame ownership barriers
// Graph Builders:
// 1. Debug = Single Queue + Serial + Image Transitions + Global Memory Barriers
// 2. Basic = Single Queue + Pass Promoting + Image/Buffer Transitions
// 3. Graph = Single Queue + Topological Sort + Image/Buffer Transitions
// 4. GraphMultiQueue = Multiple Queues + Topological Sort + Image/Buffer Transitions

pub struct BufferWrite {
    pub(crate) index: BufferIndex,
    pub(crate) callback: BufferWriteCallback,
}
impl Debug for BufferWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferWrite")
            .field("index", &self.index)
            .finish()
    }
}

pub struct BufferWrite2 {
    pub(crate) buffer_offset: BufferOffset,
    pub(crate) write_size: usize,
    pub(crate) callback: BufferWriteCallback,
}
impl Debug for BufferWrite2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferWrite")
            .field("index", &self.buffer_offset.buffer)
            .field("offset", &self.buffer_offset.offset)
            .field("write_size", &self.write_size)
            .finish()
    }
}

#[derive(Clone)]
pub struct BufferRead {
    pub(crate) index: BufferIndex,
    pub(crate) callback: BufferReadCallback,
}
impl Debug for BufferRead {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferRead")
            .field("index", &self.index)
            .finish()
    }
}

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub enum Queue {
    #[default]
    Graphics,
    Compute,
    Transfer,
}

#[derive(Debug)]
pub struct RenderPass {
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

    pub render_passes: Vec<RenderPass>,
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
pub struct BufferWrites {
    pub total_write_size: usize,
    pub buffer_writes: Vec<BufferWrite2>,
}

impl BufferWrites {
    pub fn push(&mut self, write: BufferWrite2) {
        self.total_write_size += write.write_size;
        self.buffer_writes.push(write);
    }

    pub fn calc_needed_staging_size(&self, buffer_resources: &[BufferTempResource]) -> usize {
        self.buffer_writes
            .iter()
            .map(|write| {
                if buffer_resources[write.buffer_offset.buffer]
                    .mapped_slice
                    .is_some()
                {
                    0
                } else {
                    write.write_size
                }
            })
            .sum()
    }
}

#[derive(Debug, Default)]
pub struct CompiledRenderGraph {
    pub buffer_writes2: BufferWrites,

    pub buffer_writes: Vec<BufferWrite>,
    pub buffer_reads: Vec<BufferRead>,

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
