use cwl::types::DefaultValue;
use cwl_execution::execute;
use std::{collections::HashMap, env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("/home/ubuntu/cwl-v1.2/tests");

    let cwl = path.join("capture-dirs.cwl");
    let job = path.join("dir-job.yml");

    let job_contents = fs::read_to_string(job)?;
    let inputs: HashMap<String, DefaultValue> = serde_yaml::from_str(&job_contents)?;
    let inputs = inputs
        .iter()
        .map(|(k, v)| Ok((k.clone(), load_input(v.clone(), path)?)))
        .collect::<Result<HashMap<_, _>, Box<dyn std::error::Error>>>()?;

    let outdir: Option<&Path> = None;

    execute(cwl, inputs, outdir)?;

    Ok(())
}

fn load_input(input: DefaultValue, relative_to: impl AsRef<Path>) -> Result<DefaultValue, Box<dyn std::error::Error>> {
    let current = env::current_dir()?;
    env::set_current_dir(relative_to)?;
    let result = input.load();
    env::set_current_dir(current)?;
    Ok(result)
}