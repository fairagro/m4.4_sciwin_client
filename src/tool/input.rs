#[derive(Debug, PartialEq)]
pub enum OptionType {
    Positional,
    Option,
    Flag,
}

#[derive(Debug, PartialEq)]
pub struct Input {
    pub id: Option<String>,
    pub value: Option<String>,
    pub r#type: OptionType,
    pub prefix: Option<String>,
    pub index: Option<usize>,
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Self {
        Input {
            id: None,
            value: None,
            r#type: OptionType::Option,
            prefix: None,
            index: None,
        }
    }
}

impl Input {
    pub fn new_with(
        id: &str,
        value: Option<&str>,
        input_type: OptionType,
        prefix: Option<&str>,
        index: Option<usize>,
    ) -> Self {
        Input {
            id: Some(id.to_string()),
            value: value.map(|v| v.to_string()),
            r#type: input_type,
            prefix: prefix.map(|p| p.to_string()),
            index,
        }
    }
}