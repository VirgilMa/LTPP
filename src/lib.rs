use chrono::Utc;
use log::debug;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use wasm_bindgen::prelude::*;
    }
}

mod common;
mod physics;
mod render;

#[cfg(not(target_arch = "wasm32"))]
pub fn setup_logger() {
    use log::info;

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
            wasm_bindgen_futures::spawn_local(async {
                render::window::render().await;
            });
        } else {
            setup_logger();
            pollster::block_on(render::window::render());
        }
    }

    debug!("LTPP start run");
}
