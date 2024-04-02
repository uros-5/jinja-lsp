<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/banner.png" alt="" />

jinja-lsp enhances minijinja development experience by providing Helix/Nvim users with advanced features such as autocomplete, syntax highlighting, hover, goto definition, code actions and linting.

## Installation

```sh
cargo install jinja-lsp
```


## Features

### Autocomplete

Intelligent suggestions for variables in current template, as well as variables, templates and filters defined on backend side.

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/completion.png" alt="" />

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/completion2.png" alt="" />

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/completion3.png" alt="" />

### Linting

Highlights errors and potential bugs in your jinja templates.  

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/diagnostics1.png" alt="" />

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/diagnostics2.png" alt="" />

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/diagnostics3.png" alt="" />

### Hover Preview

See the complete filter or variable description by hovering over it.

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/hover.png" alt="" />

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/hover2.png" alt="" />

### Code Actions

It's recommended to reset variables on server in case you rename/delete file.

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/code_actions.png" alt="" />

### Goto Definition

Quickly jump to definition. Works for Rust identifiers as well. 

https://github.com/uros-5/jinja-lsp/assets/59397844/015e47b4-b6f6-47c0-8504-5ce79ebafb00

### Snippets

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/snippets.png" alt="" />

### Document symbols

<img src="https://raw.githubusercontent.com/uros-5/jinja-lsp/main/.github/document_symbols.png" alt="" />

## Configuration

Language server configuration

```json
{ "templates": "./TEMPLATES_DIR", "backend": ["./BACKEND_DIR"], "lang": "rust"}
````

Helix configuration

```toml
[language-server.jinja-lsp]
command = "jinja-lsp"
config = { templates = "./templates", backend = ["./src"], lang = "rust"}
timeout = 5

[[language]]
name = "jinja"
language-servers = ["jinja-lsp"]
```

[VSCode support](https://marketplace.visualstudio.com/search?term=jinja-lsp&target=VSCode&category=Other&sortBy=Relevance)
