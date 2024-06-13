#![deny(clippy::all)]

use std::collections::HashMap;

use jinja_lsp::{
  filter::{init_filter_completions, FilterCompletion},
  lsp_files::LspFiles,
};
use jinja_lsp_queries::{
  parsers::Parsers,
  search::{
    objects::{objects_query, CompletionType, JinjaObject},
    python_identifiers::PythonIdentifier,
    queries::Queries,
    snippets_completion::snippets,
    Identifier, IdentifierType,
  },
  to_input_edit::to_position2,
};

use tower_lsp::lsp_types::{
  CompletionContext, CompletionItem, CompletionItemKind, CompletionParams, CompletionTriggerKind,
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
  _snippets: Vec<CompletionItem>,
  actions: HashMap<String, Vec<Action>>,
  action_objects: HashMap<String, Vec<ActionObject>>,
}

#[napi]
impl NodejsLspFiles {
  #[napi(constructor)]
  pub fn new() -> Self {
    let mut lsp_files2 = LspFiles::default();
    lsp_files2.parsers.update_backend("python");
    lsp_files2.queries.update_backend("python");
    Self {
      lsp_files: lsp_files2,
      counter: 0,
      filters: init_filter_completions(),
      _snippets: snippets(),
      actions: HashMap::new(),
      action_objects: HashMap::new(),
    }
  }

  /// Actions can come from unsaved context.
  #[napi]
  pub fn add_global_context(&mut self, uri: String, actions: Option<Vec<Action>>) {
    if let Some(actions) = actions {
      let mut identifiers = vec![];
      let mut action_objects = vec![];
      for action in &actions {
        let mut identifier = Identifier::new(&action.name, Point::new(0, 0), Point::new(0, 0));
        identifier.identifier_type = IdentifierType::BackendVariable;
        identifiers.push(identifier);
        let action = ActionObject::from(action);
        action_objects.push(action);
      }
      self.actions.insert(uri.to_string(), actions);
      self.action_objects.insert(uri.to_string(), action_objects);
      self.lsp_files.variables.insert(uri, identifiers);
    }
  }

  #[napi]
  pub fn delete_all(&mut self, filename: String) {
    self.lsp_files.delete_documents_with_id(filename);
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
    ext: String,
  ) -> Vec<JsIdentifier> {
    let mut all_identifiers = vec![];
    let params: DidOpenTextDocumentParams = DidOpenTextDocumentParams {
      text_document: TextDocumentItem::new(
        Url::parse(&format!("file:///home/{filename}.{id}.{ext}")).unwrap(),
        String::new(),
        0,
        content,
      ),
    };
    self.lsp_files.did_open(params);
    match ext.as_str() {
      "jinja" => {
        let objects = self
          .lsp_files
          .read_objects(Url::parse(&format!("file:///home/{filename}.{id}.{ext}")).unwrap());
        if let Some(objects) = objects {
          if let Some(global_actions) = self.action_objects.get(&filename.to_string()) {
            for obj in &objects {
              let action_object = ActionObject::from(obj);
              for global_action in global_actions {
                if global_action.compare(&action_object.fields) {
                  let mut start = JsPosition::from(obj.location.0);
                  let mut end = JsPosition::from(obj.last_field_end());
                  start.line += line;
                  end.line += line;
                  let identifier = JsIdentifier {
                    start,
                    end,
                    name: obj.name.to_owned(),
                    identifier_type: JsIdentifierType::Link,
                    error: None,
                  };
                  all_identifiers.push(identifier);
                }
              }
            }
          }
        }
      }
      "py" => {
        if let Some(ids) = self
          .lsp_files
          .read_python_ids(Url::parse(&format!("file:///home/{filename}.{id}.{ext}")).unwrap())
        {
          if let Some(global_actions) = self.action_objects.get(&filename.to_string()) {
            for obj in &ids {
              let action_object = ActionObject::from(obj);
              for global_action in global_actions {
                if global_action.compare(&action_object.fields) {
                  let mut start = JsPosition::from(obj.start);
                  let mut end = JsPosition::from(obj.end);
                  start.line += line;
                  end.line += line;
                  let identifier = JsIdentifier {
                    start,
                    end,
                    name: obj.field.to_owned(),
                    identifier_type: JsIdentifierType::Link,
                    error: None,
                  };
                  all_identifiers.push(identifier);
                }
              }
            }
          }
        }
      }
      _ => (),
    };
    // let query = &self.lsp_files.queries.jinja_objects;
    // let objects = objects_query(query, &tree, Point::new(0, 0), &content, true);
    // let objects = objects.show();
    // if let Some(content) = content {
    //   match content {
    //     DiagnosticMessage::Errors(errors) => {
    //       for i in errors {
    //         for error in i.1 {
    //           let diagnostic = error.0.to_string();
    //           let mut position = error.1;
    //           position.start.row += line as usize;
    //           position.end.row += line as usize;
    //           let mut identifier = JsIdentifier::from(&position);
    //           identifier.error = Some(diagnostic);
    //           all_errors.push(identifier);
    //         }
    //       }
    //     }
    //     DiagnosticMessage::Str(_) => {}
    //   }
    // }
    all_identifiers
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
    let full_name = hover.0.name.to_owned();
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
      let value = data_type.completion_detail().to_owned();
      let value = format!("{value}\n\n---\n**{}**", &full_name);
      let actions = vec![];
      let actions = self.actions.get(&filename).unwrap_or(&actions);
      let action = Action {
        name: full_name.to_owned(),
        description: value.to_owned(),
      };
      let value = actions
        .iter()
        .find(|item| item.name.starts_with(&full_name))
        .unwrap_or(&action);
      let markup_content = MarkupContent {
        kind: MarkupKind::Markdown,
        value: value.description.to_owned(),
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
            label: None,
            documentaion: None,
          });
        }
      }
    }
    None
  }

  #[napi]
  pub fn complete(
    &self,
    id: u32,
    filename: String,
    line: u32,
    mut position: JsPosition,
  ) -> Option<Vec<JsCompletionItem>> {
    position.line -= line;
    let uri = Url::parse(&format!("file:///home/{filename}.{id}.jinja")).unwrap();
    let position = Position::new(position.line, position.character);
    let params: CompletionParams = CompletionParams {
      text_document_position: TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(uri.clone()),
        Position::new(position.line, position.character),
      ),
      work_done_progress_params: WorkDoneProgressParams {
        work_done_token: None,
      },
      partial_result_params: PartialResultParams {
        ..Default::default()
      },
      context: Some(CompletionContext {
        trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
        trigger_character: None,
      }),
    };
    let completion = self.lsp_files.completion(params)?;
    let mut items = None;

    match completion {
      CompletionType::Filter => {
        let completions = self.filters.clone();
        let mut ret = Vec::with_capacity(completions.len());
        for item in completions.into_iter() {
          ret.push(JsCompletionItem {
            completion_type: JsCompletionType::Filter,
            label: item.name,
            kind: Kind2::FIELD,
            description: item.desc.to_string(),
            new_text: None,
            insert: None,
            replace: None,
          });
        }
        items = Some(ret);
      }
      CompletionType::Identifier => {
        if let Some(variables) = self.lsp_files.read_variables(&uri, position) {
          let mut ret = vec![];
          for item in variables {
            ret.push(JsCompletionItem {
              completion_type: JsCompletionType::Identifier,
              label: item.label,
              kind: Kind2::VARIABLE,
              description: item.detail.unwrap_or(String::new()),
              new_text: None,
              insert: None,
              replace: None,
            });
          }
          items = Some(ret);
        }
      }
      CompletionType::IncludedTemplate { .. } => {}
      CompletionType::Snippets { .. } => {
        // let mut filtered = vec![];
        // for snippet in self.snippets.iter() {
        //   let mut snippet = snippet.clone();
        //   if let Some(CompletionTextEdit::Edit(TextEdit { new_text, .. })) = snippet.text_edit {
        //   if !self.lsp_files.is_vscode {
        //     snippet.text_edit = Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
        //       new_text,
        //       insert: range,
        //       replace: range,
        //     }));
        //   } else {
        //     snippet.text_edit = None;
        //   }
        // }
        // filtered.push(snippet);
        //   }
        // }

        // if !filtered.is_empty() {
        //   items = Some(CompletionResponse::Array(filtered));
        // }
      }
      CompletionType::IncompleteIdentifier { name, mut range } => {
        range.start.line += line;
        range.end.line += line;
        let variable = self.lsp_files.get_variable(name, uri.to_string())?;
        let ret = vec![JsCompletionItem {
          completion_type: JsCompletionType::Identifier,
          label: variable.to_string(),
          kind: Kind2::VARIABLE,
          description: variable.to_string(),
          new_text: Some(variable),
          insert: Some(JsRange::from(&range)),
          replace: Some(JsRange::from(&range)),
        }];
        items = Some(ret);
      }
    };
    items
  }

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
          print!("{}", location.uri);
          let uri2 = location.uri.to_string();
          if uri2.contains(&filename) {
            location.uri = Url::parse(&uri2).unwrap();
            location.range.start.line += line;
            location.range.end.line += line;
            let mut js_location = JsLocation::from(&location);
            js_location.is_backend = true;
            definitions.push(js_location);
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
  Link,
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
  pub label: Option<String>,
  pub documentaion: Option<String>,
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
  pub is_backend: bool,
  pub name: String,
}

impl From<&Location> for JsLocation {
  fn from(value: &Location) -> Self {
    Self {
      uri: value.uri.to_string(),
      range: JsRange::from(&value.range),
      is_backend: false,
      name: String::new(),
    }
  }
}

#[napi(object)]
pub struct JsCompletionItem {
  pub completion_type: JsCompletionType,
  pub label: String,
  pub kind: Kind2,
  pub description: String,
  pub new_text: Option<String>,
  pub insert: Option<JsRange>,
  pub replace: Option<JsRange>,
}

#[napi]
pub enum Kind2 {
  VARIABLE,
  FIELD,
  FUNCTION,
  MODULE,
  CONSTANT,
  FILE,
  TEXT,
}

#[napi]
pub enum JsCompletionType {
  Filter,
  Identifier,
  Snippets,
}

impl From<CompletionItemKind> for Kind2 {
  fn from(value: CompletionItemKind) -> Self {
    if value == CompletionItemKind::VARIABLE {
      Kind2::VARIABLE
    } else if value == CompletionItemKind::FIELD {
      return Kind2::FIELD;
    } else if value == CompletionItemKind::FUNCTION {
      return Kind2::FUNCTION;
    } else if value == CompletionItemKind::MODULE {
      return Kind2::MODULE;
    } else if value == CompletionItemKind::CONSTANT {
      return Kind2::CONSTANT;
    } else if value == CompletionItemKind::FILE {
      return Kind2::FILE;
    } else {
      return Kind2::VARIABLE;
    }
  }
}

#[napi(object)]
pub struct Action {
  pub name: String,
  pub description: String,
}

pub struct ActionObject {
  pub fields: Vec<String>,
}

impl From<&Action> for ActionObject {
  fn from(value: &Action) -> Self {
    let parts = value.name.split('.');
    let mut fields = vec![];
    for part in parts {
      fields.push(part.to_owned());
    }
    Self { fields }
  }
}

impl From<&JinjaObject> for ActionObject {
  fn from(value: &JinjaObject) -> Self {
    let mut fields = vec![];
    fields.push(value.name.to_owned());
    for field in &value.fields {
      fields.push(field.0.to_owned());
    }
    Self { fields }
  }
}

impl From<&PythonIdentifier> for ActionObject {
  fn from(value: &PythonIdentifier) -> Self {
    let mut fields = vec![];
    let parts = value.field.split('.');
    for field in parts {
      fields.push(field.to_string());
    }
    // fields.push(value.name.to_owned());
    // for field in &value.fields {}
    Self { fields }
  }
}

impl ActionObject {
  pub fn compare(&self, fields: &Vec<String>) -> bool {
    &self.fields == fields
    // self.fields == fields
  }
}
