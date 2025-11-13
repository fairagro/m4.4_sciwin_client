use crate::{
    edge::{Edge, EdgeElement},
    node::{NodeElement, NodeInstance, VisualNode},
    slot::Slot,
    use_app_state,
};
use rand::Rng;
use commonwl::{StringOrDocument, load_doc, prelude::*};
use dioxus::html::geometry::euclid::Point2D;
use dioxus::prelude::*;
use petgraph::visit::IntoNodeIdentifiers;
use petgraph::{graph::NodeIndex, prelude::*};
use std::{collections::HashMap, path::Path};

pub type WorkflowGraph = StableDiGraph<VisualNode, Edge>;

pub fn load_workflow_graph(workflow: &Workflow, path: impl AsRef<Path>) -> anyhow::Result<WorkflowGraph> {
    let wgb = WorkflowGraphBuilder::from_workflow(workflow, path)?;
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

        let positions = rust_sugiyama::from_graph(
            &self.graph,
            &(|_, _| (120.0, 190.0)),
            &rust_sugiyama::configure::Config {
                vertex_spacing: 30.0,
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

        for island in &positions {
            for ix in &node_indices {
                if let Some(pos) = island.get(ix) {
                    self.graph[*ix].position = Point2D::new(pos.1 as f32, pos.0 as f32);
                }
            }
        }
    }
}

#[component]
pub fn GraphEditor() -> Element {
    let graph = use_app_state()().workflow.graph;

    rsx! {
        div {
            class:"relative select-none overflow-scroll h-full",
            onmousemove: move |e| {
                if let Some(drag_state) = use_app_state()().dragging{
                    //we are dragging

                    match drag_state {
                        crate::DragState::None => todo!(),
                        crate::DragState::Node(node_index) => {
                            //we are dragging a node
                            let current_pos = e.data.client_coordinates();
                            let last_pos = (use_app_state()().drag_offset)();

                            let deltaX = current_pos.x - last_pos.x;
                            let deltaY = current_pos.y - last_pos.y;

                            let pos = use_app_state()().workflow.graph[node_index].position;
                            use_app_state().write().workflow.graph[node_index].position = Point2D::new(pos.x + deltaX as f32, pos.y + deltaY as f32);
                            use_app_state().write().drag_offset.set(current_pos);
                        },
                        crate::DragState::Connection { .. } => todo!(),
                    }

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
