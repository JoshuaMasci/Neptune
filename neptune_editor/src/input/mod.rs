pub mod winit;

pub type StaticString = &'static str;
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ButtonEventState {
    Released,
    Pressed,
}

trait InputEventReceiver {
    fn on_button_event(&self, button_name: StaticString, state: ButtonEventState) -> bool;
    fn on_axis_event(&self, axis_name: StaticString, value: f32) -> bool;
}
