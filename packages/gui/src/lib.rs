use crate::{
    components::files::{FileType, read_node_type},
    workflow::VisualWorkflow,
};
use dioxus::{html::geometry::ClientPoint, prelude::*, router::RouterContext};
use petgraph::graph::NodeIndex;
use s4n_core::{config::Config, project::initialize_project};
use serde::{Deserialize, Serialize};
use std::{
    env::temp_dir,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub mod components;
pub mod files;
pub mod graph;
pub mod layout;
pub mod types;
pub mod workflow;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ApplicationState {
    pub working_directory: Option<PathBuf>,
    pub current_file: Option<PathBuf>,
    #[serde(skip)]
    pub config: Option<Config>,
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

//used to open a project
#[derive(Default, Clone, Debug)]
pub struct ProjectInfo {
    pub working_directory: PathBuf,
    pub config: Config,
}

pub fn use_app_state() -> Signal<ApplicationState> {
    use_context::<Signal<ApplicationState>>()
}

pub fn use_drag() -> Signal<DragContext> {
    use_context::<Signal<DragContext>>()
}

pub async fn open_project(path: impl AsRef<Path>, mut open: Signal<bool>, mut confirmed: Signal<bool>) -> anyhow::Result<Option<ProjectInfo>> {
    let config_path = path.as_ref().join("workflow.toml");

    if !config_path.exists() {
        open.set(true);

        {
            let path = path.as_ref().to_owned();
            // Check dialog result
            loop {
                if !open() {
                    if confirmed() {
                        initialize_project(&path, false).map_err(|e| anyhow::anyhow!("{e}"))?;
                        confirmed.set(false); //reset
                        return Ok::<_, anyhow::Error>(Some(open_project_inner(path.as_ref())?));
                    }
                    return Ok(None);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    } else {
        Ok(Some(open_project_inner(path.as_ref())?))
    }
}

fn open_project_inner(path: &Path) -> anyhow::Result<ProjectInfo> {
    let config_path = path.join("workflow.toml");
    let toml = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&toml)?;
    Ok(ProjectInfo {
        working_directory: path.to_path_buf(),
        config,
    })
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

pub async fn restore_last_session(open: Signal<bool>, confirmed: Signal<bool>) -> anyhow::Result<Option<ApplicationState>> {
    if last_session_data().exists() {
        let data = fs::read_to_string(last_session_data())?;
        let mut state: ApplicationState = serde_json::from_str(&data)?;

        if let Some(working_dir) = &state.working_directory {
            let info = open_project(working_dir, open, confirmed).await?;
            if let Some(info) = info {
                state.working_directory = Some(info.working_directory);
                state.config = Some(info.config);
            }
        }

        if let Some(current_file) = &state.current_file
            && current_file.exists()
        {
            open_file(current_file, router());
        }
        Ok(Some(state))
    } else {
        Ok(None)
    }
}
