use crate::{
    edge::VisualEdge,
    graph::{WorkflowGraph, load_workflow_graph},
};
use commonwl::{CWLDocument, Workflow, format::format_cwl, load_workflow};
use petgraph::graph::{EdgeIndex, NodeIndex};
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
    //...add node?

    pub fn add_connection(&mut self, from_id: NodeIndex, from_slot_id: &str, to_id: NodeIndex, to_slot_id: &str) -> anyhow::Result<()> {
        let from_node = &self.graph[from_id];
        let to_node = &self.graph[to_id];

        let from_name = from_node.instance.id().trim_end_matches(".cwl").to_string();
        let to_name = to_node.instance.id().trim_end_matches(".cwl").to_string();

        let from_filename = &from_node.path;
        let to_filename = &to_node.path;

        if self.workflow.has_step(&from_name)
            && self.workflow.has_step(&to_name)
            && let Some(from_filename) = from_filename
            && let Some(to_filename) = to_filename
        {
            s4n_core::workflow::add_workflow_step_connection(
                &mut self.workflow,
                from_filename,
                &from_name,
                from_slot_id,
                to_filename,
                &to_name,
                to_slot_id,
            )?;
        } else if !self.workflow.has_step(&from_name)
            && let Some(to_filename) = to_filename
        {
            // from name is input
            s4n_core::workflow::add_workflow_input_connection(&mut self.workflow, from_slot_id, to_filename, &to_name, to_slot_id)?;
        } else if !self.workflow.has_step(&to_name)
            && let Some(from_filename) = from_filename
        {
            // from to name is output
            s4n_core::workflow::add_workflow_output_connection(&mut self.workflow, &from_name, from_slot_id, from_filename, &to_name)?;
        } else {
            anyhow::bail!("undefined connection command")
        }

        let cwl_type = &from_node.outputs.iter().find(|o| o.id == from_slot_id).unwrap().type_;

        self.graph.add_edge(
            from_id,
            to_id,
            VisualEdge {
                source_port: from_slot_id.to_owned(),
                target_port: to_slot_id.to_owned(),
                data_type: cwl_type.clone(),
            },
        );

        self.save()
    }

    pub fn remove_connection(&mut self, index: EdgeIndex) -> anyhow::Result<()> {
        let edge = &self.graph[index];

        let (from_node_id, to_node_id) = self.graph.edge_endpoints(index).unwrap();
        let from_node_instance = &self.graph[from_node_id].instance;
        let to_node_instance = &self.graph[to_node_id].instance;

        let from_node = from_node_instance.id().trim_end_matches(".cwl").to_string();
        let to_node = to_node_instance.id().trim_end_matches(".cwl").to_string();

        let from_slot = edge.source_port.clone();
        let to_slot = edge.target_port.clone();

        if self.workflow.has_step(&from_node) && self.workflow.has_step(&to_node) {
            s4n_core::workflow::remove_workflow_step_connection(&mut self.workflow, &to_node, &to_slot)?
        } else if !self.workflow.has_step(&from_node) {
            s4n_core::workflow::remove_workflow_input_connection(&mut self.workflow, &from_node, &to_node, &to_slot, false)?
        } else if !self.workflow.has_step(&to_node) {
            s4n_core::workflow::remove_workflow_output_connection(&mut self.workflow, &from_node, &from_slot, &to_node, false)?
        } else {
            anyhow::bail!("undefined disconnection command")
        }

        self.graph.remove_edge(index);
        self.save()
    }

    fn save(&mut self) -> anyhow::Result<()> {
        let mut yaml = serde_yaml::to_string(&self.workflow)?;

        yaml = format_cwl(&yaml).map_err(|e| anyhow::anyhow!("Could not format yaml: {e}"))?;
        let mut file = fs::File::create(&self.path)?;
        file.write_all(yaml.as_bytes())?;

        Ok(())
    }
}
