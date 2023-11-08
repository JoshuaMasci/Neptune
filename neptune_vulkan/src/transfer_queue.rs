use crate::device::{AshDevice, AshQueue};
use crate::image::vk_format_get_aspect_flags;
use crate::render_graph_builder::Transfer;
use crate::resource_managers::ResourceManager;
use crate::{BufferHandle, BufferKey, ImageHandle, ImageKey, VulkanError};
use ash::vk;
use std::sync::Arc;

struct TransferFrame {
    command_buffer: vk::CommandBuffer,
    transfer_done_fence: vk::Fence,
}

pub struct TransferQueue {
    device: Arc<AshDevice>,
    queue: AshQueue,
    command_pool: vk::CommandPool,
    transfer_frame_index: usize,
    transfer_frames: Vec<TransferFrame>,
    transfers: Vec<Transfer>,
}

impl TransferQueue {
    pub fn new(
        device: Arc<AshDevice>,
        device_queue_index: u32,
        transfer_frame_count: u32,
    ) -> ash::prelude::VkResult<Self> {
        let queue = device.queues[device_queue_index as usize].clone();

        let command_pool = unsafe {
            device.core.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(queue.family_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None,
            )
        }?;

        let command_buffers = unsafe {
            device.core.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .command_buffer_count(transfer_frame_count),
            )
        }?;

        let mut transfer_frames = Vec::with_capacity(command_buffers.len());
        for command_buffer in command_buffers {
            unsafe {
                transfer_frames.push(TransferFrame {
                    command_buffer,
                    transfer_done_fence: device.core.create_fence(
                        &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                        None,
                    )?,
                });
            }
        }

        Ok(Self {
            device,
            queue,
            command_pool,
            transfer_frame_index: 0,
            transfer_frames,
            transfers: Vec::new(),
        })
    }

    pub fn add_transfer(&mut self, transfer: Transfer) {
        self.transfers.push(transfer);
    }

    pub fn submit_transfers(
        &mut self,
        resource_manager: &mut ResourceManager,
    ) -> Result<(), VulkanError> {
        if self.transfers.is_empty() {
            return Ok(());
        }

        self.transfer_frame_index = (self.transfer_frame_index + 1) % self.transfer_frames.len();
        let transfer_frame = &self.transfer_frames[self.transfer_frame_index];

        const TIMEOUT_NS: u64 = std::time::Duration::from_secs(2).as_nanos() as u64;

        unsafe {
            self.device.core.wait_for_fences(
                &[transfer_frame.transfer_done_fence],
                true,
                TIMEOUT_NS,
            )?;

            self.device
                .core
                .reset_fences(&[transfer_frame.transfer_done_fence])?;

            self.device.core.begin_command_buffer(
                transfer_frame.command_buffer,
                &vk::CommandBufferBeginInfo::builder(),
            )?;

            if let Some(debug_util) = &self.device.instance.debug_utils {
                debug_util.cmd_begin_label(
                    transfer_frame.command_buffer,
                    "Device Upload Pass",
                    [0.5, 0.0, 0.5, 1.0],
                );
            }

            //TODO: Proper Barriers and Updating resource accesses
            self.device.core.cmd_pipeline_barrier2(
                transfer_frame.command_buffer,
                &vk::DependencyInfo::builder().memory_barriers(&[vk::MemoryBarrier2::builder()
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                    .build()]),
            );

            record_transfers(
                &self.device,
                transfer_frame.command_buffer,
                resource_manager,
                &self.transfers,
            );
            self.transfers.clear();

            self.device.core.cmd_pipeline_barrier2(
                transfer_frame.command_buffer,
                &vk::DependencyInfo::builder().memory_barriers(&[vk::MemoryBarrier2::builder()
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
                    .build()]),
            );

            if let Some(debug_util) = &self.device.instance.debug_utils {
                debug_util.cmd_end_label(transfer_frame.command_buffer);
            }

            self.device
                .core
                .end_command_buffer(transfer_frame.command_buffer)?;

            let command_buffer_info = &[vk::CommandBufferSubmitInfo::builder()
                .command_buffer(transfer_frame.command_buffer)
                .build()];

            let submit_info = vk::SubmitInfo2::builder().command_buffer_infos(command_buffer_info);
            self.device
                .core
                .queue_submit2(
                    self.queue.handle,
                    &[submit_info.build()],
                    transfer_frame.transfer_done_fence,
                )
                .unwrap();
        }

        Ok(())
    }
}

impl Drop for TransferQueue {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.core.device_wait_idle();
            self.device
                .core
                .destroy_command_pool(self.command_pool, None);
            for semaphore in self.transfer_frames.drain(..) {
                self.device
                    .core
                    .destroy_fence(semaphore.transfer_done_fence, None);
            }
        }
    }
}

fn record_transfers(
    device: &AshDevice,
    command_buffer: vk::CommandBuffer,
    resource_manager: &ResourceManager,
    transfers: &[Transfer],
) {
    for transfer in transfers.iter() {
        match transfer {
            Transfer::CopyBufferToBuffer {
                src,
                dst,
                copy_size,
            } => {
                let src_buffer = resource_manager
                    .get_buffer(buffer_handle_to_key(src.buffer))
                    .unwrap();
                let dst_buffer = resource_manager
                    .get_buffer(buffer_handle_to_key(dst.buffer))
                    .unwrap();
                unsafe {
                    device.core.cmd_copy_buffer2(
                        command_buffer,
                        &vk::CopyBufferInfo2::builder()
                            .src_buffer(src_buffer.handle)
                            .dst_buffer(dst_buffer.handle)
                            .regions(&[vk::BufferCopy2::builder()
                                .src_offset(src.offset as vk::DeviceSize)
                                .dst_offset(dst.offset as vk::DeviceSize)
                                .size(*copy_size as vk::DeviceSize)
                                .build()]),
                    );
                }
            }
            Transfer::CopyBufferToImage {
                src,
                dst,
                copy_size,
            } => {
                let src_buffer = resource_manager
                    .get_buffer(buffer_handle_to_key(src.buffer))
                    .unwrap();
                let dst_image = resource_manager
                    .get_image(image_handle_to_key(dst.image))
                    .unwrap();
                unsafe {
                    device.core.cmd_copy_buffer_to_image2(
                        command_buffer,
                        &vk::CopyBufferToImageInfo2::builder()
                            .src_buffer(src_buffer.handle)
                            .dst_image(dst_image.handle)
                            .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                            .regions(&[vk::BufferImageCopy2::builder()
                                .buffer_row_length(src.row_length.unwrap_or_default())
                                .buffer_image_height(src.row_height.unwrap_or_default())
                                .buffer_offset(src.offset)
                                .image_offset(vk::Offset3D {
                                    x: dst.offset[0] as i32,
                                    y: dst.offset[1] as i32,
                                    z: 0,
                                })
                                .image_extent(vk::Extent3D {
                                    width: copy_size[0],
                                    height: copy_size[1],
                                    depth: 1,
                                })
                                .image_subresource(vk::ImageSubresourceLayers {
                                    aspect_mask: vk_format_get_aspect_flags(dst_image.format),
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                })
                                .build()]),
                    );
                }
            }
            Transfer::CopyImageToBuffer {
                src,
                dst,
                copy_size,
            } => {
                let src_image = resource_manager
                    .get_image(image_handle_to_key(src.image))
                    .unwrap();
                let dst_buffer = resource_manager
                    .get_buffer(buffer_handle_to_key(dst.buffer))
                    .unwrap();
                unsafe {
                    device.core.cmd_copy_image_to_buffer2(
                        command_buffer,
                        &vk::CopyImageToBufferInfo2::builder()
                            .src_image(src_image.handle)
                            .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                            .dst_buffer(dst_buffer.handle)
                            .regions(&[vk::BufferImageCopy2::builder()
                                .buffer_row_length(dst.row_length.unwrap_or_default())
                                .buffer_image_height(dst.row_height.unwrap_or_default())
                                .buffer_offset(dst.offset)
                                .image_offset(vk::Offset3D {
                                    x: src.offset[0] as i32,
                                    y: src.offset[1] as i32,
                                    z: 0,
                                })
                                .image_extent(vk::Extent3D {
                                    width: copy_size[0],
                                    height: copy_size[1],
                                    depth: 1,
                                })
                                .image_subresource(vk::ImageSubresourceLayers {
                                    aspect_mask: vk_format_get_aspect_flags(src_image.format),
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                })
                                .build()]),
                    );
                }
            }
            Transfer::CopyImageToImage {
                src,
                dst,
                copy_size,
            } => {
                let src_image = resource_manager
                    .get_image(image_handle_to_key(src.image))
                    .unwrap();
                let dst_image = resource_manager
                    .get_image(image_handle_to_key(dst.image))
                    .unwrap();
                unsafe {
                    device.core.cmd_copy_image2(
                        command_buffer,
                        &vk::CopyImageInfo2::builder()
                            .src_image(src_image.handle)
                            .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                            .dst_image(dst_image.handle)
                            .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                            .regions(&[vk::ImageCopy2::builder()
                                .src_offset(vk::Offset3D {
                                    x: src.offset[0] as i32,
                                    y: src.offset[1] as i32,
                                    z: 0,
                                })
                                .dst_offset(vk::Offset3D {
                                    x: dst.offset[0] as i32,
                                    y: dst.offset[1] as i32,
                                    z: 0,
                                })
                                .extent(vk::Extent3D {
                                    width: copy_size[0],
                                    height: copy_size[1],
                                    depth: 1,
                                })
                                .src_subresource(vk::ImageSubresourceLayers {
                                    aspect_mask: vk_format_get_aspect_flags(src_image.format),
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                })
                                .dst_subresource(vk::ImageSubresourceLayers {
                                    aspect_mask: vk_format_get_aspect_flags(dst_image.format),
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                })
                                .build()]),
                    )
                }
            }
        }
    }
}

fn buffer_handle_to_key(handle: BufferHandle) -> BufferKey {
    match handle {
        BufferHandle::Persistent(key) => key,
        BufferHandle::Transient(_) => panic!("Transient not supported"),
    }
}
fn image_handle_to_key(handle: ImageHandle) -> ImageKey {
    match handle {
        ImageHandle::Persistent(key) => key,
        ImageHandle::Transient(_) => panic!("Transient not supported"),
    }
}
