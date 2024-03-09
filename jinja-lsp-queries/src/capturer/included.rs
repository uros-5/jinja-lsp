use tree_sitter::Point;

use super::Capturer;

#[derive(Default, Debug)]
pub struct IncludeCapturer {
    pub template: String,
    point: (Point, Point),
}

impl IncludeCapturer {
    pub fn in_template(&self, trigger_point: Point) -> bool {
        trigger_point >= self.point.0 && trigger_point <= self.point.1
    }
}

impl Capturer for IncludeCapturer {
    fn save_by(
        &mut self,
        capture: &tree_sitter::QueryCapture<'_>,
        capture_names: &[String],
        source: &str,
    ) {
        let key = capture_names[capture.index as usize].to_owned();
        if key == "template" {
            if let Ok(value) = capture.node.utf8_text(source.as_bytes()) {
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                self.template = value.replace(['\'', '\"'], "");
                self.point.0 = start;
                self.point.1 = end;
            }
        }
    }
}
