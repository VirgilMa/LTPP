mod render;

#[cfg_attr(target_arch = "wasm", wasm_bindgen(start))]
pub fn run() {
    println!("Hello, world!");
    pollster::block_on(render::window::render());
}
