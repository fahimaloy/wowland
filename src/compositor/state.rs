use crate::compositor::{input, render, wayland};

pub struct State {
    display: wayland::DisplayState,
    input: input::InputState,
    renderer: render::RendererState,
}

impl State {
    pub fn new() -> Self {
        Self {
            display: wayland::DisplayState::new(),
            input: input::InputState::new(),
            renderer: render::RendererState::new(),
        }
    }

    pub fn initialize(&mut self) {
        self.display.initialize();
        self.input.initialize();
        self.renderer.initialize();
    }

    pub fn run(&mut self) {
        self.display.run_event_loop();
    }
}
