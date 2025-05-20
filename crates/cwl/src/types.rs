use serde::{Deserialize, Deserializer, Serialize};
use serde_yaml::{Mapping, Number, Value};
use sha1::{Digest, Sha1};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Debug, Default, PartialEq, Clone, Eq, Hash)]
pub enum CWLType {
    #[default]
    Null,
    Boolean,
    Int,
    Long,
    Float,
    Double,
    String,
    File,
    Directory,
    Any,
    Stdout,
    Stderr,
    Optional(Box<CWLType>),
    Array(Box<CWLType>),
}

impl FromStr for CWLType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(inner) = s.strip_suffix('?') {
            Ok(CWLType::Optional(Box::new(inner.parse()?)))
        } else if let Some(inner) = s.strip_suffix("[]") {
            Ok(CWLType::Array(Box::new(inner.parse()?)))
        } else {
            match s {
                "null" => Ok(CWLType::Null),
                "boolean" => Ok(CWLType::Boolean),
                "int" => Ok(CWLType::Int),
                "long" => Ok(CWLType::Long),
                "float" => Ok(CWLType::Float),
                "double" => Ok(CWLType::Double),
                "string" => Ok(CWLType::String),
                "File" => Ok(CWLType::File),
                "Directory" => Ok(CWLType::Directory),
                "Any" => Ok(CWLType::Any),
                "stdout" => Ok(CWLType::Stdout),
                "stderr" => Ok(CWLType::Stderr),
                _ => Err(format!("Invalid CWLType: {s}")),
            }
        }
    }
}

impl CWLType {
    pub fn is_optional(&self) -> bool {
        matches!(self, CWLType::Optional(_))
    }
}

fn serialize_type(t: &CWLType) -> String {
    match t {
        CWLType::Optional(inner) => format!("{}?", serialize_type(inner)),
        CWLType::Array(inner) => format!("{}[]", serialize_type(inner)),
        CWLType::Null => "null".to_string(),
        CWLType::Boolean => "boolean".to_string(),
        CWLType::Int => "int".to_string(),
        CWLType::Long => "long".to_string(),
        CWLType::Float => "float".to_string(),
        CWLType::Double => "double".to_string(),
        CWLType::String => "string".to_string(),
        CWLType::File => "File".to_string(),
        CWLType::Directory => "Directory".to_string(),
        CWLType::Any => "Any".to_string(),
        CWLType::Stdout => "stdout".to_string(),
        CWLType::Stderr => "stderr".to_string(),
    }
}

impl<'de> Deserialize<'de> for CWLType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        if let Value::String(s) = value {
            return s.parse().map_err(serde::de::Error::custom);
        } else if let Value::Mapping(map) = value {
            return deserialize_type_mapping(&map).map_err(serde::de::Error::custom);
        } else if let Value::Sequence(seq) = value {
            if seq.len() == 2 {
                if let Value::String(type_str) = &seq[0] {
                    if type_str == "null" {
                        if let Value::String(type_str) = &seq[1] {
                            return format!("{type_str}?").parse().map_err(serde::de::Error::custom);
                        }
                    }
                }
            } else if seq.len() == 1 {
                if let Value::Mapping(map) = &seq[0] {
                    return deserialize_type_mapping(map).map_err(serde::de::Error::custom);
                }
            }
        }
        Err(serde::de::Error::custom("Expected string, sequence or mapping for CWLType"))
    }
}

fn deserialize_type_mapping(map: &Mapping) -> Result<CWLType, String> {
    if let Some(Value::String(type_str)) = map.get("type") {
        if type_str == "array" {
            if let Some(Value::String(item_str)) = map.get("items") {
                return format!("{item_str}[]").parse();
            }
        }
    }
    Err("Could not parse type mapping".into())
}

impl Serialize for CWLType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = serialize_type(self);
        serializer.serialize_str(&s)
    }
}

#[derive(Serialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum DefaultValue {
    File(File),
    Directory(Directory),
    Array(Vec<DefaultValue>),
    Any(serde_yaml::Value),
}

fn number_to_string(num: &Number) -> String {
    if num.is_i64() {
        num.as_i64().unwrap().to_string()
    } else if num.is_f64() {
        num.as_f64().unwrap().to_string()
    } else if num.is_u64() {
        num.as_u64().unwrap().to_string()
    } else {
        unreachable!()
    }
}

impl DefaultValue {
    /// Returns a string of the primary value field in `DefaultValue`.
    pub fn as_value_string(&self) -> String {
        match self {
            DefaultValue::File(item) => item.location.as_ref().unwrap_or(&String::new()).clone(),
            DefaultValue::Directory(item) => item.location.as_ref().unwrap_or(&String::new()).clone(),
            DefaultValue::Any(value) => match value {
                serde_yaml::Value::Bool(_) => String::new(), // do not remove!
                serde_yaml::Value::String(s) => s.to_string(),
                serde_yaml::Value::Number(num) => number_to_string(num),
                _ => serde_yaml::to_string(value).unwrap().trim_end().to_string(),
            },
            DefaultValue::Array(item) => item.iter().map(|i| i.as_value_string()).collect::<Vec<_>>().join(" ").to_string(),
        }
    }

    /// Checks whether a `CWLType` matches this `DefaultValue` implementation
    pub fn has_matching_type(&self, cwl_type: &CWLType) -> bool {
        match (self, cwl_type) {
            (_, CWLType::Any) => true,
            (DefaultValue::File(_), CWLType::File) => true,
            (DefaultValue::Directory(_), CWLType::Directory) => true,
            (DefaultValue::Any(Value::Null), CWLType::Optional(_)) => true,
            (_, CWLType::Optional(inner)) => self.has_matching_type(inner),
            (DefaultValue::Any(inner), cwl_type) => match inner {
                Value::Bool(_) => matches!(cwl_type, CWLType::Boolean),
                Value::Number(num) => {
                    if num.is_i64() {
                        matches!(cwl_type, CWLType::Int | CWLType::Long | CWLType::Float | CWLType::Double)
                    } else if num.is_f64() {
                        matches!(cwl_type, CWLType::Float | CWLType::Double)
                    } else {
                        false
                    }
                }
                Value::String(_) => matches!(cwl_type, CWLType::String),
                Value::Sequence(_) => matches!(cwl_type, CWLType::Array(_)),
                Value::Mapping(_) => false,
                Value::Null => matches!(cwl_type, CWLType::Null),
                _ => false,
            },
            (DefaultValue::Array(_), CWLType::Array(_)) => true,
            _ => false,
        }
    }

    /// !Legacy!
    pub fn to_default_value(&self) -> DefaultValue {
        match self {
            DefaultValue::File(file) => DefaultValue::File(File::from_location(file.path.as_ref().unwrap_or(&String::new()))),
            DefaultValue::Directory(dir) => DefaultValue::Directory(Directory::from_location(dir.path.as_ref().unwrap_or(&String::new()))),
            DefaultValue::Any(val) => DefaultValue::Any(val.clone()),
            DefaultValue::Array(arr) => DefaultValue::Array(arr.iter().map(|i| i.to_default_value()).collect()),
        }
    }
}

impl<'de> Deserialize<'de> for DefaultValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        let class = value.get("class").and_then(Value::as_str);

        if let Some(class) = class {
            let location = value
                .get("location")
                .or_else(|| value.get("path"))
                .and_then(|v| v.as_str().map(|s| s.to_string()));
            let secondary_files = value
                .get("secondaryFiles")
                .map(|v| serde_yaml::from_value(v.clone()))
                .transpose()
                .map_err(serde::de::Error::custom)?;

            let basename = value
                .get("basename")
                .map(|v| serde_yaml::from_value(v.clone()))
                .transpose()
                .map_err(serde::de::Error::custom)?;

            match class {
                "File" => {
                    let format = value
                        .get("format")
                        .map(|v| serde_yaml::from_value(v.clone()))
                        .transpose()
                        .map_err(serde::de::Error::custom)?;
                    let contents = value
                        .get("contents")
                        .map(|v| serde_yaml::from_value(v.clone()))
                        .transpose()
                        .map_err(serde::de::Error::custom)?;
                    let item = File {
                        location,
                        secondary_files,
                        basename,
                        format,
                        contents,
                        ..Default::default()
                    };
                    Ok(DefaultValue::File(item))
                }
                "Directory" => {
                    let listing = value
                        .get("listing")
                        .map(|v| serde_yaml::from_value(v.clone()))
                        .transpose()
                        .map_err(serde::de::Error::custom)?
                        .unwrap_or_default();

                    let item = Directory {
                        location,
                        basename,
                        listing,
                        ..Default::default()
                    };
                    Ok(DefaultValue::Directory(item))
                }
                _ => Ok(DefaultValue::Any(value)),
            }
        } else if let Value::Sequence(map) = value {
            Ok(DefaultValue::Array(map.into_iter().map(|i| serde_yaml::from_value(i).unwrap()).collect()))
        } else {
            Ok(DefaultValue::Any(value))
        }
    }
}

/// Tries to guess the CWLType of a given value
pub fn guess_type(value: &str) -> CWLType {
    let path = Path::new(value);
    if path.exists() {
        if path.is_file() {
            return CWLType::File;
        }
        if path.is_dir() {
            return CWLType::Directory;
        }
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return CWLType::File;
    }

    //we do not have to check for files that do not exist yet, as CWLTool would run into a failure
    let yaml_value: Value = serde_yaml::from_str(value).unwrap();
    match yaml_value {
        Value::Null => CWLType::Null,
        Value::Bool(_) => CWLType::Boolean,
        Value::Number(number) => {
            if number.is_f64() {
                CWLType::Float
            } else {
                CWLType::Int
            }
        }
        Value::String(_) => CWLType::String,
        _ => CWLType::String,
    }
}

pub trait PathItem {
    fn get_location(&self) -> String;
    fn set_location(&mut self, new_location: String);
    fn secondary_files_mut(&mut self) -> Option<&mut Vec<DefaultValue>>;
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nameroot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nameext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_files: Option<Vec<DefaultValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
}

impl Default for File {
    fn default() -> Self {
        Self {
            class: String::from("File"),
            location: Default::default(),
            path: Default::default(),
            basename: Default::default(),
            dirname: Default::default(),
            nameroot: Default::default(),
            nameext: Default::default(),
            checksum: Default::default(),
            size: Default::default(),
            secondary_files: Default::default(),
            format: Default::default(),
            contents: Default::default(),
        }
    }
}

impl File {
    pub fn from_location(location: impl ToString) -> Self {
        File {
            location: Some(location.to_string()),
            ..Default::default()
        }
    }

    /// loads metadata and alters File
    pub fn load(&mut self, relative_to: impl AsRef<Path>) {
        if let Some(loc) = &self.location {
            let mut path = PathBuf::from(&loc);
            if !path.exists() {
                path = relative_to.as_ref().join(path);
            }

            let loc = path.strip_prefix(relative_to).unwrap_or(&path).to_string_lossy().into_owned();
            self.path = Some(loc);
            if self.basename.is_none() {
                self.basename = path.file_name().map(|f| f.to_string_lossy().into_owned());
            }

            if self.nameroot.is_none() {
                self.nameroot = path.file_stem().map(|f| f.to_string_lossy().into_owned());
            }

            if self.nameext.is_none() {
                self.nameext = path.extension().map(|f| format!(".{}", f.to_string_lossy()));
            }

            if self.dirname.is_none() {
                self.dirname = path.parent().map(|f| f.to_string_lossy().into_owned());
            }

            let metadata = fs::metadata(&path).expect("Could not get metadata");
            self.size = Some(metadata.len());

            let mut hasher = Sha1::new();
            let hash = fs::read(&path).ok().map(|f| {
                hasher.update(&f);
                let hash = hasher.finalize();
                format!("sha1${hash:x}")
            });
            self.checksum = hash;
        }
    }

    /// loads metadata and returns a **new** File
    pub fn snapshot(&self) -> Self {
        let loc = self.location.clone().unwrap_or_default();
        let path = Path::new(&loc);
        let current = env::current_dir().unwrap_or_default();
        let absolute_path = if path.is_absolute() { path } else { &current.join(path) };
        let absolute_str = absolute_path.display().to_string();
        let metadata = fs::metadata(path).unwrap_or_else(|_| panic!("Could not get metadata for {absolute_str}"));
        let mut hasher = Sha1::new();
        let hash = fs::read(path).ok().map(|f| {
            hasher.update(&f);
            let hash = hasher.finalize();
            format!("sha1${hash:x}")
        });

        Self {
            location: Some(format!("file://{absolute_str}")),
            path: Some(loc.clone()),
            basename: path.file_name().map(|f| f.to_string_lossy().into_owned()),
            dirname: None,
            nameroot: path.file_stem().map(|f| f.to_string_lossy().into_owned()),
            nameext: path.extension().map(|f| format!(".{}", f.to_string_lossy())),
            checksum: hash,
            size: Some(metadata.len()),
            secondary_files: self.secondary_files.clone(),
            format: resolve_format(self.format.clone()),
            ..Default::default()
        }
    }

    pub fn preload(&self) -> Self {
        let loc = self.location.clone().unwrap_or_default();
        let path = Path::new(&loc);

        let mut item = self.clone();
        if item.path.is_none() {
            item.path = Some(path.display().to_string());
        }

        if item.basename.is_none() {
            item.basename = path.file_name().map(|f| f.to_string_lossy().into_owned());
        }

        if item.nameroot.is_none() {
            item.nameroot = path.file_stem().map(|f| f.to_string_lossy().into_owned());
        }

        if item.nameext.is_none() {
            item.nameext = path.extension().map(|f| format!(".{}", f.to_string_lossy()));
        }

        item
    }
}

fn resolve_format(format: Option<String>) -> Option<String> {
    if let Some(format) = format {
        let edam_url = "http://edamontology.org/";
        Some(format.replace("edam:", edam_url))
    } else {
        None
    }
}

impl PathItem for File {
    fn set_location(&mut self, new_location: String) {
        self.location = Some(new_location);
    }

    fn secondary_files_mut(&mut self) -> Option<&mut Vec<DefaultValue>> {
        self.secondary_files.as_mut()
    }

    fn get_location(&self) -> String {
        self.location.as_ref().unwrap_or(&String::new()).clone()
    }
}

#[derive(Serialize, Debug, Default, PartialEq, Clone)]
pub struct SecondaryFileSchema {
    pub pattern: String,
    pub required: bool,
}

impl From<String> for SecondaryFileSchema {
    fn from(pattern: String) -> Self {
        if pattern.ends_with("?") {
            let pattern = pattern.trim_end_matches('?').to_string();
            SecondaryFileSchema { pattern, required: false }
        } else {
            SecondaryFileSchema { pattern, required: true }
        }
    }
}

impl<'de> Deserialize<'de> for SecondaryFileSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        match value {
            Value::String(pattern) => Ok(SecondaryFileSchema::from(pattern)),
            Value::Mapping(map) => {
                let pattern = map
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| serde::de::Error::custom("Expected string for pattern"))?
                    .to_string();
                let required = map.get("required").and_then(|v| v.as_bool()).unwrap_or(true);
                Ok(SecondaryFileSchema { pattern, required })
            }
            _ => Err(serde::de::Error::custom("Expected string or mapping for secondary file schema")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_files: Option<Vec<DefaultValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listing: Option<Vec<DefaultValue>>,
}

impl Default for Directory {
    fn default() -> Self {
        Self {
            class: String::from("Directory"),
            location: Default::default(),
            path: Default::default(),
            secondary_files: Default::default(),
            basename: Default::default(),
            listing: Default::default(),
        }
    }
}

impl Directory {
    pub fn from_location(location: impl ToString) -> Self {
        Directory {
            location: Some(location.to_string()),
            ..Default::default()
        }
    }

    pub fn load(&mut self, relative_to: impl AsRef<Path>) {
        if let Some(loc) = &self.location {
            let mut path = PathBuf::from(&loc);
            if !path.exists() {
                path = relative_to.as_ref().join(path);
            }

            let loc = path.strip_prefix(relative_to).unwrap_or(&path).to_string_lossy().into_owned();
            self.path = Some(loc);
        }
    }
}

impl PathItem for Directory {
    fn set_location(&mut self, new_location: String) {
        self.location = Some(new_location);
    }

    fn secondary_files_mut(&mut self) -> Option<&mut Vec<DefaultValue>> {
        self.secondary_files.as_mut()
    }

    fn get_location(&self) -> String {
        self.location.as_ref().unwrap_or(&String::new()).clone()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum EnviromentDefs {
    Vec(Vec<EnvironmentDef>),
    Map(HashMap<String, String>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Dirent {
    pub entryname: Option<String>,
    pub entry: Entry,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writeable: Option<bool>,
}

impl Default for Dirent {
    fn default() -> Self {
        Self {
            entryname: None,
            entry: Entry::Source(String::new()),
            writeable: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum Entry {
    Source(String),
    Include(Include),
}

impl Entry {
    pub fn from_file(path: impl ToString) -> Entry {
        Entry::Include(Include { include: path.to_string() })
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Include {
    #[serde(rename = "$include")]
    pub include: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentDef {
    pub env_name: String,
    pub env_value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::CommandInputParameter;

    #[test]
    pub fn test_deserialize_nullable_type() {
        let yaml = r"
- str:
  type: string?
- number:
  type: int?
- boolean:
  type: boolean
";
        let inputs: Vec<CommandInputParameter> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(inputs[0].type_, CWLType::Optional(Box::new(CWLType::String)));
        assert_eq!(inputs[1].type_, CWLType::Optional(Box::new(CWLType::Int)));
        assert_eq!(inputs[2].type_, CWLType::Boolean);
    }

    #[test]
    pub fn test_deserialize_nullable_type_alt() {
        let yaml = r#"
- str:
  type: ["null", string]
"#;
        let inputs: Vec<CommandInputParameter> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(inputs[0].type_, CWLType::Optional(Box::new(CWLType::String)));
    }

    #[test]
    pub fn test_deserialize_array_type() {
        let yaml = r"
- str:
  type: string[]
- number:
  type: int[]
";
        let inputs: Vec<CommandInputParameter> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(inputs[0].type_, CWLType::Array(Box::new(CWLType::String)));
        assert_eq!(inputs[1].type_, CWLType::Array(Box::new(CWLType::Int)));
    }

    #[test]
    pub fn test_deserialize_array_type_alt() {
        let yaml = r"
- str:
  type: 
      type: array
      items: string
";
        let inputs: Vec<CommandInputParameter> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(inputs[0].type_, CWLType::Array(Box::new(CWLType::String)));
    }

    #[test]
    pub fn test_serialize_nullable_type() {
        let t = CWLType::Optional(Box::new(CWLType::String));
        assert_eq!(&serialize_type(&t), "string?");
    }

    #[test]
    pub fn test_serialize_array_type() {
        let t = CWLType::Array(Box::new(CWLType::String));
        assert_eq!(&serialize_type(&t), "string[]");
    }

    #[test]
    pub fn test_matching_types() {
        let default_value_null = DefaultValue::Any(Value::Null);
        let default_value_bool = DefaultValue::Any(Value::Bool(true));
        let default_value_int = DefaultValue::Any(Value::Number(42.into()));
        let default_value_float = DefaultValue::Any(Value::Number(42.5.into()));
        let default_value_string = DefaultValue::Any(Value::String("Hello".to_string()));
        let default_value_array = DefaultValue::Any(Value::Sequence(vec![
            Value::String("Hello".to_string()),
            Value::String("World".to_string()),
        ]));

        //basic matching
        assert!(default_value_bool.has_matching_type(&CWLType::Boolean)); // true matches boolean
        assert!(default_value_int.has_matching_type(&CWLType::Int)); //42 matches int
        assert!(default_value_int.has_matching_type(&CWLType::Long)); //42 matches long
        assert!(default_value_float.has_matching_type(&CWLType::Float)); //42.4 matches float
        assert!(default_value_float.has_matching_type(&CWLType::Double)); //42.5 matches float
        assert!(default_value_string.has_matching_type(&CWLType::String)); // "Hello" is a string
        assert!(!default_value_string.has_matching_type(&CWLType::Int)); // "Hello" is a string

        //optional matching
        assert!(default_value_bool.has_matching_type(&CWLType::Optional(Box::new(CWLType::Boolean)))); // true matches boolean
        assert!(default_value_int.has_matching_type(&CWLType::Optional(Box::new(CWLType::Int)))); //42 matches int
        assert!(default_value_int.has_matching_type(&CWLType::Optional(Box::new(CWLType::Long)))); //42 matches long
        assert!(default_value_int.has_matching_type(&CWLType::Optional(Box::new(CWLType::Float)))); //42 matches float
        assert!(default_value_int.has_matching_type(&CWLType::Optional(Box::new(CWLType::Double)))); //42 matches double
        assert!(default_value_float.has_matching_type(&CWLType::Optional(Box::new(CWLType::Float)))); //42.4 matches float
        assert!(default_value_float.has_matching_type(&CWLType::Optional(Box::new(CWLType::Double)))); //42.5 matches float
        assert!(default_value_string.has_matching_type(&CWLType::Optional(Box::new(CWLType::String)))); // "Hello" is a string#
        assert!(!default_value_string.has_matching_type(&CWLType::Optional(Box::new(CWLType::Int)))); // "Hello" is not int

        //array matching
        assert!(default_value_array.has_matching_type(&CWLType::Array(Box::new(CWLType::String)))); // array of string is detected
        assert!(!default_value_array.has_matching_type(&CWLType::Optional(Box::new(CWLType::String)))); // is not optional
        assert!(!default_value_array.has_matching_type(&CWLType::String)); // is not a string!
        assert!(default_value_array.has_matching_type(&CWLType::Any)); //any type workd

        //null matching
        assert!(default_value_null.has_matching_type(&CWLType::Null)); //null matches Null
        assert!(default_value_null.has_matching_type(&CWLType::Optional(Box::new(CWLType::String))));
        //null is valid for optional
    }

    #[test]
    pub fn test_resolve_format() {
        let result = resolve_format(Some("edam:format_1234".to_string())).unwrap();
        let expected = "http://edamontology.org/format_1234";

        assert_eq!(result, expected.to_string());
    }

    #[test]
    pub fn test_guess_type() {
        let inputs = &[
            ("../../README.md", CWLType::File),
            ("/some/path/that/does/not/exist.txt", CWLType::String),
            ("src/", CWLType::Directory),
            ("--option", CWLType::String),
            ("2", CWLType::Int),
            ("1.5", CWLType::Float),
            ("https://some_url", CWLType::File), //urls are files!
        ];

        for input in inputs {
            let t = guess_type(input.0);
            println!("{:?}=>{:?}", input.0, input.1);
            assert_eq!(t, input.1);
        }
    }
}
