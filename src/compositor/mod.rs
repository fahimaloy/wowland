pub mod config;
mod input;
mod launcher;
mod layout;
mod panel;
mod runtime;
mod window;

pub fn run(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    runtime::run(config_path)
}
