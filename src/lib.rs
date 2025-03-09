use log::{debug, info};
use chrono::Utc;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use wasm_bindgen::prelude::*;
    }
}

pub mod objmgr;
mod physics;
mod render;

pub fn setup_logger() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    info!("init logger")
}

pub fn get_current_time() -> i64 {
    // 获取当前 UTC 时间
    let now = Utc::now();
    // 获取以毫秒为单位的时间戳
    now.timestamp_millis()
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
