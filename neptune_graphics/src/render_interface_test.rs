use crate::interface::{
    Buffer, ComputeShader, Device, DeviceInfo, DeviceType, DeviceVendor, GpuData, GraphicsShader,
    Instance, RenderGraphBuilder, Resource, Sampler, Surface, Texture,
};
use crate::IndexSize;
use std::sync::{Arc, Mutex};

struct VertexData {
    position: [f32; 3],
}

impl GpuData for VertexData {
    type PackedType = u32;

    fn get_gpu_packed(&mut self) -> Self::PackedType {
        0
    }
    fn append_resources(
        buffers: &mut Vec<Buffer>,
        textures: &mut Vec<Texture>,
        samplers: &mut Vec<Sampler>,
    ) {
        todo!()
    }
}

struct FragmentData {
    texture: Texture,
}

impl GpuData for FragmentData {
    type PackedType = u32;

    fn get_gpu_packed(&mut self) -> Self::PackedType {
        0
    }
    fn append_resources(
        buffers: &mut Vec<Buffer>,
        textures: &mut Vec<Texture>,
        samplers: &mut Vec<Sampler>,
    ) {
        todo!()
    }
}

///This is mostly a test of the traits to see how nice it is to write code in this api
pub fn test_render_interface() {
    let mut test_instance = NullInstance::new();
    let device_search_result = test_instance.select_and_create_device(None, |device_info| {
        println!("Device: {}", device_info.name);
        match device_info.device_type {
            DeviceType::Integrated => 50,
            DeviceType::Discrete => 100,
            DeviceType::Unknown => 0,
        }
    });

    if device_search_result.is_none() {
        println!("Failed to find a suitable device");
        return;
    }
    let device = device_search_result.unwrap();
    println!("Selected Device: {:#?}", device);

    //TODO: Should each shader stage be separate?
    let graphics_shader = device
        .create_graphics_shader(&[1, 2, 3], Some(&[1, 2, 3]))
        .expect("Failed to create Graphics Shaders");

    let compute_shader = device
        .create_compute_shader(&[1, 2, 3])
        .expect("Failed to create Compute Shaders");

    //TODO: Resource Description
    let compute_buffer = device.create_buffer().expect("Failed to create Buffer");

    //TODO: Data upload / Async upload option
    let vertex_buffer = device
        .create_static_buffer()
        .expect("Failed to create Vertex Buffer");
    let index_buffer = device
        .create_static_buffer()
        .expect("Failed to create Vertex Buffer");
    let base_texture = device
        .create_static_texture()
        .expect("Failed to create Base Texture");

    device
        .render_frame(|render_graph_builder| {
            let temp_buffer = render_graph_builder.create_buffer();

            render_graph_builder.create_compute_pass(
                "ComputePass",
                compute_shader.clone(),
                &[128, 256, 512],
                Some(ComputePushData {
                    first_buffer: compute_buffer.clone(),
                    temp_buffer,
                    some_data: 1.0,
                }),
            );

            //TODO: add clear color value to create info, the graph will determine which pass may needed to be cleared
            let temp_image = render_graph_builder.create_texture();

            let base_texture_clone = base_texture.clone();

            render_graph_builder
                .create_graphics_pass("Graphics Pass", &[temp_image], None)
                .add_pipeline(graphics_shader.clone(), 0, &[], |raster_api| {
                    println!("Render Function");
                    raster_api.bind_vertex_buffers(&[(vertex_buffer, 0)]);
                    raster_api.bind_index_buffer(index_buffer, 0, IndexSize::U32);

                    raster_api.push_vertex_data(VertexData { position: [0.0; 3] });
                    raster_api.push_fragment_data(FragmentData {
                        texture: base_texture_clone,
                    });

                    raster_api.draw_indexed(3, 0, 0, 1, 0);
                });
        })
        .expect("Failed to render frames");
}

//TODO: write/use #[derive(GpuData)]
struct ComputePushData {
    first_buffer: Buffer,
    temp_buffer: Buffer,
    some_data: f32,
}

impl GpuData for ComputePushData {
    type PackedType = u32;
    fn get_gpu_packed(&mut self) -> Self::PackedType {
        0
    }
    fn append_resources(
        buffers: &mut Vec<Buffer>,
        textures: &mut Vec<Texture>,
        samplers: &mut Vec<Sampler>,
    ) {
    }
}

struct NullInstance {
    devices: Vec<DeviceInfo>,
}

impl NullInstance {
    fn new() -> Self {
        Self {
            devices: vec![
                DeviceInfo {
                    name: String::from("Some Integrated Gpu"),
                    vendor: DeviceVendor::Intel,
                    device_type: DeviceType::Integrated,
                },
                DeviceInfo {
                    name: String::from("Some Discrete Gpu"),
                    vendor: DeviceVendor::Nvidia,
                    device_type: DeviceType::Discrete,
                },
            ],
        }
    }
}

impl Instance for NullInstance {
    type DeviceImpl = NullDevice;

    fn create_surface(&mut self) -> Option<Arc<Surface>> {
        todo!()
    }

    fn select_and_create_device(
        &mut self,
        surface: Option<Arc<Surface>>,
        score_function: impl Fn(&DeviceInfo) -> u32,
    ) -> Option<Self::DeviceImpl> {
        self.devices
            .iter()
            .map(|device_info| (device_info.clone(), score_function(device_info)))
            .max_by(|(_, score1), (_, score2)| score1.cmp(score2))
            .map(|(info, _)| Self::DeviceImpl::new(info))
    }
}

#[derive(Debug)]
struct NullDevice {
    info: DeviceInfo,
}

impl NullDevice {
    fn new(info: DeviceInfo) -> Self {
        Self { info }
    }
}

impl Device for NullDevice {
    fn get_info(&self) -> DeviceInfo {
        self.info.clone()
    }

    fn add_surface(&self, surface: Arc<Surface>) -> Option<usize> {
        todo!()
    }

    fn create_graphics_shader(
        &self,
        vertex_code: &[u8],
        fragment_code: Option<&[u8]>,
    ) -> Option<Arc<GraphicsShader>> {
        Some(Arc::new(GraphicsShader(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }

    fn create_compute_shader(&self, code: &[u8]) -> Option<Arc<ComputeShader>> {
        Some(Arc::new(ComputeShader(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }

    fn create_buffer(&self) -> Option<Buffer> {
        Some(Buffer::Mutable(Arc::new(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }

    fn create_static_buffer(&self) -> Option<Buffer> {
        Some(Buffer::Static(Arc::new(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }

    fn create_texture(&self) -> Option<Texture> {
        Some(Texture::Mutable(Arc::new(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }
    fn create_static_texture(&self) -> Option<Texture> {
        Some(Texture::Static(Arc::new(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }

    fn create_sampler(&self) -> Option<Sampler> {
        Some(Sampler(Arc::new(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }

    fn render_frame(
        &self,
        build_render_graph_fn: impl FnOnce(&mut RenderGraphBuilder),
    ) -> Result<(), ()> {
        let mut render_graph_builder = RenderGraphBuilder {};
        build_render_graph_fn(&mut render_graph_builder);
        Ok(())
    }
}
