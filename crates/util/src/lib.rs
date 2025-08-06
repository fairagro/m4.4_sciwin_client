use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, Command},
};

pub fn is_cwl_file(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("cwl"))
}

pub fn report_console_output(process: &mut Child) {
    if let Some(stdout) = process.stdout.as_mut() {
        let lines = BufReader::new(stdout).lines();
        for line in lines {
            eprintln!("{line:?}");
        }
    }

    if let Some(stderr) = process.stderr.as_mut() {
        let lines = BufReader::new(stderr).lines();
        for line in lines {
            eprintln!("{line:?}");
        }
    }
}

pub fn is_docker_installed() -> bool {
    let output = Command::new("docker").arg("--version").output();

    matches!(output, Ok(output) if output.status.success())
}

pub fn is_ci_process() -> bool {
    std::env::var("CI").is_ok()
}
