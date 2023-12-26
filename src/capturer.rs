use std::collections::HashMap;

use tree_sitter::QueryCapture;

use crate::query_helper::CaptureDetails;

pub trait Capturer {
    fn save_by(
        &mut self,
        capture: &QueryCapture<'_>,
        hm: &mut HashMap<String, CaptureDetails>,
        capture_names: &[String],
        source: &str,
    );

    fn value(&self, capture: &QueryCapture<'_>, source: &str) -> String {
        let value = if let Ok(capture_value) = capture.node.utf8_text(source.as_bytes()) {
            capture_value.to_owned()
        } else {
            "".to_owned()
        };
        value
    }
}

#[derive(Default)]
pub struct JinjaCapturer {
    cnt: u32,
    was_keyword: bool,
    force: bool,
}

impl JinjaCapturer {
    pub fn force(&mut self) {
        self.force = true;
    }
}

impl Capturer for JinjaCapturer {
    fn save_by(
        &mut self,
        capture: &QueryCapture<'_>,
        acc: &mut HashMap<String, CaptureDetails>,
        capture_names: &[String],
        source: &str,
    ) {
        let key = capture_names[capture.index as usize].to_owned();
        let value = self.value(capture, source);
        if value.parse::<u32>().is_ok() {
            return;
        }
        if key == "key_name" {
            self.was_keyword = true;
        } else if self.was_keyword || self.force {
            self.cnt += 1;
            self.was_keyword = false;

            acc.insert(
                format!("{}_{}", key, self.cnt),
                CaptureDetails {
                    value,
                    end_position: capture.node.end_position(),
                    start_position: capture.node.start_position(),
                },
            );
        }
    }
}

#[derive(Default)]
pub struct JinjaCapturer2 {
    block: String,
    cnt: u32,
    force: bool,
}

impl JinjaCapturer2 {
    pub fn force(&mut self) {
        self.force = true;
    }
}

impl Capturer for JinjaCapturer2 {
    fn save_by(
        &mut self,
        capture: &QueryCapture<'_>,
        acc: &mut HashMap<String, CaptureDetails>,
        capture_names: &[String],
        source: &str,
    ) {
        let key = capture_names[capture.index as usize].to_owned();
        let value = self.value(capture, source);

        if key == "temp_expression" || key == "temp_statement" {
            self.block = value;
        } else if key == "key_id" && !self.block.contains(&format!(".{value}")) {
            if value.parse::<u32>().is_ok() {
                return;
            }
            let new_key = {
                if self.force {
                    self.cnt += 1;
                    format!("{}_{}", key, self.cnt)
                } else {
                    key.to_string()
                }
            };
            acc.insert(
                new_key,
                CaptureDetails {
                    value,
                    end_position: capture.node.end_position(),
                    start_position: capture.node.start_position(),
                },
            );
        }
    }
}

#[derive(Default)]
pub struct RustCapturer {
    cnt: u32,
    force: bool,
}

impl RustCapturer {
    pub fn force(&mut self) {
        self.force = true;
    }
}

impl Capturer for RustCapturer {
    fn save_by(
        &mut self,
        capture: &QueryCapture<'_>,
        acc: &mut HashMap<String, CaptureDetails>,
        capture_names: &[String],
        source: &str,
    ) {
        let key = capture_names[capture.index as usize].to_owned();
        let value = self.value(capture, source);

        if key == "key_id" {
            let new_key = {
                if self.force {
                    self.cnt += 1;
                    format!("{}_{}", key, self.cnt)
                } else {
                    key.to_string()
                }
            };

            acc.insert(
                new_key,
                CaptureDetails {
                    value,
                    end_position: capture.node.end_position(),
                    start_position: capture.node.start_position(),
                },
            );
        }
    }
}

#[derive(Default)]
pub struct JinjaCompletionCapturer {
    pub filter_name: String,
}

impl Capturer for JinjaCompletionCapturer {
    fn save_by(
        &mut self,
        capture: &QueryCapture<'_>,
        acc: &mut HashMap<String, CaptureDetails>,
        capture_names: &[String],
        source: &str,
    ) {
        let key = capture_names[capture.index as usize].to_owned();
        let value = self.value(capture, source);
        acc.insert(
            key.to_string(),
            CaptureDetails {
                value,
                end_position: capture.node.end_position(),
                start_position: capture.node.start_position(),
            },
        );
    }
}
