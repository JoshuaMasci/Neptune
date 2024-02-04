use crate::input::ButtonState;
use sdl2::mouse::MouseButton;
use sdl2::sys::KeyCode;

pub struct Sdl2Input {}

impl Sdl2Input {
    pub fn mouse_button_event(button: MouseButton, state: ButtonState) {}
    pub fn key_event(key_code: KeyCode, state: ButtonState) {}
}
