pub mod backend;
pub mod frontend;
pub mod shared;

use crate::{frontend::Frontend, shared::SharedData};

pub fn start_app(shared_data: SharedData) -> eframe::Result {
    let (event_sender, event_receiver) = crossbeam::channel::bounded(10);

    let backend = backend::Backend::new(shared_data.clone(), event_receiver);
    let frontend = frontend::Frontend::new(shared_data, event_sender);

    backend.start_backend_thread();

    // prepare eframe
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Serial Monitor",
        native_options,
        Box::new(|_cc| Ok(Box::new(frontend))),
    )
}
