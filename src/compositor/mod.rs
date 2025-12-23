mod config;
mod input;
mod layout;
mod runtime;
mod window;

pub fn run() {
    if let Err(error) = runtime::run() {
        eprintln!("Compositor failed: {error}");
    }
}
