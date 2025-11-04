use std::fs;

use ignore::Walk;
use serde::{Deserialize, Serialize};

use crate::config::JinjaConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilterCompletion {
    pub name: String,
    pub desc: String,
}

impl From<(&str, &str)> for FilterCompletion {
    fn from((name, desc): (&str, &str)) -> Self {
        Self {
            name: name.to_string(),
            desc: desc.to_string(),
        }
    }
}

pub fn init_filter_completions() -> Vec<FilterCompletion> {
    vec![
        FilterCompletion::from(("abs", include_str!("md/filters/abs.md"))),
        FilterCompletion::from(("attr", include_str!("md/filters/attr.md"))),
        FilterCompletion::from(("batch", include_str!("md/filters/batch.md"))),
        FilterCompletion::from(("bool", include_str!("md/filters/bool.md"))),
        FilterCompletion::from(("capitalize", include_str!("md/filters/capitalize.md"))),
        FilterCompletion::from(("default", include_str!("md/filters/default.md"))),
        FilterCompletion::from(("dictsort", include_str!("md/filters/dictsort.md"))),
        FilterCompletion::from(("escape", include_str!("md/filters/escape.md"))),
        FilterCompletion::from(("first", include_str!("md/filters/first.md"))),
        FilterCompletion::from(("float", include_str!("md/filters/float.md"))),
        FilterCompletion::from(("indent", include_str!("md/filters/indent.md"))),
        FilterCompletion::from(("int", include_str!("md/filters/int.md"))),
        FilterCompletion::from(("items", include_str!("md/filters/items.md"))),
        FilterCompletion::from(("join", include_str!("md/filters/join.md"))),
        FilterCompletion::from(("last", include_str!("md/filters/last.md"))),
        FilterCompletion::from(("length", include_str!("md/filters/length.md"))),
        FilterCompletion::from(("list", include_str!("md/filters/list.md"))),
        FilterCompletion::from(("lower", include_str!("md/filters/lower.md"))),
        FilterCompletion::from(("map", include_str!("md/filters/map.md"))),
        FilterCompletion::from(("max", include_str!("md/filters/max.md"))),
        FilterCompletion::from(("min", include_str!("md/filters/min.md"))),
        FilterCompletion::from(("pprint", include_str!("md/filters/pprint.md"))),
        FilterCompletion::from(("rejectattr", include_str!("md/filters/rejectattr.md"))),
        FilterCompletion::from(("reject", include_str!("md/filters/reject.md"))),
        FilterCompletion::from(("replace", include_str!("md/filters/replace.md"))),
        FilterCompletion::from(("reverse", include_str!("md/filters/reverse.md"))),
        FilterCompletion::from(("round", include_str!("md/filters/round.md"))),
        FilterCompletion::from(("safe", include_str!("md/filters/safe.md"))),
        FilterCompletion::from(("selectattr", include_str!("md/filters/selectattr.md"))),
        FilterCompletion::from(("select", include_str!("md/filters/select.md"))),
        FilterCompletion::from(("slice", include_str!("md/filters/slice.md"))),
        FilterCompletion::from(("sort", include_str!("md/filters/sort.md"))),
        FilterCompletion::from(("title", include_str!("md/filters/title.md"))),
        FilterCompletion::from(("tojson", include_str!("md/filters/tojson.md"))),
        FilterCompletion::from(("trim", include_str!("md/filters/trim.md"))),
        FilterCompletion::from(("unique", include_str!("md/filters/unique.md"))),
        FilterCompletion::from(("upper", include_str!("md/filters/upper.md"))),
        FilterCompletion::from(("urlencode", include_str!("md/filters/urlencode.md"))),
    ]
}
pub fn add_custom_filter_completions(filters: &mut Vec<FilterCompletion>, config: &JinjaConfig) {
    for directory in &config.filters {
        let walk = Walk::new(directory);
        for entry in walk.into_iter() {
            let Ok(entry) = entry else { continue };
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let file_content = fs::read_to_string(entry.path());
            let Ok(file_content) = file_content else {
                continue;
            };
            let Some(file_name) = entry.path().file_stem() else {
                continue;
            };
            let Some(file_name) = file_name.to_str() else {
                continue;
            };
            let Some(index) = filters.iter().position(|item| item.name == file_name) else {
                filters.push(FilterCompletion {
                    name: file_name.to_string(),
                    desc: file_content,
                });
                continue;
            };
            let Some(filter_item) = filters.get_mut(index) else {
                filters.push(FilterCompletion {
                    name: file_name.to_string(),
                    desc: file_content,
                });
                continue;
            };
            filter_item.desc = file_content;
        }
    }
}
