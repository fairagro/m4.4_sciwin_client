use commonwl::{StringOrDocument, load_doc, load_workflow, prelude::*};
use petgraph::{graph::NodeIndex, prelude::StableDiGraph};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone)]
pub enum Node {
    Step(CWLDocument),
    Input(CommandInputParameter), //WorkflowInputParameter
    Output(WorkflowOutputParameter),
}

impl Node {
    pub fn id(&self) -> String {
        match &self {
            Self::Step(doc) => doc.id.clone().unwrap().clone(),
            Self::Input(input) => input.id.clone(),
            Self::Output(output) => output.id.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub source_port: String,
    pub target_port: String,
    pub data_type: CWLType,
}

pub type WorkflowGraph = StableDiGraph<Node, Edge>;

pub fn load_workflow_graph(path: impl AsRef<Path>) -> anyhow::Result<WorkflowGraph> {
    let workflow = load_workflow(path.as_ref()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let wgb = WorkflowGraphBuilder::from_workflow(&workflow, path)?;
    Ok(wgb.graph)
}

#[derive(Default)]
struct WorkflowGraphBuilder {
    pub graph: WorkflowGraph,
    node_map: HashMap<String, NodeIndex>,
}

impl WorkflowGraphBuilder {
    fn from_workflow(workflow: &Workflow, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut builder = Self::default();
        builder.load_workflow(workflow, path)?;
        Ok(builder)
    }

    fn load_workflow(&mut self, workflow: &Workflow, path: impl AsRef<Path>) -> anyhow::Result<()> {
        for input in &workflow.inputs {
            let node_id = self.graph.add_node(Node::Input(input.clone()));
            self.node_map.insert(input.id.clone(), node_id);
        }

        for output in &workflow.outputs {
            let node_id = self.graph.add_node(Node::Output(output.clone()));
            self.node_map.insert(output.id.clone(), node_id);
        }
        let step_ids = workflow.sort_steps().map_err(|e| anyhow::anyhow!("{e}"))?;

        for step_id in step_ids {
            let step = workflow.get_step(&step_id).unwrap();
            let StringOrDocument::String(str) = &step.run else {
                anyhow::bail!("Inline Document not supported")
            };

            let step_file = path.as_ref().parent().unwrap().join(str);
            let mut doc = load_doc(&step_file).map_err(|e| anyhow::anyhow!("{e}"))?;
            if doc.id.is_none() {
                doc.id = Some(step_file.file_name().unwrap().to_string_lossy().to_string());
            }

            let node_id = self.graph.add_node(Node::Step(doc.clone()));
            self.node_map.insert(step.id.clone(), node_id);

            for wsip in &step.in_ {
                let source = wsip.source.as_ref().unwrap(); //TODO!
                let (source, source_port) = source.split_once('/').unwrap_or((source.as_str(), ""));
                let type_ = doc.inputs.iter().find(|i| i.id == wsip.id).unwrap().type_.clone(); //TODO!

                self.connect_edge(source, &step.id, source_port, &wsip.id, type_)?;
            }
        }

        Ok(())
    }

    fn connect_edge(&mut self, source: &str, target: &str, source_port: &str, target_port: &str, type_: CWLType) -> anyhow::Result<()> {
        let source_idx = self.node_map.get(source).unwrap(); //TODO!
        let target_idx = self.node_map.get(target).unwrap(); //TODO!

        self.graph.add_edge(
            *source_idx,
            *target_idx,
            Edge {
                source_port: source_port.to_string(),
                target_port: target_port.to_string(),
                data_type: type_,
            },
        );

        Ok(())
    }
}
