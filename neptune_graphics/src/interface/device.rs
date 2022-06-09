use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};

pub struct Resource {
    id: usize,
    deleted_list: Arc<Mutex<Vec<usize>>>,
}

impl Drop for Resource {
    fn drop(&mut self) {
        self.deleted_list.borrow_mut().lock().unwrap().push(self.id)
    }
}
pub type ShaderModule = Resource;

// TODO: Use From<> Trait instead????
/// A buffer type that can be bound for render graph passes
pub trait BufferGraphResource {
    fn get_handle(&self) -> usize;
}

// Buffer type can't be bound for rendering since it must be imported into render graph for synchronization
pub struct Buffer(Resource);

// StaticBuffer is filled during creation and will be immutable, therefore no synchronization is required
pub struct StaticBuffer(Resource);
impl BufferGraphResource for StaticBuffer {
    fn get_handle(&self) -> usize {
        self.0.id
    }
}

pub type Texture = Resource;

pub trait Device {
    fn create_shader_module() -> Arc<ShaderModule>;
    fn create_buffer() -> Arc<Buffer>;
    fn create_texture() -> Arc<Texture>;

    //TODO: Should this also/only have an async version?
    fn create_static_buffer<T>(data: &[T]) -> Arc<StaticBuffer>;
    //async fn async_create_static_buffer<T>(data: &[T]) -> Arc<Buffer>;
    fn create_static_texture<T>(data: &[T]) -> Arc<Texture>;
    //async fn async_create_static_texture<T>(data: &[T]) -> Arc<Texture>;
}
