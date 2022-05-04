use crate::render_graph::{BufferResourceDescription, RenderFn, RenderGraph, ResourcesAccessType};
use crate::resource::Resource;
use crate::vulkan::{Buffer, Device};
use crate::BufferAccessType;
use ash::vk;
use std::rc::Rc;

enum VulkanBufferStorage {
    Unused,
    Temporary(Resource<Buffer>),
    Imported(Rc<Resource<Buffer>>),
}

impl VulkanBufferStorage {
    fn is_unused(&self) -> bool {
        match self {
            VulkanBufferStorage::Unused => true,
            _ => false,
        }
    }
}

struct VulkanBuffer {
    storage: VulkanBufferStorage,
    //TODO: vk buffer description
    handle: vk::Buffer,
    binding: Option<u32>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
struct BarrierFlags {
    stage: vk::PipelineStageFlags2KHR,
    access: vk::AccessFlags2KHR,
}

impl BarrierFlags {
    fn add(&mut self, other: Self) {
        self.stage |= other.stage;
        self.access |= other.access;
    }
}

struct VulkanBufferAccessFlags(vk::PipelineStageFlags2, vk::AccessFlags2);

struct BufferBarrier {
    handle: vk::Buffer,
    src: BarrierFlags,
    dst: BarrierFlags,
}

struct TextureBarrier {
    handle: vk::Image,
    src: BarrierFlags,
    old_layout: vk::ImageLayout,
    dst: BarrierFlags,
    new_layout: vk::ImageLayout,
}

struct VulkanPass {
    pub(crate) name: String,
    pub(crate) pre_buffer_barriers: Vec<BufferBarrier>,
    pub(crate) post_buffer_barriers: Vec<BufferBarrier>,

    pub(crate) render_fn: Option<Box<RenderFn>>,
}

struct VulkanGraph {
    buffers: Vec<VulkanBuffer>,
    passes: Vec<VulkanPass>,
}

const INITIAL_PASS: i32 = -1;

impl VulkanGraph {
    pub(crate) fn new(device: &mut Device, mut render_graph: RenderGraph) -> Self {
        let buffers: Vec<VulkanBuffer> = render_graph
            .buffers
            .iter()
            .enumerate()
            .map(|(i, buffer)| {
                if !buffer.accesses.is_empty() {
                    match &buffer.description {
                        BufferResourceDescription::New(description) => {
                            let buffer =
                                device.create_buffer(description.clone(), "Temporary Graph Buffer");
                            let handle = buffer.handle;
                            let binding = buffer.binding.as_ref().map(|binding| binding.index);
                            VulkanBuffer {
                                storage: VulkanBufferStorage::Temporary(buffer),
                                handle,
                                binding,
                            }
                        }
                        BufferResourceDescription::Import(buffer, last_access) => {
                            let handle = buffer.handle;
                            let binding = buffer.binding.as_ref().map(|binding| binding.index);
                            VulkanBuffer {
                                storage: VulkanBufferStorage::Imported(buffer.clone()),
                                handle,
                                binding,
                            }
                        }
                    }
                } else {
                    VulkanBuffer {
                        storage: VulkanBufferStorage::Unused,
                        handle: vk::Buffer::null(),
                        binding: None,
                    }
                }
            })
            .collect();

        //Start with a pass that will be used for Import Resource Barriers
        let mut passes: Vec<VulkanPass> = render_graph
            .passes
            .iter_mut()
            .enumerate()
            .map(|(i, pass)| VulkanPass {
                name: pass.name.clone(),
                pre_buffer_barriers: vec![],
                post_buffer_barriers: vec![],
                render_fn: pass.render_fn.take(),
            })
            .collect();

        //TODO: create pass for swapchain image transition?

        // for (i, buffer) in render_graph
        //     .buffers
        //     .iter()
        //     .enumerate()
        //     .filter(|(i, buffer)| !buffer.accesses.is_empty())
        // {
        //     let mut i = 0;
        //
        //     if let Some(last_access) = buffer.last_access {
        //         match last_access.get_type() {
        //             ResourcesAccessType::None => {}
        //             ResourcesAccessType::Read => {
        //
        //
        //
        //             }
        //             ResourcesAccessType::Write => {}
        //         }
        //     }
        //
        //     let mut read_accesses: Vec<BufferAccessType> = Vec::new();
        //
        //     if buffer.last_access.is_write() {}
        //
        //     let read_flags = BarrierFlags::default();
        //
        //     let handle = buffers[i].handle;
        //     let last_access_write = buffer.last_access.is_write();
        //     let next_access_write = buffer.accesses[0].1.is_write();
        //     let next_pass_index = buffer.accesses[0].0;
        //     passes[next_pass_index]
        //         .pre_buffer_barriers
        //         .push(BufferBarrier {
        //             handle,
        //             src: get_buffer_barrier_flags(buffer.last_access),
        //             dst: get_buffer_barrier_flags(buffer.accesses[0].1),
        //         });
        // }

        Self { buffers, passes }
    }
}

fn calc_buffer_barriers(passes: &mut Vec<VulkanPass>, buffers: &[VulkanBuffer]) {

    // for (buffer_index, (buffer, buffer_access)) in buffers
    //     .iter()
    //     .zip(buffer_accesses)
    //     .enumerate()
    //     .filter(|(_, buffer)| !buffer.0.storage.is_unused())
    // {
    //     //TODO: decide on barrier algorithm
    //     for mut i in 0..buffer_access.len() {
    //         if buffer_access[i].1.is_write() {}
    //     }
    // }
}

fn get_buffer_barrier_flags(buffer_access: BufferAccessType) -> BarrierFlags {
    match buffer_access {
        BufferAccessType::None => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::NONE,
            access: vk::AccessFlags2KHR::NONE,
        },
        BufferAccessType::IndexBuffer => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::INDEX_INPUT,
            access: vk::AccessFlags2KHR::MEMORY_READ,
        },
        BufferAccessType::VertexBuffer => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::VERTEX_INPUT,
            access: vk::AccessFlags2KHR::MEMORY_READ,
        },
        BufferAccessType::TransferRead => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::TRANSFER,
            access: vk::AccessFlags2KHR::TRANSFER_READ,
        },
        BufferAccessType::TransferWrite => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::TRANSFER,
            access: vk::AccessFlags2KHR::TRANSFER_WRITE,
        },
        BufferAccessType::ShaderRead => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
            access: vk::AccessFlags2KHR::SHADER_STORAGE_READ,
        },
        BufferAccessType::ShaderWrite => BarrierFlags {
            stage: vk::PipelineStageFlags2KHR::ALL_GRAPHICS,
            access: vk::AccessFlags2KHR::SHADER_STORAGE_WRITE,
        },
    }
}
