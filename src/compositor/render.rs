#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

pub struct RendererState {
    initialized: bool,
    background: Color,
}

impl RendererState {
    pub fn new() -> Self {
        Self {
            initialized: false,
            background: Color {
                r: 0.12,
                g: 0.14,
                b: 0.18,
            },
        }
    }

    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        // TODO: Initialize renderer backend and outputs.
        self.initialized = true;
        println!(
            "Renderer initialized (stub). Background color: {:?}",
            self.background
        );
    }
}
