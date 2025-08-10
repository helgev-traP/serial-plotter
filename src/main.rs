fn main() {
    let shared_data = serial_plotter::shared::SharedData::new(1000);

    // Start the application
    if let Err(e) = serial_plotter::start_app(shared_data) {
        eprintln!("Application failed: {e}");
    }
}
