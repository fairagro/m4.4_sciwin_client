use std::collections::HashMap;

use cwl::et::{Expression, ExpressionType};
use rustyscript::static_runtime;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use crate::{environment::RuntimeEnvironment, split_ranges};

static_runtime!(RUNTIME);

pub(crate) fn prepare_expression_engine(environment: &RuntimeEnvironment) -> Result<(), rustyscript::Error> {
    let inputs = serde_json::to_string(&environment.inputs)?;
    let runtime = serde_json::to_string(&environment.runtime)?;

    RUNTIME::with(|rt| rt.eval::<()>(format!("var inputs = {inputs}; var runtime = {runtime}")))?;

    Ok(())
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

pub(crate) fn eval(expression: &str) -> Result<Value, rustyscript::Error> {
    eval_generic(expression)
}

pub(crate) fn eval_generic<T: DeserializeOwned>(expression: &str) -> Result<T, rustyscript::Error> {
    RUNTIME::with(|rt| rt.eval::<T>(expression))
}

pub (crate) fn eval_tool<T: DeserializeOwned>(expression: &str) -> Result<T, rustyscript::Error> {
    RUNTIME::with(|rt| rt.eval::<T>(format!("var outputs = {expression}; outputs")))
}

pub(crate) fn set_self<T: Serialize>(me: &T) -> Result<(), rustyscript::Error> {
    let json = serde_json::to_string(me)?;
    RUNTIME::with(|rt| rt.eval::<()>(format!("var self = {json};")))?;
    Ok(())
}

pub(crate) fn unset_self() -> Result<(), rustyscript::Error> {
    RUNTIME::with(|rt| rt.eval::<()>("var self = undefined;".to_string()))?;
    Ok(())
}

pub(crate) fn evaluate_expression(input: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let expressions = parse_expressions(input);

    if !expressions.is_empty() {
        let expression = &expressions[0];
        let result = eval(&expression.expression())?;
        return Ok(result);
    }

    Ok(Value::String(input.to_string()))
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
}