#![deny(clippy::all)]

use jinja_lsp::lsp_files::LspFiles;
use jinja_lsp_queries::{
  parsers::Parsers,
  search::{objects::objects_query, queries::Queries},
};

use tower_lsp::lsp_types::{DidOpenTextDocumentParams, Position, TextDocumentItem, Url};
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
}

#[napi]
impl NodejsLspFiles {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      lsp_files: LspFiles::default(),
      counter: 0,
    }
  }

  /// Actions can come from unsaved context.
  #[napi]
  pub fn add_global_context(&self, actions: Option<Vec<String>>) {}

  #[napi]
  pub fn delete_all(&mut self) {
    self.lsp_files.variables.clear();
    self.lsp_files.delete_documents();
    self.counter = 0;
    // self.lsp_files.main_channel
  }

  #[napi]
  pub fn add_one(&mut self, id: u32, content: String, line: u32) -> bool {
    println!("{}", id);
    let params: DidOpenTextDocumentParams = DidOpenTextDocumentParams {
      text_document: TextDocumentItem::new(
        Url::parse(&format!("file:///home/{id}.jinja")).unwrap(),
        String::new(),
        0,
        content,
      ),
    };
    let content = self.lsp_files.did_open(params);
    if let Some(content) = content {
      match content {
        jinja_lsp::channels::diagnostics::DiagnosticMessage::Errors(errors) => {
          for i in errors {
            println!("error: {}", i.1.len());
          }
        }
        jinja_lsp::channels::diagnostics::DiagnosticMessage::Str(_) => {
          println!("str")
        }
      }
    }
    true
  }

  #[napi]
  pub fn hover(&self, position: JsPosition, id: u32) {}

  #[napi]
  pub fn complete(position: JsPosition, id: u32, content: String) {}

  #[napi]
  pub fn goto_definition(&self, position: JsPosition, id: u32) {}
}

#[napi(object)]
pub struct JsPosition {
  pub line: u32,
  pub character: u32,
}
