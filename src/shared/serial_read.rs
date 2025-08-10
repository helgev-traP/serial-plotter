// src/backend/shared_data.rs

use std::collections::VecDeque;
use chrono::{DateTime, Utc};

/// フロントエンドとバックエンドで共有されるデータ全体。
/// この構造体が Arc<RwLock<...>> でラップされる。
#[derive(Clone, Debug)]
pub struct SerialRead {
    /// シリアルモニタ用の生データ。リングバッファとして機能する。
    /// index 0: 現在受信中の行（書き込み可能）
    /// index 1..: 確定した過去の行（読み取り専用）
    pub raw_data: VecDeque<String>,

    /// シリアルプロッタ用のパース済みデータ。
    /// 各VecDequeが1つのデータ系列に対応する。
    /// 全てのVecDequeは常に同じ長さを保ち、矩形を維持する。
    pub graph_data: Vec<VecDeque<Option<f64>>>,

    /// 各行が確定したときのタイムスタンプ。raw_data[1..]とgraph_dataの各要素に対応する。
    pub timestamps: VecDeque<DateTime<Utc>>,

    /// 起動してからの総行数カウンタ。X軸の連番として利用する。
    pub line_counter: usize,

    /// raw_dataとgraph_dataが保持する最大行数。
    pub max_data_points: usize,
}

impl SerialRead {
    pub fn new(max_data_points: usize) -> Self {
        let mut raw_data = VecDeque::with_capacity(max_data_points);
        // 初期状態として、書き込み対象の空文字列を一つ入れておく
        raw_data.push_front(String::new());

        Self {
            raw_data,
            graph_data: Vec::new(),
            timestamps: VecDeque::with_capacity(max_data_points),
            line_counter: 0,
            max_data_points,
        }
    }

    pub fn change_max_data_points(&mut self, new_max: usize) {
        self.max_data_points = new_max;
        // raw_dataとgraph_dataのサイズを調整
        self.raw_data.truncate(new_max);
        self.timestamps.truncate(new_max);
        for series in &mut self.graph_data {
            series.truncate(new_max);
        }
    }
}
