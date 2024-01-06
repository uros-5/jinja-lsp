use std::collections::HashMap;

use tree_sitter::{Node, Point, QueryCapture};

use crate::{
    test_queries::CompletionType,
    tree_builder::{IdentifierState, JinjaVariable},
};

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
        } else if key == "end_statement" {
            self.state.parse_end_statement(capture, source);
        }
    }
}

#[derive(Default, Debug)]
pub struct JinjaObjectCapturer {
    objects: Vec<JinjaObject>,
    dot: (Point, Point),
    pipe: (Point, Point),
    expr: (Point, Point),
    ident: (Point, Point),
}

impl JinjaObjectCapturer {
    pub fn show(&self) -> Vec<JinjaObject> {
        self.objects.clone()
    }

    fn add_operator(&mut self, capture: &QueryCapture<'_>, dot: u8) {
        let start = capture.node.start_position();
        let end = capture.node.end_position();
        if dot == 0 {
            self.dot = (start, end);
        } else if dot == 1 {
            self.pipe = (start, end);
        } else if dot == 2 {
            self.expr = (start, end);
        }
    }

    pub fn in_pipe(&self, trigger_point: Point) -> bool {
        trigger_point >= self.pipe.0 && trigger_point <= self.pipe.1
    }

    pub fn in_expr(&self, trigger_point: Point) -> bool {
        trigger_point >= self.expr.0 && trigger_point <= self.expr.1 && trigger_point > self.ident.1
    }

    pub fn completion(&self, trigger_point: Point) -> Option<CompletionType> {
        if self.in_pipe(trigger_point) {
            return Some(CompletionType::Pipe);
        } else if self.in_expr(trigger_point) {
            return Some(CompletionType::Identifier);
        }
        None
    }

    pub fn build_object(&mut self, capture: &QueryCapture<'_>, source: &str) {
        let value = capture.node.utf8_text(source.as_bytes());
        let start = capture.node.start_position();
        let end = capture.node.end_position();
        if let Ok(value) = value {
            if start.row == self.dot.1.row && start.column == self.dot.1.column {
                match self
                    .objects
                    .last_mut()
                    .map(|last| {
                        last.fields.push((String::from(value), (start, end)));
                        self.ident = (start, end);
                    })
                    .is_none()
                {
                    true => {
                        self.objects
                            .push(JinjaObject::new(String::from(value), start, end));
                        self.ident = (start, end);
                    }
                    false => (),
                }
            } else {
                self.objects
                    .push(JinjaObject::new(String::from(value), start, end));
                self.ident = (start, end);
            }
        }
    }
}

impl Capturer for JinjaObjectCapturer {
    fn save_by(
        &mut self,
        capture: &QueryCapture<'_>,
        hm: &mut HashMap<String, CaptureDetails>,
        capture_names: &[String],
        source: &str,
    ) {
        let key = capture_names[capture.index as usize].to_owned();
        if key == "just_id" {
            self.build_object(capture, source);
        } else if key == "dot" {
            self.add_operator(capture, 0);
        } else if key == "pipe" {
            self.add_operator(capture, 1);
        } else if key == "expr" {
            self.add_operator(capture, 2);
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct JinjaObject {
    name: String,
    location: (Point, Point),
    fields: Vec<(String, (Point, Point))>,
}

impl JinjaObject {
    pub fn new(name: String, start: Point, end: Point) -> Self {
        Self {
            name,
            location: (start, end),
            fields: vec![],
        }
    }

    pub fn add_field(&mut self, field: String, start: Point, end: Point) {
        self.fields.push((field, (start, end)));
    }
}

#[derive(Default, Debug, Clone)]
pub struct RustMacro {
    variables: HashMap<String, (Point, Point)>,
    id: usize,
}

impl RustMacro {
    pub fn show(&self) -> &HashMap<String, (Point, Point)> {
        &self.variables
    }
}

#[derive(Default, Debug, Clone)]
pub struct RustCapturer {
    macros: HashMap<usize, RustMacro>,
}

impl RustCapturer {
    pub fn add_macro(&mut self, capture: &QueryCapture<'_>, source: &str) {
        let id = capture.node.id();
        if self.macros.get(&id).is_none() {
            let mut context_macro = RustMacro::default();
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
        context_macro: &mut RustMacro,
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

    pub fn show(&self) -> &HashMap<usize, RustMacro> {
        &self.macros
    }
}

impl Capturer for RustCapturer {
    fn save_by(
        &mut self,
        capture: &QueryCapture<'_>,
        hm: &mut HashMap<String, CaptureDetails>,
        capture_names: &[String],
        source: &str,
    ) {
        let key = capture_names[capture.index as usize].to_owned();
        if key == "context_macro" {
            self.add_macro(capture, source);
        }
    }
}
