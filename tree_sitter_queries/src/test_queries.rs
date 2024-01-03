#[cfg(test)]
mod query_tests {

    use tree_sitter::{Parser, Point};

    use crate::{
        capturer::JinjaInitCapturer,
        queries::{query_props, Queries},
    };

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
            .expect("could not load rust grammar");

        parser.parse(text, None).expect("not to fail")
    }

    #[test]
    fn find_ident_definition() {
        let case = r#"
        {% macro do_something(a, b,c) %}
            <p>Hello world</p>
            {% set class = "button" -%}
        {% with name = 55 %}
            <p>Hello {{ name }}</p>
        {% endwith %}

        {% endmacro %}


        {% for i in 10 -%}
        {%- endfor %}

        {{ point }}
        {{ point }}
        "#;
        let tree = prepare_jinja_tree(case);
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let query = Queries::default();
        let query = &query.jinja_init;
        let capturer = JinjaInitCapturer::default();
        let props = query_props(closest_node, case, trigger_point, query, true, capturer);
        props.1.states.iter().for_each(|item| {
            dbg!(item);
        });
        assert!(false);
    }
}
