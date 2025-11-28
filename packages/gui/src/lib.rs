use crate::{
    components::files::{FileType, read_node_type},
    workflow::VisualWorkflow,
};
use dioxus::{html::geometry::ClientPoint, prelude::*, router::RouterContext};
use petgraph::graph::NodeIndex;
use s4n_core::config::Config;
use serde::{Deserialize, Serialize};
use std::{
    env::temp_dir,
    fs,
    path::{Path, PathBuf},
};

pub mod components;
pub mod graph;
pub mod layout;
pub mod types;
pub mod workflow;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ApplicationState {
    pub working_directory: Option<PathBuf>,
    pub current_file: Option<PathBuf>,
    #[serde(skip)]
    pub project_name: Option<String>,
    #[serde(skip)]
    pub workflow: VisualWorkflow,
}

#[derive(Default, Debug, Clone)]
pub enum DragState {
    #[default]
    None, // not used maybe
    Node(NodeIndex), //used when drag starts on Node Header
    Connection {
        //used when drag starts from slot
        source_node: NodeIndex,
        source_port: String,
    },
}

#[derive(Default, Clone, Debug)]
pub struct DragContext {
    pub dragging: Option<DragState>,
    pub drag_offset: Signal<ClientPoint>,
}

pub fn use_app_state() -> Signal<ApplicationState> {
    use_context::<Signal<ApplicationState>>()
}

pub fn use_drag() -> Signal<DragContext> {
    use_context::<Signal<DragContext>>()
}

pub fn open_project(path: impl AsRef<Path>) -> anyhow::Result<()> {
    let mut app_state = use_app_state();

    let config_path = path.as_ref().join("workflow.toml");
    if !config_path.exists() {
        //ask user to init a new project
        return Ok(());
    } else {
        let toml = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&toml)?;
        app_state.write().project_name = Some(config.workflow.name);
    }
    app_state.write().working_directory = Some(path.as_ref().to_path_buf());

    Ok(())
}

pub fn close_project() -> anyhow::Result<()> {
    let mut app_state = use_app_state();

    fs::remove_file(last_session_data())?;
    app_state.set(ApplicationState::default());

    Ok(())
}

fn open_file(path: impl AsRef<Path>, router: RouterContext) {
    if path.as_ref().exists() {
        match read_node_type(&path) {
            FileType::Workflow => router.push(format!("/workflow?path={}", path.as_ref().to_string_lossy())),
            FileType::Other => router.push("/"),
            _ => router.push(format!("/tool?path={}", path.as_ref().to_string_lossy())),
        };
    }
}

pub fn last_session_data() -> PathBuf {
    let tmp = temp_dir().join("s4n");

    if !tmp.exists() {
        fs::create_dir_all(&tmp).expect("Could not create temp directory");
    }

    tmp.join("app_state.json")
}

pub fn restore_last_session() -> anyhow::Result<()> {
    if last_session_data().exists() {
        let data = fs::read_to_string(last_session_data())?;
        let state: ApplicationState = serde_json::from_str(&data)?;
        let mut current_state = use_app_state();

        if let Some(working_dir) = &state.working_directory {
            open_project(working_dir)?;
        } else {
            current_state.write().working_directory = state.working_directory
        }

        if let Some(current_file) = &state.current_file {
            open_file(current_file, router());
        }

        current_state.write().current_file = state.current_file;
    }

    Ok(())
}
