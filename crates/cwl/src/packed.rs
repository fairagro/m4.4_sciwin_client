use crate::{
    CWLDocument, CommandLineTool, DefaultValue, Entry, StringOrDocument, Workflow, WorkflowStep,
    inputs::CommandInputParameter,
    io::normalize_path,
    load_doc,
    outputs::{CommandOutputParameter, WorkflowOutputParameter},
    prelude::Requirement,
    requirements::WorkDirItem,
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{self},
    path::Path,
};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct PackedCWL {
    #[serde(rename = "$graph")]
    graph: Vec<CWLDocument>,
    cwl_version: String,
}

pub fn pack_cwl(doc: &CWLDocument, filename: impl AsRef<Path>, id: Option<&str>) -> Result<Vec<CWLDocument>, Box<dyn Error>> {
    Ok(match doc {
        CWLDocument::CommandLineTool(clt) => {
            let mut clt = clt.clone();
            pack_commandlinetool(&mut clt, filename, id)?;
            vec![CWLDocument::CommandLineTool(clt)]
        }
        CWLDocument::Workflow(wf) => {
            let packed_wf = pack_workflow(wf, filename, id)?;
            packed_wf.graph
        }
        CWLDocument::ExpressionTool(et) => todo!(),
    })
}

fn pack_workflow(wf: &Workflow, filename: impl AsRef<Path>, id: Option<&str>) -> Result<PackedCWL, Box<dyn Error>> {
    let mut wf = wf.clone(); //make mutable reference
    if let Some(id) = id {
        wf.id = Some(id.to_string());
    } else {
        wf.id = Some("#main".to_string());
    }

    let wf_dir = filename.as_ref().parent().unwrap_or(filename.as_ref());
    let wf_id = wf.id.clone().unwrap();

    let mut graph = vec![];
    for input in &mut wf.inputs {
        pack_input(input, &wf_id, wf_dir);
    }

    for output in &mut wf.outputs {
        pack_workflow_output(output, &wf_id);
    }

    for req in &mut wf.requirements {
        pack_requirement(req, wf_dir)?;
    }
    for req in &mut wf.hints {
        pack_requirement(req, wf_dir)?;
    }

    for step in &mut wf.steps {
        graph.extend(pack_step(step, wf_dir, &wf_id)?);
    }
    let cwl_version = wf.cwl_version.as_ref().map_or("v1.2".to_string(), |v| v.clone());
    wf.cwl_version = None;

    graph.push(CWLDocument::Workflow(wf));

    Ok(PackedCWL { graph, cwl_version })
}

fn pack_commandlinetool(tool: &mut CommandLineTool, filename: impl AsRef<Path>, id: Option<&str>) -> Result<(), Box<dyn Error>> {
    let tool_dir = filename.as_ref().parent().unwrap_or(filename.as_ref());
    let name = filename.as_ref().file_name().unwrap().to_string_lossy();

    if let Some(id) = id {
        tool.id = Some(id.to_string());
    } else if let Some(id) = &mut tool.id {
        *id = format!("#{id}");
    } else {
        tool.id = Some(format!("#{name}"));
    }

    let id = tool.id.clone().unwrap();
    for input in &mut tool.inputs {
        pack_input(input, &id, tool_dir);
    }

    for output in &mut tool.outputs {
        pack_command_output(output, &id);
    }

    for req in &mut tool.requirements {
        pack_requirement(req, tool_dir)?;
    }
    for req in &mut tool.hints {
        pack_requirement(req, tool_dir)?;
    }

    Ok(())
}

fn pack_input(input: &mut CommandInputParameter, root_id: &str, doc_dir: impl AsRef<Path>) {
    input.id = format!("{root_id}/{}", input.id);

    //generate absolute paths for default values
    if let Some(DefaultValue::File(file)) = &mut input.default {
        if let Some(location) = &mut file.location
            && !location.starts_with("file://")
        {
            if Path::new(location).is_absolute() {
                *location = format!("file://{location}");
            } else {
                let path = doc_dir.as_ref().join(&location);
                let path = if path.exists() {
                    path.canonicalize().unwrap_or(path).to_string_lossy().into_owned()
                } else {
                    normalize_path(&path).unwrap_or(path).to_string_lossy().into_owned()
                };
                *location = format!("file://{path}");
            }
        }
    }

    if let Some(DefaultValue::Directory(dir)) = &mut input.default {
        if let Some(location) = &mut dir.location
            && !location.starts_with("file://")
        {
            if Path::new(location).is_absolute() {
                *location = format!("file://{location}");
            } else {
                let path = doc_dir.as_ref().join(&location);
                *location = format!("file://{}", path.canonicalize().unwrap_or(path).to_string_lossy());
            }
        }
    }
}

fn pack_workflow_output(output: &mut WorkflowOutputParameter, root_id: &str) {
    output.id = format!("{root_id}/{}", output.id);
    output.output_source = format!("{root_id}/{}", output.output_source);
}

fn pack_command_output(output: &mut CommandOutputParameter, root_id: &str) {
    output.id = format!("{root_id}/{}", output.id);
}

fn pack_requirement(requirement: &mut Requirement, doc_dir: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    match requirement {
        Requirement::InitialWorkDirRequirement(iwdr) => {
            for item in &mut iwdr.listing {
                if let WorkDirItem::Dirent(dirent) = item {
                    pack_entry(&mut dirent.entry, &doc_dir)?;
                }
            }
        }
        Requirement::DockerRequirement(dr) => {
            if let Some(file) = &mut dr.docker_file {
                pack_entry(file, &doc_dir)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn pack_entry(entry: &mut Entry, doc_dir: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    if let Entry::Include(include) = &entry {
        let path = &include.include;
        let contents = fs::read_to_string(doc_dir.as_ref().join(path))?;

        *entry = Entry::Source(contents);
    }

    Ok(())
}

fn pack_step(step: &mut WorkflowStep, wf_dir: impl AsRef<Path>, wf_id: &str) -> Result<Vec<CWLDocument>, Box<dyn Error>> {
    let step_id = format!("{wf_id}/{}", step.id);
    step.id = step_id.to_string();

    let mut packed_graph = match &mut step.run {
        StringOrDocument::String(filename) => {
            let path = Path::new(filename);
            let path = if path.is_absolute() { path } else { &wf_dir.as_ref().join(path) };
            let filename = if let Some(filename) = path.file_name() {
                filename.to_string_lossy().into_owned()
            } else {
                format!("{step_id}.cwl")
            };
            let step_hash = format!("#{filename}");
            let cwl = load_doc(path)?;
            let graph = pack_cwl(&cwl, path, Some(&step_hash))?;

            step.run = StringOrDocument::String(step_hash);
            graph
        }
        StringOrDocument::Document(doc) => {
            let step_hash = format!("#{step_id}.cwl");
            let graph = pack_cwl(doc, wf_dir.as_ref().join(&step.id), Some(&step_hash))?;

            step.run = StringOrDocument::String(step_hash);
            graph
        }
    };

    for input in &mut step.in_ {
        input.id = format!("{step_id}/{}", input.id);
        if let Some(src) = &mut input.source {
            *src = format!("{wf_id}/{src}");
        }
    }

    for output in &mut step.out {
        *output = format!("{step_id}/{output}");
    }

    let packed_graph = &mut packed_graph;
    for item in packed_graph.iter_mut() {
        item.cwl_version = None;
    }

    Ok(packed_graph.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CWLType, Command, Dirent, File, Include,
        inputs::CommandLineBinding,
        load_workflow,
        outputs::CommandOutputBinding,
        prelude::{DockerRequirement, InitialWorkDirRequirement, Requirement},
    };
    use serde_json::Value;

    #[test]
    fn test_pack_input() {
        let mut input = CommandInputParameter::default()
            .with_id("population")
            .with_type(CWLType::File)
            .with_default_value(DefaultValue::File(File::from_location("../../data/population.csv")))
            .with_binding(CommandLineBinding::default().with_prefix("--population"));

        let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();

        let file_path = base_dir.join("tests/test_data/hello_world/workflows/calculation");
        pack_input(&mut input, "#calculation.cwl", file_path);

        let json = serde_json::json!(&input);

        let reference_json = r##"{
                    "id": "#calculation.cwl/population",
                    "type": "File",
                    "default": {
                        "class": "File",
                        "location": "file://XXX/tests/test_data/hello_world/data/population.csv"
                    },
                    "inputBinding": {
                        "prefix": "--population"
                    }
                }"##
        .replace("XXX", &base_dir.to_string_lossy());

        let value: Value = serde_json::from_str(&reference_json).unwrap();
        assert_eq!(json, value);
    }

    #[test]
    fn test_pack_workflow_output() {
        let mut output = WorkflowOutputParameter {
            id: "out".to_string(),
            type_: CWLType::File,
            output_source: "plot/results".to_string(),
        };

        pack_workflow_output(&mut output, "#main");
        let json = serde_json::json!(&output);

        let reference_json = r##"{
                    "id": "#main/out",
                    "type": "File",
                    "outputSource": "#main/plot/results"
                }"##;

        let value: Value = serde_json::from_str(reference_json).unwrap();
        assert_eq!(json, value);
    }

    #[test]
    fn test_pack_command_output() {
        let mut output = CommandOutputParameter::default()
            .with_id("results")
            .with_type(CWLType::File)
            .with_binding(CommandOutputBinding {
                glob: Some("results.csv".to_string()),
                ..Default::default()
            });

        pack_command_output(&mut output, "#calculation.cwl");
        let json = serde_json::json!(&output);

        let reference_json = r##"{
                    "id": "#calculation.cwl/results",
                    "type": "File",
                    "outputBinding": {
                        "glob": "results.csv"
                    }
                }"##;

        let value: Value = serde_json::from_str(reference_json).unwrap();
        assert_eq!(json, value);
    }

    #[test]
    fn test_pack_commandlinetool() {
        let mut tool = CommandLineTool::default()
            .with_base_command(Command::Multiple(vec![
                "python".to_string(),
                "workflows/calculation/calculation.py".to_string(),
            ]))
            .with_inputs(vec![
                CommandInputParameter::default()
                    .with_id("population")
                    .with_type(CWLType::File)
                    .with_default_value(DefaultValue::File(File::from_location("../../data/population.csv")))
                    .with_binding(CommandLineBinding::default().with_prefix("--population")),
                CommandInputParameter::default()
                    .with_id("speakers")
                    .with_type(CWLType::File)
                    .with_default_value(DefaultValue::File(File::from_location("../../data/speakers_revised.csv")))
                    .with_binding(CommandLineBinding::default().with_prefix("--speakers")),
            ])
            .with_outputs(vec![
                CommandOutputParameter::default()
                    .with_id("results")
                    .with_type(CWLType::File)
                    .with_binding(CommandOutputBinding {
                        glob: Some("results.csv".to_string()),
                        ..Default::default()
                    }),
            ])
            .with_requirements(vec![
                Requirement::InitialWorkDirRequirement(InitialWorkDirRequirement {
                    listing: vec![WorkDirItem::Dirent(Dirent {
                        entryname: Some("workflows/calculation/calculation.py".to_string()),
                        entry: Entry::Include(Include {
                            include: "calculation.py".to_string(),
                        }),
                        ..Default::default()
                    })],
                }),
                Requirement::DockerRequirement(DockerRequirement {
                    docker_pull: Some("pandas/pandas:pip-all".to_string()),
                    ..Default::default()
                }),
            ]);

        let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
        let file_path = base_dir.join("tests/test_data/hello_world/workflows/calculation/calculation.cwl");
        pack_commandlinetool(&mut tool, file_path, Some("#main")).unwrap();
        let json = serde_json::json!(&tool);

        let reference_json =
            include_str!("../../../tests/test_data/packed/calculation_packed.cwl").replace("/mnt/m4.4_sciwin_client", &base_dir.to_string_lossy());

        let value: Value = serde_json::from_str(&reference_json).unwrap();
        assert_eq!(json, value);
    }

    #[test]
    fn test_pack_workflow() {
        let file = "../../tests/test_data/hello_world/workflows/main/main.cwl";
        let wf = load_workflow(file).unwrap();

        let mut packed = pack_workflow(&wf, file, None).unwrap();
        packed.graph.sort_by(|a, b| a.id.cmp(&b.id));
        let json = serde_json::json!(&packed);

        let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
        let reference_json =
            include_str!("../../../tests/test_data/packed/main_packed.cwl").replace("/mnt/m4.4_sciwin_client", &base_dir.to_string_lossy());

        let value: Value = serde_json::from_str(&reference_json).unwrap();
        assert_eq!(json, value);
    }
}
