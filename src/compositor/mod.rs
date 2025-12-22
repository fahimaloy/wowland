pub mod input;
pub mod render;
pub mod state;
pub mod wayland;

pub struct Compositor {
    state: state::State,
}

impl Compositor {
    pub fn new() -> Self {
        Self {
            state: state::State::new(),
        }
    }

    pub fn run(&mut self) {
        self.state.initialize();
        self.state.run();
    }
}
