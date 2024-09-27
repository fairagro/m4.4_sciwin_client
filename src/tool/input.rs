#[derive(Debug)]
pub enum OptionType {
    Positional,
    Option,
    Flag,
}

#[derive(Debug)]
pub struct Input {
    pub id: Option<String>,
    pub value: Option<String>,
    pub r#type: OptionType,
    pub prefix: Option<String>,
    pub index: Option<usize>,
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
