use log::{debug, info};

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use wasm_bindgen::prelude::*;
    }
}

pub mod objmgr;
mod render;

pub fn setup_logger() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    info!("init logger")
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Trace).expect("Couldn't initialize logger");
        } else {
            setup_logger();
        }
    }

    debug!("LTPP start run");
    pollster::block_on(render::window::render());
}
