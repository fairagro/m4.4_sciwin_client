use crate::graph::{WorkflowGraph, load_workflow_graph};
use commonwl::{CWLDocument, Workflow, format::format_cwl, load_workflow};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// Viewmodel implementation for Workflow
#[derive(Default, Debug, Clone)]
pub struct VisualWorkflow {
    pub path: PathBuf,
    pub workflow: Workflow,
    pub graph: WorkflowGraph,
}

impl VisualWorkflow {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let workflow = load_workflow(path).map_err(|e| anyhow::anyhow!("{e}"))?;
        let graph = load_workflow_graph(&workflow, path)?;
        Ok(Self {
            path: path.to_path_buf(),
            workflow,
            graph,
        })
    }
}

impl VisualWorkflow {
    pub fn add_new_step_if_not_exists(&mut self, name: &str, path: &str, doc: &CWLDocument) {
        s4n_core::workflow::add_workflow_step(&mut self.workflow, name, path, doc);
    }
    //...

    pub fn remove_connection(&mut self, from_name: &str, from_id: &str, to_name: &str, to_id: &str) -> anyhow::Result<()> {
        if self.workflow.has_step(from_name) && self.workflow.has_step(to_name) {
            s4n_core::workflow::remove_workflow_step_connection(&mut self.workflow, to_name, to_id)?
        } else if !self.workflow.has_step(from_name) {
            s4n_core::workflow::remove_workflow_input_connection(&mut self.workflow, from_name, to_name, to_id)?
        } else if !self.workflow.has_step(to_name) {
            s4n_core::workflow::remove_workflow_output_connection(&mut self.workflow, from_name, from_id, to_name)?
        } else {
            anyhow::bail!("undefined disconnection command")
        }

        // save
        let mut yaml = serde_yaml::to_string(&self.workflow)?;

        yaml = format_cwl(&yaml).map_err(|e| anyhow::anyhow!("Could not format yaml: {e}"))?;
        let mut file = fs::File::create(&self.path)?;
        file.write_all(yaml.as_bytes())?;
        Ok(())
    }
}
