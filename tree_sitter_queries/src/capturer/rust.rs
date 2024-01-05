use std::collections::HashMap;
use tree_sitter::Node;

use tree_sitter::{Point, QueryCapture};

use super::{CaptureDetails, Capturer};

#[derive(Default, Debug, Clone)]
pub struct RustVariables {
    variables: HashMap<String, (Point, Point)>,
    id: usize,
}

impl RustVariables {
    pub fn variables(&self) -> &HashMap<String, (Point, Point)> {
        &self.variables
    }
}

#[derive(Default, Debug, Clone)]
pub struct RustCapturer {
    macros: HashMap<usize, RustVariables>,
    variables: Vec<(String, (Point, Point))>,
}

impl RustCapturer {
    pub fn add_macro(&mut self, capture: &QueryCapture<'_>, source: &str) {
        let id = capture.node.id();
        if self.macros.get(&id).is_none() {
            let mut context_macro = RustVariables::default();
            let mut walker = capture.node.walk();
            let children = capture.node.children(&mut walker);
            let mut current = 0;
            for child in children {
                match child.kind_id() {
                    1 => current = 1,
                    55 => current = 2,
                    152 => {
                        if current == 2 {
                            self.check_token_tree(child, &mut context_macro, source);
                            self.macros.insert(id, context_macro);
                            break;
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    pub fn check_token_tree(
        &mut self,
        node: Node<'_>,
        context_macro: &mut RustVariables,
        source: &str,
    ) {
        let mut walker = node.walk();
        let children = node.children(&mut walker);
        for child in children {
            if child.kind_id() == 1 {
                let text = child.utf8_text(source.as_bytes());
                if let Ok(id) = text {
                    if context_macro.variables.get(id).is_none() {
                        let start = child.start_position();
                        let end = child.end_position();
                        context_macro.variables.insert(id.to_string(), (start, end));
                    }
                }
            }
        }
    }

    pub fn macros(&self) -> &HashMap<usize, RustVariables> {
        &self.macros
    }

    pub fn variables(&self) -> &Vec<(String, (Point, Point))> {
        &self.variables
    }

    fn add_name(&mut self, capture: &QueryCapture<'_>, source: &str) {
        if let Ok(id) = capture.node.utf8_text(source.as_bytes()) {
            let start = capture.node.start_position();
            let end = capture.node.end_position();
            let id = id.to_string();
            let id = id.replace('"', "");
            self.variables.push((id.to_string(), (start, end)));
        }
    }
}

impl Capturer for RustCapturer {
    fn save_by(&mut self, capture: &QueryCapture<'_>, capture_names: &[String], source: &str) {
        let key = capture_names[capture.index as usize].to_owned();
        if key == "context_macro" {
            self.add_macro(capture, source);
        } else if key == "name" {
            self.add_name(capture, source);
        }
    }
}
