use tower_lsp::lsp_types::{Position, Range, Url};
use tree_sitter::Point;

use super::{object::CompletionType, Capturer};

#[derive(Default, Debug)]
pub struct IncludeCapturer {
    pub included: Vec<IncludedTemplate>,
}

impl IncludeCapturer {
    pub fn in_template(&self, trigger_point: Point) -> Option<&IncludedTemplate> {
        if let Some(last) = self.included.last() {
            if trigger_point >= last.template.0 && trigger_point <= last.template.1 {
                return Some(last);
            }
        }
        None
    }

    pub fn find(&self, trigger_point: Point) -> Option<(&IncludedTemplate, Include)> {
        if let Some(string) = self.in_template(trigger_point) {
            return Some((string, Include::Template));
        }
        None
    }

    pub fn completion(&self, trigger_point: Point) -> Option<CompletionType> {
        let part = self.find(trigger_point)?;
        let location = part.1.location(trigger_point, part.0);
        Some(CompletionType::IncludedTemplate {
            name: location.0,
            range: location.1,
        })
    }

    pub fn add_template(&mut self, name: String, range: (Point, Point)) {
        self.included.push(IncludedTemplate {
            name,
            template: range,
            ..Default::default()
        });
    }

    pub fn last(&self) -> Option<&String> {
        Some(&self.included.last()?.name)
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
        if key == "keyword" {
            self.add_template("".to_owned(), (Point::default(), Point::default()));
        } else if key == "error" {
            if let Some(last) = self.included.last_mut() {
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                last.error = (start, end);
            }
        } else if key == "id" {
            if let Some(last) = self.included.last_mut() {
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                last.identifier = (start, end);
                if let Ok(value) = capture.node.utf8_text(source.as_bytes()) {
                    let name = value.replace(['\'', '\"'], "");
                    last.name = name;
                }
            }
        } else if key == "template" {
            if let Ok(value) = capture.node.utf8_text(source.as_bytes()) {
                if let Some(last) = self.included.last_mut() {
                    let start = capture.node.start_position();
                    let end = capture.node.end_position();
                    let name = value.replace(['\'', '\"'], "");
                    last.name = name;
                    last.template = (start, end);
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct IncludedTemplate {
    pub name: String,
    pub template: (Point, Point),
    pub error: (Point, Point),
    pub identifier: (Point, Point),
}

impl IncludedTemplate {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            template: (Point::default(), Point::default()),
            ..Default::default()
        }
    }

    pub fn is_template(&self, root: &str) -> Option<Url> {
        let template = format!("{}/{}", root, &self.name);
        let template = std::fs::canonicalize(template).ok()?;
        let url = format!("file://{}", template.to_str()?);
        let uri = Url::parse(&url).ok()?;
        Some(uri)
    }
}

#[derive(Debug)]
pub enum Include {
    Id,
    Template,
    Error,
}

impl Include {
    pub fn location(&self, trigger_point: Point, part: &IncludedTemplate) -> (String, Range) {
        match self {
            Include::Error => {
                let range = self.to_range(part.error);
                (String::from(""), range)
            }
            Include::Id => {
                let l1 = part.identifier.1.column - trigger_point.column;
                if part.name.len() < l1 {
                    let range = self.to_range(part.identifier);
                    return (String::from(""), range);
                }
                let end = part.name.len() - l1;
                let mut name = String::new();
                for (i, item) in part.name.char_indices() {
                    name.push(item);
                    if i == end {
                        break;
                    }
                }
                let range = self.to_range(part.identifier);
                (name, range)
            }
            Include::Template => {
                let l1 = part.template.1.column - trigger_point.column;
                if part.name.len() < l1 || part.name.is_empty() {
                    let range = self.to_range(part.template);
                    return (String::from(""), range);
                }
                let end = part.name.len() - l1;
                let mut name = String::new();
                for (i, item) in part.name.char_indices() {
                    name.push(item);
                    if i == end {
                        break;
                    }
                }
                let range = self.to_range(part.template);
                (name, range)
            }
        }
    }

    pub fn to_range(&self, points: (Point, Point)) -> Range {
        let start = Position::new(points.0.row as u32, points.0.column as u32);
        let end = Position::new(points.1.row as u32, points.1.column as u32);
        Range::new(start, end)
    }
}
