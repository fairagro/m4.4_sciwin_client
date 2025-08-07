use colored::Colorize;
use commonwl::{prelude::*, requirements::WorkDirItem};
use remote_execution::parser::WorkflowJson;
use std::process::Command as SystemCommand;
use std::{env, fs, path::Path};
use util::{is_docker_installed, report_console_output};

/// Performs some compatibility adjustments on workflow json for the exeuction using REANA.
pub fn compatibility_adjustments(workflow_json: &mut WorkflowJson) -> anyhow::Result<()> {
    for item in &mut workflow_json.workflow.specification.graph {
        if let CWLDocument::CommandLineTool(tool) = item {
            adjust_basecommand(tool)?;
            publish_docker_ephemeral(tool)?;
        }
    }
    Ok(())
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
        eprintln!(
            "ℹ️  Basecommand of {} was modified to `{}` (see https://github.com/fairagro/m4.4_sciwin_client/issues/114).",
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
    if let Some(dr) = tool.get_requirement_mut::<DockerRequirement>() {
        if let Some(dockerfile) = &mut dr.docker_file {
            eprintln!("ℹ️  Tool {id} depends on Dockerfile, which not supported by REANA!");
            if !is_docker_installed() {
                return Ok(());
            }
            eprintln!("🌶️  Trying to use a workaround for Dockerfile in Tool {}...", id.green().bold());
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
                .spawn()?;
            report_console_output(&mut process);
            process.wait()?;
            eprintln!("✔️  Successfully built Docker image in Tool {}", id.green().bold());

            //push
            let mut process = SystemCommand::new("docker").arg("push").arg(&tag).spawn()?;
            report_console_output(&mut process);
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
    }
    Ok(())
}
