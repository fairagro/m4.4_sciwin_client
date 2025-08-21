use commonwl::{CWLDocument, CommandLineTool, ExpressionTool, Workflow, load_doc};
use std::path::Path;
use colored::Colorize;

pub(crate) fn list_single_cwl(filename: impl AsRef<Path>) -> anyhow::Result<()> {
    let filename = filename.as_ref();
    if !filename.exists() {
        eprintln!("Tool does not exist: {}", filename.display());
        return Ok(()); //we are okay with the non existance here!
    }

    let tool = load_doc(filename).map_err(|e| anyhow::anyhow!("Could not load CWL File: {e}"))?;
    match tool {
        CWLDocument::CommandLineTool(clt) => list_clt(&clt, filename),
        CWLDocument::ExpressionTool(et) => list_et(&et, filename),
        CWLDocument::Workflow(wf) => list_wf(&wf, filename),
    }?;

    Ok(())
}

fn list_clt(clt: &CommandLineTool, filename: &Path) -> anyhow::Result<()> {
   
    Ok(())
}

fn list_et(clt: &ExpressionTool, filename: &Path) -> anyhow::Result<()> {
    Ok(())
}

fn list_wf(clt: &Workflow, filename: &Path) -> anyhow::Result<()> {
    Ok(())
}
