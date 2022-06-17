mod device;
mod instance;

use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};

pub use device::Device;
pub use instance::Instance;

pub type ResourceId = u32;
pub struct Resource {
    pub(crate) id: ResourceId,
    pub(crate) deleted_list: Arc<Mutex<Vec<ResourceId>>>,
}

impl Drop for Resource {
    fn drop(&mut self) {
        self.deleted_list.borrow_mut().lock().unwrap().push(self.id)
    }
}

pub struct Surface(pub(crate) Resource);

pub struct GraphicsShader(pub(crate) Resource);
pub struct ComputeShader(pub(crate) Resource);
pub struct Buffer(pub(crate) Resource);
pub struct Texture(pub(crate) Resource);
pub struct Sampler(pub(crate) Resource);

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Integrated,
    Discrete,
    Unknown,
}

//Features and Extensions supported by the device
#[derive(Debug, Clone)]
pub struct DeviceFeatures {
    pub raytracing: bool,
    pub variable_rate_shading: bool,
}

//Limits of the device
#[derive(Debug, Clone)]
pub struct DeviceProperties {
    pub vram_size: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceVendor {
    AMD,
    Arm,
    ImgTec,
    Intel,
    Nvidia,
    Qualcomm,
    Unknown(u32),
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub vendor: DeviceVendor,
    pub device_type: DeviceType,
    //pub features: DeviceFeatures,
    //pub properties: DeviceProperties,
}

//TODO: Return Result not Option
//TODO: Decide between Dynamic and Static Dispatch!!!!!!!!!
//IDEA: Use Static Dispatch and Compile Time since the majority of platforms will only support 1 backend (Since OpenGL will never be supported).
//      Then for Windows which will support Vulkan and Dx12 make a Backend Wrapper that can support both with 2 modes
//          1. Use only 1
//          2. Use both at the same time(Not sure this will work???)

// Welcome to the Prototype for the Gen3? RenderGraph powered render abstraction
// This is meant to be a platform/api agnostic rendering interface, think WGPU with RenderGraphs.
// The primary target is Desktop(Windows, Linux, Mac), Mobile(and other) support should be possible, but may never get implemented.
// Vulkan will be the first backend, followed by DX12 then finally Metal (If I ever get that far)
// Note that the API will assume the backend is "Bindless", may redesign this if this poses a significant portability problem.

// Key Design Principles
// 1.  Safety
//      a. Synchronization: Using a RenderGraph(and internal state tracking) will allow automatic gpu sync that will/should prevent any gpu data race conditions
//      b. Lazy Resource Deletion: Internal Resource tracking will prevent any resource from being freed while currently in use by a frame render or referenced by another resource
//      c. TODO: Type Safety for buffers and textures?????
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
//TODO: Support f16/half-float as well

// TODO: Use From<> Trait instead????
/// A buffer type that can be bound for render graph passes
pub trait BufferGraphResource {
    fn get_handle(&self) -> usize;
}

// Buffer type can't be bound for rendering since it must be imported into render graph for synchronization
// StaticBuffer is filled during creation and will be immutable, therefore no synchronization is required after filling
// pub struct StaticBuffer(Resource);
// impl BufferGraphResource for StaticBuffer {
//     fn get_handle(&self) -> usize {
//         self.0.id
//     }
// }

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
//Trait functions to be filled by
pub trait GpuData {
    type PackedType;

    //TODO: no way to get bindings, need to rework this
    fn get_gpu_packed(&mut self) -> Self::PackedType;

    fn append_resources(
        buffers: &mut Vec<Arc<Buffer>>,
        textures: &mut Vec<Arc<Texture>>,
        samplers: &mut Vec<Arc<Sampler>>,
    );
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

//Example of what should be created by the impl
#[repr(C)]
struct TestDataStructPacked {
    buffer_binding: u32,
    texture_binding: u32,
    sampler_binding: u32,
    float: f32,
    ints_array: [i32; 2],
    uints_array: [u32; 4],
    matrix: [f32; 16],
}

impl GpuData for TestDataStruct {
    type PackedType = TestDataStructPacked;

    fn get_gpu_packed(&mut self) -> Self::PackedType {
        Self::PackedType {
            buffer_binding: 0,
            texture_binding: 0,
            sampler_binding: 0,
            float: 0.0,
            ints_array: [0; 2],
            uints_array: [0; 4],
            matrix: [0.0; 16],
        }
    }

    fn append_resources(
        buffers: &mut Vec<Arc<Buffer>>,
        textures: &mut Vec<Arc<Texture>>,
        samplers: &mut Vec<Arc<Sampler>>,
    ) {
        todo!()
    }
}

//End Example

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

//Empty trait, just to make sure nothing invalid gets used
pub trait GpuDataPacked {}
impl GpuDataPacked for i8 {}
impl GpuDataPacked for u8 {}
impl GpuDataPacked for i32 {}
impl GpuDataPacked for u32 {}
impl GpuDataPacked for f32 {}
impl GpuDataPacked for f64 {}

//TODO: Separate functions for GpuData and GpuDataPacked????
// pub fn data_to_bytes_test<T: GpuDataPacked + Sized>(data: &[T]) {
//     let byte_slice: &[u8] = unsafe {
//         std::slice::from_raw_parts(
//             data.as_ptr() as *const u8,
//             std::mem::size_of::<T>() * data.len(),
//         )
//     };
//     println!("{} to Bytes: {:?}", std::any::type_name::<T>(), byte_slice);
// }
// pub fn test_gpu_packed_function() {
//     data_to_bytes_test(&[0u8, 1u8, 2u8]);
//     data_to_bytes_test(&[0i8, 1i8, 2i8]);
//     data_to_bytes_test(&[0i32, 1i32, 2i32]);
//     data_to_bytes_test(&[0u32, 1u32, 2u32]);
//     data_to_bytes_test(&[0f32, 1f32, 2f32]);
//     data_to_bytes_test(&[0f64, 1f64, 2f64]);
// }

//RENDER GRAPH BUILDER TEST
//No implementation
pub struct RenderGraphBuilder {}

impl RenderGraphBuilder {
    pub fn create_buffer(&mut self) -> ResourceId {
        0
    }

    pub fn import_buffer(&mut self, buffer: Arc<Buffer>) -> ResourceId {
        0
    }

    //TODO: add clear color value to create info, the graph will determine which pass may needed to be cleared
    pub fn create_texture(&mut self) -> ResourceId {
        0
    }

    pub fn import_texture(&mut self, texture: Arc<Texture>) -> ResourceId {
        0
    }

    pub fn create_compute_pass<T: GpuData>(
        &mut self,
        name: &str,
        shader: Arc<ComputeShader>,
        dispatch_size: &[u32; 3],
        push_data: Option<T>,
    ) {
    }

    pub fn create_graphics_pass(
        &mut self,
        name: &str,
        color_attachments: &[ResourceId],
        depth_stencil_attachment: Option<ResourceId>,
    ) -> GraphicsPassBuilder {
        GraphicsPassBuilder {
            render_graph_builder: self,
        }
    }
}

pub struct GraphicsPassBuilder<'a> {
    render_graph_builder: &'a mut RenderGraphBuilder,
}

impl<'a> GraphicsPassBuilder<'a> {
    //TODO: replace temp values
    pub fn pipeline(
        self,
        shaders: Arc<GraphicsShader>,
        pipeline_settings: u8,
        vertex_layout: &[u8],
        render_fn: impl FnOnce() + 'static,
    ) -> Self {
        self
    }

    pub fn build(self) {
        drop(self);
    }
}
