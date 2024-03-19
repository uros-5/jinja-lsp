use tree_sitter::QueryCapture;

use crate::tree_builder::{IdentifierState, JinjaVariable};

use super::Capturer;

#[derive(Default, Debug)]
pub struct JinjaInitCapturer {
    pub data: bool,
    pub state: IdentifierState,
    pub states: Vec<IdentifierState>,
}

impl JinjaInitCapturer {
    pub fn id_exist(&self, capture: &QueryCapture<'_>) -> bool {
        let id = capture.node.id();
        self.states.iter().any(|item| item.id == id)
    }

    pub fn to_vec(&self) -> Vec<JinjaVariable> {
        let mut all = vec![];
        for state in &self.states {
            state.keyword.get_data(&mut all, state);
        }
        all
    }
}

impl Capturer for JinjaInitCapturer {
    fn save_by(&mut self, capture: &QueryCapture<'_>, capture_names: &[String], source: &str) {
        let key = capture_names[capture.index as usize].to_owned();
        if key == "start_statement" {
            if !self.id_exist(capture) {
                let mut state = IdentifierState::default();
                state.parse_start_statement(capture, source);
                self.states.push(state);
            }
        } else if key == "end_statement" {
            self.state.parse_end_statement(capture, source);
        }
    }
}
