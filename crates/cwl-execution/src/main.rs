use std::{fs, path::Path};

use cwl_execution::execute;

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let path = Path::new("/home/ubuntu/cwl-v1.2/tests");

    let cwl = path.join("wc2-tool.cwl");
    let job = path.join("wc-job.json");

    let job_contents = fs::read_to_string(job)?;
    let inputs = serde_yaml::from_str(&job_contents)?;

    let outdir: Option<&Path> = None;

    execute(cwl, inputs, outdir)?;

    Ok(())
}
