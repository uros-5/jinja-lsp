use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateTestCompletion {
    pub name: String,
    pub desc: String,
}

impl From<(&str, &str)> for TemplateTestCompletion {
    fn from((name, desc): (&str, &str)) -> TemplateTestCompletion {
        Self {
            name: name.to_string(),
            desc: desc.to_string(),
        }
    }
}

pub fn init_template_test_completions() -> Vec<TemplateTestCompletion> {
    vec![
        TemplateTestCompletion::from(("boolean", include_str!("md/is_boolean.md"))),
        TemplateTestCompletion::from(("defined", include_str!("md/is_defined.md"))),
        TemplateTestCompletion::from(("divisibleby", include_str!("md/is_divisibleby.md"))),
        TemplateTestCompletion::from(("endingwith", include_str!("md/is_endingwith.md"))),
        TemplateTestCompletion::from(("eq", include_str!("md/is_eq.md"))),
        TemplateTestCompletion::from(("even", include_str!("md/is_even.md"))),
        TemplateTestCompletion::from(("false", include_str!("md/is_false.md"))),
        TemplateTestCompletion::from(("filter", include_str!("md/is_filter.md"))),
        TemplateTestCompletion::from(("float", include_str!("md/is_float.md"))),
        TemplateTestCompletion::from(("ge", include_str!("md/is_ge.md"))),
        TemplateTestCompletion::from(("gt", include_str!("md/is_gt.md"))),
        TemplateTestCompletion::from(("in", include_str!("md/is_in.md"))),
        TemplateTestCompletion::from(("integer", include_str!("md/is_integer.md"))),
        TemplateTestCompletion::from(("iterable", include_str!("md/is_iterable.md"))),
        TemplateTestCompletion::from(("le", include_str!("md/is_le.md"))),
        TemplateTestCompletion::from(("lower", include_str!("md/is_lower.md"))),
        TemplateTestCompletion::from(("lt", include_str!("md/is_lt.md"))),
        TemplateTestCompletion::from(("mapping", include_str!("md/is_mapping.md"))),
        TemplateTestCompletion::from(("ne", include_str!("md/is_ne.md"))),
        TemplateTestCompletion::from(("none", include_str!("md/is_none.md"))),
        TemplateTestCompletion::from(("number", include_str!("md/is_number.md"))),
        TemplateTestCompletion::from(("odd", include_str!("md/is_odd.md"))),
        TemplateTestCompletion::from(("safe", include_str!("md/is_safe.md"))),
        TemplateTestCompletion::from(("sameas", include_str!("md/is_sameas.md"))),
        TemplateTestCompletion::from(("sequence", include_str!("md/is_sequence.md"))),
        TemplateTestCompletion::from(("startingwith", include_str!("md/is_startingwith.md"))),
        TemplateTestCompletion::from(("string", include_str!("md/is_string.md"))),
        TemplateTestCompletion::from(("test", include_str!("md/is_test.md"))),
        TemplateTestCompletion::from(("true", include_str!("md/is_true.md"))),
        TemplateTestCompletion::from(("undefined", include_str!("md/is_undefined.md"))),
        TemplateTestCompletion::from(("upper", include_str!("md/is_upper.md"))),
    ]
}
