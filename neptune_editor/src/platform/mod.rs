pub mod sdl2;

pub trait WindowEventReceiver {
    fn on_window_size_changed(&mut self, new_size: [u32; 2]) -> anyhow::Result<()>;
}
