use std::collections::HashMap;

use tree_sitter::{Point, Query, QueryCapture};

use crate::{
    config::LangType,
    query_helper::{CaptureDetails, Queries},
};

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

#[derive(Default)]
pub struct JinjaVariableCapturer {
    points: Vec<(Point, Point)>,
    cnt: u32,
    force: bool,
}

impl JinjaVariableCapturer {
    pub fn force(&mut self) {
        self.force = true;
    }
    pub fn add_point(&mut self, point: (Point, Point)) -> bool {
        let check = self
            .points
            .iter()
            .find(|item| item.0 == point.0 && item.1 == point.1);
        if check.is_none() {
            self.points.push(point);
            return true;
        }
        return false;
    }
}

impl Capturer for JinjaVariableCapturer {
    fn save_by(
        &mut self,
        capture: &QueryCapture<'_>,
        acc: &mut HashMap<String, CaptureDetails>,
        capture_names: &[String],
        source: &str,
    ) {
        let mut key = capture_names[capture.index as usize].to_owned();
        let value = self.value(capture, source);
        if key == "temp_expression" || key == "just_statement" {
            let identifier = capture.node.child_by_field_name("identifier");
            if let Some(identifier) = identifier {
                let to_add =
                    self.add_point((identifier.start_position(), identifier.end_position()));
                if to_add {
                    if self.force {
                        self.cnt += 1;
                        key = format!("{}_{}", key, self.cnt);
                    } else {
                        key = "key_id".to_string();
                    }
                    let value = identifier.utf8_text(source.as_bytes());
                    if value.is_err() {
                        return;
                    }

                    acc.insert(
                        key,
                        CaptureDetails {
                            value: value.unwrap().to_string(),
                            end_position: identifier.end_position(),
                            start_position: identifier.start_position(),
                        },
                    );
                }
            }
        }
    }
}

pub fn get_capturer(lang_type: LangType, queries: &Queries) -> (&Query, Box<dyn Capturer>) {
    match lang_type {
        LangType::Template => (
            &queries.jinja_ident_query,
            Box::new(JinjaCapturer::default()),
        ),
        LangType::Backend => {
            let query = &queries.rust_ident_query;
            let mut capturer2 = RustCapturer::default();
            capturer2.force();
            (query, Box::new(capturer2))
        }
    }
}
