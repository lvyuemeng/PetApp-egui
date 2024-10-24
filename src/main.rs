mod util;
use std::sync::{mpsc::channel, Arc};

use anyhow::{anyhow, Result};
use eframe::egui::{self, mutex::Mutex};
use util::{
    event::{BackendEvent, EventHandle, Handler, PetApp, RenderEvent},
    model::init_sql,
};
fn main() -> Result<()> {
    env_logger::init();

    let (backend_sender, backend_receiver) = channel::<BackendEvent>();
    let (render_sender, render_receiver) = channel::<RenderEvent>();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_always_on_top()
            .with_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    let init_query = init_sql().expect("Load init query successfully!");
    let db_con = sqlite::open(":memory:").expect("Load sqlite db successfully!");

    db_con.execute(init_query).expect("Initialize sqlite db!");

    let mut backend_handler = Handler::new(
        render_sender,
        backend_receiver,
        Arc::new(Mutex::new(db_con)),
    );
    std::thread::spawn(move || {
        backend_handler.handle_stream();
    });

    eframe::run_native(
        "PetApp",
        options,
        Box::new(|ctx| {
            egui_extras::install_image_loaders(&ctx.egui_ctx);
            Ok(PetApp::new(backend_sender, render_receiver))
        }),
    )
    .map_err(|e| anyhow!("eframe error: {}", e))
}
