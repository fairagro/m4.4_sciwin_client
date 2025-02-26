use crate::{util::split_ranges, RuntimeEnvironment};
use rustyscript::static_runtime;
use serde_json::Value;
use std::{collections::HashMap, ops::Range};

static_runtime!(RUNTIME);

pub(crate) fn prepare_expression_engine(environment: &RuntimeEnvironment) -> Result<(), rustyscript::Error> {
    let inputs = serde_json::to_string(&environment.inputs)?;
    let runtime = serde_json::to_string(&environment.runtime)?;

    RUNTIME::with(|rt| rt.eval::<()>(format!("var inputs = {inputs}; var runtime = {runtime}")))?;

    Ok(())
}

pub(crate) fn eval(expression: &str) -> Result<Value, rustyscript::Error> {
    RUNTIME::with(|rt| rt.eval::<Value>(expression))
}

pub(crate) fn reset_expression_engine() -> Result<(), rustyscript::Error> {
    RUNTIME::with(|rt| {
        rt.eval::<()>(
            r#"
            var inputs = undefined;
            var runtime = undefined;
            var self = undefined;"#,
        )
    })
}

pub(crate) fn replace_expressions(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    let expressions = parse_expressions(input);
    let evaluations = expressions
        .iter()
        .map(|e| {
            eval(&e.expression()).map(|v| match v {
                Value::String(s) => s,
                _ => v.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut result = input.to_string();

    for (i, e) in expressions.iter().enumerate() {
        let expr = &input[e.indices.clone()];
        result = result.replace(expr, &evaluations[i]);
    }
    Ok(result)
}

pub(crate) fn parse_expressions(input: &str) -> Vec<Expression> {
    if !input.contains('$') {
        return vec![];
    }

    //split into substrings
    let slices = split_ranges(input, '$');
    let map = input.char_indices().collect::<HashMap<_, _>>();

    let mut expressions = vec![];

    for (start, end) in &slices {
        if map[start] != '$' || end - start < 4 || !['(', '{'].contains(&map[&(start + 1)]) {
            continue;
        }

        let opening = map[&(start + 1)];
        let closing = if opening == '(' { ')' } else { '}' };
        let mut open_braces = 0;

        let extype = if opening == '(' {
            ExpressionType::Paren
        } else {
            ExpressionType::Bracket
        };

        //get expression body
        for i in *start..*end {
            if map[&i] == opening {
                open_braces += 1;
            }
            if map[&i] == closing {
                open_braces -= 1;
                if open_braces == 0 {
                    expressions.push(Expression {
                        expression: input[*start + 2..i].to_string(),
                        type_: extype,
                        indices: *start..i + 1,
                    });
                    break;
                }
            }
        }
    }
    expressions
}

#[derive(Debug)]
pub enum ExpressionType {
    Paren,
    Bracket,
}

#[derive(Debug)]
pub struct Expression {
    pub type_: ExpressionType,
    pub expression: String,
    pub indices: Range<usize>,
}

impl Expression {
    pub fn expression(&self) -> String {
        match self.type_ {
            ExpressionType::Paren => self.expression.clone(),
            ExpressionType::Bracket => format!("(() => {{{}}})();", self.expression),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression() {
        let expression = "parseInt(\"161\")";
        let result = eval(expression).unwrap_or_default().as_u64().unwrap_or_default();
        assert_eq!(result, 161);
    }

    #[test]
    fn test_parse_expressions() {
        let input = "This is $(\"a \")$(\"string\") for $2,50";
        let result = parse_expressions(input);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_replace_expressions() {
        let input = "This is $(\"a \")$(\"string\")";
        let result = replace_expressions(input).unwrap_or_default();
        assert_eq!(result, "This is a string".to_string());
    }

    #[test]
    fn test_replace_bodied_expression() {
        let input = r#"My favorite number is ${
        return parseInt("161");
    }"#;
        let result = replace_expressions(input).unwrap_or_default();
        assert_eq!(result, "My favorite number is 161".to_string());
    }

    #[test]
    fn test_engine_values() {
        let runtime = RuntimeEnvironment {
            runtime: HashMap::from([("my_value".to_string(), "Hello World!".to_string())]),
            ..Default::default()
        };
        let input = "$(runtime.my_value)";
        prepare_expression_engine(&runtime).unwrap();
        let result = replace_expressions(input).unwrap_or_default();
        assert_eq!(result, "Hello World!".to_string());
        reset_expression_engine().unwrap();
        let result = replace_expressions(input);
        assert!(result.is_err());
    }
}
