use std::collections::HashMap;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

pub struct WinitInputHandler {}

impl WinitInputHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn on_event<T>(&mut self, event: &winit::event::Event<T>) {
        match event {
            winit::event::Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } => {
                self.on_key_event(event);
            }
            winit::event::Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                self.on_mouse_button(button, state);
            }
            winit::event::Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                self.on_mouse_wheel(delta);
            }

            winit::event::Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                self.on_mouse_move([position.x as f32, position.y as f32]);
            }
            _ => {}
        }
    }

    pub fn on_mouse_button(&mut self, button: &MouseButton, state: &ElementState) {
        info!("on_mouse_button: {:?}:{:?}", button, state);
    }
    pub fn on_mouse_wheel(&mut self, delta: &MouseScrollDelta) {
        info!("on_mouse_wheel: {:?}", delta);
    }
    pub fn on_mouse_move(&mut self, position: [f32; 2]) {
        let _ = position;
        //info!("on_mouse_move: {:?}", position);
    }

    pub fn on_key_event(&mut self, key_event: &winit::event::KeyEvent) {
        info!("on_key_event: {:?}", key_event);
    }
}
