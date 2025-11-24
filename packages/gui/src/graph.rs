use crate::{
    DragState,
    edge::{self, EdgeElement, Line, LineProps, VisualEdge},
    node::{NodeElement, NodeInstance, VisualNode},
    slot::Slot,
    use_app_state,
};
use commonwl::{StringOrDocument, load_doc, prelude::*};
use dioxus::html::geometry::{
    Pixels, PixelsSize, PixelsVector2D,
    euclid::{Point2D, Rect},
};
use dioxus::prelude::*;
use petgraph::visit::IntoNodeIdentifiers;
use petgraph::{graph::NodeIndex, prelude::*};
use rand::Rng;
use std::{collections::HashMap, path::Path, rc::Rc};

pub type WorkflowGraph = StableDiGraph<VisualNode, VisualEdge>;

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
                path: None,
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
                path: None,
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
                path: Some(step_file),
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
            VisualEdge {
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
    let mut new_line = use_signal(|| None::<LineProps>);
    let mut div_ref: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

    struct DivDims {
        rect: Rect<f64, Pixels>,
        scroll_offset: PixelsVector2D,
        scroll_size: PixelsSize,
    }
    let read_dims = move || async move {
        let div = div_ref()?;
        Some(DivDims {
            rect: div.get_client_rect().await.ok()?,
            scroll_offset: div.get_scroll_offset().await.ok()?,
            scroll_size: div.get_scroll_size().await.ok()?,
        })
    };

    let mut dim_w = use_signal(|| 0.0);
    let mut dim_h = use_signal(|| 0.0);

    let update_dims = move || {
        spawn(async move {
            if let Some(dims) = read_dims().await {
                dim_w.set(dims.scroll_size.width);
                dim_h.set(dims.scroll_size.height);
            }
        });
    };

    rsx! {
        div {
            class:"relative select-none overflow-scroll w-full h-full",
            onresize: move |_| update_dims(),
            onscroll: move |_| update_dims(),
            onmounted: move |e| div_ref.set(Some(e.data())),
            onmousemove: move |e| async move{
                e.stop_propagation();
                if let Some(drag_state) = use_app_state()().dragging{
                    //we are dragging
                    let current_pos = e.client_coordinates();

                    match drag_state {
                        DragState::None => todo!(),
                        DragState::Node(node_index) => {
                            //we are dragging a node
                            let last_pos = (use_app_state()().drag_offset)();

                            let deltaX = current_pos.x - last_pos.x;
                            let deltaY = current_pos.y - last_pos.y;

                            let pos = use_app_state()().workflow.graph[node_index].position;
                            use_app_state().write().workflow.graph[node_index].position = Point2D::new(pos.x + deltaX as f32, pos.y + deltaY as f32);
                            use_app_state().write().drag_offset.set(current_pos);
                        },
                        DragState::Connection { source_node, source_port } => {
                            //we are dragging from a connection
                            let dims = read_dims().await.unwrap();
                            let rect = dims.rect;
                            let scroll = dims.scroll_offset;

                            let base_pos = (current_pos.x - rect.origin.x,  current_pos.y - rect.origin.y);
                            let source_node = &use_app_state()().workflow.graph[source_node];

                            let (x_source, y_source) = edge::calculate_source_position(source_node, &source_port);
                            let x_target = (base_pos.0 + scroll.x) as f32;
                            let y_target = (base_pos.1 + scroll.y) as f32;

                            let cwl_type = source_node.outputs.iter().find(|i| i.id == source_port).unwrap().type_.clone(); //danger!
                            let stroke = edge::get_stroke_from_cwl_type(cwl_type);

                            new_line.set(Some(LineProps{x_source, y_source, x_target, y_target, stroke: stroke.to_string(), onclick: None}));
                        },
                    }

                }
            },
            onmouseup: move |_| {
                //reset state
                use_app_state().write().dragging = None;
                new_line.set(None);
            },
            for id in graph.node_identifiers() {
                NodeElement {id}
            },

            svg{
                width: "{dim_w}",
                height: "{dim_h}",
                view_box: "0 0 {dim_w} {dim_h}",
                class: "absolute inset-0  pointer-events-auto",
                for id in graph.edge_indices() {
                    g {
                        EdgeElement {id}
                    }
                },
                if let Some(line) = &*new_line.read() {
                    g {
                        Line {
                            x_source: line.x_source,
                            y_source: line.y_source,
                            x_target: line.x_target,
                            y_target: line.y_target,
                            stroke: line.stroke.clone(),
                            onclick: line.onclick
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commonwl::load_workflow;

    #[test]
    fn test_load_workflow_graph() {
        let path = "../../testdata/hello_world/workflows/main/main.cwl";
        let workflow = load_workflow(path).unwrap();
        let graph = load_workflow_graph(&workflow, path).unwrap();

        assert_eq!(graph.node_count(), 5); //2 inputs, 2 steps, 1 output
        assert_eq!(graph.edge_count(), 4);
    }

    #[test]
    fn test_load_workflow_graph_02() {
        let path = "../../testdata/mkdir_wf.cwl";
        let workflow = load_workflow(path).unwrap();
        let graph = load_workflow_graph(&workflow, path).unwrap();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
    }
}
