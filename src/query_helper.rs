use std::collections::HashMap;

use tree_sitter::{Node, Point, Query, QueryCursor};

use crate::{
    capturer::{Capturer, JinjaCompletionCapturer},
    queries::{JINJA_COMPLETION, JINJA_DEF, JINJA_REF, RUST_DEF},
};

pub struct Queries {
    pub jinja_ident_query: Query,
    pub jinja_ref_query: Query,
    pub jinja_completion_query: Query,
    pub rust_ident_query: Query,
}

impl Clone for Queries {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Default for Queries {
    fn default() -> Self {
        Self {
            jinja_ident_query: Query::new(tree_sitter_jinja2::language(), JINJA_DEF).unwrap(),
            jinja_ref_query: Query::new(tree_sitter_jinja2::language(), JINJA_REF).unwrap(),
            jinja_completion_query: Query::new(tree_sitter_jinja2::language(), JINJA_COMPLETION)
                .unwrap(),
            rust_ident_query: Query::new(tree_sitter_rust::language(), RUST_DEF).unwrap(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum QueryType {
    Completion,
    Definition,
}

#[derive(PartialEq, Eq, Debug)]
pub enum CompletionType {
    Pipe,
    Identifier,
}

#[derive(Debug, Clone)]
pub struct CaptureDetails {
    pub value: String,
    pub end_position: Point,
    pub start_position: Point,
}

pub fn query_completion(
    root: Node<'_>,
    source: &str,
    trigger_point: Point,
    query_type: QueryType,
    query: &Queries,
) -> Option<CompletionType> {
    let closest_node = root.descendant_for_point_range(trigger_point, trigger_point)?;
    let element = find_element_referent_to_current_node(closest_node)?;
    let pipe = query_pipe(root, source, trigger_point, query_type, query);
    if pipe.is_some() {
        return pipe;
    }
    let ident = query_ident(root, source, trigger_point, query_type, query);
    if ident.is_some() {
        return ident;
    }

    query_expr(root, source, trigger_point, query_type, query)
}

pub fn query_pipe(
    root: Node<'_>,
    source: &str,
    trigger_point: Point,
    query_type: QueryType,
    query: &Queries,
) -> Option<CompletionType> {
    //
    let mut capturer = JinjaCompletionCapturer::default();
    let query = &query.jinja_completion_query;
    let props = query_props(root, source, trigger_point, query, false, capturer);
    let pipe_waiting = props.get("pipe_waiting")?;
    let pipe = props.get("pipe")?;
    if trigger_point >= pipe.start_position && trigger_point <= pipe.end_position {
        Some(CompletionType::Pipe)
    } else {
        None
    }
}

pub fn query_ident(
    root: Node<'_>,
    source: &str,
    trigger_point: Point,
    query_type: QueryType,
    query: &Queries,
) -> Option<CompletionType> {
    //
    let mut capturer = JinjaCompletionCapturer::default();
    let query = &query.jinja_completion_query;
    let props = query_props(root, source, trigger_point, query, false, capturer);
    dbg!(&props);
    let ident_waiting = props.get("ident_waiting")?;
    let keyword = props.get("key_name")?;
    if trigger_point > keyword.end_position && trigger_point <= ident_waiting.end_position {
        let key_id = props.get("key_id");

        match key_id {
            Some(capture) => {
                dbg!(capture);
                if trigger_point > capture.start_position && trigger_point <= capture.end_position {
                    None
                } else {
                    Some(CompletionType::Identifier)
                }
            }
            None => Some(CompletionType::Identifier),
        }
    } else {
        None
    }
}

pub fn query_expr(
    root: Node<'_>,
    source: &str,
    trigger_point: Point,
    query_type: QueryType,
    query: &Queries,
) -> Option<CompletionType> {
    let capturer = JinjaCompletionCapturer::default();
    let query = &query.jinja_completion_query;
    let props = query_props(root, source, trigger_point, query, false, capturer);
    props.get("empty_expression")?;
    let start = props.get("start")?;
    let end = props.get("end")?;
    if trigger_point >= start.start_position && trigger_point <= end.end_position {
        Some(CompletionType::Identifier)
    } else {
        None
    }
}

pub fn query_props<T: Capturer>(
    node: Node<'_>,
    source: &str,
    trigger_point: Point,
    query: &Query,
    all: bool,
    mut capturer: T,
) -> HashMap<String, CaptureDetails> {
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let matches = cursor_qry.matches(query, node, source.as_bytes());

    matches
        .into_iter()
        .flat_map(|m| {
            m.captures
                .iter()
                .filter(|capture| all || capture.node.start_position() <= trigger_point)
        })
        .fold(HashMap::new(), |mut acc, capture| {
            capturer.save_by(capture, &mut acc, capture_names, source);
            acc
        })
}

fn find_element_referent_to_current_node(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "source_file" {
        return Some(node);
    }

    return find_element_referent_to_current_node(node.parent()?);
}

#[cfg(test)]
mod tests1 {
    use tree_sitter::{Parser, Point};

    use crate::{
        capturer::{JinjaCapturer, JinjaCapturer2, JinjaCompletionCapturer, RustCapturer},
        query_helper::{query_props, CompletionType, Queries},
    };

    use super::{query_completion, QueryType};

    fn prepare_jinja_tree(text: &str) -> tree_sitter::Tree {
        let language = tree_sitter_jinja2::language();
        let mut parser = Parser::new();

        parser
            .set_language(language)
            .expect("could not load jinja grammar");

        parser.parse(text, None).expect("not to fail")
    }

    fn prepare_rust_tree(text: &str) -> tree_sitter::Tree {
        let language = tree_sitter_rust::language();
        let mut parser = Parser::new();

        parser
            .set_language(language)
            .expect("could not load jinja grammar");

        parser.parse(text, None).expect("not to fail")
    }

    #[test]
    fn find_ident_definition() {
        let case = r#"
        {% macro do_something(a, b,c) %}
            <p>Hello world</p>
        {% with name = 55 %}
            <p>Hello {{ name }}</p>
        {% endwith %}

        {% endmacro %}

        {% set class = "button" -%}

        {% for i in 10 -%}
        {%- endfor %}

        {{ point }}
        {{ point }}
        "#;
        let tree = prepare_jinja_tree(case);
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let mut query = Queries::default();
        let query = &query.jinja_ident_query;
        let capturer = JinjaCapturer::default();
        let props = query_props(closest_node, case, trigger_point, query, true, capturer);
        assert_eq!(props.len(), 10);
    }

    #[test]
    fn find_identifiers() {
        let case = r#"
            {{ user.id }}
            {% for i in 10 -%}
                {{ i }}
            {%- endfor %}
            {% set class = "button" -%}
        "#;
        let tree = prepare_jinja_tree(case);
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let query = Queries::default();
        let query = &query.jinja_ref_query;
        let mut capturer = JinjaCapturer2::default();
        capturer.force();
        let props = query_props(closest_node, case, trigger_point, query, true, capturer);
        assert_eq!(props.len(), 4);
    }

    #[test]
    fn find_identifiers_with_statements_and_expressions() {
        let case = r#"
        {{ obj.abc obj2.abc2 }}

        {{ obj.field.something.something == obj2.something }}

        {% if obj.field -%}
        111 {{ abc == def.abc }}
        {% endif %}
        "#;
        let tree = prepare_jinja_tree(case);
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let query = Queries::default();
        let query = &query.jinja_ref_query;
        let mut capturer = JinjaCapturer2::default();
        capturer.force();
        let props = query_props(closest_node, case, trigger_point, query, true, capturer);
        assert_eq!(props.len(), 6);
    }

    #[test]
    fn find_identifiers_in_macro() {
        let case = r#"
            let a = context!(name => 11 + abc, abc => "username");
            let b = context!{name, username => "username" } 
            let price = 100;
            let c = context!{ price };
        "#;

        let tree = prepare_rust_tree(case);
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let query = Queries::default();
        let query = &query.rust_ident_query;
        let mut capturer = RustCapturer::default();
        capturer.force();
        let props = query_props(closest_node, case, trigger_point, query, true, capturer);
        assert_eq!(props.len(), 6);
    }

    #[test]
    fn find_jinja_completion() {
        let source = r#"
{{ something |     filter1 | filter2 }}

{% if something == 11 -%}
{% macro example(a, b, c) -%}
<p> hello world</p>
{%- endmacro %}

{{ }}
{{ "|" }}
        "#;
        let cases = [
            (Point::new(1, 17), Some(CompletionType::Pipe)),
            (Point::new(1, 26), None),
            (Point::new(1, 29), Some(CompletionType::Pipe)),
            (Point::new(1, 38), None),
            (Point::new(3, 6), Some(CompletionType::Identifier)),
            (Point::new(4, 9), None),
            (Point::new(3, 9), None),
            (Point::new(8, 4), Some(CompletionType::Identifier)),
            (Point::new(9, 7), None),
        ];
        for case in cases {
            let tree = prepare_jinja_tree(source);
            let trigger_point = case.0;
            let closest_node = tree.root_node();
            let query = Queries::default();
            let compl = query_completion(
                closest_node,
                source,
                trigger_point,
                QueryType::Completion,
                &query,
            );
            assert_eq!(compl, case.1);
        }

        // let query = &query.jinja_completion_query;
        // let mut capturer = JinjaCompletionCapturer::default();
        // let props = query_props(closest_node, case, trigger_point, query, false, capturer);
        // dbg!(&props);
        // assert_eq!(props.len(), 6);
    }
}
