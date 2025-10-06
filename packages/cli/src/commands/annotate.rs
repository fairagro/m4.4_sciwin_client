use clap::{Args, Subcommand};
use log::error;
use std::error::Error;
use tokio::runtime::Builder;
use commonwl::annotation::common::{annotate_license, annotate_field, annotate_default};
use commonwl::annotation::process::{annotate_process_step, ProcessArgs};
use commonwl::annotation::performer::{annotate_performer, annotate_performer_default};

/// Arguments for annotate performer command
#[derive(Args, Debug)]
pub struct PerformerArgs {
    #[arg(help = "Name of the CWL file")]
    pub cwl_name: String,
    #[arg(short = 'f', long = "first_name", help = "First name of the performer")]
    pub first_name: Option<String>,
    #[arg(short = 'l', long = "last_name", help = "Last name of the performer")]
    pub last_name: Option<String>,
    #[arg(short = 'm', long = "mid_initials", help = "Middle initials of the performer")]
    pub mid_initials: Option<String>,
    #[arg(short = 'e', long = "email", help = "Email of the performer")]
    pub mail: Option<String>,
    #[arg(short = 'a', long = "affiliation", help = "Affiliation of the performer")]
    pub affiliation: Option<String>,
    #[arg(short = 'd', long = "address", help = "Address of the performer")]
    pub address: Option<String>,
    #[arg(short = 'p', long = "phone", help = "Phone number of the performer")]
    pub phone: Option<String>,
    #[arg(short = 'x', long = "fax", help = "Fax number of the performer")]
    pub fax: Option<String>,
    #[arg(short = 'r', long = "role", help = "Role of the performer")]
    pub role: Option<String>,
}

/// Arguments for annotate process command
#[derive(Args, Debug)]
pub struct AnnotateProcessArgs {
    #[arg(help = "Name of the workflow process being annotated")]
    pub cwl_name: String,
    #[arg(short = 'n', long = "name", help = "Name of the process sequence step")]
    pub name: String,
    #[arg(short = 'i', long = "input", help = "Input file or directory, e.g., folder/input.txt")]
    pub input: Option<String>,
    #[arg(short = 'o', long = "output", help = "Output file or directory, e.g., folder/output.txt")]
    pub output: Option<String>,
    #[arg(short = 'p', long = "parameter", help = "Process step parameter")]
    pub parameter: Option<String>,
    #[arg(short = 'v', long = "value", help = "Process step value")]
    pub value: Option<String>,
}


/// Enum for annotate-related subcommands
#[derive(Debug, Subcommand)]
pub enum AnnotateCommands {
    #[command(about = "Annotates name of a tool or workflow")]
    Name {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(help = "Name of the tool or workflow")]
        name: String,
    },
    #[command(about = "Annotates description of a tool or workflow")]
    Description {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(help = "Description of the tool or workflow")]
        description: String,
    },
    #[command(about = "Annotates license of a tool or workflow")]
    License {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(help = "License to annotate")]
        license: Option<String>,
    },
    #[command(about = "Annotates performer of a tool or workflow (arc ontology)")]
    Performer(PerformerArgs),
    #[command(about = "Annotates a process arc ontology")]
    Process(AnnotateProcessArgs),
}

pub fn handle_annotation_command(command: &Option<AnnotateCommands>, tool_name: &Option<String>) -> Result<(), Box<dyn Error>> {
    let runtime = Builder::new_current_thread().enable_all().build()?;
    if let Some(subcommand) = command {
        runtime.block_on(handle_annotate_commands(subcommand))?;
    } else if let Some(name) = tool_name {
        annotate_default(name)?;
    } else {
        error!("No subcommand or tool name provided for annotate.");
    }
    Ok(())
}

pub async fn handle_annotate_commands(command: &AnnotateCommands) -> Result<(), Box<dyn Error>> {
    match command {
        AnnotateCommands::Name { cwl_name, name } => annotate_field(cwl_name, "label", name),
        AnnotateCommands::Description { cwl_name, description } => annotate_field(cwl_name, "doc", description),
        AnnotateCommands::License { cwl_name, license } => annotate_license(cwl_name, license).await,
        AnnotateCommands::Performer(args) => {
            use commonwl::annotation::performer::Performer;
            let performer = Performer {
                    cwl_name: args.cwl_name.clone(),
                    first_name: args.first_name.clone(),
                    last_name: args.last_name.clone(),
                    mid_initials: args.mid_initials.clone(),
                    mail: args.mail.clone(),
                    affiliation: args.affiliation.clone(),
                    address: args.address.clone(),
                    phone: args.phone.clone(),
                    fax: args.fax.clone(),
                    role: args.role.clone(),
                };
            if args.first_name.is_none() && args.last_name.is_none() {
                annotate_performer_default(&performer).await
            } else {
               
                annotate_performer(&performer).await
            }
        }
        AnnotateCommands::Process(args) => {
            let process_args = ProcessArgs {
            cwl_name: args.cwl_name.clone(),
            name: args.name.clone(),
            input: args.input.clone(),
            output: args.output.clone(),
            parameter: args.parameter.clone(),
            value: args.value.clone(),
            };
            annotate_process_step(&process_args).await
        },
    }
}
