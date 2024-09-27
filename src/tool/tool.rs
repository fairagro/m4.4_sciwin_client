use super::input::Input;

#[derive(Debug, PartialEq)]
pub struct Tool {
    pub base_command: Vec<String>,
    pub inputs: Vec<Input>,
}