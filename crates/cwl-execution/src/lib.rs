pub mod environment;
pub mod expression;
pub mod io;
pub mod runner;
pub mod staging;
pub mod util;
pub mod validate;

use cwl::types::{guess_type, CWLType, DefaultValue, Directory, File, PathItem};
use cwl::CWLDocument;
use io::preprocess_path_join;
use runner::{run_tool, run_workflow};
use serde_yaml::Value;
use std::{cell::RefCell, collections::HashMap, error::Error, fmt::Display, fs, num::NonZero, path::Path, process::Command, thread};
use sysinfo::{Disks, MemoryRefreshKind, System};
use util::preprocess_cwl;

pub fn execute_cwlfile(cwlfile: impl AsRef<Path>, raw_inputs: &[String], outdir: Option<impl AsRef<Path>>) -> Result<(), Box<dyn Error>> {
    //gather inputs
    let mut inputs = if raw_inputs.len() == 1 && !raw_inputs[0].starts_with('-') {
        let yaml = fs::read_to_string(&raw_inputs[0])?;
        serde_yaml::from_str(&yaml).map_err(|e| format!("Could not read job file: {e}"))?
    } else {
        raw_inputs
            .chunks_exact(2)
            .filter_map(|pair| {
                if let Some(key) = pair[0].strip_prefix("--") {
                    let raw_value = &pair[1];
                    let value = match guess_type(raw_value) {
                        CWLType::File => DefaultValue::File(File::from_location(raw_value)),
                        CWLType::Directory => DefaultValue::Directory(Directory::from_location(raw_value)),
                        CWLType::String => DefaultValue::Any(Value::String(raw_value.to_string())),
                        _ => DefaultValue::Any(serde_yaml::from_str(raw_value).expect("Could not read input")),
                    };
                    Some((key.to_string(), value))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>()
    };

    fn correct_path<T: PathItem>(item: &mut T, path_prefix: &Path) {
        let mut location = item.get_location().clone();
        if location.is_empty() {
            return;
        }
        if location.starts_with("file://") {
            location = location.strip_prefix("file://").unwrap_or(&location).to_string();
        }

        item.set_location(preprocess_path_join(path_prefix, &location));
        if let Some(secondary_files) = item.secondary_files_mut() {
            for sec_file in secondary_files {
                match sec_file {
                    DefaultValue::File(file) => {
                        file.set_location(preprocess_path_join(path_prefix, &file.get_location()));
                    }
                    DefaultValue::Directory(directory) => directory.set_location(preprocess_path_join(path_prefix, &directory.get_location())),
                    _ => (),
                }
            }
        }
    }

    //make paths relative to calling object
    let path_prefix = if raw_inputs.len() == 1 && !raw_inputs[0].starts_with('-') {
        Path::new(&raw_inputs[0]).parent().unwrap() //path of job file
    } else {
        Path::new(".")
    };
    for value in inputs.values_mut() {
        match value {
            DefaultValue::File(file) => correct_path(file, path_prefix),
            DefaultValue::Directory(directory) => correct_path(directory, path_prefix),
            _ => (),
        }
    }

    let output_values = execute(cwlfile, inputs, outdir)?;
    let json = serde_json::to_string_pretty(&output_values)?;
    println!("{json}");

    Ok(())
}

pub fn execute(
    cwlfile: impl AsRef<Path>,
    inputs: HashMap<String, DefaultValue>,
    outdir: Option<impl AsRef<Path>>,
) -> Result<HashMap<String, DefaultValue>, Box<dyn Error>> {
    //load cwl
    let contents = fs::read_to_string(&cwlfile).map_err(|e| format!("Could not read CWL File {:?}: {e}", cwlfile.as_ref()))?;
    let contents = preprocess_cwl(&contents, &cwlfile)?;
    let mut doc: CWLDocument = serde_yaml::from_str(&contents).map_err(|e| format!("Could not parse CWL File {:?}: {e}", cwlfile.as_ref()))?;

    match doc {
        CWLDocument::CommandLineTool(_) | CWLDocument::ExpressionTool(_) => run_tool(
            &mut doc,
            inputs,
            Some(&cwlfile.as_ref().to_path_buf()),
            outdir.map(|d| d.as_ref().to_string_lossy().into_owned()),
        ),
        CWLDocument::Workflow(mut workflow) => run_workflow(
            &mut workflow,
            inputs,
            Some(&cwlfile.as_ref().to_path_buf()),
            outdir.map(|d| d.as_ref().to_string_lossy().into_owned()),
        ),
    }
}

pub trait ExitCode {
    fn exit_code(&self) -> i32;
}

#[derive(Debug)]
pub struct CommandError {
    pub message: String,
    pub exit_code: i32,
}

impl Error for CommandError {}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, code: {}", self.message, self.exit_code)
    }
}

impl ExitCode for CommandError {
    fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

pub(crate) fn get_processor_count() -> usize {
    thread::available_parallelism().map(NonZero::get).unwrap_or(1)
}

pub(crate) fn get_available_ram() -> u64 {
    let mut system = System::new_all();
    system.refresh_memory_specifics(MemoryRefreshKind::everything());
    system.free_memory() / 1024
}

pub(crate) fn get_available_disk_space() -> u64 {
    let disks = Disks::new_with_refreshed_list();
    disks[0].available_space() / 1024
}

pub fn format_command(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();

    let args: Vec<String> = command
        .get_args()
        .map(|arg| {
            let arg_str = arg.to_string_lossy();
            arg_str.to_string()
        })
        .collect();

    format!("{} {}", program, args.join(" "))
}

pub(crate) fn split_ranges(s: &str, delim: char) -> Vec<(usize, usize)> {
    let mut slices = Vec::new();
    let mut last_index = 0;

    for (idx, _) in s.match_indices(delim) {
        if last_index != idx {
            slices.push((last_index, idx));
        }
        last_index = idx;
    }

    if last_index < s.len() {
        slices.push((last_index, s.len()));
    }

    slices
}

#[derive(Default, Clone, Copy)]
pub enum ContainerEngine {
    #[default]
    Docker,
    Podman,
}

impl Display for ContainerEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerEngine::Docker => write!(f, "docker"),
            ContainerEngine::Podman => write!(f, "podman"),
        }
    }
}

thread_local! {static CONTAINER_ENGINE: RefCell<ContainerEngine> = const { RefCell::new(ContainerEngine::Docker) };}

pub fn set_container_engine(value: ContainerEngine) {
    CONTAINER_ENGINE.with(|engine| *engine.borrow_mut() = value);
}

pub fn container_engine() -> ContainerEngine {
    CONTAINER_ENGINE.with(|engine| *engine.borrow())
}
