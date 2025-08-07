use crate::cwl::highlight_cwl;
use clap::{Args, Subcommand};
use colored::Colorize;
use commonwl::format::format_cwl;
use dialoguer::{Confirm, FuzzySelect, Input, Select};
use log::error;
use reqwest::get;
use serde_yaml::{Mapping, Value};
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{self, Write};
use std::path::PathBuf;
use std::{env, fs, path::Path};
use tokio::runtime::Builder;
use util::is_cwl_file;

const REST_URL_TS: &str = "https://terminology.services.base4nfdi.de/api-gateway/search?query=";
const SCHEMAORG_NAMESPACE: &str = "https://schema.org/";
const SCHEMAORG_SCHEMA: &str = "https://schema.org/version/latest/schemaorg-current-https.rdf";
const ARC_NAMESPACE: &str = "https://github.com/nfdi4plants/ARC_ontology";
const ARC_SCHEMA: &str = "https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl";
const MAX_RECOMMENDATIONS: usize = 10;

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
        AnnotateCommands::Schema { cwl_name, schema } => annotate(cwl_name, "$schemas", None, Some(schema)),
        AnnotateCommands::Namespace { cwl_name, namespace, short } => annotate(cwl_name, "$namespaces", short.as_deref(), Some(namespace)),
        AnnotateCommands::Author(args) => {
            let role_args = PersonArgs {
                cwl_name: args.cwl_name.clone(),
                name: args.name.clone(),
                mail: args.mail.clone(),
                id: args.id.clone(),
            };
            annotate_person(&role_args, "author")
        }
        AnnotateCommands::Contributor(args) => {
            let role_args = PersonArgs {
                cwl_name: args.cwl_name.clone(),
                name: args.name.clone(),
                mail: args.mail.clone(),
                id: args.id.clone(),
            };
            annotate_person(&role_args, "contributor")
        }
        AnnotateCommands::Performer(args) => {
            if args.first_name.is_none() && args.last_name.is_none() {
                annotate_performer_default(args).await
            } else {
                annotate_performer(args).await
            }
        }
        AnnotateCommands::Process(args) => annotate_process_step(args).await,
        AnnotateCommands::Container { cwl_name, container } => annotate_container(cwl_name, container),
        AnnotateCommands::Custom { cwl_name, field, value } => annotate_field(cwl_name, field, value),
    }
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
    #[command(about = "Annotates schema of a tool or workflow")]
    Schema {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(help = "Schema used for annotation")]
        schema: String,
    },
    #[command(about = "Annotates namespace of a tool or workflow")]
    Namespace {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(help = "Namespace to annotate")]
        namespace: String,
        #[arg(short = 's', long = "short", help = "Namespace abbreviation to annotate")]
        short: Option<String>,
    },
    #[command(about = "Annotates author of a tool or workflow (schema.org)")]
    Author(PersonArgs),

    #[command(about = "Annotates contributor of a tool or workflow (schema.org)")]
    Contributor(PersonArgs),

    #[command(about = "Annotates performer of a tool or workflow (arc ontology)")]
    Performer(PerformerArgs),

    #[command(about = "Annotates a process arc ontolology")]
    Process(AnnotateProcessArgs),

    #[command(about = "Annotates container information of a tool or workflow")]
    Container {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(short = 'c', long = "container", help = "Annotation value for the container")]
        container: String,
    },
    #[command(about = "Annotates a CWL file with an custom field and value")]
    Custom {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(help = "Field to annotate")]
        field: String,
        #[arg(help = "Value for the field")]
        value: String,
    },
}

#[derive(Args, Debug)]
pub struct PersonArgs {
    pub cwl_name: String,

    #[arg(short = 'n', long = "name", help = "Name of the person")]
    pub name: String,

    #[arg(short = 'm', long = "mail", help = "Email of the person")]
    pub mail: Option<String>,

    #[arg(short = 'i', long = "id", help = "Identifier of the person, e.g., ORCID")]
    pub id: Option<String>,
}

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

pub async fn ask_for_license() -> Result<Option<(String, String)>, Box<dyn Error>> {
    // Fetch the SPDX license list
    let response = get("https://spdx.org/licenses/licenses.json").await?;
    let json: serde_json::Value = response.json().await?;

    // Extract and format license entries
    let licenses = json["licenses"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|l| {
            let reference = l.get("reference")?.as_str()?.to_string();
            let name = l.get("name")?.as_str()?.to_string();
            Some((name, reference))
        })
        .collect::<Vec<_>>();

    let mut sorted_list = licenses.clone();
    sorted_list.sort_by(|a, b| a.0.cmp(&b.0));

    // Prepare display list for FuzzySelect
    let display_list: Vec<String> = sorted_list.iter().map(|(name, reference)| format!("{name} ({reference})")).collect();

    // Use FuzzySelect for interactive search
    let selection = FuzzySelect::new()
        .with_prompt("Type in a license to search for and select one of the suggestions")
        .items(&display_list)
        .max_length(10)
        .interact_opt()?;

    if let Some(idx) = selection {
        let (name, reference) = &sorted_list[idx];
        Ok(Some((name.clone(), reference.clone())))
    } else {
        Ok(None)
    }
}

pub async fn annotate_license(cwl_name: &str, license: &Option<String>) -> Result<(), Box<dyn Error>> {
    if let Some(license_value) = license {
        annotate(cwl_name, "$namespaces", Some("s"), Some(SCHEMAORG_NAMESPACE))?;
        annotate(cwl_name, "$schemas", None, Some(SCHEMAORG_SCHEMA))?;
        annotate(cwl_name, "s:license", None, Some(license_value))?;
    } else {
        // If no license is provided, ask user to select one
        if let Some((_name, spdx_license)) = ask_for_license().await? {
            annotate(cwl_name, "$namespaces", Some("s"), Some(SCHEMAORG_NAMESPACE))?;
            annotate(cwl_name, "$schemas", None, Some(SCHEMAORG_SCHEMA))?;
            annotate(cwl_name, "s:license", None, Some(&spdx_license))?;
        }
    }
    Ok(())
}

//maybe remove disallowed_macros but also no error, check alternatives to eprintln!
#[allow(clippy::disallowed_macros)]
async fn get_affiliation_and_orcid(first_name: &str, last_name: &str) -> (Option<String>, Option<String>, Option<String>) {
    if first_name.is_empty() || last_name.is_empty() {
        return (None, None, None);
    }

    let query = format!(
        "given-names:{} AND family-name:{}",
        first_name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect::<String>())
            .unwrap_or_default()
            + first_name.chars().skip(1).collect::<String>().as_str(),
        last_name.chars().next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default()
            + last_name.chars().skip(1).collect::<String>().as_str()
    );
    let search_url = format!("https://pub.orcid.org/v3.0/expanded-search/?q={query}");
    let client = reqwest::Client::new();

    let resp = client.get(&search_url).header("Accept", "application/json").send().await;

    if let Ok(resp) = resp
        && let Ok(json) = resp.json::<serde_json::Value>().await
        && let Some(results) = json.get("expanded-result").and_then(|v| v.as_array())
    {
        let get_field = |result: &serde_json::Value, key: &str| {
            result
                .get(key)
                .and_then(|v| {
                    if v.is_array() {
                        v.as_array().and_then(|arr| arr.first()).and_then(|f| f.as_str())
                    } else {
                        v.as_str()
                    }
                })
                .unwrap_or("")
                .to_string()
        };

        let show_options = |results: &[serde_json::Value]| -> Option<(String, String, String)> {
            println!("\nOther possible matches:");
            let options: Vec<String> = results
                .iter()
                .take(5)
                .map(|result| {
                    let given_names = get_field(result, "given-names");
                    let family_name = get_field(result, "family-names");
                    let institution = get_field(result, "institution-name");
                    format!("{given_names} {family_name} ({institution})")
                })
                .collect();

            let selection = Select::new()
                .with_prompt("Select the correct person, or Esc to skip")
                .items(&options)
                .default(0)
                .interact_opt()
                .unwrap_or(None);

            if let Some(idx) = selection {
                let selected = &results[idx];
                Some((
                    get_field(selected, "institution-name"),
                    get_field(selected, "orcid-id"),
                    get_field(selected, "email"),
                ))
            } else {
                None
            }
        };

        if let Some(first_result) = results.first() {
            let mut first_affiliation = get_field(first_result, "institution-name");
            let mut first_orcid = get_field(first_result, "orcid-id");
            let mut first_mail = get_field(first_result, "email");
            println!(
                "\nFirst ORCID search result:\n\
                         ──────────────────────────────\n\
                         Name       : {first_name} {last_name}\n\
                         Affiliation: {first_affiliation}\n\
                         ORCID      : {first_orcid}\n\
                         Email      : {first_mail}\n",
            );
            let is_right_person = Confirm::new().with_prompt("Is this the correct person?").interact().unwrap_or(false);
            if !is_right_person && let Some((aff, orcid, mail)) = show_options(results) {
                first_affiliation = aff;
                first_orcid = orcid;
                first_mail = mail;
            }
            println!(
                "\nPerson:\n\
                        ──────────────────────────────\n\
                        Name       : {first_name} {last_name}\n\
                        Email      : {first_mail}\n"
            );
            io::stdout().flush().ok();
            // Allow user to edit fields if some of them are wrong
            if Confirm::new()
                .with_prompt("Do you want to edit any of these fields?")
                .interact()
                .unwrap_or(false)
            {
                //not possible to edit ORCID or name but mail and affiliation?
                first_affiliation = Input::new()
                    .with_prompt("Edit affiliation (leave blank to keep)")
                    .default(first_affiliation.clone())
                    .allow_empty(true)
                    .interact_text()
                    .unwrap_or(first_affiliation.clone());
                first_mail = Input::new()
                    .with_prompt("Edit email (leave blank to keep)")
                    .default(first_mail.clone())
                    .allow_empty(true)
                    .interact_text()
                    .unwrap_or(first_mail.clone());
            }
            return (
                if !first_affiliation.is_empty() { Some(first_affiliation) } else { None },
                if !first_orcid.is_empty() { Some(first_orcid) } else { None },
                if !first_mail.is_empty() { Some(first_mail) } else { None },
            );
        }
    }
    (None, None, None)
}

async fn annotate_performer_default(args: &PerformerArgs) -> Result<(), Box<dyn Error>> {
    // Ask user for first name and last name
    let first_name: String = Input::new().with_prompt("Enter performer's first name").interact_text()?;
    let last_name: String = Input::new().with_prompt("Enter performer's last name").interact_text()?;

    // Ask for other fields
    let mid_initials = {
        let input: String = Input::new()
            .with_prompt("Enter performer's middle initials (or leave blank)")
            .allow_empty(true)
            .interact_text()?;
        if input.is_empty() { None } else { Some(input) }
    };

    let address = {
        let input: String = Input::new()
            .with_prompt("Enter performer's address (or leave blank)")
            .allow_empty(true)
            .interact_text()?;
        if input.is_empty() { None } else { Some(input) }
    };

    let phone = {
        let input: String = Input::new()
            .with_prompt("Enter performer's phone number (or leave blank)")
            .allow_empty(true)
            .interact_text()?;
        if input.is_empty() { None } else { Some(input) }
    };

    let fax = {
        let input: String = Input::new()
            .with_prompt("Enter performer's fax number (or leave blank)")
            .allow_empty(true)
            .interact_text()?;
        if input.is_empty() { None } else { Some(input) }
    };

    // Ask if user wants to search person via ORCID
    let search_orcid = Confirm::new()
        .with_prompt("Do you want to search for this person via ORCID?")
        .interact()?;

    let mut mail: Option<String> = None;
    let mut affiliation = None;
    let mut role: Option<String> = None;

    if search_orcid {
        let (aff, _orcid, m) = get_affiliation_and_orcid(&first_name, &last_name).await;
        affiliation = aff;
        mail = m;
        // Optionally ask for role
        if Confirm::new().with_prompt("Do you want to annotate a role?").interact()? {
            let input: String = Input::new()
                .with_prompt("Enter role (or leave blank)")
                .allow_empty(true)
                .interact_text()?;
            if !input.is_empty() {
                role = Some(input);
            }
        }
    } else {
        // Ask if user wants to annotate other fields
        if Confirm::new()
            .with_prompt("Do you want to annotate additional fields (email, affiliation, role)?")
            .interact()?
        {
            let input: String = Input::new()
                .with_prompt("Enter email (or leave blank)")
                .allow_empty(true)
                .interact_text()?;
            if !input.is_empty() {
                mail = Some(input);
            }
            let input: String = Input::new()
                .with_prompt("Enter affiliation (or leave blank)")
                .allow_empty(true)
                .interact_text()?;
            if !input.is_empty() {
                affiliation = Some(input);
            }
            let input: String = Input::new()
                .with_prompt("Enter role (or leave blank)")
                .allow_empty(true)
                .interact_text()?;
            if !input.is_empty() {
                role = Some(input);
            }
        }
    }

    let default_performer = PerformerArgs {
        cwl_name: args.cwl_name.clone(),
        first_name: Some(first_name),
        last_name: Some(last_name),
        mid_initials,
        mail,
        affiliation,
        address,
        phone,
        fax,
        role,
    };
    annotate_performer(&default_performer).await
}

pub async fn annotate_performer(args: &PerformerArgs) -> Result<(), Box<dyn Error>> {
    // Ensure ARC namespace and schema are defined
    annotate(&args.cwl_name, "$schemas", None, Some(ARC_SCHEMA))?;
    annotate(&args.cwl_name, "$namespaces", Some("arc"), Some(ARC_NAMESPACE))?;
    // Parse CWL file into YAML
    let mut yaml = parse_cwl(&args.cwl_name)?;

    // Ensure the root of the YAML is a mapping
    let Value::Mapping(ref mut mapping) = yaml else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    };

    // Prepare performer information
    let mut performer_info = Mapping::new();
    performer_info.insert(Value::String("class".to_string()), Value::String("arc:Person".to_string()));
    performer_info.insert(
        Value::String("arc:first name".to_string()),
        Value::String(args.first_name.clone().unwrap_or_default()),
    );
    performer_info.insert(
        Value::String("arc:last name".to_string()),
        Value::String(args.last_name.clone().unwrap_or_default()),
    );

    // Add optional fields if present
    if let Some(ref mid_initials) = args.mid_initials {
        performer_info.insert(Value::String("arc:mid initials".to_string()), Value::String(mid_initials.clone()));
    }
    if let Some(ref mail) = args.mail {
        performer_info.insert(Value::String("arc:email".to_string()), Value::String(mail.clone()));
    }
    if let Some(ref affiliation) = args.affiliation {
        performer_info.insert(Value::String("arc:affiliation".to_string()), Value::String(affiliation.clone()));
    }
    if let Some(ref address) = args.address {
        performer_info.insert(Value::String("arc:address".to_string()), Value::String(address.clone()));
    }
    if let Some(ref phone) = args.phone {
        performer_info.insert(Value::String("arc:phone".to_string()), Value::String(phone.clone()));
    }
    if let Some(ref fax) = args.fax {
        performer_info.insert(Value::String("arc:fax".to_string()), Value::String(fax.clone()));
    }

    // Handle role information
    if let Some(ref role) = args.role {
        let mut role_mapping = Mapping::new();
        role_mapping.insert(Value::String("class".to_string()), Value::String("arc:role".to_string()));

        // Process role annotations
        let annotation_value = process_annotation_with_mapping(role, role_mapping.clone(), false).await?;
        role_mapping.extend(annotation_value);

        // Add the role to the performer info
        let has_role_key = Value::String("arc:has role".to_string());
        match performer_info.get_mut(&has_role_key) {
            Some(Value::Sequence(has_roles)) => has_roles.push(Value::Mapping(role_mapping)),
            _ => {
                performer_info.insert(has_role_key, Value::Sequence(vec![Value::Mapping(role_mapping)]));
            }
        }
    }

    // Add or update the performer in the "arc:performer" field
    let performer_key = Value::String("arc:performer".to_string());
    match mapping.get_mut(&performer_key) {
        Some(Value::Sequence(performers)) => {
            // Check if a performer with the same first and last name exists
            let mut existing_index = None;
            for (idx, performer) in performers.iter().enumerate() {
                if let Value::Mapping(existing_performer) = performer {
                    let first_name_match = existing_performer.get(Value::String("arc:first name".to_string()))
                        == Some(&Value::String(args.first_name.clone().unwrap_or_default()));
                    let last_name_match = existing_performer.get(Value::String("arc:last name".to_string()))
                        == Some(&Value::String(args.last_name.clone().unwrap_or_default()));
                    if first_name_match && last_name_match {
                        existing_index = Some(idx);
                        break;
                    }
                }
            }

            if let Some(idx) = existing_index {
                let existing_performer = &performers[idx];
                // Compare existing performer with new performer_info
                let is_same = if let Value::Mapping(existing_map) = existing_performer {
                    *existing_map == performer_info
                } else {
                    false
                };

                if is_same {
                    // Information is identical, nothing to do
                    return Ok(());
                } else {
                    // Display existing performer information before asking to extend
                    eprintln!("A performer with the same first and last name exists:");
                    if let Ok(yaml_str) = serde_yaml::to_string(existing_performer) {
                        highlight_cwl(&yaml_str);
                    }
                    let extend = Confirm::new()
                        .with_prompt("Do you want to update this performer with new information?")
                        .interact()
                        .unwrap_or(false);
                    if extend {
                        if let Value::Mapping(existing_performer) = &mut performers[idx] {
                            // Extend the existing performer with new fields from performer_info
                            for (k, v) in performer_info {
                                existing_performer.insert(k, v);
                            }
                        }
                    } else {
                        // Add as a new performer
                        performers.push(Value::Mapping(performer_info));
                    }
                }
            } else {
                // No matching performer, add as new
                performers.push(Value::Mapping(performer_info));
            }
        }
        _ => {
            // Initialize "arc:performer" as a sequence if it doesn't exist
            mapping.insert(performer_key, Value::Sequence(vec![Value::Mapping(performer_info)]));
        }
    }

    // Write the updated YAML back to the CWL file
    write_updated_yaml(&args.cwl_name, &yaml)
}

pub fn annotate_default(tool_name: &str) -> Result<(), Box<dyn Error>> {
    annotate(tool_name, "$namespaces", Some("s"), Some(SCHEMAORG_NAMESPACE))?;
    annotate(tool_name, "$schemas", None, Some(SCHEMAORG_SCHEMA))?;
    annotate(tool_name, "$namespaces", Some("arc"), Some(ARC_NAMESPACE))?;
    annotate(tool_name, "$schemas", None, Some(ARC_SCHEMA))?;
    let filename = get_filename(tool_name)?;

    if contains_docker_requirement(&filename)? {
        annotate_container(tool_name, "Docker Container")?;
    }
    Ok(())
}

pub fn annotate_container(cwl_name: &str, container_value: &str) -> Result<(), Box<dyn Error>> {
    annotate(cwl_name, "$schemas", None, Some(ARC_SCHEMA))?;
    annotate(cwl_name, "$namespaces", Some("arc"), Some(ARC_NAMESPACE))?;
    // Prepare the container information
    let mut container_info = Mapping::new();
    container_info.insert(Value::String("class".to_string()), Value::String("arc:technology type".to_string()));
    container_info.insert(
        Value::String("arc:annotation value".to_string()),
        Value::String(container_value.to_string()),
    );

    let yaml_result = parse_cwl(cwl_name)?;
    let mut yaml = yaml_result;

    if let Value::Mapping(mapping) = &mut yaml {
        if let Some(Value::Sequence(container)) = mapping.get_mut("arc:has technology type") {
            // Check if the container_info already exists in the sequence
            let container_exists = container.iter().any(|existing| {
                if let Value::Mapping(existing_map) = existing {
                    return existing_map == &container_info;
                }
                false
            });

            // Add container_info only if it doesn't already exist
            if !container_exists {
                container.push(Value::Mapping(container_info));
            }
        } else {
            // If `arc:has technology type` doesn't exist, create it and add the container info
            let containers = vec![Value::Mapping(container_info)];
            mapping.insert(Value::String("arc:has technology type".to_string()), Value::Sequence(containers));
        }
    } else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    }

    write_updated_yaml(cwl_name, &yaml)
}

pub fn annotate(name: &str, namespace_key: &str, key: Option<&str>, value: Option<&str>) -> Result<(), Box<dyn Error>> {
    let mut yaml = parse_cwl(name)?;
    if let Value::Mapping(mapping) = &mut yaml {
        match mapping.get_mut(namespace_key) {
            // Handle case where the namespace key exists as a sequence
            Some(Value::Sequence(sequence)) if key.is_none() && value.is_none() => {
                if let Some(namespace) = key {
                    // Add to sequence if not already present
                    if !sequence.iter().any(|x| matches!(x, Value::String(s) if s == namespace)) {
                        sequence.push(Value::String(namespace.to_string()));
                    }
                }
            }
            // Handle case where the namespace key exists as a mapping
            Some(Value::Mapping(namespaces)) => {
                if let (Some(key), Some(value)) = (key, value)
                    && !namespaces.contains_key(Value::String(key.to_string()))
                {
                    namespaces.insert(Value::String(key.to_string()), Value::String(value.to_string()));
                }
            }
            // Handle case where the namespace key does not exist
            _ => {
                if let (Some(key), Some(value)) = (key, value) {
                    let mut namespaces = Mapping::new();
                    namespaces.insert(Value::String(key.to_string()), Value::String(value.to_string()));
                    mapping.insert(Value::String(namespace_key.to_string()), Value::Mapping(namespaces.clone()));
                } else if let Some(namespace) = key {
                    let sequence = vec![Value::String(namespace.to_string())];
                    mapping.insert(Value::String(namespace_key.to_string()), Value::Sequence(sequence.clone()));
                } else if let Some(value) = value {
                    if let Some(Value::Sequence(schemas)) = mapping.get_mut(namespace_key) {
                        // Check if the schema URL is already in the list
                        if !schemas.iter().any(|x| matches!(x, Value::String(s) if s == value)) {
                            // If not, add the new schema to the sequence
                            schemas.push(Value::String(value.to_string()));
                        }
                    } else {
                        let schemas = vec![Value::String(value.to_string())];
                        mapping.insert(Value::String(namespace_key.to_string()), Value::Sequence(schemas));
                    }
                }
            }
        }
    }
    write_updated_yaml(name, &yaml)
}

pub fn annotate_person(args: &PersonArgs, role: &str) -> Result<(), Box<dyn Error>> {
    // part of schema.org annotation, ensure it is present
    annotate(&args.cwl_name, "$namespaces", Some("s"), Some(SCHEMAORG_NAMESPACE))?;
    annotate(&args.cwl_name, "$schemas", None, Some(SCHEMAORG_SCHEMA))?;

    let yaml_result = parse_cwl(&args.cwl_name)?;
    let mut yaml = yaml_result;

    if let Value::Mapping(ref mut mapping) = yaml {
        let mut person_info = Mapping::new();
        person_info.insert(Value::String("class".to_string()), Value::String("s:Person".to_string()));

        if let Some(ref person_id) = args.id {
            person_info.insert(Value::String("s:identifier".to_string()), Value::String(person_id.clone()));
        }

        if let Some(ref person_mail) = args.mail {
            person_info.insert(Value::String("s:email".to_string()), Value::String(format!("mailto:{person_mail}")));
        }

        person_info.insert(Value::String("s:name".to_string()), Value::String(args.name.clone()));

        // select the role (either 's:author' or 's:contributor')
        let role_key = match role {
            "author" => "s:author",
            "contributor" => "s:contributor",
            _ => return Err("Role must be either 'author' or 'contributor'.".into()),
        };

        // Check if the selected role (author or contributor) exists and is a sequence, then add new person
        if let Some(Value::Sequence(persons)) = mapping.get_mut(role_key) {
            // Check if the person already exists
            let person_exists = persons.iter().any(|person| {
                if let Value::Mapping(existing_person) = person
                    && let Some(Value::String(id)) = existing_person.get(Value::String("s:identifier".to_string()))
                {
                    return id == &args.id.clone().unwrap_or_default();
                }
                false
            });

            // If the person doesn't exist, add it to the sequence
            if !person_exists {
                persons.push(Value::Mapping(person_info));
            }
        } else {
            // If the role doesn't exist (author or contributor), create it with the new person information
            let persons = vec![Value::Mapping(person_info)];
            mapping.insert(Value::String(role_key.to_string()), Value::Sequence(persons));
        }
    } else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    }

    write_updated_yaml(&args.cwl_name, &yaml)
}

/// Helper function to write updated YAML to a file.
pub fn write_updated_yaml(name: &str, yaml: &Value) -> Result<(), Box<dyn Error>> {
    let path = get_filename(name)?;

    // Convert the YAML content to a string and write it to the file
    let yaml_str = serde_yaml::to_string(&yaml).map_err(|e| format!("Failed to serialize YAML: {e}"))?;
    let formatted_yaml = format_cwl(&yaml_str)?;
    File::create(&path)
        .and_then(|mut file| file.write_all(formatted_yaml.as_bytes()))
        .map_err(|e| format!("Failed to write to file '{path}': {e}"))?;

    Ok(())
}

pub fn annotate_field(cwl_name: &str, field: &str, value: &str) -> Result<(), Box<dyn Error>> {
    let mut yaml = parse_cwl(cwl_name)?;

    if let Value::Mapping(ref mut mapping) = yaml {
        // Check if the field is already present for fields like `s:license`
        if let Some(existing_value) = mapping.get(Value::String(field.to_string()))
            && existing_value == &Value::String(value.to_string())
        {
            eprintln!("Field '{field}' already has the value '{value}'.");
            return Ok(());
        }

        // Add or update the field
        mapping.insert(Value::String(field.to_string()), Value::String(value.to_string()));
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "CWL file is not a valid mapping.",
        )));
    }

    write_updated_yaml(cwl_name, &yaml)
}

pub fn parse_cwl(name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let path = Path::new(name);

    // Check if the provided name is an absolute path and the file exists
    let file_path = if path.is_absolute() && path.exists() {
        path.to_path_buf()
    } else {
        // Attempt to resolve the file in other locations
        let filename = get_filename(name)?;
        PathBuf::from(filename)
    };

    // Read and parse the file content
    let content = fs::read_to_string(&file_path)?;
    let yaml: Value = serde_yaml::from_str(&content)?;
    Ok(yaml)
}

pub fn get_filename(name: &str) -> Result<String, Box<dyn Error>> {
    // Ensure the filename ends with `.cwl`
    let filename = if is_cwl_file(name) { name.to_string() } else { format!("{name}.cwl") };

    // Get the current working directory
    let current_dir = env::current_dir()?;
    let current_path = current_dir.join(&filename);

    // Extract the base name
    let base_name = current_path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("").to_string();

    // Construct the path to the workflows directory
    let workflows_path = current_dir.join(Path::new("workflows").join(&base_name).join(&filename));

    // Check file existence in the current directory or workflows directory
    let file_path = if current_path.is_file() {
        current_path
    } else if workflows_path.is_file() {
        workflows_path
    } else {
        return Err(format!("CWL file '{filename}' not found in current directory or workflows/{base_name}/{filename}").into());
    };

    Ok(file_path.canonicalize()?.to_string_lossy().to_string())
}

pub fn contains_docker_requirement(file_path: &str) -> Result<bool, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        if line?.contains("DockerRequirement") {
            return Ok(true);
        }
    }

    Ok(false)
}

pub async fn annotate_process_step(args: &AnnotateProcessArgs) -> Result<(), Box<dyn Error>> {
    // Ensure ARC namespace and schema are defined
    annotate(&args.cwl_name, "$schemas", None, Some(ARC_SCHEMA))?;
    annotate(&args.cwl_name, "$namespaces", Some("arc"), Some(ARC_NAMESPACE))?;
    // Read and parse the existing CWL file
    let yaml_result = parse_cwl(&args.cwl_name)?;
    let mut yaml = yaml_result;

    if let Value::Mapping(ref mut mapping) = yaml {
        // Create a process sequence if it doesn't exist
        if mapping.contains_key(Value::String("arc:has process sequence".to_string())) {
            eprintln!("Process sequence already exists");
        } else {
            let mut process_sequence = Mapping::new();
            process_sequence.insert(Value::String("class".to_string()), Value::String("arc:process sequence".to_string()));
            process_sequence.insert(Value::String("arc:name".to_string()), Value::String(args.name.clone()));

            // Add inputs
            if let Some(ref input) = args.input {
                let mut input_data = Mapping::new();
                input_data.insert(Value::String("class".to_string()), Value::String("arc:data".to_string()));
                input_data.insert(Value::String("arc:name".to_string()), Value::String(input.clone()));

                process_sequence.insert(
                    Value::String("arc:has input".to_string()),
                    Value::Sequence(vec![Value::Mapping(input_data)]),
                );
            }

            // Add outputs
            if let Some(ref output) = args.output {
                let mut output_data = Mapping::new();
                output_data.insert(Value::String("class".to_string()), Value::String("arc:data".to_string()));
                output_data.insert(Value::String("arc:name".to_string()), Value::String(output.clone()));

                process_sequence.insert(
                    Value::String("arc:has output".to_string()),
                    Value::Sequence(vec![Value::Mapping(output_data)]),
                );
            }

            // Add parameters
            if let Some(ref parameter) = args.parameter {
                let mut parameter_value = Mapping::new();
                parameter_value.insert(
                    Value::String("class".to_string()),
                    Value::String("arc:process parameter value".to_string()),
                );

                let mut protocol_parameter = Mapping::new();
                protocol_parameter.insert(Value::String("class".to_string()), Value::String("arc:protocol parameter".to_string()));

                let mut parameter_name = Mapping::new();
                parameter_name.insert(Value::String("class".to_string()), Value::String("arc:parameter name".to_string()));

                let annotation_value = process_annotation_with_mapping(parameter, parameter_name.clone(), true).await?;
                parameter_name.extend(annotation_value);
                protocol_parameter.insert(
                    Value::String("arc:has parameter name".to_string()),
                    Value::Sequence(vec![Value::Mapping(parameter_name)]),
                );

                parameter_value.insert(
                    Value::String("arc:has parameter".to_string()),
                    Value::Sequence(vec![Value::Mapping(protocol_parameter)]),
                );

                // Add value if present
                if let Some(ref value) = args.value {
                    let mut value_name = Mapping::new();
                    value_name.insert(Value::String("class".to_string()), Value::String("arc:ontology annotation".to_string()));
                    let annotation_value = process_annotation_with_mapping(value, value_name.clone(), true).await?;
                    value_name.extend(annotation_value);

                    parameter_value.insert(Value::String("arc:value".to_string()), Value::Sequence(vec![Value::Mapping(value_name)]));
                }

                process_sequence.insert(
                    Value::String("arc:has parameter value".to_string()),
                    Value::Sequence(vec![Value::Mapping(parameter_value)]),
                );
            }

            // Add process sequence to the root mapping
            mapping.insert(
                Value::String("arc:has process sequence".to_string()),
                Value::Sequence(vec![Value::Mapping(process_sequence)]),
            );
        }
    }
    write_updated_yaml(&args.cwl_name, &yaml)
}

pub async fn process_annotation_with_mapping(value: &str, mut parameter_name: Mapping, complete: bool) -> Result<Mapping, Box<dyn Error>> {
    match ts_recommendations(value, MAX_RECOMMENDATIONS).await {
        Ok((annotation_value, source_ref, term_accession)) => {
            let mut annotation_mapping = Mapping::new();
            annotation_mapping.insert(Value::String("arc:term accession".to_string()), Value::String(term_accession));
            if complete {
                annotation_mapping.insert(Value::String("arc:term source REF".to_string()), Value::String(source_ref));
            }
            annotation_mapping.insert(Value::String("arc:annotation value".to_string()), Value::String(annotation_value));

            parameter_name.extend(annotation_mapping);
        }
        Err(e) => return Err(format!("Failed to process annotation value  {value}: {e}").into()),
    }

    Ok(parameter_name)
}

pub fn select_annotation(recommendations: &HashSet<(String, String, String)>, term: String) -> Result<(String, String, String), Box<dyn Error>> {
    eprintln!("{}", format!("Available annotations for '{term}':").green());

    // Collect elements into a vector for indexing
    let elements: Vec<&(String, String, String)> = recommendations.iter().collect();

    // Determine column widths
    let max_label_width = elements.iter().map(|(label, _, _)| label.len()).max().unwrap_or(0);
    let max_ontology_width = elements.iter().map(|(_, ontology, _)| ontology.len()).max().unwrap_or(0);
    let max_id_width = elements.iter().map(|(_, _, id)| id.len()).max().unwrap_or(0);

    // Create a vector of options for the menu
    let mut menu_options: Vec<String> = elements
        .iter()
        .map(|(label, ontology, id)| format!("{label: <max_label_width$} | {ontology: <max_ontology_width$} | {id: <max_id_width$}"))
        .collect();
    menu_options.push(format!("Do not use ontology, annotate '{term}'")); // Add skip option

    // Use dialoguer's Select to create a menu
    let selection = Select::new()
        .with_prompt("Use the arrow keys to navigate, Enter to select")
        .items(&menu_options)
        .default(0)
        .interact()?;

    // Process the selection
    if selection == elements.len() {
        // Skip option selected
        Ok((term, "N/A".to_string(), "N/A".to_string()))
    } else {
        // Return selected element
        Ok(elements[selection].clone())
    }
}

pub async fn ts_recommendations(search_term: &str, max_recommendations: usize) -> Result<(String, String, String), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let query = format!("{REST_URL_TS}{search_term}");
    // GET request
    let response = client.get(&query).send().await?;

    let ts_json: serde_json::Value = response.json().await?;

    let mut recommendations: HashSet<(String, String, String)> = HashSet::new();
    // Iterate over annotations
    if let Some(results) = ts_json.as_array() {
        for result in results {
            let id = result["iri"].as_str().unwrap_or("").trim_matches('"').to_string();
            let label = result["label"].as_str().unwrap_or("").trim_matches('"').to_string();
            let ontology = result["ontology"].as_str().unwrap_or("").trim_matches('"').to_string();
            if recommendations.len() < max_recommendations {
                recommendations.insert((label, ontology, id));
            }
        }
    } else {
        eprintln!("No valid annotations found.");
    }

    select_annotation(&recommendations, search_term.to_string())
}
