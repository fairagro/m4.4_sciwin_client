use anyhow::{Context, Result};
use commonwl::{StringOrDocument, load_doc, prelude::*};

pub fn add_new_step_if_not_exists(workflow: &mut Workflow, name: &str, path: &str, doc: &CWLDocument) {
    if !workflow.has_step(name) {
        let path = if path.starts_with("workflows") {
            path.replace("workflows", "..")
        } else {
            format!("../../{path}")
        };
        let workflow_step = WorkflowStep {
            id: name.to_string(),
            run: StringOrDocument::String(path),
            out: doc.get_output_ids(),
            ..Default::default()
        };
        workflow.steps.push(workflow_step);

        if let CWLDocument::Workflow(_) = doc
            && !workflow
                .requirements
                .iter()
                .any(|r| matches!(r, Requirement::SubworkflowFeatureRequirement))
        {
            workflow.requirements.push(Requirement::SubworkflowFeatureRequirement);
        }
    }
}

/// Adds a connection between an input and a `CommandLineTool`. The tool will be registered as step if it is not already and an Workflow input will be added.
pub fn add_input_connection(workflow: &mut Workflow, from_input: &str, to_filename: &str, to_name: &str, to_slot_id: &str) -> Result<()> {
    let to_cwl = load_doc(to_filename).map_err(|e| anyhow::anyhow!("Failed to load CWL document: {e}"))?;
    let to_slot = to_cwl.inputs.iter().find(|i| i.id == to_slot_id).expect("No slot");

    //register input
    if !workflow.has_input(from_input) {
        let mut input = CommandInputParameter::default().with_id(from_input).with_type(to_slot.type_.clone());
        input.default = to_slot.default.clone();
        workflow.inputs.push(input);
    }

    add_new_step_if_not_exists(workflow, to_name, to_filename, &to_cwl);
    //add input in step
    workflow
        .steps
        .iter_mut()
        .find(|step| step.id == to_name)
        .unwrap()
        .in_
        .push(WorkflowStepInputParameter {
            id: to_slot_id.to_string(),
            source: Some(from_input.to_owned()),
            ..Default::default()
        });
    Ok(())
}

/// Adds a connection between an output and a `CommandLineTool`. The tool will be registered as step if it is not already and an Workflow output will be added.
pub fn add_output_connection(workflow: &mut Workflow, from_name: &str, from_slot_id: &str, from_filename: &str, to_output: &str) -> Result<()> {
    let from_cwl = load_doc(from_filename).map_err(|e| anyhow::anyhow!("Failed to load CWL document: {e}"))?;
    let from_type = from_cwl.get_output_type(from_slot_id).context("No slot")?;
    add_new_step_if_not_exists(workflow, from_name, from_filename, &from_cwl);

    if !workflow.has_output(to_output) {
        workflow.outputs.push(WorkflowOutputParameter::default().with_id(to_output).clone());
    }

    let output = workflow.outputs.iter_mut().find(|o| o.id == to_output).unwrap();
    output.type_ = from_type;
    output.output_source = format!("{from_name}/{from_slot_id}");

    Ok(())
}

/// Adds a connection between two `CommandLineTools`. The tools will be registered as step if registered not already.
pub fn add_step_connection(
    workflow: &mut Workflow,
    from_filename: &str,
    from_name: &str,
    from_slot_id: &str,
    to_filename: &str,
    to_name: &str,
    to_slot_id: &str,
) -> Result<()> {
    //check if step already exists and create if not
    if !workflow.has_step(from_name) {
        let from_cwl = load_doc(from_filename).map_err(|e| anyhow::anyhow!("Failed to load CWL document: {e}"))?;
        let from_outputs = from_cwl.get_output_ids();
        if !from_outputs.contains(&from_slot_id.to_string()) {
            anyhow::bail!(
                "Tool {} does not have output `{}`. Cannot not create node from {} in Workflow!",
                from_name,
                from_slot_id,
                from_filename
            );
        }

        //create step
        add_new_step_if_not_exists(workflow, from_name, from_filename, &from_cwl);
    }

    //check if step exists
    if !workflow.has_step(to_name) {
        let to_cwl = load_doc(to_filename).map_err(|e| anyhow::anyhow!("Failed to load CWL document: {e}"))?;
        add_new_step_if_not_exists(workflow, to_name, to_filename, &to_cwl);
    }

    let step = workflow.steps.iter_mut().find(|s| s.id == to_name).unwrap(); //safe here!
    step.in_.push(WorkflowStepInputParameter {
        id: to_slot_id.to_string(),
        source: Some(format!("{from_name}/{from_slot_id}")),
        ..Default::default()
    });

    Ok(())
}

/// Removes a connection between two `CommandLineTools` by removing input from `tool_y` that is also output of `tool_x`.
pub fn remove_step_connection(workflow: &mut Workflow, to_name: &str, to_slot_id: &str) -> Result<()> {
    let step = workflow.steps.iter_mut().find(|s| s.id == to_name);
    // If the step is found, try to remove the connection by removing input from `tool_y` that uses output of `tool_x`
    // Input is empty, change that?
    if let Some(step) = step {
        if step.in_.iter().any(|v| v.id == to_slot_id) {
            step.in_.retain(|v| v.id != to_slot_id);
        }
        Ok(())
    } else {
        anyhow::bail!("Failed to find step {} in workflow!", to_name);
    }
}

/// Removes an input from inputs and removes it from `CommandLineTool` input.
pub fn remove_input_connection(workflow: &mut Workflow, from_input: &str, to_name: &str, to_slot_id: &str) -> Result<()> {
    if let Some(index) = workflow.inputs.iter().position(|s| s.id == *from_input.to_string()) {
        workflow.inputs.remove(index);
    }
    if let Some(step) = workflow.steps.iter_mut().find(|s| s.id == to_name) {
        if step.in_.iter().any(|v| v.id == to_slot_id) {
            step.in_.retain(|v| v.id != to_slot_id);
            Ok(())
        } else {
            anyhow::bail!("Input {} not found in step {}!", to_slot_id, to_name);
        }
    } else {
        anyhow::bail!("Step {} not found in workflow!", to_name);
    }
}

/// Removes a connection between an output and a `CommandLineTool`.
pub fn remove_output_connection(workflow: &mut Workflow, from_name: &str, from_slot_id: &str, to_output: &str) -> Result<()> {
    if let Some(index) = workflow.outputs.iter().position(|o| o.id == to_output) {
        // Remove the output connection
        workflow.outputs.remove(index);
    }
    // Check if this output is part of any step output and remove it, do we want that?
    if let Some(step) = workflow.steps.iter_mut().find(|s| s.id == from_name)
        && let Some(output_index) = step.out.iter().position(|out| out == from_slot_id)
    {
        step.out.remove(output_index);
    }
    Ok(())
}
