use clap::Parser;
use cwl::types::DefaultValue;
use cwl_execution::{execute, CommandError, ExitCode};
use std::{
    collections::HashMap,
    env::{self},
    fs,
    path::{Path, PathBuf},
    process::exit,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let args = ExecutionParameters::parse();
    //let cwl = args.file;
    //let job = args.inputs;
    //let outdir = args.out_dir;
    let path = Path::new("/home/ubuntu/cwl-v1.2");
    let cwl = path.join("tests/dir4.cwl");
    let job = path.join("tests/dir4-job.yml");
    let outdir: Option<String> = None;

    let job_contents = fs::read_to_string(&job)?;

    let path = job.parent().unwrap();
    let inputs: HashMap<String, DefaultValue> = serde_yaml::from_str(&job_contents)?;
    let inputs = inputs
        .iter()
        .map(|(k, v)| Ok((k.clone(), load_input(v.clone(), path)?)))
        .collect::<Result<HashMap<_, _>, Box<dyn std::error::Error>>>()?;

    if let Err(e) = execute(cwl, inputs, outdir) {
        eprintln!("{e}");
        if let Some(cmd_err) = e.downcast_ref::<CommandError>() {
            exit(cmd_err.exit_code());
        } else {
            exit(1);
        }
    }

    Ok(())
}

fn load_input(input: DefaultValue, relative_to: impl AsRef<Path>) -> Result<DefaultValue, Box<dyn std::error::Error>> {
    let current = env::current_dir()?;
    env::set_current_dir(relative_to)?;
    let result = input.load();
    env::set_current_dir(current)?;
    Ok(result)
}

#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
struct ExecutionParameters {
    pub file: PathBuf,
    pub inputs: PathBuf,
    #[arg(long = "outdir", help = "A path to output resulting files to")]
    pub out_dir: Option<String>,
    #[arg(long = "quiet", help = "Runner does not print to stdout")]
    pub is_quiet: bool,
}
