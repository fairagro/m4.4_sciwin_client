use crate::{
    DragContext, DragState,
    components::graph::{EdgeElement, Line, LineProps, NodeElement, calculate_source_position, get_stroke_from_cwl_type},
    use_app_state,
    workflow::VisualWorkflow,
};
use commonwl::load_doc;
use dioxus::html::geometry::{
    ClientPoint, Pixels, PixelsSize, PixelsVector2D,
    euclid::{Point2D, Rect},
};
use dioxus::prelude::*;
use petgraph::visit::IntoNodeIdentifiers;
use std::{path::Path, rc::Rc};

#[component]
pub fn GraphEditor(path: String) -> Element {
    let dragging = None::<DragState>;
    let drag_offset = use_signal(ClientPoint::zero);
    let mut drag_state = use_signal(|| DragContext { drag_offset, dragging });
    use_context_provider(|| drag_state);

    let mut app_state = use_app_state();

    {
        let tmp = path.clone();
        use_effect(move || {
            let path = Path::new(&tmp);
            let data = load_doc(path).unwrap();
            if let commonwl::CWLDocument::Workflow(_) = data {
                let workflow = VisualWorkflow::from_file(path).unwrap();
                app_state.write().workflow = workflow;
            }
        });
    }

    let graph = app_state().workflow.graph;

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
            class: "relative select-none overflow-scroll w-full h-full",
            onresize: move |_| update_dims(),
            onscroll: move |_| update_dims(),
            onmounted: move |e| div_ref.set(Some(e.data())),
            onmousemove: move |e| async move {
                e.stop_propagation();
                if let Some(dragstate) = drag_state().dragging {
                    //we are dragging
                    let current_pos = e.client_coordinates();

                    match dragstate {
                        DragState::None => todo!(),
                        DragState::Node(node_index) => {
                            //we are dragging a node
                            let last_pos = (drag_state().drag_offset)();

                            let deltaX = current_pos.x - last_pos.x;
                            let deltaY = current_pos.y - last_pos.y;

                            let pos = app_state.read().workflow.graph[node_index].position;
                            app_state.write().workflow.graph[node_index].position = Point2D::new(
                                //we are dragging from a connection

                                pos.x + deltaX as f32,
                                pos.y + deltaY as f32,
                            );
                            drag_state.write().drag_offset.set(current_pos);
                        }
                        DragState::Connection { source_node, source_port } => {
                            let dims = read_dims().await.unwrap();
                            let rect = dims.rect;
                            let scroll = dims.scroll_offset;
                            let base_pos = (
                                current_pos.x - rect.origin.x,
                                current_pos.y - rect.origin.y,
                            );
                            let source_node = &app_state.read().workflow.graph[source_node];
                            let (x_source, y_source) = calculate_source_position(
                                source_node,
                                &source_port,
                            );
                            let x_target = (base_pos.0 + scroll.x) as f32;
                            let y_target = (base_pos.1 + scroll.y) as f32;
                            let cwl_type = source_node
                                .outputs
                                .iter()
                                .find(|i| i.id == source_port)
                                .unwrap()
                                .type_
                                .clone();
                            let stroke = get_stroke_from_cwl_type(cwl_type);
                            new_line
                                .set(
                                    Some(LineProps {
                                        x_source,
                                        y_source,
                                        x_target,
                                        y_target,
                                        stroke: stroke.to_string(),
                                        onclick: None,
                                    }),
                                );
                        }
                    }
                }
            },
            onmouseup: move |_| {
                //reset state
                drag_state.write().dragging = None;
                new_line.set(None);
            },
            for id in graph.node_identifiers() {
                NodeElement { id }
            }

            svg {
                width: "{dim_w}",
                height: "{dim_h}",
                view_box: "0 0 {dim_w} {dim_h}",
                class: "absolute inset-0  pointer-events-auto",
                for id in graph.edge_indices() {
                    g {
                        EdgeElement { id }
                    }
                }
                if let Some(line) = &*new_line.read() {
                    g {
                        Line {
                            x_source: line.x_source,
                            y_source: line.y_source,
                            x_target: line.x_target,
                            y_target: line.y_target,
                            stroke: line.stroke.clone(),
                            onclick: line.onclick,
                        }
                    }
                }
            }
        }
    }
}
