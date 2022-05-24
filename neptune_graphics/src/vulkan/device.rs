use crate::buffer::BufferDescription;
use crate::resource::{Resource, ResourceDeleter};
use crate::texture::TextureDescription;
use crate::vulkan::buffer::Buffer;
use crate::vulkan::descriptor_set::DescriptorSet;
use crate::vulkan::pipeline_cache::PipelineCache;
use crate::vulkan::shader::ShaderModule;
use crate::vulkan::swapchain::Swapchain;
use crate::vulkan::texture::Texture;
use crate::{BufferUsages, TextureDimensions, TextureUsages};
use ash::vk;
use std::cell::RefCell;
use std::ffi::CStr;
use std::rc::Rc;

struct DeviceDrop(Rc<ash::Device>);
impl DeviceDrop {
    fn new(device: &Rc<ash::Device>) -> Self {
        Self(device.clone())
    }
}

impl Drop for DeviceDrop {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
        }
    }
}

#[derive(Clone)]
struct CommandPool {
    device: Rc<ash::Device>,
    command_pool: vk::CommandPool,
}
impl CommandPool {
    fn new(device: Rc<ash::Device>, graphics_queue_family_index: u32) -> Self {
        let command_pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(graphics_queue_family_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None,
            )
        }
        .expect("Failed to create command pool");

        Self {
            device,
            command_pool,
        }
    }

    fn create_command_buffer(&self) -> vk::CommandBuffer {
        unsafe {
            self.device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(self.command_pool)
                    .command_buffer_count(1)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .build(),
            )
        }
        .expect("Failed to allocate command_buffers")[0]
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}

pub struct Device {
    device: Rc<ash::Device>,
    resource_deleter: ResourceDeleter,
    descriptor_set: DescriptorSet,

    swapchain: Swapchain,

    frame_index: usize,
    frames: Vec<Frame>,

    pipeline_layout: vk::PipelineLayout,
    pipeline_cache: PipelineCache,

    allocator: Rc<RefCell<gpu_allocator::vulkan::Allocator>>,
    device_drop: DeviceDrop,

    //TODO: find better place for this stuff?????
    graphics_queue: vk::Queue,
}

impl Device {
    pub(crate) fn new(
        instance: &ash::Instance,
        surface: vk::SurfaceKHR,
        surface_ext: Rc<ash::extensions::khr::Surface>,
        physical_device: vk::PhysicalDevice,
        graphics_queue_family_index: u32,
        frame_in_flight_count: u32,
    ) -> Self {
        //Device creation
        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe { CStr::from_ptr(device_properties.device_name.as_ptr()) }
            .to_str()
            .expect("Failed to convert CStr to string");

        println!(
            "Device: \n\tName: {}\n\tDriver: {:?}\n\tType: {:?}",
            device_name, device_properties.driver_version, device_properties.device_type,
        );

        let device_extension_names_raw = vec![ash::extensions::khr::Swapchain::name().as_ptr()];

        let mut robustness2_features = vk::PhysicalDeviceRobustness2FeaturesEXT::builder()
            .null_descriptor(true)
            .build();

        let mut vulkan1_2_features = vk::PhysicalDeviceVulkan12Features::builder()
            .descriptor_indexing(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .build();

        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::builder()
                .synchronization2(true)
                .build();

        let mut dynamic_rendering_features =
            vk::PhysicalDeviceDynamicRenderingFeaturesKHR::builder()
                .dynamic_rendering(true)
                .build();

        let priorities = &[1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_family_index)
            .queue_priorities(priorities)
            .build()];

        //Load all device functions
        let device = Rc::new(
            unsafe {
                instance.create_device(
                    physical_device,
                    &vk::DeviceCreateInfo::builder()
                        .queue_create_infos(&queue_info)
                        .enabled_extension_names(&device_extension_names_raw)
                        .push_next(&mut robustness2_features)
                        .push_next(&mut vulkan1_2_features)
                        .push_next(&mut synchronization2_features)
                        .push_next(&mut dynamic_rendering_features),
                    None,
                )
            }
            .expect("Failed to initialize vulkan device"),
        );

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };

        let allocator = Rc::new(RefCell::new(
            gpu_allocator::vulkan::Allocator::new(&gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: (*device).clone(),
                physical_device,
                debug_settings: gpu_allocator::AllocatorDebugSettings {
                    log_memory_information: false,
                    log_leaks_on_shutdown: true,
                    store_stack_traces: false,
                    log_allocations: false,
                    log_frees: false,
                    log_stack_traces: false,
                },
                buffer_device_address: false,
            })
            .expect("Failed to create device allocator"),
        ));

        const RESOURCE_DESCRIPTOR_COUNT: u32 = 2048;
        const SAMPLER_DESCRIPTOR_COUNT: u32 = 128;
        let descriptor_set = DescriptorSet::new(
            device.clone(),
            RESOURCE_DESCRIPTOR_COUNT,
            RESOURCE_DESCRIPTOR_COUNT,
            RESOURCE_DESCRIPTOR_COUNT,
            SAMPLER_DESCRIPTOR_COUNT,
        );

        let resource_deleter = ResourceDeleter::new(frame_in_flight_count as usize);

        let swapchain = Swapchain::new(
            physical_device,
            device.clone(),
            surface_ext,
            surface,
            Rc::new(ash::extensions::khr::Swapchain::new(
                instance,
                device.as_ref(),
            )),
        );

        let command_pool = Rc::new(CommandPool::new(
            device.clone(),
            graphics_queue_family_index,
        ));

        let frames: Vec<Frame> = (0..frame_in_flight_count)
            .map(|_| Frame::new(device.clone(), command_pool.clone()))
            .collect();

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[descriptor_set.get_layout()])
                    .push_constant_ranges(&[vk::PushConstantRange::builder()
                        .size(256)
                        .offset(0)
                        .stage_flags(vk::ShaderStageFlags::ALL)
                        .build()])
                    .build(),
                None,
            )
        }
        .expect("Failed to create pipeline layout");

        let pipeline_cache = PipelineCache::new(device.clone(), pipeline_layout);

        let device_drop = DeviceDrop::new(&device);
        Self {
            device,
            resource_deleter,
            descriptor_set,
            swapchain,
            frame_index: 0,
            frames,
            pipeline_layout,
            pipeline_cache,
            allocator,
            device_drop,
            graphics_queue,
        }
    }

    pub fn create_buffer(&mut self, description: BufferDescription) -> Resource<Buffer> {
        let is_storage = description.usage.contains(BufferUsages::STORAGE);

        let mut buffer = Buffer::new(self.device.clone(), self.allocator.clone(), description);

        if is_storage {
            buffer.binding = Some(self.descriptor_set.bind_storage_buffer(&buffer));
        }

        self.resource_deleter.create_resource(buffer)
    }

    pub fn create_texture(&mut self, description: TextureDescription) -> Resource<Texture> {
        let is_storage = description.usage.contains(TextureUsages::STORAGE);
        let is_sampled = description.usage.contains(TextureUsages::SAMPLED);

        let mut texture = Texture::new(self.device.clone(), self.allocator.clone(), description);

        if is_storage {
            texture.storage_binding = Some(self.descriptor_set.bind_storage_image(&texture));
        }

        if is_sampled {
            texture.sampled_binding = Some(self.descriptor_set.bind_sampled_image(&texture));
        }

        self.resource_deleter.create_resource(texture)
    }

    pub fn create_shader_module(&mut self, code: &[u32]) -> ShaderModule {
        ShaderModule::new(self.device.clone(), code)
    }

    pub fn render(
        &mut self,
        build_render_graph: impl FnOnce(&mut crate::render_graph::RenderGraphBuilder),
    ) {
        unsafe {
            self.device
                .wait_for_fences(
                    &[self.frames[self.frame_index].frame_done_fence],
                    true,
                    u64::MAX,
                )
                .expect("Failed to wait for fence")
        };

        self.resource_deleter.clear_frame();
        self.descriptor_set.commit_changes();

        let swapchain_image_index = self
            .swapchain
            .acquire_next_image(self.frames[self.frame_index].image_ready_semaphore);

        if swapchain_image_index.is_none() {
            return;
        }

        let swapchain_image_index = swapchain_image_index.unwrap();

        unsafe {
            self.device
                .reset_fences(&[self.frames[self.frame_index].frame_done_fence])
                .expect("Failed to reset fence")
        };

        unsafe {
            self.device
                .begin_command_buffer(
                    self.frames[self.frame_index].graphics_command_buffer,
                    &vk::CommandBufferBeginInfo::builder().build(),
                )
                .expect("Failed to begin command buffer recording");
        }

        let swapchain_image = &self.swapchain.images[swapchain_image_index as usize];

        let mut render_graph_builder =
            crate::render_graph::RenderGraphBuilder::new(swapchain_image.size);

        build_render_graph(&mut render_graph_builder);

        let mut render_graph = crate::vulkan::Graph::new(
            self,
            (
                swapchain_image.format,
                swapchain_image.handle,
                swapchain_image.view,
                TextureDimensions::D2(swapchain_image.size[0], swapchain_image.size[1]),
            ),
            render_graph_builder,
        );

        let swapchain_layout = render_graph.record_command_buffer(
            &self.device,
            self.frames[self.frame_index].graphics_command_buffer,
            &mut self.pipeline_cache,
        );

        unsafe {
            let image_memory_barriers = vk::ImageMemoryBarrier2::builder()
                .image(self.swapchain.images[swapchain_image_index as usize].handle)
                .old_layout(swapchain_layout)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .src_access_mask(vk::AccessFlags2::NONE)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_stage_mask(vk::PipelineStageFlags2::NONE)
                .dst_access_mask(vk::AccessFlags2KHR::NONE)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_array_layer(0)
                        .layer_count(1)
                        .base_mip_level(0)
                        .level_count(1)
                        .build(),
                )
                .build();

            self.device.cmd_pipeline_barrier2(
                self.frames[self.frame_index].graphics_command_buffer,
                &vk::DependencyInfo::builder()
                    .image_memory_barriers(&[image_memory_barriers])
                    .build(),
            );
        }

        unsafe {
            self.device
                .end_command_buffer(self.frames[self.frame_index].graphics_command_buffer)
                .expect("Failed to end command buffer recording");
        }

        let wait_semaphore_infos = &[vk::SemaphoreSubmitInfoKHR::builder()
            .semaphore(self.frames[self.frame_index].image_ready_semaphore)
            .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
            .build()];

        let command_buffer_infos = &[vk::CommandBufferSubmitInfoKHR::builder()
            .command_buffer(self.frames[self.frame_index].graphics_command_buffer)
            .build()];

        let signal_semaphore_infos = &[vk::SemaphoreSubmitInfoKHR::builder()
            .semaphore(self.frames[self.frame_index].present_semaphore)
            .stage_mask(vk::PipelineStageFlags2KHR::ALL_COMMANDS)
            .build()];

        let submit_info = vk::SubmitInfo2KHR::builder()
            .wait_semaphore_infos(wait_semaphore_infos)
            .command_buffer_infos(command_buffer_infos)
            .signal_semaphore_infos(signal_semaphore_infos)
            .build();
        unsafe {
            self.device
                .queue_submit2(
                    self.graphics_queue,
                    &[submit_info],
                    self.frames[self.frame_index].frame_done_fence,
                )
                .expect("Failed to queue command buffer");
        }

        self.swapchain.present_image(
            self.graphics_queue,
            swapchain_image_index,
            self.frames[self.frame_index].present_semaphore,
        );

        self.frame_index = (self.frame_index + 1) % self.frames.len();
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            println!("Drop Device!");
            let _ = self.device_drop.0.device_wait_idle();
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

struct Frame {
    command_pool: Rc<CommandPool>,
    device: Rc<ash::Device>,
    frame_done_fence: vk::Fence,
    graphics_command_buffer: vk::CommandBuffer,
    image_ready_semaphore: vk::Semaphore,
    present_semaphore: vk::Semaphore,
}

impl Frame {
    fn new(device: Rc<ash::Device>, command_pool: Rc<CommandPool>) -> Self {
        let frame_done_fence = unsafe {
            device.create_fence(
                &vk::FenceCreateInfo::builder()
                    .flags(vk::FenceCreateFlags::SIGNALED)
                    .build(),
                None,
            )
        }
        .expect("Failed to create fence");

        let image_ready_semaphore =
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::builder().build(), None) }
                .expect("Failed to create semaphore");
        let present_semaphore =
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::builder().build(), None) }
                .expect("Failed to create semaphore");

        let graphics_command_buffer = command_pool.create_command_buffer();

        Self {
            command_pool,
            device,
            frame_done_fence,
            graphics_command_buffer,
            image_ready_semaphore,
            present_semaphore,
        }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_fence(self.frame_done_fence, None);
            self.device
                .destroy_semaphore(self.image_ready_semaphore, None);
            self.device.destroy_semaphore(self.present_semaphore, None);
            let _ = self.command_pool.command_pool;
        }
    }
}
