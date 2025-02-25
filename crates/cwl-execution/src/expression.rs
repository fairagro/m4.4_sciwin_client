use crate::RuntimeEnvironment;
use rustyscript::static_runtime;
use serde_json::Value;

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
    RUNTIME::with(|rt| rt.eval::<()>(r#"
            var inputs = undefined;
            var runtime = undefined;
            var self = undefined;"#))
}
