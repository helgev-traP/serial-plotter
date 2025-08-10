use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

pub mod port_info;
pub mod serial_read;

#[derive(Clone, Debug)]
pub struct SharedData {
    pub read_data: Arc<RwLock<serial_read::SerialRead>>,
    pub port_info: Arc<RwLock<port_info::PortsInfo>>,
    pub error_log: Arc<Mutex<String>>,
}

impl SharedData {
    pub fn new(max_data_points: usize) -> Self {
        Self {
            read_data: Arc::new(RwLock::new(serial_read::SerialRead::new(max_data_points))),
            port_info: Arc::new(RwLock::new(port_info::PortsInfo::new())),
            error_log: Arc::new(Mutex::new(String::new())),
        }
    }
}

// send to backend to change settings
pub enum Event {
    SelectPort(String),
    SelectBaudRate(u32),
    RefreshAvailablePorts,
    ChangeMaxDataPoints(usize),
    SendText(String),
    ClearLog,
    Shutdown,
}
