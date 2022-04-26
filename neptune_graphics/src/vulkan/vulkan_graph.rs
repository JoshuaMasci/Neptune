use crate::render_graph::{BufferAccessType, BufferResourceDescription, RenderFn, RenderGraph};
use crate::resource::Resource;
use crate::vulkan::{Buffer, Device};
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

struct VulkanBarrierFlags {
    src: (vk::PipelineStageFlags2, vk::AccessFlags2),
    dst: (vk::PipelineStageFlags2, vk::AccessFlags2),
}

struct VulkanBufferAccessFlags(vk::PipelineStageFlags2, vk::AccessFlags2);

struct BufferBarrier {
    handle: vk::Buffer,
    flags: VulkanBarrierFlags,
}

struct TextureBarrier {
    handle: vk::Image,
    flags: VulkanBarrierFlags,
    old_layout: vk::ImageLayout,
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
        let buffers = render_graph
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
        let mut passes = render_graph
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
