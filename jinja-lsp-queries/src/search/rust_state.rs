use super::{rust_identifiers::RustIdentifiers, rust_template_completion::RustTemplates};

#[derive(Default)]
pub struct RustState {
    rust_identifiers: RustIdentifiers,
    rust_templates: RustTemplates,
}

impl RustState {
    pub fn reset(&mut self) {
        self.rust_identifiers = RustIdentifiers::default();
        self.rust_templates = RustTemplates::default();
    }
}
