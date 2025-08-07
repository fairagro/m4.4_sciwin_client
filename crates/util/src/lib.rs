use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, Command},
};

pub fn is_cwl_file(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("cwl"))
}

pub struct Output {
    pub stdout: String,
    pub stderr: String,
}

pub fn report_console_output(process: &mut Child) -> Result<Output, Box<dyn std::error::Error>> {
    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();

    if let Some(stdout) = process.stdout.as_mut() {
        let lines = BufReader::new(stdout).lines();
        for line in lines {
            let line = line?;
            eprintln!("{line:?}");
            stdout_buf.push_str(&line);
            stdout_buf.push('\n');
        }
    }
    if let Some(stderr) = process.stderr.as_mut() {
        let lines = BufReader::new(stderr).lines();
        for line in lines {
            let line = line?;
            eprintln!("{line:?}");
            stderr_buf.push_str(&line);
            stderr_buf.push('\n');
        }
    }

    Ok(Output {
        stdout: stdout_buf,
        stderr: stderr_buf,
    })
}

pub fn is_docker_installed() -> bool {
    let output = Command::new("docker").arg("--version").output();

    matches!(output, Ok(output) if output.status.success())
}
