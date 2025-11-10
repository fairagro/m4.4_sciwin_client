use std::collections::HashMap;

use commonwl::{StringOrDocument, prelude::*};
use petgraph::{graph::NodeIndex, prelude::StableDiGraph};

pub enum Node {
    Step(CWLDocument),
    Input(CommandInputParameter), //WorkflowInputParameter
    Output(WorkflowOutputParameter),
}

pub struct Edge {
    pub source_port: String,
    pub target_port: String,
    pub data_type: CWLType,
}

type WorkflowGraph = StableDiGraph<Node, Edge>;

#[derive(Default)]
pub struct WorkflowGraphBuilder {
    pub graph: WorkflowGraph,
    node_map: HashMap<String, NodeIndex>,
}

impl WorkflowGraphBuilder {
    pub fn from_workflow(&mut self, workflow: &Workflow) -> anyhow::Result<()> {
        for input in &workflow.inputs {
            let node_id = self.graph.add_node(Node::Input(input.clone()));
            self.node_map.insert(input.id.clone(), node_id);
        }

        for output in &workflow.outputs {
            let node_id = self.graph.add_node(Node::Output(output.clone()));
            self.node_map.insert(output.id.clone(), node_id);
        }

        for step in &workflow.steps {
            let StringOrDocument::Document(doc) = &step.run else {
                anyhow::bail!("String not supported")
            };
            let node_id = self.graph.add_node(Node::Step(*doc.clone()));
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
