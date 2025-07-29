use crate::{InputObject, environment::RuntimeEnvironment, split_ranges};
use commonwl::{
    Argument, CWLDocument, Command, DefaultValue, Entry, Expression, ExpressionType,
    requirements::{Requirement, WorkDirItem},
};
use rustyscript::static_runtime;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{collections::HashMap, error::Error, fs, path::Path};

static_runtime!(RUNTIME);

pub(crate) fn prepare_expression_engine(environment: &RuntimeEnvironment) -> Result<(), Box<dyn Error>> {
    let inputs = serde_json::to_string(&environment.inputs)?;
    let runtime = serde_json::to_string(&environment.runtime)?;

    RUNTIME::with(|rt| rt.eval::<()>(format!("var inputs = {inputs}; var runtime = {runtime}")))?;

    Ok(())
}

pub(crate) fn reset_expression_engine() -> Result<(), Box<dyn Error>> {
    Ok(RUNTIME::with(|rt| {
        rt.eval::<()>(
            r#"
            var inputs = undefined;
            var runtime = undefined;
            var self = undefined;"#,
        )
    })?)
}

pub(crate) fn eval(expression: &str) -> Result<Value, Box<dyn Error>> {
    eval_generic(expression)
}

pub(crate) fn eval_generic<T: DeserializeOwned>(expression: &str) -> Result<T, Box<dyn Error>> {
    Ok(RUNTIME::with(|rt| rt.eval::<T>(expression))?)
}

pub(crate) fn eval_tool<T: DeserializeOwned>(expression: &str) -> Result<T, Box<dyn Error>> {
    Ok(RUNTIME::with(|rt| rt.eval::<T>(format!("var outputs = {expression}; outputs")))?)
}

pub(crate) fn load_lib(lib: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    Ok(RUNTIME::with(|rt| {
        let contents = fs::read_to_string(lib.as_ref()).unwrap();
        rt.eval(contents)
    })?)
}

pub(crate) fn set_self<T: Serialize>(me: &T) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string(me)?;
    RUNTIME::with(|rt| rt.eval::<()>(format!("var self = {json};")))?;
    Ok(())
}

pub(crate) fn unset_self() -> Result<(), Box<dyn Error>> {
    RUNTIME::with(|rt| rt.eval::<()>("var self = undefined;".to_string()))?;
    Ok(())
}

pub(crate) fn evaluate_expression(input: &str) -> Result<Value, Box<dyn Error>> {
    let expressions = parse_expressions(input);

    if !expressions.is_empty() {
        let expression = &expressions[0];
        let result = eval(&expression.expression())?;
        return Ok(result);
    }

    Ok(Value::String(input.to_string()))
}

pub(crate) fn evaluate_condition(input: &str, inputs: &HashMap<String, DefaultValue>) -> Result<bool, Box<dyn Error>> {
    prepare_expression_engine(&RuntimeEnvironment {
        inputs: inputs.clone(),
        ..Default::default()
    })?;
    let result = evaluate_expression(input)?.as_bool().unwrap_or(false);
    reset_expression_engine()?;
    Ok(result)
}

pub(crate) fn output_eval(input: &str) -> Result<Value, Box<dyn Error>> {
    let expressions = parse_expressions(input);

    if expressions.is_empty() {
        return Ok(Value::String(input.to_string()));
    }

    // Special case: the input is a single full expression
    if expressions.len() == 1 {
        let expr = &expressions[0];
        if expr.indices.start == 0 && expr.indices.end == input.len() {
            return evaluate_expression(input);
        }
    }

    let mut output = String::new();
    let mut last = 0;

    for expr in expressions {
        // Append the part before the expression
        output.push_str(&input[last..expr.indices.start]);

        let result = eval(&expr.expression())?;

        // Replace expression with result
        match result {
            Value::String(s) => output.push_str(&s),
            Value::Number(n) => output.push_str(&n.to_string()),
            Value::Bool(b) => output.push_str(&b.to_string()),
            _ => output.push_str(&serde_json::to_string(&result)?),
        }

        last = expr.indices.end;
    }

    // Append remaining string
    output.push_str(&input[last..]);

    match serde_json::from_str(&output) {
        Ok(parsed) => Ok(parsed),
        Err(_) => Ok(Value::String(output)),
    }
}

pub(crate) fn replace_expressions(input: &str) -> Result<String, Box<dyn Error>> {
    let expressions = parse_expressions(input);
    let evaluations = expressions
        .iter()
        .map(|e| eval_generic::<DefaultValue>(&e.expression()).map(|v| v.as_value_string()))
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

pub(crate) fn process_expressions(tool: &mut CWLDocument, input_values: &mut InputObject) -> Result<(), Box<dyn Error>> {
    for requirement in &mut input_values.requirements {
        if let Requirement::InitialWorkDirRequirement(wd_req) = requirement {
            for listing in &mut wd_req.listing {
                if let WorkDirItem::Dirent(dirent) = listing {
                    if let Some(entryname) = &mut dirent.entryname {
                        *entryname = replace_expressions(entryname)?;
                    }
                    dirent.entry = match &mut dirent.entry {
                        Entry::Source(src) => {
                            *src = replace_expressions(src)?;
                            Entry::Source(src.clone())
                        }
                        Entry::Include(include) => {
                            include.include = replace_expressions(&include.include)?;
                            Entry::Include(include.clone())
                        }
                    }
                }
            }
        }
    }
    for input in &mut tool.inputs {
        let tmp = input.id.clone();
        if input.secondary_files.is_empty() {
            continue;
        }

        if let Some(DefaultValue::File(file)) = input_values.inputs.get(&tmp) {
            set_self(&file.preload())?;
            for sec_file in &mut input.secondary_files {
                sec_file.pattern = replace_expressions(&sec_file.pattern)?;
            }
            unset_self()?;
        }
    }
    if let CWLDocument::CommandLineTool(clt) = tool {
        clt.base_command = match std::mem::take(&mut clt.base_command) {
            Command::Single(cmd) => Command::Single(replace_expressions(&cmd)?),
            Command::Multiple(mut vec) => {
                for item in vec.iter_mut() {
                    *item = replace_expressions(item)?
                }
                Command::Multiple(vec)
            }
        };

        if let Some(args) = &mut clt.arguments {
            for arg in args.iter_mut() {
                *arg = match arg {
                    Argument::String(str) => Argument::String(replace_expressions(str)?),
                    Argument::Binding(binding) => {
                        let mut new_binding = binding.clone();
                        if let Some(value_from) = &mut new_binding.value_from {
                            *value_from = replace_expressions(value_from)?;
                        }
                        Argument::Binding(new_binding)
                    }
                }
            }
        }

        for output in &mut clt.outputs {
            if let Some(binding) = &mut output.output_binding
                && let Some(glob) = binding.glob.as_mut()
            {
                *glob = replace_expressions(glob)?;
            }
            if let Some(format) = &mut output.format {
                let format = replace_expressions(format)?;
                output.format = Some(format);
            }
        }

        if let Some(stdin) = &mut clt.stdin {
            *stdin = replace_expressions(stdin)?;
        }

        if let Some(stdout) = &mut clt.stdout {
            *stdout = replace_expressions(stdout)?;
        }

        if let Some(stderr) = &mut clt.stderr {
            *stderr = replace_expressions(stderr)?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use commonwl::StringOrNumber;

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
            runtime: HashMap::from([("my_value".to_string(), StringOrNumber::String("Hello World!".to_string()))]),
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

    #[test]
    fn test_output_eval() {
        let input = "This is $(\"a \")$(\"string\")";
        let result = output_eval(input).unwrap_or_default();
        assert_eq!(result, "This is a string".to_string());
    }

    #[test]
    fn test_output_eval_single_expression() {
        let input = "$(\"string\")";
        let result = output_eval(input).unwrap_or_default();
        assert_eq!(result, "string".to_string());
    }

    #[test]
    fn test_process_tool_expressions() {
        let tool = r#"
        #!/usr/bin/env cwl-runner

cwlVersion: v1.2
class: CommandLineTool

inputs:
- id: dirname
  type: string

outputs: 
  out: 
    type: Directory
    outputBinding:
      glob: 
        $(inputs.dirname)
arguments:
- mkdir
- $(inputs.dirname)

stdout: $(inputs.dirname)/stdout
stderr: $(inputs.dirname)/stderr
"#;

        let runtime = RuntimeEnvironment {
            inputs: HashMap::from([("dirname".to_string(), DefaultValue::Any(serde_yaml::Value::String("testdir".to_string())))]),
            ..Default::default()
        };
        prepare_expression_engine(&runtime).unwrap();
        let mut tool: CWLDocument = serde_yaml::from_str(tool).unwrap();
        let mut input_values = InputObject::default();
        process_expressions(&mut tool, &mut input_values).unwrap();
        reset_expression_engine().unwrap();

        assert!(matches!(tool, CWLDocument::CommandLineTool(_)));
        if let CWLDocument::CommandLineTool(tool) = tool {
            assert_eq!(tool.stdout, Some("testdir/stdout".to_string()));
            assert_eq!(tool.stderr, Some("testdir/stderr".to_string()));
        }
    }
}
