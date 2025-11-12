use crate::{
    edge::{Edge, EdgeElement},
    node::{NodeElement, NodeInstance, VisualNode},
    slot::Slot,
    use_app_state,
};
use commonwl::{StringOrDocument, load_doc, load_workflow, prelude::*};
use dioxus::html::geometry::euclid::Point2D;
use dioxus::prelude::*;
use petgraph::visit::IntoNodeIdentifiers;
use petgraph::{graph::NodeIndex, prelude::*};
use rand::Rng;
use std::{collections::HashMap, fs, path::Path};
use s4n::commands::ConnectWorkflowArgs;
use s4n::commands::disconnect_workflow_nodes;

pub type WorkflowGraph = StableDiGraph<VisualNode, Edge>;

pub fn load_workflow_graph(path: impl AsRef<Path>) -> anyhow::Result<(WorkflowGraph, String)> {
    let workflow_text = fs::read_to_string(path.as_ref())?;
    let workflow = load_workflow(path.as_ref()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let wgb = WorkflowGraphBuilder::from_workflow(&workflow, path)?;
    Ok((wgb.graph, workflow_text))
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
        let mut rng = rand::rng();

        for input in &workflow.inputs {
            let node_id = self.graph.add_node(VisualNode {
                instance: NodeInstance::Input(input.clone()),
                outputs: vec![Slot {
                    id: input.id.clone(),
                    type_: input.type_.clone(),
                }],
                inputs: vec![],
                position: Point2D::new(0.0, rng.random_range(0.0..=1.0)),
            });
            self.node_map.insert(input.id.clone(), node_id);
        }

        for output in &workflow.outputs {
            let node_id = self.graph.add_node(VisualNode {
                instance: NodeInstance::Output(output.clone()),
                inputs: vec![Slot {
                    id: output.id.clone(),
                    type_: output.type_.clone(),
                }],
                outputs: vec![],
                position: Point2D::new(rng.random_range(0.0..=1.0), 1.0),
            });
            self.node_map.insert(output.id.clone(), node_id);
        }

        // add steps sorted by execution order
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

            let node_id = self.graph.add_node(VisualNode {
                instance: NodeInstance::Step(doc.clone()),
                inputs: doc
                    .inputs
                    .iter()
                    .map(|i| Slot {
                        id: i.id.clone(),
                        type_: i.type_.clone(),
                    })
                    .collect(),
                outputs: doc
                    .get_output_ids()
                    .iter()
                    .map(|i| Slot {
                        id: i.to_string(),
                        type_: doc.get_output_type(i).unwrap(),
                    })
                    .collect(),
                position: Point2D::new(rng.random_range(0.0..=1.0), rng.random_range(0.0..=1.0)),
            });
            self.node_map.insert(step.id.clone(), node_id);

            for wsip in &step.in_ {
                let source = wsip.source.as_ref().unwrap(); //TODO!
                let (source, source_port) = source.split_once('/').unwrap_or((source.as_str(), source.as_str()));
                let type_ = doc.inputs.iter().find(|i| i.id == wsip.id).unwrap().type_.clone(); //TODO!

                self.connect_edge(source, &step.id, source_port, &wsip.id, type_)?;
            }
        }

        //add output connections
        for output in &workflow.outputs {
            let (source, source_port) = output.output_source.split_once("/").unwrap(); //EVIL!
            let type_ = output.type_.clone();
            self.connect_edge(source, &output.id, source_port, &output.id, type_)?
        }

        //layout
        self.auto_layout();

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

    pub fn auto_layout(&mut self) {
        let node_indices: Vec<_> = self.graph.node_indices().collect();
        info!("{node_indices:?}");
        let positions = rust_sugiyama::from_graph(
            &self.graph,
            &(|_, _| (120.0, 190.0)),
            &rust_sugiyama::configure::Config {
                vertex_spacing: 50.0,
                ..Default::default()
            },
        )
        .into_iter()
        .map(|(layout, _, _)| {
            let mut new_layout = HashMap::new();
            for (id, coords) in layout {
                new_layout.insert(id, coords);
            }
            new_layout
        })
        .collect::<Vec<_>>();
        let first = &positions[0];
        for ix in node_indices {
            let pos = first[&ix];
            self.graph[ix].position = Point2D::new(pos.1 as f32, pos.0 as f32);
        }
    }
}

#[component]
pub fn GraphEditor() -> Element {
    let graph = use_app_state()().graph;
    let mut app_state = use_app_state();

    rsx! {
        div {
            class:"relative select-none overflow-scroll h-full",
             onclick: move |_| {
                let maybe_edge_id = app_state.read().selected_edge;
                let workflow_path = app_state.read().workflow_path.clone();

                if let (Some(edge_id), Some(workflow_path)) = (maybe_edge_id, workflow_path) {
                    let read_state = app_state.read();
                    let edge = read_state.graph[edge_id].clone();
                    let (from_node_id, to_node_id) = read_state.graph.edge_endpoints(edge_id).unwrap();
                    let from_node_instance = read_state.graph[from_node_id].instance.clone();
                    let to_node_instance = read_state.graph[to_node_id].instance.clone();
                    drop(read_state);
                    let mut state = app_state.write();
                    state.graph.remove_edge(edge_id);
                    state.selected_edge = None;
                    // Construct connection arguments
                    let args = ConnectWorkflowArgs {
                        name: workflow_path.clone(),
                        from: format!("{}/{}", from_node_instance.id().trim_end_matches(".cwl"), edge.source_port),
                        to: format!("{}/{}", to_node_instance.id().trim_end_matches(".cwl"), edge.target_port),
                    };

                    if let Err(err) = disconnect_workflow_nodes(&args) {
                        error!("Failed to disconnect workflow nodes: {err}");
                    }
                     if let Ok(new_code) = fs::read_to_string(&workflow_path) {
                        state.cwl_code = Some(new_code);
                    }
                }
            },
            onmousemove: move |e| {
                if let Some(current) = use_app_state()().dragging{
                    //we are dragging
                    let current_pos = e.data.client_coordinates();
                    let last_pos = (use_app_state()().drag_offset)();

                    let deltaX = current_pos.x - last_pos.x;
                    let deltaY = current_pos.y - last_pos.y;

                    let pos = use_app_state()().graph[current].position;
                    use_app_state().write().graph[current].position = Point2D::new(pos.x + deltaX as f32, pos.y + deltaY as f32);
                    use_app_state().write().drag_offset.set(current_pos);
                }
            },
            onmouseup: move |_| {
                use_app_state().write().dragging = None;
            },
            for id in graph.node_identifiers() {
                NodeElement {id}
            },
            for id in graph.edge_indices() {
                EdgeElement {id}
            }
        }
    }
}
