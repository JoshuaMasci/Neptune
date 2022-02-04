#[allow(dead_code)]
struct Instance {} //Just a vulkan instance

#[allow(dead_code)]
struct Surface {} //Just a vulkan surface

#[allow(dead_code)]
struct Device {} //Vulkan device with allocator, pipeline cache, resource management
                 //TODO: keep track of read-only descriptor set here?

#[allow(dead_code)]
impl Device {
    fn create_static_buffer() {}
    fn create_static_texture() {}
}

#[allow(dead_code)]
struct DeviceGraphics {} //Vulkan swapchain, render graph renderer
                         //TODO: create dynamic descriptor set here

#[allow(dead_code)]
impl DeviceGraphics {
    fn start_render_graph() {}
    fn submit_render_graph() {}
}

#[allow(dead_code)]
struct DeviceCompute {} //Async compute queue, returns "Future" for outputted resources
                        //TODO: create dynamic descriptor set here
