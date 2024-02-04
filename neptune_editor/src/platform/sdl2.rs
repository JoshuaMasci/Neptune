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

pub enum ButtonBinding {
    Button(StaticString),
    Axis {
        name: StaticString,
        direction: ButtonAxisDirection,
    },
}

pub struct Sdl2Platform {
    context: sdl2::Sdl,
    event_pump: sdl2::EventPump,

    video: sdl2::VideoSubsystem,
    pub(crate) window: sdl2::video::Window,

    should_quit: bool,

    // Move binding into App at some point
    mouse_captured: bool,
    key_bindings: HashMap<Keycode, ButtonBinding>,
    button_axis_state: HashMap<StaticString, ButtonAxisState>,
}

impl Sdl2Platform {
    pub fn new(name: &str, size: [u32; 2]) -> anyhow::Result<Self> {
        let context = sdl2::init().map_err(|err| anyhow!("sdl2 init error: {}", err))?;
        let video = context
            .video()
            .map_err(|err| anyhow!("sdl2 video init error: {}", err))?;

        let window = video
            .window(name, size[0], size[1])
            .position_centered()
            .resizable()
            .build()?;

        let event_pump = context
            .event_pump()
            .map_err(|err| anyhow!("sdl2 event error: {}", err))?;

        //TODO: load bindings from file
        let mut key_bindings = HashMap::new();

        key_bindings.insert(
            Keycode::D,
            ButtonBinding::Axis {
                name: "player_move_right_left",
                direction: ButtonAxisDirection::Positive,
            },
        );
        key_bindings.insert(
            Keycode::A,
            ButtonBinding::Axis {
                name: "player_move_right_left",
                direction: ButtonAxisDirection::Negitive,
            },
        );

        key_bindings.insert(
            Keycode::LShift,
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

        Ok(Self {
            context,
            event_pump,
            video,
            window,
            should_quit: false,
            mouse_captured: false,
            key_bindings,
            button_axis_state: HashMap::new(),
        })
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn process_events<T: WindowEventReceiver + InputEventReceiver>(
        &mut self,
        app: &mut T,
    ) -> anyhow::Result<()> {
        if !app.requests_mouse_capture() && self.mouse_captured {
            self.capture_mouse(false);
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

                Event::MouseButtonDown { mouse_btn, .. } => {
                    //if !self.window.has_mouse_focus() && mouse_btn == MouseButton::Left {
                    if app.requests_mouse_capture()
                        && !self.window.mouse_grab()
                        && mouse_btn == MouseButton::Left
                    {
                        self.capture_mouse(true);
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
        if capture {
            debug!("sdl2 capture mouse");
            self.window.set_grab(true);
            self.context.mouse().show_cursor(false);
        } else {
            debug!("sdl2 free mouse");
            self.context.mouse().capture(false);
            self.window.set_grab(false);
            self.context.mouse().show_cursor(true);
        }
        self.mouse_captured = capture;
    }

    pub fn process_key_event<T: InputEventReceiver>(
        &mut self,
        app: &mut T,
        keycode: Option<Keycode>,
        state: ButtonState,
    ) {
        if let Some(keycode) = keycode {
            if let Some(binding) = self.key_bindings.get(&keycode) {
                match binding {
                    ButtonBinding::Button(name) => {
                        let _ = app.on_button_event(name, state);
                    }
                    ButtonBinding::Axis { name, direction } => {
                        let entry = self.button_axis_state.entry(name).or_default();
                        entry.set(*direction, state);
                        let _ = app.on_axis_event(name, entry.to_f32());
                    }
                }
            }
        }
    }
}
