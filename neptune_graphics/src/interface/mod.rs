use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};

// Welcome to the Prototype for the Gen3? RenderGraph powered render abstraction
// This is meant to be a platform/api agnostic rendering interface, think WGPU with RenderGraphs.
// The primary target is Desktop(Windows, Linux, Mac), Mobile(and other) support should be possible, but may never get implemented.
// Vulkan will be the first backend, followed by DX12 then finally Metal (If I ever get that far)
// Note that the API will assume the backend is "Bindless", may redesign this if this poses a significant portability problem.

// Key Design Principles
// 1.  Safety
//      a. Synchronization: Using a RenderGraph(and internal state tracking) will allow automatic gpu sync that will/should prevent any gpu data race conditions
//      b. Lazy Resource Deletion: Internal Resource tracking will prevent any resource from being freed while currently in use by a frame render or referenced by another resource
// 2. Simple
//      a. Render commands should be written at a high level
//      c. Reduces the amount of boilerplate code needed
//      d. Use of GpuData/GpuDataPacked will simplify Gpu Data Upload
// 3. Abstract:
//      a. Rendering code should work on any platform/backend
//      b. Shaders will still need to be implemented per backend (At least until a abstract shader graph editor is created)

// Design Sacrifices:
// 1. Bindless: API will requires the backend to support "Bindless" resources, may pose significant portability problem. (May have drop this requirement)
// 2. Performance: While it should be somewhat preformat (due to the nature of a RenderGraph), without significant optimization the graphics commands generate will not be "optimal", also lacks the ability for many micro optimizations
// 3. Memory: Despite many Professional RenderGraphs being quite good at memory optimization and reuse, without significant optimization rendering may waste more vram than necessary
// 4. Limited Features: There will be limits across all backends on Texture Formats, Pipeline Settings, Texture Samplers, Etc based on what the target api's support

//TODO: needs ability to create and support new windows/surfaces
//TODO: Async Upload and Compute

pub enum DeviceType {
    Integrated,
    Dedicated,
}

//Features and Extensions supported by the device
pub struct DeviceFeatures {
    pub raytracing: bool,
    pub variable_rate_shading: bool,
}

//Limits of the device
pub struct DeviceProperies {
    pub vram_size: usize,
}

pub struct DeviceInfo {
    pub name: String,
    pub vendor: String,
    pub device_type: DeviceType,
    pub features: DeviceFeatures,
    pub properties: DeviceProperies,
}

pub struct Surface(usize);

pub trait Instance {
    fn create_surface() -> Option<Arc<Surface>>;

    //Evaluates all available devices and assigns a score them, 0 means device is not usable, highest scoring device is initialized and returned.
    //Scoring function should take into account total Vram, Supported Features, DeviceType, etc.
    //If surface is not provided, device will run in headless mode.
    fn create_device(
        surface: Option<Arc<Surface>>,
        score_function: impl Fn(&DeviceInfo) -> u32,
    ) -> Option<()>;
}

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
pub type Sampler = Resource;

// TODO: Use From<> Trait instead????
/// A buffer type that can be bound for render graph passes
pub trait BufferGraphResource {
    fn get_handle(&self) -> usize;
}

// Buffer type can't be bound for rendering since it must be imported into render graph for synchronization
pub struct Buffer(Resource);

// StaticBuffer is filled during creation and will be immutable, therefore no synchronization is required after filling
pub struct StaticBuffer(Resource);
impl BufferGraphResource for StaticBuffer {
    fn get_handle(&self) -> usize {
        self.0.id
    }
}

pub type Texture = Resource;

//Device interface
//TODO: Return Result not Option
pub trait Device {
    fn get_info() -> DeviceInfo;

    fn add_surface(new_surface: Arc<Surface>) -> Result<(), ()>;

    fn create_shader_module() -> Option<Arc<ShaderModule>>;
    fn create_buffer() -> Option<Arc<Buffer>>;
    fn create_texture() -> Option<Arc<Texture>>;
    fn create_sampler() -> Option<Arc<Sampler>>;

    //TODO: Should this also/only have an async version?
    //Should these even exist or do they just complicated this even more
    fn create_static_buffer<T>(data: &[T]) -> Option<Arc<StaticBuffer>>;
    fn create_static_texture<T>(data: &[T]) -> Option<Arc<Texture>>;

    fn draw_frame() -> Option<()>;
}

//The interface containing all draw commands for raster based rendering after a raster pipeline has been bound
pub trait RasterCommandBuffer {
    fn bind_vertex_buffers(&self, buffer_offsets: &[(usize, u32)]);
    fn bind_index_buffer(&self, buffer: usize, offset: u32, index_type: u32);
    fn set_scissor(&self, offset: [i32; 2], extent: [u32; 2]);
    fn draw(&self, vertex_count: u32, first_vertex: u32, instance_count: u32, first_instance: u32);
    fn draw_indexed(
        &self,
        index_count: u32,
        index_offset: u32,
        vertex_offset: i32,
        instance_count: u32,
        instance_offset: u32,
    );
}

//Will likely become a derive macro
//The interface for all data that will be uploaded to the gpu for either buffer or push constants (Not texture)
//Can store data (f32,i32,u32...) and Resources(Buffers and Textures)
//Will store the Arc<Buffer>/Arc<Texture>/ETC so that those resources aren't deleted while in use
//TODO: how to make this not suck for data only things (I.E. Vertex/Index buffers or Texture Upload)
pub trait GpuData {
    fn get_gpu_size() -> usize;
}

//2 Types of GPU data
// 1. Struct (I.E. Material Info)
//      Requires Type Conversion + Offset
// 2. Packed Array (I.E. Vertex/Index Buffer)
//      Raw Data no Conversion

//Here is example definition for the Gpu Data Struct
//This can be used for the layout of a buffer or push data
//Before being uploaded to the gpu, a conversion(/packing?) pass is required
//Only supported raw types (f32, i32, u32, [f32; n]), resource bindings, and nested structs of GpuData/GpuDataPacked will be allowed by the Macro
//In the case of a buffer, the binding Arc's will be copied and stored in the struct as to preserve the resources as long as the buffer that references them lives
//In the case of push data, the binding Arc's will be stored in the render frame struct s to preserve the resources as long as the frame that references them is still executing
//TODO: Figure out Struct alignment/packing and nested types
//#[derive(GpuData)]
struct TestDataStruct {
    buffer_binding: Arc<Buffer>,
    texture_binding: Arc<Texture>,
    sampler_binding: Arc<Sampler>,
    float: f32,
    ints_array: [i32; 2],
    uints_array: [u32; 4],
    matrix: [f32; 16],
}

//Here is example definition for the Gpu Data Packed Array Element
//This can be used for the layout of a buffer or push data
//This can be uploaded as the raw bytes to the
//Only supported raw types (f32, i32, u32, [f32; n]), and nested structs of GpuDataPacked are allowed by the Macro
//Resource Bindings not allowed,
//Meant to be used as an array element, uploaded like as &[TestDataPackedVertex],
//TODO: raw elements without the #[derive(GpuDataPacked)] should be allowed too, I.E. &[f32]
//#[derive(GpuDataPacked)]
struct TestDataPackedVertex {
    pos: [f32; 3],
    uv: [f32; 2],
}
