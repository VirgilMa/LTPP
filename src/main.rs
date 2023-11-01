use crate::render::window::render;

mod render;

fn main() {
    println!("Hello, world!");
    pollster::block_on(render());
}
