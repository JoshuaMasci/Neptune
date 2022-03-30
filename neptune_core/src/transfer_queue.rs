use crate::render_backend::RenderDevice;
use crate::vulkan::{Buffer, BufferDescription, Image};
use ash::vk;
use gpu_allocator::MemoryLocation;
use std::cell::RefCell;
use std::rc::Rc;

struct BufferTransfer {
    src_buffer: Buffer,
    dst_buffer: Buffer,
}

struct ImageTransfer {
    src_buffer: Buffer,
    dst_image: Image,
    final_layout: vk::ImageLayout,
}

pub struct TransferQueue {
    device: RenderDevice,

    buffer_transfers: Vec<BufferTransfer>,
    old_buffer_transfers: Vec<BufferTransfer>,

    image_transfers: Vec<ImageTransfer>,
    old_image_transfers: Vec<ImageTransfer>,
}

impl TransferQueue {
    pub fn new(device: RenderDevice) -> Self {
        Self {
            device,
            buffer_transfers: vec![],
            old_buffer_transfers: vec![],
            image_transfers: vec![],
            old_image_transfers: vec![],
        }
    }

    fn create_staging_buffer<T: std::marker::Copy>(&mut self, data: &[T]) -> Buffer {
        let staging_buffer = Buffer::new(
            &self.device,
            BufferDescription {
                size: std::mem::size_of::<T>() * data.len(),
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                memory_location: MemoryLocation::CpuToGpu,
            },
        );
        staging_buffer.fill(data);
        staging_buffer
    }

    pub fn copy_to_buffer<T: std::marker::Copy>(&mut self, buffer: &Buffer, data: &[T]) {
        if !buffer
            .description
            .usage
            .contains(vk::BufferUsageFlags::TRANSFER_DST)
        {
            panic!("Buffer must include vk::BufferUsageFlags::TRANSFER_DST to write to it");
        }

        let staging_buffer = self.create_staging_buffer(data);

        self.buffer_transfers.push(BufferTransfer {
            src_buffer: staging_buffer,
            dst_buffer: buffer.clone_no_drop(),
        });
    }

    pub fn copy_to_image<T: std::marker::Copy>(
        &mut self,
        image: &Image,
        final_layout: vk::ImageLayout,
        data: &[T],
    ) {
        if !image
            .description
            .usage
            .contains(vk::ImageUsageFlags::TRANSFER_DST)
        {
            panic!("Image must include vk::ImageUsageFlags::TRANSFER_DST to write to it");
        }

        let staging_buffer = self.create_staging_buffer(data);

        self.image_transfers.push(ImageTransfer {
            src_buffer: staging_buffer,
            dst_image: image.clone_no_drop(),
            final_layout,
        });
    }

    pub fn commit_transfers(&mut self, command_buffer: vk::CommandBuffer) {
        for buffer_transfer in self.buffer_transfers.iter() {
            let copy_size = buffer_transfer
                .dst_buffer
                .description
                .size
                .min(buffer_transfer.src_buffer.description.size)
                as vk::DeviceSize;

            unsafe {
                self.device.base.cmd_copy_buffer(
                    command_buffer,
                    buffer_transfer.src_buffer.handle,
                    buffer_transfer.dst_buffer.handle,
                    &[vk::BufferCopy {
                        src_offset: 0,
                        dst_offset: 0,
                        size: copy_size,
                    }],
                );
            }
        }

        let image_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_array_layer(0)
            .layer_count(1)
            .base_mip_level(0)
            .level_count(1)
            .build();

        let image_barriers1: Vec<vk::ImageMemoryBarrier2KHR> = self
            .image_transfers
            .iter()
            .map(|transfer| {
                vk::ImageMemoryBarrier2KHR::builder()
                    .image(transfer.dst_image.handle)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .src_access_mask(vk::AccessFlags2KHR::NONE)
                    .src_stage_mask(vk::PipelineStageFlags2KHR::NONE)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_access_mask(vk::AccessFlags2KHR::TRANSFER_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2KHR::TRANSFER)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .subresource_range(image_range)
                    .build()
            })
            .collect();
        let image_barriers2: Vec<vk::ImageMemoryBarrier2KHR> = self
            .image_transfers
            .iter()
            .map(|transfer| {
                vk::ImageMemoryBarrier2KHR::builder()
                    .image(transfer.dst_image.handle)
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(transfer.final_layout)
                    .src_access_mask(vk::AccessFlags2KHR::TRANSFER_WRITE)
                    .src_stage_mask(vk::PipelineStageFlags2KHR::TRANSFER)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_access_mask(vk::AccessFlags2KHR::NONE)
                    .dst_stage_mask(vk::PipelineStageFlags2KHR::NONE)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .subresource_range(image_range)
                    .build()
            })
            .collect();

        unsafe {
            self.device.synchronization2.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoKHR::builder().image_memory_barriers(&image_barriers1),
            );
        }

        for transfer in self.image_transfers.iter() {
            unsafe {
                self.device.base.cmd_copy_buffer_to_image(
                    command_buffer,
                    transfer.src_buffer.handle,
                    transfer.dst_image.handle,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[vk::BufferImageCopy {
                        buffer_offset: 0,
                        buffer_row_length: 0,
                        buffer_image_height: 0,
                        image_subresource: vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                        image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
                        image_extent: vk::Extent3D {
                            width: transfer.dst_image.description.size[0],
                            height: transfer.dst_image.description.size[1],
                            depth: 1,
                        },
                    }],
                );
            }
        }

        unsafe {
            self.device.synchronization2.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoKHR::builder().image_memory_barriers(&image_barriers2),
            );
        }

        self.old_image_transfers = self.image_transfers.drain(..).collect();
        self.old_buffer_transfers = self.buffer_transfers.drain(..).collect();
    }
}
