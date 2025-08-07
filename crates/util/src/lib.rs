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

    if let Some(stdout) = process.stdout.take() {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            eprint!("{line}");
            stdout_buf.push_str(&line);
            line.clear();
        }
    }

    if let Some(stderr) = process.stderr.take() {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            eprint!("{line}");
            stderr_buf.push_str(&line);
            line.clear();
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
