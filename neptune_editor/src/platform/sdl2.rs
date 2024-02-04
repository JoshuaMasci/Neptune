use crate::editor::Editor;
use anyhow::anyhow;
use sdl2::event::{Event, WindowEvent};

pub struct Sdl2Platform {
    context: sdl2::Sdl,
    event_pump: sdl2::EventPump,

    video: sdl2::VideoSubsystem,
    pub(crate) window: sdl2::video::Window,

    should_quit: bool,
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

        let mut event_pump = context
            .event_pump()
            .map_err(|err| anyhow!("sdl2 event error: {}", err))?;

        Ok(Self {
            context,
            event_pump,
            video,
            window,
            should_quit: false,
        })
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn process_events(&mut self, editor: &mut Editor) -> anyhow::Result<()> {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    self.should_quit = true;
                }
                Event::Window {
                    win_event: WindowEvent::SizeChanged(width, height),
                    ..
                } => {
                    editor.window_resize([width as u32, height as u32])?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
