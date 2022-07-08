pub use log;

pub fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .filter(|metadata| !metadata.target().contains("wgpu"))
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Warn)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}
