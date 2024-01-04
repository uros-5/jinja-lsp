use std::collections::HashMap;

use tree_sitter::{Point, QueryCapture};

use super::{CaptureDetails, Capturer};

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
    fn save_by(&mut self, capture: &QueryCapture<'_>, capture_names: &[String], source: &str) {
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum CompletionType {
    Pipe,
    Identifier,
}
