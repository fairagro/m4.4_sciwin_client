use anyhow::{anyhow, Context, Result};
use cwl_core::{StringOrDocument, Workflow};
use serde_json::Value as JsonValue;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use zip::ZipArchive;

/// Find the main CWL file inside a RO-Crate folder
pub fn find_cwl_in_rocrate(crate_root: &Path) -> Result<PathBuf> {
    let meta_path = crate_root.join("ro-crate-metadata.json");
    let json_str = fs::read_to_string(&meta_path).with_context(|| format!("Failed to read RO-Crate metadata: {meta_path:?}"))?;
    let json_value: JsonValue = serde_json::from_str(&json_str).context("Failed to parse RO-Crate metadata JSON")?;
    let graph = json_value
        .get("@graph")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("RO-Crate metadata missing @graph"))?;
    let dataset_node = graph
        .iter()
        .find(|node| node.get("@id").and_then(|id| id.as_str()) == Some("./"))
        .ok_or_else(|| anyhow!("No Dataset node with @id './' found"))?;
    let main_entity_id = dataset_node
        .get("mainEntity")
        .and_then(|me| me.get("@id"))
        .and_then(|id| id.as_str())
        .ok_or_else(|| anyhow!("Dataset node './' missing mainEntity.@id"))?;
    let cwl_path = crate_root.join(main_entity_id);
    if !cwl_path.exists() {
        return Err(anyhow!("CWL file not found at {cwl_path:?}"));
    }

    Ok(cwl_path)
}

/// Verify that all step files in Workflow CWL exist, sometimes they are missing for rocrates on workflowhub (only workflow cwl is present)
pub fn verify_cwl_references(cwl_path: &Path) -> Result<bool> {
    let content = fs::read_to_string(cwl_path).with_context(|| format!("Failed to read {cwl_path:?}"))?;
    let workflow: Workflow = serde_yaml::from_str(&content).with_context(|| format!("Invalid CWL structure {cwl_path:?}"))?;
    let parent = cwl_path.parent().unwrap_or_else(|| Path::new("."));
    let mut all_exist = true;
    for step in &workflow.steps {
        if let StringOrDocument::String(run_str) = &step.run {
            let run_path = parent.join(run_str);
            if !run_path.exists() {
                eprintln!("⚠ Missing referenced run file: {run_path:?}");
                all_exist = false;
            }
        }
    }

    Ok(all_exist)
}

// clone repo either from isBasedOn in ro-crate-metadata.json or from s:codeRepository in CWL file
pub fn clone_from_rocrate_or_cwl(ro_crate_meta: &Path, cwl_path: &Path) -> Result<(TempDir, Option<PathBuf>, Option<PathBuf>)> {
    let meta_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(ro_crate_meta).with_context(|| format!("Failed to read RO-Crate metadata at {:?}", ro_crate_meta))?)
            .context("Failed to parse RO-Crate metadata JSON")?;
    let graph = meta_json
        .get("@graph")
        .and_then(|v| v.as_array())
        .context("RO-Crate metadata missing @graph")?;
    let git_url = graph
        .iter()
        .find(|item| item.get("@id").and_then(|v| v.as_str()) == Some("./"))
        .and_then(|root| root.get("isBasedOn"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            if cwl_path.exists() {
                fs::read_to_string(cwl_path).ok().and_then(|content| {
                    content
                        .lines()
                        .find(|l| l.trim_start().starts_with("s:codeRepository:"))
                        .and_then(|line| line.split_once(':'))
                        .map(|(_, v)| v.trim().to_string())
                })
            } else {
                None
            }
        })
        .context("No repository URL found in RO-Crate or CWL")?;

    eprintln!("📦 Cloning repository from {git_url}...");

    let temp = tempfile::tempdir().context("Failed to create temporary directory")?;
    let repo_path = temp.path();

    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", &git_url, repo_path.to_str().unwrap()])
        .status()
        .with_context(|| format!("Failed to execute git clone from {git_url}"))?;

    if !status.success() {
        anyhow::bail!("❌ Git clone failed from {git_url}");
    }
    let git_dir = repo_path.join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(&git_dir).context("Failed to remove .git directory")?;
    }
    let (cwl_candidate, inputs_yaml_candidate) = find_cwl_and_inputs(repo_path, cwl_path);

    Ok((temp, cwl_candidate, inputs_yaml_candidate))
}

/// find CWL and inputs.yaml files in a cloned repository
pub fn find_cwl_and_inputs(repo_path: &Path, cwl_path: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    let cwl_file_name = cwl_path.file_name().and_then(|s| s.to_str());
    let mut cwl_candidate: Option<PathBuf> = None;
    let mut inputs_yaml_candidate: Option<PathBuf> = None;
    for entry in walkdir::WalkDir::new(repo_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let (Some(cwl_name), Some(name)) = (cwl_file_name, path.file_name().and_then(|s| s.to_str())) {
                if name == cwl_name {
                    cwl_candidate = Some(path.to_path_buf());
                }
            }
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name == "inputs.yaml" || name == "inputs.yml" {
                    inputs_yaml_candidate = Some(path.to_path_buf());
                }
            }
            if cwl_candidate.is_some() && inputs_yaml_candidate.is_some() {
                break;
            }
        }
    }
    (cwl_candidate, inputs_yaml_candidate)
}

/// Unzip a RO-Crate ZIP into a directory
pub fn unzip_rocrate(zip_path: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let file = fs::File::open(zip_path).with_context(|| format!("Failed to open ZIP file: {}", zip_path.display()))?;
    let mut archive = ZipArchive::new(file)?;
    archive
        .extract(dest_dir)
        .with_context(|| format!("Failed to extract ZIP file: {}", zip_path.display()))?;
    let mut crate_root = dest_dir.to_path_buf();
    let entries = fs::read_dir(dest_dir).with_context(|| format!("Failed to read directory: {dest_dir:?}"))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("ro-crate-metadata.json").exists() {
            crate_root = path;
            break;
        }
    }
    if !crate_root.join("ro-crate-metadata.json").exists() {
        anyhow::bail!("RO-Crate metadata not found in extracted ZIP: {dest_dir:?}");
    }

    Ok(crate_root)
}
