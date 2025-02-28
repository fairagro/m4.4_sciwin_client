use crate::{expression::evaluate_expression, replace_expressions};
use cwl::{
    clt::{Argument, CommandLineTool},
    requirements::Requirement,
    types::{Entry, EnviromentDefs, EnvironmentDef, Include},
};
use fancy_regex::{Captures, Regex};
use std::{fs, path::Path};

pub(crate) fn preprocess_imports(contents: &str, path: impl AsRef<Path>) -> String {
    let import_regex = Regex::new(r#"(?P<indent>[\p{Z}-]*)\{*"*\$import"*: (?P<file>[\w\.\-_]*)\}*"#).unwrap();

    import_regex
        .replace_all(contents, |captures: &Captures| {
            let filename = captures.name("file").map_or("", |m| m.as_str());
            let indent = captures.name("indent").map_or("", |m| m.as_str());
            let indent_level: String = " ".repeat(indent.len());

            let path = path
                .as_ref()
                .parent()
                .map(|p| p.join(filename))
                .unwrap_or_else(|| Path::new(filename).to_path_buf());

            fs::read_to_string(&path)
                .map(|c| {
                    let mut lines = c.lines();
                    let first = lines.next().unwrap_or_default();
                    let mut result = format!("{indent}{first}");
                    for line in lines {
                        result.push('\n');
                        result.push_str(&format!("{indent_level}{line}"));
                    }
                    result
                })
                .unwrap_or_default()
        })
        .to_string()
}

fn eval(input: &str) -> String {
    replace_expressions(input).unwrap_or(input.to_string())
}

pub(crate) fn process_expressions(tool: &mut CommandLineTool) {
    //evaluate arguments
    if let Some(args) = &mut tool.arguments {
        for arg in args.iter_mut() {
            *arg = match arg {
                Argument::String(str) => Argument::String(eval(str)),
                Argument::Binding(binding) => {
                    let mut binding = binding.clone();
                    if let Some(value_from) = &mut binding.value_from {
                        *value_from = eval(value_from);
                    }
                    Argument::Binding(binding)
                }
            }
        }
    }

    //evaluate output.output_binding & output.format
    for output in tool.outputs.iter_mut() {
        if let Some(binding) = &mut output.output_binding {
            let value = evaluate_expression(&binding.glob)
                .map(|r| serde_json::to_string(&r).unwrap().replace(r#"""#, ""))
                .unwrap_or(binding.glob.clone());
            binding.glob = value;
        }
        if let Some(format) = &mut output.format {
            *format = eval(format);
        }
    }
    //evaluate input.format
    for input in tool.inputs.iter_mut() {
        if let Some(format) = &mut input.format {
            *format = eval(format);
        }
    }

    //evaluate requirements
    if let Some(requirements) = &mut tool.requirements {
        for requirement in requirements.iter_mut() {
            process_requirement(requirement);
        }
    }
    //evaluate hints
    if let Some(requirements) = &mut tool.hints {
        for requirement in requirements.iter_mut() {
            process_requirement(requirement);
        }
    }

    //evaluate stdXs
    if let Some(stdin) = &mut tool.stdin {
        *stdin = eval(stdin);
    }
    if let Some(stdout) = &mut tool.stdout {
        *stdout = eval(stdout);
    }
    if let Some(stderr) = &mut tool.stderr {
        *stderr = eval(stderr);
    }
}

fn process_requirement(req: &mut Requirement) {
    if let Requirement::EnvVarRequirement(evr) = req {
        evr.env_def = match &mut evr.env_def {
            EnviromentDefs::Vec(defs) => EnviromentDefs::Vec(
                defs.iter()
                    .map(|d| EnvironmentDef {
                        env_value: eval(&d.env_value),
                        env_name: d.env_name.clone(),
                    })
                    .collect(),
            ),
            EnviromentDefs::Map(map) => EnviromentDefs::Map(map.iter().map(|(k, v)| (k.clone(), eval(v))).collect()),
        }
    }

    if let Requirement::InitialWorkDirRequirement(iwdr) = req {
        for listing in iwdr.listing.iter_mut() {
            listing.entryname = eval(&listing.entryname);
            listing.entry = match &mut listing.entry {
                Entry::Source(src) => Entry::Source(eval(src)),
                Entry::Include(include) => Entry::Include(Include {
                    include: eval(&include.include),
                }),
            }
        }
    }
}
