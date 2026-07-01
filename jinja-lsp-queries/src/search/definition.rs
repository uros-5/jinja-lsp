use std::collections::{HashMap, HashSet, LinkedList};

use tree_sitter::{Point, Query, QueryCapture, QueryCursor, StreamingIterator, Tree};

use crate::{
    search::{Identifier, IdentifierType},
    tree_builder::JinjaDiagnostic,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub id: usize,
    pub keyword: String,
    pub start: Point,
    pub end: Point,
}

impl Scope {
    pub fn new(end: Point) -> Self {
        Self {
            start: Point::default(),
            end,
            keyword: String::new(),
            id: 0,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ScopeError {
    WrongEndScopeKeyword(Scope),
    ElifStatement(Scope),
    ElseStatement(Scope),
}

impl ScopeError {
    pub fn diagnostic(&self) -> (JinjaDiagnostic, Identifier) {
        let mut identifier: Identifier = Default::default();
        match self {
            ScopeError::WrongEndScopeKeyword(scope) => {
                identifier.start = scope.start;
                identifier.end = scope.end;
            }
            ScopeError::ElifStatement(scope) => {
                identifier.start = scope.start;
                identifier.end = scope.end;
            }
            ScopeError::ElseStatement(scope) => {
                identifier.start = scope.start;
                identifier.end = scope.end;
            }
        }

        (JinjaDiagnostic::ScopeError(self.clone()), identifier)
    }
}

#[derive(Default, Debug)]
pub struct JinjaDefinitions {
    pub current_scope: LinkedList<Scope>,
    pub definitions: HashMap<usize, HashMap<usize, Identifier>>,
    pub statements: HashSet<usize>,
    pub errors: Vec<ScopeError>,
    scope_id: usize,
    last_keyword: String,
    keyword_location: (Point, Point),
    last_point: Point,
}

impl JinjaDefinitions {
    pub fn new_scope(&mut self, keyword: String, scope_start: Point) {
        if self.last_point == scope_start {
            return;
        }
        self.scope_id += 1;
        let scope = Scope {
            id: self.scope_id,
            keyword: keyword,
            start: scope_start,
            end: scope_start,
        };
        self.current_scope.push_front(scope);
        self.definitions.insert(self.scope_id, Default::default());
    }
    pub fn check(&mut self, name: &str, capture: &QueryCapture<'_>, source: &str) -> Option<bool> {
        match name {
            "error" => return None,
            "macro_name" => {
                if self.statements.contains(&capture.node.id()) {
                    return Some(true);
                }
                let scope = self.current_scope.front()?;
                self.statements.insert(capture.node.id());
                let node_id = capture.node.id();
                let definitions = self.definitions.get_mut(&scope.id)?;
                let exist = definitions.contains_key(&node_id);
                if exist {
                    return Some(true);
                }
                let name = capture.node.utf8_text(source.as_bytes()).unwrap();
                let mut macro_name = Identifier::new(
                    name,
                    capture.node.start_position(),
                    capture.node.end_position(),
                );
                macro_name.scope_ends = (scope.id, capture.node.end_position());
                macro_name.identifier_type = IdentifierType::MacroName;
                let _ = self
                    .definitions
                    .get_mut(&scope.id)
                    .is_some_and(|hm| hm.insert(node_id, macro_name).is_none());

                self.statements.insert(capture.node.id());
                self.new_scope("macro".to_string(), capture.node.end_position());
            }
            "macro_parameter" => {
                let node_id = capture.node.id();
                let scope = self.current_scope.front()?;
                let definitions = self.definitions.get_mut(&scope.id)?;
                if definitions.contains_key(&node_id) {
                    return Some(true);
                }
                let name = capture.node.utf8_text(source.as_bytes()).unwrap();
                let mut macro_name = Identifier::new(
                    name,
                    capture.node.start_position(),
                    capture.node.end_position(),
                );
                macro_name.scope_ends = (scope.id, capture.node.end_position());
                macro_name.identifier_type = IdentifierType::MacroParameter;
                definitions.insert(node_id, macro_name);
            }
            "set_variable" => {
                if self.statements.contains(&capture.node.id()) {
                    return Some(true);
                }
                let scope = self.current_scope.front()?;
                // self.statements.insert(capture.node.id());
                let defs = self.definitions.get_mut(&scope.id)?;
                if defs.contains_key(&capture.node.id()) {
                    return Some(true);
                }
                let name = capture.node.utf8_text(source.as_bytes()).unwrap();
                let mut set_variable = Identifier::new(
                    name,
                    capture.node.start_position(),
                    capture.node.end_position(),
                );
                set_variable.identifier_type = IdentifierType::SetVariable;
                set_variable.scope_ends.0 = scope.id;
                defs.insert(capture.node.id(), set_variable);
                let last = capture.node.parent()?.child_count();
                if last == 4 {
                    let last = capture.node.parent()?.child(last - 1)?;
                    self.new_scope("set".to_string(), last.end_position());
                }
            }
            "with_definition" => {
                if self.statements.contains(&capture.node.id()) {
                    return Some(true);
                }
                self.statements.insert(capture.node.id());
                self.new_scope("with".to_string(), capture.node.end_position());
            }
            "with_variable" => {
                let scope = self.current_scope.front()?;
                let defs = self.definitions.get_mut(&scope.id)?;
                if defs.contains_key(&capture.node.id()) {
                    return Some(true);
                }
                let name = capture.node.utf8_text(source.as_bytes()).unwrap();
                let mut set_variable = Identifier::new(
                    name,
                    capture.node.start_position(),
                    capture.node.end_position(),
                );
                set_variable.identifier_type = IdentifierType::WithVariable;
                set_variable.scope_ends.0 = scope.id;
                defs.insert(capture.node.id(), set_variable);
            }
            "for_statement" => {
                if self.statements.contains(&capture.node.id()) {
                    return Some(true);
                }
                self.statements.insert(capture.node.id());
                self.new_scope("for".to_string(), capture.node.end_position());
            }
            "for_key" => {
                let scope = self.current_scope.front()?;
                let defs = self.definitions.get_mut(&scope.id)?;
                if defs.contains_key(&capture.node.id()) {
                    return Some(true);
                }
                let name = capture.node.utf8_text(source.as_bytes()).unwrap();
                let mut set_variable = Identifier::new(
                    name,
                    capture.node.start_position(),
                    capture.node.end_position(),
                );
                set_variable.identifier_type = IdentifierType::ForLoopKey;
                set_variable.scope_ends.0 = scope.id;
                defs.insert(capture.node.id(), set_variable);
            }
            "for_value" => {
                let scope = self.current_scope.front()?;
                let defs = self.definitions.get_mut(&scope.id)?;
                if defs.contains_key(&capture.node.id()) {
                    return Some(true);
                }
                let name = capture.node.utf8_text(source.as_bytes()).unwrap();
                let mut set_variable = Identifier::new(
                    name,
                    capture.node.start_position(),
                    capture.node.end_position(),
                );
                set_variable.identifier_type = IdentifierType::ForLoopValue;
                set_variable.scope_ends.0 = scope.id as usize;
                defs.insert(capture.node.id(), set_variable);
            }
            "keyword" => {
                self.last_keyword = capture
                    .node
                    .utf8_text(source.as_bytes())
                    .unwrap()
                    .to_string();
                self.keyword_location =
                    (capture.node.start_position(), capture.node.end_position());
            }

            "statement_start" => {
                if self.statements.contains(&capture.node.id()) {
                    return Some(true);
                }
                self.statements.insert(capture.node.id());
                let mut end_if_scope = false;
                if self.last_keyword == "elif" || self.last_keyword == "else" {
                    let parent_scope_check = self
                        .current_scope
                        .front()
                        .is_some_and(|scope| scope.keyword == "elif" || scope.keyword == "if");
                    if !parent_scope_check {
                        let scope = Scope {
                            id: 0,
                            keyword: self.last_keyword.to_string(),
                            start: self.keyword_location.0,
                            end: self.keyword_location.1,
                        };
                        if scope.keyword == "elif" {
                            self.errors.push(ScopeError::ElifStatement(scope));
                        } else if scope.keyword == "else" {
                            self.errors.push(ScopeError::ElseStatement(scope));
                        }
                    } else {
                        end_if_scope = true;
                    }
                }
                if end_if_scope {
                    let mut scope = self.current_scope.pop_front()?;
                    scope.end = capture.node.start_position();
                    let definitions = self.definitions.get_mut(&scope.id)?;
                    for definition in definitions {
                        definition.1.scope_ends.1 = scope.end;
                    }
                }
                self.new_scope(self.last_keyword.to_string(), capture.node.end_position());
            }
            "statement_end" => {
                if self.statements.contains(&capture.node.id()) {
                    return Some(true);
                }
                self.statements.insert(capture.node.id());
                let mut scope = self.current_scope.pop_front()?;
                scope.end = capture.node.start_position();
                if !self.last_keyword.ends_with(&scope.keyword) {
                    let is_error = {
                        if self.last_keyword == "endif" {
                            match scope.keyword.as_str() {
                                "if" | "elif" | "else" => false,
                                _ => true,
                            }
                        } else {
                            true
                        }
                    };
                    if is_error {
                        let mut scope = scope.clone();
                        scope.start = self.keyword_location.0;
                        scope.end = self.keyword_location.1;
                        self.errors
                            .push(ScopeError::WrongEndScopeKeyword(scope.clone()));
                    }
                }
                let definitions = self.definitions.get_mut(&scope.id)?;
                for (_, definition) in definitions {
                    definition.scope_ends.1 = scope.end;
                }
                return Some(true);
            }
            "block_variable" => {
                let scope = self.current_scope.front()?;
                let name = capture.node.utf8_text(source.as_bytes()).unwrap();
                let definitions = self.definitions.get_mut(&scope.id)?;
                let mut identifier = Identifier::new(
                    name,
                    capture.node.start_position(),
                    capture.node.end_position(),
                );
                identifier.identifier_type = IdentifierType::TemplateBlock;
                definitions.insert(capture.node.id(), identifier);
            }

            _ => {}
        }
        return Some(true);
        // None
    }

    pub fn collect(&self) -> Vec<Identifier> {
        let mut all = vec![];
        for (_, defs) in &self.definitions {
            for (_, ident) in defs {
                all.push(ident.clone());
            }
        }
        all.sort();
        return all;
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
    let mut scope = Scope::default();
    scope.end = tree.root_node().end_position();
    definitions.current_scope.push_front(scope);
    definitions.definitions.insert(0, Default::default());
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let mut matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            if all || capture.node.start_position() <= trigger_point {
                let name = &capture_names[capture.index as usize];
                let err = definitions.check(name, capture, text);
                if err.is_none() {
                    break;
                }
            }
        }
    }
    if let Some(scope) = definitions.current_scope.front() {
        if let Some(definitions) = definitions.definitions.get_mut(&0) {
            for (_, id) in definitions {
                id.scope_ends.1 = scope.end;
            }
        }
    }
    definitions
}
