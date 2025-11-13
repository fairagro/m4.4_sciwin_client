use crate::graph::{WorkflowGraph, load_workflow_graph};
use commonwl::{Workflow, load_workflow};
use std::path::{Path, PathBuf};

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
