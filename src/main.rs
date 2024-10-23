mod util;
use std::sync::mpsc::channel;

use anyhow::{anyhow, Result};
use eframe::egui::{self};
use util::{
    event::{Event, PetApp},
    model::init_sql,
};
fn main() -> Result<()> {
    env_logger::init();

    let (back_event_sender, back_event_receiver) = channel::<Event>();
    let (event_sender, event_receiver) = channel::<Event>();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_always_on_top()
            .with_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    std::thread::spawn(move || {
        while let Ok(event) = back_event_receiver.recv() {
            let sender = event_sender.clone();
            event.handle_op(sender);
        }
    });

    let init_query = init_sql().expect("Load init query successfully!");
    let db_con = sqlite::open(":memory:").expect("Load sqlite db successfully!");

    db_con.execute(init_query).expect("Initialize sqlite db!");

    eframe::run_native(
        "PetApp",
        options,
        Box::new(|ctx| {
            egui_extras::install_image_loaders(&ctx.egui_ctx);
            Ok(PetApp::new(back_event_sender, event_receiver, db_con)?)
        }),
    )
    .map_err(|e| anyhow!("eframe error: {}", e))
}
