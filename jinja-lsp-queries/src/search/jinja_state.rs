use super::{definition::JinjaDefinitions, objects::JinjaObjects, templates::JinjaImports};

#[derive(Default)]
pub struct JinjaState {
    jinja_definitions: JinjaDefinitions,
    jinja_objects: JinjaObjects,
    jinja_imports: JinjaImports,
}

impl JinjaState {
    pub fn reset(&mut self) {
        self.jinja_definitions = Default::default();
        self.jinja_objects = Default::default();
        self.jinja_imports = Default::default();
    }
}
