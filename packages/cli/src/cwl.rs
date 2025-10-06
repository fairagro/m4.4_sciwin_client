use crate::util::{get_workflows_folder, repo::get_submodule_paths, resolve_path};
use commonwl::{
    CWLDocument, CommandLineTool, DefaultValue, Entry, PathItem, Workflow,
    requirements::{Requirement, WorkDirItem},
};
use dialoguer::{Select, theme::ColorfulTheme};
use git2::Repository;
use log::info;
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};

pub trait Connectable {
    fn remove_output_connection(&mut self, from: &str, to_output: &str) -> Result<(), Box<dyn Error>>;
    fn remove_input_connection(&mut self, from_input: &str, to: &str) -> Result<(), Box<dyn Error>>;
    fn add_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>>;
    fn add_output_connection(&mut self, from: &str, to_output: &str) -> Result<(), Box<dyn Error>>;
    fn add_input_connection(&mut self, from_input: &str, to: &str) -> Result<(), Box<dyn Error>>;
    fn add_new_step_if_not_exists(&mut self, name: &str, path: &str, doc: &CWLDocument);
    fn remove_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>>;
}

pub trait Saveable {
    fn prepare_save(&mut self, path: &str) -> String;
}

impl Saveable for CommandLineTool {
    fn prepare_save(&mut self, path: &str) -> String {
        //rewire paths to new location
        for input in &mut self.inputs {
            if let Some(DefaultValue::File(value)) = &mut input.default {
                value.location = Some(resolve_path(value.get_location(), path));
            }
            if let Some(DefaultValue::Directory(value)) = &mut input.default {
                value.location = Some(resolve_path(value.get_location(), path));
            }
        }

        for requirement in &mut self.requirements {
            if let Requirement::DockerRequirement(docker) = requirement {
                if let Some(Entry::Include(include)) = &mut docker.docker_file {
                    include.include = resolve_path(&include.include, path);
                }
            } else if let Requirement::InitialWorkDirRequirement(iwdr) = requirement {
                for listing in &mut iwdr.listing {
                    if let WorkDirItem::Dirent(dirent) = listing
                        && let Entry::Include(include) = &mut dirent.entry
                    {
                        include.include = resolve_path(&include.include, path);
                    }
                }
            }
        }
        self.to_string()
    }
}

impl Connectable for Workflow {
    fn add_new_step_if_not_exists(&mut self, name: &str, path: &str, doc: &CWLDocument) {
        s4n_core::workflow::add_new_step_if_not_exists(self, name, path, doc);
        info!("➕ Added step {name} to workflow");
    }

    /// Adds a connection between an input and a `CommandLineTool`. The tool will be registered as step if it is not already and an Workflow input will be added.
    fn add_input_connection(&mut self, from_input: &str, to: &str) -> Result<(), Box<dyn Error>> {
        let to_parts = to.split('/').collect::<Vec<_>>();
        let to_filename = resolve_filename(to_parts[0])?;

        s4n_core::workflow::add_input_connection(self, from_input, to_parts[0], to_parts[1], &to_filename)?;
        info!("➕ Added or updated connection from inputs.{from_input} to {to} in workflow");

        Ok(())
    }

    /// Adds a connection between an output and a `CommandLineTool`. The tool will be registered as step if it is not already and an Workflow output will be added.
    fn add_output_connection(&mut self, from: &str, to_output: &str) -> Result<(), Box<dyn Error>> {
        let from_parts = from.split('/').collect::<Vec<_>>();
        let from_filename = resolve_filename(from_parts[0])?;

        s4n_core::workflow::add_output_connection(self, from_parts[0], from_parts[1], &from_filename, to_output)?;
        info!("➕ Added or updated connection from {from} to outputs.{to_output} in workflow!");

        Ok(())
    }

    /// Adds a connection between two `CommandLineTools`. The tools will be registered as step if registered not already.
    fn add_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>> {
        //handle from
        let from_parts = from.split('/').collect::<Vec<_>>();
        let from_filename = resolve_filename(from_parts[0])?;
        //handle to
        let to_parts = to.split('/').collect::<Vec<_>>();
        let to_filename = resolve_filename(to_parts[0])?;

        s4n_core::workflow::add_step_connection(self, &from_filename, from_parts[0], from_parts[1], &to_filename, to_parts[0], to_parts[1])?;
        info!("🔗 Added connection from {from} to {to} in workflow!");

        Ok(())
    }

    /// Removes a connection between two `CommandLineTools` by removing input from `tool_y` that is also output of `tool_x`.
    fn remove_step_connection(&mut self, from: &str, to: &str) -> Result<(), Box<dyn Error>> {
        let from_parts = from.split('/').collect::<Vec<_>>();
        let to_parts = to.split('/').collect::<Vec<_>>();
        if from_parts.len() != 2 {
            return Err(format!("Invalid '--from' format: {from}. Please use tool/parameter or @inputs/parameter.").into());
        }
        if to_parts.len() != 2 {
            return Err(format!("Invalid '--to' format: {to}. Please use tool/parameter or @outputs/parameter.").into());
        }
        if !self.has_step(to_parts[0]) {
            return Err(format!("Step {} not found!", to_parts[0]).into());
        }

        s4n_core::workflow::remove_step_connection(self, to_parts[0], to_parts[1])?;
        info!("➖ Removed connection from {from} to {to} in workflow!");
        Ok(())
    }

    /// Removes an input from inputs and removes it from `CommandLineTool` input.
    fn remove_input_connection(&mut self, from_input: &str, to: &str) -> Result<(), Box<dyn Error>> {
        let to_parts = to.split('/').collect::<Vec<_>>();
        if to_parts.len() != 2 {
            return Err(format!("Invalid 'to' format for input connection: {from_input} to:{to}").into());
        }

        s4n_core::workflow::remove_input_connection(self, from_input, to_parts[0], to_parts[1])?;
        info!("➖ Removed connection from inputs.{from_input} to {to} in workflow");
        Ok(())
    }

    /// Removes a connection between an output and a `CommandLineTool`.
    fn remove_output_connection(&mut self, from: &str, to_output: &str) -> Result<(), Box<dyn Error>> {
        let from_parts = from.split('/').collect::<Vec<_>>();

        s4n_core::workflow::remove_output_connection(self, from_parts[0], from_parts[1], to_output)?;
        info!("➖ Removed connection to {to_output} from workflow!");
        Ok(())
    }
}

/// Locates CWL File by name
pub fn resolve_filename(cwl_filename: &str) -> Result<String, Box<dyn Error>> {
    let mut candidates: Vec<PathBuf> = vec![];

    //check if exists in workflows folder
    let cwl_filename = cwl_filename.strip_suffix(".cwl").unwrap_or(cwl_filename);
    let path = format!("{}{}/{}.cwl", get_workflows_folder(), cwl_filename, cwl_filename);
    let path = Path::new(&path);
    if path.exists() {
        candidates.push(path.to_path_buf());
    }

    //let else = hell yeah!
    let Ok(repo) = Repository::open(".") else {
        if !candidates.is_empty() {
            return Ok(candidates[0].to_string_lossy().into_owned());
        }
        return Err("No candidates available".into());
    };

    for module_path in get_submodule_paths(&repo)? {
        let sub_path = module_path.join(path);
        if sub_path.exists() {
            candidates.push(sub_path);
        }
    }

    match candidates.len() {
        1 => Ok(candidates[0].to_string_lossy().into_owned()),
        0 => Err("Could not resolve filename".into()),
        _ => {
            let items: Vec<String> = candidates.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Multiple candidates are found. Select the CWL File to use")
                .items(&items)
                .default(0)
                .report(true)
                .interact()?;
            Ok(items[selection].clone())
        }
    }
}

#[allow(clippy::disallowed_macros)]
pub fn highlight_cwl(yaml: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("yaml").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["InspiredGitHub"]);

    for line in LinesWithEndings::from(yaml) {
        let ranges = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{escaped}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{CreateArgs, create_tool};
    use commonwl::{
        CWLType, Command, Dirent, File,
        inputs::{CommandInputParameter, CommandLineBinding},
        requirements::{DockerRequirement, InitialWorkDirRequirement},
    };
    use fstest::fstest;
    use serde_yaml::Value;
    use std::{
        env,
        path::{MAIN_SEPARATOR, Path},
    };
    use test_utils::os_path;

    #[fstest(repo = true, files = ["../../tests/test_data/input.txt", "../../tests/test_data/echo.py"])]
    fn test_resolve_filename() {
        create_tool(&CreateArgs {
            command: vec!["python".to_string(), "echo.py".to_string(), "--test".to_string(), "input.txt".to_string()],
            ..Default::default()
        })
        .unwrap();

        let name = "echo";
        let path = resolve_filename(name).unwrap();
        assert_eq!(path, format!("{}{name}/{name}.cwl", get_workflows_folder()));
    }

    #[fstest(repo = true, files = ["../../tests/test_data/input.txt", "../../tests/test_data/echo.py"])]
    fn test_resolve_filename_in_submodule() {
        let repo = Repository::open(env::current_dir().unwrap()).unwrap();
        let mut module = repo
            .submodule("https://github.com/fairagro/M4.4_UC6_ARC", Path::new("uc6"), false)
            .unwrap();
        module.init(false).unwrap();
        let subrepo = module.open().unwrap();

        subrepo
            .find_remote("origin")
            .unwrap()
            .fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)
            .unwrap();
        subrepo.set_head("refs/remotes/origin/main").unwrap();
        subrepo.checkout_head(None).unwrap();
        module.add_finalize().unwrap();

        let name = "get_soil_data";
        let path = resolve_filename(name).unwrap();
        assert_eq!(
            path,
            format!(
                "{}{MAIN_SEPARATOR}{}{name}/{name}.cwl",
                module.path().to_string_lossy(),
                get_workflows_folder()
            )
        );
    }

    #[test]
    pub fn test_cwl_save() {
        let inputs = vec![
            CommandInputParameter::default()
                .with_id("positional1")
                .with_default_value(DefaultValue::File(File::from_location("test_data/input.txt")))
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_position(0)),
            CommandInputParameter::default()
                .with_id("option1")
                .with_type(CWLType::String)
                .with_binding(CommandLineBinding::default().with_prefix("--option1"))
                .with_default_value(DefaultValue::Any(Value::String("value1".to_string()))),
        ];
        let mut clt = CommandLineTool::default()
            .with_base_command(Command::Multiple(vec!["python".to_string(), "test/script.py".to_string()]))
            .with_inputs(inputs)
            .with_requirements(vec![
                Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement::from_file("test/script.py")),
                Requirement::DockerRequirement(DockerRequirement::from_file("test/data/Dockerfile", "test")),
            ]);

        clt.prepare_save("workflows/tool/tool.cwl");

        //check if paths are rewritten upon tool saving

        assert_eq!(
            clt.inputs[0].default,
            Some(DefaultValue::File(File::from_location(&os_path("../../test_data/input.txt"))))
        );
        let requirements = &clt.requirements;
        let req_0 = &requirements[0];
        let req_1 = &requirements[1];
        assert_eq!(
            *req_0,
            Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement {
                listing: vec![WorkDirItem::Dirent(Dirent {
                    entry: Entry::from_file(&os_path("../../test/script.py")),
                    entryname: Some("test/script.py".to_string()),
                    ..Default::default()
                })]
            })
        );
        assert_eq!(
            *req_1,
            Requirement::DockerRequirement(DockerRequirement {
                docker_file: Some(Entry::from_file(&os_path("../../test/data/Dockerfile"))),
                docker_image_id: Some("test".to_string()),
                ..Default::default()
            })
        );
    }
}
