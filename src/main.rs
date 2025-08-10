fn main() {
    let shared_data = serial::shared::SharedData::new(1000);

    // Start the application
    if let Err(e) = serial::start_app(shared_data) {
        eprintln!("Application failed: {e}");
    }
}
