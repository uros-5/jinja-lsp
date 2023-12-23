use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HxCompletion {
    pub name: String,
    pub desc: String,
}

impl From<&(&str, &str)> for HxCompletion {
    fn from((name, desc): &(&str, &str)) -> Self {
        Self {
            name: name.to_string(),
            desc: desc.to_string(),
        }
    }
}

fn to_filter_completions(values: Vec<(&str, &str)>) -> Vec<HxCompletion> {
    values.iter().filter_map(|x| x.try_into().ok()).collect()
}

pub fn init_filter_completions() -> HashMap<String, String> {
    let mut hm = HashMap::new();
    hm.insert(
        "selectattr".to_string(),
        include_str!("./md/filters/selectattr.md").to_string(),
    );
    hm.insert(
        "map".to_string(),
        include_str!("./md/filters/map.md").to_string(),
    );
    hm.insert(
        "reject".to_string(),
        include_str!("./md/filters/reject.md").to_string(),
    );
    hm.insert(
        "join".to_string(),
        include_str!("./md/filters/join.md").to_string(),
    );
    hm.insert(
        "slice".to_string(),
        include_str!("./md/filters/slice.md").to_string(),
    );
    hm.insert(
        "upper".to_string(),
        include_str!("./md/filters/upper.md").to_string(),
    );
    hm.insert(
        "escape".to_string(),
        include_str!("./md/filters/escape.md").to_string(),
    );
    hm.insert(
        "items".to_string(),
        include_str!("./md/filters/items.md").to_string(),
    );
    hm.insert(
        "max".to_string(),
        include_str!("./md/filters/max.md").to_string(),
    );
    hm.insert(
        "tojson".to_string(),
        include_str!("./md/filters/tojson.md").to_string(),
    );
    hm.insert(
        "rejectattr".to_string(),
        include_str!("./md/filters/rejectattr.md").to_string(),
    );
    hm.insert(
        "safe".to_string(),
        include_str!("./md/filters/safe.md").to_string(),
    );
    hm.insert(
        "int".to_string(),
        include_str!("./md/filters/int.md").to_string(),
    );
    hm.insert(
        "batch".to_string(),
        include_str!("./md/filters/batch.md").to_string(),
    );
    hm.insert(
        "first".to_string(),
        include_str!("./md/filters/first.md").to_string(),
    );
    hm.insert(
        "abs".to_string(),
        include_str!("./md/filters/abs.md").to_string(),
    );
    hm.insert(
        "indent".to_string(),
        include_str!("./md/filters/indent.md").to_string(),
    );
    hm.insert(
        "urlencode".to_string(),
        include_str!("./md/filters/urlencode.md").to_string(),
    );
    hm.insert(
        "trim".to_string(),
        include_str!("./md/filters/trim.md").to_string(),
    );
    hm.insert(
        "float".to_string(),
        include_str!("./md/filters/float.md").to_string(),
    );
    hm.insert(
        "sort".to_string(),
        include_str!("./md/filters/sort.md").to_string(),
    );
    hm.insert(
        "reverse".to_string(),
        include_str!("./md/filters/reverse.md").to_string(),
    );
    hm.insert(
        "attr".to_string(),
        include_str!("./md/filters/attr.md").to_string(),
    );
    hm.insert(
        "title".to_string(),
        include_str!("./md/filters/title.md").to_string(),
    );
    hm.insert(
        "unique".to_string(),
        include_str!("./md/filters/unique.md").to_string(),
    );
    hm.insert(
        "select".to_string(),
        include_str!("./md/filters/select.md").to_string(),
    );
    hm.insert(
        "round".to_string(),
        include_str!("./md/filters/round.md").to_string(),
    );
    hm.insert(
        "lower".to_string(),
        include_str!("./md/filters/lower.md").to_string(),
    );
    hm.insert(
        "last".to_string(),
        include_str!("./md/filters/last.md").to_string(),
    );
    hm.insert(
        "bool".to_string(),
        include_str!("./md/filters/bool.md").to_string(),
    );
    hm.insert(
        "capitalize".to_string(),
        include_str!("./md/filters/capitalize.md").to_string(),
    );
    hm.insert(
        "list".to_string(),
        include_str!("./md/filters/list.md").to_string(),
    );
    hm.insert(
        "dictsort".to_string(),
        include_str!("./md/filters/dictsort.md").to_string(),
    );
    hm.insert(
        "length".to_string(),
        include_str!("./md/filters/length.md").to_string(),
    );
    hm.insert(
        "replace".to_string(),
        include_str!("./md/filters/replace.md").to_string(),
    );
    hm.insert(
        "pprint".to_string(),
        include_str!("./md/filters/pprint.md").to_string(),
    );
    hm.insert(
        "min".to_string(),
        include_str!("./md/filters/min.md").to_string(),
    );
    hm.insert(
        "default".to_string(),
        include_str!("./md/filters/default.md").to_string(),
    );
    hm
}
