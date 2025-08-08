use std::error::Error;
use dialoguer::{Input, Confirm};
use serde_yaml::{Mapping, Value};
use crate::common::{get_affiliation_and_orcid, annotate, parse_cwl};
use crate::consts::{ARC_SCHEMA, ARC_NAMESPACE};
use crate::process::process_annotation_with_mapping;
use crate::common::write_updated_yaml;

#[derive(Debug, Clone)]
pub struct Performer {
    pub cwl_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub mid_initials: Option<String>,
    pub mail: Option<String>,
    pub affiliation: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub fax: Option<String>,
    pub role: Option<String>,
}

/// Insert a key-value if Option<String> is Some
fn insert_if_some(map: &mut Mapping, key: &str, value: &Option<String>) {
    if let Some(v) = value {
        map.insert(str_val(key), str_val(v));
    }
}

/// Convenience for YAML string value
fn str_val(s: &str) -> Value {
    Value::String(s.to_string())
}

pub async fn annotate_performer_default(args: &Performer) -> Result<(), Box<dyn Error>> {
    // Utility for optional text input
    fn prompt_optional(prompt: &str) -> Result<Option<String>, Box<dyn Error>> {
        let input: String = Input::new()
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text()?;
        Ok(if input.is_empty() { None } else { Some(input) })
    }

    // Required fields
    let first_name: String = Input::new()
        .with_prompt("Enter performer's first name")
        .interact_text()?;
    let last_name: String = Input::new()
        .with_prompt("Enter performer's last name")
        .interact_text()?;

    // Optional fields
    let mid_initials = prompt_optional("Enter performer's middle initials (or leave blank)")?;
    let address = prompt_optional("Enter performer's address (or leave blank)")?;
    let phone = prompt_optional("Enter performer's phone number (or leave blank)")?;
    let fax = prompt_optional("Enter performer's fax number (or leave blank)")?;

    // ORCID search option
    let search_orcid = Confirm::new()
        .with_prompt("Do you want to search for this person via ORCID?")
        .interact()?;

    let (mut mail, mut affiliation, mut role) = (None, None, None);

    if search_orcid {
        let (aff, _orcid, m) = get_affiliation_and_orcid(&first_name, &last_name).await;
        affiliation = aff;
        mail = m;
    }
    // Ask to annotate additional fields if not already provided
    if mail.is_none() || affiliation.is_none() || role.is_none() {
        // Directly prompt for missing fields without asking for confirmation
        if mail.is_none() {
            mail = prompt_optional("Enter email (or leave blank)")?;
        }
        if affiliation.is_none() {
            affiliation = prompt_optional("Enter affiliation (or leave blank)")?;
        }
        if role.is_none() {
            role = prompt_optional("Enter role (or leave blank)")?;
        }
    }

    // Construct performer args
    let default_performer = Performer {
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

pub async fn annotate_performer(args: &Performer) -> Result<(), Box<dyn Error>> {
    // Ensure ARC namespace and schema
    annotate(&args.cwl_name, "$schemas", None, Some(ARC_SCHEMA))?;
    annotate(&args.cwl_name, "$namespaces", Some("arc"), Some(ARC_NAMESPACE))?;
    // Parse CWL YAML
    let mut yaml = parse_cwl(&args.cwl_name)?;
    let mapping = yaml.as_mapping_mut()
        .ok_or("The CWL file does not have a valid YAML mapping at its root.")?;
    // Build performer info
    let mut performer_info = build_performer_info(args)?;
    // Add role if provided
    if let Some(ref role) = args.role {
        add_role_to_performer(&mut performer_info, role).await?;
    }
    // Insert or update performer in YAML
    update_performer(mapping, performer_info, args)?;
    // Save back to file
    write_updated_yaml(&args.cwl_name, &yaml)
}

/// Build a performer info mapping from args
fn build_performer_info(args: &Performer) -> Result<Mapping, Box<dyn Error>> {
    let mut info = Mapping::new();
    info.insert(str_val("class"), str_val("arc:Person"));
    insert_if_some(&mut info, "arc:first name", &args.first_name);
    insert_if_some(&mut info, "arc:last name", &args.last_name);
    insert_if_some(&mut info, "arc:mid initials", &args.mid_initials);
    insert_if_some(&mut info, "arc:email", &args.mail);
    insert_if_some(&mut info, "arc:affiliation", &args.affiliation);
    insert_if_some(&mut info, "arc:address", &args.address);
    insert_if_some(&mut info, "arc:phone", &args.phone);
    insert_if_some(&mut info, "arc:fax", &args.fax);

    Ok(info)
}

/// Add role information to performer info
async fn add_role_to_performer(info: &mut Mapping, role: &str) -> Result<(), Box<dyn Error>> {
    let mut role_mapping = Mapping::new();
    role_mapping.insert(str_val("class"), str_val("arc:role"));

    let annotation_value = process_annotation_with_mapping(role, role_mapping.clone(), false).await?;
    role_mapping.extend(annotation_value);

    let key = str_val("arc:has role");
    match info.get_mut(&key) {
        Some(Value::Sequence(seq)) => seq.push(Value::Mapping(role_mapping)),
        _ => {
            info.insert(key, Value::Sequence(vec![Value::Mapping(role_mapping)]));
        }
    }
    Ok(())
}

/// Update the performer list in YAML
fn update_performer(mapping: &mut Mapping, performer_info: Mapping, args: &Performer) -> Result<(), Box<dyn Error>> {
    let performer_key = str_val("arc:performer");
    match mapping.get_mut(&performer_key) {
        Some(Value::Sequence(performers)) => {
            if let Some(idx) = find_existing_performer(performers, args) {
                handle_existing_performer(performers, idx, performer_info)?;
            } else {
                performers.push(Value::Mapping(performer_info));
            }
        }
        _ => {
            mapping.insert(performer_key, Value::Sequence(vec![Value::Mapping(performer_info)]));
        }
    }
    Ok(())
}

/// Find index of performer with matching first & last name
fn find_existing_performer(performers: &[Value], args: &Performer) -> Option<usize> {
    performers.iter().enumerate().find_map(|(idx, performer)| {
        if let Value::Mapping(existing) = performer {
            let first_match = existing.get(str_val("arc:first name")) == Some(&str_val(args.first_name.as_ref().unwrap_or(&String::new())));
            let last_match = existing.get(str_val("arc:last name")) == Some(&str_val(args.last_name.as_ref().unwrap_or(&String::new())));
            if first_match && last_match { Some(idx) } else { None }
        } else {
            None
        }
    })
}

/// Handle updating or appending performer if duplicate exists
fn handle_existing_performer(performers: &mut Vec<Value>, idx: usize, new_info: Mapping) -> Result<(), Box<dyn Error>> {
    if let Value::Mapping(existing) = &performers[idx] {
        if *existing == new_info {
            return Ok(()); // No changes needed
        }
        eprintln!("A performer with the same first and last name exists:");
        if let Ok(yaml_str) = serde_yaml::to_string(existing) {
            eprintln!("{}", &yaml_str);
        }
        if Confirm::new()
            .with_prompt("Do you want to update this performer with new information?")
            .interact()
            .unwrap_or(false) {
            if let Value::Mapping(existing_mut) = &mut performers[idx] {
                for (k, v) in new_info {
                    existing_mut.insert(k, v);
                }
            }
        } else {
            performers.push(Value::Mapping(new_info));
        }
    }
    Ok(())
}