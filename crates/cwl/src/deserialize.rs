use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_yaml::Value;
use std::fmt::Debug;

pub trait Identifiable {
    fn id(&self) -> &str;
    fn set_id(&mut self, id: String);
}

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
