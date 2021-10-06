use crate::image::Image;
use ash::vk;

pub struct CommandBuffer {
    pub(crate) device: ash::Device,
    pub(crate) command_buffer: vk::CommandBuffer,
}

//TODO: implement useful commands? Use wgpu as ref mabye?
impl CommandBuffer {
    pub fn clear_color_image(&mut self, image: &mut Image, color: &[f32; 4]) {
        //TODO: get correct ImageLayout and ImageSubresourceRange
        unsafe {
            self.device.cmd_clear_color_image(
                self.command_buffer,
                image.image,
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue {
                    float32: color.clone(),
                },
                &[vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .layer_count(1)
                    .base_mip_level(0)
                    .level_count(1)
                    .build()],
            );
        }
    }

    pub fn blit_image(&mut self, src_image: &Image, dst_image: &mut Image) {
        //TODO: get correct ImageLayout and ImageSubresourceRange
        let basic_subresource_layer = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        let zero_offset = vk::Offset3D { x: 0, y: 0, z: 0 };

        unsafe {
            self.device.cmd_blit_image(
                self.command_buffer,
                src_image.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                dst_image.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::ImageBlit::builder()
                    .src_subresource(basic_subresource_layer)
                    .src_offsets([
                        zero_offset,
                        vk::Offset3D {
                            x: src_image.size.width as i32,
                            y: src_image.size.height as i32,
                            z: 0,
                        },
                    ])
                    .dst_subresource(basic_subresource_layer)
                    .dst_offsets([
                        zero_offset,
                        vk::Offset3D {
                            x: dst_image.size.width as i32,
                            y: dst_image.size.height as i32,
                            z: 0,
                        },
                    ])
                    .build()],
                vk::Filter::NEAREST,
            );
        }
    }
}
