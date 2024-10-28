use super::clt::{Command, CommandInputParameter, CommandLineTool, CommandOutputParameter, DefaultValue, Entry, Requirement};
use crate::{
    cwl::types::{CWLType, OutputDirectory, OutputFile, OutputItem},
    io::{copy_dir, copy_file, create_and_write_file, get_file_checksum, get_file_size, get_filename_without_extension},
};
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{self, create_dir_all},
    path::{Path, PathBuf},
    process::Command as SystemCommand,
    vec,
};
use tempfile::{tempdir, TempDir};

pub fn run_commandlinetool(tool: &CommandLineTool, input_values: Option<HashMap<String, DefaultValue>>, cwl_path: Option<&str>, outdir: Option<String>) -> Result<(), Box<dyn Error>> {
    //TODO: handle container
    let dir = tempdir()?;
    eprintln!("📁 Created staging directory: {:?}", dir.path());

    //save current dir
    let current = env::current_dir()?;
    let tool_path = if let Some(file) = cwl_path { Path::new(file).parent().unwrap() } else { Path::new(".") };
    //change to cwl dir as paths are given relative to here
    env::set_current_dir(current.join(tool_path))?;

    //stage files
    let staged_files = stage_needed_files(tool, &dir, &input_values, tool_path)?;

    //change working directory
    let tmp_tool_dir = dir.path().join(tool_path);
    create_dir_all(&tmp_tool_dir)?;
    env::set_current_dir(tmp_tool_dir)?;

    //run the tool's command
    run_command(tool, input_values).map_err(|e| format!("Could not execute tool command: {}", e))?;

    //remove staged files
    for file in staged_files {
        fs::remove_file(file)?;
    }

    //evaluate outputs
    let output_directory = if let Some(out) = outdir { &PathBuf::from(out) } else { &current };
    evaluate_outputs(&tool.outputs, output_directory)?;

    //reset dir to calling directory
    env::set_current_dir(&current)?;

    eprintln!("✔️  Command Executed with status: success!");
    Ok(())
}

pub fn run_command(tool: &CommandLineTool, input_values: Option<HashMap<String, DefaultValue>>) -> Result<(), Box<dyn Error>> {
    //get executable
    let cmd = match &tool.base_command {
        Command::Single(cmd) => cmd,
        Command::Multiple(vec) => &vec[0],
    };

    let mut command = SystemCommand::new(cmd);
    //append rest of base command as args
    if let Command::Multiple(ref vec) = &tool.base_command {
        command.args(&vec[1..]);
    }

    //TODO: handle arguments field...

    //build inputs from either fn-args or default values.
    let mut inputs = vec![];
    for input in &tool.inputs {
        if let Some(binding) = &input.input_binding {
            if let Some(prefix) = &binding.prefix {
                inputs.push(prefix.clone());
            }
        }
        inputs.push(evaluate_input(input, &input_values)?);
    }
    command.args(inputs);

    //run
    let output = command.output()?;

    //handle redirection of stdout
    if !output.stdout.is_empty() {
        if let Some(stdout) = &tool.stdout {
            let out = &String::from_utf8_lossy(&output.stdout);
            create_and_write_file(stdout, out)?;
        } else {
            eprintln!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }

    //handle redirection of stderr
    if !output.stderr.is_empty() {
        if let Some(stderr) = &tool.stderr {
            let out = &String::from_utf8_lossy(&output.stderr);
            create_and_write_file(stderr, out)?;
        } else {
            eprintln!("❌ {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}

///Either gets the default value for input or the provided one (preferred)
fn evaluate_input(input: &CommandInputParameter, input_values: &Option<HashMap<String, DefaultValue>>) -> Result<String, Box<dyn Error>> {
    if let Some(ref values) = input_values {
        if let Some(value) = values.get(&input.id) {
            if !value.has_matching_type(&input.type_) {
                //change handling accordingly in utils on main branch!
                eprintln!("CWLType is not matching input type");
                Err("CWLType is not matching input type")?;
            }
            return Ok(value.as_value_string());
        }
    } else if let Some(default_) = &input.default {
        return Ok(default_.as_value_string());
    } else {
        eprintln!("You did not include a value for {}", input.id);
        Err(format!("You did not include a value for {}", input.id).as_str())?;
    }
    Err(format!("Could not evaluate input: {}", input.id))?
}

fn evaluate_outputs(tool_outputs: &Vec<CommandOutputParameter>, initial_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    //copy back requested output
    let mut outputs: HashMap<&String, OutputItem> = HashMap::new();
    for output in tool_outputs {
        if output.type_ == CWLType::File {
            if let Some(binding) = &output.output_binding {
                let path = &initial_dir.join(&binding.glob);
                fs::copy(&binding.glob, path).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", &binding.glob, path, e))?;
                eprintln!("📜 Wrote output file: {:?}", path);
                outputs.insert(&output.id, OutputItem::OutputFile(get_file_metadata(path.into(), output.format.clone())));
            }
        } else if output.type_ == CWLType::Directory {
            if let Some(binding) = &output.output_binding {
                let dir = if &binding.glob != "." {
                    initial_dir
                } else {
                    let working_dir = env::current_dir()?;
                    let raw_basename = working_dir.file_name().unwrap().to_string_lossy();
                    let glob_name = if raw_basename.starts_with(".") { raw_basename[1..].to_owned() } else { raw_basename.into_owned() };
                    &initial_dir.join(&glob_name)
                };
                fs::create_dir_all(dir)?;

                let mut out_dir = OutputDirectory {
                    location: format!("file://{}", dir.display()),
                    basename: dir.file_name().unwrap().to_string_lossy().into_owned(),
                    class: "Directory".to_string(),
                    listing: vec![],
                    path: dir.to_string_lossy().into_owned(),
                };
                let files = copy_dir(&binding.glob, dir.to_str().unwrap()).map_err(|e| format!("Failed to copy: {}", e))?;
                for file in files {
                    out_dir.listing.push(get_file_metadata(file.into(), None));
                }
                outputs.insert(&output.id, OutputItem::OutputDirectory(out_dir));
            }
        }
    }
    //print output metadata
    let json = serde_json::to_string_pretty(&outputs)?;
    println!("{}", json);
    Ok(())
}

fn get_file_metadata(path: PathBuf, format: Option<String>) -> OutputFile {
    let p_str = path.to_str().unwrap();
    let basename = get_filename_without_extension(p_str).unwrap();
    let size = get_file_size(&path).unwrap_or_else(|_| panic!("Could not get filesize: {:?}", path));
    let checksum = format!("sha1${}", get_file_checksum(&path).unwrap_or_else(|_| panic!("Could not get checksum: {:?}", path)));

    OutputFile {
        location: format!("file://{}", path.display()),
        basename,
        class: "File".to_string(),
        checksum,
        size,
        path: path.to_string_lossy().into_owned(),
        format,
    }
}

fn stage_needed_files(tool: &CommandLineTool, into_dir: &TempDir, input_values: &Option<HashMap<String, DefaultValue>>, tool_path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut files = vec![];
    //stage initial workdir
    if let Some(req) = &tool.requirements {
        for item in req {
            if let Requirement::InitialWorkDirRequirement(iwdr) = item {
                for listing in &iwdr.listing {
                    let path = into_dir.path().join(tool_path).join(&listing.entryname);
                    let path_str = &path.to_string_lossy();
                    files.push(path_str.clone().into_owned());
                    match &listing.entry {
                        Entry::Source(src) => {
                            create_and_write_file(path_str, src).map_err(|e| format!("Failed to create and write file {:?}: {}", path, e))?;
                        }
                        Entry::Include(f) => {
                            copy_file(&f.include, path_str).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", f.include, path, e))?;
                        }
                    }
                }
            }
        }
    }

    //stage inputs
    for input in &tool.inputs {
        //TODO: Handle directories
        if input.type_ == CWLType::File {
            let in_file = evaluate_input(input, input_values)?;
            let file = in_file.trim_start_matches("../");
            let path = into_dir.path().join(tool_path).join(file);
            let path_str = &path.to_string_lossy();
            copy_file(&in_file, path_str).map_err(|e| format!("Failed to copy file from {:?} to {:?}: {}", file, path, e))?;
            files.push(path_str.clone().into_owned());
        }
    }

    Ok(files)
}
