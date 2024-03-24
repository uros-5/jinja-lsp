use std::collections::HashMap;

use tree_sitter::{Point, Query, QueryCapture, QueryCursor, Tree};

use super::Identifier;

macro_rules! range_start {
    ($self: ident, $capture: ident, $member: ident) => {
        let def = $self.last()?;
        if let Definition::$member { start, .. } = def {
            let end_point = $capture.node.end_position();
            *start = end_point;
            $self.current = Current::NoDefinition;
        }
    };
}

macro_rules! range_end {
    ($self: ident, $capture: ident, $member: ident) => {
        let id = $capture.node.id();
        $self.current = Current::NoDefinition;
        let v = $self.end.get_mut(&Current::$member)?;
        let item = v.iter().find(|i| i.0 == id);
        if item.is_none() {
            v.push(($capture.node.id(), $capture.node.start_position()));
        }
    };
}

#[derive(Debug, Clone)]
pub enum Definition {
    ForLoop {
        id: usize,
        key: Identifier,
        value: Option<Identifier>,
        start: Point,
        end: Point,
        open_par: bool,
        comma: bool,
        closed_par: bool,
    },
    Set {
        id: usize,
        key: Identifier,
        start: Point,
        end: Point,
        equals: bool,
    },
    With {
        id: usize,
        keys: Vec<Identifier>,
        start: Point,
        end: Point,
    },
    Macro {
        id: usize,
        keys: Vec<Identifier>,
        start: Point,
        end: Point,
    },
    Block {
        id: usize,
        name: Identifier,
        start: Point,
        end: Point,
    },
}

impl Definition {
    pub fn id(&self) -> &usize {
        match &self {
            Definition::ForLoop { id, .. } => id,
            Definition::Set { id, .. } => id,
            Definition::With { id, .. } => id,
            Definition::Macro { id, .. } => id,
            Definition::Block { id, .. } => id,
        }
    }

    fn key<'a>(&'a self, ids: &mut Vec<&'a Identifier>) {
        match self {
            Definition::ForLoop { key, .. } => {
                ids.push(key);
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
    #[default]
    NoDefinition,
}

#[derive(Default)]
pub struct JinjaDefinitions {
    pub definitions: Vec<Definition>,
    pub current: Current,
    pub end: HashMap<Current, Vec<(usize, Point)>>,
}

impl JinjaDefinitions {
    pub fn init_end(&mut self) {
        self.end.insert(Current::For, vec![]);
        self.end.insert(Current::Set, vec![]);
        self.end.insert(Current::With, vec![]);
        self.end.insert(Current::Macro, vec![]);
        self.end.insert(Current::Block, vec![]);
        self.end.insert(Current::NoDefinition, vec![]);
    }
    pub fn last(&mut self) -> Option<&mut Definition> {
        self.definitions.last_mut()
    }

    pub fn exist(&self, id: usize) -> bool {
        self.definitions.iter().any(|item| item.id() == &id)
    }

    pub fn add(&mut self, id: usize, current: Current) {
        self.current = current;
        match current {
            Current::For => {
                let d = Definition::ForLoop {
                    id,
                    key: Identifier::default(),
                    value: None,
                    start: Point::default(),
                    end: Point::default(),
                    open_par: false,
                    comma: false,
                    closed_par: false,
                };
                self.definitions.push(d);
            }
            Current::Set => {
                let d = Definition::Set {
                    id,
                    key: Identifier::default(),
                    start: Point::default(),
                    end: Point::default(),
                    equals: false,
                };
                self.definitions.push(d);
            }
            Current::With => {
                let d = Definition::With {
                    id,
                    keys: vec![],
                    start: Point::default(),
                    end: Point::default(),
                };
                self.definitions.push(d);
            }
            Current::Macro => {
                let d = Definition::Macro {
                    id,
                    keys: vec![],
                    start: Point::default(),
                    end: Point::default(),
                };
                self.definitions.push(d);
            }
            Current::Block => {
                let d = Definition::Block {
                    id,
                    name: Identifier::default(),
                    start: Point::default(),
                    end: Point::default(),
                };
                self.definitions.push(d);
            }
            Current::NoDefinition => {}
        }
    }

    fn check(&mut self, name: &str, capture: &QueryCapture<'_>, text: &str) -> Option<()> {
        match name {
            "for_start" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::For);
                } else {
                    self.current = Current::NoDefinition;
                    //
                }
            }
            "for_key" => {
                if self.current == Current::For {
                    let def = self.last()?;
                    if let Definition::ForLoop { key, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        key.name = content.to_string();
                        key.start = start;
                        key.end = end;
                    }
                }
            }
            "for_value" => {
                if self.current == Current::For {
                    let def = self.last()?;
                    if let Definition::ForLoop { value, .. } = def {
                        let mut identifier = Identifier::default();
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        identifier.name = content.to_owned();
                        identifier.start = start;
                        identifier.end = end;
                        *value = Some(identifier);
                    }
                }
            }
            "for_end" => {
                let hm = self.end.get(&Current::For)?;
                let all = hm.get(capture.node.id());
                if all.is_none() {
                    self.current = Current::For;
                } else {
                    self.current = Current::NoDefinition;
                }
            }
            "set" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::Set);
                } else {
                    self.current = Current::NoDefinition;
                }
            }
            "set_identifier" => {
                if self.current == Current::Set {
                    let def = self.last()?;
                    if let Definition::Set { key, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        key.name = content.to_string();
                        key.start = start;
                        key.end = end;
                    }
                }
            }
            "equals" => {
                if self.current == Current::Set {
                    let def = self.last()?;
                    if let Definition::Set { equals, .. } = def {
                        *equals = true;
                    }
                }
            }
            "endset" => {
                let hm = self.end.get(&Current::Set)?;
                let all = hm.get(capture.node.id());
                if all.is_none() {
                    self.current = Current::Set;
                } else {
                    self.current = Current::NoDefinition;
                }
            }
            "with" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::With);
                } else {
                    // self.current = Current::NoDefinition;
                }
            }
            "with_identifier" => {
                if self.current == Current::With {
                    let def = self.last()?;
                    if let Definition::With { keys, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        let key = Identifier::new(content, start, end);
                        keys.push(key);
                    }
                }
            }
            "endwith" => {
                let hm = self.end.get(&Current::With)?;
                let all = hm.get(capture.node.id());
                if all.is_none() {
                    self.current = Current::With;
                } else {
                    self.current = Current::NoDefinition;
                }
            }
            "block" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::Block);
                } else {
                    self.current = Current::NoDefinition;
                }
            }
            "block_identifier" => {
                if self.current == Current::Block {
                    let def = self.last()?;
                    if let Definition::Block { name, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        name.name = content.to_string();
                        name.start = start;
                        name.end = end;
                    }
                }
            }
            "endblock" => {
                let hm = self.end.get(&Current::Block)?;
                let all = hm.get(capture.node.id());
                if all.is_none() {
                    self.current = Current::Block;
                } else {
                    self.current = Current::NoDefinition;
                }
            }
            "macro" => {
                if !self.exist(capture.node.id()) {
                    self.add(capture.node.id(), Current::Macro);
                } else {
                    self.current = Current::Macro;
                    // self.current = Current::NoDefinition;
                }
            }
            "macro_identifier" => {
                if self.current == Current::Macro {
                    let def = self.last()?;
                    if let Definition::Macro { keys, .. } = def {
                        let start = capture.node.start_position();
                        let end = capture.node.end_position();
                        let content = capture.node.utf8_text(text.as_bytes()).ok()?;
                        let key = Identifier::new(content, start, end);
                        keys.push(key);
                    }
                }
            }
            "endmacro" => {
                let hm = self.end.get(&Current::Macro)?;
                let all = hm.get(capture.node.id());
                if all.is_none() {
                    self.current = Current::Macro;
                } else {
                    self.current = Current::NoDefinition;
                }
            }
            "range_start" => {
                if self.current == Current::For {
                    range_start!(self, capture, ForLoop);
                } else if self.current == Current::Set {
                    range_start!(self, capture, Set);
                } else if self.current == Current::With {
                    range_start!(self, capture, With);
                } else if self.current == Current::Block {
                    range_start!(self, capture, Block);
                } else if self.current == Current::Macro {
                    range_start!(self, capture, Macro);
                }
            }
            "range_end" => {
                if self.current == Current::For {
                    range_end!(self, capture, For);
                } else if self.current == Current::Set {
                    range_end!(self, capture, Set);
                } else if self.current == Current::With {
                    range_end!(self, capture, With);
                } else if self.current == Current::Block {
                    range_end!(self, capture, Block);
                } else if self.current == Current::Macro {
                    range_end!(self, capture, Macro);
                }
            }
            _ => (),
        };

        None
    }

    pub fn fix_end(&mut self) {
        self.fix_for_loop();
        self.fix_set();
        self.fix_with();
        self.fix_block();
        self.fix_macro();
    }

    fn fix_for_loop(&mut self) {
        let mut for_loops: Vec<_> = self
            .definitions
            .iter_mut()
            .filter(|item| matches!(item, Definition::ForLoop { .. }))
            .collect();
        for_loops.reverse();
        let ended = self.end.get_mut(&Current::For).unwrap();
        for (index, reversed) in ended.iter().enumerate() {
            if let Some(Definition::ForLoop { end, .. }) = for_loops.get_mut(index) {
                *end = reversed.1;
            }
        }
    }

    fn fix_set(&mut self) {
        let mut set_block: Vec<_> = self
            .definitions
            .iter_mut()
            .filter(|item| matches!(item, Definition::Set { equals: false, .. }))
            .collect();
        set_block.reverse();
        let ended = self.end.get_mut(&Current::Set).unwrap();
        for (index, reversed) in ended.iter().enumerate() {
            if let Some(Definition::Set { end, .. }) = set_block.get_mut(index) {
                *end = reversed.1;
            }
        }
    }

    fn fix_with(&mut self) {
        let mut with_block: Vec<_> = self
            .definitions
            .iter_mut()
            .filter(|item| matches!(item, Definition::With { .. }))
            .collect();
        with_block.reverse();
        let ended = self.end.get_mut(&Current::With).unwrap();
        for (index, reversed) in ended.iter().enumerate() {
            if let Some(Definition::With { end, .. }) = with_block.get_mut(index) {
                *end = reversed.1;
            }
        }
    }

    fn fix_block(&mut self) {
        let mut block: Vec<_> = self
            .definitions
            .iter_mut()
            .filter(|item| matches!(item, Definition::Block { .. }))
            .collect();
        block.reverse();
        let ended = self.end.get_mut(&Current::Block).unwrap();
        for (index, reversed) in ended.iter().enumerate() {
            if let Some(Definition::Block { end, .. }) = block.get_mut(index) {
                *end = reversed.1;
            }
        }
    }

    fn fix_macro(&mut self) {
        let mut macro_block: Vec<_> = self
            .definitions
            .iter_mut()
            .filter(|item| matches!(item, Definition::Macro { .. }))
            .collect();
        macro_block.reverse();
        let ended = self.end.get_mut(&Current::Macro).unwrap();
        for (index, reversed) in ended.iter().enumerate() {
            if let Some(Definition::Macro { end, .. }) = macro_block.get_mut(index) {
                *end = reversed.1;
            }
        }
    }

    pub fn identifiers(&self) -> Vec<&Identifier> {
        let mut ids = vec![];
        for id in &self.definitions {
            id.key(&mut ids);
        }
        ids
    }
}

pub fn definition_query(
    query: &Query,
    tree: Tree,
    trigger_point: Point,
    text: &str,
    all: bool,
) -> JinjaDefinitions {
    let closest_node = tree.root_node();
    let mut definitions = JinjaDefinitions::default();
    definitions.init_end();
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let matches = cursor_qry.matches(&query, closest_node, text.as_bytes());
    let captures = matches.into_iter().flat_map(|m| {
        m.captures
            .iter()
            .filter(|capture| all || capture.node.start_position() <= trigger_point)
    });
    for capture in captures {
        let name = &capture_names[capture.index as usize];
        definitions.check(name, capture, text);
    }
    definitions.fix_end();
    definitions
}
