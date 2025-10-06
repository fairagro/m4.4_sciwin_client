use colored::Colorize;
use commonwl::{prelude::*, requirements::WorkDirItem};
use log::{info, warn};
use remote_execution::parser::WorkflowJson;
use std::collections::HashMap;
use std::process::{Command as SystemCommand, Stdio};
use std::{env, fs, path::Path};
use util::{handle_process, is_docker_installed};

use crate::parser::SCRIPT_EXECUTORS;

/// Performs some compatibility adjustments on workflow json for the exeuction using REANA.
pub fn compatibility_adjustments(workflow_json: &mut WorkflowJson) -> anyhow::Result<()> {
    for item in &mut workflow_json.workflow.specification.graph {
        if let CWLDocument::CommandLineTool(tool) = item {
            adjust_basecommand(tool)?;
            // if tool has a docker pull not necessary to inject a docker pull?
            if !has_docker_pull(tool) {
                publish_docker_ephemeral(tool)?;
                if !has_docker_pull(tool) {
                    inject_docker_pull(tool)?;
                }
            }
        }
    }
    Ok(())
}

///checks if tool has a docker pull already
fn has_docker_pull(tool: &CommandLineTool) -> bool {
    tool.requirements.iter().any(|req| {
        if let Requirement::DockerRequirement(docker_req) = req {
            docker_req.docker_pull.is_some()
        } else {
            false
        }
    })
}

/// adjusts path as a workaround for <https://github.com/fairagro/m4.4_sciwin_client/issues/114>
fn adjust_basecommand(tool: &mut CommandLineTool) -> anyhow::Result<()> {
    let mut changed = false;
    let mut command_vec = match &tool.base_command {
        Command::Multiple(vec) => vec.clone(),
        _ => return Ok(()),
    };
    if let Some(iwdr) = tool.get_requirement_mut::<InitialWorkDirRequirement>() {
        for item in &mut iwdr.listing {
            if let WorkDirItem::Dirent(dirent) = item
                && let Some(entryname) = &mut dirent.entryname
                && command_vec.contains(entryname)
            {
                //check whether entryname has a path attached to script item and rewrite command and entryname if so
                let path = Path::new(entryname);
                if path.parent().is_some() {
                    let pos = command_vec
                        .iter()
                        .position(|c| c == entryname)
                        .ok_or(anyhow::anyhow!("Failed to find command item {entryname}"))?;
                    *entryname = path
                        .file_name()
                        .ok_or(anyhow::anyhow!("Failed to get filename from {path:?}"))?
                        .to_string_lossy()
                        .into_owned();
                    command_vec[pos] = (*entryname).to_string();
                    changed = true;
                }
            }
        }
    }
    if changed {
        info!(
            "Basecommand of {} was modified to `{}` (see https://github.com/fairagro/m4.4_sciwin_client/issues/114).",
            tool.id.clone().unwrap().green().bold(),
            command_vec.join(" ")
        );
        tool.base_command = Command::Multiple(command_vec);
    }
    Ok(())
}

/// adjusts dockerrequirement as a workaround for <https://github.com/fairagro/m4.4_sciwin_client/issues/119>
fn publish_docker_ephemeral(tool: &mut CommandLineTool) -> anyhow::Result<()> {
    let id = tool.id.clone().unwrap();
    if let Some(dr) = tool.get_requirement_mut::<DockerRequirement>()
        && let Some(dockerfile) = &mut dr.docker_file
    {
        warn!("Tool {id} depends on Dockerfile, which not supported by REANA!");
        if !is_docker_installed() {
            return Ok(());
        }
        info!("Trying to use a workaround for Dockerfile in Tool {}...", id.green().bold());
        //we build the image and send it to ttl.sh
        let image_name = uuid::Uuid::new_v4().to_string();
        let tag = format!("ttl.sh/{image_name}:1h");
        //write dockerfile to temp dir
        let file_content = match dockerfile {
            commonwl::Entry::Source(src) => src.clone(),
            commonwl::Entry::Include(include) => fs::read_to_string(include.include.clone())?,
        };
        let filenname = env::temp_dir().join(&image_name);
        fs::write(&filenname, file_content)?;

        //build docker file
        let mut process = SystemCommand::new("docker")
            .arg("build")
            .arg("-t")
            .arg(&tag)
            .arg("-f")
            .arg(filenname)
            .arg(".")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        handle_process(&mut process, 0).map_err(|e| anyhow::anyhow!("{e}"))?;
        process.wait()?;
        eprintln!("✔️  Successfully built Docker image in Tool {}", id.green().bold());

        //push
        let mut process = SystemCommand::new("docker")
            .arg("push")
            .arg(&tag)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        handle_process(&mut process, 0).map_err(|e| anyhow::anyhow!("{e}"))?;
        process.wait()?;
        eprintln!(
            "✔️  Docker image was published at {tag} and is available for 1 hour in Tool {}",
            id.green().bold()
        );

        //set docker pull and remove dockerfile
        dr.docker_pull = Some(tag);
        dr.docker_file = None;
        dr.docker_image_id = None;
    }
    Ok(())
}

/// check whether "python", "Rscript", ... is used and inject Docker image
/// We can not rely on the REANA server has those tools installed
fn inject_docker_pull(tool: &mut CommandLineTool) -> anyhow::Result<()> {
    let id = tool.id.clone().unwrap();

    let command_vec = match &tool.base_command {
        Command::Multiple(vec) => vec.clone(),
        _ => return Ok(()),
    };

    let default_images = HashMap::from([("python", "python"), ("Rscript", "r-base"), ("node", "node")]);

    if SCRIPT_EXECUTORS.contains(&&*command_vec[0]) && tool.get_requirement::<DockerRequirement>().is_some() {
        //is script executor but does not use containerization
        warn!(
            "Tool {} is using {} and does not use a proper container",
            id.green().bold(),
            command_vec[0].bold()
        );

        if let Some(container) = default_images.get(&&*command_vec[0]) {
            tool.requirements
                .push(Requirement::DockerRequirement(DockerRequirement::from_pull(container)));

            eprintln!("✔️  Added container {} to tool {}", container.bold(), id.green().bold());
        }
    }

    Ok(())
}
