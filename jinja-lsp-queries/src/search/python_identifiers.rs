use tree_sitter::{Point, Query, QueryCursor, Tree};

pub fn _python_identifiers(
    query: &Query,
    tree: &Tree,
    mut _trigger_point: Point,
    text: &str,
    all: bool,
) {
    let closest_node = tree.root_node();
    let mut cursor_qry = QueryCursor::new();
    let _capture_names = query.capture_names();
    let matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    let captures = matches.into_iter().flat_map(|m| {
        m.captures
            .iter()
            .filter(|capture| all || capture.node.start_position() <= _trigger_point)
    });
    for _capture in captures {
        // if check.is_none() {
        //     break;
        // }
    }
}
