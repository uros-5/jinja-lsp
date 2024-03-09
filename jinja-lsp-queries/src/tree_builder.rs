use tree_sitter::{Node, Point, QueryCapture};

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum LangType {
    Template,
    Backend,
}

#[derive(Clone, Debug)]
pub enum JinjaKeyword {
    For {
        key: String,
        value: String,
        passed_open_paren: bool,
    },
    Macro {
        name: String,
        parameters: Vec<(String, (Point, Point))>,
    },
    Block {
        name: String,
    },
    Set {
        name: String,
        equals: bool,
    },
    From {
        name: String,
        import: String,
    },
    With {
        name: String,
    },
    NoKeyword,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataType {
    Macro,
    MacroParameter,
    Variable,
    BackendVariable,
    WithVariable,
    Block,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct JinjaVariable {
    pub location: (Point, Point),
    pub name: String,
    pub data_type: DataType,
}

impl JinjaVariable {
    pub fn new(name: &String, location: (Point, Point), data_type: DataType) -> Self {
        Self {
            name: String::from(name),
            location,
            data_type,
        }
    }
}

impl JinjaKeyword {
    pub fn add_identifier(&mut self, identifier: &str, range: (Point, Point)) -> Option<()> {
        match self {
            JinjaKeyword::For { key, value, .. } => {
                if key.is_empty() {
                    *key = String::from(identifier);
                    Some(())
                } else if value.is_empty() {
                    *value = String::from(identifier);
                    Some(())
                } else {
                    None
                }
            }
            JinjaKeyword::Macro { name, parameters } => {
                if name.is_empty() {
                    *name = String::from(identifier);
                    Some(())
                } else {
                    parameters.push((String::from(identifier), range));
                    Some(())
                }
            }
            JinjaKeyword::Block { name } => {
                if name.is_empty() {
                    *name = String::from(identifier);
                    Some(())
                } else {
                    None
                }
            }
            JinjaKeyword::Set { name, .. } => {
                if name.is_empty() {
                    *name = String::from(identifier);
                    Some(())
                } else {
                    None
                }
            }
            JinjaKeyword::From { name, .. } => {
                if name.is_empty() {
                    *name = String::from(identifier);
                    Some(())
                } else {
                    None
                }
            }
            JinjaKeyword::With { name } => {
                if name.is_empty() {
                    *name = String::from(identifier);
                    Some(())
                } else {
                    None
                }
            }
            JinjaKeyword::NoKeyword => None,
        }
    }

    pub fn add_operator(&mut self, operator: &str) -> Option<()> {
        match self {
            JinjaKeyword::For {
                passed_open_paren, ..
            } => {
                if !passed_open_paren.to_owned() && operator.starts_with('(') {
                    *passed_open_paren = true;
                }
                None
            }
            JinjaKeyword::Set { equals, .. } => {
                if !*equals && operator.starts_with('=') {
                    *equals = true;
                    Some(())
                } else {
                    None
                }
            }
            _ => Some(()),
        }
    }

    pub fn get_data(&self, all: &mut Vec<JinjaVariable>, data: &IdentifierState) {
        match self {
            JinjaKeyword::For { key, value, .. } => {
                let key = JinjaVariable::new(key, data.location, DataType::Variable);
                all.push(key);
                if !value.is_empty() {
                    let value = JinjaVariable::new(value, data.location, DataType::Variable);
                    all.push(value);
                }
            }
            JinjaKeyword::Macro { name, parameters } => {
                let name = JinjaVariable::new(name, data.location, DataType::Macro);
                all.push(name);
                for param in parameters {
                    let param = JinjaVariable::new(&param.0, param.1, DataType::MacroParameter);
                    all.push(param);
                }
            }
            JinjaKeyword::Block { name } => {
                let name = JinjaVariable::new(name, data.location, DataType::Block);
                all.push(name);
            }
            JinjaKeyword::Set { name, equals: _ } => {
                let name = JinjaVariable::new(name, data.location, DataType::Variable);
                all.push(name);
            }
            JinjaKeyword::From { name: _, import: _ } => {
                //
            }
            JinjaKeyword::With { name } => {
                let name = JinjaVariable::new(name, data.location, DataType::WithVariable);
                all.push(name);
            }
            JinjaKeyword::NoKeyword => {
                //
            }
        }
    }
}

pub static KEYWORDS: [&str; 7] = ["for", "macro", "block", "set", "from", "import", "with"];

#[derive(Clone, Default, Debug)]
pub struct IdentifierState {
    pub keyword: JinjaKeyword,
    pub location: (Point, Point),
    pub statement_started: bool,
    pub statement_ended: bool,
    pub id: usize,
    pub have_keyword: bool,
}

impl IdentifierState {
    pub fn parse_start_statement(&mut self, capture: &QueryCapture<'_>, source: &str) {
        let mut walker = capture.node.walk();
        let children = capture.node.children(&mut walker);
        self.id = capture.node.id();
        for child in children {
            match child.kind_id() {
                57 => self.statement_started = true,
                58 => self.statement_ended = true,
                63 => self.add_keyword(child, source),
                1 => self.add_identifier(child, source),
                50 => self.add_operator(child, source),
                _ => (),
            }
        }
    }

    pub fn parse_end_statement(&mut self, _capture: &QueryCapture<'_>, _source: &str) {
        // let mut c2 = capture.node.walk();
        // let children = capture.node.children(&mut c2);
        // for child in children {
        //     match child.kind_id() {
        //         26 => (),
        //         _ => (),
        //     }
        // }
    }

    pub fn add_keyword(&mut self, child: Node<'_>, source: &str) {
        if self.have_keyword || !self.statement_started || self.statement_ended {
            return;
        }
        let kw = child.utf8_text(source.as_bytes());
        if let Ok(kw) = kw {
            if !KEYWORDS.contains(&kw) {
                return;
            }
            let kw = JinjaKeyword::try_from(kw);
            if let Ok(kw) = kw {
                self.keyword = kw
            }
            self.have_keyword = true;
        }
    }

    pub fn add_identifier(&mut self, child: Node<'_>, source: &str) {
        if !self.have_keyword || !self.statement_started || self.statement_ended {
            return;
        }
        let identifier = child.utf8_text(source.as_bytes());
        let start = child.start_position();
        let end = child.end_position();
        if let Some(_s) = identifier
            .ok()
            .and_then(|id| {
                if id.parse::<u16>().is_ok() {
                    None
                } else {
                    Some(id)
                }
            })
            .and_then(|id| -> Option<()> { self.keyword.add_identifier(id, (start, end)) })
        {
            self.have_keyword = true;
            if self.location.0.row == 0 && self.location.0.column == 0 {
                self.location = (start, end);
            }
        }
    }

    fn add_operator(&mut self, child: Node<'_>, source: &str) {
        if !self.have_keyword || !self.statement_started || self.statement_ended {
            return;
        }
        let operator = child.utf8_text(source.as_bytes());
        operator
            .ok()
            .and_then(|operator| {
                if operator.starts_with('(')
                    || operator.starts_with(',')
                    || operator.starts_with(')')
                    || operator.starts_with('=')
                {
                    Some(operator)
                } else {
                    None
                }
            })
            .and_then(|operator| self.keyword.add_operator(operator));
    }
}

impl Default for JinjaKeyword {
    fn default() -> Self {
        Self::NoKeyword
    }
}

impl TryFrom<&str> for JinjaKeyword {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let keyword = match value {
            "for" => Some(JinjaKeyword::For {
                key: String::new(),
                value: String::new(),
                passed_open_paren: false,
            }),
            "macro" => Some(JinjaKeyword::Macro {
                name: String::new(),
                parameters: vec![],
            }),
            "block" => Some(JinjaKeyword::Block {
                name: String::new(),
            }),
            "set" => Some(JinjaKeyword::Set {
                name: String::new(),
                equals: false,
            }),
            "from" => Some(JinjaKeyword::From {
                name: String::new(),
                import: String::new(),
            }),
            "with" => Some(JinjaKeyword::With {
                name: String::new(),
            }),
            _ => None,
        };
        match keyword.is_none() {
            true => Err(()),
            false => Ok(keyword.unwrap()),
        }
    }
}

pub enum JinjaDiagnostic {
    DefinedSomewhere,
    Undefined,
}

impl ToString for JinjaDiagnostic {
    fn to_string(&self) -> String {
        match self {
            JinjaDiagnostic::Undefined => String::from("Undefined variable"),
            JinjaDiagnostic::DefinedSomewhere => String::from("Variable is defined in other file."),
        }
    }
}
