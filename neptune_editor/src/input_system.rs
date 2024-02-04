pub type StaticString = &'static str;

trait InputDevice {
    fn get_axis(&self, axis_name: StaticString) -> Option<f32>;
    fn is_button_down(&self, button_name: StaticString) -> Option<bool>;
    fn is_button_pressed(&self, button_name: StaticString) -> Option<bool>;
}

#[derive(Default)]
pub struct InputSystem {}

impl InputSystem {
    pub fn get_axis(&self, axis_name: StaticString) -> Option<f32> {
        None
    }

    pub fn is_button_pressed(&self, button_name: StaticString) -> Option<bool> {
        None
    }

    pub fn is_button_down(&self, button_name: StaticString) -> Option<bool> {
        None
    }
}
