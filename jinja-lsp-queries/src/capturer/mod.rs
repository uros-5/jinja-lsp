pub mod included;
pub mod init;
pub mod object;
pub mod rust;

use tree_sitter::{Point, QueryCapture};

pub trait Capturer {
    fn save_by(&mut self, capture: &QueryCapture<'_>, capture_names: &[String], source: &str);

    fn value(&self, capture: &QueryCapture<'_>, source: &str) -> String {
        let value = if let Ok(capture_value) = capture.node.utf8_text(source.as_bytes()) {
            capture_value.to_owned()
        } else {
            "".to_owned()
        };
        value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaptureDetails {
    pub start_position: Point,
    pub end_position: Point,
    pub value: String,
}
