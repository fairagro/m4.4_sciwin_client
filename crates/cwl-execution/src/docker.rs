use crate::environment::RuntimeEnvironment;
use commonwl::{requirements::DockerRequirement, Entry, StringOrNumber};
use rand::Rng;
use rand::distr::Alphanumeric;
use std::cell::RefCell;
use std::fmt::Display;
use std::process::Command as SystemCommand;
use std::{fs, path::MAIN_SEPARATOR_STR, process::Command};

pub fn is_docker_installed() -> bool {
    let engine = container_engine().to_string();
    let output = Command::new(engine).arg("--version").output();

    matches!(output, Ok(output) if output.status.success())
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

pub(crate) fn build_docker_command(command: &mut SystemCommand, docker: &DockerRequirement, runtime: &RuntimeEnvironment) -> SystemCommand {
    let container_engine = container_engine().to_string();

    let docker_image = if let Some(pull) = &docker.docker_pull {
        pull
    } else if let (Some(docker_file), Some(docker_image_id)) = (&docker.docker_file, &docker.docker_image_id) {
        let path = match docker_file {
            Entry::Include(include) => include.include.clone(),
            Entry::Source(src) => {
                let path = format!("{}/Dockerfile", runtime.runtime["tmpdir"]);
                fs::write(&path, src).unwrap();
                path
            }
        };
        let path = path.trim_start_matches(&("..".to_owned() + MAIN_SEPARATOR_STR)).to_string();

        let mut build = SystemCommand::new(&container_engine);
        build.args(["build", "-f", &path, "-t", docker_image_id, "."]);
        let output = build.output().expect("Could not build container!");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        docker_image_id
    } else {
        unreachable!()
    };
    let mut docker_command = SystemCommand::new(&container_engine);

    //create workdir vars
    let workdir = if let Some(docker_output_directory) = &docker.docker_output_directory {
        docker_output_directory
    } else {
        &format!("/{}", rand::rng().sample_iter(&Alphanumeric).take(5).map(char::from).collect::<String>())
    };
    let outdir = &runtime.runtime["outdir"];
    let tmpdir = &runtime.runtime["tmpdir"];

    let workdir_mount = format!("--mount=type=bind,source={outdir},target={workdir}");
    let tmpdir_mount = format!("--mount=type=bind,source={tmpdir},target=/tmp");
    let workdir_arg = format!("--workdir={}", &workdir);
    docker_command.args(["run", "-i", &workdir_mount, &tmpdir_mount, &workdir_arg, "--rm"]);
    #[cfg(unix)]
    {
        docker_command.arg(get_user_flag());
    }
    //add all environment vars
    docker_command.arg(format!("--env=HOME={}", &workdir));
    docker_command.arg("--env=TMPDIR=/tmp");
    for (key, val) in command.get_envs().skip_while(|(key, _)| *key == "HOME" || *key == "TMPDIR") {
        docker_command.arg(format!("--env={}={}", key.to_string_lossy(), val.unwrap().to_string_lossy()));
    }

    if let Some(StringOrNumber::Integer(i)) = runtime.runtime.get("network") {
        if *i != 1 {
            docker_command.arg("--net=none");
        }
        //net enabled if i == 1 = not append arg
    } else {
        docker_command.arg("--net=none");
    }

    docker_command.arg(docker_image);
    docker_command.arg(command.get_program());

    //rewrite home dir
    let args = command
        .get_args()
        .map(|arg| {
            arg.to_string_lossy()
                .into_owned()
                .replace(&runtime.runtime["outdir"].to_string(), workdir)
                .replace("\\", "/")
        })
        .collect::<Vec<_>>();
    docker_command.args(args);

    docker_command
}

#[cfg(unix)]
fn get_user_flag() -> String {
    use nix::unistd::{getgid, getuid};
    format!("--user={}:{}", getuid().as_raw(), getgid().as_raw())
}
