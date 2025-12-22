mod compositor;

fn main() {
    let mut compositor = compositor::Compositor::new();
    compositor.run();
}
