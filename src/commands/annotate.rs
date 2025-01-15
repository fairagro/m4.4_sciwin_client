use clap::{Args, Subcommand, ValueEnum};
use std::error::Error;
use serde_yml::{Value, Mapping};
use std::{fs, path::Path, env, io};
use std::fs::File;
use std::io::BufReader;
use std::io::BufRead;
use std::io::Write;
use std::collections::HashSet;
use reqwest::Client;
use serde_json::Value as jsonValue;
use colored::*;
use urlencoding::encode;

const REST_URL_BIOPORTAL: &str = "http://data.bioontology.org";

pub async fn handle_annotate_commands(command: &AnnotateCommands) -> Result<(), Box<dyn Error>> {
    match command {
        AnnotateCommands::Author(args) => annotate_author(args)?,
        AnnotateCommands::Performer(args) => annotate_performer(args)?,
        AnnotateCommands::Process(args) => annotate_process_step(args).await?,
        AnnotateCommands::Container { cwl_name, container } => {
            annotate_container(cwl_name, container)?
        }
        AnnotateCommands::Schema { cwl_name, schema } => {
            annotate(cwl_name, "$schemas", None, Some(schema))?
        }
        AnnotateCommands::Namespace { cwl_name, namespace, short } => {
            annotate(cwl_name, "$namespaces", short.as_deref(), Some(namespace))?
        }
        AnnotateCommands::Name { cwl_name, name } => annotate_field(cwl_name, "label", name)?,
        AnnotateCommands::Description { cwl_name, description } => {
            annotate_field(cwl_name, "doc", description)?
        }
        AnnotateCommands::License { cwl_name, license } => {
            annotate_field(cwl_name, "s:license", license)?
        }
        AnnotateCommands::Custom { cwl_name, field, value } => {
            annotate_field(cwl_name, field, value)?
        }
    }
    Ok(())
}

/// Enum for annotate-related subcommands
#[derive(Debug, Subcommand)]
pub enum AnnotateCommands {
    #[command(about = "Annotates author of a tool or workflow from schema.org")]
    Author(AuthorArgs),

    #[command(about = "Annotates performer of a tool or workflow from arc ontology")]
    Performer(PerformerArgs),

    #[command(about = "Annotates a process within a workflow")]
    Process(AnnotateProcessArgs),

    #[command(about = "Annotates container information of a tool or workflow")]
    Container {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(short = 'c', long = "container", help = "Annotation value for the container")]
        container: String,
    },

    #[command(about = "Annotates license of a tool or workflow")]
    License {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(help = "License to annotate")]
        license: String,
    },
    #[command(about = "Annotates schema of a tool or workflow")]
    Schema {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(help = "Schema to annotate")]
        schema: String,
    },
    #[command(about = "Annotates namespace of a tool or workflow")]
    Namespace {
        #[arg(help = "Name of the CWL file")]
        cwl_name: String,
        #[arg(help = "Namespace to annotate")]
        namespace: String,
        #[arg(help = "Namespace abbreviation to annotate")]
        short: Option<String>,
    },
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

/// Arguments for annotate author command
#[derive(Args, Debug)]
pub struct AuthorArgs {
    pub cwl_name: String,

    #[arg(short = 'n', long = "name", help = "Name of the author")]
    pub author_name: String,

    #[arg(short = 'm', long = "mail", help = "Email of the author")]
    pub author_mail: Option<String>,

    #[arg(short = 'i', long = "id", help = "Identifier of the author, e.g., ORCID")]
    pub author_id: Option<String>,
}

/// Arguments for annotate performer command
#[derive(Args, Debug)]
pub struct PerformerArgs {
    pub cwl_name: String,

    #[arg(short = 'f', long = "first_name", help = "First name of the performer")]
    pub first_name: String,

    #[arg(short = 'l', long = "last_name", help = "Last name of the performer")]
    pub last_name: String,

    #[arg(short = 'm', long = "mail", help = "Email of the performer")]
    pub mail: Option<String>,

    #[arg(short = 'a', long = "affiliation", help = "Affiliation of the performer")]
    pub affiliation: Option<String>,
}


/// Arguments for annotate process command
#[derive(Args, Debug)]
pub struct AnnotateProcessArgs {
    #[arg(help = "Name of the workflow process being annotated")]
    pub cwl_name: String,

    #[arg(short = 'n', long = "name", help = "Name of the process sequence step")]
    pub name: Option<String>,

    #[arg(short = 'i', long = "input", help = "Input file or directory, e.g., folder/input.txt")]
    pub input: Option<String>,

    #[arg(short = 'o', long = "output", help = "Output file or directory, e.g., folder/output.txt")]
    pub output: Option<String>,

    #[arg(short = 'p', long = "parameter", help = "Process step parameter")]
    pub parameter: Option<String>,

    #[arg(short = 'v', long = "value", help = "Process step value")]
    pub value: Option<String>,

    #[arg(short = 'm', long = "mapper", value_enum, default_value_t = OntologyMapper::default(), help = "Ontology mapping service to use: zooma, biotools, or text2term")]
    pub mapper: OntologyMapper,

    #[arg(short = 'k', long = "key", help = "Bioportal API key")]
    pub key: Option<String>,
}



#[derive(ValueEnum, Clone, Debug, Default)]
pub enum OntologyMapper {
    #[default]
    Zooma,
    //better? but requires API key
    Bioportal,
}

pub async fn process_annotation(args: &AnnotateProcessArgs, term: &str) -> Result<(String, String, String), Box<dyn Error>> {
    let max_recommendations: usize = 10; 
    match args.mapper {
        OntologyMapper::Zooma => {
            match zooma_recommendations(term, max_recommendations).await {
                Ok(recommendations) => Ok(recommendations),
                Err(e) => {
                    eprintln!(
                        "Error in Zoma recommendation process for term '{}': {}",
                        term, e
                    );
                    Err(e) 
                }
            }
        }
        OntologyMapper::Bioportal => {
            let bioportal_key: &str = &args.key.clone().unwrap_or_default(); 
            
            match bioportal_recommendations(term, bioportal_key, max_recommendations).await {
                Ok(recommendations) => Ok(recommendations),
                Err(e) => {
                    eprintln!(
                        "Error in Bioportal recommendation process for term '{}': {}",
                        term, e
                    );
                    Err(e) 
                }
            }
        }
    }
}


pub fn annotate_default(tool_name: &str) -> Result<(), Box<dyn Error>> {
    annotate(tool_name, "$namespaces", Some("s"), Some("https://schema.org/"))?;
    annotate(tool_name, "$schemas", None, Some("https://schema.org/version/latest/schemaorg-current-https.rdf"))?;
    annotate(tool_name, "$namespaces", Some("arc"), Some("https://github.com/nfdi4plants/ARC_ontology"))?;
    annotate(tool_name, "$schemas", None, Some("https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl"))?;
    let filename = get_filename(tool_name)?;

    if contains_docker_requirement(&filename)?{
        annotate_container(tool_name, "Docker Container")?;
    }
    Ok(())
}


pub fn annotate_container(cwl_name: &str, container_value: &str) -> Result<(), Box<dyn Error>> {

    // Prepare the container information
    let mut container_info = Mapping::new();
    container_info.insert(Value::String("class".to_string()), Value::String("arc:technology type".to_string()));
    container_info.insert(Value::String("arc:annotation value".to_string()), Value::String(container_value.to_string()));

    let yaml_result = parse_cwl(cwl_name)?;
    let mut yaml = yaml_result; 

    if let Value::Mapping(ref mut mapping) = yaml {
        if let Some(Value::Sequence(ref mut container)) = mapping.get_mut("arc:has technology type") {
            // Check if the container_info already exists in the sequence
            let container_exists = container.iter().any(|existing| {
                if let Value::Mapping(ref existing_map) = existing {
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

pub fn annotate_author(args: &AuthorArgs) -> Result<(), Box<dyn Error>> {
    annotate(&args.cwl_name, "$namespaces", Some("s"), Some("https://schema.org/"))?;
    annotate(&args.cwl_name, "$schemas", None, Some("https://schema.org/version/latest/schemaorg-current-https.rdf"))?;

    let yaml_result = parse_cwl(&args.cwl_name)?; 
    let mut yaml = yaml_result; 

    if let Value::Mapping(ref mut mapping) = yaml {
        // Create the author_info mapping with required fields
        let mut author_info = Mapping::new();
        author_info.insert(Value::String("class".to_string()), Value::String("s:Person".to_string()));
        
        if let Some(ref author_id) = args.author_id {
            author_info.insert(Value::String("s:identifier".to_string()), Value::String(author_id.clone()));
        }

        // Only insert author_mail if it's Some
        if let Some(ref author_mail) = args.author_mail {
            author_info.insert(Value::String("s:email".to_string()), Value::String(format!("mailto:{}", author_mail)));
        }

        author_info.insert(Value::String("s:name".to_string()), Value::String(args.author_name.clone()));

        // Check if "s:author" exists and is a sequence, then add new author
        if let Some(Value::Sequence(ref mut authors)) = mapping.get_mut("s:author") {
            // Check if the author already exists by matching the identifier or name
            let author_exists = authors.iter().any(|author| {
                if let Value::Mapping(ref existing_author) = author {
                    if let Some(Value::String(ref id)) = existing_author.get(Value::String("s:identifier".to_string())) {
                        return id == &args.author_id.clone().unwrap_or_default();
                    }
                }
                false
            });

            // If the author doesn't exist, add it to the sequence
            if !author_exists {
                authors.push(Value::Mapping(author_info));
            }
        } else {
            // If 's:author' doesn't exist, create it with the new author information
            let authors = vec![Value::Mapping(author_info)];
            mapping.insert(Value::String("s:author".to_string()), Value::Sequence(authors));
        }
    } else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    }

    write_updated_yaml(&args.cwl_name, &yaml)
}


pub fn annotate_performer(args: &PerformerArgs) -> Result<(), Box<dyn Error>> {
    annotate(&args.cwl_name, "$schemas", None, Some("https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl"))?;
    annotate(&args.cwl_name, "$namespaces", Some("arc"), Some("https://github.com/nfdi4plants/ARC_ontology"))?;
    // Read the existing CWL file

    let yaml_result = parse_cwl(&args.cwl_name)?; 
    let mut yaml = yaml_result; 

    // Ensure the root is a mapping
    if let Value::Mapping(ref mut mapping) = yaml {
        // Prepare the performer information as a mapping
        let mut performer_info = Mapping::new();
        performer_info.insert(Value::String("class".to_string()), Value::String("arc:Person".to_string()));
        performer_info.insert(Value::String("arc:first name".to_string()), Value::String(args.first_name.clone()));
        performer_info.insert(Value::String("arc:last name".to_string()), Value::String(args.last_name.clone()));

        // Only insert mail if it is Some
        if let Some(ref mail) = args.mail {
            performer_info.insert(Value::String("arc:email".to_string()), Value::String(mail.clone()));
        }

        // Only insert affiliation if it is Some
        if let Some(ref affiliation) = args.affiliation {
            performer_info.insert(Value::String("arc:affiliation".to_string()), Value::String(affiliation.clone()));
        }

        if let Some(Value::Sequence(ref mut performers)) = mapping.get_mut("arc:performer") {
            // Check if the performer already exists by comparing all fields
            let performer_exists = performers.iter().any(|performer| {
                if let Value::Mapping(ref existing_performer) = performer {
                    let first_name_match = existing_performer.get(Value::String("arc:first name".to_string()))
                        == Some(&Value::String(args.first_name.clone()));
                    let last_name_match = existing_performer.get(Value::String("arc:last name".to_string()))
                        == Some(&Value::String(args.last_name.clone()));
                    let email_match = if let Some(ref mail) = args.mail {
                        existing_performer.get(Value::String("arc:email".to_string())) == Some(&Value::String(mail.clone()))
                    } else {
                        true // If email is None, consider as a match
                    };
                    let affiliation_match = if let Some(ref affiliation) = args.affiliation {
                        existing_performer.get(Value::String("arc:affiliation".to_string())) == Some(&Value::String(affiliation.clone()))
                    } else {
                        true // If affiliation is None, consider as a match
                    };
                
                    return first_name_match && last_name_match && email_match && affiliation_match;             
                }
                false
            });

            // If the performer doesn't exist, add it to the sequence
            if !performer_exists {
                performers.push(Value::Mapping(performer_info));
            }
        } else {
            // If 'arc:performer' doesn't exist, create it with the new performer information
            let performers = vec![Value::Mapping(performer_info)];
            mapping.insert(Value::String("arc:performer".to_string()), Value::Sequence(performers));
        }
    } else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    }

    write_updated_yaml(&args.cwl_name, &yaml)
}


pub fn annotate(
    name: &str,
    namespace_key: &str,
    key: Option<&str>,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let mut yaml = parse_cwl(name)?;
    if let Value::Mapping(ref mut mapping) = yaml {
        match mapping.get_mut(namespace_key) { 
            // Handle case where the namespace key exists as a sequence
            Some(Value::Sequence(ref mut sequence)) if key.is_none() && value.is_none() => {
                if let Some(namespace) = key {
                    // Add to sequence if not already present
                    if !sequence.iter().any(|x| matches!(x, Value::String(s) if s == namespace)) {
                        sequence.push(Value::String(namespace.to_string()));
                    }
                }
            }
            // Handle case where the namespace key exists as a mapping
            Some(Value::Mapping(ref mut namespaces)) => {
                if let (Some(key), Some(value)) = (key, value) {
                    if !namespaces.contains_key(Value::String(key.to_string())) {
                        namespaces.insert(
                            Value::String(key.to_string()),
                            Value::String(value.to_string()),
                        );
                    }
                }
            }
            // Handle case where the namespace key does not exist
            _ => {
                if let (Some(key), Some(value)) = (key, value) {
                    let mut namespaces = Mapping::new();
                    namespaces.insert(
                        Value::String(key.to_string()),
                        Value::String(value.to_string()),
                    );
                    mapping.insert(
                        Value::String(namespace_key.to_string()),
                        Value::Mapping(namespaces.clone()),
                    );
                } else if let Some(namespace) = key {
                    let sequence = vec![Value::String(namespace.to_string())];
                    mapping.insert(
                        Value::String(namespace_key.to_string()),
                        Value::Sequence(sequence.clone()),
                    );
                }
                else if let Some(value) = value {
                    if let Some(Value::Sequence(ref mut schemas)) = mapping.get_mut(namespace_key) {
                        // Check if the schema URL is already in the list
                        if !schemas.iter().any(|x| matches!(x, Value::String(s) if s == value)) {
                            // If not, add the new schema to the sequence
                            schemas.push(Value::String(value.to_string()));
                        }
                    } else {
                        let schemas= vec![Value::String(value.to_string())];
                        mapping.insert(Value::String(namespace_key.to_string()), Value::Sequence(schemas));
                    }
                }
            }
        }
    }
    write_updated_yaml(name, &yaml)
}

/// Helper function to write updated YAML to a file.
fn write_updated_yaml(name: &str, yaml: &Value) -> Result<(), Box<dyn Error>> {
    let path = get_filename(name)?;

    // Convert the YAML content to a string and write it to the file
    let yaml_str = serde_yml::to_string(&yaml)
        .map_err(|e| format!("Failed to serialize YAML: {}", e))?;
    File::create(&path)
        .and_then(|mut file| file.write_all(yaml_str.as_bytes()))
        .map_err(|e| format!("Failed to write to file '{}': {}", path, e))?;

    Ok(())
}

pub fn annotate_field(cwl_name: &str, field: &str, value: &str) -> Result<(), Box<dyn Error>> {
    if field == "s:license" {
        annotate(cwl_name, "$namespaces", Some("s"), Some("https://schema.org/"))?;
        annotate(cwl_name, "$schemas", None, Some("https://schema.org/version/latest/schemaorg-current-https.rdf"))?;
    }
    let mut yaml = parse_cwl(cwl_name)?;

    if let Value::Mapping(ref mut mapping) = yaml {
        // Check if the field is already present for fields like `s:license`
        if let Some(existing_value) = mapping.get(Value::String(field.to_string())) {
            if existing_value == &Value::String(value.to_string()) {
                println!("Field '{}' already has the value '{}'.", field, value);
                return Ok(());
            }
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

fn parse_cwl(name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    // Check if 'name' ends with ".cwl" and append if necessary
    let filename = if name.ends_with(".cwl") {
        name.to_string()
    } else {
        format!("{}.cwl", name)
    };

    let cwl_name = Path::new(&filename).extension().unwrap();
    // Define the paths to check
    let current_dir = env::current_dir()?;
    let current_path = current_dir.join(cwl_name);
    let workflows_path = current_dir.join(format!("workflows/{}/{}", name, filename));

    // Attempt to read the file from the current directory
    let file_path = if current_path.exists() {
        current_path
    } else if workflows_path.exists() {
        workflows_path
    } else {
        return Err(format!("CWL file '{}' not found in current directory or workflows/{}/{}", filename, name, filename).into());
    };

    // Read the file content
    let content = fs::read_to_string(file_path)?;

    // Parse the YAML content
    let yaml: Value = serde_yml::from_str(&content)?;

    Ok(yaml)
}

fn get_filename(name: &str) -> Result<String, Box<dyn Error>> {
    // Check if 'name' ends with ".cwl" and append if necessary
    let filename = if name.ends_with(".cwl") {
        name.to_string()
    } else {
        format!("{}.cwl", name)
    };

    // Define the paths to check
    let current_dir = env::current_dir()?;
    let current_path = current_dir.join(&filename);
    let workflows_path = current_dir.join(format!("workflows/{}/{}", name, filename));

    // Attempt to find the file in the current directory or workflows directory
    let file_path = if current_path.exists() {
        current_path
    } else if workflows_path.exists() {
        workflows_path
    } else {
        return Err(format!(
            "CWL file '{}' not found in current directory or workflows/{}/{}",
            filename, name, filename
        ).into());
    };

    // Return the file path as a string
    Ok(file_path.display().to_string())
}


fn contains_docker_requirement(file_path: &str) -> Result<bool, Box<dyn Error>> {
    // Open the file in read-only mode
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    // Iterate over each line in the file
    for line in reader.lines() {
        // Check if the line contains "DockerRequirement"
        if line?.contains("DockerRequirement") {
            return Ok(true);
        }
    }

    Ok(false)
}

pub async fn annotate_process_step(args: &AnnotateProcessArgs) -> Result<(), Box<dyn Error>> {
    // Read and parse the existing CWL file
    let yaml_result = parse_cwl(&args.cwl_name)?;
    let mut yaml = yaml_result;

    if let Value::Mapping(ref mut mapping) = yaml {
        // Create a process sequence if it doesn't exist
        if !mapping.contains_key(Value::String("arc:has process sequence".to_string())) {
            let mut process_sequence = Mapping::new();
            process_sequence.insert(
                Value::String("class".to_string()),
                Value::String("arc:process sequence".to_string()),
            );

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
                protocol_parameter.insert(
                    Value::String("class".to_string()),
                    Value::String("arc:protocol parameter".to_string()),
                );

                let mut parameter_name = Mapping::new();
                parameter_name.insert(
                    Value::String("class".to_string()),
                    Value::String("arc:parameter name".to_string()),
                );

                match process_annotation(args, parameter).await {
                    Ok(recommendations) => {
                        parameter_name.insert(
                            Value::String("arc:term accession".to_string()),
                            Value::String(recommendations.2),
                        );
                        parameter_name.insert(
                            Value::String("arc:term source REF".to_string()),
                            Value::String(recommendations.1),
                        );
                        parameter_name.insert(
                            Value::String("arc:annotation value".to_string()),
                            Value::String(recommendations.0),
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to process annotation for parameter '{}': {}", parameter, e);
                    }
                }

                protocol_parameter.insert(
                    Value::String("arc:has parameter name".to_string()),
                    Value::Sequence(vec![Value::Mapping(parameter_name)]),
                );
                parameter_value.insert(
                    Value::String("arc:has parameter".to_string()),
                    Value::Sequence(vec![Value::Mapping(protocol_parameter)]),
                );

                if let Some(ref value) = args.value {
                    parameter_value.insert(Value::String("arc:value".to_string()), Value::String(value.clone()));
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
        } else {
            //allow multiple?
            println!("Process sequence already exists");
        }
    } else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    }
    write_updated_yaml(&args.cwl_name, &yaml)

}



async fn get_json_biotools(url: &str, client: &Client, biotools_key: &str) -> Result<jsonValue, Box<dyn Error>> {
    let response = client.get(url)
        .header("Authorization", format!("apikey token={}", biotools_key)) // Replace with your API key
        .send()
        .await?
        .json::<jsonValue>()
        .await?;

    Ok(response)
}


fn select_annotation(
    recommendations: &HashSet<(String, String, String)>,
    term: String,
) -> Result<(String, String, String), Box<dyn Error>> {
    println!("{}", format!("Available annotations for '{}':", term).green());

    // Collect elements into a vector for indexing
    let elements: Vec<&(String, String, String)> = recommendations.iter().collect();

     // Add an option to skip annotation
     println!("{:<4} {}", "0".yellow(),format!("Do not use ontology, annotate '{}'", term).yellow());

    for (index, (label, ontology, id)) in elements.iter().enumerate() {
        println!(
            "{:<4} {:<30} {:<60} {:<50}",
            index + 1,
            label,
            id,
            ontology
        );
    }

    println!("==================================");
    print!("{}", "Enter the number of your choice: ".green());
    io::stdout().flush()?; 

    let mut user_input = String::new();
    io::stdin()
        .read_line(&mut user_input)
        .expect("Failed to read input");

    // Parse the user input
    match user_input.trim().parse::<usize>() {
        Ok(0) => {
            // Return a default value
            Ok((
                term,
                "N/A".to_string(),
                "N/A".to_string(),
            ))
        }
        Ok(choice) if choice > 0 && choice <= elements.len() => {
            Ok(elements[choice - 1].clone())
        }
        _ => {
            println!("\n{}", "Invalid choice. Please try again.".red());
            Err("Invalid choice".into())
        }
    }
}

async fn bioportal_recommendations(
    search_term: &str,
    biotools_key: &str,
    max_recommendations: usize,
) -> Result<(String, String, String), Box<dyn Error>> {
    let client = Client::new();
    let annotations = get_json_biotools(
        //&format!("{}/recommender?input={}", REST_URL, urlencoding::encode(search_term)),
        &format!("{}/annotator?text={}", REST_URL_BIOPORTAL, urlencoding::encode(search_term)),
        &client,
        biotools_key
    ).await?;

    let mut recommendations: HashSet<(String, String, String)> = HashSet::new();
    // Iterate over annotations
    if let Some(results) = annotations.as_array() {
        for result in results {
            let id = result["annotatedClass"]["@id"].as_str().unwrap_or("").trim_matches('"').to_string();
            let label = result["annotations"][0]["text"].as_str().unwrap_or("").trim_matches('"').to_string();
            let ontology_str = result["annotatedClass"]["links"]["ontology"].as_str().unwrap_or("").trim_matches('"');
            // Split and get the last part (ontology)
            let ontology = ontology_str.split('/').last().unwrap_or("").to_string();
            if recommendations.len() < max_recommendations {
                recommendations.insert((label, ontology, id)); 
            }
        }
    } else {
        println!("No valid annotations found.");
    }
    select_annotation(&recommendations, search_term.to_string())

}

async fn zooma_recommendations(
    search_term: &str,
    max_recommendations: usize,
) -> Result<(String, String, String), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let mut recommendations: HashSet<(String, String, String)> = HashSet::new();

    let zooma_base = "https://www.ebi.ac.uk/spot/zooma/v2/api/services/annotate?propertyValue=";

    // Replace spaces with "+" and URL encode the term
    let tt = search_term.replace(" ", "+");

    let query = format!("{}{}", zooma_base, encode(&tt));

    // Make the GET request
    let response = client.get(&query).send().await?;
    
    let zooma_json: serde_json::Value = response.json().await?;


    if let Some(json_array) = zooma_json.as_array() {
        for entry in json_array {

            let mut property_value = None;
            let mut source_name = None;
            let mut semantic_tag = None;
    
            if let Some(tag) = entry.get("semanticTags")
                .and_then(|tags| tags.as_array())
                .and_then(|tags| tags.first())
                .and_then(|tag| tag.as_str())
            {
                semantic_tag = Some(tag.to_string());
            }
    
            if let Some(value) = entry.get("annotatedProperty")
                .and_then(|prop| prop.get("propertyValue"))
                .and_then(|val| val.as_str())
            {
                property_value = Some(value.to_string());
            }
    
            if let Some(name) = entry.get("derivedFrom")
                .and_then(|prov| prov.get("provenance"))
                .and_then(|source| source.get("generator"))
                .and_then(|name| name.as_str())
            {
                source_name = Some(name.to_string());
            }
        
            if recommendations.len() < max_recommendations {
                if let (Some(property_value), Some(source_name), Some(semantic_tag)) =
                    (property_value, source_name, semantic_tag)
                {
                    recommendations.insert((property_value, source_name, semantic_tag));
                }
            }
        }
    }

    select_annotation(&recommendations, search_term.to_string())

}
