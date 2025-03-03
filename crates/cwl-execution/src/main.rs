use clap::Parser;
use cwl::{load_tool, types::DefaultValue};
use cwl_execution::{execute, CommandError, ExitCode};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::exit,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = ExecutionParameters::parse();
    let cwl = args.file;
    let job = args.inputs;
    let outdir = args.out_dir;
    //let path = Path::new("/home/ubuntu/cwl-v1.2");
    //let cwl = path.join("tests/any-type-compat.cwl");
    //let job = path.join("tests/any-type-job.json");
    //let outdir: Option<String> = None;

    let job_contents = fs::read_to_string(&job)?;
    let inputs: HashMap<String, DefaultValue> = serde_yaml::from_str(&job_contents)?;
    match execute(cwl, inputs, outdir) {
        Ok(outputs) => {
            let json = serde_json::to_string_pretty(&outputs)?;
            println!("{json}");
        }
        Err(e) => {
            if let Some(cmd_err) = e.downcast_ref::<CommandError>() {
                exit(cmd_err.exit_code());
            } else {
                exit(1);
            }
        }
    }
    Ok(())
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
