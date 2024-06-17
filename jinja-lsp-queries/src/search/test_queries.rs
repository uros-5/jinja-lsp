#[cfg(test)]
mod query_tests {
    use crate::search::{
        objects::objects_query, python_identifiers::python_identifiers,
        snippets_completion::snippets_query, to_range,
    };
    use tree_sitter::{Parser, Point};

    use crate::{
        search::objects::CompletionType,
        search::{
            completion_start, definition::definition_query, queries::Queries,
            rust_identifiers::backend_definition_query,
            rust_template_completion::backend_templates_query, templates::templates_query,
        },
    };

    fn prepare_jinja_tree(text: &str) -> tree_sitter::Tree {
        let language = tree_sitter_jinja2::language();
        let mut parser = Parser::new();

        parser
            .set_language(&language)
            .expect("could not load jinja grammar");

        parser.parse(text, None).expect("not to fail")
    }

    fn prepare_rust_tree(text: &str) -> tree_sitter::Tree {
        let language = tree_sitter_rust::language();
        let mut parser = Parser::new();

        parser
            .set_language(&language)
            .expect("could not load rust grammar");

        parser.parse(text, None).expect("not to fail")
    }

    fn prepare_python_tree(text: &str) -> tree_sitter::Tree {
        let language = tree_sitter_python::language();
        let mut parser = Parser::new();

        parser
            .set_language(&language)
            .expect("could not load rust grammar");

        parser.parse(text, None).expect("not to fail")
    }

    #[test]
    fn jinja_definitions() {
        let cases = [
            (
                r#"
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
        "#,
                7,
            ),
            (
                r#"
            {% with name = 55 %}
                <p>Hello {{ name }}</p>
                {% set a = "hello world" %}
                {% set b %}
                    some content
                {% endset %}
            {% endwith %}
            {% for i in 10 -%}
            {%- endfor %}

            {{ point }}
            {{ point }}
            "#,
                4,
            ),
        ];
        let query = Queries::default();
        let query = query.jinja_definitions;

        for case in cases {
            let tree = prepare_jinja_tree(case.0);
            let trigger_point = Point::new(0, 0);
            // let closest_node = tree.root_node();
            let definitions = definition_query(&query, &tree, trigger_point, case.0, true);
            assert_eq!(definitions.identifiers().len(), case.1);
        }
    }

    #[test]
    fn jinja_identifiers() {
        let query = Queries::default();
        let query = query.jinja_objects;
        let cases = [
            (
                r#"
            {{ user.id }}
            {% for i in 10 -%}
                {{ i }}
            {%- endfor %}
            {% set class = "button" -%}
        "#,
                4,
            ),
            (
                r#"
        {{ obj.abc obj2.abc2 }}

        {{ obj3.field.something.something == obj4.something }}

        {% if obj5.field -%}
        111 {{ abc == def.abc }}
        {% endif %}
        "#,
                7,
            ),
            (
                r#"
        <p> {{ something }}</p>
        <p hx-swap="innerHTML"> {{ something | some_filter(a, b,c) }} </p>            
        {% for i in something -%}
            {{ i }}
        {%- endfor %}
        {% if something %}
            {{ something }}
        {% endif %}
        "#,
                11,
            ),
        ];
        for case in cases {
            let tree = prepare_jinja_tree(case.0);
            let trigger_point = Point::new(0, 0);
            let objects = objects_query(&query, &tree, trigger_point, case.0, true);
            let len = objects.show().len();
            assert_eq!(len, case.1);
        }
    }

    #[test]
    fn rust_definition() {
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
        let query = Queries::default();
        let query = &query.backend_definitions;
        let rust = backend_definition_query(query, &tree, trigger_point, case, true);
        assert_eq!(rust.show().len(), 8);
    }

    #[test]
    fn python_definition() {
        let case = r#"
            jinja_env.globals['a'] = 1
            render_template(data=123)
            some_obj.render_template(first_name = "John", last_name = "Doe")
            render(a=11)        
        "#;

        let tree = prepare_python_tree(case);
        let trigger_point = Point::new(0, 0);
        let mut query = Queries::default();
        query.update_backend("python");
        let query = &query.backend_definitions;
        let rust = backend_definition_query(query, &tree, trigger_point, case, true);
        assert_eq!(rust.show().len(), 5);
    }

    #[test]
    fn find_jinja_completion() {
        let source = r#"
            {{ something |     filter1 | filter2 }}
            {{ some_identifier.otherfields }}
            {% if something == 11 -%}
            {% macro example(a, b, c) -%}
            <p> hello world</p> 
            {%- endmacro %}

            {{ }}
            {{ "|" }}
            {{ identifier }}
            {{}}
            {{ identifier }}
        "#;
        let cases = [
            (
                Point::new(2, 24),
                Some((
                    CompletionType::IncompleteIdentifier {
                        name: "some_iden".to_string(),
                        range: to_range((Point::new(2, 15), Point::new(2, 42))),
                    },
                    false,
                )),
            ),
            (Point::new(1, 27), Some((CompletionType::Filter, false))),
            (
                Point::new(1, 47),
                Some((
                    CompletionType::IncompleteIdentifier {
                        name: "filter".to_string(),
                        range: to_range((Point::new(1, 41), Point::new(1, 48))),
                    },
                    false,
                )),
            ),
            (
                Point::new(1, 46),
                Some((
                    CompletionType::IncompleteIdentifier {
                        name: "filte".to_string(),
                        range: to_range((Point::new(1, 41), Point::new(1, 48))),
                    },
                    false,
                )),
            ),
            (Point::new(1, 40), Some((CompletionType::Filter, false))),
            (Point::new(1, 50), Some((CompletionType::Identifier, false))),
            (
                Point::new(3, 18),
                None, // Some((CompletionType::IncompleteIdentifier {
                      //     name: "something".to_owned(),
                      //     range: to_range((Point::new(3, 18), Point::new(3, 27))),
                      // }, false)),
            ),
            (Point::new(4, 20), None),
            (
                Point::new(3, 22),
                None, // Some((CompletionType::IncompleteIdentifier {
                      //     name: "something".to_owned(),
                      //     range: to_range((Point::new(3, 18), Point::new(3, 27))),
                      // }, false)),
            ),
            (Point::new(8, 15), Some((CompletionType::Identifier, false))),
            (Point::new(9, 18), Some((CompletionType::Identifier, false))),
            (
                Point::new(10, 18),
                Some((
                    CompletionType::IncompleteIdentifier {
                        name: "ide".to_string(),
                        range: to_range((Point::new(10, 15), Point::new(10, 25))),
                    },
                    false,
                )),
            ),
            (
                Point::new(10, 25),
                Some((
                    CompletionType::IncompleteIdentifier {
                        name: "identifier".to_string(),
                        range: to_range((Point::new(10, 15), Point::new(10, 25))),
                    },
                    false,
                )),
            ),
            (
                Point::new(11, 14),
                Some((CompletionType::Identifier, false)),
            ),
            (
                Point::new(12, 25),
                Some((
                    CompletionType::IncompleteIdentifier {
                        name: "identifier".to_string(),
                        range: to_range((Point::new(12, 15), Point::new(12, 25))),
                    },
                    false,
                )),
            ),
        ];
        for case in cases {
            let tree = prepare_jinja_tree(source);
            let trigger_point = case.0;
            let query = Queries::default();
            let query = &query.jinja_objects;
            let objects = objects_query(query, &tree, trigger_point, source, false);
            assert_eq!(objects.completion(trigger_point), case.1);
        }
        let source = r#"
            {{
        "#;
        let cases = [(Point::new(1, 14), Some((CompletionType::Identifier, true)))];
        for case in cases {
            let tree = prepare_jinja_tree(source);
            let trigger_point = case.0;
            let query = Queries::default();
            let query = &query.jinja_objects;
            let objects = objects_query(query, &tree, trigger_point, source, false);
            assert_eq!(objects.completion(trigger_point), case.1);
        }
    }

    #[test]
    fn check_jinja_templates() {
        let source = r#"
        <div class="bg-white overflow-hidden shadow rounded-lg border">
            {% include 'header.jinja' %}
            {% include 'customization.jinja' ignore missing %}
            {% include ['page_detailed.jinja', 'page.jinja'] %}
            {% import "header.jinja"  %}
            {% from "page_detailed.jinja" import a %}
        </div> 	    
        "#;

        let cases = [
            (Point::new(2, 25), "header.jinja"),
            (Point::new(3, 30), "customization.jinja"),
            (Point::new(4, 36), "page_detailed.jinja"),
            (Point::new(4, 50), "page.jinja"),
            (Point::new(5, 34), "header.jinja"),
            (Point::new(6, 39), "page_detailed.jinja"),
        ];
        for case in cases {
            let tree = prepare_jinja_tree(source);
            let trigger_point = case.0;
            let query = Queries::default();
            let query = &query.jinja_imports;
            let templates = templates_query(query, &tree, trigger_point, source, false);
            let template = templates.in_template(trigger_point);
            assert!(template.is_some());
            assert_eq!(
                template
                    .unwrap()
                    .get_identifier(trigger_point)
                    .unwrap()
                    .name,
                case.1
            );
        }
    }

    #[test]
    fn jinja_templates_in_rust() {
        let source = r#"
            get_template("",11);
            render_jinja("some_template.jinja");
            render_jinja(1,2, 3, "some_template.jinja");
            render_jinja(1,2, 3);
            add_global("PROJECT_NAME", "Example");
            
        "#;
        let tree = prepare_rust_tree(source);
        let trigger_point = Point::default();
        let query = Queries::default();
        let query = &query.backend_templates;
        let templates = backend_templates_query(query, &tree, trigger_point, source, true);
        assert_eq!(templates.templates.len(), 3);
    }

    #[test]
    fn template_completion_in_rust() {
        let source = r#"
            let tmp2 = jinja.get_template("account3");
            let tmp2 = jinja.get_template("account2");
            let tmp = jinja.get_template("account");
            let tmp = jinja.anything("account");
        "#;
        let tree = prepare_rust_tree(source);
        let trigger_point = Point::new(3, 47);
        let query = Queries::default();
        let query = &query.backend_templates;
        let templates = backend_templates_query(query, &tree, trigger_point, source, false);
        if let Some(template) = templates.in_template(trigger_point) {
            if let Some(completion) = completion_start(trigger_point, template) {
                assert_eq!(completion, "acco");
            }
        }
    }

    #[test]
    fn template_completion_in_python() {
        let source = r#"
                tmp2 = jinja.get_template("account3");
                tmp2 = jinja.get_template("account2");
                tmp = jinja.get_template("account");
                tmp = jinja.anything("account");
            "#;
        let tree = prepare_python_tree(source);
        let trigger_point = Point::new(3, 47);
        let mut query = Queries::default();
        query.update_backend("python");
        let query = &query.backend_templates;
        let templates = backend_templates_query(query, &tree, trigger_point, source, false);
        if let Some(template) = templates.in_template(trigger_point) {
            if let Some(completion) = completion_start(trigger_point, template) {
                assert_eq!(completion, "acco");
            }
        }
    }

    #[test]
    fn jinja_definition_scope() {
        let source = r#"
    {% macro hello_world(parameter) -%}
        <p class="text-sm font-medium text-green-500"> hello world <p>
        {{ PROJECT_NAME | length }}
        {{ parameter }}
            {% macro primer(parameter2) %}
                {{ parameter }}
            {% endmacro %}
        {% set b = 11 %}
    {% endmacro %}
        "#;
        let query = Queries::default();
        let query = query.jinja_definitions;
        let tree = prepare_jinja_tree(source);
        let trigger_point = Point::new(0, 0);
        let definitions = definition_query(&query, &tree, trigger_point, source, true);
        let keys = definitions.identifiers();
        if let Some(last) = keys.last() {
            assert_eq!(last.scope_ends.1, Point::new(9, 4));
        }
    }

    #[test]
    fn snippets() {
        let cases = [
            (
                "{% if} {% if a == 123 %} {{ a }} {% endif %}",
                Point::new(0, 5),
                true,
            ),
            ("{% with} {{ var }} {% with %}", Point::new(0, 26), true),
            ("{% with  ", Point::new(0, 9), false),
        ];
        let query = Queries::default();
        let query = query.jinja_snippets;
        for case in cases {
            let tree = prepare_jinja_tree(case.0);
            let snippets = snippets_query(&query, &tree, case.1, case.0, false);
            assert_eq!(snippets.is_error, case.2);
        }
    }

    #[test]
    fn test_python_identifiers() {
        let cases = [r#"
            [page.text
                 for page in retrieval.result.other.field]
             "#];

        let query = Queries::default();
        let query = query.python_identifiers;
        for _case in cases {
            let tree = prepare_python_tree(cases[0]);
            let ids = python_identifiers(&query, &tree, Point::new(0, 0), cases[0], 0);
            assert_eq!(ids.len(), 2);
        }
    }
}
