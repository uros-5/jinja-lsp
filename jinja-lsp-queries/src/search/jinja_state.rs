use tree_sitter::{Point, Query, Tree};

use super::{
    definition::{definition_query, JinjaDefinitions},
    objects::{objects_query, JinjaObjects},
    templates::{templates_query, JinjaImports},
};

#[derive(Default)]
pub struct JinjaState {
    jinja_definitions: JinjaDefinitions,
    jinja_objects: JinjaObjects,
    jinja_imports: JinjaImports,
}

impl JinjaState {
    pub fn init(
        trigger_point: Point,
        query: (&Query, &Query, &Query),
        tree: &Tree,
        source: &str,
        all: bool,
    ) -> Self {
        let definitions = definition_query(query.0, tree, trigger_point, source, all);
        let objects = objects_query(query.1, tree, trigger_point, source, all);
        let imports = templates_query(query.2, tree, trigger_point, source, all);
        Self {
            jinja_definitions: definitions,
            jinja_objects: objects,
            jinja_imports: imports,
        }
    }

    pub fn reset(&mut self) {
        self.jinja_definitions = Default::default();
        self.jinja_objects = Default::default();
        self.jinja_imports = Default::default();
    }
}
