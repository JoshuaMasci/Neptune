use crate::{
    Buffer, ComputePipeline, CubeTexture, Device, HandleType, RasterPipeline,
    RasterPipelineDescription, RenderGraphBuilder, Sampler, Swapchain, Texture,
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::collections::{HashMap, HashSet};

pub(crate) struct TestDevice {
    //Use same handle "pool" for all object types cause I'm lazy
    next_handle: HandleType,

    buffers: HashMap<Buffer, (String, u32)>,
    textures: HashMap<Texture, (String, [u32; 2])>,
    samplers: HashMap<Sampler, String>,
    compute_pipelines: HashMap<ComputePipeline, String>,
    raster_pipelines: HashMap<RasterPipeline, String>,
    swapchains: HashSet<Swapchain>,
}

impl TestDevice {
    pub(crate) fn new() -> Self {
        Self {
            next_handle: 0,
            buffers: Default::default(),
            textures: Default::default(),
            samplers: Default::default(),
            compute_pipelines: Default::default(),
            raster_pipelines: Default::default(),
            swapchains: Default::default(),
        }
    }
}

impl Drop for TestDevice {
    fn drop(&mut self) {}
}

impl Device for TestDevice {
    fn create_buffer(&mut self, size: u32, name: &str) -> crate::Result<Buffer> {
        let handle = Buffer::Handle(self.next_handle);
        self.next_handle += 1;
        let _ = self.buffers.insert(handle, (name.to_string(), size));
        Ok(handle)
    }

    fn destroy_buffer(&mut self, handle: Buffer) -> crate::Result<()> {
        if self.buffers.remove(&handle).is_some() {
            Ok(())
        } else {
            Err(crate::Error::InvalidHandle)
        }
    }

    fn create_texture(&mut self, size: [u32; 2], name: &str) -> crate::Result<Texture> {
        let handle = Texture(self.next_handle);
        self.next_handle += 1;
        let _ = self.textures.insert(handle, (name.to_string(), size));
        Ok(handle)
    }

    fn destroy_texture(&mut self, handle: Texture) -> crate::Result<()> {
        if self.textures.remove(&handle).is_some() {
            Ok(())
        } else {
            Err(crate::Error::InvalidHandle)
        }
    }

    fn create_sampler(&mut self, size: usize, name: &str) -> crate::Result<Sampler> {
        let handle = Sampler(self.next_handle);
        self.next_handle += 1;
        let _ = self.samplers.insert(handle, name.to_string());
        Ok(handle)
    }

    fn destroy_sampler(&mut self, handle: Sampler) -> crate::Result<()> {
        if self.samplers.remove(&handle).is_some() {
            Ok(())
        } else {
            Err(crate::Error::InvalidHandle)
        }
    }

    fn create_compute_pipeline(
        &mut self,
        code: &[u8],
        name: &str,
    ) -> crate::Result<ComputePipeline> {
        let _ = code;

        let handle = ComputePipeline(self.next_handle);
        self.next_handle += 1;
        let _ = self.compute_pipelines.insert(handle, name.to_string());
        Ok(handle)
    }

    fn destroy_compute_pipeline(&mut self, handle: ComputePipeline) -> crate::Result<()> {
        if self.compute_pipelines.remove(&handle).is_some() {
            Ok(())
        } else {
            Err(crate::Error::InvalidHandle)
        }
    }

    fn create_raster_pipeline(
        &mut self,
        raster_pipeline_description: &mut RasterPipelineDescription,
        name: &str,
    ) -> crate::Result<RasterPipeline> {
        let _ = raster_pipeline_description;

        let handle = RasterPipeline(self.next_handle);
        self.next_handle += 1;
        let _ = self.raster_pipelines.insert(handle, name.to_string());
        Ok(handle)
    }

    fn destroy_raster_pipeline(&mut self, handle: RasterPipeline) -> crate::Result<()> {
        if self.raster_pipelines.remove(&handle).is_some() {
            Ok(())
        } else {
            Err(crate::Error::InvalidHandle)
        }
    }

    fn create_swapchain<WindowType: HasRawWindowHandle + HasRawDisplayHandle>(
        &mut self,
        window: &WindowType,
    ) -> crate::Result<Swapchain> {
        let _raw_window_handle = window.raw_window_handle();
        let handle = Swapchain(self.next_handle);
        self.next_handle += 1;
        let _ = self.swapchains.insert(handle);
        Ok(handle)
    }

    fn destroy_swapchain(&mut self, handle: Swapchain) -> crate::Result<()> {
        if self.swapchains.remove(&handle) {
            Ok(())
        } else {
            Err(crate::Error::InvalidHandle)
        }
    }

    fn update_swapchain(&mut self, handle: Swapchain) -> crate::Result<()> {
        if self.swapchains.get(&handle).is_some() {
            Ok(())
        } else {
            Err(crate::Error::InvalidHandle)
        }
    }

    fn execute_graph(
        &mut self,
        render_graph_builder: &mut RenderGraphBuilder,
    ) -> crate::Result<()> {
        Ok(())
    }
}
