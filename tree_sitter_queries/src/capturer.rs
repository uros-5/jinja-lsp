use std::collections::HashMap;

use tree_sitter::{Point, QueryCapture};

use crate::tree_builder::IdentifierState;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaptureDetails {
    pub start_position: Point,
    pub end_position: Point,
    pub value: String,
}

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
pub struct JinjaInitCapturer {
    pub data: bool,
    pub state: IdentifierState,
    pub states: Vec<IdentifierState>,
}

impl JinjaInitCapturer {
    pub fn abc(&self) {}

    pub fn id_exist(&self, capture: &QueryCapture<'_>) -> bool {
        let id = capture.node.id();
        self.states.iter().any(|item| item.id == id)
    }
}

impl Capturer for JinjaInitCapturer {
    fn save_by(
        &mut self,
        capture: &QueryCapture<'_>,
        hm: &mut HashMap<String, CaptureDetails>,
        capture_names: &[String],
        source: &str,
    ) {
        let key = capture_names[capture.index as usize].to_owned();
        if key == "start_statement" {
            if !self.id_exist(capture) {
                let mut state = IdentifierState::default();
                state.parse_start_statement(capture, source);
                self.states.push(state);
            }
            // self.state.iter_data();
        } else if key == "end_statement" {
            self.state.parse_end_statement(capture, source);
        }
        //
    }
}
