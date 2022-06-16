use crate::interface::{
    Buffer, Device, DeviceInfo, DeviceType, DeviceVendor, GraphicsShader, Instance, Resource,
    Sampler, Surface, Texture,
};
use std::sync::{Arc, Mutex};

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

    fn create_buffer(&self) -> Option<Arc<Buffer>> {
        Some(Arc::new(Buffer(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }

    fn create_texture(&self) -> Option<Arc<Texture>> {
        Some(Arc::new(Texture(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }

    fn create_sampler(&self) -> Option<Arc<Sampler>> {
        Some(Arc::new(Sampler(Resource {
            id: 0,
            deleted_list: Arc::new(Mutex::new(vec![])),
        })))
    }

    fn draw_frame(&self) -> Option<()> {
        todo!()
    }
}
