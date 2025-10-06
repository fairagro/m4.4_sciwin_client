use std::{
    collections::HashSet,
    error::Error,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    env,
};
use dialoguer::{Select, FuzzySelect, Confirm, Input};
use reqwest::{get, Client};
use serde_yaml::{Mapping, Value};
use urlencoding::encode;
use crate::consts::{SCHEMAORG_NAMESPACE, SCHEMAORG_SCHEMA, ARC_NAMESPACE, ARC_SCHEMA, 
    REST_URL_TS, TS_COLLECTION_ID, SPDX};
use util::is_cwl_file;


pub fn annotate_default(tool_name: &str) -> Result<(), Box<dyn Error>> {
    annotate(tool_name, "$namespaces", Some("s"), Some(SCHEMAORG_NAMESPACE))?;
    annotate(tool_name, "$schemas", None, Some(SCHEMAORG_SCHEMA))?;
    annotate(tool_name, "$namespaces", Some("arc"), Some(ARC_NAMESPACE))?;
    annotate(tool_name, "$schemas", None, Some(ARC_SCHEMA))?;
    Ok(())
}

pub fn annotate(name: &str, namespace_key: &str, key: Option<&str>, value: Option<&str>) -> Result<(), Box<dyn Error>> {
    let mut yaml = parse_cwl(name)?;
    let mapping = yaml.as_mapping_mut().ok_or("Root YAML is not a mapping")?;
    let ns_key = Value::String(namespace_key.into());

    match mapping.get_mut(&ns_key) {
        Some(Value::Sequence(seq)) => {
            if let Some(val) = key.or(value) {
                let val_str = Value::String(val.into());
                if !seq.contains(&val_str) {
                    seq.push(val_str);
                }
            }
        }
        Some(Value::Mapping(map)) => {
            if let (Some(k), Some(v)) = (key, value) {
                let k = Value::String(k.into());
                if !map.contains_key(&k) {
                    map.insert(k, Value::String(v.into()));
                }
            }
        }
        _ => {
            match (key, value) {
                (Some(k), Some(v)) => {
                    let mut new_map = Mapping::new();
                    new_map.insert(Value::String(k.into()), Value::String(v.into()));
                    mapping.insert(ns_key, Value::Mapping(new_map));
                }
                (Some(single), None) | (None, Some(single)) => {
                    let new_seq = vec![Value::String(single.into())];
                    mapping.insert(ns_key, Value::Sequence(new_seq));
                }
                (None, None) => {
                    return Err("annotate called with neither key nor value".into());
                }
            }
        }
    }

    write_updated_yaml(name, &yaml)
}

pub fn write_updated_yaml(name: &str, yaml: &Value) -> Result<(), Box<dyn Error>> {
    let path = get_filename(name)?;

    // Convert the YAML content to a string and write it to the file
    let yaml_str = serde_yaml::to_string(&yaml).map_err(|e| format!("Failed to serialize YAML: {e}"))?;
    File::create(&path)
        .and_then(|mut file| file.write_all(yaml_str.as_bytes()))
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

#[allow(clippy::disallowed_macros)]
pub fn select_annotation(recommendations: &HashSet<(String, String, String)>, term: String) -> Result<(String, String, String), Box<dyn Error>> {
    //println!("{}", format!("Available annotations for '{term}':"));
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
        .with_prompt(format!("Available annotations for '{term}': Use the arrow keys to navigate, Enter to select"))
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


pub async fn ts_recommendations(
    search_term: &str,
    max_recommendations: usize,
) -> Result<(String, String, String), Box<dyn Error>> {
    let client = Client::new();
    let encoded_term = encode(search_term.trim());

    // Construct the full URL with proper encoding
    let url = format!(
        "{REST_URL_TS}{encoded_term}&collectionId={TS_COLLECTION_ID}"
    );
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let ts_json: serde_json::Value = response.json().await?;
    let mut recommendations: HashSet<(String, String, String)> = HashSet::new();

   if let Some(results) = ts_json.as_array() {
        for result in results.iter().take(max_recommendations) {
            let id = result.get("iri").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let label = result.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let ontology = result
                .get("ontology")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            if !id.is_empty() && !label.is_empty() {
                recommendations.insert((label, ontology, id));
            }
        }
    } else {
        eprintln!("Warning: No valid results found in TS4NFDI response.");
    }
    select_annotation(&recommendations, search_term.to_string())
}

pub async fn ask_for_license() -> Result<Option<(String, String)>, Box<dyn Error>> {
    // Fetch the SPDX license list
    let response = get(SPDX).await?;
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
    let display_list: Vec<String> = sorted_list
        .iter()
        .map(|(name, reference)| format!("{name} ({reference})"))
        .collect();

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

#[allow(clippy::disallowed_macros)]
pub async fn get_affiliation_and_orcid(first_name: &str, last_name: &str) -> (Option<String>, Option<String>, Option<String>) {
    if first_name.is_empty() || last_name.is_empty() {
        return (None, None, None);
    }

    let query = format!(
        "given-names:{} AND family-name:{}",
        first_name.chars().next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default()
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
            result.get(key)
                .and_then(|v| if v.is_array() { v.as_array().and_then(|arr| arr.first()).and_then(|f| f.as_str()) } else { v.as_str() })
                .unwrap_or("").to_string()
        };

        let show_options = |results: &[serde_json::Value]| -> Option<(String, String, String)> {
            println!("\nOther possible matches:");
            let options: Vec<String> = results.iter().take(5).map(|result| {
                let given_names = get_field(result, "given-names");
                let family_name = get_field(result, "family-names");
                let institution = get_field(result, "institution-name");
                format!("{given_names} {family_name} ({institution})")
            }).collect();

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
            let is_right_person = Confirm::new()
                .with_prompt("Is this the correct person?")
                .interact()
                .unwrap_or(false);
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