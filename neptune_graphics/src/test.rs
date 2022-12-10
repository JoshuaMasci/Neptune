use crate::{
    Buffer, ComputePipeline, CubeTexture, Device, HandleType, RenderGraphBuilder, Sampler, Texture,
};
use std::collections::HashMap;

pub(crate) struct TestDevice {
    //Use same handle "pool" for all object types cause I'm lazy
    next_handle: HandleType,

    buffers: HashMap<Buffer, (String, u32)>,
    textures: HashMap<Texture, (String, [u32; 2])>,
    cube_textures: HashMap<CubeTexture, (String, u32)>,
    samplers: HashMap<Sampler, String>,
    compute_pipelines: HashMap<ComputePipeline, String>,
}

impl TestDevice {
    pub(crate) fn new() -> Self {
        Self {
            next_handle: 0,
            buffers: Default::default(),
            textures: Default::default(),
            cube_textures: Default::default(),
            samplers: Default::default(),
            compute_pipelines: Default::default(),
        }
    }
}

impl Drop for TestDevice {
    fn drop(&mut self) {}
}

impl Device for TestDevice {
    fn create_buffer(&mut self, size: u32, name: &str) -> crate::Result<Buffer> {
        let handle = Buffer(self.next_handle);
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

    fn execute_graph(
        &mut self,
        render_graph_builder: &mut RenderGraphBuilder,
    ) -> crate::Result<()> {
        Ok(())
    }
}
