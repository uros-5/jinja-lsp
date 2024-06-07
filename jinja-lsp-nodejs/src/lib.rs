#![deny(clippy::all)]

use jinja_lsp::{
  channels::diagnostics::DiagnosticMessage,
  filter::{init_filter_completions, FilterCompletion},
  lsp_files::LspFiles,
};
use jinja_lsp_queries::{
  parsers::Parsers,
  search::{objects::objects_query, queries::Queries, Identifier, IdentifierType},
  to_input_edit::to_position2,
};

use tower_lsp::lsp_types::{
  DidOpenTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
  HoverParams, Location, MarkupContent, MarkupKind, PartialResultParams, Position, Range,
  TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Url,
  WorkDoneProgressParams,
};
use tree_sitter::Point;

#[macro_use]
extern crate napi_derive;

#[napi]
pub fn basic(content: String) -> Option<i32> {
  let queries = Queries::default();
  let mut parsers = Parsers::default();
  let tree = parsers.parse(
    jinja_lsp_queries::tree_builder::LangType::Template,
    &content,
    None,
  )?;
  let query = &queries.jinja_objects;
  let objects = objects_query(query, &tree, Point::new(0, 0), &content, true);
  let count = objects.show();
  Some(count.len() as i32)
}

#[napi]
#[derive(Default)]
pub struct NodejsLspFiles {
  lsp_files: LspFiles,
  counter: u32,
  filters: Vec<FilterCompletion>,
}

#[napi]
impl NodejsLspFiles {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      lsp_files: LspFiles::default(),
      counter: 0,
      filters: init_filter_completions(),
    }
  }

  /// Actions can come from unsaved context.
  #[napi]
  pub fn add_global_context(&self, actions: Option<Vec<String>>) {}

  #[napi]
  pub fn delete_all(&mut self, filename: String) {
    self.lsp_files.variables.clear();
    self.lsp_files.delete_documents();
    self.counter = 0;
    // self.lsp_files.main_channel
  }

  #[napi]
  pub fn add_one(
    &mut self,
    id: u32,
    filename: String,
    content: String,
    line: u32,
  ) -> Vec<JsIdentifier> {
    let params: DidOpenTextDocumentParams = DidOpenTextDocumentParams {
      text_document: TextDocumentItem::new(
        Url::parse(&format!("file:///home/{filename}.{id}.jinja")).unwrap(),
        String::new(),
        0,
        content,
      ),
    };
    let content = self.lsp_files.did_open(params);
    let mut all_errors = vec![];
    if let Some(content) = content {
      match content {
        DiagnosticMessage::Errors(errors) => {
          for i in errors {
            for error in i.1 {
              let diagnostic = error.0.to_string();
              let mut position = error.1;
              position.start.row += line as usize;
              position.end.row += line as usize;
              let mut identifier = JsIdentifier::from(&position);
              identifier.error = Some(diagnostic);
              all_errors.push(identifier);
            }
          }
        }
        DiagnosticMessage::Str(_) => {}
      }
    }
    all_errors
  }

  #[napi]
  pub fn get_variables(&self, id: String, line: u32) -> Option<Vec<JsIdentifier>> {
    let variables = self.lsp_files.variables.get(&id)?;
    let mut converted = vec![];
    for variable in variables {
      let mut variable2 = JsIdentifier::from(variable);
      variable2.start.line += line;
      variable2.end.line += line;
      converted.push(variable2);
    }
    Some(converted)
  }

  #[napi]
  pub fn hover(
    &self,
    id: u32,
    filename: String,
    line: u32,
    mut position: JsPosition,
  ) -> Option<JsHover> {
    position.line -= line;
    let uri = Url::parse(&format!("file:///home/{filename}.{id}.jinja")).unwrap();
    let params: HoverParams = HoverParams {
      text_document_position_params: TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(uri.clone()),
        Position::new(position.line, position.character),
      ),
      work_done_progress_params: WorkDoneProgressParams {
        work_done_token: None,
      },
    };
    let hover = self.lsp_files.hover(params)?;
    let mut res = None;
    let mut range = Range {
      start: to_position2(hover.0.start),
      end: to_position2(hover.0.end),
    };
    range.start.line += line;
    range.end.line += line;
    let range = Some(range);
    if hover.1 {
      let filter = self
        .filters
        .iter()
        .find(|name| name.name == hover.0.name && hover.1);
      if let Some(filter) = filter {
        let markup_content = MarkupContent {
          kind: MarkupKind::Markdown,
          value: filter.desc.to_string(),
        };
        let hover_contents = HoverContents::Markup(markup_content);
        let hover = Hover {
          contents: hover_contents,
          range,
        };
        res = Some(hover);
      }
    } else if let Some(data_type) = self.lsp_files.data_type(uri.clone(), hover.0) {
      let markup_content = MarkupContent {
        kind: MarkupKind::Markdown,
        value: data_type.completion_detail().to_owned(),
      };
      let hover_contents = HoverContents::Markup(markup_content);
      let hover = Hover {
        contents: hover_contents,
        range,
      };
      res = Some(hover);
    }
    if let Some(res) = res {
      if let HoverContents::Markup(hover_contents) = res.contents {
        if let Some(range) = res.range {
          return Some(JsHover {
            kind: "markdown".to_owned(),
            value: hover_contents.value,
            range: Some(JsRange::from(&range)),
          });
        }
      }
    }
    None
  }

  #[napi]
  pub fn complete(position: JsPosition, id: u32, content: String) {}

  #[napi]
  pub fn goto_definition(
    &self,
    id: u32,
    filename: String,
    line: u32,
    mut position: JsPosition,
  ) -> Option<Vec<JsLocation>> {
    position.line -= line;
    let uri = Url::parse(&format!("file:///home/{filename}.{id}.jinja")).unwrap();
    let params: GotoDefinitionParams = GotoDefinitionParams {
      text_document_position_params: TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(uri.clone()),
        Position::new(position.line, position.character),
      ),
      work_done_progress_params: WorkDoneProgressParams {
        work_done_token: None,
      },
      partial_result_params: PartialResultParams {
        ..Default::default()
      },
    };
    let defintion = self.lsp_files.goto_definition(params)?;
    let mut definitions = vec![];
    println!("here we are, {}", definitions.len());
    match defintion {
      GotoDefinitionResponse::Scalar(mut location) => {
        let uri2 = location.uri.to_string();
        if uri2.contains(&filename) {
          location.uri = Url::parse(&filename).unwrap();
          location.range.start.line += line;
          location.range.end.line += line;
          definitions.push(JsLocation::from(&location));
        }
      }
      GotoDefinitionResponse::Array(locations) => {
        for mut location in locations {
          let uri2 = location.uri.to_string();
          if uri2.contains(&filename) {
            location.uri = Url::parse(&filename).unwrap();
            location.range.start.line += line;
            location.range.end.line += line;
            definitions.push(JsLocation::from(&location));
          }
        }
      }
      _ => (),
    }
    Some(definitions)
  }
}

#[napi(object)]
#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct JsPosition {
  pub line: u32,
  pub character: u32,
}

impl From<Point> for JsPosition {
  fn from(value: Point) -> Self {
    Self {
      line: value.row as u32,
      character: value.column as u32,
    }
  }
}

impl From<&Position> for JsPosition {
  fn from(value: &Position) -> Self {
    Self {
      line: value.line,
      character: value.character,
    }
  }
}

#[napi]
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum JsIdentifierType {
  ForLoopKey,
  ForLoopValue,
  ForLoopCount,
  SetVariable,
  WithVariable,
  MacroName,
  MacroParameter,
  TemplateBlock,
  BackendVariable,
  #[default]
  UndefinedVariable,
  JinjaTemplate,
}

#[napi(object)]
#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct JsIdentifier {
  pub start: JsPosition,
  pub end: JsPosition,
  pub name: String,
  pub identifier_type: JsIdentifierType,
  pub error: Option<String>,
}

impl From<&Identifier> for JsIdentifier {
  fn from(value: &Identifier) -> Self {
    Self {
      start: JsPosition::from(value.start),
      end: JsPosition::from(value.end),
      name: value.name.to_string(),
      identifier_type: JsIdentifierType::from(&value.identifier_type),
      error: None,
    }
  }
}

impl From<&IdentifierType> for JsIdentifierType {
  fn from(value: &IdentifierType) -> Self {
    match value {
      IdentifierType::ForLoopKey => JsIdentifierType::ForLoopKey,
      IdentifierType::ForLoopValue => JsIdentifierType::ForLoopValue,
      IdentifierType::ForLoopCount => JsIdentifierType::ForLoopCount,
      IdentifierType::SetVariable => JsIdentifierType::SetVariable,
      IdentifierType::WithVariable => JsIdentifierType::WithVariable,
      IdentifierType::MacroName => JsIdentifierType::MacroName,
      IdentifierType::MacroParameter => JsIdentifierType::MacroParameter,
      IdentifierType::TemplateBlock => JsIdentifierType::TemplateBlock,
      IdentifierType::BackendVariable => JsIdentifierType::BackendVariable,
      IdentifierType::UndefinedVariable => JsIdentifierType::UndefinedVariable,
      IdentifierType::JinjaTemplate => JsIdentifierType::JinjaTemplate,
    }
  }
}

impl From<&Range> for JsRange {
  fn from(value: &Range) -> Self {
    Self {
      start: JsPosition::from(&value.start),
      end: JsPosition::from(&value.end),
    }
  }
}

#[napi(object)]
pub struct JsHover {
  pub kind: String,
  pub value: String,
  pub range: Option<JsRange>,
}

#[napi(object)]
pub struct JsRange {
  pub start: JsPosition,
  pub end: JsPosition,
}

#[napi(object)]
pub struct JsLocation {
  pub uri: String,
  pub range: JsRange,
}

impl From<&Location> for JsLocation {
  fn from(value: &Location) -> Self {
    Self {
      uri: value.uri.to_string(),
      range: JsRange::from(&value.range),
    }
  }
}
