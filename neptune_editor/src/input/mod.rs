mod sdl2;

pub type StaticString = &'static str;
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ButtonState {
    Released,
    Pressed,
}

impl ButtonState {
    pub fn is_down(self) -> bool {
        self == ButtonState::Pressed
    }
}

pub trait InputEventReceiver {
    fn requests_mouse_capture(&mut self) -> bool;

    fn on_button_event(&mut self, button_name: StaticString, state: ButtonState) -> bool;
    fn on_axis_event(&mut self, axis_name: StaticString, value: f32) -> bool;
    fn on_text_event(&mut self) -> bool;
}
