use tower_lsp::lsp_types::Url;
use tree_sitter::Point;

use super::Capturer;

#[derive(Default, Debug)]
pub struct IncludeCapturer {
    pub included: Vec<IncludedTemplate>,
}

impl IncludeCapturer {
    pub fn in_template(&self, trigger_point: Point) -> Option<&IncludedTemplate> {
        if let Some(last) = self.included.last() {
            if trigger_point >= last.range.0 && trigger_point <= last.range.1 {
                return Some(last);
            }
        }
        None
    }

    pub fn add_template(&mut self, name: String, range: (Point, Point)) {
        self.included.push(IncludedTemplate { name, range });
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
        if key == "template" {
            if let Ok(value) = capture.node.utf8_text(source.as_bytes()) {
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                let name = value.replace(['\'', '\"'], "");
                self.add_template(name, (start, end));
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct IncludedTemplate {
    pub name: String,
    pub range: (Point, Point),
}

impl IncludedTemplate {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            range: (Point::default(), Point::default()),
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
