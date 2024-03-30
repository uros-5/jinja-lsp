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
            Definition::ForLoop { key, value, .. } => {
                ids.push(key);
                if let Some(value) = value {
                    ids.push(value);
                }
            }
            Definition::Set { key, .. } => {
                ids.push(key);
            }
            Definition::With { keys, .. } => {
                for key in keys {
                    ids.push(key);
                }
            }
            Definition::Macro { keys, .. } => {
                for key in keys {
                    ids.push(key);
                }
            }
            Definition::Block { name, .. } => {
                ids.push(name);
            }
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
    fn from_end(name: &str) -> Self {
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

    fn from_rest(name: &str) -> Self {
        match name {
            "for" => Current::For,
            "set" => Current::Set,
            "with" => Current::With,
            "macro" => Current::Macro,
            "block" => Current::Block,
            "if" | "elseif" => Current::If,
            "else" => Current::Else,
            "filter" => Current::Filter,
            "autoescape" => Current::Autoescape,
            "raw" => Current::Raw,
            _ => Current::NoDefinition,
        }
    }
}

#[derive(Default, Debug)]
pub struct Scope {
    id: usize,
    start: Point,
    end: Point,
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
    pub current_definition: Current,
    pub current_scope: LinkedList<Scope>,
    pub all_scopes: Vec<Scope>,
    all_ids: HashSet<usize>,
    pub in_end: bool,
}

impl JinjaDefinitions {
    pub fn exist(&self, id: usize) -> bool {
        self.all_ids.iter().any(|item| item == &id)
    }

    pub fn add(&mut self, id: usize, current: Current) {
        self.current_definition = current;
        let mut def = None;
        let mut add_scope = false;
        match current {
            Current::For => {
                def = Some(Definition::ForLoop {
                    key: Identifier::default(),
                    value: None,
                });
                add_scope = true;
            }
            Current::Set => {
                // if !self.all_ids.contains(&id) {
                self.all_ids.insert(id);
                def = Some(Definition::Set {
                    key: Identifier::default(),
                    equals: false,
                });
                // }
            }
            Current::With => {
                def = Some(Definition::With { keys: vec![] });
                add_scope = true;
            }
            Current::Macro => {
                def = Some(Definition::Macro {
                    keys: vec![],
                    scope: self.current_scope.front().unwrap_or(&Scope::default()).id,
                });
                add_scope = true;
            }
            Current::Block => {
                def = Some(Definition::Block {
                    name: Identifier::default(),
                });
                add_scope = true;
            }
            Current::NoDefinition => (),
            _ => {
                add_scope = true;
            }
        }
        if let Some(def) = def {
            // let same_def = self.definitions.iter().find(|item| item.)
            self.definitions.push(def);
            self.all_ids.insert(id);
        }
        if add_scope {
            self.current_scope.push_front(Scope {
                id,
                ..Default::default()
            });
        }
    }

    fn check(&mut self, name: &str, capture: &QueryCapture<'_>, text: &str) -> Option<bool> {
        match name {
            "error" => {
                return None;
            }
            "for" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::For);
                } else {
                    self.current_definition = Current::NoDefinition;
                }
            }

            "for_key" => {
                if self.current_definition == Current::For {
                    let def = self.definitions.last_mut()?;
                    if let Definition::ForLoop { key, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        key.name = content.to_string();
                        key.start = start;
                        key.end = end;
                        key.identifier_type = IdentifierType::ForLoopKey;
                        key.scope_ends.0 =
                            self.current_scope.front().unwrap_or(&Scope::default()).id;
                    }
                }
            }

            "for_value" => {
                if self.current_definition == Current::For {
                    let def = self.definitions.last_mut()?;
                    if let Definition::ForLoop { value, .. } = def {
                        let mut identifier = Identifier::default();
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        identifier.name = content.to_owned();
                        identifier.start = start;
                        identifier.end = end;
                        identifier.identifier_type = IdentifierType::ForLoopValue;
                        identifier.scope_ends.0 =
                            self.current_scope.front().unwrap_or(&Scope::default()).id;
                        *value = Some(identifier);
                    }
                }
            }

            "set" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::Set);
                } else {
                    self.current_definition = Current::NoDefinition;
                }
            }

            "set_identifier" => {
                if self.current_definition == Current::Set {
                    let def = self.definitions.last_mut()?;
                    if let Definition::Set { key, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        if content == key.name {
                            return None;
                        }
                        key.name = content.to_string();
                        key.start = start;
                        key.end = end;
                        let scope = self.current_scope.front().unwrap_or(&Scope::default()).id;
                        key.scope_ends.0 = scope;
                        key.identifier_type = IdentifierType::SetVariable;
                    }
                }
            }

            "equals" => {
                if self.current_definition == Current::Set {
                    let def = self.definitions.last_mut()?;
                    if let Definition::Set { equals, key } = def {
                        *equals = true;
                        key.scope_ends.0 =
                            self.current_scope.front().unwrap_or(&Scope::default()).id;
                    }
                }
            }

            "with" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::With);
                } else {
                    // self.current_definition = Current::NoDefinition;
                }
            }

            "with_identifier" => {
                if self.current_definition == Current::With {
                    let def = self.definitions.last_mut()?;
                    if let Definition::With { keys, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        let mut key = Identifier::new(content, start, end);
                        key.identifier_type = IdentifierType::WithVariable;
                        key.scope_ends.0 =
                            self.current_scope.front().unwrap_or(&Scope::default()).id;
                        keys.push(key);
                    }
                }
            }
            "block" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::Block);
                } else {
                    self.current_definition = Current::NoDefinition;
                }
            }

            "block_identifier" => {
                if self.current_definition == Current::Block {
                    let def = self.definitions.last_mut()?;
                    if let Definition::Block { name, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        name.name = content.to_string();
                        name.start = start;
                        name.end = end;
                        name.identifier_type = IdentifierType::TemplateBlock;
                        name.scope_ends.0 =
                            self.current_scope.front().unwrap_or(&Scope::default()).id;
                    }
                }
            }

            "macro" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::Macro);
                } else {
                    self.current_definition = Current::Macro;
                    // self.current_definition = Current::NoDefinition;
                }
            }

            "macro_identifier" => {
                if self.current_definition == Current::Macro {
                    let def = self.definitions.last_mut()?;
                    if let Definition::Macro { keys, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;

                        let mut key = Identifier::new(content, start, end);
                        key.scope_ends.0 =
                            self.current_scope.front().unwrap_or(&Scope::default()).id;
                        if keys.is_empty() {
                            key.identifier_type = IdentifierType::MacroName;
                        } else {
                            key.identifier_type = IdentifierType::MacroParameter;
                        }
                        keys.push(key);
                    }
                }
            }

            "if" | "elif" | "else" | "filter" | "autoescape" | "raw" => {
                let current = Current::from_rest(name);
                self.add(capture.node.id(), current);
            }
            "ended" => {
                self.in_end = true;
            }

            "range_end" => {
                if self.in_end {
                    self.in_end = false;
                    let mut last = self.current_scope.pop_front()?;
                    if last.end == Point::default() {
                        last.end = capture.node.start_position();
                        self.current_definition = Current::NoDefinition;
                        self.all_scopes.push(last);
                    }
                }
            }

            "range_start" => {
                let mut can_add = true;
                let mut is_set = false;
                if let Some(last) = self.current_scope.front_mut() {
                    if let Some(Definition::Set { equals, .. }) = self.definitions.last() {
                        is_set = true;
                        if self.current_definition == Current::Set && *equals {
                            can_add = false;
                        }
                    }
                    if is_set && can_add {
                        self.current_scope.push_front(Scope {
                            id: capture.node.id(),
                            start: capture.node.end_position(),
                            ..Default::default()
                        });
                    } else if can_add {
                        last.start = capture.node.end_position();
                    }
                } else if self.current_definition != Current::NoDefinition {
                    if let Some(Definition::Set { equals, .. }) = self.definitions.last() {
                        if self.current_definition == Current::Set && !(*equals) {
                            can_add = true;
                        }
                    }

                    if can_add {
                        self.current_scope.push_front(Scope {
                            id: capture.node.id(),
                            start: capture.node.end_position(),
                            ..Default::default()
                        });
                    }
                }
            }

            // "range_end" => {
            //     let last = self.definitions.last()?;
            //     match last {
            //         Definition::ForLoop { .. } => {
            //             self.current = Current::For;
            //         }
            //         Definition::Set { .. } => {
            //             self.current = Current::Set;
            //         }
            //         Definition::With { .. } => {
            //             self.current = Current::With;
            //         }
            //         Definition::Macro { .. } => {
            //             self.current = Current::Macro;
            //         }
            //         Definition::Block { .. } => {
            //             self.current = Current::Block;
            //         }
            //     }
            // }
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
