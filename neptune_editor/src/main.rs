mod camera;
mod editor;
mod game;
mod gltf_loader;
mod input;
mod input_system;
mod material;
mod mesh;
mod physics;
mod platform;
mod scene;
mod shader;
mod transform;
mod universe;

#[macro_use]
extern crate log;

use crate::editor::{Editor, EditorConfig};
use crate::platform::sdl2::WindowSize;
use clap::Parser;
use std::time::Instant;

pub const APP_NAME: &str = "Neptune Editor";

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init_timed();

    let config = EditorConfig::parse();

    let mut platform = platform::sdl2::Sdl2Platform::new(
        APP_NAME,
        if config.fullscreen {
            WindowSize::Fullscreen
        } else {
            WindowSize::Maximized
        },
    )?;

    let window_size = platform.window.drawable_size();
    info!("window_size: {:?}", window_size);
    let mut editor = Editor::new(&platform.window, [window_size.0, window_size.1], &config)?;

    let mut last_frame_start = Instant::now();
    let mut frame_count_time: (u32, f32) = (0, 0.0);
    while !platform.should_quit() {
        platform.process_events(&mut editor)?;

        let last_frame_time = last_frame_start.elapsed();
        last_frame_start = Instant::now();

        editor.update(last_frame_time.as_secs_f32());

        editor.render().expect("Failed to render a frame");

        frame_count_time.0 += 1;
        frame_count_time.1 += last_frame_time.as_secs_f32();

        if frame_count_time.1 >= 1.0 {
            info!("FPS: {}", frame_count_time.0);
            frame_count_time = (0, 0.0);
        }
    }

    info!("Exiting Main Loop!");
    Ok(())
}
