use crate::DeviceTrait;

//TODO: Should RenderGraphBuilder just be a trait, letting is manage internally best for that backend
pub struct RenderGraphBuilderImpl<T: DeviceTrait> {
    //Just used so the compiler doesn't complain about unused generic type
    used_buffers: Vec<T::Buffer>,
}

impl<T: DeviceTrait> Default for RenderGraphBuilderImpl<T> {
    fn default() -> Self {
        Self {
            used_buffers: vec![],
        }
    }
}

impl<T: DeviceTrait> RenderGraphBuilderImpl<T> {
    pub fn transfer_buffer_to_buffer(
        &mut self,
        src: T::Buffer,
        src_offset: usize,
        dst: T::Buffer,
        dst_offset: usize,
        size: usize,
    ) {
        self.used_buffers.push(src);
        self.used_buffers.push(dst);
        let _ = src_offset;
        let _ = dst_offset;
        let _ = size;
    }
}

pub trait RenderGraphBuilderTrait {
    type Device: DeviceTrait;
    type Buffer: Sync + Clone;
    type Texture: Sync + Clone;
    type Sampler: Sync + Clone;

    fn transfer_buffer_to_buffer(
        &mut self,
        src: Self::Buffer,
        src_offset: usize,
        dst: Self::Buffer,
        dst_offset: usize,
        size: usize,
    );
}
