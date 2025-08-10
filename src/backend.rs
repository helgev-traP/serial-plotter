pub mod data_parser;

use core::str;
use std::collections::VecDeque;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use chrono::Utc;
use crossbeam::channel::{Receiver, TryRecvError};
use serialport::{ClearBuffer, SerialPort};

use self::data_parser::parse_line_to_values;
use crate::shared::serial_read::SerialRead;
use crate::shared::{Event, SharedData};

pub struct Backend {
    shared_data: SharedData,
    port: Option<Box<dyn SerialPort>>,

    // receiver for events from the frontend
    event_receiver: Receiver<Event>,
}

impl Backend {
    pub fn new(shared_data: SharedData, event_receiver: Receiver<Event>) -> Self {
        Self {
            shared_data,
            port: None,
            event_receiver,
        }
    }

    fn reconnect(&mut self) {
        // 現在のポートをドロップ
        self.port = None;

        let port_info = self.shared_data.port_info.read();

        if let Some(port_name) = &port_info.selected_port {
            let baud_rate = port_info.baud_rate;
            match serialport::new(port_name, baud_rate).open() {
                Ok(port) => {
                    self.port = Some(port);
                    if let Some(p) = self.port.as_mut() {
                        p.set_timeout(Duration::from_millis(10)).unwrap();
                        p.clear(ClearBuffer::All).unwrap();
                    }
                    println!("Reconnected to port {port_name} with baud rate {baud_rate}");
                }
                Err(e) => {
                    eprintln!("Failed to reconnect to port {port_name}: {e}");
                }
            }
        }
    }

    /// イベントを処理し、スレッドを継続するかどうかを返す
    /// `true`なら継続、`false`なら終了
    fn handle_event(&mut self, event: Event) -> bool {
        match event {
            Event::SelectPort(port_name) => {
                {
                    let mut port_info = self.shared_data.port_info.write();
                    port_info.selected_port = Some(port_name);
                }
                self.reconnect();
                true // 継続
            }
            Event::SelectBaudRate(baud_rate) => {
                {
                    let mut port_info = self.shared_data.port_info.write();
                    port_info.baud_rate = baud_rate;
                }
                self.reconnect();
                true // 継続
            }
            Event::RefreshAvailablePorts => {
                match serialport::available_ports() {
                    Ok(ports) => {
                        let port_names = ports.into_iter().map(|p| p.port_name).collect();
                        self.shared_data.port_info.write().available_ports = port_names;
                    }
                    Err(e) => {
                        eprintln!("Failed to get available ports: {e}");
                        *self.shared_data.error_log.lock() =
                            format!("Failed to get available ports: {e}");
                    }
                }
                true // 継続
            }
            Event::ChangeMaxDataPoints(max_data_points) => {
                self.shared_data.read_data.write().change_max_data_points(max_data_points);
                true // 継続
            }
            Event::SendText(text) => {
                if let Some(port) = self.port.as_mut() {
                    if let Err(e) = port.write_all(text.as_bytes()) {
                        eprintln!("Failed to send text: {e}");
                        *self.shared_data.error_log.lock() = format!("Failed to send text: {e}");
                    }
                } else {
                    eprintln!("No port selected to send text");
                    *self.shared_data.error_log.lock() =
                        "No port selected to send text".to_string();
                }
                true // 継続
            }
            Event::ClearLog => {
                self.shared_data.read_data.write().clear();
                true // 継続
            }
            Event::Shutdown => {
                println!("Shutdown event received. Exiting loop.");
                false // 終了
            }
        }
    }

    pub fn start_backend_thread(mut self) -> JoinHandle<Self> {
        thread::spawn(move || {
            if let Some(port) = self.port.as_mut() {
                port.set_timeout(Duration::from_millis(10)).unwrap();
            }

            loop {
                let mut should_continue = true;
                match self.event_receiver.try_recv() {
                    Ok(event) => {
                        should_continue = self.handle_event(event);
                    }
                    Err(TryRecvError::Empty) => { /* イベントがなければ何もしない */ }
                    Err(TryRecvError::Disconnected) => {
                        eprintln!("Event channel disconnected");
                        should_continue = false;
                    }
                }

                if !should_continue {
                    break;
                }

                if let Some(port) = self.port.as_mut() {
                    let mut serial_buf: [u8; 1024] = [0; 1024];
                    match port.read(&mut serial_buf) {
                        Ok(bytes_read) if bytes_read > 0 => {
                            let received_str = String::from_utf8_lossy(&serial_buf[..bytes_read]);
                            self.shared_data.read(&received_str);
                        }
                        Ok(_) => {} // 0バイト読み込み
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                            // タイムアウトは正常なので何もしない
                        }
                        Err(e) => {
                            eprintln!("Reading error: {e}");
                            *self.shared_data.error_log.lock() = format!("Reading error: {e}");
                            self.port = None;
                            self.shared_data.port_info.write().selected_port = None;
                        }
                    }
                }

                thread::sleep(Duration::from_millis(1));
            }
            self
        })
    }
}

impl SharedData {
    fn read(&self, s: &str) {
        self.read_data.write().read(s);
    }
}

impl SerialRead {
    pub fn clear(&mut self) {
        self.raw_data.clear();
        self.graph_data.clear();
        self.timestamps.clear();
        self.line_counter = 0;
        self.raw_data.push_front(String::new());
    }

    fn read(&mut self, mut received_str: &str) {
        // received_strが空になるまでループ
        while !received_str.is_empty() {
            // 1. 先頭から最初の改行まで（または文字列の終わりまで）を一行として切り出す
            let (is_line_completed, line_end_index) = received_str
                .find('\n')
                .map(|idx| (true, idx)) // 改行が見つかった場合
                .unwrap_or((false, received_str.len())); // 見つからなかった場合

            let line_to_append = &received_str[..line_end_index];
            if is_line_completed {
                received_str = &received_str[line_end_index + 1..];
            } else {
                received_str = ""; // 文字列の終わりまで処理したので空にする
            }

            // 2. 現在の行バッファ（raw_data[0]）に追記
            self.raw_data[0].push_str(line_to_append);

            // 3. 行が確定した場合（改行が見つかった場合）の処理
            if is_line_completed {
                // 確定した行をクローンして処理に回す
                let completed_line = self.raw_data[0].clone();

                // パース処理
                let values = parse_line_to_values(&completed_line);
                let num_new_series = values.len();
                let num_existing_series = self.graph_data.len();

                // graph_dataの矩形維持
                if num_new_series > num_existing_series {
                    for _ in 0..(num_new_series - num_existing_series) {
                        let mut new_series = VecDeque::with_capacity(self.max_data_points);
                        for _ in 0..self.timestamps.len() {
                            new_series.push_back(None);
                        }
                        self.graph_data.push(new_series);
                    }
                }

                // graph_dataの更新
                for i in 0..self.graph_data.len() {
                    let value = values.get(i).cloned().flatten();
                    self.graph_data[i].push_front(value);
                }

                // timestampsとカウンタの更新
                self.timestamps.push_front(Utc::now());
                self.line_counter += 1;

                // 新しい空の行を先頭に用意
                self.raw_data.push_front(String::new());

                // リングバッファのサイズ維持
                let max_points = self.max_data_points;
                if self.raw_data.len() > max_points + 1 {
                    self.raw_data.pop_back();
                }
                if self.timestamps.len() > max_points {
                    self.timestamps.pop_back();
                }
                for series in &mut self.graph_data {
                    if series.len() > max_points {
                        series.pop_back();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::shared::port_info::PortsInfo;
    use crossbeam::channel;
    use parking_lot::{Mutex, RwLock};

    fn setup_test_backend() -> (Backend, SharedData, channel::Sender<Event>) {
        let (tx, rx) = channel::unbounded();
        let shared_data = SharedData {
            read_data: Arc::new(RwLock::new(SerialRead::new(100))),
            port_info: Arc::new(RwLock::new(PortsInfo {
                available_ports: vec![],
                available_baud_rates: vec![],
                selected_port: None,
                baud_rate: 115200,
            })),
            error_log: Arc::new(Mutex::new(String::new())),
        };
        let backend = Backend::new(shared_data.clone(), rx);
        (backend, shared_data, tx)
    }

    #[test]
    fn test_shutdown_event() {
        let (backend, _, tx) = setup_test_backend();
        let handle = backend.start_backend_thread();
        tx.send(Event::Shutdown).unwrap();
        let backend_returned = handle.join().unwrap();
        assert!(backend_returned.port.is_none());
    }

    #[test]
    fn test_clear_log_event() {
        let (backend, shared_data, tx) = setup_test_backend();

        {
            let mut read_data = shared_data.read_data.write();
            read_data.read("1,2,3\n");
        }

        {
            let read_data = shared_data.read_data.read();
            assert_eq!(read_data.line_counter, 1);
            assert_eq!(read_data.graph_data.len(), 3);
        }

        let handle = backend.start_backend_thread();
        tx.send(Event::ClearLog).unwrap();
        thread::sleep(Duration::from_millis(50));

        {
            let read_data = shared_data.read_data.read();
            assert_eq!(read_data.line_counter, 0);
            assert!(read_data.graph_data.is_empty());
            assert_eq!(read_data.raw_data.len(), 1);
            assert_eq!(read_data.raw_data[0], "");
        }

        tx.send(Event::Shutdown).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn test_serial_read_logic() {
        let mut read_data = SerialRead::new(10);

        read_data.read("1.1,2.2\n");
        assert_eq!(read_data.line_counter, 1);
        assert_eq!(read_data.raw_data.len(), 2);
        assert_eq!(read_data.raw_data[1], "1.1,2.2");
        assert_eq!(read_data.graph_data.len(), 2);
        assert_eq!(read_data.graph_data[0][0], Some(1.1));
        assert_eq!(read_data.graph_data[1][0], Some(2.2));
        assert_eq!(read_data.timestamps.len(), 1);

        read_data.read("3.3,4.4,5.5\n");
        assert_eq!(read_data.line_counter, 2);
        assert_eq!(read_data.graph_data.len(), 3);
        assert_eq!(read_data.graph_data[0][1], Some(1.1));
        assert_eq!(read_data.graph_data[1][1], Some(2.2));
        assert_eq!(read_data.graph_data[2][1], None);
        assert_eq!(read_data.graph_data[0][0], Some(3.3));
        assert_eq!(read_data.graph_data[1][0], Some(4.4));
        assert_eq!(read_data.graph_data[2][0], Some(5.5));

        read_data.read("6.6\n");
        assert_eq!(read_data.line_counter, 3);
        assert_eq!(read_data.graph_data.len(), 3);
        assert_eq!(read_data.graph_data[0][0], Some(6.6));
        assert_eq!(read_data.graph_data[1][0], None);
        assert_eq!(read_data.graph_data[2][0], None);
    }

    #[test]
    fn test_serial_read_incomplete_lines() {
        let mut read_data = SerialRead::new(10);

        // 1. 途中で途切れたデータを受信
        read_data.read("1,2\n3,");
        assert_eq!(read_data.line_counter, 1);
        assert_eq!(read_data.raw_data.len(), 2);
        assert_eq!(read_data.raw_data[1], "1,2");
        assert_eq!(read_data.raw_data[0], "3,"); // 未完了行がバッファに残る

        // 2. 残りのデータを受信
        read_data.read("4\n5,6\n");
        assert_eq!(read_data.line_counter, 3);
        assert_eq!(read_data.raw_data.len(), 4);
        assert_eq!(read_data.raw_data[1], "5,6"); // 最新の完了行
        assert_eq!(read_data.raw_data[2], "3,4"); // 結合された行
        assert_eq!(read_data.raw_data[0], ""); // 完了しているのでバッファは空
    }
}
