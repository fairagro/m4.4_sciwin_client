use crate::{
    cwl::{
        clt::{CommandInputParameter, CommandOutputParameter, DefaultValue},
        types::{CWLType, OutputDirectory, OutputFile, OutputItem},
    },
    io::{copy_dir, get_file_checksum, get_file_size, get_filename_without_extension},
};
use std::{collections::HashMap, env, error::Error, fs, path::PathBuf};

///Either gets the default value for input or the provided one (preferred)
pub fn evaluate_input_as_string(input: &CommandInputParameter, input_values: &Option<HashMap<String, DefaultValue>>) -> Result<String, Box<dyn Error>> {
    Ok(evaluate_input(input, input_values)?.as_value_string())
}

///Either gets the default value for input or the provided one (preferred)
pub fn evaluate_input(input: &CommandInputParameter, input_values: &Option<HashMap<String, DefaultValue>>) -> Result<DefaultValue, Box<dyn Error>> {
    if let Some(ref values) = input_values {
        if let Some(value) = values.get(&input.id) {
            if !value.has_matching_type(&input.type_) {
                //change handling accordingly in utils on main branch!
                eprintln!("CWLType is not matching input type");
                Err("CWLType is not matching input type")?;
            }
            return Ok(value.clone());
        }
    } else if let Some(default_) = &input.default {
        return Ok(default_.clone());
    } else {
        eprintln!("You did not include a value for {}", input.id);
        Err(format!("You did not include a value for {}", input.id).as_str())?;
    }
    Err(format!("Could not evaluate input: {}", input.id))?
}

///Copies back requested outputs and writes to commandline
pub fn evaluate_outputs(tool_outputs: &Vec<CommandOutputParameter>, initial_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
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
                    let glob_name = if let Some(stripped) = raw_basename.strip_prefix(".") {
                        stripped.to_owned()
                    } else {
                        raw_basename.into_owned()
                    };
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
