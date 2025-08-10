use crossbeam::channel::Sender;
use eframe::{App, egui};

use crate::shared::{Event, SharedData, serial_read::SerialRead};

const BUTTON_WIDTH: f32 = 70.0;
const BUTTON_HEIGHT: f32 = 20.0;
const DEFAULT_PLOT_RANGE: usize = 1000;
const REPAINT_AFTER_MILLIS: u64 = 1000;
const SELECTED_BUTTON_COLOR: egui::Color32 = egui::Color32::from_rgb(20, 100, 180);

pub struct Frontend {
    shared_data: SharedData,
    event_sender: Sender<Event>,

    port_menu_open: bool,
    baud_rate_menu_open: bool,

    enter_max_data_points: EnterMaxDataPoints,

    text_sender: String,

    show_type: ShowType,

    plot_range: usize,
}

enum EnterMaxDataPoints {
    Value(usize),
    Typing {
        current_value: usize,
        string: String,
    },
}

impl EnterMaxDataPoints {
    fn ui(&mut self, event_sender: &mut Sender<Event>, ui: &mut eframe::egui::Ui) {
        match self {
            EnterMaxDataPoints::Value(val) => {
                let button = egui::Button::new(format!("Data holds:  {val}"));
                let button = ui.add_sized(eframe::egui::vec2(BUTTON_WIDTH * 2.0, BUTTON_HEIGHT), button);
                if button.clicked() {
                    *self = EnterMaxDataPoints::Typing {
                        current_value: *val,
                        string: val.to_string(),
                    };
                }
            }
            EnterMaxDataPoints::Typing {
                current_value,
                string,
            } => {
                let text_edit = ui.add_sized(
                    eframe::egui::vec2(BUTTON_WIDTH * 2.0, BUTTON_HEIGHT),
                    egui::TextEdit::singleline(string)
                        .hint_text("Enter max data points")
                        .desired_width(BUTTON_WIDTH - 20.0),
                );

                if text_edit.lost_focus() && ui.input(|i| i.key_pressed(eframe::egui::Key::Enter)) {
                    if let Ok(new_value) = string.parse::<usize>() {
                        event_sender
                            .send(Event::ChangeMaxDataPoints(new_value))
                            .expect("Failed to send ChangeMaxDataPoints event");
                        *self = EnterMaxDataPoints::Value(new_value);
                    } else {
                        *self = EnterMaxDataPoints::Value(*current_value);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ShowType {
    SerialMonitor,
    SerialPlotter,
}

impl Frontend {
    pub fn new(shared_data: SharedData, event_sender: Sender<Event>) -> Self {
        let enter_max_data_points =
            EnterMaxDataPoints::Value(shared_data.read_data.read().max_data_points);
        Self {
            shared_data,
            event_sender,
            port_menu_open: false,
            baud_rate_menu_open: false,
            enter_max_data_points,
            text_sender: String::new(),
            show_type: ShowType::SerialMonitor,
            plot_range: DEFAULT_PLOT_RANGE,
        }
    }
}

impl App for Frontend {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        egui::containers::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            self.menu(ui);
            ui.add_space(5.0);
        });

        egui::containers::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            self.text_sender(ui);
            if !self.shared_data.error_log.lock().is_empty() {
                ui.add_space(5.0);
                ui.label(self.shared_data.error_log.lock().clone());
            }
            ui.add_space(2.0);
        });

        egui::containers::CentralPanel::default().show(ctx, |ui| match self.show_type {
            ShowType::SerialMonitor => self.monitor(ui),
            ShowType::SerialPlotter => self.plotter(ui),
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(REPAINT_AFTER_MILLIS));
    }
}

impl Frontend {
    fn menu(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.with_layout(
                eframe::egui::Layout::left_to_right(eframe::egui::Align::Center),
                |ui| {
                    ui.label("Port:");
                    let port_menu_button = ui.add_sized(
                        eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                        egui::Button::new(
                            self.shared_data
                                .port_info
                                .read()
                                .selected_port
                                .clone()
                                .unwrap_or_else(|| "Select Port".into()),
                        ),
                    );
                    if port_menu_button.clicked() {
                        self.port_menu_open = !self.port_menu_open;
                    }
                    if self.port_menu_open {
                        egui::Popup::menu(&port_menu_button).show(|ui| {
                            ui.set_min_width(BUTTON_WIDTH);
                            for port in self.shared_data.port_info.read().available_ports.iter() {
                                let button = ui.add_sized(
                                    eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                                    egui::Button::new(port),
                                );
                                if button.clicked() {
                                    self.event_sender
                                        .send(Event::SelectPort(port.clone()))
                                        .expect("Failed to send SelectPort event");
                                }
                            }
                            ui.separator();
                            let refresh_button = ui.add_sized(
                                eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                                egui::Button::new("Refresh"),
                            );
                            if refresh_button.clicked() {
                                self.event_sender
                                    .send(Event::RefreshAvailablePorts)
                                    .expect("Failed to send RefreshAvailablePorts event");
                            }
                        });
                    }
                    ui.label("Baud Rate:");
                    let baud_rate_menu_button = ui.add_sized(
                        eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                        egui::Button::new(self.shared_data.port_info.read().baud_rate.to_string()),
                    );
                    if baud_rate_menu_button.clicked() {
                        self.baud_rate_menu_open = !self.baud_rate_menu_open;
                    }
                    if self.baud_rate_menu_open {
                        egui::Popup::menu(&baud_rate_menu_button).show(|ui| {
                            ui.set_min_width(BUTTON_WIDTH);
                            for &baud_rate in self
                                .shared_data
                                .port_info
                                .read()
                                .available_baud_rates
                                .iter()
                            {
                                let button = ui.add_sized(
                                    eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                                    egui::Button::new(baud_rate.to_string()),
                                );
                                if button.clicked() {
                                    self.event_sender
                                        .send(Event::SelectBaudRate(baud_rate))
                                        .expect("Failed to send SelectBaudRate event");
                                }
                            }
                        });
                    }

                    ui.separator();

                    self.enter_max_data_points.ui(&mut self.event_sender, ui);

                    let clear_log_button = ui.add_sized(
                        eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                        egui::Button::new("Clear Log"),
                    );
                    if clear_log_button.clicked() {
                        self.event_sender
                            .send(Event::ClearLog)
                            .expect("Failed to send ClearLog event");
                    }
                },
            );

            ui.with_layout(
                eframe::egui::Layout::right_to_left(eframe::egui::Align::Center),
                |ui| {
                    let mut plotter_button = egui::Button::new("Plotter");
                    if self.show_type == ShowType::SerialPlotter {
                        plotter_button = plotter_button.fill(SELECTED_BUTTON_COLOR);
                    }
                    let plotter_button = ui.add_sized(
                        eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                        plotter_button,
                    );
                    if plotter_button.clicked() {
                        self.show_type = ShowType::SerialPlotter;
                    }

                    let mut monitor_button = egui::Button::new("Monitor");
                    if self.show_type == ShowType::SerialMonitor {
                        monitor_button = monitor_button.fill(SELECTED_BUTTON_COLOR);
                    }
                    let monitor_button = ui.add_sized(
                        eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                        monitor_button,
                    );
                    if monitor_button.clicked() {
                        self.show_type = ShowType::SerialMonitor;
                    }
                },
            );
        });
    }

    fn text_sender(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Send:");

            let size = ui.available_size()[0];

            let text_edit_width = size - BUTTON_WIDTH - 10.0;

            ui.add_sized(
                eframe::egui::vec2(text_edit_width, BUTTON_HEIGHT),
                egui::TextEdit::singleline(&mut self.text_sender),
            );

            let send_button = ui.add_sized(
                eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                egui::Button::new("Send"),
            );

            if send_button.clicked() && !self.text_sender.is_empty() {
                self.event_sender
                    .send(Event::SendText(self.text_sender.clone()))
                    .expect("Failed to send SendText event");
                self.text_sender.clear();
            }
        });
    }

    fn monitor(&self, ui: &mut eframe::egui::Ui) {
        let read_data = self.shared_data.read_data.read();

        egui::ScrollArea::vertical()
            .scroll([true, true])
            // .stick_to_bottom(true) // 新しい要素追加時に一番下に追従
            .show(ui, |ui| {
                for line in read_data.raw_data.iter().rev() {
                    ui.label(line.to_string());
                }
            });
    }

    fn plotter(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.with_layout(
                eframe::egui::Layout::centered_and_justified(eframe::egui::Direction::LeftToRight),
                |ui| {
                    let range_input = ui.add_sized(
                        eframe::egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT),
                        egui::DragValue::new(&mut self.plot_range)
                            .speed(1.0)
                            .prefix("Plot Range: "),
                    );
                    if range_input.changed() {
                        self.plot_range = self.plot_range.max(1);
                    }
                },
            );
        });

        ui.add_space(5.0);

        Self::graph(&self.shared_data.read_data.read(), self.plot_range, ui);
    }

    fn graph(serial_read: &SerialRead, plot_range: usize, ui: &mut eframe::egui::Ui) {
        // --- ステージ1 & 2: データ抽出、サニタイズ、座標マッピング ---
        let data_guard = &serial_read.graph_data;

        // 各データ系列を処理し、プロット可能な座標のベクタに変換する。
        // この処理はイテレータチェーンを駆使して効率的に行われる。
        let processed_series: Vec<Vec<[f64; 2]>> = data_guard
            .iter()
            .map(|each_data_series| {
                each_data_series
                    .iter()
                    .take(plot_range) // 指定された範囲のデータポイントを抽出
                    // 先頭が最新
                    .enumerate() // X軸のインデックスを付与
                    .filter_map(|(index, &value)| {
                        Some([(serial_read.line_counter - index - 1) as f64, value?])
                    }) // [x, y]形式のPlotPointに変換
                    .rev()
                    // 末尾が最新
                    .collect()
            })
            .collect();

        // --- ステージ3: 動的なY軸境界の事前計算 ---
        let mut plot = egui_plot::Plot::new("serial plot")
            .x_axis_label("Index")
            .y_axis_label("Value")
            .legend(egui_plot::Legend::default());

        // 描画対象の全ポイントをフラットなリストに集める
        let all_points: Vec<[f64; 2]> = processed_series
            .iter()
            .flat_map(|series_points| series_points.iter())
            .cloned()
            .collect();

        if all_points.is_empty() {
            // 表示するデータがない場合は、デフォルトの表示範囲を設定する
            plot = plot.include_y(0.0).include_y(1.0);
        } else {
            // f64はOrdを実装していないため、foldを使用して最小/最大値を見つける
            let min_y = all_points
                .iter()
                .map(|p| p[1]) // Noneを除外
                .fold(f64::INFINITY, |min, p| min.min(p));
            let max_y = all_points
                .iter()
                .map(|p| p[1])
                .fold(f64::NEG_INFINITY, |max, p| max.max(p));

            // グラフが見やすくなるように、上下に5%のマージンを追加する
            let margin = (max_y - min_y) * 0.05;
            // マージンが0（全データが同じ値）の場合のフォールバック
            let final_margin = if margin > 0.0 { margin } else { 1.0 };

            plot = plot
                .include_y(min_y - final_margin)
                .include_y(max_y + final_margin);
        }

        // --- ステージ4: プロットのレンダリング ---
        plot.show(ui, |plot_ui| {
            // 事前に定義された色のリスト
            let colors = [
                egui::Color32::from_rgb(100, 200, 100),
                egui::Color32::from_rgb(200, 100, 100),
                egui::Color32::from_rgb(100, 100, 200),
                egui::Color32::from_rgb(200, 150, 100),
            ];

            for (i, series_points) in processed_series.iter().enumerate() {
                if !series_points.is_empty() {
                    let line = egui_plot::Line::new(
                        i.to_string(),
                        egui_plot::PlotPoints::new(series_points.clone()),
                    )
                    .name(format!("Series {}", i + 1))
                    .color(colors[i % colors.len()]);
                    plot_ui.line(line);
                }
            }
        });
    }
}
