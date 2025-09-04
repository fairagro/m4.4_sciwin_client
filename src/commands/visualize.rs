
use crate::{
    util::{
        DotRenderer, MermaidRenderer, render,
    },
};
use anyhow::anyhow;
use clap::{Args, ValueEnum};
use commonwl::load_workflow;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct VisualizeWorkflowArgs {
    #[arg(help = "Path to a workflow")]
    pub filename: PathBuf,
    #[arg(short = 'r', long = "renderer", help = "Select a flavor", value_enum, default_value_t = Renderer::Mermaid)]
    pub renderer: Renderer,
    #[arg(long = "no-defaults", help = "Do not print default values", default_value_t = false)]
    pub no_defaults: bool,
}

#[derive(Default, Debug, Clone, ValueEnum)]
pub enum Renderer {
    #[default]
    Mermaid,
    Dot,
}

#[allow(clippy::disallowed_macros)]
pub fn visualize(filename: &PathBuf, renderer: &Renderer, no_defaults: bool) -> anyhow::Result<()> {
    let cwl = load_workflow(filename).map_err(|e| anyhow!("Could mot load Workflow {filename:?}: {e}"))?;

    let code = match renderer {
        Renderer::Dot => render(&mut DotRenderer::default(), &cwl, filename, no_defaults),
        Renderer::Mermaid => render(&mut MermaidRenderer::default(), &cwl, filename, no_defaults),
    }
    .map_err(|e| anyhow!("Could not render visualization for {filename:?} using {renderer:?}: {e}"))?;

    println!("{code}");
    Ok(())
}
