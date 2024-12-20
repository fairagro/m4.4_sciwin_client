use clap::{Args, Subcommand, ValueEnum};
use std::error::Error;
use serde_yml::{Value, Mapping};
use std::{fs, path::Path, env};
use serde_yml::Sequence;
use std::fs::File;
use std::io::BufReader;
use std::io::BufRead;
use std::io::Write;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;


// Handle annotate-related commands
pub fn handle_annotate_commands(command: &AnnotateCommands) -> Result<(), Box<dyn Error>> {
    match command {
        AnnotateCommands::Author(args) => annotate_author(args)?,
        AnnotateCommands::Performer(args) => annotate_performer(args)?,
        AnnotateCommands::Container(args) => annotate_container(args)?,
        AnnotateCommands::Process(args) => annotate_process_step(args)?,
        AnnotateCommands::Schema(args) => annotate_schema(args)?,
        //AnnotateCommands::ProcessStep(args) => annotate_process_step(args)?,
    }
    Ok(())
}

// Enum for annotate-related subcommands
#[derive(Debug, Subcommand)]
pub enum AnnotateCommands {
    #[command(about = "Annotates author of a tool or workflow from schema.org")]
    Author(AuthorArgs),
    #[command(about = "Annotates performer of a tool or workflow from arc ontology")]
    Performer(PerformerArgs),
    #[command(about = "Annotates performer of a tool or workflow from arc ontology")]
    Container(ContainerArgs),
    #[command(about = "Annotates a process within a workflow")]
    Process(AnnotateProcessArgs),
    #[command(about = "Annotates schema of a tool or workflow")]
    Schema(AnnotateSchemaArgs),
}

// Arguments for annotate command
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

// Arguments for performer command
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

#[derive(Args, Debug)]
pub struct ContainerArgs {
    pub cwl_name: String,
    #[arg(short = 'a', long = "annotation", help = "Annotation value for the container")]
    pub annotation_value: String,
}

// Arguments for annotate process command
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
    #[arg(short = 'k', long = "key", help = "Biotools API key")]
    pub key: Option<String>,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OntologyMapper {
    Zooma,
    Biotools,
    Text2term,
}

impl Default for OntologyMapper {
    fn default() -> Self {
        OntologyMapper::Biotools // Set your desired default here
    }
}

pub fn process_annotation(args: &AnnotateProcessArgs, term: &str) -> Result<(), Box<dyn Error>> {
    match args.mapper {
        OntologyMapper::Zooma => {
           // zooma_recommendations(/* parameters */)?;
        }
        OntologyMapper::Biotools => {
            let biotools_key: &str = &args.key.clone().unwrap_or_default(); 
            let max_recommendations: usize = 10; 
            let _recommendations = biotools_recommendations(term, biotools_key, max_recommendations); 
        }
        OntologyMapper::Text2term => {
            // Call the function that handles Text2Term recommendations
          //  text2term_recommendations(/* parameters */)?;
        }
    }
    Ok(())
}

#[derive(Args, Debug)]
pub struct AnnotateSchemaArgs {
    #[arg(help = "Name of the workflow process being annotated")]
    pub name: String,
    #[arg(short = 's', long = "schema", help = "Schema, e.g. https://schema.org/version/latest/schemaorg-current-https.rdf, https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl")]
    pub schema: String,

}

pub fn annotate_default(tool_name: &str) -> Result<(), Box<dyn Error>> {
    annotate_arc_namespace(tool_name)?;
    annotate_arc_schema(tool_name)?;
    annotate_schemaorg_namespace(tool_name)?;
    annotate_schemaorg_schema(tool_name)?;
    let filename = get_filename(tool_name)?;

    if contains_docker_requirement(&filename)?{
        let container_args = ContainerArgs {
            cwl_name: tool_name.to_string(),
            annotation_value: "Docker Container".to_string(),
        };
        annotate_container(&container_args)?;
    }
    Ok(())
}
pub fn annotate_container(args: &ContainerArgs) -> Result<(), Box<dyn Error>> {
    // Read the existing CWL file
    let yaml_result = parse_cwl(&args.cwl_name);

    // Handle the result from parse_cwl
    let mut yaml = match yaml_result {
        Ok(value) => value,
        Err(e) => return Err(format!("Failed to parse CWL file: {}", e).into()),
    };

    // Prepare the container information
    let mut container_info = Mapping::new();
    container_info.insert(Value::String("class".to_string()), Value::String("arc:technology type".to_string()));
    container_info.insert(Value::String("arc:annotation value".to_string()), Value::String(args.annotation_value.clone()));

    // Ensure the root is a mapping
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
                println!("Adding new container information: {:?}", container_info);
                container.push(Value::Mapping(container_info));
            }
        } else {
            // If `arc:has technology type` doesn't exist, create it and add the container info
            let mut containers = Sequence::new();
            containers.push(Value::Mapping(container_info));
            mapping.insert(Value::String("arc:has technology type".to_string()), Value::Sequence(containers));
        }
    } else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    }

    // Get the filename and write the updated YAML content to it
    let path = get_filename(&args.cwl_name)?;

    // Create the file at the specified path
    let mut file = File::create(path)?;

    // Convert the YAML content to a string and write it to the file
    let yaml_str = serde_yml::to_string(&yaml)?;
    file.write_all(yaml_str.as_bytes())?;

    Ok(())
}

pub fn annotate_author(args: &AuthorArgs) -> Result<(), Box<dyn Error>> {
    // Read the existing CWL file
    let yaml_result = parse_cwl(&args.cwl_name);

    // Handle the result from parse_cwl
    let mut yaml = match yaml_result {
        Ok(value) => value,
        Err(e) => return Err(format!("Failed to parse CWL file: {}", e).into()),
    };

    // Ensure the root is a mapping
    if let Value::Mapping(ref mut mapping) = yaml {
        // Create the author_info mapping with required fields
        let mut author_info = Mapping::new();
        author_info.insert(Value::String("class".to_string()), Value::String("s:Person".to_string()));
        
        // Only insert author_id if it's Some
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
                    if let Some(Value::String(ref id)) = existing_author.get(&Value::String("s:identifier".to_string())) {
                        return id == &args.author_id.clone().unwrap_or_default();
                    }
                }
                false
            });

            // If the author doesn't exist, add it to the sequence
            if !author_exists {
                println!("Adding new author: {:?}", author_info);
                authors.push(Value::Mapping(author_info));
            }
        } else {
            // If 's:author' doesn't exist, create it with the new author information
            let mut authors = Vec::new();
            authors.push(Value::Mapping(author_info));
            mapping.insert(Value::String("s:author".to_string()), Value::Sequence(authors));
        }
    } else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    }

    // Get the filename and write the updated YAML content to it
    let path = get_filename(&args.cwl_name)?;

    // Create the file at the specified path
    let mut file = File::create(path)?;

    // Convert the YAML content to a string and write it to the file
    let yaml_str = serde_yml::to_string(&yaml)?;
    file.write_all(yaml_str.as_bytes())?;

    Ok(())
}


pub fn annotate_performer(args: &PerformerArgs) -> Result<(), Box<dyn Error>> {
    // Read the existing CWL file
    let yaml_result = parse_cwl(&args.cwl_name);

    // Handle the result from parse_cwl
    let mut yaml = match yaml_result {
        Ok(value) => value,
        Err(e) => return Err(format!("Failed to parse CWL file: {}", e).into()),
    };

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
                    let first_name_match = existing_performer.get(&Value::String("arc:first name".to_string()))
                        == Some(&Value::String(args.first_name.clone()));
                    let last_name_match = existing_performer.get(&Value::String("arc:last name".to_string()))
                        == Some(&Value::String(args.last_name.clone()));
                    let email_match = if let Some(ref mail) = args.mail {
                        existing_performer.get(&Value::String("arc:email".to_string())) == Some(&Value::String(mail.clone()))
                    } else {
                        true // If email is None, consider as a match
                    };
                    let affiliation_match = if let Some(ref affiliation) = args.affiliation {
                        existing_performer.get(&Value::String("arc:affiliation".to_string())) == Some(&Value::String(affiliation.clone()))
                    } else {
                        true // If affiliation is None, consider as a match
                    };
                
                    return first_name_match && last_name_match && email_match && affiliation_match;             
                }
                false
            });

            // If the performer doesn't exist, add it to the sequence
            if !performer_exists {
                println!("Adding new performer: {:?}", performer_info);
                performers.push(Value::Mapping(performer_info));
            }
        } else {
            // If 'arc:performer' doesn't exist, create it with the new performer information
            let mut performers = Vec::new();
            performers.push(Value::Mapping(performer_info));
            mapping.insert(Value::String("arc:performer".to_string()), Value::Sequence(performers));
        }
    } else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    }

    // Get the filename for saving the updated CWL file
    let path = get_filename(&args.cwl_name)?;

    // Create or overwrite the file at the specified path
    let mut file = File::create(path)?;

    // Serialize the updated YAML back to a string and write it to the file
    let yaml_str = serde_yml::to_string(&yaml)?;
    file.write_all(yaml_str.as_bytes())?;

    Ok(())
}


    pub fn annotate_schema(args: &AnnotateSchemaArgs) -> Result<(), Box<dyn Error>> {
        println!("Annotating tool or workflow '{}'", args.name);
        println!("Schema: {}", args.schema);
    
        // Parse the CWL file into a YAML value
        let mut yaml = parse_cwl(&args.name)?;
    
        // Check if the YAML has a "$schemas" field
        if let Value::Mapping(ref mut mapping) = yaml {
            // Check if $schemas exists and is a Sequence
            if let Some(Value::Sequence(ref mut schemas)) = mapping.get_mut("$schemas") {
                // Check if the schema URL is already in the list
                if !schemas.iter().any(|x| matches!(x, Value::String(s) if s == &args.schema)) {
                    // If not, add the new schema to the sequence
                    schemas.push(Value::String(args.schema.to_string()));
                }
            } else {
                // If $schemas doesn't exist, create it and add the schema
                let mut schemas = Sequence::new();
                schemas.push(Value::String(args.schema.to_string()));
                mapping.insert(Value::String("$schemas".to_string()), Value::Sequence(schemas));
            }
    
            // If the schema contains "arc", ensure the "arc" namespace is present
            if args.schema.contains("ARC") {
                // Check if $namespaces exists and is a Mapping
                if let Some(Value::Mapping(ref mut namespaces)) = mapping.get_mut("$namespaces") {
                    if !namespaces.contains_key(&Value::String("arc".to_string())) {
                        namespaces.insert(
                            Value::String("arc".to_string()),
                            Value::String("https://github.com/nfdi4plants/ARC_ontology".to_string()),
                        );
                    }
                } else {
                    let mut namespaces = Mapping::new();
                    namespaces.insert(
                        Value::String("arc".to_string()),
                        Value::String("https://github.com/nfdi4plants/ARC_ontology".to_string()),
                    );
                    mapping.insert(Value::String("$namespaces".to_string()), Value::Mapping(namespaces));
                }
        }
    }
    
        // Get the filename to write the updated YAML
        let path = get_filename(&args.name)?;
    
        // Create the file at the specified path
        let mut file = File::create(path)?;
    
        // Convert the YAML content to a string and write it to the file
        let yaml_str = serde_yml::to_string(&yaml)?;
        file.write_all(yaml_str.as_bytes())?;
    
        Ok(())
    }


    pub fn annotate_schemaorg_schema(name: &str) -> Result<(), Box<dyn Error>> {
        println!("Annotating tool or workflow '{}'", name);
        let arc_schema = "https://schema.org/version/latest/schemaorg-current-https.rdf".to_string(); 
    
        // Parse the CWL file into a YAML value
        let mut yaml = parse_cwl(name)?;
    
        // Check if the YAML has a "$schemas" field
        if let Value::Mapping(ref mut mapping) = yaml {

            if let Some(Value::Sequence(ref mut schemas)) = mapping.get_mut("$schemas") {
                // Check if the schema URL is already in the list
                if !schemas.iter().any(|x| matches!(x, Value::String(s) if s == &arc_schema)) {
                    // If not, add the new schema to the sequence
                    schemas.push(Value::String(arc_schema));
                }
            } else {
                // If $schemas doesn't exist, create it and add the schema
                let mut schemas = Sequence::new();
                schemas.push(Value::String(arc_schema));
                mapping.insert(Value::String("$schemas".to_string()), Value::Sequence(schemas));
            }
                
        }
        // Get the filename to write the updated YAML
        let path = get_filename(name)?;
    
        // Create the file at the specified path
        let mut file = File::create(path)?;
    
        // Convert the YAML content to a string and write it to the file
        let yaml_str = serde_yml::to_string(&yaml)?;
        file.write_all(yaml_str.as_bytes())?;
    
        Ok(())
    }

    pub fn annotate_schemaorg_namespace(name: &str) -> Result<(), Box<dyn Error>> {
        println!("Annotating tool or workflow '{}'", name);
    
        // Parse the CWL file into a YAML value
        let mut yaml = parse_cwl(name)?;
    
        // Check if the YAML has a "$schemas" field
        if let Value::Mapping(ref mut mapping) = yaml {
            // Check if $schemas exists and is a Sequence
            if let Some(Value::Mapping(ref mut namespaces)) = mapping.get_mut("$namespaces") {
                if !namespaces.contains_key(&Value::String("schema.org".to_string())) {
                    namespaces.insert(
                        Value::String("s".to_string()),
                        Value::String("https://schema.org/".to_string()),
                    );
                }
            } else {
                let mut namespaces = Mapping::new();
                namespaces.insert(
                    Value::String("s".to_string()),
                    Value::String("https://schema.org/".to_string()),
                );
                mapping.insert(Value::String("$namespaces".to_string()), Value::Mapping(namespaces));
            }
        }
    
        // Get the filename to write the updated YAML
        let path = get_filename(name)?;
    
        // Create the file at the specified path
        let mut file = File::create(path)?;
    
        // Convert the YAML content to a string and write it to the file
        let yaml_str = serde_yml::to_string(&yaml)?;
        file.write_all(yaml_str.as_bytes())?;
    
        Ok(())
    }


    pub fn annotate_arc_schema(name: &str) -> Result<(), Box<dyn Error>> {
        println!("Annotating tool or workflow '{}'", name);
        let arc_schema = "https://raw.githubusercontent.com/nfdi4plants/ARC_ontology/main/ARC_v2.0.owl".to_string(); 
    
        // Parse the CWL file into a YAML value
        let mut yaml = parse_cwl(name)?;
    
        // Check if the YAML has a "$schemas" field
        if let Value::Mapping(ref mut mapping) = yaml {

            if let Some(Value::Sequence(ref mut schemas)) = mapping.get_mut("$schemas") {
                // Check if the schema URL is already in the list
                if !schemas.iter().any(|x| matches!(x, Value::String(s) if s == &arc_schema)) {
                    // If not, add the new schema to the sequence
                    schemas.push(Value::String(arc_schema));
                }
            } else {
                // If $schemas doesn't exist, create it and add the schema
                let mut schemas = Sequence::new();
                schemas.push(Value::String(arc_schema));
                mapping.insert(Value::String("$schemas".to_string()), Value::Sequence(schemas));
            }
                
        }
        // Get the filename to write the updated YAML
        let path = get_filename(name)?;
    
        // Create the file at the specified path
        let mut file = File::create(path)?;
    
        // Convert the YAML content to a string and write it to the file
        let yaml_str = serde_yml::to_string(&yaml)?;
        file.write_all(yaml_str.as_bytes())?;
    
        Ok(())
    }

    pub fn annotate_arc_namespace(name: &str) -> Result<(), Box<dyn Error>> {
        println!("Annotating tool or workflow '{}'", name);
    
        // Parse the CWL file into a YAML value
        let mut yaml = parse_cwl(name)?;
    
        // Check if the YAML has a "$schemas" field
        if let Value::Mapping(ref mut mapping) = yaml {
            // Check if $schemas exists and is a Sequence
            if let Some(Value::Mapping(ref mut namespaces)) = mapping.get_mut("$namespaces") {
                if !namespaces.contains_key(&Value::String("arc".to_string())) {
                    namespaces.insert(
                        Value::String("arc".to_string()),
                        Value::String("https://github.com/nfdi4plants/ARC_ontology".to_string()),
                    );
                }
            } else {
                let mut namespaces = Mapping::new();
                namespaces.insert(
                    Value::String("arc".to_string()),
                    Value::String("https://github.com/nfdi4plants/ARC_ontology".to_string()),
                );
                mapping.insert(Value::String("$namespaces".to_string()), Value::Mapping(namespaces));
            }
        }
    
        // Get the filename to write the updated YAML
        let path = get_filename(name)?;
    
        // Create the file at the specified path
        let mut file = File::create(path)?;
    
        // Convert the YAML content to a string and write it to the file
        let yaml_str = serde_yml::to_string(&yaml)?;
        file.write_all(yaml_str.as_bytes())?;
    
        Ok(())
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
    let current_path = current_dir.join(&cwl_name);
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


pub fn annotate_process_step(args: &AnnotateProcessArgs) -> Result<(), Box<dyn Error>> {
    // Read and parse the existing CWL file
    let yaml_result = parse_cwl(&args.cwl_name)?;
    let mut yaml = yaml_result;

    // Construct the process sequence mapping
    let mut process_sequence = Mapping::new();
    process_sequence.insert(Value::String("class".to_string()), Value::String("arc:process sequence".to_string()));

    if let Some(ref name) = args.name {
        process_sequence.insert(Value::String("arc:name".to_string()), Value::String(name.clone()));
    }

    if let Some(ref input) = args.input {
        let mut input_data = Mapping::new();
        input_data.insert(Value::String("class".to_string()), Value::String("arc:data".to_string()));
        input_data.insert(Value::String("arc:name".to_string()), Value::String(input.clone()));
        process_sequence.insert(Value::String("arc:has input".to_string()), Value::Sequence(vec![Value::Mapping(input_data)]));
    }

    if let Some(ref output) = args.output {
        let mut output_data = Mapping::new();
        output_data.insert(Value::String("class".to_string()), Value::String("arc:data".to_string()));
        output_data.insert(Value::String("arc:name".to_string()), Value::String(output.clone()));
        process_sequence.insert(Value::String("arc:has output".to_string()), Value::Sequence(vec![Value::Mapping(output_data)]));
    }

    if let Some(ref parameter) = args.parameter {
        let mut parameter_value = Mapping::new();
        parameter_value.insert(Value::String("class".to_string()), Value::String("arc:process parameter value".to_string()));

        let mut protocol_parameter = Mapping::new();
        protocol_parameter.insert(Value::String("class".to_string()), Value::String("arc:protocol parameter".to_string()));

        let mut parameter_name = Mapping::new();
        parameter_name.insert(Value::String("class".to_string()), Value::String("arc:parameter name".to_string()));
        parameter_name.insert(Value::String("arc:term accession".to_string()), Value::String("".to_string()));
        parameter_name.insert(Value::String("arc:term source REF".to_string()), Value::String("".to_string()));
        parameter_name.insert(Value::String("arc:annotation value".to_string()), Value::String(parameter.clone()));

        protocol_parameter.insert(Value::String("arc:has parameter name".to_string()), Value::Sequence(vec![Value::Mapping(parameter_name)]));
        parameter_value.insert(Value::String("arc:has parameter".to_string()), Value::Sequence(vec![Value::Mapping(protocol_parameter)]));

        if let Some(ref value) = args.value {
            let mut ontology_annotation = Mapping::new();
            ontology_annotation.insert(Value::String("class".to_string()), Value::String("arc:ontology annotation".to_string()));
            ontology_annotation.insert(Value::String("arc:term accession".to_string()), Value::String("".to_string()));
            ontology_annotation.insert(Value::String("arc:term source REF".to_string()), Value::String("".to_string()));
            ontology_annotation.insert(Value::String("arc:annotation value".to_string()), Value::String(value.clone()));

            parameter_value.insert(Value::String("arc:value".to_string()), Value::Sequence(vec![Value::Mapping(ontology_annotation)]));
        }

        process_sequence.insert(Value::String("arc:has parameter value".to_string()), Value::Sequence(vec![Value::Mapping(parameter_value)]));
    }

    // Ensure the root is a mapping
    if let Value::Mapping(ref mut mapping) = yaml {
        // Add the process sequence to 'arc:has process sequence'
        if let Some(Value::Sequence(ref mut process_sequences)) = mapping.get_mut("arc:has process sequence") {
            // Check for duplicates before adding
            if !process_sequences.iter().any(|ps| ps == &Value::Mapping(process_sequence.clone())) {
                process_sequences.push(Value::Mapping(process_sequence));
            }
        } else {
            // If 'arc:has process sequence' doesn't exist, create it
            mapping.insert(Value::String("arc:has process sequence".to_string()), Value::Sequence(vec![Value::Mapping(process_sequence)]));
        }
    } else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    }

    // Write the updated YAML back to the file
    let path = get_filename(&args.cwl_name)?;
    let mut file = File::create(path)?;
    let yaml_str = serde_yml::to_string(&yaml)?;
    file.write_all(yaml_str.as_bytes())?;

    Ok(())
}





//const ZOOMA_URL: &str = "http://www.ebi.ac.uk/spot/zooma/v2/api/services/annotate";
const BIOTOOLS_URL: &str = "http://data.bioontology.org";

#[derive(Deserialize)]
struct AnnotatedClass {
    #[serde(rename = "@id")]
    id: String,
    pref_label: Option<String>,
    links: Option<Links>,
}

#[derive(Deserialize)]
struct Links {
    #[serde(rename = "self")]
    self_link: String,
}

#[derive(Deserialize)]
struct Annotation {
    annotated_class: AnnotatedClass,
}

async fn get_json_biotools(url: &str, biotools_key: &str) -> Result<AnnotatedClass, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("apikey token={}", biotools_key))?,
    );

    let response = client.get(url).headers(headers).send().await?;
    let annotated_class = response.json::<AnnotatedClass>().await?;
    Ok(annotated_class)
}

fn dict_equal(d1: &AnnotatedClass, d2: &AnnotatedClass) -> bool {
    d1.id == d2.id && d1.pref_label == d2.pref_label
}

async fn biotools_recommendations(
    search_term: &str,
    biotools_key: &str,
    max_recommendations: usize,
) -> Result<Vec<AnnotatedClass>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("apikey token={}", biotools_key))?,
    );

    let url = format!(
        "{}/annotator?text={}",
        BIOTOOLS_URL,
        urlencoding::encode(search_term)
    );
    let response = client.get(&url).headers(headers).send().await?;
    let annotations = response.json::<Vec<Annotation>>().await?;

    let mut recommendations = Vec::new();
    for result in annotations.iter().take(max_recommendations) {
        if let Some(links) = &result.annotated_class.links {
            if let Ok(class_details) = get_json_biotools(&links.self_link, biotools_key).await {
                recommendations.push(class_details);
            }
        }
    }

    recommendations.dedup_by(|a, b| dict_equal(a, b));
    Ok(recommendations)
}


