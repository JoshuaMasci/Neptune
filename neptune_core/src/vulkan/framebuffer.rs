use crate::render_backend::RenderDevice;
use crate::vulkan::image::Image;
use crate::vulkan::ImageDescription;
use ash::vk;
use ash::vk::AttachmentReference;
use std::rc::Rc;

pub struct FrameBufferSet {
    device: RenderDevice,
    color_formats: Vec<vk::Format>,
    depth_stencil_format: Option<vk::Format>,

    pub(crate) render_pass: vk::RenderPass,
    pub framebuffers: Vec<Framebuffer>,

    pub(crate) current_size: vk::Extent2D,
}

impl FrameBufferSet {
    pub(crate) fn new(
        device: &RenderDevice,
        size: vk::Extent2D,
        color_formats: Vec<vk::Format>,
        depth_stencil_format: Option<vk::Format>,
        count: usize,
    ) -> Self {
        let mut attachments: Vec<vk::AttachmentDescription> = Vec::new();
        let mut references: Vec<AttachmentReference> = Vec::new();
        for (i, &color_format) in color_formats.iter().enumerate() {
            attachments.push(vk::AttachmentDescription {
                flags: Default::default(),
                format: color_format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            });
            references.push(
                vk::AttachmentReference::builder()
                    .attachment(i as u32)
                    .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .build(),
            );
        }

        if let Some(depth_format) = &depth_stencil_format {
            attachments.push(vk::AttachmentDescription {
                flags: Default::default(),
                format: *depth_format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::DONT_CARE,
                store_op: vk::AttachmentStoreOp::DONT_CARE,
                stencil_load_op: vk::AttachmentLoadOp::CLEAR,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::UNDEFINED,
            });
            references.push(
                vk::AttachmentReference::builder()
                    .attachment(references.len() as u32)
                    .layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                    .build(),
            );
        }

        let render_pass = unsafe {
            device.base.create_render_pass(
                &vk::RenderPassCreateInfo::builder()
                    .attachments(&attachments)
                    .subpasses(&[vk::SubpassDescription::builder()
                        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                        .color_attachments(&references)
                        .build()]),
                None,
            )
        }
        .expect("Failed to create swapchain render pass");

        let framebuffers: Vec<Framebuffer> = (0..count)
            .map(|_| {
                Framebuffer::new(
                    &device,
                    render_pass,
                    size,
                    color_formats.clone(),
                    depth_stencil_format.clone(),
                )
            })
            .collect();

        Self {
            device: device.clone(),
            color_formats,
            depth_stencil_format,
            render_pass,
            framebuffers,
            current_size: size,
        }
    }

    pub(crate) fn set_size(&mut self, new_size: vk::Extent2D) {
        self.current_size = new_size;
    }

    pub(crate) fn update_frame(&mut self, frame_index: usize) {
        if self.framebuffers[frame_index].size != self.current_size {
            self.framebuffers[frame_index] = Framebuffer::new(
                &self.device,
                self.render_pass,
                self.current_size,
                self.color_formats.clone(),
                self.depth_stencil_format,
            );
        }
    }
}

impl Drop for FrameBufferSet {
    fn drop(&mut self) {
        let _ = unsafe {
            self.device.base.destroy_render_pass(self.render_pass, None);
        };
    }
}

pub struct Framebuffer {
    device: Rc<ash::Device>,
    size: vk::Extent2D,
    pub color_attachments: Vec<Image>,
    depth_attachment: Option<Image>,
    pub(crate) handle: vk::Framebuffer,
}

impl Framebuffer {
    pub(crate) fn new(
        device: &RenderDevice,
        render_pass: vk::RenderPass,
        size: vk::Extent2D,
        color_formats: Vec<vk::Format>,
        depth_stencil_format: Option<vk::Format>,
    ) -> Self {
        let color_attachments: Vec<Image> = color_formats
            .iter()
            .map(|&format| {
                let mut image = Image::new(
                    device,
                    ImageDescription {
                        format,
                        size: [size.width, size.height],
                        usage: vk::ImageUsageFlags::COLOR_ATTACHMENT
                            | vk::ImageUsageFlags::TRANSFER_SRC,
                        memory_location: gpu_allocator::MemoryLocation::GpuOnly,
                    },
                );
                image.create_image_view();
                image
            })
            .collect();

        let depth_attachment = depth_stencil_format.map(|depth_format| {
            let mut image = Image::new(
                device,
                ImageDescription {
                    format: depth_format,
                    size: [size.width, size.height],
                    usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                    memory_location: gpu_allocator::MemoryLocation::GpuOnly,
                },
            );
            image.create_image_view();
            image
        });

        let mut image_views: Vec<vk::ImageView> =
            color_attachments.iter().map(|image| image.view).collect();

        if let Some(depth_image) = &depth_attachment {
            image_views.push(depth_image.view);
        }

        let handle = unsafe {
            device.base.create_framebuffer(
                &vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .width(size.width)
                    .height(size.height)
                    .layers(1)
                    .attachments(&image_views),
                None,
            )
        }
        .expect("Failed to create framebuffer");

        Self {
            device: device.base.clone(),
            size,
            color_attachments,
            depth_attachment,
            handle,
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        let _ = unsafe {
            self.device.destroy_framebuffer(self.handle, None);
        };
    }
}
