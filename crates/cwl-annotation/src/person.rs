use std::error::Error;
use serde_yaml::{Mapping, Value};
use crate::consts::{SCHEMAORG_NAMESPACE, SCHEMAORG_SCHEMA};
use crate::common::{annotate, parse_cwl, write_updated_yaml, get_affiliation_and_orcid};


#[derive(Debug)]
pub struct Person {
    pub cwl_name: String,
    pub name: String,
    pub mail: Option<String>,
    pub id: Option<String>,
}

pub async fn annotate_person(
    cwl_name: &str,
    name: &str,
    mut mail: Option<String>,
    mut id: Option<String>,
    role: &str,
) -> Result<(), Box<dyn Error>> {
    // Ensure schema.org namespace and schema are present
    annotate(cwl_name, "$namespaces", Some("s"), Some(SCHEMAORG_NAMESPACE))?;
    annotate(cwl_name, "$schemas", None, Some(SCHEMAORG_SCHEMA))?;

    // If name is provided but id is not, try to get id using get_affiliation_and_orcid
    if !name.is_empty() && id.is_none() {
        // Split name into first and last parts
        let mut parts = name.split_whitespace();
        let first = parts.next().unwrap_or("");
        let last = parts.last().unwrap_or("");
        let (_affiliation, orcid, mail_opt) = get_affiliation_and_orcid(first, last).await;
        if let Some(orcid) = orcid
            && !orcid.is_empty() {
            id = Some(orcid);
        }
        if let Some(mail_val) = mail_opt
            && !mail_val.is_empty() {
            mail = Some(mail_val);
        }
    }
    let mut yaml = parse_cwl(cwl_name)?;
    let Some(mapping) = yaml.as_mapping_mut() else {
        return Err("The CWL file does not have a valid YAML mapping at its root.".into());
    };
    let mut person_info = Mapping::new();
    person_info.insert(Value::String("class".to_string()), Value::String("s:Person".to_string()));
    if let Some(ref person_id) = id {
        person_info.insert(Value::String("s:identifier".to_string()), Value::String(person_id.clone()));
    }
    if let Some(ref person_mail) = mail {
        person_info.insert(Value::String("s:email".to_string()), Value::String(format!("mailto:{person_mail}")));
    }
    person_info.insert(Value::String("s:name".to_string()), Value::String(name.to_string()));

    // Helper closure to check if a person with the same identifier or email already exists
    let person_exists = |persons: &Vec<Value>| {
        persons.iter().any(|person| {
            if let Value::Mapping(existing_person) = person {
                if let Some(Value::String(existing_id)) = existing_person.get(Value::String("s:identifier".to_string()))
                    && let Some(ref pid) = id
                    && existing_id == pid {
                    return true;
                }
                if let Some(Value::String(existing_email)) = existing_person.get(Value::String("s:email".to_string()))
                    && let Some(ref pmail) = mail
                    && existing_email == &format!("mailto:{pmail}") {
                    return true;
                }
            }
            false
        })
    };

    match mapping.get_mut(Value::String(role.to_string())) {
        Some(Value::Sequence(persons)) => {
            if !person_exists(persons) {
                persons.push(Value::Mapping(person_info));
            }
        }
        _ => {
            mapping.insert(Value::String(role.to_string()), Value::Sequence(vec![Value::Mapping(person_info)]));
        }
    }

    write_updated_yaml(cwl_name, &yaml)
}
