use std::collections::HashMap;

use tree_sitter::{Point, Query, QueryCapture, QueryCursor, Tree};

use super::Identifier;

#[derive(Debug)]
pub enum Import {
    Extends {
        template: Identifier,
    },
    Include {
        templates: Vec<Identifier>,
    },
    From {
        template: Identifier,
        identifiers: Vec<Identifier>,
    },
    Import {
        template: Identifier,
        identifier: Identifier,
    },
}

impl Import {
    pub fn get_name(&self, trigger_point: Point) -> Option<&str> {
        match &self {
            Import::Extends { template }
            | Import::From { template, .. }
            | Import::Import { template, .. } => {
                if trigger_point >= template.start && trigger_point <= template.end {
                    Some(&template.name)
                } else {
                    None
                }
            }
            Import::Include { templates } => {
                let template = templates.iter().find(|template| {
                    trigger_point >= template.start && trigger_point <= template.end
                })?;
                Some(&template.name)
            }
        }
    }
}

#[derive(Default)]
pub enum Current {
    Id(usize),
    #[default]
    Nothing,
}

#[derive(Default)]
pub struct JinjaImports {
    pub imports: HashMap<usize, Import>,
    pub current: Current,
    pub last_id: usize,
}

impl JinjaImports {
    pub fn in_template(&self, trigger_point: Point) -> Option<&Import> {
        if let Current::Id(id) = self.current {
            let last = self.imports.get(&id)?;
            return Some(last);
        }
        None
    }
    pub fn check(&mut self, name: &str, capture: &QueryCapture<'_>, text: &str) -> Option<()> {
        match name {
            "extends" => {
                let id = capture.node.id();
                let last = self.imports.get_mut(&id);
                if last.is_some() {
                    self.current = Current::Nothing;
                } else {
                    let import = Import::Extends {
                        template: Identifier::default(),
                    };
                    self.imports.insert(id, import);
                    self.current = Current::Id(id);
                    self.last_id = id;
                }
            }
            "include" => {
                let id = capture.node.id();
                let last = self.imports.get_mut(&id);
                if last.is_some() {
                    self.current = Current::Id(id);
                } else {
                    let import = Import::Include { templates: vec![] };
                    self.imports.insert(id, import);
                    self.current = Current::Id(id);
                    self.last_id = id;
                }
            }
            "import" => {
                let id = capture.node.id();
                let last = self.imports.get_mut(&id);
                if last.is_some() {
                    self.current = Current::Id(id);
                } else {
                    let import = Import::Import {
                        template: Identifier::default(),
                        identifier: Identifier::default(),
                    };
                    self.imports.insert(id, import);
                    self.current = Current::Id(id);
                    self.last_id = id;
                }
            }
            "from" => {
                let id = capture.node.id();
                let last = self.imports.get_mut(&id);
                if last.is_some() {
                    self.current = Current::Id(id);
                } else {
                    let import = Import::From {
                        template: Identifier::default(),
                        identifiers: vec![],
                    };
                    self.imports.insert(id, import);
                    self.current = Current::Id(id);
                    self.last_id = id;
                }
            }
            "template_name" => {
                if let Current::Id(id) = self.current {
                    let last = self.imports.get_mut(&id)?;
                    let name = capture.node.utf8_text(text.as_bytes()).ok()?;
                    let name = name.replace(['\"', '\''], "");
                    let mut start = capture.node.start_position();
                    start.column += 1;
                    let mut end = capture.node.end_position();
                    end.column -= 1;
                    match last {
                        Import::Extends { template } => {
                            if template.name.is_empty() {
                                template.name = name;
                                template.start = start;
                                template.end = end;
                            }
                        }
                        Import::Include { templates } => {
                            let template = Identifier::new(&name, start, end);
                            templates.push(template);
                        }
                        Import::From { template, .. } => {
                            if template.name.is_empty() {
                                template.name = name;
                                template.start = start;
                                template.end = end;
                            }
                        }
                        Import::Import { template, .. } => {
                            if template.name.is_empty() {
                                template.name = name;
                                template.start = start;
                                template.end = end;
                            }
                        }
                    }
                }
            }
            "import_identifier" => {
                if let Current::Id(id) = self.current {
                    let name = capture.node.utf8_text(text.as_bytes()).ok()?;
                    let start = capture.node.start_position();
                    let end = capture.node.end_position();
                    let last = self.imports.get_mut(&id)?;
                    match last {
                        Import::From { identifiers, .. } => {
                            let identifier = Identifier::new(name, start, end);
                            identifiers.push(identifier);
                        }
                        Import::Import { identifier, .. } => {
                            if identifier.name.is_empty() {
                                identifier.name = String::from(name);
                                self.current = Current::Nothing;
                            }
                        }
                        _ => self.current = Current::Nothing,
                    }
                }
            }

            _ => (),
        }
        None
    }
}
pub fn templates_query(
    query: &Query,
    tree: Tree,
    trigger_point: Point,
    text: &str,
    all: bool,
) -> JinjaImports {
    let closest_node = tree.root_node();
    let mut imports = JinjaImports::default();
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    let captures = matches.into_iter().flat_map(|m| {
        m.captures
            .iter()
            .filter(|capture| all || capture.node.start_position() <= trigger_point)
    });
    for capture in captures {
        let name = &capture_names[capture.index as usize];
        imports.check(name, capture, text);
    }
    imports
}
