use env_logger::Builder;

pub mod raytracer;

pub fn init_logger() {
    Builder::new().filter_level(log::LevelFilter::Info).init();

    log::info!("Logger initialized");
}
