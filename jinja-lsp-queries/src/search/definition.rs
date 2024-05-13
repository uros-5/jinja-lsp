use std::collections::{HashSet, LinkedList};

use tree_sitter::{Point, Query, QueryCapture, QueryCursor, Tree};

use super::{Identifier, IdentifierType};

#[derive(Debug, Clone)]
pub enum Definition {
    ForLoop {
        key: Identifier,
        value: Option<Identifier>,
    },
    Set {
        key: Identifier,
        equals: bool,
    },
    With {
        keys: Vec<Identifier>,
    },
    Macro {
        keys: Vec<Identifier>,
        scope: usize,
    },
    Block {
        name: Identifier,
    },
}

impl Definition {
    fn collect(self, ids: &mut Vec<Identifier>) {
        match self {
            Definition::ForLoop { mut key, value, .. } => {
                key.identifier_type = IdentifierType::ForLoopKey;
                ids.push(key);
                if let Some(mut value) = value {
                    value.identifier_type = IdentifierType::ForLoopValue;
                    ids.push(value);
                }
            }
            Definition::Set { mut key, .. } => {
                key.identifier_type = IdentifierType::SetVariable;
                ids.push(key);
            }
            Definition::With { keys, .. } => {
                for mut key in keys {
                    key.identifier_type = IdentifierType::WithVariable;
                    ids.push(key);
                }
            }
            Definition::Macro { keys, .. } => {
                for mut key in keys.into_iter().enumerate() {
                    if key.0 == 0 {
                        key.1.identifier_type = IdentifierType::MacroName;
                    } else {
                        key.1.identifier_type = IdentifierType::MacroParameter;
                    }
                    ids.push(key.1);
                }
            }
            Definition::Block { mut name, .. } => {
                name.identifier_type = IdentifierType::TemplateBlock;
                ids.push(name);
            }
        }
    }
}

impl From<&str> for Definition {
    fn from(value: &str) -> Self {
        match value {
            "for" => Self::ForLoop {
                key: Identifier::default(),
                value: None,
            },
            "set" => Self::Set {
                key: Identifier::default(),
                equals: false,
            },
            "with" => Self::With { keys: vec![] },
            "macro" => Definition::Macro {
                keys: vec![],
                scope: 0,
            },
            "block" => Self::Block {
                name: Identifier::default(),
            },
            _ => Self::ForLoop {
                key: Identifier::default(),
                value: None,
            },
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Current {
    For,
    Set,
    With,
    Macro,
    Block,
    If,
    Else,
    Filter,
    Autoescape,
    Raw,
    #[default]
    NoDefinition,
}

impl Current {
    fn _from_end(name: &str) -> Self {
        match name {
            "endfor" => Self::For,
            "endset" => Self::Set,
            "endwith" => Self::With,
            "endmacro" => Self::Macro,
            "endblock" => Self::Block,
            "endif" => Self::If,
            "endelse" => Self::Else,
            "endfilter" => Self::Filter,
            "endautoescape" => Self::Autoescape,
            "endraw" => Self::Raw,
            _ => Self::NoDefinition,
        }
    }
}

#[derive(Default, Debug)]
pub struct Scope {
    pub id: usize,
    pub start: Point,
    pub end: Point,
}

impl Scope {
    pub fn new(end: Point) -> Self {
        Self {
            id: 0,
            start: Point::default(),
            end,
        }
    }
}

#[derive(Default, Debug)]
pub struct JinjaDefinitions {
    pub definitions: Vec<Definition>,
    can_close_scope: bool,
    can_open_scope: bool,
    can_add_id: bool,
    is_end: bool,
    pub current_scope: LinkedList<Scope>,
    pub all_scopes: Vec<Scope>,
    all_ids: HashSet<usize>,
}

impl JinjaDefinitions {
    fn check(&mut self, name: &str, capture: &QueryCapture<'_>, text: &str) -> Option<bool> {
        let id = capture.node.id();
        match name {
            "definition" => {
                if self.all_ids.contains(&id) {
                    return Some(false);
                }
                let content = capture.node.utf8_text(text.as_bytes()).unwrap();
                self.all_ids.insert(id);
                let mut add_new_scope = true;
                let mut definition = Definition::from(content);
                if let Definition::Set { .. } = definition {
                    add_new_scope = false;
                } else if let Definition::Macro { ref mut scope, .. } = &mut definition {
                    let current_scope = self.current_scope.front().unwrap_or(&Scope::default()).id;
                    *scope = current_scope;
                }
                self.definitions.push(definition);
                if add_new_scope {
                    self.current_scope.push_front(Scope {
                        id,
                        ..Default::default()
                    });
                    self.can_close_scope = false;
                    self.can_open_scope = true;
                    self.is_end = false;
                    self.can_add_id = true;
                } else {
                    self.can_add_id = true;
                }
            }
            "scope" => {
                self.can_close_scope = false;
                self.can_open_scope = true;
                self.is_end = false;
                self.can_add_id = false;
                self.current_scope.push_front(Scope {
                    id,
                    ..Default::default()
                });
            }
            "endblock" => {
                self.is_end = true;
                self.can_close_scope = true;
                self.can_open_scope = false;
            }
            "equals" => {
                let last = self.definitions.last_mut();
                if let Some(Definition::Set { equals, .. }) = last {
                    *equals = true;
                    self.can_open_scope = false;
                }
            }
            "error" => {
                return None;
            }
            "id" => {
                if !self.can_add_id {
                    return Some(false);
                }
                let mut identifier = Identifier::default();
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                let current_scope = self.current_scope.front().unwrap_or(&Scope::default()).id;
                identifier.start = start;
                identifier.end = end;
                content.to_owned().clone_into(&mut identifier.name);
                identifier.scope_ends.0 = current_scope;
                let last = self.definitions.last_mut();
                if let Some(last) = last {
                    match last {
                        Definition::ForLoop { key, value } => {
                            if key.name.is_empty() {
                                *key = identifier;
                            } else if let Some(value) = value {
                                if value.name.is_empty() {
                                    *value = identifier;
                                    self.can_add_id = false;
                                }
                            }
                        }
                        Definition::Set { key, .. } => {
                            if key.name.is_empty() {
                                *key = identifier;
                                self.can_add_id = false;
                            }
                        }
                        Definition::With { keys } => {
                            keys.push(identifier);
                        }
                        Definition::Macro { keys, scope } => {
                            if keys.is_empty() {
                                identifier.scope_ends.0 = *scope;
                            }
                            keys.push(identifier);
                        }
                        Definition::Block { name } => {
                            if name.name.is_empty() {
                                *name = identifier;
                                self.can_add_id = false;
                            }
                        }
                    }
                }
            }
            "scope_end" => {
                if self.can_close_scope && self.is_end {
                    self.can_close_scope = false;
                    self.can_add_id = false;
                    self.is_end = false;
                    if let Some(mut last) = self.current_scope.pop_front() {
                        last.end = capture.node.start_position();
                        self.all_scopes.push(last);
                    }
                }
            }
            "scope_start" => {
                if self.can_open_scope {
                    self.can_open_scope = false;
                    self.can_add_id = false;
                    if let Some(last) = self.current_scope.front_mut() {
                        last.start = capture.node.end_position();
                    }
                }
            }
            _ => {}
        }
        Some(true)
    }

    pub fn identifiers(self) -> Vec<Identifier> {
        let mut ids = vec![];
        for id in self.definitions {
            id.collect(&mut ids);
        }

        ids
    }

    fn fix_end(&mut self, last_line: Point) {
        let global_scope = Scope::new(last_line);
        for def in self.definitions.iter_mut() {
            match def {
                Definition::ForLoop { key, value } => {
                    let scope = self
                        .all_scopes
                        .iter()
                        .find(|item| item.id == key.scope_ends.0);
                    let scope = scope.unwrap_or(&global_scope);
                    key.scope_ends.1 = scope.end;
                    if let Some(value) = value {
                        value.scope_ends.1 = scope.end;
                    }
                }
                Definition::Set { key, .. } => {
                    let scope = self
                        .all_scopes
                        .iter()
                        .find(|item| item.id == key.scope_ends.0);
                    let scope = scope.unwrap_or(&global_scope);
                    key.scope_ends.1 = scope.end;
                }
                Definition::With { keys } => {
                    for key in keys {
                        let scope = self
                            .all_scopes
                            .iter()
                            .find(|item| item.id == key.scope_ends.0);
                        let scope = scope.unwrap_or(&global_scope);
                        key.scope_ends.1 = scope.end;
                    }
                }
                Definition::Macro { keys, scope } => {
                    let scope = self.all_scopes.iter().find(|item| item.id == *scope);
                    let scope = scope.unwrap_or(&global_scope);
                    for (index, key) in keys.iter_mut().enumerate() {
                        if index == 0 {
                            key.scope_ends.1 = scope.end;
                        } else {
                            let scope = self
                                .all_scopes
                                .iter()
                                .find(|item| item.id == key.scope_ends.0);
                            let scope = scope.unwrap_or(&global_scope);
                            key.scope_ends.1 = scope.end;
                        }
                    }
                }
                Definition::Block { name } => {
                    let scope = self
                        .all_scopes
                        .iter()
                        .find(|item| item.id == name.scope_ends.0);
                    let scope = scope.unwrap_or(&global_scope);
                    name.scope_ends.1 = scope.end;
                }
            }
            // let id = self.all_scopes.iter().find(|scope| scope.id ==  )
        }
    }
}

pub fn definition_query(
    query: &Query,
    tree: &Tree,
    trigger_point: Point,
    text: &str,
    all: bool,
) -> JinjaDefinitions {
    let closest_node = tree.root_node();
    let mut definitions = JinjaDefinitions::default();
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
        let err = definitions.check(name, capture, text);
        if err.is_none() {
            break;
        }
    }
    let root = tree.root_node().end_position();
    definitions.fix_end(root);
    definitions
}
