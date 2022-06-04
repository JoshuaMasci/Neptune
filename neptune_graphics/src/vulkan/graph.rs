use crate::render_graph::{
    BufferAccess, BufferResourceDescription, RenderPass, RenderPassData, TextureAccess,
    TextureResourceDescription,
};
use crate::resource::Resource;
use crate::vulkan::pipeline_cache::{FramebufferLayout, PipelineCache};
use crate::vulkan::{Buffer, Device, ShaderModule, Texture, VulkanRasterCommandBuffer};
use crate::{BufferDescription, TextureDimensions, UploadData};
use ash::vk;
use std::cmp::min;
use std::rc::Rc;

pub type RasterFnVulkan = dyn FnOnce(&mut VulkanRasterCommandBuffer);

enum PassData {
    None,
    BufferCopy {
        src_buffer: vk::Buffer,
        src_offset: usize,
        dst_buffer: vk::Buffer,
        dst_offset: usize,
        copy_size: usize,
    },
    Raster {
        render_area: vk::Rect2D,
        color_attachments: Vec<vk::RenderingAttachmentInfoKHR>,
        depth_stencil_attachment: Option<vk::RenderingAttachmentInfoKHR>,
        pipelines: Vec<(
            vk::Pipeline,
            Box<RasterFnVulkan>,
            Rc<ShaderModule>,
            Option<Rc<ShaderModule>>,
        )>,
    },
    Compute {
        pipeline: vk::Pipeline,
        dispatch_size: [u32; 3],
    },
    // Raytrace,
    // Custom,
}

struct Pass {
    pub id: usize,
    pub data: PassData,
}

impl Pass {
    fn from(
        render_pass: RenderPass,
        buffers: &[BufferStorage],
        textures: &[TextureStorage],
        pipeline_cache: &mut PipelineCache,
    ) -> Self {
        Self {
            id: render_pass.id,
            data: match render_pass.data {
                RenderPassData::BufferUpload {
                    src_buffer,
                    src_data,
                    dst_buffer,
                    dst_offset,
                } => {
                    match src_data {
                        UploadData::U8(data) => {
                            buffers[src_buffer as usize].fill_cpu_visible(&data);
                        }
                        UploadData::F32(data) => {
                            buffers[src_buffer as usize].fill_cpu_visible(&data);
                        }
                        UploadData::U32(data) => {
                            buffers[src_buffer as usize].fill_cpu_visible(&data);
                        }
                    }

                    let copy_size = min(
                        buffers[src_buffer as usize].get_size(),
                        buffers[dst_buffer as usize].get_size() - dst_offset,
                    );

                    PassData::BufferCopy {
                        src_buffer: buffers[src_buffer as usize].get_handle(),
                        src_offset: 0,
                        dst_buffer: buffers[dst_buffer as usize].get_handle(),
                        dst_offset,
                        copy_size,
                    }
                }
                RenderPassData::Raster {
                    color_attachments,
                    depth_stencil_attachment,
                    mut pipelines,
                } => {
                    let framebuffer_layout = FramebufferLayout {
                        color_attachments: color_attachments
                            .iter()
                            .map(|attachment| textures[attachment.id].get_texture_format())
                            .collect(),
                        depth_stencil_attachment: depth_stencil_attachment
                            .as_ref()
                            .map(|attachment| textures[attachment.id].get_texture_format()),
                    };

                    let framebuffer_size: [u32; 2] = {
                        let mut framebuffer_size: Option<[u32; 2]> = None;
                        for attachment in color_attachments.iter() {
                            let attachment_size =
                                textures[attachment.id].get_texture_size().expect_2d();
                            if let Some(size) = framebuffer_size {
                                if size != attachment_size {
                                    panic!(
                                        "Color attachment size doesn't match rest of framebuffer"
                                    );
                                }
                            } else {
                                framebuffer_size = Some(attachment_size);
                            }
                        }

                        if let Some(attachment) = &depth_stencil_attachment {
                            let attachment_size =
                                textures[attachment.id].get_texture_size().expect_2d();
                            if let Some(size) = framebuffer_size {
                                if size != attachment_size {
                                    panic!(
                                        "Depth stencil attachment size doesn't match rest of framebuffer"
                                    );
                                }
                            } else {
                                framebuffer_size = Some(attachment_size);
                            }
                        }

                        framebuffer_size.expect("No textures found for framebuffer")
                    };

                    let color_attachments = color_attachments
                        .iter()
                        .map(|attachment| {
                            vk::RenderingAttachmentInfoKHR::builder()
                                .image_view(textures[attachment.id].get_texture_view())
                                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                                .load_op(match attachment.clear {
                                    None => vk::AttachmentLoadOp::LOAD,
                                    Some(_) => vk::AttachmentLoadOp::CLEAR,
                                })
                                .store_op(vk::AttachmentStoreOp::STORE)
                                .clear_value(vk::ClearValue {
                                    color: vk::ClearColorValue {
                                        float32: attachment.clear.unwrap_or_default(),
                                    },
                                })
                                .build()
                        })
                        .collect();

                    let depth_stencil_attachment = depth_stencil_attachment.map(|attachment| {
                        let clear = attachment.clear.unwrap_or_default();

                        vk::RenderingAttachmentInfoKHR::builder()
                            .image_view(textures[attachment.id].get_texture_view())
                            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                            .load_op(match attachment.clear {
                                None => vk::AttachmentLoadOp::LOAD,
                                Some(_) => vk::AttachmentLoadOp::CLEAR,
                            })
                            .store_op(vk::AttachmentStoreOp::STORE)
                            .clear_value(vk::ClearValue {
                                depth_stencil: vk::ClearDepthStencilValue {
                                    depth: clear.0,
                                    stencil: clear.1,
                                },
                            })
                            .build()
                    });

                    let pipelines = pipelines
                        .drain(..)
                        .map(|pipeline_description| {
                            (
                                pipeline_cache.get_graphics(
                                    pipeline_description.vertex_module.clone(),
                                    pipeline_description.fragment_module.clone(),
                                    pipeline_description.vertex_elements,
                                    pipeline_description.pipeline_state,
                                    framebuffer_layout.clone(),
                                ),
                                pipeline_description.raster_fn,
                                pipeline_description.vertex_module,
                                pipeline_description.fragment_module,
                            )
                        })
                        .collect();

                    PassData::Raster {
                        render_area: vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: vk::Extent2D {
                                width: framebuffer_size[0],
                                height: framebuffer_size[1],
                            },
                        },
                        color_attachments,
                        depth_stencil_attachment,
                        pipelines,
                    }
                }
            },
        }
    }
}

#[derive(Default)]
pub(crate) struct PassSetBarrier {
    pub(crate) memory_barriers: Vec<vk::MemoryBarrier2>,
    pub(crate) buffer_barriers: Vec<vk::BufferMemoryBarrier2>,
    pub(crate) image_barriers: Vec<vk::ImageMemoryBarrier2>,
}

impl PassSetBarrier {
    pub(crate) fn record(&self, device: &Rc<ash::Device>, command_buffer: vk::CommandBuffer) {
        unsafe {
            device.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfo::builder()
                    .memory_barriers(&self.memory_barriers)
                    .buffer_memory_barriers(&self.buffer_barriers)
                    .image_memory_barriers(&self.image_barriers)
                    .build(),
            );
        }
    }
}

pub(crate) enum BufferStorage {
    Unused,
    Temporary(Resource<Buffer>),
    Imported(Rc<Resource<Buffer>>),
}

impl BufferStorage {
    pub(crate) fn fill_cpu_visible<T>(&self, data: &[T]) {
        match self {
            BufferStorage::Unused => panic!("Tried to access Unused Buffer"),
            BufferStorage::Temporary(buffer) => buffer.fill(data),
            BufferStorage::Imported(buffer) => buffer.fill(data),
        }
    }

    pub(crate) fn get_handle(&self) -> vk::Buffer {
        match self {
            BufferStorage::Unused => panic!("Tried to access Unused Buffer"),
            BufferStorage::Temporary(buffer) => buffer.handle,
            BufferStorage::Imported(buffer) => buffer.handle,
        }
    }

    pub(crate) fn get_size(&self) -> usize {
        match self {
            BufferStorage::Unused => panic!("Tried to access Unused Buffer"),
            BufferStorage::Temporary(buffer) => buffer.description.size,
            BufferStorage::Imported(buffer) => buffer.description.size,
        }
    }
}

pub(crate) enum TextureStorage {
    Unused,
    Swapchain(vk::Format, vk::Image, vk::ImageView, TextureDimensions),
    Temporary(Resource<Texture>),
    Imported(Rc<Resource<Texture>>),
}

impl TextureStorage {
    fn get_texture_size(&self) -> TextureDimensions {
        match self {
            TextureStorage::Unused => panic!("Tried to access Unused Texture"),
            TextureStorage::Swapchain(_, _, _, dimensions) => *dimensions,
            TextureStorage::Temporary(texture) => texture.description.size,
            TextureStorage::Imported(texture) => texture.description.size,
        }
    }

    fn get_texture_format(&self) -> vk::Format {
        match self {
            TextureStorage::Unused => panic!("Tried to access Unused Texture"),
            TextureStorage::Swapchain(format, _, _, _) => *format,
            TextureStorage::Temporary(texture) => texture.format,
            TextureStorage::Imported(texture) => texture.format,
        }
    }

    pub(crate) fn get_handle(&self) -> vk::Image {
        match self {
            TextureStorage::Unused => panic!("Tried to access Unused Texture"),
            TextureStorage::Swapchain(_, handle, _, _) => *handle,
            TextureStorage::Temporary(texture) => texture.handle,
            TextureStorage::Imported(texture) => texture.handle,
        }
    }

    fn get_texture_view(&self) -> vk::ImageView {
        match self {
            TextureStorage::Unused => panic!("Tried to access Unused Texture"),
            TextureStorage::Swapchain(_, _, view, _) => *view,
            TextureStorage::Temporary(texture) => texture.view,
            TextureStorage::Imported(texture) => texture.view,
        }
    }

    fn get_texture_range(&self) -> vk::ImageSubresourceRange {
        match self {
            TextureStorage::Unused => panic!("Tried to access Unused Texture"),
            TextureStorage::Swapchain(_, _, _, _) => vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            TextureStorage::Temporary(texture) => texture.sub_resource_range,
            TextureStorage::Imported(texture) => texture.sub_resource_range,
        }
    }
}

#[derive(Default)]
struct PassSet {
    pre_barrier: PassSetBarrier,
    passes: Vec<Pass>,
    post_barrier: PassSetBarrier,
}

#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct Graph {
    buffers: Vec<BufferStorage>,
    textures: Vec<TextureStorage>,
    sets: Vec<PassSet>,

    temp_final_swapchain_layout: vk::ImageLayout,
}

impl Graph {
    pub(crate) fn new(
        device: &mut crate::vulkan::Device,
        swapchain_image: (vk::Format, vk::Image, vk::ImageView, TextureDimensions),
        mut render_graph: crate::render_graph::RenderGraphBuilder,
    ) -> Self {
        let buffers: Vec<BufferStorage> = render_graph
            .buffers
            .iter()
            .map(|buffer| {
                if buffer.access_list.is_empty() {
                    BufferStorage::Unused
                } else {
                    match &buffer.description {
                        BufferResourceDescription::New(description) => {
                            BufferStorage::Temporary(device.create_buffer(*description))
                        }
                        BufferResourceDescription::Imported(buffer) => {
                            BufferStorage::Imported(buffer.clone())
                        }
                    }
                }
            })
            .collect();

        let textures: Vec<TextureStorage> = render_graph
            .textures
            .iter()
            .map(|texture| {
                if texture.access_list.is_empty() {
                    TextureStorage::Unused
                } else {
                    match &texture.description {
                        TextureResourceDescription::Swapchain(_swapchain_id) => {
                            TextureStorage::Swapchain(
                                swapchain_image.0,
                                swapchain_image.1,
                                swapchain_image.2,
                                swapchain_image.3,
                            )
                        }
                        TextureResourceDescription::New(description) => {
                            TextureStorage::Temporary(device.create_texture(*description))
                        }
                        TextureResourceDescription::Imported(texture) => {
                            TextureStorage::Imported(texture.clone())
                        }
                    }
                }
            })
            .collect();

        //TODO: make better
        //WARNING: Bad implementation of barriers
        //This assumes that the execution order is the same as the submission order, rather than a traditional RenderGraph which can reorder itself when needed.
        //Also this produces too many barriers as it produces many barriers in the Case: WRITE -> MANY READ or MANY READ -> WRITE where it can be done with only 1 barrier
        let mut last_access_buffer: Vec<BufferAccess> = render_graph
            .buffers
            .iter()
            .map(|_buffer| BufferAccess::None)
            .collect();

        let mut last_access_texture: Vec<TextureAccess> = render_graph
            .textures
            .iter()
            .map(|_texture| TextureAccess::None)
            .collect();

        let sets: Vec<PassSet> = render_graph
            .passes
            .drain(..)
            .map(|pass| PassSet {
                pre_barrier: PassSetBarrier {
                    memory_barriers: vec![],
                    buffer_barriers: pass
                        .buffer_accesses
                        .iter()
                        .map(|(id, access)| {
                            let last = last_access_buffer[*id];
                            last_access_buffer[*id] = *access;

                            let src = last.get_vk();
                            let dst = access.get_vk();

                            vk::BufferMemoryBarrier2::builder()
                                .buffer(buffers[*id].get_handle())
                                .offset(0)
                                .size(vk::WHOLE_SIZE)
                                .src_stage_mask(src.0)
                                .src_access_mask(src.1)
                                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                                .dst_stage_mask(dst.0)
                                .dst_access_mask(dst.1)
                                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                                .build()
                        })
                        .collect(),
                    image_barriers: pass
                        .texture_accesses
                        .iter()
                        .map(|(id, access)| {
                            let last = last_access_texture[*id];
                            last_access_texture[*id] = *access;

                            let src = last.get_vk();
                            let dst = access.get_vk();

                            vk::ImageMemoryBarrier2::builder()
                                .image(textures[*id].get_handle())
                                .subresource_range(textures[*id].get_texture_range())
                                .old_layout(src.2)
                                .src_stage_mask(src.0)
                                .src_access_mask(src.1)
                                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                                .new_layout(dst.2)
                                .dst_stage_mask(dst.0)
                                .dst_access_mask(dst.1)
                                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                                .build()
                        })
                        .collect(),
                },
                passes: vec![Pass::from(
                    pass,
                    &buffers,
                    &textures,
                    &mut device.pipeline_cache,
                )],
                post_barrier: Default::default(),
            })
            .collect();

        Self {
            buffers,
            textures,
            sets,
            temp_final_swapchain_layout: last_access_texture[0].get_vk().2,
        }
    }

    pub(crate) fn record_command_buffer(
        &mut self,
        device: &Rc<ash::Device>,
        command_buffer: vk::CommandBuffer,
        pipeline_layout: vk::PipelineLayout,
        descriptor_set: vk::DescriptorSet,
    ) -> vk::ImageLayout {
        for set in self.sets.iter_mut() {
            set.pre_barrier.record(device, command_buffer);

            for pass in set.passes.iter_mut() {
                //println!("Render Pass {}: {}", pass.id, pass.name);
                match &mut pass.data {
                    PassData::None => {}
                    PassData::BufferCopy {
                        src_buffer,
                        src_offset,
                        dst_buffer,
                        dst_offset,
                        copy_size,
                    } => unsafe {
                        device.cmd_copy_buffer(
                            command_buffer,
                            *src_buffer,
                            *dst_buffer,
                            &[vk::BufferCopy {
                                src_offset: *src_offset as vk::DeviceSize,
                                dst_offset: *dst_offset as vk::DeviceSize,
                                size: *copy_size as vk::DeviceSize,
                            }],
                        );
                    },
                    PassData::Raster {
                        render_area,
                        color_attachments,
                        depth_stencil_attachment,
                        pipelines,
                    } => {
                        let mut rendering_info = vk::RenderingInfoKHR::builder()
                            .render_area(*render_area)
                            .layer_count(1)
                            .color_attachments(color_attachments);
                        if let Some(depth_stencil_attachment) = depth_stencil_attachment {
                            rendering_info =
                                rendering_info.depth_attachment(depth_stencil_attachment);
                            rendering_info =
                                rendering_info.stencil_attachment(depth_stencil_attachment);
                        }

                        unsafe {
                            device.cmd_begin_rendering(command_buffer, &rendering_info);
                        }

                        let raster_command_buffer = &mut VulkanRasterCommandBuffer::new(
                            device.clone(),
                            command_buffer,
                            &self.buffers,
                            &self.textures,
                        );

                        for pipeline in pipelines.drain(..) {
                            unsafe {
                                device.cmd_bind_descriptor_sets(
                                    command_buffer,
                                    vk::PipelineBindPoint::GRAPHICS,
                                    pipeline_layout,
                                    0,
                                    &[descriptor_set],
                                    &[],
                                );

                                device.cmd_bind_pipeline(
                                    command_buffer,
                                    vk::PipelineBindPoint::GRAPHICS,
                                    pipeline.0,
                                );

                                device.cmd_set_viewport(
                                    command_buffer,
                                    0,
                                    &[vk::Viewport {
                                        x: render_area.offset.x as f32,
                                        y: render_area.offset.y as f32,
                                        width: render_area.extent.width as f32,
                                        height: render_area.extent.height as f32,
                                        min_depth: 0.0,
                                        max_depth: 1.0,
                                    }],
                                );
                                device.cmd_set_scissor(command_buffer, 0, &[*render_area]);

                                pipeline.1(raster_command_buffer);
                            }
                        }

                        unsafe {
                            device.cmd_end_rendering(command_buffer);
                        }
                    }
                    PassData::Compute {
                        pipeline,
                        dispatch_size,
                    } => unsafe {
                        //TODO: Push used resource bindings
                        device.cmd_bind_descriptor_sets(
                            command_buffer,
                            vk::PipelineBindPoint::COMPUTE,
                            pipeline_layout,
                            0,
                            &[descriptor_set],
                            &[],
                        );
                        device.cmd_bind_pipeline(
                            command_buffer,
                            vk::PipelineBindPoint::COMPUTE,
                            *pipeline,
                        );
                        device.cmd_dispatch(
                            command_buffer,
                            dispatch_size[0],
                            dispatch_size[1],
                            dispatch_size[2],
                        );
                    },
                }
            }

            set.post_barrier.record(device, command_buffer);
        }
        self.temp_final_swapchain_layout
    }
}
