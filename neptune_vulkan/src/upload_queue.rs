use crate::render_graph::{
    BufferBarrier, BufferBarrierSource, BufferGraphResource, BufferIndex, BufferOffset,
    BufferResourceDescription, CommandBuffer, ImageBarrier, ImageBarrierSource, ImageCopyBuffer,
    ImageCopyImage, ImageGraphResource, ImageIndex, ImageResourceDescription, Queue, RenderPass,
    RenderPassCommand, RenderPassSet, Transfer,
};
use crate::resource_managers::{BufferResourceAccess, ImageResourceAccess};
use crate::{BufferHandle, ImageHandle};

//TODO: switch to new pass system
pub(crate) struct UploadPass {
    pub(crate) buffer_resources: Vec<BufferGraphResource>,
    pub(crate) image_resources: Vec<ImageGraphResource>,
    pub(crate) command_buffer: CommandBuffer,
}

#[derive(Default)]
pub(crate) struct UploadQueue {
    buffer_resources: Vec<BufferGraphResource>,
    image_resources: Vec<ImageGraphResource>,

    buffer_access: Vec<(BufferIndex, BufferResourceAccess)>,
    image_access: Vec<(ImageIndex, ImageResourceAccess)>,
    transfers: Vec<Transfer>,
}

impl UploadQueue {
    fn add_buffer(&mut self, buffer: BufferHandle, access: BufferResourceAccess) -> BufferIndex {
        let index = self.buffer_resources.len();
        self.buffer_resources.push(BufferGraphResource {
            description: BufferResourceDescription::Persistent(buffer.as_key()),
            last_access: access,
        });
        self.buffer_access.push((index, access));
        index
    }

    fn add_image(&mut self, image: ImageHandle, access: ImageResourceAccess) -> ImageIndex {
        let index = self.image_resources.len();
        self.image_resources.push(ImageGraphResource {
            description: ImageResourceDescription::Persistent(image.as_key()),
            first_access: None,
            last_access: Some(access),
        });
        self.image_access.push((index, access));
        index
    }

    pub(crate) fn add_buffer_upload(
        &mut self,
        src: crate::render_graph_builder::BufferOffset,
        dst: crate::render_graph_builder::BufferOffset,
        copy_size: usize,
    ) {
        let src = BufferOffset {
            buffer: self.add_buffer(src.buffer, BufferResourceAccess::TransferRead),
            offset: src.offset as u64,
        };

        let dst = BufferOffset {
            buffer: self.add_buffer(dst.buffer, BufferResourceAccess::TransferWrite),
            offset: dst.offset as u64,
        };

        self.transfers.push(Transfer::BufferToBuffer {
            src,
            dst,
            copy_size: copy_size as u64,
        })
    }

    pub(crate) fn add_image_upload(
        &mut self,
        src: crate::render_graph_builder::ImageCopyBuffer,
        dst: crate::render_graph_builder::ImageCopyImage,
        copy_size: [u32; 2],
    ) {
        let src = ImageCopyBuffer {
            buffer: self.add_buffer(src.buffer, BufferResourceAccess::TransferRead),
            offset: src.offset,
            row_length: src.row_length,
            row_height: src.row_height,
        };

        let dst = ImageCopyImage {
            image: self.add_image(dst.image, ImageResourceAccess::TransferWrite),
            offset: dst.offset,
        };

        self.transfers.push(Transfer::BufferToImage {
            src,
            dst,
            copy_size,
        });
    }

    pub(crate) fn get_pass(&mut self) -> Option<UploadPass> {
        if self.transfers.is_empty() {
            None
        } else {
            let buffer_barriers = self
                .buffer_resources
                .iter()
                .enumerate()
                .map(|(index, _buffer)| BufferBarrier {
                    index,
                    src: BufferBarrierSource::FirstUsage,
                    dst: BufferResourceAccess::TransferWrite,
                })
                .collect();

            let image_barriers = self
                .image_resources
                .iter()
                .enumerate()
                .map(|(index, _image)| ImageBarrier {
                    index,
                    src: ImageBarrierSource::FirstUsage,
                    dst: ImageResourceAccess::TransferWrite,
                })
                .collect();

            Some(UploadPass {
                buffer_resources: std::mem::take(&mut self.buffer_resources),
                image_resources: std::mem::take(&mut self.image_resources),
                command_buffer: CommandBuffer {
                    queue: Queue::Graphics,
                    command_buffer_wait_dependencies: Vec::new(),
                    render_pass_sets: vec![RenderPassSet {
                        memory_barriers: vec![],
                        buffer_barriers,
                        image_barriers,
                        render_passes: vec![RenderPass {
                            label_name: "Device Upload Pass".to_string(),
                            label_color: [0.5, 0.0, 0.5, 1.0],
                            command: Some(RenderPassCommand::Transfer {
                                transfers: std::mem::take(&mut self.transfers),
                            }),
                        }],
                    }],
                    command_buffer_signal_dependencies: Vec::new(),
                },
            })
        }
    }
}
