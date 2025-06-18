use std::collections::HashMap;

use tree_sitter::{Point, Query, QueryCapture, QueryCursor, StreamingIterator, Tree};

use super::{Identifier, IdentifierType};

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
    pub fn get_identifier(&self, trigger_point: Point) -> Option<&Identifier> {
        match &self {
            Import::Extends { template }
            | Import::From { template, .. }
            | Import::Import { template, .. } => {
                if trigger_point >= template.start && trigger_point <= template.end {
                    Some(template)
                } else {
                    None
                }
            }
            Import::Include { templates } => {
                let template = templates.iter().find(|template| {
                    trigger_point >= template.start && trigger_point <= template.end
                })?;
                Some(template)
            }
        }
    }

    fn collect(self, ids: &mut Vec<Identifier>) {
        match self {
            Import::Extends { template } => ids.push(template),
            Import::Include { templates } => {
                for i in templates {
                    ids.push(i);
                }
            }
            Import::From {
                template,
                identifiers,
            } => {
                ids.push(template);
                for i in identifiers {
                    ids.push(i);
                }
            }
            Import::Import { template, .. } => {
                ids.push(template);
            }
        }
    }
}

#[derive(Default, Debug)]
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
    pub fn in_template(&self, _: Point) -> Option<&Import> {
        if let Current::Id(id) = self.current {
            let last = self.imports.get(&id)?;
            return Some(last);
        }
        None
    }
    pub fn check(&mut self, name: &str, capture: &QueryCapture<'_>, text: &str) -> Option<()> {
        match name {
            "error" => {
                return None;
            }
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
                    let start = capture.node.start_position();
                    let end = capture.node.end_position();
                    match last {
                        Import::Extends { template } => {
                            if template.name.is_empty() {
                                template.name = name;
                                template.start = start;
                                template.end = end;
                                template.identifier_type = IdentifierType::JinjaTemplate;
                            }
                        }
                        Import::Include { templates } => {
                            let mut template = Identifier::new(&name, start, end);
                            template.identifier_type = IdentifierType::JinjaTemplate;
                            templates.push(template);
                        }
                        Import::From { template, .. } => {
                            if template.name.is_empty() {
                                template.name = name;
                                template.start = start;
                                template.end = end;
                                template.identifier_type = IdentifierType::JinjaTemplate;
                            }
                        }
                        Import::Import { template, .. } => {
                            if template.name.is_empty() {
                                template.name = name;
                                template.start = start;
                                template.end = end;
                                template.identifier_type = IdentifierType::JinjaTemplate;
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
        Some(())
    }

    pub fn collect(self, ids: &mut Vec<Identifier>) {
        for i in self.imports {
            i.1.collect(ids);
        }
    }
}
pub fn templates_query(
    query: &Query,
    tree: &Tree,
    trigger_point: Point,
    text: &str,
    all: bool,
) -> JinjaImports {
    let closest_node = tree.root_node();
    let mut imports = JinjaImports::default();
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let mut matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            if all || capture.node.start_position() <= trigger_point {
                let name = &capture_names[capture.index as usize];
                let res = imports.check(name, capture, text);
                if res.is_none() {
                    break;
                }
            }
        }
    }
    imports
}
