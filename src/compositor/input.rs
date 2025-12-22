pub struct InputState {
    initialized: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        // TODO: Set up seat, keyboard, and pointer capabilities.
        self.initialized = true;
        println!("Input system initialized (stub).");
    }
}
