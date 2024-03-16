use crate::buffer::{AshBuffer, Buffer};
use crate::descriptor_set::{DescriptorCount, DescriptorSet};
use crate::device::AshDevice;
use crate::image::{AshImage, Image, TransientImageSize};
use crate::render_graph::{
    BufferGraphResource, BufferRead, BufferResourceDescription, BufferWrites, ImageGraphResource,
    ImageResourceDescription,
};
use crate::sampler::Sampler;
use crate::swapchain::AcquiredSwapchainImage;
use crate::{BufferKey, BufferUsage, ImageHandle, ImageKey, SamplerKey, VulkanError};
use ash::vk;
use gpu_allocator::vulkan::Allocation;
use gpu_allocator::MemoryLocation;
use log::{error, warn};
use slotmap::SlotMap;
use std::sync::Arc;

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub enum Queue {
    #[default]
    Graphics,
}

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub struct BufferBarrierFlags {
    pub stage_mask: vk::PipelineStageFlags2,
    pub access_mask: vk::AccessFlags2,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferResourceAccess {
    #[default]
    None,
    TransferRead,
    TransferWrite,
    VertexRead,
    IndexRead,
    IndirectRead,
    UniformRead,
    StorageRead,
    StorageWrite,
}

impl BufferResourceAccess {
    pub fn get_barrier_flags(&self) -> BufferBarrierFlags {
        //TODO: select shader flags based on pass type or pipeline?
        let shader_all: vk::PipelineStageFlags2 = vk::PipelineStageFlags2::VERTEX_SHADER
            | vk::PipelineStageFlags2::FRAGMENT_SHADER
            | vk::PipelineStageFlags2::COMPUTE_SHADER;
        // | vk::PipelineStageFlags2::TASK_SHADER_EXT
        // | vk::PipelineStageFlags2::MESH_SHADER_EXT
        // | vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR;

        match self {
            Self::None => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::NONE,
                access_mask: vk::AccessFlags2::NONE,
            },
            Self::TransferRead => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::TRANSFER,
                access_mask: vk::AccessFlags2::MEMORY_READ,
            },
            Self::TransferWrite => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::TRANSFER,
                access_mask: vk::AccessFlags2::MEMORY_WRITE,
            },
            Self::VertexRead => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::VERTEX_INPUT,
                access_mask: vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
            },
            Self::IndexRead => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::VERTEX_INPUT,
                access_mask: vk::AccessFlags2::INDEX_READ,
            },
            Self::IndirectRead => BufferBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::DRAW_INDIRECT,
                access_mask: vk::AccessFlags2::INDIRECT_COMMAND_READ,
            },
            Self::UniformRead => BufferBarrierFlags {
                stage_mask: shader_all,
                access_mask: vk::AccessFlags2::UNIFORM_READ,
            },
            Self::StorageRead => BufferBarrierFlags {
                stage_mask: shader_all,
                access_mask: vk::AccessFlags2::SHADER_STORAGE_READ,
            },
            Self::StorageWrite => BufferBarrierFlags {
                stage_mask: shader_all,
                access_mask: vk::AccessFlags2::SHADER_WRITE,
            },
        }
    }
}

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub struct ImageBarrierFlags {
    pub stage_mask: vk::PipelineStageFlags2,
    pub access_mask: vk::AccessFlags2,
    pub layout: vk::ImageLayout,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageResourceAccess {
    #[default]
    None,
    TransferRead,
    TransferWrite,
    AttachmentWrite,
    SampledRead,
    StorageRead,
    StorageWrite,
}

impl ImageResourceAccess {
    pub fn get_barrier_flags(&self, is_color_image: bool) -> ImageBarrierFlags {
        //TODO: select shader flags based on pass type or pipeline?
        let shader_all: vk::PipelineStageFlags2 = vk::PipelineStageFlags2::VERTEX_SHADER
            | vk::PipelineStageFlags2::FRAGMENT_SHADER
            | vk::PipelineStageFlags2::COMPUTE_SHADER;
        // | vk::PipelineStageFlags2::TASK_SHADER_EXT
        // | vk::PipelineStageFlags2::MESH_SHADER_EXT
        // | vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR;

        match self {
            Self::None => ImageBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::NONE,
                access_mask: vk::AccessFlags2::NONE,
                layout: vk::ImageLayout::UNDEFINED,
            },
            Self::TransferRead => ImageBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::TRANSFER,
                access_mask: vk::AccessFlags2::TRANSFER_READ,
                layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            },
            Self::TransferWrite => ImageBarrierFlags {
                stage_mask: vk::PipelineStageFlags2::TRANSFER,
                access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            },
            Self::AttachmentWrite => {
                if is_color_image {
                    ImageBarrierFlags {
                        stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    }
                } else {
                    ImageBarrierFlags {
                        stage_mask: vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                        access_mask: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
                    }
                }
            }
            Self::SampledRead => ImageBarrierFlags {
                stage_mask: shader_all,
                access_mask: vk::AccessFlags2::SHADER_READ,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            Self::StorageRead => ImageBarrierFlags {
                stage_mask: shader_all,
                access_mask: vk::AccessFlags2::SHADER_READ,
                layout: vk::ImageLayout::GENERAL,
            },
            Self::StorageWrite => ImageBarrierFlags {
                stage_mask: shader_all,
                access_mask: vk::AccessFlags2::SHADER_WRITE,
                layout: vk::ImageLayout::GENERAL,
            },
        }
    }
}

pub struct BufferResource {
    pub buffer: Buffer,
    pub queue_owner: Option<Queue>,

    //TODO: move to frame context
    pub last_access: BufferResourceAccess,
}

pub enum BufferTempDescription {
    Persistent(BufferKey),
    Transient(usize),
}

pub struct MappedSlice {
    ptr: std::ptr::NonNull<std::ffi::c_void>,
    size: usize,
}

impl MappedSlice {
    pub fn new(allocation: &Allocation) -> Option<Self> {
        allocation.mapped_ptr().map(|ptr| Self {
            ptr,
            size: allocation.size() as usize,
        })
    }

    pub fn slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.cast().as_ptr(), self.size) }
    }

    pub fn slice_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.cast().as_ptr(), self.size) }
    }
}

pub struct BufferTempResource {
    pub description: BufferTempDescription,
    pub supports_direct_upload: bool,
    pub mapped_slice: Option<MappedSlice>,
    pub buffer: AshBuffer,
    pub last_access: BufferResourceAccess,
}

pub struct ImageResource {
    pub image: Image,
    pub last_access: ImageResourceAccess,
}

pub struct ImageTempResource {
    pub image: AshImage,
    pub last_access: ImageResourceAccess,
}

#[derive(Default)]
struct ResourceFrame {
    freed_buffers: Vec<BufferKey>,
    freed_images: Vec<ImageKey>,
    transient_buffers: Vec<Buffer>,
    transient_images: Vec<Image>,

    write_staging_buffer: Option<Buffer>,

    buffer_reads: Vec<BufferRead>,
}

pub struct ResourceManager {
    #[allow(unused)]
    device: Arc<AshDevice>,

    pub(crate) descriptor_set: DescriptorSet,

    buffers: SlotMap<BufferKey, BufferResource>,
    freed_buffers: Vec<BufferKey>,

    images: SlotMap<ImageKey, ImageResource>,
    freed_images: Vec<ImageKey>,

    samplers: SlotMap<SamplerKey, Arc<Sampler>>,

    frames_in_flight: Vec<ResourceFrame>,
    frame_index: usize,
}

impl ResourceManager {
    pub fn new(device: Arc<AshDevice>, frame_in_flight_count: u32) -> Self {
        let descriptor_set = DescriptorSet::new(
            device.clone(),
            DescriptorCount {
                storage_buffers: 1024,
                storage_images: 1024,
                sampled_images: 1024,
                samplers: 128,
                ..Default::default()
            },
        )
        .unwrap();

        let frames_in_flight = (0..frame_in_flight_count as usize)
            .map(|_| ResourceFrame::default())
            .collect();

        Self {
            device,

            buffers: SlotMap::with_key(),
            freed_buffers: Vec::new(),

            images: SlotMap::with_key(),
            freed_images: Vec::new(),

            samplers: SlotMap::with_key(),

            descriptor_set,
            frames_in_flight,
            frame_index: 0,
        }
    }

    pub fn flush_frame(&mut self) {
        self.frame_index = (self.frame_index + 1) % self.frames_in_flight.len();
        let frame = &mut self.frames_in_flight[self.frame_index];

        //Read callbacks
        // for buffer_read in frame.buffer_reads.drain(..) {
        //     let buffer = frame.transient_buffers.get(&buffer_read.index).unwrap();
        //     match buffer.allocation.mapped_slice() {
        //         None => {
        //             todo!("Return Error, target buffer not mapped");
        //         }
        //         Some(buffer_slice) => {
        //             (buffer_read.callback)(buffer_slice);
        //         }
        //     }
        // }

        for key in frame.freed_buffers.drain(..) {
            if self.buffers.remove(key).is_none() {
                warn!("BufferKey({:?}) was invalid on deletion", key);
            }
        }
        for key in frame.freed_images.drain(..) {
            if self.images.remove(key).is_none() {
                warn!("ImageKey({:?}) was invalid on deletion", key);
            }
        }

        frame.freed_buffers = std::mem::take(&mut self.freed_buffers);
        frame.freed_images = std::mem::take(&mut self.freed_images);
        frame.transient_buffers.clear();
        frame.transient_images.clear();
    }

    //Buffers
    pub fn create_buffer(
        &mut self,
        name: &str,
        size: usize,
        usage: BufferUsage,
        location: MemoryLocation,
    ) -> Result<BufferKey, VulkanError> {
        let mut buffer = Buffer::new2(
            self.device.clone(),
            name,
            size as vk::DeviceSize,
            usage.to_vk(),
            location,
        )?;

        if buffer.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
            buffer.storage_binding = Some(self.descriptor_set.bind_storage_buffer(&buffer));
        }

        Ok(self.buffers.insert(BufferResource {
            buffer,
            queue_owner: None,
            last_access: Default::default(),
        }))
    }

    pub fn add_buffer(&mut self, mut buffer: Buffer) -> BufferKey {
        if buffer.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
            buffer.storage_binding = Some(self.descriptor_set.bind_storage_buffer(&buffer));
        }

        self.buffers.insert(BufferResource {
            buffer,
            queue_owner: None,
            last_access: Default::default(),
        })
    }
    pub fn remove_buffer(&mut self, key: BufferKey) {
        self.freed_buffers.push(key);
    }

    //Images
    pub fn add_image(&mut self, mut image: Image) -> ImageKey {
        if image.usage.contains(vk::ImageUsageFlags::STORAGE) {
            image.storage_binding = Some(self.descriptor_set.bind_storage_image(&image));
        }

        if image.usage.contains(vk::ImageUsageFlags::SAMPLED) {
            image.sampled_binding = Some(self.descriptor_set.bind_sampled_image(&image));
        }

        self.images.insert(ImageResource {
            image,
            last_access: ImageResourceAccess::None,
        })
    }
    pub fn get_image(&self, key: ImageKey) -> Option<&Image> {
        self.images.get(key).map(|resource| &resource.image)
    }
    pub fn remove_image(&mut self, key: ImageKey) {
        self.freed_images.push(key);
    }

    //Samplers
    pub fn add_sampler(&mut self, mut sampler: Sampler) -> SamplerKey {
        sampler.binding = Some(self.descriptor_set.bind_sampler(&sampler));
        self.samplers.insert(Arc::new(sampler))
    }
    pub fn get_sampler(&self, key: SamplerKey) -> Option<Arc<Sampler>> {
        self.samplers.get(key).cloned()
    }
    pub fn remove_sampler(&mut self, key: SamplerKey) {
        if self.samplers.remove(key).is_none() {
            warn!("Tried to remove invalid SamplerKey({:?})", key);
        }
    }

    pub fn get_write_staging_buffer(
        &mut self,
        buffer_size: usize,
    ) -> Result<Option<BufferTempResource>, VulkanError> {
        let needed_staging_buffer_size = buffer_size;

        //Resize if needed
        Ok(if needed_staging_buffer_size > 0 {
            let frame = &mut self.frames_in_flight[self.frame_index];

            let current_size = frame
                .write_staging_buffer
                .as_ref()
                .map(|buffer| buffer.size as usize)
                .unwrap_or_default();

            if needed_staging_buffer_size > current_size {
                frame.write_staging_buffer = Some(Buffer::new2(
                    self.device.clone(),
                    "Write Staging Buffer",
                    needed_staging_buffer_size as vk::DeviceSize,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    MemoryLocation::CpuToGpu,
                )?);
            }

            frame
                .write_staging_buffer
                .as_ref()
                .map(|buffer| BufferTempResource {
                    description: BufferTempDescription::Transient(0),
                    supports_direct_upload: true,
                    mapped_slice: MappedSlice::new(&buffer.allocation),
                    buffer: buffer.get_copy(),
                    last_access: BufferResourceAccess::None,
                })
        } else {
            None
        })
    }

    pub fn write_buffers2(
        &mut self,
        buffer_writes: &BufferWrites,
        buffer_resources: &[BufferTempResource],
    ) -> Result<(), VulkanError> {
        // Make sure the staging buffer is large enough
        let needed_staging_buffer_size: usize = buffer_writes
            .buffer_writes
            .iter()
            .map(|write| {
                if buffer_resources[write.buffer_offset.buffer].supports_direct_upload {
                    0
                } else {
                    write.write_size
                }
            })
            .sum();

        let frame = &mut self.frames_in_flight[self.frame_index];

        //Resize if needed
        if needed_staging_buffer_size > 0 {
            let current_size = frame
                .write_staging_buffer
                .as_ref()
                .map(|buffer| buffer.size as usize)
                .unwrap_or_default();

            if needed_staging_buffer_size > current_size {
                frame.write_staging_buffer = Some(Buffer::new2(
                    self.device.clone(),
                    "Write Staging Buffer",
                    needed_staging_buffer_size as vk::DeviceSize,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    MemoryLocation::CpuToGpu,
                )?);
            }
        }

        let mut staring_buffer_offset = 0;
        for buffer_write in buffer_writes.buffer_writes.iter() {
            let buffer = match &buffer_resources[buffer_write.buffer_offset.buffer].description {
                BufferTempDescription::Persistent(buffer_key) => {
                    &mut self.buffers.get_mut(*buffer_key).unwrap().buffer
                }
                BufferTempDescription::Transient(index) => &mut frame.transient_buffers[*index],
            };

            if buffer_resources[buffer_write.buffer_offset.buffer].supports_direct_upload {
                let buffer_slice = buffer
                    .allocation
                    .mapped_slice_mut()
                    .expect("Buffer should be memory mapped");
                let write_start = buffer_write.buffer_offset.offset as usize;
                let write_end = write_start + buffer_write.write_size;
                buffer_write
                    .callback
                    .call(&mut buffer_slice[write_start..write_end]);
            } else {
                let buffer_slice = frame
                    .write_staging_buffer
                    .as_mut()
                    .unwrap()
                    .allocation
                    .mapped_slice_mut()
                    .expect("Staging buffer must be memory mapped");
                let write_start = staring_buffer_offset;
                let write_end = write_start + buffer_write.write_size;

                buffer_write
                    .callback
                    .call(&mut buffer_slice[write_start..write_end]);
                staring_buffer_offset = write_end;
            }
        }

        Ok(())
    }

    pub fn read_buffers(&mut self, buffer_reads: &[BufferRead]) {
        let frame = &mut self.frames_in_flight[self.frame_index];
        frame.buffer_reads = buffer_reads.to_vec();
    }

    //Graph Functions
    //TODO: take in vector to reuse memory?
    /// Get the buffer resources and update the last usages
    pub fn get_buffer_resources(
        &mut self,
        graph_buffers: &[BufferGraphResource],
    ) -> Result<Vec<BufferTempResource>, VulkanError> {
        let frame_count = self.frames_in_flight.len();
        let frame = &mut self.frames_in_flight[self.frame_index];

        let mut buffer_resources = Vec::with_capacity(graph_buffers.len());
        for (index, graph_buffer) in graph_buffers.iter().enumerate() {
            buffer_resources.push(match &graph_buffer.description {
                BufferResourceDescription::Persistent(key) => {
                    let resource = &mut self.buffers[*key];

                    // Can directly upload to persistent buffers only if there is one frame in flight
                    let supports_direct_upload = resource.buffer.is_mapped() && frame_count == 1;

                    //TODO: get usages with multiple frames in flight
                    //TODO: write last usages + queue
                    BufferTempResource {
                        description: BufferTempDescription::Persistent(*key),
                        supports_direct_upload,
                        mapped_slice: supports_direct_upload
                            .then(|| MappedSlice::new(&resource.buffer.allocation))
                            .flatten(),
                        buffer: resource.buffer.get_copy(),
                        last_access: std::mem::replace(
                            &mut resource.last_access,
                            graph_buffer.last_access,
                        ),
                    }
                }
                BufferResourceDescription::Transient {
                    size,
                    usage,
                    location,
                } => {
                    let mut buffer = Buffer::new2(
                        self.device.clone(),
                        "Transient Buffer",
                        *size as vk::DeviceSize,
                        usage.to_vk(),
                        *location,
                    )?;
                    if buffer.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
                        buffer.storage_binding =
                            Some(self.descriptor_set.bind_storage_buffer(&buffer));
                    }

                    let resource = BufferTempResource {
                        description: BufferTempDescription::Transient(
                            frame.transient_buffers.len(),
                        ),
                        supports_direct_upload: buffer.is_mapped(),
                        mapped_slice: MappedSlice::new(&buffer.allocation),
                        buffer: buffer.get_copy(),
                        last_access: BufferResourceAccess::None, //Never used before
                    };
                    frame.transient_buffers.push(buffer);
                    resource
                }
            });
        }

        Ok(buffer_resources)
    }

    //TODO: take in vector to reuse memory?
    /// Get the image resources and update the last usages
    pub fn get_image_resources(
        &mut self,
        swapchain_images: &[AcquiredSwapchainImage],
        graph_images: &[ImageGraphResource],
    ) -> Result<Vec<ImageTempResource>, VulkanError> {
        let mut image_resources = Vec::with_capacity(graph_images.len());
        for graph_image in graph_images {
            image_resources.push(match &graph_image.description {
                ImageResourceDescription::Persistent(key) => {
                    let image = &mut self.images[*key];
                    //TODO: get usages with multiple frames in flight
                    //TODO: write last usages + queue + layout
                    ImageTempResource {
                        image: image.image.get_copy(),
                        last_access: std::mem::replace(
                            &mut image.last_access,
                            graph_image.last_access.unwrap_or_default(),
                        ),
                    }
                }
                ImageResourceDescription::Transient(transient_image_description) => {
                    let image_size = get_transient_image_size(
                        transient_image_description.size.clone(),
                        self,
                        graph_images,
                        swapchain_images,
                    );
                    let image_description = transient_image_description
                        .to_image_description([image_size.width, image_size.height]);
                    let mut image =
                        Image::new_2d(self.device.clone(), "Transient Image", &image_description)?;

                    if image.usage.contains(vk::ImageUsageFlags::STORAGE) {
                        image.storage_binding =
                            Some(self.descriptor_set.bind_storage_image(&image));
                    }

                    if image.usage.contains(vk::ImageUsageFlags::SAMPLED) {
                        image.sampled_binding =
                            Some(self.descriptor_set.bind_sampled_image(&image));
                    }

                    let resource = ImageTempResource {
                        image: image.get_copy(),
                        last_access: ImageResourceAccess::None, //Never used before
                    };
                    let frame = &mut self.frames_in_flight[self.frame_index];
                    frame.transient_images.push(image);
                    resource
                }
                ImageResourceDescription::Swapchain(index) => {
                    //Swapchain always starts out unused
                    ImageTempResource {
                        image: swapchain_images[*index].image,
                        last_access: ImageResourceAccess::None,
                    }
                }
            });
        }

        Ok(image_resources)
    }
}

fn get_transient_image_size(
    size: TransientImageSize,
    resource_manager: &ResourceManager,
    images: &[ImageGraphResource],
    swapchain_images: &[AcquiredSwapchainImage],
) -> vk::Extent2D {
    match size {
        TransientImageSize::Exact(extent) => extent,
        TransientImageSize::Relative(scale, target) => {
            let mut extent = match target {
                ImageHandle::Persistent(image_key) => {
                    resource_manager.get_image(image_key).as_ref().unwrap().size
                }
                ImageHandle::Transient(index) => match &images[index].description {
                    ImageResourceDescription::Persistent(image_key) => {
                        error!("Found a Persistent image when looking up a transient image size, this shouldn't happened (but I won't crash)");
                        resource_manager
                            .get_image(*image_key)
                            .as_ref()
                            .unwrap()
                            .size
                    }
                    ImageResourceDescription::Transient(desc) => get_transient_image_size(
                        desc.size.clone(),
                        resource_manager,
                        images,
                        swapchain_images,
                    ),
                    ImageResourceDescription::Swapchain(swapchain_index) => {
                        swapchain_images[*swapchain_index].image.size
                    }
                },
            };
            extent.width = ((extent.width as f32) * scale[0]) as u32;
            extent.height = ((extent.height as f32) * scale[1]) as u32;

            extent
        }
    }
}
