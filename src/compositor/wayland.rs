pub struct DisplayState {
    initialized: bool,
}

impl DisplayState {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        // TODO: Initialize Wayland display, socket, and event loop.
        self.initialized = true;
        println!("Wayland display initialized (stub).");
    }

    pub fn run_event_loop(&self) {
        // TODO: Replace with calloop loop when smithay integration is added.
        println!("Running event loop (stub).");
    }
}
