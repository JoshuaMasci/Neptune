use crate::input::{ButtonState, InputEventReceiver, StaticString};
use crate::platform::WindowEventReceiver;
use anyhow::anyhow;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ButtonAxisDirection {
    Positive,
    Negitive,
}

#[derive(Default, Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct ButtonAxisState {
    positive_state: bool,
    negative_state: bool,
}

impl ButtonAxisState {
    pub fn set(&mut self, dir: ButtonAxisDirection, state: ButtonState) {
        let state = state.is_down();
        match dir {
            ButtonAxisDirection::Positive => self.positive_state = state,
            ButtonAxisDirection::Negitive => self.negative_state = state,
        }
    }

    pub fn to_f32(&self) -> f32 {
        if self.positive_state && !self.negative_state {
            1.0
        } else if self.negative_state && !self.positive_state {
            -1.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ButtonBinding {
    Button(StaticString),
    Axis {
        name: StaticString,
        direction: ButtonAxisDirection,
    },
}

#[derive(Debug, Copy, Clone)]
pub struct MouseAxisBinding {
    name: StaticString,
    sensitivity: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct ControllerAxisBinding {
    name: StaticString,
    sensitivity: f32,
    deadzone: f32,
    inverted: bool,
}

impl ControllerAxisBinding {
    pub(crate) fn calc(&self, value: i16) -> f32 {
        let value = (value as f32) / (i16::MAX as f32);
        let abs_value = value.abs();

        if abs_value > self.deadzone {
            let range = 1.0 - self.deadzone;
            let sign = value.signum();
            let invert = if self.inverted { -1.0 } else { 1.0 };
            (((abs_value - self.deadzone) / range) * self.sensitivity * sign * invert)
                .clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }
}

pub struct SdlController {
    controller: sdl2::controller::GameController,

    button_bindings: HashMap<sdl2::controller::Button, ButtonBinding>,
    button_axis_state: HashMap<StaticString, ButtonAxisState>,

    axis_bindings: HashMap<sdl2::controller::Axis, ControllerAxisBinding>,
}

impl SdlController {
    fn new(controller: sdl2::controller::GameController) -> Self {
        Self {
            controller,
            button_bindings: HashMap::new(),
            button_axis_state: HashMap::new(),
            axis_bindings: HashMap::new(),
        }
    }
}

pub enum WindowSize {
    Windowed([u32; 2]),
    Fullscreen,
    Maximized,
}

pub struct Sdl2Platform {
    context: sdl2::Sdl,
    event_pump: sdl2::EventPump,

    video: sdl2::VideoSubsystem,
    game_controller: sdl2::GameControllerSubsystem,
    haptic: sdl2::HapticSubsystem,

    pub(crate) window: sdl2::video::Window,

    should_quit: bool,

    // Move binding into App at some point
    mouse_captured: bool,
    key_bindings: HashMap<Keycode, ButtonBinding>,

    mouse_button_bindings: HashMap<MouseButton, ButtonBinding>,

    mouse_moved: bool,
    mouse_axis_x_binding: Option<MouseAxisBinding>,
    mouse_axis_y_binding: Option<MouseAxisBinding>,

    button_axis_state: HashMap<StaticString, ButtonAxisState>,

    controllers: HashMap<u32, SdlController>,
}

impl Sdl2Platform {
    pub fn new(name: &str, window_size: WindowSize) -> anyhow::Result<Self> {
        let context = sdl2::init().map_err(|err| anyhow!("sdl2 init error: {}", err))?;
        let video = context
            .video()
            .map_err(|err| anyhow!("sdl2 video init error: {}", err))?;
        let game_controller = context
            .game_controller()
            .map_err(|err| anyhow!("sdl2 game_controller init error: {}", err))?;
        let haptic = context
            .haptic()
            .map_err(|err| anyhow!("sdl2 haptic init error: {}", err))?;

        let window = match window_size {
            WindowSize::Windowed(size) => video
                .window(name, size[0], size[1])
                .position_centered()
                .resizable()
                .build()?,
            WindowSize::Fullscreen => video
                .window(name, 1920, 1080)
                .fullscreen_desktop()
                .position_centered()
                .resizable()
                .build()?,
            WindowSize::Maximized => video
                .window(name, 1920, 1080)
                .maximized()
                .position_centered()
                .resizable()
                .build()?,
        };

        let event_pump = context
            .event_pump()
            .map_err(|err| anyhow!("sdl2 event error: {}", err))?;

        //TODO: load bindings from file
        let mut key_bindings = HashMap::new();

        key_bindings.insert(
            Keycode::A,
            ButtonBinding::Axis {
                name: "player_move_left_right",
                direction: ButtonAxisDirection::Positive,
            },
        );
        key_bindings.insert(
            Keycode::D,
            ButtonBinding::Axis {
                name: "player_move_left_right",
                direction: ButtonAxisDirection::Negitive,
            },
        );

        key_bindings.insert(
            Keycode::Space,
            ButtonBinding::Axis {
                name: "player_move_up_down",
                direction: ButtonAxisDirection::Positive,
            },
        );
        key_bindings.insert(
            Keycode::LCtrl,
            ButtonBinding::Axis {
                name: "player_move_up_down",
                direction: ButtonAxisDirection::Negitive,
            },
        );

        key_bindings.insert(
            Keycode::W,
            ButtonBinding::Axis {
                name: "player_move_forward_back",
                direction: ButtonAxisDirection::Positive,
            },
        );
        key_bindings.insert(
            Keycode::S,
            ButtonBinding::Axis {
                name: "player_move_forward_back",
                direction: ButtonAxisDirection::Negitive,
            },
        );

        key_bindings.insert(Keycode::LShift, ButtonBinding::Button("player_move_sprint"));

        let mouse_button_bindings = HashMap::new();

        //TODO: allow as setting
        //const HINT_MOUSE_RELATIVE_SYSTEM_SCALE: &str = "SDL_HINT_MOUSE_RELATIVE_SYSTEM_SCALE"; // bool

        Ok(Self {
            context,
            event_pump,

            video,
            game_controller,
            haptic,

            window,
            should_quit: false,
            mouse_captured: false,
            key_bindings,
            mouse_button_bindings,
            mouse_moved: false,
            mouse_axis_x_binding: Some(MouseAxisBinding {
                name: "player_move_yaw",
                sensitivity: 0.2,
            }),
            mouse_axis_y_binding: Some(MouseAxisBinding {
                name: "player_move_pitch",
                sensitivity: 0.2,
            }),
            button_axis_state: HashMap::new(),
            controllers: HashMap::new(),
        })
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn process_events<T: WindowEventReceiver + InputEventReceiver>(
        &mut self,
        app: &mut T,
    ) -> anyhow::Result<()> {
        if !app.requests_mouse_capture() {
            self.capture_mouse(false);
        }

        // Clear movement from last frame
        if self.mouse_moved {
            self.proccess_mouse_move_event(app, 0, 0);
            self.mouse_moved = false;
        }

        while let Some(event) = self.event_pump.poll_event() {
            match event {
                Event::Quit { .. } => {
                    self.should_quit = true;
                }

                Event::KeyDown {
                    keycode, repeat, ..
                } => {
                    if !repeat {
                        // Escape should always free mouse, hardcoded here so that game bad logic can't hold the mouse hostage
                        if keycode == Some(Keycode::Escape) {
                            self.capture_mouse(false);
                        } else {
                            self.process_key_event(app, keycode, ButtonState::Pressed);
                        }
                    }
                }
                Event::KeyUp {
                    keycode, repeat, ..
                } => {
                    if !repeat {
                        self.process_key_event(app, keycode, ButtonState::Released);
                    }
                }

                Event::TextInput { text, .. } => {
                    app.on_text_event(text);
                }

                Event::MouseButtonDown { mouse_btn, .. } => {
                    if app.requests_mouse_capture()
                        && !self.window.mouse_grab()
                        && mouse_btn == MouseButton::Left
                    {
                        self.capture_mouse(true);
                    }

                    if self.mouse_captured {
                        self.process_mouse_button_event(app, mouse_btn, ButtonState::Pressed);
                    }
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    if self.mouse_captured {
                        self.process_mouse_button_event(app, mouse_btn, ButtonState::Pressed);
                    }
                }
                Event::MouseMotion { xrel, yrel, .. } => {
                    if self.mouse_captured {
                        self.proccess_mouse_move_event(app, xrel, yrel);
                        self.mouse_moved = true;
                    }
                }

                Event::ControllerDeviceAdded { which, .. } => {
                    if let Ok(game_controller) = self.game_controller.open(which) {
                        //let _ = game_controller.set_led(127, 0, 255);
                        //let _ = game_controller.set_rumble_triggers(u16::MAX, u16::MAX, 10);
                        info!(
                            "Game Controller Added: {}({}) Rumble {} Trigger Rumble {}",
                            game_controller.name(),
                            which,
                            game_controller.has_rumble(),
                            game_controller.has_rumble_triggers(),
                        );

                        let mut controller = SdlController::new(game_controller);

                        //TODO: load config from file
                        {
                            let _ = controller.axis_bindings.insert(
                                sdl2::controller::Axis::LeftX,
                                ControllerAxisBinding {
                                    name: "player_move_left_right",
                                    sensitivity: 1.0,
                                    deadzone: 0.1,
                                    inverted: true,
                                },
                            );
                            let _ = controller.axis_bindings.insert(
                                sdl2::controller::Axis::LeftY,
                                ControllerAxisBinding {
                                    name: "player_move_forward_back",
                                    sensitivity: 1.0,
                                    deadzone: 0.1,
                                    inverted: true,
                                },
                            );

                            let _ = controller.axis_bindings.insert(
                                sdl2::controller::Axis::RightX,
                                ControllerAxisBinding {
                                    name: "player_move_yaw",
                                    sensitivity: 0.75,
                                    deadzone: 0.1,
                                    inverted: false,
                                },
                            );
                            let _ = controller.axis_bindings.insert(
                                sdl2::controller::Axis::RightY,
                                ControllerAxisBinding {
                                    name: "player_move_pitch",
                                    sensitivity: 0.75,
                                    deadzone: 0.1,
                                    inverted: false,
                                },
                            );
                        }

                        let _ = self.controllers.insert(which, controller);
                    }
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    if let Some(game_controller) = self.controllers.remove(&which) {
                        info!(
                            "Game Controller Removed: {}({})",
                            game_controller.controller.name(),
                            which
                        );
                    }
                }
                Event::ControllerButtonDown { which, button, .. } => {
                    if let Some(controller) = self.controllers.get_mut(&which) {
                        if button == sdl2::controller::Button::RightShoulder {
                            if let Err(err) = controller.controller.set_rumble(0, u16::MAX, 128) {
                                error!("Rumble not supported: {}", err);
                            }
                        } else if button == sdl2::controller::Button::LeftShoulder {
                            if let Err(err) = controller.controller.set_rumble(u16::MAX, 0, 128) {
                                error!("Rumble not supported: {}", err);
                            }
                        }
                    }
                }
                Event::ControllerAxisMotion {
                    which, axis, value, ..
                } => {
                    if let Some(controller) = self.controllers.get_mut(&which) {
                        if let Some(axis_binding) = controller.axis_bindings.get(&axis) {
                            app.on_axis_event(axis_binding.name, axis_binding.calc(value));
                        }
                    }
                }

                Event::Window {
                    win_event: WindowEvent::SizeChanged(width, height),
                    ..
                } => {
                    app.on_window_size_changed([width as u32, height as u32])?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn capture_mouse(&mut self, capture: bool) {
        // Don't re capture/free mouse
        if capture != self.mouse_captured {
            if capture {
                debug!("sdl2 capture mouse");
                self.context.mouse().set_relative_mouse_mode(true);
            } else {
                debug!("sdl2 free mouse");
                self.context.mouse().set_relative_mouse_mode(false);
            }
            self.mouse_captured = capture;
        }
    }

    pub fn proccess_mouse_move_event<T: InputEventReceiver>(
        &mut self,
        app: &mut T,
        x_move: i32,
        y_move: i32,
    ) {
        if let Some(binding) = &self.mouse_axis_x_binding {
            let _ = app.on_axis_event(binding.name, x_move as f32 * binding.sensitivity);
        }

        if let Some(binding) = &self.mouse_axis_y_binding {
            let _ = app.on_axis_event(binding.name, y_move as f32 * binding.sensitivity);
        }
    }

    pub fn process_mouse_button_event<T: InputEventReceiver>(
        &mut self,
        app: &mut T,
        mouse_button: MouseButton,
        state: ButtonState,
    ) {
        if let Some(binding) = self.mouse_button_bindings.get(&mouse_button).cloned() {
            self.process_button_event(app, binding, state);
        }
    }

    pub fn process_key_event<T: InputEventReceiver>(
        &mut self,
        app: &mut T,
        keycode: Option<Keycode>,
        state: ButtonState,
    ) {
        if let Some(keycode) = keycode {
            if let Some(binding) = self.key_bindings.get(&keycode).cloned() {
                self.process_button_event(app, binding, state);
            }
        }
    }

    pub fn process_button_event<T: InputEventReceiver>(
        &mut self,
        app: &mut T,
        binding: ButtonBinding,
        state: ButtonState,
    ) {
        match binding {
            ButtonBinding::Button(name) => {
                let _ = app.on_button_event(name, state);
            }
            ButtonBinding::Axis { name, direction } => {
                let entry = self.button_axis_state.entry(name).or_default();
                entry.set(direction, state);
                let _ = app.on_axis_event(name, entry.to_f32());
            }
        }
    }
}
