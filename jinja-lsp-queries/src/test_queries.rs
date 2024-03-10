#[cfg(test)]
mod query_tests {

    use tree_sitter::{Parser, Point};

    use crate::{
        capturer::{
            included::{IncludeCapturer, IncludedTemplate},
            init::JinjaInitCapturer,
            object::{CompletionType, JinjaObjectCapturer},
            rust::RustCapturer,
        },
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
        let capturer = query_props(closest_node, case, trigger_point, query, true, capturer);
        assert_eq!(capturer.to_vec().len(), 7);
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
        let query = &query.jinja_idents;
        let capturer = JinjaObjectCapturer::default();
        let props = query_props(closest_node, case, trigger_point, query, true, capturer);
        assert_eq!(props.show().len(), 4);
    }

    #[test]
    fn find_identifiers_with_statements_and_expressions() {
        let case = r#"
        {{ obj.abc obj2.abc2 }}

        {{ obj3.field.something.something == obj4.something }}

        {% if obj5.field -%}
        111 {{ abc == def.abc }}
        {% endif %}
        "#;
        let tree = prepare_jinja_tree(case);
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let query = Queries::default();
        let query = &query.jinja_idents;
        let capturer = JinjaObjectCapturer::default();
        let props = query_props(closest_node, case, trigger_point, query, true, capturer);
        assert_eq!(props.show().len(), 7);
    }

    #[test]
    fn find_identifiers_quick() {
        let case = r#"
        <p> {{ something }}</p>
        <p hx-swap="innerHTML"> {{ something | some_filter(a, b,c) }} </p>            
        {% for i in something -%}
            {{ i }}
        {%- endfor %}
        {% if something %}
            {{ something }}
        {% endif %}
        "#;
        let tree = prepare_jinja_tree(case);
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let query = Queries::default();
        let query = &query.jinja_idents;
        let capturer = JinjaObjectCapturer::default();
        let props = query_props(closest_node, case, trigger_point, query, true, capturer);
        assert_eq!(props.show().len(), 11);
    }

    #[test]
    fn find_identifiers_in_macro() {
        let case = r#"
            let a = context!(name => 11 + abc, abc => "username");
            let b = context!{name, username => "username" } 
            let price = 100;
            let c = context!{ price };
            jinja.add_filter("running_locally", true);        
            jinja.add_function("some_fn", some_fn);
        "#;

        let tree = prepare_rust_tree(case);
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let query = Queries::default();
        let query = &query.rust_idents;
        let capturer = RustCapturer::default();
        let props = query_props(closest_node, case, trigger_point, query, true, capturer);
        let macros = props.macros();
        assert_eq!(macros.len(), 3);
        let mut count = 0;
        for context in macros {
            count += context.1.variables().len();
        }
        let variables = props.variables();
        count += variables.len();
        assert_eq!(count, 7);
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
            (Point::new(1, 27), Some(CompletionType::Filter)),
            (Point::new(1, 48), None),
            (Point::new(1, 40), Some(CompletionType::Filter)),
            (Point::new(1, 50), Some(CompletionType::Identifier)),
            (Point::new(3, 18), None),
            (Point::new(4, 20), None),
            (Point::new(3, 22), None),
            (Point::new(8, 15), Some(CompletionType::Identifier)),
            (Point::new(9, 18), Some(CompletionType::Identifier)),
        ];
        for case in cases {
            let tree = prepare_jinja_tree(source);
            let trigger_point = case.0;
            let closest_node = tree.root_node();
            let query = Queries::default();

            let query = &query.jinja_idents;
            let capturer = JinjaObjectCapturer::default();
            let props = query_props(closest_node, source, trigger_point, query, false, capturer);
            assert_eq!(props.completion(trigger_point), case.1);
        }
    }

    #[test]
    fn find_includes() {
        let source = r#"
            <div class="bg-white overflow-hidden shadow rounded-lg border">
                {% include 'header.jinja' %}
            {% include 'customization.jinja' ignore missing %}
                {% include ['page_detailed.jinja', 'page.jinja'] %}
            </div>   
        "#;
        let cases = [
            (Point::new(2, 31), "header.jinja"),
            (Point::new(3, 23), "customization.jinja"),
            (Point::new(4, 62), "page.jinja"),
        ];
        for case in cases {
            let tree = prepare_jinja_tree(source);
            let trigger_point = case;
            let closest_node = tree.root_node();
            let query = Queries::default();

            let query = &query.jinja_imports;
            let capturer = IncludeCapturer::default();
            let props = query_props(
                closest_node,
                source,
                trigger_point.0,
                query,
                false,
                capturer,
            );
            let template = props.in_template(case.0);
            assert!(template.is_some());
            assert_eq!(&template.unwrap().name, &case.1);
        }
    }
}
