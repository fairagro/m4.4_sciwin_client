use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_yaml::Value;
use std::fmt::Debug;

pub trait Identifiable {
    fn id(&self) -> &str;
    fn set_id(&mut self, id: String);
}

/// Deserializes a list of `Identifiable` items from a YAML value.
/// The input can be either a sequence (list) or a mapping (dictionary).
/// If it's a mapping, the keys are used as IDs for the items.
/// # Examples
/// ```
/// use serde_yaml::Value;
/// use serde::{Deserialize, Serialize};
/// use std::fmt::Debug;
/// use std::collections::HashMap;
/// use commonwl::deserialize::Identifiable;
/// use commonwl::deserialize::deserialize_list;
///
/// #[derive(Debug, Deserialize)]
/// struct ItemBag {
///     #[serde(deserialize_with = "deserialize_list")]
///     items: Vec<MyItem>,
/// }
///
/// #[derive(Debug, Deserialize)]
/// struct MyItem {
///     #[serde(default)]
///     id: String,
///     name: String,
/// }
///
/// impl Identifiable for MyItem {
///     fn id(&self) -> &str {      
///        &self.id
///   }
///   fn set_id(&mut self, id: String) {
///       self.id = id;
///  }
/// }
/// let yaml_seq = r#"
/// items:
/// - id: item1
///   name: Item 1
/// - id: item2
///   name: Item 2
/// "#;
///     
/// let yaml_map = r#"
/// items:
///   item1:
///     name: item1
///   item2:
///     name: item2
/// "#;
///
/// let seq: ItemBag = serde_yaml::from_str(yaml_seq).unwrap();
/// let map: ItemBag = serde_yaml::from_str(yaml_map).unwrap();
/// ```
pub fn deserialize_list<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Identifiable + Debug,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    let parameters = match value {
        Value::Sequence(seq) => seq
            .into_iter()
            .map(|item| {
                let param: T = serde_yaml::from_value(item).map_err(serde::de::Error::custom)?;
                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        Value::Mapping(map) => map
            .into_iter()
            .map(|(key, value)| {
                let id = key.as_str().ok_or_else(|| serde::de::Error::custom("Expected string key"))?;
                let mut param: T = serde_yaml::from_value(value).map_err(serde::de::Error::custom)?;
                param.set_id(id.to_string());
                Ok(param)
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(serde::de::Error::custom("Expected sequence or mapping")),
    };

    Ok(parameters)
}
