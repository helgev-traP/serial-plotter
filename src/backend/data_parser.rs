// src/backend/data_parser.rs

use once_cell::sync::Lazy;
use regex::Regex;

// 正規表現を一度だけコンパイルして再利用する
static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^,\s]+").unwrap());

/// 1行の文字列をパースし、数値(f64)のVecに変換する。
/// パースに失敗した値はNoneとなる。
pub fn parse_line_to_values(line: &str) -> Vec<Option<f64>> {
    RE.find_iter(line)
        .map(|m| m.as_str().parse::<f64>().ok())
        .collect()
}
