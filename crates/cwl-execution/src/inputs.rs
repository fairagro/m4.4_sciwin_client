use commonwl::{CWLType, DefaultValue, inputs::CommandInputParameter};
use serde_yaml::Value;
use std::{collections::HashMap, error::Error};

///Either gets the default value for input or the provided one (preferred)
pub(crate) fn evaluate_input_as_string(
    input: &CommandInputParameter,
    input_values: &HashMap<String, DefaultValue>,
) -> Result<String, Box<dyn Error>> {
    Ok(evaluate_input(input, input_values)?.as_value_string())
}

///Either gets the default value for input or the provided one (preferred)
pub(crate) fn evaluate_input(input: &CommandInputParameter, input_values: &HashMap<String, DefaultValue>) -> Result<DefaultValue, Box<dyn Error>> {
    if let Some(value) = input_values.get(&input.id) {
        if (matches!(input.type_, CWLType::Any) || input.type_.is_optional())
            && matches!(value, DefaultValue::Any(Value::Null))
            && let Some(default_) = &input.default
        {
            return Ok(default_.clone());
        }

        if value.has_matching_type(&input.type_) {
            return Ok(value.clone());
        } else {
            Err(format!(
                "CWLType '{:?}' is not matching input type. Input was: \n{:#?}",
                &input.type_, value
            ))?;
        }
    } else if let Some(default_) = &input.default {
        return Ok(default_.clone());
    }

    if let CWLType::Optional(_) = input.type_ {
        return Ok(DefaultValue::Any(Value::Null));
    } else {
        Err(format!("You did not include a value for {}", input.id).as_str())?;
    }

    Err(format!("Could not evaluate input: {}. Expected type: {:?}", input.id, input.type_))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use commonwl::inputs::CommandLineBinding;
    use serde_yaml::{Value, value};

    #[test]
    pub fn test_evaluate_input() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_prefix("--arg"));
        let mut values = HashMap::new();
        values.insert("test".to_string(), DefaultValue::Any(value::Value::String("Hello!".to_string())));

        let evaluation = evaluate_input(&input, &values.clone()).unwrap();

        assert_eq!(evaluation, values["test"]);
    }

    #[test]
    pub fn test_evaluate_input_as_string() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_prefix("--arg"));
        let mut values = HashMap::new();
        values.insert("test".to_string(), DefaultValue::Any(value::Value::String("Hello!".to_string())));

        let evaluation = evaluate_input_as_string(&input, &values.clone()).unwrap();

        assert_eq!(evaluation, values["test"].as_value_string());
    }

    #[test]
    pub fn test_evaluate_input_empty_values() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_prefix("--arg"))
            .with_default_value(DefaultValue::Any(Value::String("Nice".to_string())));
        let values = HashMap::new();
        let evaluation = evaluate_input_as_string(&input, &values.clone()).unwrap();

        assert_eq!(evaluation, "Nice".to_string());
    }

    #[test]
    pub fn test_evaluate_input_no_values() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::String)
            .with_binding(CommandLineBinding::default().with_prefix("--arg"))
            .with_default_value(DefaultValue::Any(Value::String("Nice".to_string())));
        let evaluation = evaluate_input_as_string(&input, &HashMap::new()).unwrap();

        assert_eq!(evaluation, "Nice".to_string());
    }

    #[test]
    pub fn test_evaluate_input_any() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::Any)
            .with_binding(CommandLineBinding::default().with_prefix("--arg"))
            .with_default_value(DefaultValue::Any(Value::String("Nice".to_string())));
        let evaluation = evaluate_input_as_string(&input, &HashMap::new()).unwrap();

        assert_eq!(evaluation, "Nice".to_string());
    }

    #[test]
    pub fn test_evaluate_input_any_null() {
        let input = CommandInputParameter::default()
            .with_id("test")
            .with_type(CWLType::Any)
            .with_binding(CommandLineBinding::default().with_prefix("--arg"))
            .with_default_value(DefaultValue::Any(Value::String("Nice".to_string())));
        let evaluation = evaluate_input_as_string(&input, &HashMap::from([("test".to_string(), DefaultValue::Any(Value::Null))])).unwrap();
        //if any and null, take default
        assert_eq!(evaluation, "Nice".to_string());
    }
}
