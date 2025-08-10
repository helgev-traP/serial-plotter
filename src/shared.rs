use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

pub mod port_info;
pub mod serial_read;

#[derive(Clone)]
pub struct SharedData {
    pub read_data: Arc<RwLock<serial_read::SerialRead>>,
    pub port_info: Arc<RwLock<port_info::PortsInfo>>,
    pub error_log: Arc<Mutex<String>>,
}

// send to backend to change settings
pub enum Event {
    SelectPort(String),
    SelectBaudRate(u32),
    ClearLog,
    Shutdown,
}
